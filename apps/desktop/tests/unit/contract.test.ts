import { describe, it, expect, vi, beforeEach } from "vitest";
import contractFixture from "../../contracts/ipc-v1.json";
import {
  type Action,
  type DesktopRequest,
  type DesktopResponse,
  type ErrorCode,
} from "../../src/types/ipc";
import {
  createBridge,
  validateDesktopRequest,
  type IpcTransport,
} from "../../src/services/bridge";

describe("IPC v1 Contract Fixture Conformance", () => {
  it("matches locked supported lighting modes and patterns", () => {
    expect(contractFixture.version).toBe(1);
    expect(contractFixture.supportedModes).toEqual([
      "off",
      "static",
      "breathing",
      "wave",
      "rainbow",
      "ripple",
      "reactive",
    ]);
    expect(contractFixture.supportedPatterns).toEqual([
      "red",
      "green",
      "blue",
      "white",
      "black",
      "grayscale",
      "color_bars",
      "geometry",
    ]);
  });

  it("contains all 12 defined actions with valid request and success shapes", () => {
    const expectedActions: Action[] = [
      "list_devices",
      "probe",
      "diagnostics",
      "pick_asset",
      "start_lcd",
      "stop_lcd",
      "job_status",
      "rgb_status",
      "rgb_preview",
      "rgb_commit",
      "export_profile",
      "import_profile",
    ];

    const fixtureActions = contractFixture.actions.map((a) => a.action);
    expect(fixtureActions).toEqual(expectedActions);

    for (const item of contractFixture.actions) {
      expect(item.request.version).toBe(1);
      expect(typeof item.request.requestId).toBe("string");
      expect(item.request.requestId.length).toBeGreaterThan(0);
      expect(item.request.action).toBe(item.action);

      expect(item.successResponse.version).toBe(1);
      expect(item.successResponse.ok).toBe(true);
      expect(item.successResponse.data).toBeDefined();
    }
  });

  it("contains all 10 defined error codes with retryable flags", () => {
    const expectedCodes: ErrorCode[] = [
      "not_connected",
      "permission_denied",
      "unsupported",
      "busy",
      "unsafe_operation",
      "low_battery",
      "invalid_input",
      "io_error",
      "cancelled",
      "internal",
    ];

    const fixtureCodes = contractFixture.errors.map((e) => e.code);
    expect(fixtureCodes).toEqual(expectedCodes);

    for (const err of contractFixture.errors) {
      expect(err.response.version).toBe(1);
      expect(err.response.ok).toBe(false);
      expect(err.response.error.code).toBe(err.code);
      expect(typeof err.response.error.message).toBe("string");
      expect(typeof err.response.error.retryable).toBe("boolean");
    }
  });
});

describe("Desktop IPC Bridge Service", () => {
  let mockTransport: IpcTransport;

  beforeEach(() => {
    mockTransport = {
      invoke: vi.fn(),
    };
  });

  it("enforces desktop_request format with version 1 and unique requestId", async () => {
    const bridge = createBridge(mockTransport);

    const listDevicesFixture = contractFixture.actions.find(
      (a) => a.action === "list_devices"
    )!;

    vi.mocked(mockTransport.invoke).mockImplementationOnce(async (_cmd, args) => {
      const req = (args as { request: DesktopRequest<"list_devices"> }).request;
      return {
        ...listDevicesFixture.successResponse,
        requestId: req.requestId,
      } as DesktopResponse<"list_devices">;
    });

    const result = await bridge.send("list_devices", null, {});

    expect(mockTransport.invoke).toHaveBeenCalledTimes(1);
    const [command, payload] = vi.mocked(mockTransport.invoke).mock.calls[0];
    expect(command).toBe("desktop_request");

    const req = (payload as { request: DesktopRequest<"list_devices"> }).request;
    expect(req.version).toBe(1);
    expect(req.action).toBe("list_devices");
    expect(req.deviceId).toBeNull();
    expect(typeof req.requestId).toBe("string");
    expect(req.requestId.length).toBeGreaterThan(4);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.data.devices.length).toBeGreaterThan(0);
    }
  });

  it("correctly surfaces typed error responses from transport", async () => {
    const bridge = createBridge(mockTransport);
    const busyError = contractFixture.errors.find((e) => e.code === "busy")!;

    vi.mocked(mockTransport.invoke).mockImplementationOnce(async (_cmd, args) => {
      const req = (args as { request: DesktopRequest<"start_lcd"> }).request;
      return {
        ...busyError.response,
        requestId: req.requestId,
      } as DesktopResponse<"start_lcd">;
    });

    const result = await bridge.send("start_lcd", "dev-mock-0", {
      assetId: null,
      pattern: "color_bars",
    });

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.code).toBe("busy");
      expect(result.error.retryable).toBe(true);
    }
  });

  it("catches unhandled transport exceptions and wraps in internal error", async () => {
    const bridge = createBridge(mockTransport);

    vi.mocked(mockTransport.invoke).mockRejectedValueOnce(
      new Error("Transport channel closed")
    );

    const result = await bridge.send("probe", "dev-mock-0", {});

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.code).toBe("internal");
      expect(result.error.message).toContain("Transport channel closed");
      expect(result.error.retryable).toBe(false);
    }
  });

  it("verifies and rejects all malformedCases from backend fixture contracts/ipc-v1.json", () => {
    const malformedCases = (contractFixture as {
      malformedCases?: Array<{ name: string; rawJson: string; expectedErrorCode: string }>;
    }).malformedCases;

    expect(malformedCases).toBeDefined();
    expect(malformedCases!.length).toBeGreaterThanOrEqual(4);

    for (const testCase of malformedCases!) {
      const parsed = JSON.parse(testCase.rawJson) as Record<string, unknown>;
      const validation = validateDesktopRequest(parsed);
      expect(validation.valid).toBe(false);
      expect(validation.errorCode).toBe(testCase.expectedErrorCode);
    }
  });

  it("rejects responses with mismatched requestId or invalid version", async () => {
    const bridge = createBridge(mockTransport);

    // Mismatched requestId
    vi.mocked(mockTransport.invoke).mockResolvedValueOnce({
      version: 1,
      requestId: "wrong-req-id",
      ok: true,
      data: { devices: [] },
    } as unknown as DesktopResponse<"list_devices">);

    const res1 = await bridge.send("list_devices", null, {});
    expect(res1.ok).toBe(false);
    if (!res1.ok) {
      expect(res1.error.code).toBe("internal");
      expect(res1.error.message).toMatch(/mismatch|invalid/i);
    }

    // Invalid version in response (match requestId to isolate version validation)
    vi.mocked(mockTransport.invoke).mockImplementationOnce(async (_cmd, args) => {
      const req = (args as { request: { requestId: string } })?.request;
      return {
        version: 2,
        requestId: req?.requestId || "matched-req-id",
        ok: true,
        data: { devices: [] },
      } as unknown as DesktopResponse<"list_devices">;
    });

    const res2 = await bridge.send("list_devices", null, {});
    expect(res2.ok).toBe(false);
    if (!res2.ok) {
      expect(res2.error.code).toBe("internal");
      expect(res2.error.message).toMatch(/version or requestId mismatch/i);
    }
  });
});
