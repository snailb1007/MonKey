use monkey_core::bench::BenchmarkConfig;
use monkey_core::device::{
    DiscoveredDevice, InterfaceCheckStatus, InterfacePolicy, InterfaceRole, MonkaDevice,
    MonkaDeviceSet, OpenError, MONKA_PID, MONKA_VID,
};
use monkey_core::lcd::{LcdPacingConfig, LCD_CHUNK_COUNT, LCD_FRAME_BYTES};
use monkey_core::rgb::{FlowDirection, LightingConfig, LightingMode, RgbColor};
use monkey_core::transport::MockTransport;
use std::ffi::CString;
use std::time::Duration;

fn make_test_device(
    usage_page: u16,
    usage: u16,
    interface_number: i32,
    path_str: &str,
) -> DiscoveredDevice {
    DiscoveredDevice {
        vid: MONKA_VID,
        pid: MONKA_PID,
        manufacturer: Some("Shenzhen HFD Technology Co., Ltd.".to_string()),
        product: Some("RKGK890".to_string()),
        serial_number: Some("MK3075P001".to_string()),
        path: CString::new(path_str).expect("Valid CString"),
        usage_page,
        usage,
        interface_number,
    }
}

fn make_dual_device_set() -> MonkaDeviceSet {
    let mut set = MonkaDeviceSet::new(MONKA_VID, MONKA_PID);
    set.interface_a = Some(make_test_device(0xFF68, 0x0061, 0, "path_a"));
    set.interface_b = Some(make_test_device(0xFFFF, 0x0001, 1, "path_b"));
    set.product = Some("RKGK890".to_string());
    set
}

fn make_interface_a_only_set() -> MonkaDeviceSet {
    let mut set = MonkaDeviceSet::new(MONKA_VID, MONKA_PID);
    set.interface_a = Some(make_test_device(0xFF68, 0x0061, 0, "path_a"));
    set.product = Some("RKGK890".to_string());
    set
}

fn make_interface_b_only_set() -> MonkaDeviceSet {
    let mut set = MonkaDeviceSet::new(MONKA_VID, MONKA_PID);
    set.interface_b = Some(make_test_device(0xFFFF, 0x0001, 1, "path_b"));
    set.product = Some("RKGK890".to_string());
    set
}

#[test]
fn test_monka_device_from_transport_and_builders() {
    let mock = Box::new(MockTransport::new());
    let dev = MonkaDevice::from_transport(mock)
        .with_hardware_writes_allowed(true)
        .with_wireless(true);

    assert!(dev.is_wireless());
    assert_eq!(dev.role(), None);
    assert!(dev.device_set().is_none());
    assert!(dev.safety().is_hardware_write_allowed());

    let set = make_dual_device_set();
    let dev_with_set = dev.with_device_set(set);
    assert!(dev_with_set.device_set().is_some());
    assert_eq!(dev_with_set.device_set().unwrap().vid, MONKA_VID);
}

#[test]
fn test_interface_policy_resolution_dual_set() {
    let set = make_dual_device_set();

    // RequireA -> Interface A
    let (dev_a, role_a) = InterfacePolicy::RequireA.resolve(&set).unwrap();
    assert_eq!(role_a, InterfaceRole::InterfaceA);
    assert_eq!(dev_a.path, CString::new("path_a").unwrap());

    // RequireB -> Interface B
    let (dev_b, role_b) = InterfacePolicy::RequireB.resolve(&set).unwrap();
    assert_eq!(role_b, InterfaceRole::InterfaceB);
    assert_eq!(dev_b.path, CString::new("path_b").unwrap());

    // PreferB -> Interface B
    let (_, role_pref) = InterfacePolicy::PreferB.resolve(&set).unwrap();
    assert_eq!(role_pref, InterfaceRole::InterfaceB);

    // BulkFirst -> Interface A
    let (_, role_bulk) = InterfacePolicy::BulkFirst.resolve(&set).unwrap();
    assert_eq!(role_bulk, InterfaceRole::InterfaceA);

    // ControlFirst -> Interface B
    let (_, role_ctrl) = InterfacePolicy::ControlFirst.resolve(&set).unwrap();
    assert_eq!(role_ctrl, InterfaceRole::InterfaceB);

    // Any -> Interface B
    let (_, role_any) = InterfacePolicy::Any.resolve(&set).unwrap();
    assert_eq!(role_any, InterfaceRole::InterfaceB);

    // MonkaDevice::resolve_policy wrapper matches
    let res = MonkaDevice::resolve_policy(&set, InterfacePolicy::RequireA).unwrap();
    assert_eq!(res.1, InterfaceRole::InterfaceA);
}

#[test]
fn test_interface_policy_resolution_interface_a_only() {
    let set = make_interface_a_only_set();

    // RequireA -> Interface A
    let (_, role_a) = InterfacePolicy::RequireA.resolve(&set).unwrap();
    assert_eq!(role_a, InterfaceRole::InterfaceA);

    // RequireB -> Error InterfaceUnavailable
    let err_b = InterfacePolicy::RequireB.resolve(&set).unwrap_err();
    assert_eq!(
        err_b,
        OpenError::InterfaceUnavailable(InterfaceRole::InterfaceB)
    );

    // PreferB -> Fallback to Interface A
    let (_, role_pref) = InterfacePolicy::PreferB.resolve(&set).unwrap();
    assert_eq!(role_pref, InterfaceRole::InterfaceA);

    // BulkFirst -> Interface A
    let (_, role_bulk) = InterfacePolicy::BulkFirst.resolve(&set).unwrap();
    assert_eq!(role_bulk, InterfaceRole::InterfaceA);

    // ControlFirst -> Fallback to Interface A
    let (_, role_ctrl) = InterfacePolicy::ControlFirst.resolve(&set).unwrap();
    assert_eq!(role_ctrl, InterfaceRole::InterfaceA);
}

#[test]
fn test_interface_policy_resolution_interface_b_only() {
    let set = make_interface_b_only_set();

    // RequireA -> Error InterfaceUnavailable
    let err_a = InterfacePolicy::RequireA.resolve(&set).unwrap_err();
    assert_eq!(
        err_a,
        OpenError::InterfaceUnavailable(InterfaceRole::InterfaceA)
    );

    // RequireB -> Interface B
    let (_, role_b) = InterfacePolicy::RequireB.resolve(&set).unwrap();
    assert_eq!(role_b, InterfaceRole::InterfaceB);

    // PreferB -> Interface B
    let (_, role_pref) = InterfacePolicy::PreferB.resolve(&set).unwrap();
    assert_eq!(role_pref, InterfaceRole::InterfaceB);

    // BulkFirst -> Fallback to Interface B
    let (_, role_bulk) = InterfacePolicy::BulkFirst.resolve(&set).unwrap();
    assert_eq!(role_bulk, InterfaceRole::InterfaceB);

    // ControlFirst -> Interface B
    let (_, role_ctrl) = InterfacePolicy::ControlFirst.resolve(&set).unwrap();
    assert_eq!(role_ctrl, InterfaceRole::InterfaceB);
}

#[test]
fn test_interface_policy_resolution_empty_set() {
    let set = MonkaDeviceSet::new(MONKA_VID, MONKA_PID);

    assert_eq!(
        InterfacePolicy::RequireA.resolve(&set).unwrap_err(),
        OpenError::InterfaceUnavailable(InterfaceRole::InterfaceA)
    );
    assert_eq!(
        InterfacePolicy::RequireB.resolve(&set).unwrap_err(),
        OpenError::InterfaceUnavailable(InterfaceRole::InterfaceB)
    );
    assert_eq!(
        InterfacePolicy::PreferB.resolve(&set).unwrap_err(),
        OpenError::InterfaceUnavailable(InterfaceRole::InterfaceB)
    );
    assert_eq!(
        InterfacePolicy::BulkFirst.resolve(&set).unwrap_err(),
        OpenError::InterfaceUnavailable(InterfaceRole::InterfaceA)
    );
    assert_eq!(
        InterfacePolicy::ControlFirst.resolve(&set).unwrap_err(),
        OpenError::InterfaceUnavailable(InterfaceRole::InterfaceB)
    );
}

#[derive(Clone)]
struct SharedMock(std::sync::Arc<std::sync::Mutex<MockTransport>>);

impl monkey_core::transport::Transport for SharedMock {
    fn write_bulk(
        &mut self,
        report_id: u8,
        data: &[u8],
    ) -> Result<usize, monkey_core::error::TransportError> {
        self.0.lock().unwrap().write_bulk(report_id, data)
    }
    fn send_feature_report(
        &mut self,
        data: &[u8],
    ) -> Result<(), monkey_core::error::TransportError> {
        self.0.lock().unwrap().send_feature_report(data)
    }
    fn get_feature_report(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, monkey_core::error::TransportError> {
        self.0.lock().unwrap().get_feature_report(report_id, buf)
    }
    fn read_input_report(
        &mut self,
        buf: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, monkey_core::error::TransportError> {
        self.0.lock().unwrap().read_input_report(buf, timeout_ms)
    }
}

#[test]
fn test_monka_device_probe_read_only_guarantee() {
    let mut mock = MockTransport::new();
    // Simulate feature report 0 response with valid header and marker
    let mut resp = vec![0u8; 64];
    resp[0] = 0x04; // magic
    resp[1] = 0x01; // command / rev major
    resp[2] = 0x02; // rev minor
    resp[3] = 0x01; // ver major
    resp[4] = 0x00; // ver minor
    resp[5] = 0x03; // ver patch
    resp[14] = 0xAA; // marker 0
    resp[15] = 0x55; // marker 1
    mock.set_feature_response(0, resp);

    let shared = std::sync::Arc::new(std::sync::Mutex::new(mock));
    let set = make_dual_device_set();
    let mut device =
        MonkaDevice::from_transport(Box::new(SharedMock(shared.clone()))).with_device_set(set);

    let output = device.probe().expect("Probe should succeed");

    assert_eq!(output.model, "Monka 3075 Pro / RKGK890");
    assert!(output.interface_a_detected);
    assert!(output.interface_b_detected);
    assert!(output.read_only_verified);
    assert_eq!(output.hardware_revision, "rev1.2");
    assert_eq!(output.firmware_version, "v1.0.3");
    assert!(output
        .capabilities
        .contains(&"LCD display 128x128 RGB565".to_string()));
    assert!(output
        .capabilities
        .contains(&"dual composite interface".to_string()));

    // Verify zero writes were emitted during probe per D-12 and T-06-03
    shared
        .lock()
        .unwrap()
        .assert_no_writes()
        .expect("Probing must not emit any writes");
}

#[test]
fn test_monka_device_lcd_streaming() {
    let mock = MockTransport::new();
    let mut device = MonkaDevice::from_transport(Box::new(mock)).with_hardware_writes_allowed(true);

    let frame = [0x55u8; LCD_FRAME_BYTES];
    let config = LcdPacingConfig {
        inter_chunk_delay: Duration::ZERO,
        target_fps: 15,
    };

    let mut chunk_count = 0;
    let metrics = device
        .stream_frame_with_progress(&frame, config, |done, total| {
            chunk_count = done;
            assert_eq!(total, LCD_CHUNK_COUNT);
        })
        .expect("Stream frame should succeed");

    assert_eq!(metrics.chunks_sent, LCD_CHUNK_COUNT);
    assert_eq!(metrics.bytes_sent, LCD_FRAME_BYTES);
    assert_eq!(chunk_count, LCD_CHUNK_COUNT);
}

#[test]
fn test_monka_device_rgb_operations() {
    let mock = MockTransport::new();
    let mut device = MonkaDevice::from_transport(Box::new(mock)).with_hardware_writes_allowed(true);

    let config = LightingConfig {
        mode: LightingMode::Static,
        brightness: 100,
        speed: 50,
        direction: FlowDirection::LeftToRight,
        color: RgbColor::new(255, 0, 0),
    };

    // Apply preview
    device
        .apply_rgb_preview(&config)
        .expect("Preview should succeed");

    // Apply commit
    device
        .apply_rgb_commit(&config, false, None, false)
        .expect("Commit should succeed");
}

#[test]
fn test_monka_device_benchmarks() {
    let mock = MockTransport::new();
    let mut device = MonkaDevice::from_transport(Box::new(mock)).with_hardware_writes_allowed(true);

    let bench_config = BenchmarkConfig {
        duration: Duration::from_millis(100),
        frame_count: 2,
        iterations: 5,
        ..Default::default()
    };

    let tp = device
        .run_bulk_benchmark(&bench_config, |_, _| {})
        .expect("Bulk bench should succeed");
    assert!(tp.frames >= 2);

    let lat = device
        .run_transaction_benchmark(&bench_config, |_, _| {})
        .expect("Latency bench should succeed");
    assert_eq!(lat.samples, 5);
}

#[test]
fn test_monka_device_diagnose_structure() {
    let diag = MonkaDevice::diagnose();
    if diag.hid_init.is_ok() && diag.device_set.is_none() {
        assert_eq!(diag.interface_a_status, InterfaceCheckStatus::NotPresent);
        assert_eq!(diag.interface_b_status, InterfaceCheckStatus::NotPresent);
    }
}
