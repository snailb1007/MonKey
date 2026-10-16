import type { IpcTransport } from "./bridge";
import type {
  DesktopRequest,
  DesktopResponse,
  DeviceSnapshot,
  DiagnosticCheck,
  LightingConfigDto,
  PersistenceState,
  AssetPreview,
  JobSnapshot,
  Action,
} from "../types/ipc";

export function isDemoMode(): boolean {
  if (import.meta.env.VITE_MONKEY_DEMO === "1") {
    return true;
  }
  if (typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window)) {
    return true;
  }
  return false;
}

const DEMO_DEVICES: DeviceSnapshot[] = [
  {
    id: "dev-demo-wired",
    label: "Monka 3075 Pro (USB Wired)",
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
    firmware: "v1.0.0-hfd",
  },
  {
    id: "dev-demo-wireless",
    label: "Monka 3075 Pro (2.4G Wireless)",
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
    batteryPercent: 18,
    firmware: null,
  },
];

const DEMO_CHECKS: DiagnosticCheck[] = [
  {
    id: "chk-usb-a",
    label: "Interface A (LCD Bulk Display Pipe)",
    status: "pass",
    detail: "Usage Page 0xFF68 Usage 0x0061 (OUT report size 4096 bytes) ready",
    remediation: null,
  },
  {
    id: "chk-usb-b",
    label: "Interface B (Config & Feature Reports)",
    status: "pass",
    detail: "Usage Page 0xFFFF Usage 0x0001 (Feature reports 64 bytes) accessible via shared device",
    remediation: null,
  },
  {
    id: "chk-battery",
    label: "Battery Power & Charging Status",
    status: "pass",
    detail: "Wired USB power active (95% battery level)",
    remediation: null,
  },
  {
    id: "chk-flash-safety",
    label: "Flash Write Rails & Safety Gate",
    status: "pass",
    detail: "Hardware safety whitelist active. Blind writes disabled.",
    remediation: null,
  },
];

const DEMO_SAMPLE_ASSET: AssetPreview = {
  id: "asset-demo-pixel-cat",
  name: "cyber_cat_128x128.gif",
  width: 128,
  height: 128,
  frameCount: 12,
  previewDataUrl:
    "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='128' height='128' viewBox='0 0 128 128'><rect width='128' height='128' fill='%23111411'/><rect x='32' y='32' width='64' height='64' fill='%23315B3A'/><rect x='44' y='48' width='12' height='12' fill='%23FFFCF4'/><rect x='72' y='48' width='12' height='12' fill='%23FFFCF4'/><rect x='52' y='72' width='24' height='8' fill='%23F4F0E6'/></svg>",
};

export function createDemoTransport(): IpcTransport {
  let lightingConfig: LightingConfigDto = {
    mode: "wave",
    color: { r: 49, g: 91, b: 58 },
    brightness: 85,
    speed: 60,
    direction: "left_to_right",
  };
  let lightingPersistence: PersistenceState = "verified";
  let activeJob: JobSnapshot | null = null;
  let jobFrameCounter = 0;

  return {
    async invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
      if (cmd !== "desktop_request") {
        throw new Error(`Unsupported demo invoke command: ${cmd}`);
      }

      const req = (args as { request: DesktopRequest })?.request;
      if (!req) {
        throw new Error("Missing request envelope");
      }

      // Small simulated latency (20ms)
      await new Promise((resolve) => setTimeout(resolve, 20));

      const action = req.action as Action;
      let response: DesktopResponse;

      switch (action) {
        case "list_devices":
          response = {
            version: 1,
            requestId: req.requestId,
            ok: true,
            data: { devices: DEMO_DEVICES },
          };
          break;

        case "probe": {
          const found =
            DEMO_DEVICES.find((d) => d.id === req.deviceId) || DEMO_DEVICES[0];
          response = {
            version: 1,
            requestId: req.requestId,
            ok: true,
            data: { device: found },
          };
          break;
        }

        case "diagnostics":
          response = {
            version: 1,
            requestId: req.requestId,
            ok: true,
            data: { checks: DEMO_CHECKS },
          };
          break;

        case "pick_asset":
          response = {
            version: 1,
            requestId: req.requestId,
            ok: true,
            data: { asset: DEMO_SAMPLE_ASSET },
          };
          break;

        case "start_lcd": {
          const jobId = `demo-job-${Date.now()}`;
          jobFrameCounter = 0;
          activeJob = {
            id: jobId,
            deviceId: req.deviceId || "dev-demo-wired",
            state: "running",
            framesSent: 0,
            error: null,
          };
          response = {
            version: 1,
            requestId: req.requestId,
            ok: true,
            data: { jobId },
          };
          break;
        }

        case "stop_lcd":
          if (activeJob) {
            activeJob.state = "cancelled";
          }
          response = {
            version: 1,
            requestId: req.requestId,
            ok: true,
            data: { stopped: true },
          };
          break;

        case "job_status": {
          if (activeJob && activeJob.state === "running") {
            jobFrameCounter += 2;
            activeJob.framesSent = jobFrameCounter;
            if (jobFrameCounter >= 24) {
              activeJob.state = "completed";
            }
          }
          response = {
            version: 1,
            requestId: req.requestId,
            ok: true,
            data: {
              job: activeJob ?? {
                id: (req.payload as { jobId: string }).jobId,
                deviceId: req.deviceId || "dev-demo-wired",
                state: "completed",
                framesSent: 24,
                error: null,
              },
            },
          };
          break;
        }

        case "rgb_status":
          response = {
            version: 1,
            requestId: req.requestId,
            ok: true,
            data: {
              config: lightingConfig,
              persistence: lightingPersistence,
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
          break;

        case "rgb_preview": {
          const p = req.payload as { config: LightingConfigDto };
          lightingConfig = { ...p.config };
          lightingPersistence = "ram";
          response = {
            version: 1,
            requestId: req.requestId,
            ok: true,
            data: {
              config: lightingConfig,
              persistence: "ram",
            },
          };
          break;
        }

        case "rgb_commit":
          // If wireless with battery < 20%, simulate low_battery error
          if (req.deviceId === "dev-demo-wireless") {
            response = {
              version: 1,
              requestId: req.requestId,
              ok: false,
              error: {
                code: "low_battery",
                message: "Persistence is blocked below 20% battery on wireless",
                retryable: false,
              },
            };
          } else {
            lightingPersistence = "verified";
            response = {
              version: 1,
              requestId: req.requestId,
              ok: true,
              data: { persistence: "verified" },
            };
          }
          break;

        case "export_profile":
          response = {
            version: 1,
            requestId: req.requestId,
            ok: true,
            data: {
              profileJson: JSON.stringify(
                {
                  version: 1,
                  device: req.deviceId,
                  config: lightingConfig,
                  timestamp: new Date().toISOString(),
                },
                null,
                2
              ),
            },
          };
          break;

        case "import_profile": {
          const p = req.payload as { profileJson: string };
          try {
            const parsed = JSON.parse(p.profileJson);
            if (parsed.config) {
              lightingConfig = { ...parsed.config };
              lightingPersistence = "ram";
              response = {
                version: 1,
                requestId: req.requestId,
                ok: true,
                data: { config: lightingConfig },
              };
            } else {
              response = {
                version: 1,
                requestId: req.requestId,
                ok: false,
                error: {
                  code: "invalid_input",
                  message: "Profile JSON missing config object",
                  retryable: false,
                },
              };
            }
          } catch {
            response = {
              version: 1,
              requestId: req.requestId,
              ok: false,
              error: {
                code: "invalid_input",
                message: "Malformed JSON payload",
                retryable: false,
              },
            };
          }
          break;
        }

        default:
          response = {
            version: 1,
            requestId: req.requestId,
            ok: false,
            error: {
              code: "invalid_input",
              message: `Unknown action: ${action}`,
              retryable: false,
            },
          };
          break;
      }

      return response as T;
    },
  };
}
