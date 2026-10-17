import { describe, it, expect, vi, afterEach } from "vitest";
import { openExternalUrl, isTauriRuntime } from "../../src/services/externalUrl";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("externalUrl (web runtime)", () => {
  it("is not a Tauri runtime under jsdom", () => {
    expect(isTauriRuntime()).toBe(false);
  });

  it("falls back to window.open on web", async () => {
    const openSpy = vi.spyOn(window, "open").mockReturnValue(null);
    await openExternalUrl("https://github.com/snailb1007/MonKey/issues/new?x=1");
    expect(openSpy).toHaveBeenCalledWith(
      "https://github.com/snailb1007/MonKey/issues/new?x=1",
      "_blank",
      "noopener,noreferrer"
    );
  });

  it("refuses non-https URLs", async () => {
    await expect(openExternalUrl("file:///etc/passwd")).rejects.toThrow(/non-https/);
    await expect(openExternalUrl("javascript:alert(1)")).rejects.toThrow(/non-https/);
  });
});
