import React, { useState, useEffect, useCallback, useRef } from "react";
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

// 34ms corresponds to ~29.4Hz, strictly enforcing the <= 30Hz contract requirement
const THROTTLE_INTERVAL_MS = 34;

export const LightingPage: React.FC = () => {
  const { selectedDevice, bridge } = useDevice();
  const [config, setConfig] = useState<LightingConfigDto>(DEFAULT_CONFIG);
  const [supportedModes, setSupportedModes] = useState<LightingMode[]>([]);
  const [persistence, setPersistence] = useState<PersistenceState>("unknown");
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<TypedError | null>(null);
  const [successNotice, setSuccessNotice] = useState<string | null>(null);
  const [previewAccepted, setPreviewAccepted] = useState<boolean>(false);
  const [importJson, setImportJson] = useState<string>("");
  const [showImportDialog, setShowImportDialog] = useState<boolean>(false);

  const importBtnRef = useRef<HTMLButtonElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const currentDeviceIdRef = useRef<string | null>(selectedDevice?.id ?? null);

  // Throttling refs
  const lastPreviewTimeRef = useRef<number>(0);
  const pendingPreviewTimerRef = useRef<NodeJS.Timeout | null>(null);
  const pendingConfigRef = useRef<LightingConfigDto | null>(null);

  // Sync current device ID and reset state on switch
  useEffect(() => {
    currentDeviceIdRef.current = selectedDevice?.id ?? null;
    setPreviewAccepted(false);
    setError(null);
    setSuccessNotice(null);
    if (pendingPreviewTimerRef.current) {
      clearTimeout(pendingPreviewTimerRef.current);
      pendingPreviewTimerRef.current = null;
    }
  }, [selectedDevice?.id]);

  // Clean up timer on unmount
  useEffect(() => {
    return () => {
      if (pendingPreviewTimerRef.current) {
        clearTimeout(pendingPreviewTimerRef.current);
        pendingPreviewTimerRef.current = null;
      }
    };
  }, []);

  // Modal focus trap & Escape listener
  useEffect(() => {
    if (!showImportDialog) return;

    // Focus textarea on modal open
    const focusTimer = setTimeout(() => {
      textareaRef.current?.focus();
    }, 10);

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setShowImportDialog(false);
        importBtnRef.current?.focus();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      clearTimeout(focusTimer);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [showImportDialog]);

  const fetchRgbStatus = useCallback(async () => {
    if (!selectedDevice || !selectedDevice.interfaces.config) return;
    const targetDeviceId = selectedDevice.id;
    setLoading(true);
    setError(null);
    const resp = await bridge.send("rgb_status", targetDeviceId, {});
    setLoading(false);

    // Stale device check
    if (currentDeviceIdRef.current !== targetDeviceId) {
      return;
    }

    if (resp.ok) {
      if (resp.data.config) {
        // Retain direction as-is (do not default null direction to left_to_right)
        setConfig(resp.data.config);
      }
      setPersistence(resp.data.persistence);
      if (resp.data.supportedModes && resp.data.supportedModes.length > 0) {
        setSupportedModes(resp.data.supportedModes);
      }
    } else {
      setError(resp.error);
    }
  }, [bridge, selectedDevice]);

  useEffect(() => {
    fetchRgbStatus();
  }, [fetchRgbStatus]);

  const executePreviewCall = useCallback(
    async (cfg: LightingConfigDto, targetDeviceId: string) => {
      setError(null);
      setSuccessNotice(null);

      const resp = await bridge.send("rgb_preview", targetDeviceId, {
        config: cfg,
      });

      // Stale device check
      if (currentDeviceIdRef.current !== targetDeviceId) {
        return;
      }

      if (resp.ok) {
        setPersistence("ram");
        setPreviewAccepted(true);
        setSuccessNotice("RAM preview active (throttled <=30Hz). Zero flash memory wear.");
      } else {
        setPreviewAccepted(false);
        setError(resp.error);
      }
    },
    [bridge]
  );

  const throttledPreview = useCallback(
    (cfg: LightingConfigDto) => {
      if (!selectedDevice) return;
      const targetDeviceId = selectedDevice.id;

      // Validate direction: preview rejects direction=null
      if (cfg.direction === null) {
        setError({
          code: "invalid_input",
          message: "Direction cannot be null for preview. Please select an animation direction.",
          retryable: false,
        });
        setPreviewAccepted(false);
        return;
      }

      const now = Date.now();
      const elapsed = now - lastPreviewTimeRef.current;

      pendingConfigRef.current = cfg;

      if (elapsed >= THROTTLE_INTERVAL_MS) {
        lastPreviewTimeRef.current = now;
        if (pendingPreviewTimerRef.current) {
          clearTimeout(pendingPreviewTimerRef.current);
          pendingPreviewTimerRef.current = null;
        }
        executePreviewCall(cfg, targetDeviceId);
      } else {
        if (!pendingPreviewTimerRef.current) {
          const waitTime = THROTTLE_INTERVAL_MS - elapsed;
          pendingPreviewTimerRef.current = setTimeout(() => {
            lastPreviewTimeRef.current = Date.now();
            pendingPreviewTimerRef.current = null;
            if (pendingConfigRef.current) {
              executePreviewCall(pendingConfigRef.current, targetDeviceId);
            }
          }, waitTime);
        }
      }
    },
    [executePreviewCall, selectedDevice]
  );

  const handlePreview = () => {
    throttledPreview(config);
  };

  const handleCommit = async () => {
    if (!selectedDevice || !previewAccepted) return;
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
        setConfig(resp.data.config);
        setPersistence("ram");
        setPreviewAccepted(true);
        setShowImportDialog(false);
        importBtnRef.current?.focus();
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

  const updateConfig = (updater: (prev: LightingConfigDto) => LightingConfigDto) => {
    setPreviewAccepted(false);
    setConfig((prev) => updater(prev));
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
              disabled={!hasConfigInterface || loading || supportedModes.length === 0}
              onChange={(e) =>
                updateConfig((prev) => ({
                  ...prev,
                  mode: e.target.value as LightingMode,
                }))
              }
            >
              {supportedModes.length === 0 && (
                <option value="">No modes available</option>
              )}
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
                updateConfig((prev) => ({
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
                updateConfig((prev) => ({
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
              value={config.direction ?? ""}
              disabled={!hasConfigInterface || loading}
              onChange={(e) =>
                updateConfig((prev) => ({
                  ...prev,
                  direction: (e.target.value as LightingDirection) || null,
                }))
              }
            >
              <option value="">Select direction...</option>
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
                    updateConfig((prev) => ({
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
                    updateConfig((prev) => ({
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
                    updateConfig((prev) => ({
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
              disabled={!hasConfigInterface || loading || !previewAccepted}
              title={
                previewAccepted
                  ? "Commit current previewed config to flash memory"
                  : "Preview in RAM must succeed before saving to flash"
              }
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
              ref={importBtnRef}
              className="pixel-btn"
              onClick={() => setShowImportDialog(true)}
              disabled={!hasConfigInterface}
            >
              Import Profile (JSON)
            </button>
          </div>

          {showImportDialog && (
            <div
              className="import-modal"
              role="dialog"
              aria-modal="true"
              aria-labelledby="import-title"
            >
              <h2 id="import-title">Paste Profile JSON</h2>
              <textarea
                ref={textareaRef}
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
                  onClick={() => {
                    setShowImportDialog(false);
                    importBtnRef.current?.focus();
                  }}
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
