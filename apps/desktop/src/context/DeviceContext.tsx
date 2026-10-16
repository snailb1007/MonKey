import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  useRef,
  useCallback,
} from "react";
import type { DeviceSnapshot, TypedError } from "../types/ipc";
import { type DesktopBridge, createBridge } from "../services/bridge";

interface DeviceContextValue {
  devices: DeviceSnapshot[];
  selectedDevice: DeviceSnapshot | null;
  loading: boolean;
  error: TypedError | null;
  bridge: DesktopBridge;
  refreshDevices: () => Promise<void>;
  selectDevice: (deviceId: string) => Promise<void>;
}

const DeviceContext = createContext<DeviceContextValue | null>(null);

const DEFAULT_GLOBAL_BRIDGE = createBridge();

export interface DeviceProviderProps {
  children: React.ReactNode;
  bridge?: DesktopBridge;
}

export const DeviceProvider: React.FC<DeviceProviderProps> = ({
  children,
  bridge = DEFAULT_GLOBAL_BRIDGE,
}) => {
  const [devices, setDevices] = useState<DeviceSnapshot[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<DeviceSnapshot | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<TypedError | null>(null);

  // Counter to discard out-of-order stale probe responses
  const probeSequenceRef = useRef<number>(0);
  const selectedDeviceIdRef = useRef<string | null>(null);

  useEffect(() => {
    selectedDeviceIdRef.current = selectedDevice?.id ?? null;
  }, [selectedDevice?.id]);

  const probeDevice = useCallback(
    async (deviceId: string, seq: number) => {
      const resp = await bridge.send("probe", deviceId, {});
      // If a newer probe was started while this one was in-flight, discard this response
      if (probeSequenceRef.current !== seq) {
        return;
      }

      if (resp.ok) {
        setSelectedDevice(resp.data.device);
        setError(null);
      } else {
        setError(resp.error);
      }
    },
    [bridge]
  );

  const selectDevice = useCallback(
    async (deviceId: string) => {
      const target = devices.find((d) => d.id === deviceId);
      if (target) {
        setSelectedDevice(target);
      }
      probeSequenceRef.current += 1;
      const currentSeq = probeSequenceRef.current;
      await probeDevice(deviceId, currentSeq);
    },
    [devices, probeDevice]
  );

  const refreshDevices = useCallback(async () => {
    setLoading(true);
    setError(null);
    const resp = await bridge.send("list_devices", null, {});
    setLoading(false);

    if (resp.ok) {
      const devList = resp.data.devices;
      setDevices(devList);
      if (devList.length > 0) {
        // Auto-select first device if none selected or if selected is not in list
        const currentId = selectedDeviceIdRef.current;
        const exists = devList.find((d) => d.id === currentId);
        const nextDevice = exists || devList[0];
        setSelectedDevice(nextDevice);

        probeSequenceRef.current += 1;
        await probeDevice(nextDevice.id, probeSequenceRef.current);
      } else {
        setSelectedDevice(null);
      }
    } else {
      setError(resp.error);
      setDevices([]);
      setSelectedDevice(null);
    }
  }, [bridge, probeDevice]);

  useEffect(() => {
    refreshDevices();
  }, [refreshDevices]);

  return (
    <DeviceContext.Provider
      value={{
        devices,
        selectedDevice,
        loading,
        error,
        bridge,
        refreshDevices,
        selectDevice,
      }}
    >
      {children}
    </DeviceContext.Provider>
  );
};

export function useDevice(): DeviceContextValue {
  const ctx = useContext(DeviceContext);
  if (!ctx) {
    throw new Error("useDevice must be used within a DeviceProvider");
  }
  return ctx;
}
