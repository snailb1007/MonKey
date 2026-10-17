import React from "react";
import { useDevice } from "../context/DeviceContext";
import { OperationNotice } from "../components/OperationNotice";
import { ActionButton } from "../components/ActionButton";
import { buildErrorReport } from "../services/issueReport";

export const DevicePage: React.FC = () => {
  const { devices, selectedDevice, loading, error, refreshDevices, selectDevice } =
    useDevice();

  return (
    <div className="page device-page">
      <header className="page-header">
        <h1 className="page-title">Device Identity & Status</h1>
        <p className="page-description">
          Hardware discovery and interface negotiation for Shenzhen HFD / RKGK890 keyboard.
        </p>
      </header>

      {error && (
        <OperationNotice
          kind="error"
          title="Communication Notice"
          message={error.message}
          code={error.code}
          details={buildErrorReport({
            action: "device_discovery",
            deviceLabel: selectedDevice?.label,
            code: error.code,
            message: error.message,
          })}
          issueTitle={`Device discovery failed: ${error.code}`}
          issueAction="Device discovery"
          retryable={error.retryable}
          onRetry={error.retryable ? () => void refreshDevices() : undefined}
        />
      )}

      {loading && devices.length === 0 ? (
        <div className="panel loading-panel" aria-live="polite">
          <p>Scanning for devices...</p>
        </div>
      ) : devices.length === 0 ? (
        <div className="panel empty-panel">
          <h2>No compatible keyboards detected</h2>
          <p>
            Please connect your Monka 3075 Pro via USB cable. Ensure the rear switch is set
            to USB/Wired mode, or wake the device if asleep.
          </p>
          <ActionButton
            className="pixel-btn primary-btn"
            onClick={() => refreshDevices()}
            pending={loading}
          >
            Rescan USB Bus
          </ActionButton>
        </div>
      ) : (
        <div className="device-layout">
          <section className="panel device-list-panel" aria-labelledby="device-list-heading">
            <div className="panel-header" id="device-list-heading">
              <span>Detected Hardware ({devices.length})</span>
              <ActionButton
                className="pixel-btn small-btn"
                onClick={() => refreshDevices()}
                pending={loading}
                aria-label="Rescan connected keyboards"
              >
                Scan
              </ActionButton>
            </div>
            <ul className="device-list" role="list">
              {devices.map((dev) => {
                const isSelected = selectedDevice?.id === dev.id;
                return (
                  <li key={dev.id} className="device-item">
                    <button
                      type="button"
                      className={`device-btn ${isSelected ? "selected" : ""}`}
                      onClick={() => selectDevice(dev.id)}
                      aria-pressed={isSelected}
                      aria-label={`Select ${dev.label}`}
                    >
                      <span className="device-item-label">{dev.label}</span>
                      <span className="badge connection-badge">{dev.connection}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </section>

          {selectedDevice && (
            <section
              className="panel device-details-panel"
              aria-labelledby="device-details-heading"
            >
              <div className="panel-header" id="device-details-heading">
                <span>Active Device: {selectedDevice.label}</span>
                <span className="device-id-tag">ID: {selectedDevice.id}</span>
              </div>

              <div className="details-grid">
                <div className="detail-card">
                  <span className="detail-label">Connection</span>
                  <span className="detail-value text-capitalize">
                    {selectedDevice.connection}
                  </span>
                </div>
                <div className="detail-card">
                  <span className="detail-label">Battery Level</span>
                  <span className="detail-value">
                    {selectedDevice.batteryPercent !== null
                      ? `${selectedDevice.batteryPercent}%`
                      : "Unknown (AC/Wired)"}
                  </span>
                </div>
                <div className="detail-card">
                  <span className="detail-label">Firmware</span>
                  <span className="detail-value">
                    {selectedDevice.firmware ? `Firmware: ${selectedDevice.firmware}` : "Unknown"}
                  </span>
                </div>
              </div>

              <div className="interfaces-section">
                <h3>Hardware USB Interfaces</h3>
                <div className="interface-row">
                  <span className="interface-name">
                    Interface A (LCD 0xFF68):
                  </span>
                  <span
                    className={`badge ${
                      selectedDevice.interfaces.lcd ? "badge-success" : "badge-inactive"
                    }`}
                  >
                    {selectedDevice.interfaces.lcd ? "Active" : "Inactive"}
                  </span>
                  <span className="interface-desc">
                    4096-byte bulk display pipe for 128x128 RGB565 frames
                  </span>
                </div>

                <div className="interface-row">
                  <span className="interface-name">
                    Interface B (Config 0xFFFF):
                  </span>
                  <span
                    className={`badge ${
                      selectedDevice.interfaces.config ? "badge-success" : "badge-inactive"
                    }`}
                  >
                    {selectedDevice.interfaces.config ? "Active" : "Inactive"}
                  </span>
                  <span className="interface-desc">
                    64-byte feature reports for RGB lighting & parameters
                  </span>
                </div>
              </div>

              <div className="capabilities-section">
                <h3>Capability Matrix & Safety Policy</h3>
                <ul className="capabilities-list">
                  <li className="cap-item">
                    <span className="cap-name">LCD Streaming:</span>
                    <span
                      className={`badge ${
                        selectedDevice.capabilities.lcd ? "badge-success" : "badge-danger"
                      }`}
                    >
                      {selectedDevice.capabilities.lcd ? "Available" : "Unavailable"}
                    </span>
                  </li>
                  <li className="cap-item">
                    <span className="cap-name">RGB Lighting:</span>
                    <span
                      className={`badge ${
                        selectedDevice.capabilities.rgb ? "badge-success" : "badge-danger"
                      }`}
                    >
                      {selectedDevice.capabilities.rgb ? "Available" : "Unavailable"}
                    </span>
                  </li>
                  <li className="cap-item">
                    <span className="cap-name">Flash Persistence:</span>
                    <span
                      className={`badge ${
                        selectedDevice.capabilities.persist ? "badge-success" : "badge-caution"
                      }`}
                    >
                      {selectedDevice.capabilities.persist ? "Supported" : "Gated"}
                    </span>
                  </li>
                  <li className="cap-item unsupported">
                    <span className="cap-name">Key Remap: Unsupported</span>
                    <span className="cap-reason">
                      (Vendor dynamic remapping protocol unverified)
                    </span>
                  </li>
                  <li className="cap-item unsupported">
                    <span className="cap-name">Macro Engine: Unsupported</span>
                    <span className="cap-reason">
                      (OEM macro format unverified; writes blocked for hardware safety)
                    </span>
                  </li>
                  <li className="cap-item unsupported">
                    <span className="cap-name">Firmware Flasher: Unsupported</span>
                    <span className="cap-reason">
                      (DFU/ISP flash write opcodes disabled to prevent permanent bricking)
                    </span>
                  </li>
                </ul>
              </div>
            </section>
          )}
        </div>
      )}
    </div>
  );
};
