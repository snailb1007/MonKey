import type { MouseEvent } from "react";

/** True when running inside the Tauri webview (packaged app or `tauri dev`). */
export function isTauriRuntime(): boolean {
  return (
    typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}

/**
 * Open an https URL in the system browser.
 *
 * The packaged app has no new-window handler, so a bare target="_blank"
 * anchor goes nowhere — route through the opener plugin under Tauri and fall
 * back to window.open on web.
 */
export async function openExternalUrl(url: string): Promise<void> {
  if (!/^https:\/\//i.test(url)) {
    throw new Error("refusing to open non-https URL");
  }
  if (isTauriRuntime()) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
    return;
  }
  window.open(url, "_blank", "noopener,noreferrer");
}

/**
 * Click handler for external anchors (e.g. prefilled GitHub issue links).
 * Keeps href semantics for a11y/tests while routing the open correctly.
 */
export function handleExternalLinkClick(e: MouseEvent<HTMLAnchorElement>): void {
  const href = e.currentTarget.getAttribute("href");
  if (!href) {
    return;
  }
  e.preventDefault();
  void openExternalUrl(href).catch(() => {
    window.open(href, "_blank", "noopener,noreferrer");
  });
}
