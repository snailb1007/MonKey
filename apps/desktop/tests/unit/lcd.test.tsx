import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { DeviceProvider } from "../../src/context/DeviceContext";
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
});
