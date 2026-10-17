import { describe, it, expect, vi, afterEach } from "vitest";
import { openUrl } from "@tauri-apps/plugin-opener";
import { openExternalUrl, isTauriRuntime } from "../../src/services/externalUrl";

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn().mockResolvedValue(undefined),
}));

afterEach(() => {
  vi.clearAllMocks();
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe("externalUrl (Tauri runtime)", () => {
  it("routes through the opener plugin, not window.open", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    expect(isTauriRuntime()).toBe(true);
    const openSpy = vi.spyOn(window, "open").mockReturnValue(null);
    await openExternalUrl("https://github.com/snailb1007/MonKey/issues/new?x=1");
    expect(openUrl).toHaveBeenCalledWith("https://github.com/snailb1007/MonKey/issues/new?x=1");
    expect(openSpy).not.toHaveBeenCalled();
  });
});
