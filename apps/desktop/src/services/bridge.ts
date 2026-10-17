import type {
  Action,
  ActionPayloadMap,
  DesktopRequest,
  DesktopResponse,
  ErrorCode,
} from "../types/ipc";

export interface IpcTransport {
  invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T>;
}

const KNOWN_ACTIONS = new Set<string>([
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
]);

const ALLOWED_REQUEST_KEYS = new Set<string>([
  "version",
  "requestId",
  "deviceId",
  "action",
  "payload",
]);

export interface RequestValidationResult {
  valid: boolean;
  errorCode?: ErrorCode;
  reason?: string;
}

export function validateDesktopRequest(raw: unknown): RequestValidationResult {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    return { valid: false, errorCode: "invalid_input", reason: "Request must be a JSON object" };
  }

  const obj = raw as Record<string, unknown>;

  // Reject unexpected keys (e.g. unknown_field case)
  for (const key of Object.keys(obj)) {
    if (!ALLOWED_REQUEST_KEYS.has(key)) {
      return {
        valid: false,
        errorCode: "invalid_input",
        reason: `Unexpected field in request: ${key}`,
      };
    }
  }

  // Version must be exactly 1
  if (obj.version !== 1) {
    return {
      valid: false,
      errorCode: "invalid_input",
      reason: `Unsupported contract version: ${String(obj.version)}`,
    };
  }

  // RequestId must be non-empty string
  if (typeof obj.requestId !== "string" || obj.requestId.trim().length === 0) {
    return {
      valid: false,
      errorCode: "invalid_input",
      reason: "requestId must be a non-empty string",
    };
  }

  // Action must be known
  if (typeof obj.action !== "string" || !KNOWN_ACTIONS.has(obj.action)) {
    return {
      valid: false,
      errorCode: "invalid_input",
      reason: `Unknown action: ${String(obj.action)}`,
    };
  }

  // Payload must be an object
  if (typeof obj.payload !== "object" || obj.payload === null || Array.isArray(obj.payload)) {
    return {
      valid: false,
      errorCode: "invalid_input",
      reason: "payload must be an object",
    };
  }

  return { valid: true };
}

let requestCounter = 0;

export function generateRequestId(action: string): string {
  requestCounter += 1;
  const time = Date.now().toString(36);
  const rand = Math.random().toString(36).substring(2, 6);
  return `req-${action}-${time}-${rand}-${requestCounter}`;
}

export interface DesktopBridge {
  send<A extends Action>(
    action: A,
    deviceId: string | null,
    payload: ActionPayloadMap[A]
  ): Promise<DesktopResponse<A>>;
}

export function createBridge(transport?: IpcTransport): DesktopBridge {
  // Default transport attempts Tauri invoke or errors if running bare in browser without mock
  const activeTransport: IpcTransport = transport ?? {
    async invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        return await invoke<T>(cmd, args);
      } catch (err: unknown) {
        throw new Error(
          `Tauri invoke failed (${cmd}): ${err instanceof Error ? err.message : String(err)}`
        );
      }
    },
  };

  return {
    async send<A extends Action>(
      action: A,
      deviceId: string | null,
      payload: ActionPayloadMap[A]
    ): Promise<DesktopResponse<A>> {
      const requestId = generateRequestId(action);
      const request: DesktopRequest<A> = {
        version: 1,
        requestId,
        deviceId,
        action,
        payload,
      };

      try {
        const response = await activeTransport.invoke<DesktopResponse<A>>(
          "desktop_request",
          { request }
        );

        // Strict verification of IPC response contract
        if (!response || typeof response !== "object" || response.version !== 1 || response.requestId !== requestId) {
          return {
            version: 1,
            requestId,
            ok: false,
            error: {
              code: "internal",
              message: "Invalid response from native bridge: version or requestId mismatch",
              retryable: false,
            },
          };
        }

        return response;
      } catch (err: unknown) {
        return {
          version: 1,
          requestId,
          ok: false,
          error: {
            code: "internal",
            message:
              err instanceof Error
                ? err.message
                : typeof err === "string"
                  ? err
                  : "An unexpected transport failure occurred",
            retryable: false,
          },
        };
      }
    },
  };
}
