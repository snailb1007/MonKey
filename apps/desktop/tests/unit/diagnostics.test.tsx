import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { DeviceProvider } from "../../src/context/DeviceContext";
import { DiagnosticsPage } from "../../src/pages/DiagnosticsPage";
import type { DesktopBridge } from "../../src/services/bridge";
import type {
  DeviceSnapshot,
  DesktopResponse,
  Action,
  ActionPayloadMap,
  DiagnosticCheck,
} from "../../src/types/ipc";

const mockConnectedDevice: DeviceSnapshot = {
  id: "dev-mock-0",
  label: "Monka 3075 Pro / RKGK890",
  connection: "wired",
  interfaces: {
    lcd: true,
    config: true,
  },
  capabilities: {
    lcd: true,
    rgb: true,
    persist: true,
    remap: false,
    macro: false,
    firmware: false,
  },
  batteryPercent: 90,
  firmware: "v1.0.0",
};

const mockChecks: DiagnosticCheck[] = [
  {
    id: "chk-usb-a",
    label: "Interface A (LCD Display Bulk Pipe)",
    status: "pass",
    detail: "Usage Page 0xFF68 Usage 0x0061 accessible without permissions",
    remediation: null,
  },
  {
    id: "chk-usb-b",
    label: "Interface B (Config & Feature Reports)",
    status: "warn",
    detail: "Shared device handle requires macos-shared-device entitlement",
    remediation: "Ensure keyboard is connected via USB cable or grant Input Monitoring if prompted",
  },
  {
    id: "chk-battery",
    label: "Battery & Power Stability",
    status: "pass",
    detail: "Battery reporting 90% (wired charging)",
    remediation: null,
  },
  {
    id: "chk-flash",
    label: "Flash Persistence Safety Gate",
    status: "pass",
    detail: "Hardware safety rails active: RAM preview throttled, flash commit gated",
    remediation: null,
  },
];

function createMockDiagBridge(): DesktopBridge {
  const sendFn = vi.fn(async (action: Action, _deviceId: string | null, _payload: unknown) => {
    if (action === "list_devices") {
      return {
        version: 1,
        requestId: "req-list",
        ok: true,
        data: { devices: [mockConnectedDevice] },
      };
    }
    if (action === "probe") {
      return {
        version: 1,
        requestId: "req-probe",
        ok: true,
        data: { device: mockConnectedDevice },
      };
    }
    if (action === "diagnostics") {
      return {
        version: 1,
        requestId: "req-diag",
        ok: true,
        data: { checks: mockChecks },
      };
    }
    return {
      version: 1,
      requestId: "req-unknown",
      ok: false,
      error: { code: "internal", message: "unhandled", retryable: false },
    };
  });

  return {
    send: sendFn as unknown as <A extends Action>(
      action: A,
      deviceId: string | null,
      payload: ActionPayloadMap[A]
    ) => Promise<DesktopResponse<A>>,
  };
}

describe("DiagnosticsPage Verification and Sanitization", () => {
  it("runs diagnostic doctor and renders grouped check statuses", async () => {
    const bridge = createMockDiagBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <DiagnosticsPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(bridge.send).toHaveBeenCalledWith("diagnostics", "dev-mock-0", {});
    });

    expect(screen.getByText(/Interface A \(LCD Display Bulk Pipe\)/i)).toBeInTheDocument();
    expect(screen.getByText(/Shared device handle requires macos-shared-device entitlement/i)).toBeInTheDocument();
    expect(
      screen.getByText(/Ensure keyboard is connected via USB cable or grant Input Monitoring/i)
    ).toBeInTheDocument();
  });

  it("provides sanitized diagnostic report export without raw device paths", async () => {
    const bridge = createMockDiagBridge();

    // Mock clipboard
    let clipboardText = "";
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn((text: string) => {
          clipboardText = text;
          return Promise.resolve();
        }),
      },
    });

    render(
      <DeviceProvider bridge={bridge}>
        <DiagnosticsPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(bridge.send).toHaveBeenCalledWith("diagnostics", "dev-mock-0", {});
    });

    const copyBtn = screen.getByRole("button", { name: /Copy Sanitized Report/i });
    fireEvent.click(copyBtn);

    await waitFor(() => {
      expect(screen.getByText(/Report copied to clipboard/i)).toBeInTheDocument();
    });

    expect(clipboardText).toContain("MonKey Diagnostics Report");
    expect(clipboardText).toContain("chk-usb-a: PASS");
    // Ensure no raw private local disk paths are present in report
    expect(clipboardText).not.toMatch(/\/Volumes\/|\/Users\//);
  });

  it("displays explicit CLI benchmark and parity guidance", async () => {
    const bridge = createMockDiagBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <DiagnosticsPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/Advanced Hardware Benchmarking/i)).toBeInTheDocument();
    });

    expect(screen.getByText(/monkey bench --fps 15/i)).toBeInTheDocument();
    expect(
      screen.getByText(/Desktop GUI intentionally defers raw microsecond benchmark loops/i)
    ).toBeInTheDocument();
  });
});
