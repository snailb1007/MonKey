import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { DeviceProvider } from "../../src/context/DeviceContext";
import { LightingPage } from "../../src/pages/LightingPage";
import type { DesktopBridge } from "../../src/services/bridge";
import type {
  DeviceSnapshot,
  DesktopResponse,
  Action,
  ActionPayloadMap,
  LightingConfigDto,
} from "../../src/types/ipc";

const mockDeviceWithConfig: DeviceSnapshot = {
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
  batteryPercent: 80,
  firmware: "v1.0.0",
};

const initialLightingConfig: LightingConfigDto = {
  mode: "wave",
  color: { r: 255, g: 0, b: 128 },
  brightness: 80,
  speed: 50,
  direction: "left_to_right",
};

function createMockLightingBridge(): DesktopBridge {
  let currentConfig: LightingConfigDto = { ...initialLightingConfig };
  let currentPersistence = "verified";

  const sendFn = vi.fn(async (action: Action, _deviceId: string | null, payload: unknown) => {
    if (action === "list_devices") {
      return {
        version: 1,
        requestId: "req-list",
        ok: true,
        data: { devices: [mockDeviceWithConfig] },
      };
    }
    if (action === "probe") {
      return {
        version: 1,
        requestId: "req-probe",
        ok: true,
        data: { device: mockDeviceWithConfig },
      };
    }
    if (action === "rgb_status") {
      return {
        version: 1,
        requestId: "req-rgb-status",
        ok: true,
        data: {
          config: currentConfig,
          persistence: currentPersistence,
          supportedModes: [
            "off",
            "static",
            "breathing",
            "wave",
            "rainbow",
            "ripple",
            "reactive",
          ],
        },
      };
    }
    if (action === "rgb_preview") {
      const p = payload as { config: LightingConfigDto };
      currentConfig = { ...p.config };
      currentPersistence = "ram";
      return {
        version: 1,
        requestId: "req-rgb-prev",
        ok: true,
        data: {
          config: currentConfig,
          persistence: "ram",
        },
      };
    }
    if (action === "rgb_commit") {
      currentPersistence = "verified";
      return {
        version: 1,
        requestId: "req-rgb-commit",
        ok: true,
        data: {
          persistence: "verified",
        },
      };
    }
    if (action === "export_profile") {
      return {
        version: 1,
        requestId: "req-export",
        ok: true,
        data: {
          profileJson: JSON.stringify({
            version: 1,
            config: currentConfig,
          }),
        },
      };
    }
    if (action === "import_profile") {
      const p = payload as { profileJson: string };
      const parsed = JSON.parse(p.profileJson);
      return {
        version: 1,
        requestId: "req-import",
        ok: true,
        data: {
          config: parsed.config,
        },
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

describe("LightingPage Workflow and Safety Rails", () => {
  it("fetches rgb status on mount and populates modes and sliders", async () => {
    const bridge = createMockLightingBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <LightingPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(bridge.send).toHaveBeenCalledWith("rgb_status", "dev-mock-0", {});
    });

    const modeSelect = screen.getByRole("combobox", { name: /Lighting Mode/i });
    expect(modeSelect).toHaveValue("wave");

    const brightnessSlider = screen.getByRole("slider", { name: /Brightness/i });
    expect(brightnessSlider).toHaveValue("80");
  });

  it("updates RAM preview and marks unsaved changes without committing to flash", async () => {
    const bridge = createMockLightingBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <LightingPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/RGB Lighting Configuration/i)).toBeInTheDocument();
    });

    // Change brightness slider
    const brightnessSlider = screen.getByRole("slider", { name: /Brightness/i });
    fireEvent.change(brightnessSlider, { target: { value: "45" } });

    // Click Preview in RAM button
    const previewBtn = screen.getByRole("button", { name: /Preview in RAM/i });
    fireEvent.click(previewBtn);

    await waitFor(() => {
      expect(bridge.send).toHaveBeenCalledWith("rgb_preview", "dev-mock-0", {
        config: expect.objectContaining({
          brightness: 45,
          mode: "wave",
        }),
      });
    });

    // Persistence should be indicated as RAM (unsaved)
    expect(screen.getByText(/RAM Preview \(Unsaved\)/i)).toBeInTheDocument();
    // rgb_commit must NOT have been called
    expect(bridge.send).not.toHaveBeenCalledWith("rgb_commit", expect.anything(), expect.anything());
  });

  it("requires explicit user confirmation before committing to flash memory", async () => {
    const bridge = createMockLightingBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <LightingPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/RGB Lighting Configuration/i)).toBeInTheDocument();
    });

    const commitBtn = screen.getByRole("button", { name: /Save to Keyboard Flash/i });
    fireEvent.click(commitBtn);

    await waitFor(() => {
      expect(bridge.send).toHaveBeenCalledWith("rgb_commit", "dev-mock-0", {});
    });

    expect(screen.getByText(/Verified in Flash/i)).toBeInTheDocument();
  });

  it("handles low battery persistence block accurately", async () => {
    const lowBatteryDevice: DeviceSnapshot = {
      ...mockDeviceWithConfig,
      connection: "wireless",
      batteryPercent: 12,
    };

    const sendFn = vi.fn(async (action: Action) => {
      if (action === "list_devices") {
        return {
          version: 1,
          requestId: "req-list",
          ok: true,
          data: { devices: [lowBatteryDevice] },
        };
      }
      if (action === "probe") {
        return {
          version: 1,
          requestId: "req-probe",
          ok: true,
          data: { device: lowBatteryDevice },
        };
      }
      if (action === "rgb_status") {
        return {
          version: 1,
          requestId: "req-rgb-status",
          ok: true,
          data: {
            config: initialLightingConfig,
            persistence: "ram",
            supportedModes: ["off", "static", "wave"],
          },
        };
      }
      if (action === "rgb_commit") {
        return {
          version: 1,
          requestId: "req-commit-err",
          ok: false,
          error: {
            code: "low_battery",
            message: "Persistence is blocked below 20% battery on wireless",
            retryable: false,
          },
        };
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
        <LightingPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/RGB Lighting Configuration/i)).toBeInTheDocument();
    });

    const commitBtn = screen.getByRole("button", { name: /Save to Keyboard Flash/i });
    fireEvent.click(commitBtn);

    await waitFor(() => {
      expect(
        screen.getByText(/Persistence is blocked below 20% battery on wireless/i)
      ).toBeInTheDocument();
    });

    // Should NOT pretend save succeeded
    expect(screen.queryByText(/Verified in Flash/i)).not.toBeInTheDocument();
  });
});
