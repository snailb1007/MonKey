import { describe, it, expect } from "vitest";
import {
  GITHUB_ISSUES_NEW_URL,
  buildDiagnosticsReport,
  buildErrorReport,
  buildGitHubIssueUrl,
  sanitizeReportText,
} from "../../src/services/issueReport";

describe("sanitizeReportText", () => {
  it("redacts absolute user and volume paths", () => {
    const out = sanitizeReportText("open /Users/ethan/key.bin and /Volumes/D/fw.bin failed");
    expect(out).not.toMatch(/\/Users\/|\/Volumes\//);
    expect(out).toContain("[REDACTED_PATH]");
  });

  it("redacts device serial assignments", () => {
    const out = sanitizeReportText("probe failed serial=ABC123XYZ");
    expect(out).not.toContain("ABC123XYZ");
    expect(out).toContain("serial=[REDACTED]");
  });

  it("redacts Windows and macOS private paths", () => {
    const out = sanitizeReportText(
      "failed meantime C:\\Users\\ethan\\key.bin and C:/Program Files/MonKey/fw.bin and /private/var/folders/xy/T/tmp.bin and \\\\NAS\\share\\k.bin"
    );
    expect(out).not.toMatch(/[A-Za-z]:[\\/]|\\\\NAS|\/private\//);
    expect(out).toContain("[REDACTED_PATH]");
  });

  it("redacts whole quoted paths containing spaces", () => {
    const out = sanitizeReportText(
      'open "/Users/ethan/My Keyboards/config.json" failed; try \'C:\\Users\\ethan\\My Keys\\k.bin\' next'
    );
    expect(out).not.toMatch(/Keyboards|config\.json|My Keys|k\.bin|ethan/);
    expect(out).toContain("[REDACTED_PATH]");
  });

  it("leaves clean text untouched", () => {
    expect(sanitizeReportText("rgb_preview ok")).toBe("rgb_preview ok");
  });
});

describe("buildErrorReport", () => {
  it("contains action, code, message and app stamp", () => {
    const report = buildErrorReport({
      action: "rgb_preview",
      deviceLabel: "Monka 3075 Pro",
      code: "io_error",
      message: "Bulk write timeout",
      extra: "jobId=abc framesSent=3",
    });
    expect(report).toContain("Action: rgb_preview");
    expect(report).toContain("Code: io_error");
    expect(report).toContain("Bulk write timeout");
    expect(report).toContain("jobId=abc framesSent=3");
    expect(report).toContain("monkey-desktop");
  });
});

describe("buildDiagnosticsReport", () => {
  it("keeps the stable report format with sanitized details", () => {
    const report = buildDiagnosticsReport({
      deviceLabel: "Monka 3075 Pro",
      connection: "wired",
      firmware: "v1.0.0",
      batteryPercent: 80,
      checks: [
        {
          id: "chk-usb-a",
          label: "USB Interface A",
          status: "pass",
          detail: "Bulk pipe open at /Users/ethan/hid serial=ZZ9",
          remediation: null,
        },
      ],
    });
    expect(report).toContain("MonKey Diagnostics Report");
    expect(report).toContain("chk-usb-a: PASS");
    expect(report).not.toMatch(/\/Users\//);
    expect(report).not.toContain("serial=ZZ9");
  });
});

describe("buildGitHubIssueUrl", () => {
  it("points at issues/new with encoded title and report body", () => {
    const url = buildGitHubIssueUrl({
      title: "Lighting preview failed: io_error",
      report: "=== MonKey Error Report ===\nAction: rgb_preview",
      actionLabel: "Preview in RAM",
    });
    expect(url.startsWith(`${GITHUB_ISSUES_NEW_URL}?`)).toBe(true);
    const parsed = new URL(url);
    expect(parsed.searchParams.get("title")).toBe("Lighting preview failed: io_error");
    expect(parsed.searchParams.get("body")).toContain("Action: rgb_preview");
    expect(parsed.searchParams.get("body")).toContain("monkey-desktop");
  });

  it("truncates very long reports in the URL", () => {
    const url = buildGitHubIssueUrl({
      title: "x",
      report: "A".repeat(5000),
      actionLabel: "y",
    });
    const body = new URL(url).searchParams.get("body") ?? "";
    expect(body).toContain("truncated here");
    expect(body.length).toBeLessThan(5000);
  });
});
