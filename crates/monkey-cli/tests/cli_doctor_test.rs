use monkey_core::doctor::{run_doctor_checks, CheckStatus, DoctorReport};

#[test]
fn test_doctor_mock_report_structure() {
    let report = run_doctor_checks(true);

    assert_eq!(report.summary.total, 6);
    assert_eq!(report.summary.passed, 6);
    assert_eq!(report.summary.warned, 0);
    assert_eq!(report.summary.failed, 0);
    assert!(report.summary.all_healthy);

    let check_ids: Vec<&str> = report.checks.iter().map(|c| c.id.as_str()).collect();
    assert!(check_ids.contains(&"platform"));
    assert!(check_ids.contains(&"hid_subsystem"));
    assert!(check_ids.contains(&"device_enumeration"));
    assert!(check_ids.contains(&"interface_a"));
    assert!(check_ids.contains(&"interface_b"));
    assert!(check_ids.contains(&"safety_rails"));

    for check in &report.checks {
        assert_eq!(check.status, CheckStatus::Pass);
        assert!(check.remediation.is_none());
    }
}

#[test]
fn test_doctor_report_json_serialization() {
    let report = run_doctor_checks(true);
    let json_str = serde_json::to_string_pretty(&report).expect("Failed to serialize DoctorReport");

    let deserialized: DoctorReport =
        serde_json::from_str(&json_str).expect("Failed to deserialize DoctorReport");
    assert_eq!(report, deserialized);
    assert!(json_str.contains("\"all_healthy\": true"));
}

#[test]
fn test_doctor_remediation_guidance() {
    use monkey_core::doctor::DiagnosticCheck;

    let mut report = run_doctor_checks(true);
    report.checks.push(DiagnosticCheck {
        id: "test_failure".into(),
        name: "Test Failure Check".into(),
        status: CheckStatus::Fail,
        message: "Simulated permission failure".into(),
        remediation: Some("Grant input permissions in system settings".into()),
    });

    let failed_checks: Vec<_> = report
        .checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail)
        .collect();
    assert_eq!(failed_checks.len(), 1);
    assert_eq!(
        failed_checks[0].remediation.as_deref(),
        Some("Grant input permissions in system settings")
    );
}
