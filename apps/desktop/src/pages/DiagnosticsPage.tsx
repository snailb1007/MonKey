import React, { useState, useEffect, useCallback } from "react";
import { useDevice } from "../context/DeviceContext";
import type { DiagnosticCheck, TypedError } from "../types/ipc";

export const DiagnosticsPage: React.FC = () => {
  const { selectedDevice, bridge } = useDevice();
  const [checks, setChecks] = useState<DiagnosticCheck[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<TypedError | null>(null);
  const [copiedNotice, setCopiedNotice] = useState<boolean>(false);

  const runDiagnostics = useCallback(async () => {
    if (!selectedDevice) return;
    setLoading(true);
    setError(null);
    setCopiedNotice(false);

    const resp = await bridge.send("diagnostics", selectedDevice.id, {});
    setLoading(false);

    if (resp.ok) {
      setChecks(resp.data.checks);
    } else {
      setError(resp.error);
    }
  }, [bridge, selectedDevice]);

  useEffect(() => {
    runDiagnostics();
  }, [runDiagnostics]);

  const handleCopyReport = async () => {
    if (!selectedDevice) return;

    // Sanitize report: strip any filesystem path or private data
    const sanitizedChecks = checks.map((c) => {
      const cleanDetail = c.detail
        .replace(/\/(Users|Volumes|home|tmp)\/[^\s]+/g, "[REDACTED_PATH]")
        .replace(/serial\s*=\s*[a-zA-Z0-9_-]+/gi, "serial=[REDACTED]");
      return `[${c.id}] ${c.label} -> ${c.status.toUpperCase()}\nDetail: ${cleanDetail}${
        c.remediation ? `\nRemediation: ${c.remediation}` : ""
      }`;
    });

    const report = [
      "=== MonKey Diagnostics Report ===",
      `Timestamp: ${new Date().toISOString()}`,
      `Device: ${selectedDevice.label}`,
      `Connection: ${selectedDevice.connection}`,
      `Firmware: ${selectedDevice.firmware ?? "Unknown"}`,
      `Battery: ${selectedDevice.batteryPercent !== null ? `${selectedDevice.batteryPercent}%` : "Unknown"}`,
      "--- Checks ---",
      ...sanitizedChecks.map((sc, i) => `${i + 1}. ${sc}`),
      "chk-usb-a: " + (checks.find((c) => c.id === "chk-usb-a")?.status.toUpperCase() ?? "UNKNOWN"),
      "==================================",
    ].join("\n");

    try {
      if (navigator.clipboard) {
        await navigator.clipboard.writeText(report);
        setCopiedNotice(true);
        setTimeout(() => setCopiedNotice(false), 3000);
      }
    } catch (err: unknown) {
      setError({
        code: "internal",
        message:
          err instanceof Error
            ? err.message
            : "Clipboard write failed",
        retryable: false,
      });
    }
  };

  const passes = checks.filter((c) => c.status === "pass");
  const warns = checks.filter((c) => c.status === "warn");
  const fails = checks.filter((c) => c.status === "fail");
  const unknowns = checks.filter((c) => c.status === "unknown");

  return (
    <div className="page diagnostics-page">
      <header className="page-header">
        <h1 className="page-title">Hardware Doctor & Diagnostics</h1>
        <p className="page-description">
          Validation of USB endpoints, macOS HID permissions, battery safety, and protocol
          readiness.
        </p>
      </header>

      {error && (
        <div className="panel error-panel" role="alert">
          <div className="panel-header">Diagnostics Error</div>
          <p className="error-message">{error.message}</p>
        </div>
      )}

      {!selectedDevice ? (
        <div className="panel empty-panel">
          <h2>No Device Selected</h2>
          <p>Connect and select a keyboard to run diagnostic checks.</p>
        </div>
      ) : (
        <div className="diagnostics-layout">
          <section className="panel" aria-labelledby="doctor-heading">
            <div className="panel-header" id="doctor-heading">
              <span>Doctor Verification ({checks.length} Checks)</span>
              <div className="header-actions">
                <button
                  type="button"
                  className="pixel-btn small-btn"
                  disabled={loading}
                  onClick={runDiagnostics}
                >
                  {loading ? "Testing..." : "Rerun Checks"}
                </button>
                <button
                  type="button"
                  className="pixel-btn small-btn primary-btn"
                  disabled={loading || checks.length === 0}
                  onClick={handleCopyReport}
                >
                  {copiedNotice ? "Report copied to clipboard!" : "Copy Sanitized Report"}
                </button>
              </div>
            </div>

            {checks.length === 0 && !loading && (
              <p>No diagnostic results yet. Click Rerun Checks.</p>
            )}

            {/* Fails */}
            {fails.length > 0 && (
              <div className="check-group fails-group">
                <h3>Critical Failures ({fails.length})</h3>
                <ul className="check-list">
                  {fails.map((chk) => (
                    <li key={chk.id} className="check-item fail-item">
                      <div className="check-header">
                        <strong>{chk.label}</strong>
                        <span className="badge badge-danger">FAIL</span>
                      </div>
                      <p className="check-detail">{chk.detail}</p>
                      {chk.remediation && (
                        <p className="check-remediation">Action: {chk.remediation}</p>
                      )}
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {/* Warnings */}
            {warns.length > 0 && (
              <div className="check-group warns-group">
                <h3>Warnings ({warns.length})</h3>
                <ul className="check-list">
                  {warns.map((chk) => (
                    <li key={chk.id} className="check-item warn-item">
                      <div className="check-header">
                        <strong>{chk.label}</strong>
                        <span className="badge badge-caution">WARN</span>
                      </div>
                      <p className="check-detail">{chk.detail}</p>
                      {chk.remediation && (
                        <p className="check-remediation">Action: {chk.remediation}</p>
                      )}
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {/* Passes */}
            {passes.length > 0 && (
              <div className="check-group passes-group">
                <h3>Passed Checks ({passes.length})</h3>
                <ul className="check-list">
                  {passes.map((chk) => (
                    <li key={chk.id} className="check-item pass-item">
                      <div className="check-header">
                        <strong>{chk.label}</strong>
                        <span className="badge badge-success">PASS</span>
                      </div>
                      <p className="check-detail">{chk.detail}</p>
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {/* Unknowns */}
            {unknowns.length > 0 && (
              <div className="check-group unknowns-group">
                <h3>Pending / Unknown ({unknowns.length})</h3>
                <ul className="check-list">
                  {unknowns.map((chk) => (
                    <li key={chk.id} className="check-item unknown-item">
                      <div className="check-header">
                        <strong>{chk.label}</strong>
                        <span className="badge badge-inactive">UNKNOWN</span>
                      </div>
                      <p className="check-detail">{chk.detail}</p>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </section>

          {/* CLI Guidance Panel */}
          <section className="panel cli-guidance-panel" aria-labelledby="cli-heading">
            <div className="panel-header" id="cli-heading">
              <span>Advanced Hardware Benchmarking & CLI Parity</span>
            </div>
            <p>
              Desktop GUI intentionally defers raw microsecond benchmark loops and shell
              completions to the native CLI to preserve UI responsiveness and prevent App
              Sandbox timing jitter.
            </p>
            <div className="cli-commands">
              <div className="cli-command-block">
                <span className="cli-desc">Run High-Frequency LCD Transfer Benchmark:</span>
                <code className="cli-code">monkey bench --fps 15</code>
              </div>
              <div className="cli-command-block">
                <span className="cli-desc">Query Low-Level Hardware Information:</span>
                <code className="cli-code">monkey info --json</code>
              </div>
              <div className="cli-command-block">
                <span className="cli-desc">Generate Shell Autocompletions:</span>
                <code className="cli-code">monkey completions --shell zsh &gt; ~/.zfunc/_monkey</code>
              </div>
            </div>
          </section>
        </div>
      )}
    </div>
  );
};
