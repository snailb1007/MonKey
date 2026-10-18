use monkey_core::device::{
    classify_interface, group_monka_devices, DiscoveredDevice, InterfaceRole, MONKA_PID, MONKA_VID,
    PRODUCT_IDENTIFIER,
};
use std::ffi::CString;

fn make_test_device(
    usage_page: u16,
    usage: u16,
    interface_number: i32,
    path_str: &str,
    serial: Option<&str>,
) -> DiscoveredDevice {
    DiscoveredDevice {
        vid: MONKA_VID,
        pid: MONKA_PID,
        manufacturer: Some("Shenzhen HFD Technology Co., Ltd.".to_string()),
        product: Some(PRODUCT_IDENTIFIER.to_string()),
        serial_number: serial.map(|s| s.to_string()),
        path: CString::new(path_str).expect("Valid CString"),
        usage_page,
        usage,
        interface_number,
    }
}

#[test]
fn test_device_info_json_constants_match() {
    assert_eq!(MONKA_VID, 0x05AC, "VID must be 0x05AC (1452) per D-01");
    assert_eq!(MONKA_PID, 0x024F, "PID must be 0x024F (591) per D-01");
    assert_eq!(
        PRODUCT_IDENTIFIER, "RKGK890",
        "Product identifier must match RKGK890 per D-01"
    );
}

#[test]
fn test_classify_interface_a_bulk_pipe() {
    let role = classify_interface(0xFF68, 0x0061, 0);
    assert_eq!(role, Some(InterfaceRole::InterfaceA));

    let dev = make_test_device(0xFF68, 0x0061, 0, "dev_a", None);
    assert_eq!(dev.role(), Some(InterfaceRole::InterfaceA));
}

#[test]
fn test_classify_interface_b_control_pipe_standard() {
    // Host-policy conditional: the legacy Mac/Linux branch keeps the Consumer
    // Control alias; the strict Windows policy rejects it (GH #18 / D-03).
    if cfg!(target_os = "windows") {
        assert_eq!(classify_interface(0x000C, 0x0001, 1), None);
    } else {
        let role = classify_interface(0x000C, 0x0001, 1);
        assert_eq!(role, Some(InterfaceRole::InterfaceB));

        let dev = make_test_device(0x000C, 0x0001, 1, "dev_b", None);
        assert_eq!(dev.role(), Some(InterfaceRole::InterfaceB));
    }
}

#[test]
fn test_classify_interface_b_control_pipe_vendor() {
    // Host-policy conditional: on Windows only the real 65-byte feature report
    // collection (0xFF13, 0x0001) on MI_03 is Interface B; (0xFFFF, 0x0001) on
    // MI_01&COL05 has FeatureReportByteLength=0 and is rejected.
    if cfg!(target_os = "windows") {
        assert_eq!(
            classify_interface(0xFF13, 0x0001, 3),
            Some(InterfaceRole::InterfaceB)
        );
        assert_eq!(classify_interface(0xFFFF, 0x0001, 1), None);
        assert_eq!(classify_interface(0xFFFF, 0x0000, 1), None);
    } else {
        let role_vendor_1 = classify_interface(0xFFFF, 0x0001, 1);
        assert_eq!(role_vendor_1, Some(InterfaceRole::InterfaceB));

        let role_vendor_0 = classify_interface(0xFFFF, 0x0000, 1);
        assert_eq!(role_vendor_0, Some(InterfaceRole::InterfaceB));
    }
}

#[test]
fn test_classify_interface_fallback_to_interface_number() {
    // When usage page and usage are 0 (e.g. Linux libusb backend without hidraw).
    // The strict Windows policy rejects missing usage metadata instead of
    // guessing from interface_number (GH #18 / D-03).
    if cfg!(target_os = "windows") {
        assert_eq!(classify_interface(0, 0, 0), None);
        assert_eq!(classify_interface(0, 0, 1), None);
    } else {
        assert_eq!(classify_interface(0, 0, 0), Some(InterfaceRole::InterfaceA));
        assert_eq!(classify_interface(0, 0, 1), Some(InterfaceRole::InterfaceB));
    }
    assert_eq!(classify_interface(0, 0, 2), None);
    assert_eq!(classify_interface(0, 0, -1), None);
}

#[test]
fn test_classify_unrelated_hid_interfaces_rejected() {
    // Standard keyboard (Generic Desktop 0x0001 / Keyboard 0x0006)
    assert_eq!(classify_interface(0x0001, 0x0006, 0), None);
    // Mouse (Generic Desktop 0x0001 / Mouse 0x0002)
    assert_eq!(classify_interface(0x0001, 0x0002, 1), None);
    // Unrelated vendor page
    assert_eq!(classify_interface(0xFF00, 0x0001, 0), None);
}

fn vendor_b_usage_page() -> u16 {
    if cfg!(target_os = "windows") {
        0xFF13
    } else {
        0xFFFF
    }
}

#[test]
fn test_group_devices_deduplicates_macos_multi_usage_records() {
    // macOS hidapi duplicates IOHIDDevice records when multiple usage pairs are present.
    // Interface B typically yields both (0x000C, 0x0001) and (0xFFFF, 0x0001) sharing path DevSrvsID:1002.
    let dev_a = make_test_device(0xFF68, 0x0061, 0, "DevSrvsID:1001", Some("SN-12345"));
    let dev_b_primary = make_test_device(0x000C, 0x0001, 1, "DevSrvsID:1002", Some("SN-12345"));
    let dev_b_vendor = make_test_device(
        vendor_b_usage_page(),
        0x0001,
        1,
        "DevSrvsID:1002",
        Some("SN-12345"),
    );
    let unrelated = make_test_device(0x0001, 0x0006, 2, "DevSrvsID:1003", Some("SN-12345"));

    let devices = vec![
        dev_a.clone(),
        dev_b_primary.clone(),
        dev_b_vendor.clone(),
        unrelated,
    ];
    let sets = group_monka_devices(&devices);

    assert_eq!(
        sets.len(),
        1,
        "Multi-record descriptor should group into exactly one device set"
    );
    let set = &sets[0];

    assert!(
        set.is_complete(),
        "Device set should be complete with both Interface A and B"
    );
    assert!(set.has_interface_a());
    assert!(set.has_interface_b());

    let if_a = set.interface_a.as_ref().expect("Interface A present");
    assert_eq!(if_a.path, CString::new("DevSrvsID:1001").unwrap());
    assert_eq!(if_a.usage_page, 0xFF68);

    let if_b = set.interface_b.as_ref().expect("Interface B present");
    assert_eq!(if_b.path, CString::new("DevSrvsID:1002").unwrap());
    // Host-policy conditional: the legacy branch keeps the Consumer Control
    // alias as the first B candidate; the strict Windows policy drops it, so
    // the vendor tuple (0xFF13, 0x0001) supplies Interface B instead (GH #18 / D-03).
    if cfg!(target_os = "windows") {
        assert_eq!(if_b.usage_page, 0xFF13);
        assert_eq!(set.observed_usages.len(), 2);
        assert!(set.observed_usages.contains(&(0xFF68, 0x0061)));
        assert!(set.observed_usages.contains(&(0xFF13, 0x0001)));
    } else {
        assert_eq!(if_b.usage_page, 0x000C);
        assert_eq!(set.observed_usages.len(), 3);
        assert!(set.observed_usages.contains(&(0xFF68, 0x0061)));
        assert!(set.observed_usages.contains(&(0x000C, 0x0001)));
        assert!(set.observed_usages.contains(&(0xFFFF, 0x0001)));
    }
}

#[test]
fn test_group_devices_isolates_multiple_physical_keyboards() {
    let b_page = vendor_b_usage_page();
    let kb1_a = make_test_device(0xFF68, 0x0061, 0, "path_kb1_a", Some("KEYBOARD-A"));
    let kb1_b = make_test_device(b_page, 0x0001, 1, "path_kb1_b", Some("KEYBOARD-A"));

    let kb2_a = make_test_device(0xFF68, 0x0061, 0, "path_kb2_a", Some("KEYBOARD-B"));
    let kb2_b = make_test_device(b_page, 0x0001, 1, "path_kb2_b", Some("KEYBOARD-B"));

    let devices = vec![kb1_a, kb2_a, kb1_b, kb2_b];
    let sets = group_monka_devices(&devices);

    assert_eq!(
        sets.len(),
        2,
        "Should create two distinct sets for two distinct serials"
    );

    let set1 = sets
        .iter()
        .find(|s| s.serial_number.as_deref() == Some("KEYBOARD-A"))
        .expect("KB1 found");
    assert!(set1.is_complete());
    assert_eq!(
        set1.interface_a.as_ref().unwrap().path,
        CString::new("path_kb1_a").unwrap()
    );
    assert_eq!(
        set1.interface_b.as_ref().unwrap().path,
        CString::new("path_kb1_b").unwrap()
    );

    let set2 = sets
        .iter()
        .find(|s| s.serial_number.as_deref() == Some("KEYBOARD-B"))
        .expect("KB2 found");
    assert!(set2.is_complete());
    assert_eq!(
        set2.interface_a.as_ref().unwrap().path,
        CString::new("path_kb2_a").unwrap()
    );
    assert_eq!(
        set2.interface_b.as_ref().unwrap().path,
        CString::new("path_kb2_b").unwrap()
    );
}

#[test]
fn test_group_devices_without_serial_numbers() {
    // When no serial number is provided, single set of incomplete interfaces is safely paired
    let dev_a = make_test_device(0xFF68, 0x0061, 0, "dev_no_sn_a", None);
    let dev_b = make_test_device(vendor_b_usage_page(), 0x0001, 1, "dev_no_sn_b", None);

    let sets = group_monka_devices(&[dev_a, dev_b]);
    assert_eq!(sets.len(), 1);
    assert!(sets[0].is_complete());
}

#[test]
fn test_group_devices_macos_devsrvsid_proximity() {
    let b_page = vendor_b_usage_page();
    // macOS DevSrvsID proximity correctly groups two separate physical keyboards with no serial numbers
    // KB1: DevSrvsID:4295837800 (A) and DevSrvsID:4295837807 (B) -> delta 7 <= 32
    // KB2: DevSrvsID:4295900000 (A) and DevSrvsID:4295900007 (B) -> delta 7 <= 32
    let kb1_a = make_test_device(0xFF68, 0x0061, 0, "DevSrvsID:4295837800", None);
    let kb1_b = make_test_device(b_page, 0x0001, 1, "DevSrvsID:4295837807", None);

    let kb2_a = make_test_device(0xFF68, 0x0061, 0, "DevSrvsID:4295900000", None);
    let kb2_b = make_test_device(b_page, 0x0001, 1, "DevSrvsID:4295900007", None);

    // Interleaved discovery order
    let sets = group_monka_devices(&[kb1_a, kb2_a, kb2_b, kb1_b]);
    assert_eq!(
        sets.len(),
        2,
        "Must group into exactly 2 sets based on DevSrvsID proximity"
    );

    let set1 = sets
        .iter()
        .find(|s| {
            s.interface_a.as_ref().map(|d| d.path.to_str().unwrap()) == Some("DevSrvsID:4295837800")
        })
        .expect("KB1 found");
    assert_eq!(
        set1.interface_b.as_ref().map(|d| d.path.to_str().unwrap()),
        Some("DevSrvsID:4295837807")
    );

    let set2 = sets
        .iter()
        .find(|s| {
            s.interface_a.as_ref().map(|d| d.path.to_str().unwrap()) == Some("DevSrvsID:4295900000")
        })
        .expect("KB2 found");
    assert_eq!(
        set2.interface_b.as_ref().map(|d| d.path.to_str().unwrap()),
        Some("DevSrvsID:4295900007")
    );
}

#[test]
fn test_group_devices_multiple_devices_closest_matching() {
    let b_page = vendor_b_usage_page();
    // Two keyboards with DevSrvsID 1000/1007 and 1010/1017
    let kb1_a = make_test_device(0xFF68, 0x0061, 0, "DevSrvsID:1000", None);
    let kb2_a = make_test_device(0xFF68, 0x0061, 0, "DevSrvsID:1010", None);
    let kb2_b = make_test_device(b_page, 0x0001, 1, "DevSrvsID:1017", None);
    let kb1_b = make_test_device(b_page, 0x0001, 1, "DevSrvsID:1007", None);

    // Pass in mixed order: 1000, 1010, 1017, 1007
    let sets = group_monka_devices(&[kb1_a, kb2_a, kb2_b, kb1_b]);
    assert_eq!(sets.len(), 2);

    let set1 = sets
        .iter()
        .find(|s| {
            s.interface_a.as_ref().map(|d| d.path.to_str().unwrap()) == Some("DevSrvsID:1000")
        })
        .expect("KB1 found");
    assert_eq!(
        set1.interface_b.as_ref().map(|d| d.path.to_str().unwrap()),
        Some("DevSrvsID:1007")
    );

    let set2 = sets
        .iter()
        .find(|s| {
            s.interface_a.as_ref().map(|d| d.path.to_str().unwrap()) == Some("DevSrvsID:1010")
        })
        .expect("KB2 found");
    assert_eq!(
        set2.interface_b.as_ref().map(|d| d.path.to_str().unwrap()),
        Some("DevSrvsID:1017")
    );
}

#[test]
fn test_group_devices_ambiguous_devices_without_keys_do_not_cross_pair() {
    let b_page = vendor_b_usage_page();
    // When multiple unkeyed devices have non-proximity paths, they must not be cross-paired
    let kb1_a = make_test_device(0xFF68, 0x0061, 0, "mock_path_1_a", None);
    let kb2_a = make_test_device(0xFF68, 0x0061, 0, "mock_path_2_a", None);
    let kb1_b = make_test_device(b_page, 0x0001, 1, "mock_path_1_b", None);
    let kb2_b = make_test_device(b_page, 0x0001, 1, "mock_path_2_b", None);

    let sets = group_monka_devices(&[kb1_a, kb2_a, kb2_b, kb1_b]);
    // Since ambiguous, no cross-pairing occurs: none of the sets should pair mock_path_1_a with mock_path_2_b
    for s in &sets {
        if s.interface_a.as_ref().map(|d| d.path.to_str().unwrap()) == Some("mock_path_1_a") {
            assert_ne!(
                s.interface_b.as_ref().map(|d| d.path.to_str().unwrap()),
                Some("mock_path_2_b")
            );
        }
    }
}

#[test]
fn test_isp_bootloader_pid_detection_and_rejection() {
    use monkey_core::device::{is_isp_bootloader, open_device_path, ISP_BOOTLOADER_PID, MONKA_VID};

    assert!(is_isp_bootloader(MONKA_VID, ISP_BOOTLOADER_PID));
    assert!(is_isp_bootloader(0x1234, ISP_BOOTLOADER_PID));
    assert!(!is_isp_bootloader(MONKA_VID, 0x024F));

    let mut dev = make_test_device(0xFF68, 0x0061, 0, "mock_path_isp", None);
    dev.pid = ISP_BOOTLOADER_PID;

    // Fake API call or verify open_device_path directly rejects ISP bootloader
    if let Ok(api) = monkey_core::device::init_hidapi() {
        let err = open_device_path(&api, &dev).unwrap_err();
        assert!(matches!(
            err,
            monkey_core::error::TransportError::ProtocolViolation(_)
        ));
    }
}

// --- 08-01 T2: Windows interface policy (strict classification, no quirk table) ---

/// Builds a Windows hidapi device path for interface `mi` (`mi_00`, `mi_01`, ...)
/// on one physical keyboard identified by `instance`.
fn win_instance_path(mi: &str, instance: &str) -> String {
    format!("\\\\?\\hid#vid_05ac&pid_024f&{mi}#{instance}#{{4d1e55b2-f16f-11cf-88cb-001111000030}}")
}

#[test]
fn test_classify_windows_vendor_preview_collection() {
    // Windows enumerates the 65-byte RGB control collection on MI_03 as
    // Usage Page 0xFF13 / Usage 0x0001; the legacy Mac/Linux branch still
    // classifies 0xFFFF / 0x0001.
    assert_eq!(
        classify_interface(vendor_b_usage_page(), 0x0001, 3),
        Some(InterfaceRole::InterfaceB)
    );
}

#[test]
fn test_classify_quirk_tolerant_unknown_vendor_usage_kept() {
    // Unknown usages on the vendor page still route to B on the legacy
    // Mac/Linux branch. The strict Windows policy rejects them (GH #18 / D-03).
    if cfg!(target_os = "windows") {
        assert_eq!(classify_interface(0xFFFF, 0x0002, 1), None);
    } else {
        assert_eq!(
            classify_interface(0xFFFF, 0x0002, 1),
            Some(InterfaceRole::InterfaceB)
        );
    }
}

#[test]
fn test_group_a_only_singleton_stays_unpaired() {
    let a = make_test_device(
        0xFF68,
        0x0061,
        0,
        &win_instance_path("mi_00", "8&2d0c3b4c&0&0000"),
        None,
    );
    let sets = group_monka_devices(&[a]);
    assert_eq!(sets.len(), 1);
    assert!(sets[0].interface_a.is_some());
    assert!(sets[0].interface_b.is_none());
}

#[test]
fn test_group_b_only_singleton_stays_unpaired() {
    let b = make_test_device(
        vendor_b_usage_page(),
        0x0001,
        3,
        &win_instance_path("mi_03", "8&2d0c3b4c&0&0000"),
        None,
    );
    let sets = group_monka_devices(&[b]);
    assert_eq!(sets.len(), 1);
    assert!(sets[0].interface_a.is_none());
    assert!(sets[0].interface_b.is_some());
}

#[test]
fn test_group_zero_usage_fallback_pairs_with_proximity() {
    // (0,0) usages still pair via DevSrvsID proximity on macOS-style paths.
    // Under the strict Windows policy missing usage metadata is rejected
    // outright, so no sets form (GH #18 / D-03).
    let a = make_test_device(0, 0, 0, "DevSrvsID:1001", None);
    let b = make_test_device(0, 0, 1, "DevSrvsID:1007", None);
    let sets = group_monka_devices(&[a, b]);
    if cfg!(target_os = "windows") {
        assert!(
            sets.is_empty(),
            "zero-usage candidates must not group under the Windows policy"
        );
    } else {
        assert_eq!(sets.len(), 1);
        assert!(sets[0].is_complete());
    }
}

#[test]
fn test_group_unrelated_only_yields_no_sets() {
    let kb = make_test_device(0x0001, 0x0006, 0, "DevSrvsID:1003", None);
    let mouse = make_test_device(0x0001, 0x0002, 1, "DevSrvsID:1004", None);
    let sets = group_monka_devices(&[kb, mouse]);
    assert!(
        sets.is_empty(),
        "Unrelated HID interfaces must not form sets"
    );
}

#[cfg(target_os = "windows")]
#[test]
fn test_group_windows_interfaces_pair_into_one_set() {
    // Native-Windows grouping: the strict policy classifies both exact vendor
    // tuples (MI_02 0xFF68:0x0061 and MI_03 0xFF13:0x0001), and production
    // grouping pairs them into a single complete set.
    let a_path = win_instance_path("mi_02", "8&2d0c3b4c&0&0000");
    let b_path = win_instance_path("mi_03", "8&2d0c3b4c&0&0000");
    let a = make_test_device(0xFF68, 0x0061, 2, &a_path, None);
    let b = make_test_device(0xFF13, 0x0001, 3, &b_path, None);
    let sets = group_monka_devices(&[a, b]);
    assert_eq!(sets.len(), 1);
    let set = &sets[0];
    assert!(set.is_complete());
}

#[cfg(target_os = "windows")]
#[test]
fn test_group_two_windows_keyboards_do_not_cross_pair() {
    // Reliable serial same-identity A/B pairs: without serials the two
    // unkeyed A candidates are ambiguous and default-deny isolation yields
    // three singletons, so serials are the supported pairing signal here.
    let kb1_a = make_test_device(
        0xFF68,
        0x0061,
        2,
        &win_instance_path("mi_02", "8&2d0c3b4c&0&0000"),
        Some("KB1"),
    );
    let kb1_b = make_test_device(
        0xFF13,
        0x0001,
        3,
        &win_instance_path("mi_03", "8&2d0c3b4c&0&0000"),
        Some("KB1"),
    );
    let kb2_a = make_test_device(
        0xFF68,
        0x0061,
        2,
        &win_instance_path("mi_02", "9&4e5f6a7b&0&0000"),
        Some("KB2"),
    );
    let sets = group_monka_devices(&[kb1_a, kb1_b, kb2_a]);
    assert_eq!(sets.len(), 2);

    let complete = sets.iter().find(|s| s.is_complete()).expect("KB1 paired");
    assert!(complete.interface_a.is_some());
    assert!(complete.interface_b.is_some());

    let singleton = sets
        .iter()
        .find(|s| !s.is_complete())
        .expect("KB2 singleton");
    assert!(singleton.interface_b.is_none());
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_group_consumer_b_pairs_on_legacy_policy() {
    // Legacy Mac/Linux behavior: Consumer Control (0x000C, 0x0001) still
    // supplies Interface B outside the strict Windows policy.
    let a = make_test_device(0xFF68, 0x0061, 0, "DevSrvsID:2001", Some("LEGACY-A"));
    let consumer_b = make_test_device(0x000C, 0x0001, 1, "DevSrvsID:2007", Some("LEGACY-A"));
    let sets = group_monka_devices(&[a, consumer_b]);
    assert_eq!(sets.len(), 1, "Consumer-B must pair on legacy policy");
    assert!(sets[0].is_complete(), "legacy Consumer-B completes the set");
}
