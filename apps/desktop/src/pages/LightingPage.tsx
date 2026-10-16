import React, { useState, useEffect, useCallback } from "react";
import { useDevice } from "../context/DeviceContext";
import type {
  LightingConfigDto,
  LightingMode,
  LightingDirection,
  PersistenceState,
  TypedError,
} from "../types/ipc";

const DEFAULT_CONFIG: LightingConfigDto = {
  mode: "wave",
  color: { r: 255, g: 0, b: 0 },
  brightness: 100,
  speed: 50,
  direction: "left_to_right",
};

export const LightingPage: React.FC = () => {
  const { selectedDevice, bridge } = useDevice();
  const [config, setConfig] = useState<LightingConfigDto>(DEFAULT_CONFIG);
  const [supportedModes, setSupportedModes] = useState<LightingMode[]>([
    "off",
    "static",
    "breathing",
    "wave",
    "rainbow",
    "ripple",
    "reactive",
  ]);
  const [persistence, setPersistence] = useState<PersistenceState>("unknown");
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<TypedError | null>(null);
  const [successNotice, setSuccessNotice] = useState<string | null>(null);
  const [importJson, setImportJson] = useState<string>("");
  const [showImportDialog, setShowImportDialog] = useState<boolean>(false);

  useEffect(() => {
    if (!showImportDialog) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setShowImportDialog(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [showImportDialog]);

  const fetchRgbStatus = useCallback(async () => {
    if (!selectedDevice || !selectedDevice.interfaces.config) return;
    setLoading(true);
    setError(null);
    const resp = await bridge.send("rgb_status", selectedDevice.id, {});
    setLoading(false);

    if (resp.ok) {
      if (resp.data.config) {
        setConfig({
          ...resp.data.config,
          direction: resp.data.config.direction ?? "left_to_right",
        });
      }
      setPersistence(resp.data.persistence);
      if (resp.data.supportedModes.length > 0) {
        setSupportedModes(resp.data.supportedModes);
      }
    } else {
      setError(resp.error);
    }
  }, [bridge, selectedDevice]);

  useEffect(() => {
    fetchRgbStatus();
  }, [fetchRgbStatus]);

  const handlePreview = async () => {
    if (!selectedDevice) return;
    setError(null);
    setSuccessNotice(null);

    const safeConfig: LightingConfigDto = {
      ...config,
      direction: config.direction ?? "left_to_right",
    };

    const resp = await bridge.send("rgb_preview", selectedDevice.id, {
      config: safeConfig,
    });

    if (resp.ok) {
      setPersistence("ram");
      setSuccessNotice("RAM preview active (throttled). Zero flash memory wear.");
    } else {
      setError(resp.error);
    }
  };

  const handleCommit = async () => {
    if (!selectedDevice) return;
    setError(null);
    setSuccessNotice(null);

    const resp = await bridge.send("rgb_commit", selectedDevice.id, {});

    if (resp.ok) {
      setPersistence("verified");
      setSuccessNotice("Configuration verified and committed to flash memory.");
    } else {
      setError(resp.error);
    }
  };

  const handleExport = async () => {
    if (!selectedDevice) return;
    setError(null);
    const resp = await bridge.send("export_profile", selectedDevice.id, {
      config,
      description: `MonKey Profile for ${selectedDevice.label}`,
    });

    if (resp.ok && "profileJson" in resp.data) {
      const blob = new Blob([resp.data.profileJson], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `monkey-lighting-profile-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
      setSuccessNotice("Profile JSON exported locally.");
    } else if (!resp.ok) {
      setError(resp.error);
    }
  };

  const handleImportSubmit = async () => {
    if (!selectedDevice || !importJson.trim()) return;
    setError(null);
    try {
      const resp = await bridge.send("import_profile", selectedDevice.id, {
        profileJson: importJson,
      });
      if (resp.ok && resp.data.config) {
        setConfig({
          ...resp.data.config,
          direction: resp.data.config.direction ?? "left_to_right",
        });
        setPersistence("ram");
        setShowImportDialog(false);
        setSuccessNotice("Profile validated and imported to local preview.");
      } else if (!resp.ok) {
        setError(resp.error);
      }
    } catch (err: unknown) {
      setError({
        code: "invalid_input",
        message: err instanceof Error ? err.message : "Malformed profile JSON",
        retryable: false,
      });
    }
  };

  const hasConfigInterface = selectedDevice?.interfaces.config ?? false;

  return (
    <div className="page lighting-page">
      <header className="page-header">
        <h1 className="page-title">RGB Lighting Configuration</h1>
        <p className="page-description">
          64-byte feature reports via Interface B (0xFFFF:0x0001). Live RAM preview with
          debounced flash persistence.
        </p>
      </header>

      {error && (
        <div className="panel error-panel" role="alert">
          <div className="panel-header">Lighting Operation Notice</div>
          <p className="error-message">{error.message}</p>
          <span className="badge badge-inactive">Code: {error.code}</span>
        </div>
      )}

      {successNotice && (
        <div className="panel success-panel" role="status">
          <p>{successNotice}</p>
        </div>
      )}

      {!hasConfigInterface && (
        <div className="panel error-panel">
          <div className="panel-header">Interface B Inactive</div>
          <p>
            Interface B (0xFFFF:0x0001) is inactive on this device. Lighting control requires
            USB connection or valid macOS shared-device access.
          </p>
        </div>
      )}

      <div className="lighting-layout">
        {/* Controls Column */}
        <section className="panel" aria-labelledby="controls-heading">
          <div className="panel-header" id="controls-heading">
            <span>Lighting Parameters</span>
            <span
              className={`badge ${
                persistence === "verified"
                  ? "badge-success"
                  : persistence === "ram"
                    ? "badge-caution"
                    : "badge-inactive"
              }`}
            >
              {persistence === "verified"
                ? "Verified in Flash"
                : persistence === "ram"
                  ? "RAM Preview (Unsaved)"
                  : "Status Unknown"}
            </span>
          </div>

          <div className="form-group">
            <label htmlFor="mode-select">Lighting Mode</label>
            <select
              id="mode-select"
              className="pixel-select"
              value={config.mode}
              disabled={!hasConfigInterface || loading}
              onChange={(e) =>
                setConfig((prev) => ({
                  ...prev,
                  mode: e.target.value as LightingMode,
                }))
              }
            >
              {supportedModes.map((mode) => (
                <option key={mode} value={mode}>
                  {mode.toUpperCase()}
                </option>
              ))}
            </select>
          </div>

          <div className="form-group">
            <label htmlFor="brightness-slider">
              Brightness: {config.brightness}%
            </label>
            <input
              type="range"
              id="brightness-slider"
              min="0"
              max="100"
              value={config.brightness}
              disabled={!hasConfigInterface || loading}
              onChange={(e) =>
                setConfig((prev) => ({
                  ...prev,
                  brightness: Number(e.target.value),
                }))
              }
            />
          </div>

          <div className="form-group">
            <label htmlFor="speed-slider">Speed: {config.speed}%</label>
            <input
              type="range"
              id="speed-slider"
              min="0"
              max="100"
              value={config.speed}
              disabled={!hasConfigInterface || loading}
              onChange={(e) =>
                setConfig((prev) => ({
                  ...prev,
                  speed: Number(e.target.value),
                }))
              }
            />
          </div>

          <div className="form-group">
            <label htmlFor="direction-select">Animation Direction</label>
            <select
              id="direction-select"
              className="pixel-select"
              value={config.direction ?? "left_to_right"}
              disabled={!hasConfigInterface || loading}
              onChange={(e) =>
                setConfig((prev) => ({
                  ...prev,
                  direction: e.target.value as LightingDirection,
                }))
              }
            >
              <option value="left_to_right">Left to Right</option>
              <option value="right_to_left">Right to Left</option>
            </select>
          </div>

          <div className="form-group color-group">
            <label>RGB Primary Color</label>
            <div className="color-inputs">
              <label htmlFor="color-r" className="color-sublabel">
                R:
                <input
                  id="color-r"
                  type="number"
                  min="0"
                  max="255"
                  value={config.color.r}
                  disabled={!hasConfigInterface || loading}
                  onChange={(e) =>
                    setConfig((prev) => ({
                      ...prev,
                      color: { ...prev.color, r: Number(e.target.value) },
                    }))
                  }
                />
              </label>
              <label htmlFor="color-g" className="color-sublabel">
                G:
                <input
                  id="color-g"
                  type="number"
                  min="0"
                  max="255"
                  value={config.color.g}
                  disabled={!hasConfigInterface || loading}
                  onChange={(e) =>
                    setConfig((prev) => ({
                      ...prev,
                      color: { ...prev.color, g: Number(e.target.value) },
                    }))
                  }
                />
              </label>
              <label htmlFor="color-b" className="color-sublabel">
                B:
                <input
                  id="color-b"
                  type="number"
                  min="0"
                  max="255"
                  value={config.color.b}
                  disabled={!hasConfigInterface || loading}
                  onChange={(e) =>
                    setConfig((prev) => ({
                      ...prev,
                      color: { ...prev.color, b: Number(e.target.value) },
                    }))
                  }
                />
              </label>
              <div
                className="color-swatch"
                role="img"
                style={{
                  backgroundColor: `rgb(${config.color.r}, ${config.color.g}, ${config.color.b})`,
                }}
                aria-label={`Color preview swatch: rgb(${config.color.r}, ${config.color.g}, ${config.color.b})`}
              />
            </div>
          </div>

          <div className="action-row">
            <button
              type="button"
              className="pixel-btn"
              disabled={!hasConfigInterface || loading}
              onClick={handlePreview}
            >
              Preview in RAM
            </button>
            <button
              type="button"
              className="pixel-btn primary-btn"
              disabled={!hasConfigInterface || loading}
              onClick={handleCommit}
            >
              Save to Keyboard Flash
            </button>
          </div>
        </section>

        {/* Profile Management Column */}
        <section className="panel" aria-labelledby="profile-heading">
          <div className="panel-header" id="profile-heading">
            <span>Profile Storage (Local & Device)</span>
          </div>
          <p className="section-note">
            Profiles can be exported as portable JSON files or loaded into memory. Saving to
            device flash is separate from local file export.
          </p>
          <div className="btn-group">
            <button
              type="button"
              className="pixel-btn"
              onClick={handleExport}
              disabled={!hasConfigInterface}
            >
              Export Profile (JSON)
            </button>
            <button
              type="button"
              className="pixel-btn"
              onClick={() => setShowImportDialog(true)}
              disabled={!hasConfigInterface}
            >
              Import Profile (JSON)
            </button>
          </div>

          {showImportDialog && (
            <div className="import-modal" role="dialog" aria-modal="true" aria-labelledby="import-title">
              <h2 id="import-title">Paste Profile JSON</h2>
              <textarea
                className="profile-textarea"
                rows={6}
                value={importJson}
                onChange={(e) => setImportJson(e.target.value)}
                placeholder='{"version": 1, "config": { ... }}'
                aria-label="Profile JSON content"
              />
              <div className="action-row">
                <button
                  type="button"
                  className="pixel-btn primary-btn"
                  onClick={handleImportSubmit}
                >
                  Validate & Preview
                </button>
                <button
                  type="button"
                  className="pixel-btn"
                  onClick={() => setShowImportDialog(false)}
                >
                  Cancel
                </button>
              </div>
            </div>
          )}
        </section>
      </div>
    </div>
  );
};
