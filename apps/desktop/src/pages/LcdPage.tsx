import React, { useState, useEffect, useRef, useCallback } from "react";
import { useDevice } from "../context/DeviceContext";
import type {
  AssetPreview,
  JobSnapshot,
  TestPatternType,
  TypedError,
} from "../types/ipc";

const PATTERN_OPTIONS: { id: TestPatternType; label: string }[] = [
  { id: "color_bars", label: "Color Bars (RGB)" },
  { id: "geometry", label: "Geometry Grid" },
  { id: "red", label: "Solid Red" },
  { id: "green", label: "Solid Green" },
  { id: "blue", label: "Solid Blue" },
  { id: "white", label: "Solid White" },
  { id: "black", label: "Solid Black" },
  { id: "grayscale", label: "Grayscale Gradient" },
];

export const LcdPage: React.FC = () => {
  const { selectedDevice, bridge } = useDevice();
  const [selectedAsset, setSelectedAsset] = useState<AssetPreview | null>(null);
  const [selectedPattern, setSelectedPattern] = useState<TestPatternType>("color_bars");
  const [activeJob, setActiveJob] = useState<JobSnapshot | null>(null);
  const [error, setError] = useState<TypedError | null>(null);
  const [isPicking, setIsPicking] = useState<boolean>(false);

  const pollTimerRef = useRef<NodeJS.Timeout | null>(null);
  const currentDeviceIdRef = useRef<string | null>(selectedDevice?.id ?? null);
  const activeJobIdRef = useRef<string | null>(null);

  // Clear active job and poll timer on device switch or unmount
  useEffect(() => {
    currentDeviceIdRef.current = selectedDevice?.id ?? null;
    if (pollTimerRef.current) {
      clearInterval(pollTimerRef.current);
      pollTimerRef.current = null;
    }
    activeJobIdRef.current = null;
    setActiveJob(null);
    setError(null);
  }, [selectedDevice?.id]);

  useEffect(() => {
    return () => {
      if (pollTimerRef.current) {
        clearInterval(pollTimerRef.current);
        pollTimerRef.current = null;
      }
      activeJobIdRef.current = null;
    };
  }, []);

  const stopPolling = useCallback(() => {
    if (pollTimerRef.current) {
      clearInterval(pollTimerRef.current);
      pollTimerRef.current = null;
    }
    activeJobIdRef.current = null;
  }, []);

  const startPolling = useCallback(
    (jobId: string, targetDeviceId: string) => {
      stopPolling();
      activeJobIdRef.current = jobId;

      // Poll at 2Hz (every 500ms) <= 4Hz limit required by contract
      pollTimerRef.current = setInterval(async () => {
        // Pre-flight check: ensure device or job haven't been stopped/switched
        if (
          !selectedDevice ||
          currentDeviceIdRef.current !== targetDeviceId ||
          activeJobIdRef.current !== jobId
        ) {
          stopPolling();
          return;
        }

        const resp = await bridge.send("job_status", targetDeviceId, { jobId });

        // Post-flight race check: drop late response if job was stopped or device switched
        if (
          activeJobIdRef.current !== jobId ||
          currentDeviceIdRef.current !== targetDeviceId
        ) {
          return;
        }

        if (resp.ok) {
          const job = resp.data.job;
          setActiveJob(job);
          if (
            job.state === "completed" ||
            job.state === "cancelled" ||
            job.state === "failed"
          ) {
            stopPolling();
          }
        } else {
          stopPolling();
          setError(resp.error);
        }
      }, 500);
    },
    [bridge, selectedDevice, stopPolling]
  );

  const handlePickAsset = async (kind: "image" | "animation" | "sequence") => {
    if (!selectedDevice) return;
    setIsPicking(true);
    setError(null);
    const resp = await bridge.send("pick_asset", selectedDevice.id, { kind });
    setIsPicking(false);

    if (resp.ok) {
      if (resp.data.asset) {
        setSelectedAsset(resp.data.asset);
      }
    } else {
      setError(resp.error);
    }
  };

  const handleStartLcd = async (useAsset: boolean) => {
    if (!selectedDevice) return;
    if (activeJob && (activeJob.state === "running" || activeJob.state === "queued")) {
      setError({
        code: "busy",
        message: "A streaming job is already active. Stop current job before starting a new one.",
        retryable: true,
      });
      return;
    }
    setError(null);

    const assetId = useAsset && selectedAsset ? selectedAsset.id : null;
    const pattern = useAsset ? null : selectedPattern;

    const resp = await bridge.send("start_lcd", selectedDevice.id, {
      assetId,
      pattern,
    });

    if (resp.ok) {
      const jobId = resp.data.jobId;
      setActiveJob({
        id: jobId,
        deviceId: selectedDevice.id,
        state: "running",
        framesSent: 0,
        error: null,
      });
      startPolling(jobId, selectedDevice.id);
    } else {
      setError(resp.error);
    }
  };

  const handleStopLcd = async () => {
    if (!selectedDevice || !activeJob) return;
    stopPolling();
    const resp = await bridge.send("stop_lcd", selectedDevice.id, {
      jobId: activeJob.id,
    });
    if (resp.ok) {
      setActiveJob((prev) => (prev ? { ...prev, state: "cancelled" } : null));
    } else {
      setError(resp.error);
    }
  };

  const hasLcdInterface = selectedDevice?.interfaces.lcd ?? false;
  const isStreaming = activeJob?.state === "running" || activeJob?.state === "queued";

  return (
    <div className="page lcd-page">
      <header className="page-header">
        <h1 className="page-title">LCD Display Management</h1>
        <p className="page-description">
          128x128 RGB565 frame streaming via Interface A (0xFF68:0x0061) bulk OUT reports.
        </p>
      </header>

      {error && (
        <div className="panel error-panel" role="alert">
          <div className="panel-header">LCD Stream Alert</div>
          <p className="error-message">{error.message}</p>
          <span className="badge badge-inactive">Code: {error.code}</span>
        </div>
      )}

      {!hasLcdInterface && (
        <div className="panel error-panel">
          <div className="panel-header">Interface Warning</div>
          <p>
            Interface A (0xFF68) is inactive on this device. LCD streaming commands are disabled.
          </p>
        </div>
      )}

      <div className="lcd-layout">
        {/* Left Column: Asset Chooser & Patterns */}
        <div className="lcd-controls-col">
          <section className="panel" aria-labelledby="asset-chooser-heading">
            <div className="panel-header" id="asset-chooser-heading">
              <span>Image & Animation Chooser</span>
            </div>
            <p className="section-note">
              Select local image, GIF, or image sequence. Files are converted to 128x128
              RGB565 in memory with zero permanent flash impact.
            </p>
            <div className="btn-group">
              <button
                type="button"
                className="pixel-btn"
                disabled={!hasLcdInterface || isPicking || isStreaming}
                onClick={() => handlePickAsset("animation")}
              >
                Choose Animation (GIF)
              </button>
              <button
                type="button"
                className="pixel-btn"
                disabled={!hasLcdInterface || isPicking || isStreaming}
                onClick={() => handlePickAsset("image")}
              >
                Choose Still Image
              </button>
              <button
                type="button"
                className="pixel-btn"
                disabled={!hasLcdInterface || isPicking || isStreaming}
                onClick={() => handlePickAsset("sequence")}
              >
                Choose Frame Sequence
              </button>
            </div>
          </section>

          <section className="panel" aria-labelledby="pattern-heading">
            <div className="panel-header" id="pattern-heading">
              <span>Diagnostic Test Patterns</span>
            </div>
            <div className="form-group">
              <label htmlFor="pattern-select">Test Pattern</label>
              <select
                id="pattern-select"
                className="pixel-select"
                value={selectedPattern}
                disabled={!hasLcdInterface || isStreaming}
                onChange={(e) => setSelectedPattern(e.target.value as TestPatternType)}
              >
                {PATTERN_OPTIONS.map((opt) => (
                  <option key={opt.id} value={opt.id}>
                    {opt.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="action-row">
              <button
                type="button"
                className="pixel-btn primary-btn"
                disabled={!hasLcdInterface || isStreaming}
                onClick={() => handleStartLcd(false)}
              >
                Send Pattern to Screen
              </button>
            </div>
          </section>
        </div>

        {/* Right Column: Preview & Streaming State */}
        <div className="lcd-preview-col">
          <section className="panel" aria-labelledby="preview-heading">
            <div className="panel-header" id="preview-heading">
              <span>Frame Preview</span>
              {selectedAsset && (
                <span className="badge badge-success">Asset Loaded</span>
              )}
            </div>

            <div className="lcd-preview-container">
              {selectedAsset ? (
                <>
                  <img
                    src={selectedAsset.previewDataUrl}
                    alt="Converted 128x128 RGB565 Preview"
                    className="lcd-preview-canvas"
                    width={128}
                    height={128}
                  />
                  <div className="lcd-meta">
                    <strong>{selectedAsset.name}</strong> &bull;{" "}
                    {selectedAsset.width}x{selectedAsset.height} px &bull;{" "}
                    {selectedAsset.frameCount} frames
                  </div>
                  <p className="preview-truthfulness-notice">
                    [Converted Local Preview] — Frame data verified in memory. Not yet
                    transmitted to hardware.
                  </p>
                </>
              ) : (
                <div className="lcd-preview-placeholder">
                  <div className="placeholder-screen">128 x 128</div>
                  <p className="lcd-meta">No asset selected. Select a file or pattern.</p>
                </div>
              )}
            </div>

            {selectedAsset && (
              <div className="action-row">
                <button
                  type="button"
                  className="pixel-btn primary-btn"
                  disabled={!hasLcdInterface || isStreaming}
                  onClick={() => handleStartLcd(true)}
                >
                  Stream Asset to LCD
                </button>
              </div>
            )}
          </section>

          {activeJob && (
            <section className="panel job-status-panel" aria-labelledby="job-heading">
              <div className="panel-header" id="job-heading">
                <span>Active Streaming Job</span>
                <span className="badge">{activeJob.state}</span>
              </div>
              <div className="job-details">
                <p>
                  Job ID: <code>{activeJob.id}</code>
                </p>
                <p>Frames Transmitted: {activeJob.framesSent}</p>
                {activeJob.error && (
                  <p className="error-message">{activeJob.error.message}</p>
                )}
              </div>
              {isStreaming && (
                <button
                  type="button"
                  className="pixel-btn danger-btn"
                  onClick={handleStopLcd}
                >
                  Stop Streaming
                </button>
              )}
            </section>
          )}
        </div>
      </div>
    </div>
  );
};
