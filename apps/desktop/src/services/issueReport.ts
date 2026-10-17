/**
 * Helpers for building copyable, sanitized bug reports and prefilled GitHub issue links.
 *
 * Privacy rule: reports must never include absolute filesystem paths or device
 * serial numbers — everything is redacted before it reaches the clipboard or a URL.
 */

export const GITHUB_REPO = "snailb1007/MonKey";
export const GITHUB_ISSUES_NEW_URL = `https://github.com/${GITHUB_REPO}/issues/new`;
export const APP_NAME = "monkey-desktop";
export const APP_VERSION = "0.1.0";

const PATH_PATTERN =
  /(?:\/private\/[^\s"']*|\/(?:Users|Volumes|home|tmp)\/[^\s"']*|[A-Za-z]:[\\/][^\s"']*|\\\\[^\s"']+)/g;
const SERIAL_PATTERN = /serial\s*=\s*[a-zA-Z0-9_-]+/gi;

/** Redact filesystem paths and serial numbers from free-form text. */
export function sanitizeReportText(input: string): string {
  return input
    .replace(PATH_PATTERN, "[REDACTED_PATH]")
    .replace(SERIAL_PATTERN, "serial=[REDACTED]");
}

export interface ErrorReportInput {
  /** IPC action that was attempted, e.g. "rgb_preview" or "start_lcd". */
  action: string;
  deviceLabel?: string | null;
  code?: string;
  message: string;
  /** Optional extra context, e.g. job id or frames sent. */
  extra?: string;
}

/** Build a short sanitized plain-text report for a failed operation. */
export function buildErrorReport(input: ErrorReportInput): string {
  const lines = [
    "=== MonKey Error Report ===",
    `Timestamp: ${new Date().toISOString()}`,
    `App: ${APP_NAME} ${APP_VERSION}`,
    `Action: ${input.action}`,
    `Device: ${input.deviceLabel ?? "none selected"}`,
    `Code: ${input.code ?? "unknown"}`,
    `Message: ${input.message}`,
  ];
  if (input.extra) {
    lines.push(`Context: ${input.extra}`);
  }
  lines.push("===========================");
  return sanitizeReportText(lines.join("\n"));
}

export interface DiagnosticsCheckInput {
  id: string;
  label: string;
  status: string;
  detail: string;
  remediation: string | null;
}

export interface DiagnosticsReportInput {
  deviceLabel: string;
  connection: string;
  firmware: string | null;
  batteryPercent: number | null;
  checks: DiagnosticsCheckInput[];
}

/**
 * Build the sanitized diagnostics report pasted into GitHub issues.
 * Format is stable by contract — asserted by diagnostics.test.tsx.
 */
export function buildDiagnosticsReport(input: DiagnosticsReportInput): string {
  const sanitizedChecks = input.checks.map((c) => {
    const cleanDetail = sanitizeReportText(c.detail);
    return `[${c.id}] ${c.label} -> ${c.status.toUpperCase()}\nDetail: ${cleanDetail}${
      c.remediation ? `\nRemediation: ${c.remediation}` : ""
    }`;
  });

  return [
    "=== MonKey Diagnostics Report ===",
    `Timestamp: ${new Date().toISOString()}`,
    `Device: ${input.deviceLabel}`,
    `Connection: ${input.connection}`,
    `Firmware: ${input.firmware ?? "Unknown"}`,
    `Battery: ${input.batteryPercent !== null ? `${input.batteryPercent}%` : "Unknown"}`,
    "--- Checks ---",
    ...sanitizedChecks.map((sc, i) => `${i + 1}. ${sc}`),
    `chk-usb-a: ${input.checks.find((c) => c.id === "chk-usb-a")?.status.toUpperCase() ?? "UNKNOWN"}`,
    "==================================",
  ].join("\n");
}

const MAX_ISSUE_REPORT_CHARS = 3000;

export interface GitHubIssueInput {
  title: string;
  /** Sanitized report text to embed in the issue body. */
  report: string;
  /** Short human label of the failed action, e.g. "RGB preview". */
  actionLabel?: string;
}

/**
 * Build a prefilled `issues/new` URL: title + short how-to + sanitized report.
 * Long reports are truncated in the URL (the user pastes the full clipboard text).
 */
export function buildGitHubIssueUrl(input: GitHubIssueInput): string {
  const trimmed =
    input.report.length > MAX_ISSUE_REPORT_CHARS
      ? `${input.report.slice(0, MAX_ISSUE_REPORT_CHARS)}\n...(truncated here - paste the full copied report instead)`
      : input.report;
  const body = [
    "### What happened",
    `"${input.actionLabel ?? "Desktop action"}" failed in monkey-desktop.`,
    "",
    "### Steps to reproduce",
    "1. Connect the keyboard via USB",
    "2. Retry the action",
    "3. Paste the copied report under 'Error report' below",
    "",
    "### Environment",
    `- App: ${APP_NAME} ${APP_VERSION}`,
    "- OS: (e.g. macOS 15, Windows 11, Ubuntu 24.04)",
    "- Keyboard: Monka 3075 Pro (RKGK890)",
    "",
    "### Error report (sanitized - safe to share)",
    "```text",
    trimmed,
    "```",
  ].join("\n");
  const query = new URLSearchParams({ title: input.title, body });
  return `${GITHUB_ISSUES_NEW_URL}?${query.toString()}`;
}
