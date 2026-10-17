import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { OperationNotice } from "../../src/components/OperationNotice";
import { ActionButton } from "../../src/components/ActionButton";

const DETAILS = "=== MonKey Error Report ===\nAction: rgb_preview\nCode: io_error";

function mockClipboard() {
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", {
    value: { writeText },
    configurable: true,
    writable: true,
  });
  return writeText;
}

describe("OperationNotice", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("renders errors as alert with copy, GitHub link, retry and dismiss", async () => {
    const writeText = mockClipboard();
    const onRetry = vi.fn();
    const onDismiss = vi.fn();
    render(
      <OperationNotice
        kind="error"
        title="Lighting Operation Notice"
        message="Bulk write timeout"
        code="io_error"
        details={DETAILS}
        issueTitle="Lighting preview failed: io_error"
        issueAction="Preview in RAM"
        retryable
        onRetry={onRetry}
        onDismiss={onDismiss}
      />
    );

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("Bulk write timeout")).toBeInTheDocument();
    expect(screen.getByText(/Code: io_error/)).toBeInTheDocument();

    const githubLink = screen.getByRole("link", { name: /Report on GitHub/i });
    expect(githubLink.getAttribute("href")).toContain("github.com/snailb1007/MonKey/issues/new");
    expect(githubLink.getAttribute("href")).toContain("Action%3A+rgb_preview");

    fireEvent.click(screen.getByRole("button", { name: /Copy details/i }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith(DETAILS));
    expect(await screen.findByText(/Copied to clipboard/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /^Retry$/i }));
    expect(onRetry).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: /Dismiss/i }));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("renders warnings as alert without GitHub link when no details", () => {
    render(
      <OperationNotice
        kind="warning"
        title="LCD Stream Alert"
        message="A streaming job is already active."
        code="busy"
      />
    );
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /Report on GitHub/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Copy details/i })).not.toBeInTheDocument();
  });

  it("renders success as status without retry or GitHub link", () => {
    render(
      <OperationNotice
        kind="success"
        title="Lighting Operation Success"
        message="RAM preview active."
        details="RAM preview active."
      />
    );
    expect(screen.getByRole("status")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^Retry$/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /Report on GitHub/i })).not.toBeInTheDocument();
  });
});

describe("ActionButton", () => {
  it("keeps the base label in the accessible name and exposes aria-busy while pending", () => {
    const { rerender } = render(
      <ActionButton className="pixel-btn" onClick={() => {}}>
        Preview in RAM
      </ActionButton>
    );
    const btn = screen.getByRole("button", { name: /Preview in RAM/i });
    expect(btn).not.toHaveAttribute("aria-busy");
    expect(btn).not.toBeDisabled();

    rerender(
      <ActionButton className="pixel-btn" onClick={() => {}} pending>
        Preview in RAM
      </ActionButton>
    );
    // Base label still matches so existing queries keep working during loading.
    const pendingBtn = screen.getByRole("button", { name: /Preview in RAM/i });
    expect(pendingBtn).toHaveAttribute("aria-busy", "true");
    expect(pendingBtn).toBeDisabled();
  });

  it("does not block clicks when blockWhilePending is false (throttled actions)", () => {
    render(
      <ActionButton className="pixel-btn" onClick={() => {}} pending blockWhilePending={false}>
        Preview in RAM
      </ActionButton>
    );
    const btn = screen.getByRole("button", { name: /Preview in RAM/i });
    expect(btn).toHaveAttribute("aria-busy", "true");
    expect(btn).not.toBeDisabled();
  });
});
