import React, { useState, useEffect, useCallback, useMemo } from "react";
import { useDevice } from "../context/DeviceContext";
import { OperationNotice } from "../components/OperationNotice";
import { ActionButton } from "../components/ActionButton";
import {
  buildDiagnosticsReport,
  buildErrorReport,
  buildGitHubIssueUrl,
} from "../services/issueReport";
import { handleExternalLinkClick } from "../services/externalUrl";
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

    try {
      const resp = await bridge.send("diagnostics", selectedDevice.id, {});

      if (resp.ok) {
        setChecks(resp.data.checks);
      } else {
        setError(resp.error);
      }
    } finally {
      setLoading(false);
    }
  }, [bridge, selectedDevice]);

  useEffect(() => {
    runDiagnostics();
  }, [runDiagnostics]);

  const reportText = useMemo(() => {
    if (!selectedDevice || checks.length === 0) {
      return null;
    }
    return buildDiagnosticsReport({
      deviceLabel: selectedDevice.label,
      connection: selectedDevice.connection,
      firmware: selectedDevice.firmware,
      batteryPercent: selectedDevice.batteryPercent,
      checks: checks.map((c) => ({
        id: c.id,
        label: c.label,
        status: c.status,
        detail: c.detail,
        remediation: c.remediation,
      })),
    });
  }, [selectedDevice, checks]);

  const handleCopyReport = async () => {
    if (!selectedDevice || !reportText) return;

    try {
      if (navigator.clipboard) {
        await navigator.clipboard.writeText(reportText);
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
        <OperationNotice
          kind="error"
          title="Diagnostics Error"
          message={error.message}
          code={error.code}
          details={buildErrorReport({
            action: "diagnostics",
            deviceLabel: selectedDevice?.label,
            code: error.code,
            message: error.message,
          })}
          issueTitle={`Diagnostics failed: ${error.code}`}
          issueAction="Run diagnostics"
          retryable={error.retryable}
          onRetry={error.retryable ? () => void runDiagnostics() : undefined}
          onDismiss={() => setError(null)}
        />
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
                <ActionButton
                  className="pixel-btn small-btn"
                  disabled={loading}
                  pending={loading}
                  onClick={runDiagnostics}
                >
                  {loading ? "Testing..." : "Rerun Checks"}
                </ActionButton>
                <button
                  type="button"
                  className="pixel-btn small-btn primary-btn"
                  disabled={loading || checks.length === 0}
                  onClick={handleCopyReport}
                >
                  {copiedNotice ? "Report copied to clipboard!" : "Copy Sanitized Report"}
                </button>
                {reportText && (
                  <a
                    className="notice-link"
                    href={buildGitHubIssueUrl({
                      title: `Diagnostics report: ${fails.length} failure(s), ${warns.length} warning(s)`,
                      report: reportText,
                      actionLabel: "Run diagnostics",
                    })}
                    target="_blank"
                    rel="noreferrer"
                    onClick={handleExternalLinkClick}
                  >
                    Report on GitHub
                  </a>
                )}
              </div>
            </div>

            {reportText && (
              <p className="github-hint">
                Found a bug? 1) Copy Sanitized Report, 2) open “Report on
                GitHub”, 3) paste into the issue body.
              </p>
            )}

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
