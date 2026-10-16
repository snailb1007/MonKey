import type {
  Action,
  ActionPayloadMap,
  DesktopRequest,
  DesktopResponse,
} from "../types/ipc";

export interface IpcTransport {
  invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T>;
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
