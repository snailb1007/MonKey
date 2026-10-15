use crate::dto::*;
use crate::error::DesktopError;
use crate::worker::{HardwareWorkerHandle, MockHardwareBackend};
use monkey_core::transport::MockTransport;

#[test]
fn test_worker_list_devices_and_probe_no_writes() {
    let mock = MockTransport::new();
    let (backend, transport_ref) = MockHardwareBackend::with_mock_transport(mock);
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    // List devices
    let devices = worker.list_devices().expect("list_devices should succeed");
    assert!(
        !devices.is_empty(),
        "Should discover at least one mock device"
    );
    let dev = &devices[0];
    assert_eq!(dev.label, "Monka 3075 Pro / RKGK890");
    assert_eq!(dev.connection, ConnectionType::Wired);
    assert!(dev.interfaces.lcd);
    assert!(dev.interfaces.config);
    assert!(dev.capabilities.lcd);
    assert!(dev.capabilities.rgb);
    assert!(dev.capabilities.persist);
    assert!(!dev.capabilities.remap);
    assert!(!dev.capabilities.macro_);
    assert!(!dev.capabilities.firmware);

    // Probe device
    let probed = worker.probe(&dev.id).expect("probe should succeed");
    assert_eq!(probed.id, dev.id);

    // CRITICAL SAFETY CHECK: Prove no configuration writes occurred during list & probe
    let transport = transport_ref.lock().unwrap();
    transport
        .assert_no_writes()
        .expect("Probing and listing must NEVER perform any configuration write");

    worker.shutdown();
}

#[test]
fn test_worker_probe_unknown_device_fails() {
    let mock = MockTransport::new();
    let (backend, _) = MockHardwareBackend::with_mock_transport(mock);
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    let res = worker.probe("unknown-dev-999");
    match res {
        Err(DesktopError::NotConnected(msg)) => {
            assert!(msg.contains("unknown-dev-999"));
        }
        other => panic!("Expected NotConnected error, got: {other:?}"),
    }

    worker.shutdown();
}

#[test]
fn test_worker_diagnostics_sanitized() {
    let mock = MockTransport::new();
    let (backend, _) = MockHardwareBackend::with_mock_transport(mock);
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    let checks = worker.diagnostics().expect("diagnostics should succeed");
    assert!(
        !checks.is_empty(),
        "Doctor checks must return at least platform check"
    );

    for check in &checks {
        // Assert no raw paths or serials in check details
        assert!(
            !check.detail.contains("/dev/"),
            "Check detail leaked device path"
        );
        assert!(
            !check.detail.contains("serial_number"),
            "Check detail leaked serial number"
        );
        assert!(
            !check.detail.contains("IOUSB"),
            "Check detail leaked IOUSB internal path"
        );
    }

    worker.shutdown();
}

#[test]
fn test_worker_orderly_shutdown() {
    let mock = MockTransport::new();
    let (backend, _) = MockHardwareBackend::with_mock_transport(mock);
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    // Shutdown cleans up background OS thread cleanly
    worker.shutdown();
}
