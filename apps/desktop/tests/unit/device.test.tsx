import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { DeviceProvider } from "../../src/context/DeviceContext";
import { DevicePage } from "../../src/pages/DevicePage";
import type { DesktopBridge } from "../../src/services/bridge";
import type {
  DeviceSnapshot,
  DesktopResponse,
  DesktopSuccessResponse,
  Action,
  ActionPayloadMap,
} from "../../src/types/ipc";

const mockDevice1: DeviceSnapshot = {
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
  batteryPercent: 95,
  firmware: "v1.0.0",
};

const mockDevice2: DeviceSnapshot = {
  id: "dev-mock-1",
  label: "Monka 3075 Pro (Wireless)",
  connection: "wireless",
  interfaces: {
    lcd: true,
    config: false,
  },
  capabilities: {
    lcd: true,
    rgb: false,
    persist: false,
    remap: false,
    macro: false,
    firmware: false,
  },
  batteryPercent: 15,
  firmware: null,
};

function createMockBridge(): DesktopBridge {
  const sendFn = vi.fn(async (action: Action, _deviceId: string | null, _payload: unknown) => {
    if (action === "list_devices") {
      return {
        version: 1,
        requestId: "req-list",
        ok: true,
        data: { devices: [mockDevice1, mockDevice2] },
      };
    }
    if (action === "probe") {
      return {
        version: 1,
        requestId: "req-probe",
        ok: true,
        data: { device: mockDevice1 },
      };
    }
    return {
      version: 1,
      requestId: "req-unknown",
      ok: false,
      error: { code: "internal", message: "unhandled action", retryable: false },
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

describe("DevicePage and DeviceContext Tracer", () => {
  it("discovers and lists connected devices on mount", async () => {
    const bridge = createMockBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <DevicePage />
      </DeviceProvider>
    );

    expect(screen.getByText(/Scanning for devices/i)).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText("Monka 3075 Pro / RKGK890")).toBeInTheDocument();
    });

    expect(screen.getByText("Monka 3075 Pro (Wireless)")).toBeInTheDocument();
    expect(bridge.send).toHaveBeenCalledWith("list_devices", null, {});
  });

  it("probes selected device and displays interfaces, battery and firmware", async () => {
    const bridge = createMockBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <DevicePage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText("Monka 3075 Pro / RKGK890")).toBeInTheDocument();
    });

    expect(screen.getByText(/95%/i)).toBeInTheDocument();
    expect(screen.getByText(/Firmware: v1.0.0/i)).toBeInTheDocument();
    expect(screen.getByText(/Interface A \(LCD 0xFF68\):/i)).toBeInTheDocument();
    expect(screen.getByText(/Interface B \(Config 0xFFFF\):/i)).toBeInTheDocument();
  });

  it("displays truthful disabled capabilities and reasons", async () => {
    const bridge = createMockBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <DevicePage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText("Monka 3075 Pro / RKGK890")).toBeInTheDocument();
    });

    expect(screen.getByText(/Key Remap: Unsupported/i)).toBeInTheDocument();
    expect(screen.getByText(/Macro Engine: Unsupported/i)).toBeInTheDocument();
    expect(screen.getByText(/Firmware Flasher: Unsupported/i)).toBeInTheDocument();
  });

  it("prevents stale probe responses when switching devices quickly", async () => {
    let resolveProbe0!: (val: DesktopSuccessResponse<"probe">) => void;
    const probe0Promise = new Promise<DesktopSuccessResponse<"probe">>((resolve) => {
      resolveProbe0 = resolve;
    });

    const sendFn = vi.fn(async (action: Action, deviceId: string | null, _payload: unknown) => {
      if (action === "list_devices") {
        return {
          version: 1,
          requestId: "req-list",
          ok: true,
          data: { devices: [mockDevice1, mockDevice2] },
        };
      }
      if (action === "probe") {
        if (deviceId === "dev-mock-0") {
          return probe0Promise;
        }
        if (deviceId === "dev-mock-1") {
          return {
            version: 1,
            requestId: "req-probe-1",
            ok: true,
            data: { device: mockDevice2 },
          };
        }
      }
      return {
        version: 1,
        requestId: "req-unknown",
        ok: false,
        error: { code: "internal", message: "unhandled", retryable: false },
      };
    });

    const bridge: DesktopBridge = {
      send: sendFn as unknown as <A extends Action>(
        action: A,
        deviceId: string | null,
        payload: ActionPayloadMap[A]
      ) => Promise<DesktopResponse<A>>,
    };

    render(
      <DeviceProvider bridge={bridge}>
        <DevicePage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText("Monka 3075 Pro (Wireless)")).toBeInTheDocument();
    });

    const dev2Button = screen.getByRole("button", {
      name: /Select Monka 3075 Pro \(Wireless\)/i,
    });
    fireEvent.click(dev2Button);

    await waitFor(() => {
      expect(screen.getByText(/15%/i)).toBeInTheDocument();
    });

    // Now resolve probe for device 0 (stale)
    resolveProbe0({
      version: 1,
      requestId: "req-probe-0-stale",
      ok: true,
      data: { device: mockDevice1 },
    });

    await new Promise((r) => setTimeout(r, 50));
    expect(screen.getByText(/15%/i)).toBeInTheDocument();
  });

  it("handles empty device list gracefully with reconnection guidance", async () => {
    const sendFn = vi.fn(async (action: Action) => {
      if (action === "list_devices") {
        return {
          version: 1,
          requestId: "req-empty",
          ok: true,
          data: { devices: [] },
        };
      }
      return {
        version: 1,
        requestId: "req-unknown",
        ok: false,
        error: { code: "not_connected", message: "No keyboard connected", retryable: false },
      };
    });

    const bridge: DesktopBridge = {
      send: sendFn as unknown as <A extends Action>(
        action: A,
        deviceId: string | null,
        payload: ActionPayloadMap[A]
      ) => Promise<DesktopResponse<A>>,
    };

    render(
      <DeviceProvider bridge={bridge}>
        <DevicePage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/No compatible keyboards detected/i)).toBeInTheDocument();
    });

    expect(
      screen.getByText(/Please connect your Monka 3075 Pro via USB cable/i)
    ).toBeInTheDocument();
  });
});
