import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor, fireEvent, act } from "@testing-library/react";
import { DeviceProvider, useDevice } from "../../src/context/DeviceContext";
import { LcdPage } from "../../src/pages/LcdPage";
import type { DesktopBridge } from "../../src/services/bridge";
import type {
  DeviceSnapshot,
  DesktopResponse,
  Action,
  ActionPayloadMap,
  AssetPreview,
  JobSnapshot,
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

const mockAsset: AssetPreview = {
  id: "asset-mock-1",
  name: "retro_cat.gif",
  width: 128,
  height: 128,
  frameCount: 16,
  previewDataUrl:
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
};

function createMockLcdBridge(): DesktopBridge {
  let activeJob: JobSnapshot | null = null;

  const sendFn = vi.fn(async (action: Action, _deviceId: string | null, payload: unknown) => {
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
    if (action === "pick_asset") {
      return {
        version: 1,
        requestId: "req-pick",
        ok: true,
        data: { asset: mockAsset },
      };
    }
    if (action === "start_lcd") {
      activeJob = {
        id: "job-101",
        deviceId: "dev-mock-0",
        state: "running",
        framesSent: 1,
        error: null,
      };
      return {
        version: 1,
        requestId: "req-start",
        ok: true,
        data: { jobId: "job-101" },
      };
    }
    if (action === "stop_lcd") {
      if (activeJob) {
        activeJob.state = "cancelled";
      }
      return {
        version: 1,
        requestId: "req-stop",
        ok: true,
        data: { stopped: true },
      };
    }
    if (action === "job_status") {
      const p = payload as { jobId: string };
      return {
        version: 1,
        requestId: "req-status",
        ok: true,
        data: {
          job: activeJob ?? {
            id: p.jobId,
            deviceId: "dev-mock-0",
            state: "completed",
            framesSent: 16,
            error: null,
          },
        },
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

describe("LcdPage Workflow and Safety Rails", () => {
  it("renders pattern selection with all 8 diagnostic patterns", async () => {
    const bridge = createMockLcdBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <LcdPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/LCD Display Management/i)).toBeInTheDocument();
    });

    // Check presence of diagnostic pattern options
    expect(screen.getByRole("option", { name: /Color Bars/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /Geometry/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /Red/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /Green/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /Blue/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /White/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /Black/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /Grayscale/i })).toBeInTheDocument();
  });

  it("chooses an asset and renders actual converted preview with metadata", async () => {
    const bridge = createMockLcdBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <LcdPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/LCD Display Management/i)).toBeInTheDocument();
    });

    const chooseBtn = screen.getByRole("button", { name: /Choose Animation \(GIF\)/i });
    fireEvent.click(chooseBtn);

    await waitFor(() => {
      expect(screen.getByText("retro_cat.gif")).toBeInTheDocument();
    });

    expect(screen.getByText(/128x128 px/i)).toBeInTheDocument();
    expect(screen.getByText(/16 frames/i)).toBeInTheDocument();
    // Confirm preview is labeled as converted local preview, not hardware success
    expect(screen.getByText(/Converted Local Preview/i)).toBeInTheDocument();
  });

  it("starts and stops an LCD streaming job with status polling and cleanup", async () => {
    const bridge = createMockLcdBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <LcdPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/LCD Display Management/i)).toBeInTheDocument();
    });

    // Select pattern color_bars and send
    const patternSelect = screen.getByRole("combobox", { name: "Test Pattern" });
    fireEvent.change(patternSelect, { target: { value: "color_bars" } });

    const sendBtn = screen.getByRole("button", { name: /Send Pattern to Screen/i });
    fireEvent.click(sendBtn);

    await waitFor(() => {
      expect(bridge.send).toHaveBeenCalledWith("start_lcd", "dev-mock-0", {
        assetId: null,
        pattern: "color_bars",
      });
    });

    // Active streaming should show Stop button
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Stop Streaming/i })).toBeInTheDocument();
    });

    const stopBtn = screen.getByRole("button", { name: /Stop Streaming/i });
    fireEvent.click(stopBtn);

    await waitFor(() => {
      expect(bridge.send).toHaveBeenCalledWith("stop_lcd", "dev-mock-0", {
        jobId: "job-101",
      });
    });
  });

  it("disables start button when Interface A is not available on device", async () => {
    const noLcdDevice: DeviceSnapshot = {
      ...mockConnectedDevice,
      interfaces: { lcd: false, config: true },
      capabilities: { ...mockConnectedDevice.capabilities, lcd: false },
    };

    const sendFn = vi.fn(async (action: Action) => {
      if (action === "list_devices") {
        return {
          version: 1,
          requestId: "req-list",
          ok: true,
          data: { devices: [noLcdDevice] },
        };
      }
      if (action === "probe") {
        return {
          version: 1,
          requestId: "req-probe",
          ok: true,
          data: { device: noLcdDevice },
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
        <LcdPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/LCD Display Management/i)).toBeInTheDocument();
    });

    const sendBtn = screen.getByRole("button", { name: /Send Pattern to Screen/i });
    expect(sendBtn).toBeDisabled();
    expect(screen.getByText(/Interface A \(0xFF68\) is inactive on this device/i)).toBeInTheDocument();
  });

  it("prevents duplicate start when a streaming job is already running", async () => {
    const bridge = createMockLcdBridge();

    render(
      <DeviceProvider bridge={bridge}>
        <LcdPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/LCD Display Management/i)).toBeInTheDocument();
    });

    const sendBtn = screen.getByRole("button", { name: /Send Pattern to Screen/i });
    fireEvent.click(sendBtn);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Stop Streaming/i })).toBeInTheDocument();
    });

    // Both start buttons should now be disabled or hidden
    expect(sendBtn).toBeDisabled();
  });

  it("surfaces LCD error when start_lcd fails with busy or io_error", async () => {
    const bridge = createMockLcdBridge();
    vi.mocked(bridge.send).mockImplementation(async (action: Action) => {
      if (action === "list_devices") {
        return { version: 1, requestId: "r1", ok: true, data: { devices: [mockConnectedDevice] } };
      }
      if (action === "probe") {
        return { version: 1, requestId: "r2", ok: true, data: { device: mockConnectedDevice } };
      }
      if (action === "start_lcd") {
        return {
          version: 1,
          requestId: "r3",
          ok: false,
          error: { code: "busy", message: "MCU LCD buffer locked by another process", retryable: true },
        };
      }
      return { version: 1, requestId: "r4", ok: false, error: { code: "internal", message: "fail", retryable: false } };
    });

    render(
      <DeviceProvider bridge={bridge}>
        <LcdPage />
      </DeviceProvider>
    );

    await waitFor(() => {
      expect(screen.getByText(/LCD Display Management/i)).toBeInTheDocument();
    });

    const sendBtn = screen.getByRole("button", { name: /Send Pattern to Screen/i });
    fireEvent.click(sendBtn);

    await waitFor(() => {
      expect(screen.getByText(/MCU LCD buffer locked/i)).toBeInTheDocument();
    });
    // Stop button must not be visible since job never started
    expect(screen.queryByRole("button", { name: /Stop Streaming/i })).not.toBeInTheDocument();
  });

  it("guards against stale job_status polling responses after device switch or stop", async () => {
    vi.useFakeTimers();

    let resolveJobStatus: ((val: unknown) => void) | undefined;
    const delayedJobStatusPromise = new Promise((resolve) => {
      resolveJobStatus = resolve;
    });

    const sendFn = vi.fn(async (action: Action, _deviceId: string | null) => {
      if (action === "list_devices") {
        return { version: 1, requestId: "r1", ok: true, data: { devices: [mockConnectedDevice] } };
      }
      if (action === "probe") {
        return { version: 1, requestId: "r2", ok: true, data: { device: mockConnectedDevice } };
      }
      if (action === "start_lcd") {
        return { version: 1, requestId: "r3", ok: true, data: { jobId: "job-stale-test" } };
      }
      if (action === "stop_lcd") {
        return { version: 1, requestId: "r4", ok: true, data: { stopped: true } };
      }
      if (action === "job_status") {
        // Return unresolved promise first to simulate slow in-flight response
        return delayedJobStatusPromise;
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
        <LcdPage />
      </DeviceProvider>
    );

    // Run pending effects
    await vi.runOnlyPendingTimersAsync();

    // Start LCD
    const sendBtn = screen.getByRole("button", { name: /Send Pattern to Screen/i });
    fireEvent.click(sendBtn);

    await vi.runOnlyPendingTimersAsync();

    // Fast-forward 500ms to trigger job_status poll
    vi.advanceTimersByTime(500);

    // Stop streaming before the delayed response resolves
    const stopBtn = screen.getByRole("button", { name: /Stop Streaming/i });
    fireEvent.click(stopBtn);
    await vi.runOnlyPendingTimersAsync();

    // Now resolve the late/stale job_status response claiming job is still running
    if (resolveJobStatus) {
      resolveJobStatus({
        version: 1,
        requestId: "r-stale",
        ok: true,
        data: {
          job: {
            id: "job-stale-test",
            deviceId: "dev-mock-0",
            state: "running",
            framesSent: 999,
            error: null,
          },
        },
      });
    }

    await vi.runOnlyPendingTimersAsync();

    // Since the job was stopped, the stale "running" status must be discarded
    expect(screen.queryByText(/999 frames/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Stop Streaming/i })).not.toBeInTheDocument();

    vi.useRealTimers();
  });

  it("discards late job_status poll responses after device switch", async () => {
    vi.useFakeTimers();

    let resolveSlowStatus: ((val: unknown) => void) | undefined;
    const slowStatusPromise = new Promise((resolve) => {
      resolveSlowStatus = resolve;
    });

    const mockDeviceB: DeviceSnapshot = {
      ...mockConnectedDevice,
      id: "dev-mock-1",
      label: "Monka 3075 Pro (Second Device)",
    };

    let activeDevice = mockConnectedDevice;

    const sendFn = vi.fn(async (action: Action, _deviceId: string | null) => {
      if (action === "list_devices") {
        return { version: 1, requestId: "r1", ok: true, data: { devices: [mockConnectedDevice, mockDeviceB] } };
      }
      if (action === "probe") {
        return { version: 1, requestId: "r2", ok: true, data: { device: activeDevice } };
      }
      if (action === "start_lcd") {
        return { version: 1, requestId: "r3", ok: true, data: { jobId: "job-dev0" } };
      }
      if (action === "job_status") {
        return slowStatusPromise;
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

    let selectDeviceCallback: ((id: string) => Promise<void>) | undefined;

    function TestHost() {
      const { selectDevice } = useDevice();
      selectDeviceCallback = selectDevice;
      return <LcdPage />;
    }

    render(
      <DeviceProvider bridge={bridge}>
        <TestHost />
      </DeviceProvider>
    );

    await vi.runOnlyPendingTimersAsync();

    // Start streaming on dev-mock-0
    const sendBtn = screen.getByRole("button", { name: /Send Pattern to Screen/i });
    fireEvent.click(sendBtn);

    await vi.runOnlyPendingTimersAsync();

    // Advance 500ms to trigger job_status poll
    vi.advanceTimersByTime(500);

    // Switch device via context to trigger useEffect cleanup
    activeDevice = mockDeviceB;
    await act(async () => {
      await selectDeviceCallback?.("dev-mock-1");
    });
    // Run pending timers to process state updates from device switch
    await vi.runOnlyPendingTimersAsync();

    // Resolve the late job_status from dev-mock-0
    if (resolveSlowStatus) {
      resolveSlowStatus({
        version: 1,
        requestId: "r-stale-dev",
        ok: true,
        data: {
          job: {
            id: "job-dev0",
            deviceId: "dev-mock-0",
            state: "running",
            framesSent: 500,
            error: null,
          },
        },
      });
    }

    await vi.runOnlyPendingTimersAsync();

    // Stale status from dev-mock-0 must be ignored on dev-mock-1
    expect(screen.queryByText(/500 frames/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Stop Streaming/i })).not.toBeInTheDocument();

    vi.useRealTimers();
  });

  it("handles cancelled asset picker returning { asset: null } without modifying previous asset or setting error", async () => {
    let pickCount = 0;
    const sendFn = vi.fn(async (action: Action, _deviceId: string | null) => {
      if (action === "list_devices") {
        return { version: 1, requestId: "r1", ok: true, data: { devices: [mockConnectedDevice] } };
      }
      if (action === "probe") {
        return { version: 1, requestId: "r2", ok: true, data: { device: mockConnectedDevice } };
      }
      if (action === "pick_asset") {
        pickCount += 1;
        if (pickCount === 1) {
          return { version: 1, requestId: "r-pick-1", ok: true, data: { asset: mockAsset } };
        }
        // Second pick cancelled by user (dialog closed / cancelled)
        return { version: 1, requestId: "r-pick-cancel", ok: true, data: { asset: null } };
      }
      return { version: 1, requestId: "r-err", ok: false, error: { code: "internal", message: "fail", retryable: false } };
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
        <LcdPage />
      </DeviceProvider>
    );

    expect(await screen.findByText(/LCD Display Management/i)).toBeInTheDocument();

    // 1. Pick initial asset
    const gifBtn = await screen.findByRole("button", { name: /Choose Animation/i });
    fireEvent.click(gifBtn);

    expect(await screen.findByText("retro_cat.gif")).toBeInTheDocument();

    // 2. Trigger second pick that gets cancelled
    fireEvent.click(gifBtn);

    await waitFor(() => {
      expect(sendFn).toHaveBeenCalledTimes(4); // list_devices, probe, pick 1, pick 2
    });

    // Previous asset must still be preserved
    expect(screen.getByText("retro_cat.gif")).toBeInTheDocument();
    expect(screen.getByText(/128x128 px/i)).toBeInTheDocument();
    // No error panel should be shown
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("handles cancelled error code from asset picker without modifying previous asset or setting error", async () => {
    let pickCount = 0;
    const sendFn = vi.fn(async (action: Action, _deviceId: string | null) => {
      if (action === "list_devices") {
        return { version: 1, requestId: "r1", ok: true, data: { devices: [mockConnectedDevice] } };
      }
      if (action === "probe") {
        return { version: 1, requestId: "r2", ok: true, data: { device: mockConnectedDevice } };
      }
      if (action === "pick_asset") {
        pickCount += 1;
        if (pickCount === 1) {
          return { version: 1, requestId: "r-pick-1", ok: true, data: { asset: mockAsset } };
        }
        return {
          version: 1,
          requestId: "r-pick-err",
          ok: false,
          error: { code: "cancelled", message: "User dismissed file picker", retryable: false },
        };
      }
      return { version: 1, requestId: "r-err", ok: false, error: { code: "internal", message: "fail", retryable: false } };
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
        <LcdPage />
      </DeviceProvider>
    );

    expect(await screen.findByText(/LCD Display Management/i)).toBeInTheDocument();

    // Initial pick
    const gifBtn = await screen.findByRole("button", { name: /Choose Animation/i });
    fireEvent.click(gifBtn);

    expect(await screen.findByText("retro_cat.gif")).toBeInTheDocument();

    // Second pick returning cancelled error
    fireEvent.click(gifBtn);

    await waitFor(() => {
      expect(sendFn).toHaveBeenCalledTimes(4);
    });

    // Previous asset preserved and no error alert displayed
    expect(screen.getByText("retro_cat.gif")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("handles device unplug / not_connected error gracefully while polling job_status", async () => {
    vi.useFakeTimers();

    let pollCount = 0;
    const sendFn = vi.fn(async (action: Action, _deviceId: string | null) => {
      if (action === "list_devices") {
        return { version: 1, requestId: "r1", ok: true, data: { devices: [mockConnectedDevice] } };
      }
      if (action === "probe") {
        return { version: 1, requestId: "r2", ok: true, data: { device: mockConnectedDevice } };
      }
      if (action === "start_lcd") {
        return { version: 1, requestId: "r3", ok: true, data: { jobId: "job-unplug-test" } };
      }
      if (action === "job_status") {
        pollCount += 1;
        if (pollCount === 1) {
          return {
            version: 1,
            requestId: "r4",
            ok: true,
            data: {
              job: {
                id: "job-unplug-test",
                deviceId: "dev-mock-0",
                state: "running",
                framesSent: 10,
                error: null,
              },
            },
          };
        }
        // Device unplugged during next poll
        return {
          version: 1,
          requestId: "r5",
          ok: false,
          error: {
            code: "not_connected",
            message: "USB device disconnected or interface lost",
            retryable: true,
          },
        };
      }
      return { version: 1, requestId: "r-err", ok: false, error: { code: "internal", message: "fail", retryable: false } };
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
        <LcdPage />
      </DeviceProvider>
    );

    await vi.runOnlyPendingTimersAsync();

    // Start streaming
    const sendBtn = screen.getByRole("button", { name: /Send Pattern to Screen/i });
    fireEvent.click(sendBtn);

    await vi.runOnlyPendingTimersAsync();

    // Advance 500ms to get first status poll
    vi.advanceTimersByTime(500);
    await vi.runOnlyPendingTimersAsync();
    expect(screen.getByText(/Frames Transmitted:\s*10/i)).toBeInTheDocument();

    // Advance another 500ms for second status poll where device is disconnected
    vi.advanceTimersByTime(500);
    await vi.runOnlyPendingTimersAsync();

    // Error alert should be displayed and polling stopped
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText(/USB device disconnected or interface lost/i)).toBeInTheDocument();
    expect(screen.getByText(/Code: not_connected/i)).toBeInTheDocument();

    vi.useRealTimers();
  });
});
