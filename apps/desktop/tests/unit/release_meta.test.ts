import { describe, it, expect } from "vitest";
import tauriConfig from "../../src-tauri/tauri.conf.json";
import pkg from "../../package.json";

export interface ReleaseMeta {
  channel: "stable" | "beta";
  tagName: string;
  releaseTitle: string;
  isPrerelease: boolean;
  isLatest: boolean;
}

export function computeReleaseMeta(
  version: string,
  refName: string,
  runNumber: number,
  channelInput?: string
): ReleaseMeta {
  const isStable =
    channelInput === "stable" || (channelInput !== "beta" && refName === "main");
  if (isStable) {
    return {
      channel: "stable",
      tagName: `v${version}`,
      releaseTitle: `MonKey Desktop v${version}`,
      isPrerelease: false,
      isLatest: true,
    };
  }

  return {
    channel: "beta",
    tagName: `v${version}-beta.${runNumber}`,
    releaseTitle: `MonKey Desktop v${version} Preview (Beta #${runNumber})`,
    isPrerelease: true,
    isLatest: false,
  };
}

describe("Release Metadata & Version Alignment", () => {
  it("synchronizes version between package.json and tauri.conf.json", () => {
    expect(tauriConfig.version).toBe(pkg.version);
    expect(tauriConfig.version).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it("computes stable release metadata on main push", () => {
    const meta = computeReleaseMeta("0.1.0", "main", 42);
    expect(meta.channel).toBe("stable");
    expect(meta.tagName).toBe("v0.1.0");
    expect(meta.releaseTitle).toBe("MonKey Desktop v0.1.0");
    expect(meta.isPrerelease).toBe(false);
    expect(meta.isLatest).toBe(true);
  });

  it("computes preview-beta release metadata on dev push", () => {
    const meta = computeReleaseMeta("0.1.0", "dev", 42);
    expect(meta.channel).toBe("beta");
    expect(meta.tagName).toBe("v0.1.0-beta.42");
    expect(meta.releaseTitle).toBe("MonKey Desktop v0.1.0 Preview (Beta #42)");
    expect(meta.isPrerelease).toBe(true);
    expect(meta.isLatest).toBe(false);
  });

  it("supports manual workflow_dispatch override for channel", () => {
    const stableMeta = computeReleaseMeta("0.1.0", "dev", 10, "stable");
    expect(stableMeta.channel).toBe("stable");
    expect(stableMeta.isPrerelease).toBe(false);

    const betaMeta = computeReleaseMeta("0.1.0", "main", 11, "beta");
    expect(betaMeta.channel).toBe("beta");
    expect(betaMeta.isPrerelease).toBe(true);
  });
});
