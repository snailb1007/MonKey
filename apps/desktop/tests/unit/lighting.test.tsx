import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor, fireEvent, act } from "@testing-library/react";
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

    // Hardware sends stay locked until the first rgb_status round-trip resolves.
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Preview in RAM/i })).not.toBeDisabled();
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

    // Hardware sends stay locked until the first rgb_status round-trip resolves.
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Preview in RAM/i })).not.toBeDisabled();
    });

    // Save to Keyboard Flash requires preview to be accepted first
    const previewBtn = screen.getByRole("button", { name: /Preview in RAM/i });
    fireEvent.click(previewBtn);

    await waitFor(() => {
      expect(screen.getByText(/RAM preview active/i)).toBeInTheDocument();
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
      if (action === "rgb_preview") {
        return {
          version: 1,
          requestId: "req-rgb-preview",
          ok: true,
          data: {
            config: initialLightingConfig,
            persistence: "ram",
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

    // Hardware sends stay locked until the first rgb_status round-trip resolves.
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Preview in RAM/i })).not.toBeDisabled();
    });

    const previewBtn = screen.getByRole("button", { name: /Preview in RAM/i });
    fireEvent.click(previewBtn);

    await waitFor(() => {
      expect(screen.getByText(/RAM preview active/i)).toBeInTheDocument();
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

  it("does not default null direction to left_to_right and rejects preview when direction is null", async () => {
    const configWithNullDir: LightingConfigDto = {
      ...initialLightingConfig,
      direction: null,
    };

    const sendFn = vi.fn(async (action: Action) => {
      if (action === "list_devices") {
        return { version: 1, requestId: "r1", ok: true, data: { devices: [mockDeviceWithConfig] } };
      }
      if (action === "probe") {
        return { version: 1, requestId: "r2", ok: true, data: { device: mockDeviceWithConfig } };
      }
      if (action === "rgb_status") {
        return {
          version: 1,
          requestId: "r3",
          ok: true,
          data: {
            config: configWithNullDir,
            persistence: "unknown",
            supportedModes: ["wave", "static"],
          },
        };
      }
      return { version: 1, requestId: "r4", ok: false, error: { code: "internal", message: "fail", retryable: false } };
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

    // Hardware sends stay locked until the first rgb_status round-trip resolves.
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Preview in RAM/i })).not.toBeDisabled();
    });

    // Check direction select value is empty / unselected, not defaulted to left_to_right
    const dirSelect = screen.getByRole("combobox", { name: /Animation Direction/i });
    expect((dirSelect as HTMLSelectElement).value).toBe("");

    // Attempt preview without setting direction
    const previewBtn = screen.getByRole("button", { name: /Preview in RAM/i });
    fireEvent.click(previewBtn);

    await waitFor(() => {
      expect(screen.getByText(/Direction cannot be null/i)).toBeInTheDocument();
    });

    // Ensure bridge was NOT called with implicit left_to_right
    expect(sendFn).not.toHaveBeenCalledWith("rgb_preview", expect.anything(), expect.anything());
  });

  it("only advertises capability-approved modes returned by backend", async () => {
    const sendFn = vi.fn(async (action: Action) => {
      if (action === "list_devices") {
        return { version: 1, requestId: "r1", ok: true, data: { devices: [mockDeviceWithConfig] } };
      }
      if (action === "probe") {
        return { version: 1, requestId: "r2", ok: true, data: { device: mockDeviceWithConfig } };
      }
      if (action === "rgb_status") {
        return {
          version: 1,
          requestId: "r3",
          ok: true,
          data: {
            config: initialLightingConfig,
            persistence: "verified",
            supportedModes: ["static", "breathing"], // Only 2 approved modes
          },
        };
      }
      return { version: 1, requestId: "r4", ok: false, error: { code: "internal", message: "fail", retryable: false } };
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

    // Hardware sends stay locked until the first rgb_status round-trip resolves.
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Preview in RAM/i })).not.toBeDisabled();
    });

    const modeSelect = screen.getByRole("combobox", { name: /Lighting Mode/i });
    const options = Array.from(modeSelect.querySelectorAll("option")).map((o) => o.value);
    expect(options).toEqual(["static", "breathing"]);
    expect(options).not.toContain("wave");
    expect(options).not.toContain("rainbow");
  });

  it("enforces RAM preview throttling capped at <=30Hz (>=33ms between sends)", async () => {
    let now = 1000;
    vi.spyOn(Date, "now").mockImplementation(() => now);
    vi.useFakeTimers();

    const sendFn = vi.fn(async (action: Action) => {
      if (action === "list_devices") {
        return { version: 1, requestId: "r1", ok: true, data: { devices: [mockDeviceWithConfig] } };
      }
      if (action === "probe") {
        return { version: 1, requestId: "r2", ok: true, data: { device: mockDeviceWithConfig } };
      }
      if (action === "rgb_status") {
        return {
          version: 1,
          requestId: "r3",
          ok: true,
          data: {
            config: initialLightingConfig,
            persistence: "ram",
            supportedModes: ["wave"],
          },
        };
      }
      if (action === "rgb_preview") {
        return {
          version: 1,
          requestId: "r-prev",
          ok: true,
          data: { config: initialLightingConfig, persistence: "ram" },
        };
      }
      return { version: 1, requestId: "r-err", ok: false, error: { code: "internal", message: "err", retryable: false } };
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

    await vi.runOnlyPendingTimersAsync();

    const previewBtn = screen.getByRole("button", { name: /Preview in RAM/i });

    // Rapidly trigger 5 preview requests within 10ms
    for (let i = 0; i < 5; i++) {
      fireEvent.click(previewBtn);
      now += 2;
      vi.advanceTimersByTime(2);
    }

    // Immediately, at most 1 call should have fired
    const immediatePreviewCalls = sendFn.mock.calls.filter((c) => c[0] === "rgb_preview");
    expect(immediatePreviewCalls.length).toBe(1);

    // Advance time by 35ms (surpassing the 34ms throttle interval for <=30Hz)
    now += 35;
    vi.advanceTimersByTime(35);
    await vi.runOnlyPendingTimersAsync();

    // Now trailing call should have fired
    const throttledCalls = sendFn.mock.calls.filter((c) => c[0] === "rgb_preview");
    expect(throttledCalls.length).toBe(2);

    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("requires accepted RAM preview before enabling Save to Keyboard Flash", async () => {
    const bridge = createMockLightingBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <LightingPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/RGB Lighting Configuration/i)).toBeInTheDocument();
    });

    // Hardware sends stay locked until the first rgb_status round-trip resolves.
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Preview in RAM/i })).not.toBeDisabled();
    });

    const commitBtn = screen.getByRole("button", { name: /Save to Keyboard Flash/i });
    // Initially disabled because config hasn't been previewed and accepted in RAM yet
    expect(commitBtn).toBeDisabled();

    // Now click Preview in RAM
    const previewBtn = screen.getByRole("button", { name: /Preview in RAM/i });
    fireEvent.click(previewBtn);

    await waitFor(() => {
      expect(screen.getByText(/RAM preview active/i)).toBeInTheDocument();
    });

    // Commit button should now be enabled
    expect(commitBtn).not.toBeDisabled();
  });

  it("locks preview until rgb status loads and retries a failed status fetch", async () => {
    let statusCalls = 0;
    let releaseFirstStatus!: (resp: unknown) => void;
    const firstStatusGate = new Promise((resolve) => {
      releaseFirstStatus = resolve as (resp: unknown) => void;
    });

    const sendFn = vi.fn(async (action: Action) => {
      if (action === "list_devices") {
        return { version: 1, requestId: "r1", ok: true, data: { devices: [mockDeviceWithConfig] } };
      }
      if (action === "probe") {
        return { version: 1, requestId: "r2", ok: true, data: { device: mockDeviceWithConfig } };
      }
      if (action === "rgb_status") {
        statusCalls += 1;
        if (statusCalls === 1) {
          return firstStatusGate;
        }
        return {
          version: 1,
          requestId: "r3",
          ok: true,
          data: {
            config: initialLightingConfig,
            persistence: "ram",
            supportedModes: ["wave"],
          },
        };
      }
      if (action === "rgb_preview") {
        return {
          version: 1,
          requestId: "r-prev",
          ok: true,
          data: { config: initialLightingConfig, persistence: "ram" },
        };
      }
      return { version: 1, requestId: "r-err", ok: false, error: { code: "internal", message: "err", retryable: false } };
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

    // Status still in flight: preview locked, early click sends nothing.
    expect(screen.getByRole("button", { name: /Preview in RAM/i })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: /Preview in RAM/i }));
    expect(sendFn).not.toHaveBeenCalledWith("rgb_preview", expect.anything(), expect.anything());

    // First fetch fails with a retryable Busy (e.g. worker streaming LCD).
    await act(async () => {
      releaseFirstStatus({
        version: 1,
        requestId: "r3",
        ok: false,
        error: { code: "busy", message: "Worker busy with LCD streaming", retryable: true },
      });
    });

    await waitFor(() => {
      expect(screen.getByText(/Worker busy with LCD streaming/i)).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: /Preview in RAM/i })).toBeDisabled();

    // Retry refetches status; preview unlocks afterwards.
    fireEvent.click(screen.getByRole("button", { name: /^Retry$/i }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Preview in RAM/i })).not.toBeDisabled();
    });
    expect(statusCalls).toBe(2);

    // Preview now sends device values (brightness 80), never local defaults (100).
    fireEvent.click(screen.getByRole("button", { name: /Preview in RAM/i }));
    await waitFor(() => {
      expect(sendFn).toHaveBeenCalledWith(
        "rgb_preview",
        "dev-mock-0",
        { config: expect.objectContaining({ brightness: 80 }) }
      );
    });
  });

  it("disables irrelevant lighting parameter controls based on selected mode (off, static, rainbow)", async () => {
    const bridge = createMockLightingBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <LightingPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByLabelText(/Lighting Mode/i)).not.toBeDisabled();
    });

    const modeSelect = screen.getByLabelText(/Lighting Mode/i);
    const brightnessSlider = screen.getByLabelText(/Brightness:/i);
    const speedSlider = screen.getByLabelText(/Speed:/i);
    const directionSelect = screen.getByLabelText(/Animation Direction/i);
    const colorR = screen.getByLabelText(/R:/i);
    const colorG = screen.getByLabelText(/G:/i);
    const colorB = screen.getByLabelText(/B:/i);

    // Initial mode is "wave" -> all controls enabled
    expect(brightnessSlider).not.toBeDisabled();
    expect(speedSlider).not.toBeDisabled();
    expect(directionSelect).not.toBeDisabled();
    expect(colorR).not.toBeDisabled();
    expect(colorG).not.toBeDisabled();
    expect(colorB).not.toBeDisabled();

    // Switch to "off" -> brightness, speed, direction, and RGB color inputs must be disabled
    fireEvent.change(modeSelect, { target: { value: "off" } });
    expect(brightnessSlider).toBeDisabled();
    expect(speedSlider).toBeDisabled();
    expect(directionSelect).toBeDisabled();
    expect(colorR).toBeDisabled();
    expect(colorG).toBeDisabled();
    expect(colorB).toBeDisabled();

    // Clear direction and preview in "off" mode -> must succeed without direction validation error
    fireEvent.click(screen.getByRole("button", { name: /Preview in RAM/i }));
    await waitFor(() => {
      expect(bridge.send).toHaveBeenCalledWith(
        "rgb_preview",
        "dev-mock-0",
        expect.objectContaining({
          config: expect.objectContaining({ mode: "off" }),
        })
      );
    });

    // Switch to "static" -> brightness & color enabled, speed & direction disabled
    fireEvent.change(modeSelect, { target: { value: "static" } });
    expect(brightnessSlider).not.toBeDisabled();
    expect(colorR).not.toBeDisabled();
    expect(colorG).not.toBeDisabled();
    expect(colorB).not.toBeDisabled();
    expect(speedSlider).toBeDisabled();
    expect(directionSelect).toBeDisabled();

    // Switch to "rainbow" -> brightness, speed, direction enabled; RGB color disabled
    fireEvent.change(modeSelect, { target: { value: "rainbow" } });
    expect(brightnessSlider).not.toBeDisabled();
    expect(speedSlider).not.toBeDisabled();
    expect(directionSelect).not.toBeDisabled();
    expect(colorR).toBeDisabled();
    expect(colorG).toBeDisabled();
    expect(colorB).toBeDisabled();
  });
});
