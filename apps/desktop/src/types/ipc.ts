/**
 * Typed definitions matching locked shared IPC contract v1 (apps/desktop/contracts/ipc-v1.json).
 */

export type ErrorCode =
  | "not_connected"
  | "permission_denied"
  | "unsupported"
  | "busy"
  | "unsafe_operation"
  | "low_battery"
  | "invalid_input"
  | "io_error"
  | "cancelled"
  | "internal";

export interface TypedError {
  code: ErrorCode;
  message: string;
  retryable: boolean;
}

export type ConnectionState = "wired" | "wireless" | "asleep" | "disconnected" | "unknown";

export interface DeviceInterfaces {
  lcd: boolean;
  config: boolean;
}

export interface DeviceCapabilities {
  lcd: boolean;
  rgb: boolean;
  persist: boolean;
  remap: false;
  macro: false;
  firmware: false;
}

export interface DeviceSnapshot {
  id: string;
  label: string;
  connection: ConnectionState;
  interfaces: DeviceInterfaces;
  capabilities: DeviceCapabilities;
  batteryPercent: number | null;
  firmware: string | null;
}

export type LightingMode =
  | "off"
  | "static"
  | "breathing"
  | "wave"
  | "rainbow"
  | "ripple"
  | "reactive";

export type LightingDirection = "left_to_right" | "right_to_left";

export interface RgbColor {
  r: number;
  g: number;
  b: number;
}

export interface LightingConfigDto {
  mode: LightingMode;
  color: RgbColor;
  brightness: number;
  speed: number;
  direction: LightingDirection | null;
}

export type PersistenceState = "unknown" | "ram" | "verified";

export interface AssetPreview {
  id: string;
  name: string;
  width: 128;
  height: 128;
  frameCount: number;
  previewDataUrl: string;
}

export type JobState = "queued" | "running" | "completed" | "cancelled" | "failed";

export interface JobSnapshot {
  id: string;
  deviceId: string;
  state: JobState;
  framesSent: number;
  error: TypedError | null;
}

export type DiagnosticStatus = "pass" | "warn" | "fail" | "unknown";

export interface DiagnosticCheck {
  id: string;
  label: string;
  status: DiagnosticStatus;
  detail: string;
  remediation: string | null;
}

export type TestPatternType =
  | "red"
  | "green"
  | "blue"
  | "white"
  | "black"
  | "grayscale"
  | "color_bars"
  | "geometry";

export type Action =
  | "list_devices"
  | "probe"
  | "diagnostics"
  | "pick_asset"
  | "start_lcd"
  | "stop_lcd"
  | "job_status"
  | "rgb_status"
  | "rgb_preview"
  | "rgb_commit"
  | "export_profile"
  | "import_profile";

export interface ActionPayloadMap {
  list_devices: Record<string, never>;
  probe: Record<string, never>;
  diagnostics: Record<string, never>;
  pick_asset: { kind: "image" | "animation" | "sequence" };
  start_lcd: { assetId: string | null; pattern: TestPatternType | null };
  stop_lcd: { jobId: string };
  job_status: { jobId: string };
  rgb_status: Record<string, never>;
  rgb_preview: { config: LightingConfigDto };
  rgb_commit: Record<string, never>;
  export_profile: { config: LightingConfigDto; description?: string };
  import_profile: { profileJson: string };
}

export interface ActionDataMap {
  list_devices: { devices: DeviceSnapshot[] };
  probe: { device: DeviceSnapshot };
  diagnostics: { checks: DiagnosticCheck[] };
  pick_asset: { asset: AssetPreview | null };
  start_lcd: { jobId: string };
  stop_lcd: { stopped: boolean };
  job_status: { job: JobSnapshot };
  rgb_status: {
    config: LightingConfigDto | null;
    persistence: PersistenceState;
    supportedModes: LightingMode[];
  };
  rgb_preview: {
    config: LightingConfigDto;
    persistence: "ram";
  };
  rgb_commit: {
    persistence: "verified";
  };
  export_profile: { profileJson: string } | { saved: boolean };
  import_profile: { config: LightingConfigDto | null };
}

export interface DesktopRequest<A extends Action = Action> {
  version: 1;
  requestId: string;
  deviceId: string | null;
  action: A;
  payload: ActionPayloadMap[A];
}

export type DesktopSuccessResponse<A extends Action = Action> = {
  version: 1;
  requestId: string;
  ok: true;
  data: ActionDataMap[A];
};

export type DesktopErrorResponse = {
  version: 1;
  requestId: string;
  ok: false;
  error: TypedError;
};

export type DesktopResponse<A extends Action = Action> =
  | DesktopSuccessResponse<A>
  | DesktopErrorResponse;
