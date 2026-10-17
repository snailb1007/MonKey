import React from "react";
import { useDevice } from "../context/DeviceContext";

export const Header: React.FC = () => {
  const { selectedDevice } = useDevice();

  return (
    <header className="top-bar" role="banner">
      <div className="brand">
        <span>MonKey</span>
        <span className="brand-badge">v2.0</span>
      </div>

      <div className="device-summary-bar" aria-label="Device summary">
        {selectedDevice ? (
          <>
            <span className="device-name">{selectedDevice.label}</span>
            <span
              className={`badge ${
                selectedDevice.connection === "wired"
                  ? "badge-success"
                  : selectedDevice.connection === "wireless"
                    ? "badge-caution"
                    : "badge-inactive"
              }`}
            >
              {selectedDevice.connection}
            </span>
            <span
              className={`badge ${
                selectedDevice.interfaces.lcd ? "badge-success" : "badge-inactive"
              }`}
              title="Interface A (0xFF68): Bulk Display Pipe"
            >
              LCD {selectedDevice.interfaces.lcd ? "ON" : "OFF"}
            </span>
            <span
              className={`badge ${
                selectedDevice.interfaces.config ? "badge-success" : "badge-inactive"
              }`}
              title="Interface B (0xFFFF): Configuration & RGB"
            >
              RGB {selectedDevice.interfaces.config ? "ON" : "OFF"}
            </span>
          </>
        ) : (
          <span className="badge badge-inactive">No Keyboard Connected</span>
        )}
      </div>
    </header>
  );
};
