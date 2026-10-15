use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{bounded, Receiver, Sender};
use monkey_core::doctor::{run_doctor_checks, CheckStatus};
use monkey_core::lcd::LCD_FRAME_BYTES;
use monkey_core::rgb::LightingConfig;
use monkey_core::transport::{MockTransport, Transport};

use crate::dto::*;
use crate::error::{DesktopError, DesktopResult};

/// Trait abstracting hardware transport / backend for testing without USB devices.
pub trait HardwareBackend: Send + 'static {
    fn is_mock(&self) -> bool;
    fn discover_devices(&mut self) -> DesktopResult<Vec<DiscoveredDeviceEntry>>;
    fn probe_device(&mut self, entry: &DiscoveredDeviceEntry) -> DesktopResult<ProbeDetails>;
    fn stream_lcd_frame(
        &mut self,
        entry: &DiscoveredDeviceEntry,
        frame: &[u8; LCD_FRAME_BYTES],
        cancelled: &AtomicBool,
    ) -> DesktopResult<()>;
    fn rgb_preview(
        &mut self,
        entry: &DiscoveredDeviceEntry,
        config: &LightingConfig,
    ) -> DesktopResult<()>;
    fn rgb_commit(
        &mut self,
        entry: &DiscoveredDeviceEntry,
        config: &LightingConfig,
        battery_percent: Option<u8>,
    ) -> DesktopResult<()>;
    fn rgb_readback(
        &mut self,
        entry: &DiscoveredDeviceEntry,
    ) -> DesktopResult<Option<LightingConfig>>;
}

#[derive(Debug, Clone)]
pub struct DiscoveredDeviceEntry {
    pub opaque_id: String,
    pub is_wireless: bool,
    pub has_interface_a: bool,
    pub has_interface_b: bool,
    pub label: String,
    pub battery_percent: Option<u8>,
    pub firmware: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProbeDetails {
    pub firmware_version: String,
    pub battery_percent: Option<u8>,
}

/// Mock implementation of [`HardwareBackend`] backed by a shared [`MockTransport`].
pub struct MockHardwareBackend {
    pub transport: Arc<Mutex<MockTransport>>,
    pub battery_percent: Option<u8>,
    pub is_wireless: bool,
    pub has_interface_a: bool,
    pub has_interface_b: bool,
    pub active_config: Option<LightingConfig>,
}

impl MockHardwareBackend {
    pub fn with_mock_transport(mock: MockTransport) -> (Self, Arc<Mutex<MockTransport>>) {
        let shared = Arc::new(Mutex::new(mock));
        let backend = Self {
            transport: Arc::clone(&shared),
            battery_percent: Some(95),
            is_wireless: false,
            has_interface_a: true,
            has_interface_b: true,
            active_config: None,
        };
        (backend, shared)
    }
}

impl HardwareBackend for MockHardwareBackend {
    fn is_mock(&self) -> bool {
        true
    }

    fn discover_devices(&mut self) -> DesktopResult<Vec<DiscoveredDeviceEntry>> {
        Ok(vec![DiscoveredDeviceEntry {
            opaque_id: "dev-mock-0".to_string(),
            is_wireless: self.is_wireless,
            has_interface_a: self.has_interface_a,
            has_interface_b: self.has_interface_b,
            label: "Monka 3075 Pro / RKGK890".to_string(),
            battery_percent: self.battery_percent,
            firmware: Some("v1.0.0".to_string()),
        }])
    }

    fn probe_device(&mut self, _entry: &DiscoveredDeviceEntry) -> DesktopResult<ProbeDetails> {
        let mut transport = self.transport.lock().unwrap();
        // Probe queries feature report 0 in a read-only manner
        let mut buf = [0u8; 65];
        let _ = transport.get_feature_report(0, &mut buf);
        Ok(ProbeDetails {
            firmware_version: "v1.0.0".to_string(),
            battery_percent: self.battery_percent,
        })
    }

    fn stream_lcd_frame(
        &mut self,
        _entry: &DiscoveredDeviceEntry,
        frame: &[u8; LCD_FRAME_BYTES],
        cancelled: &AtomicBool,
    ) -> DesktopResult<()> {
        if cancelled.load(Ordering::Relaxed) {
            return Err(DesktopError::Cancelled("Job was cancelled".to_string()));
        }
        let mut transport = self.transport.lock().unwrap();
        // 8 chunks of 4096 bytes
        for chunk_idx in 0..8 {
            if cancelled.load(Ordering::Relaxed) {
                return Err(DesktopError::Cancelled("Job was cancelled".to_string()));
            }
            let start = chunk_idx * 4096;
            let end = start + 4096;
            let chunk = &frame[start..end];
            transport
                .write_bulk(0, chunk)
                .map_err(|e| DesktopError::IoError(e.to_string()))?;
        }
        Ok(())
    }

    fn rgb_preview(
        &mut self,
        _entry: &DiscoveredDeviceEntry,
        config: &LightingConfig,
    ) -> DesktopResult<()> {
        let mut transport = self.transport.lock().unwrap();
        let packet = monkey_core::rgb::encode_rgb_control_packet(config)
            .map_err(|e| DesktopError::InvalidInput(e.to_string()))?;
        transport
            .send_feature_report(packet.as_bytes())
            .map_err(|e| DesktopError::IoError(e.to_string()))?;
        self.active_config = Some(config.clone());
        Ok(())
    }

    fn rgb_commit(
        &mut self,
        entry: &DiscoveredDeviceEntry,
        config: &LightingConfig,
        battery_percent: Option<u8>,
    ) -> DesktopResult<()> {
        if entry.is_wireless {
            match battery_percent {
                Some(b) if b < 20 => return Err(DesktopError::LowBattery(b)),
                None => {
                    return Err(DesktopError::LowBattery(0));
                }
                _ => {}
            }
        }
        let mut transport = self.transport.lock().unwrap();
        let packet = monkey_core::rgb::encode_rgb_control_packet(config)
            .map_err(|e| DesktopError::InvalidInput(e.to_string()))?;
        transport
            .send_feature_report(packet.as_bytes())
            .map_err(|e| DesktopError::IoError(e.to_string()))?;
        self.active_config = Some(config.clone());
        Ok(())
    }

    fn rgb_readback(
        &mut self,
        _entry: &DiscoveredDeviceEntry,
    ) -> DesktopResult<Option<LightingConfig>> {
        let mut transport = self.transport.lock().unwrap();
        let mut buf = [0u8; 65];
        let _ = transport.get_feature_report(0, &mut buf);
        Ok(self.active_config.clone())
    }
}

/// Commands sent to the background hardware worker thread.
pub enum HardwareWorkerCommand {
    ListDevices {
        resp: Sender<DesktopResult<Vec<DeviceSnapshot>>>,
    },
    Probe {
        device_id: String,
        resp: Sender<DesktopResult<DeviceSnapshot>>,
    },
    Diagnostics {
        resp: Sender<DesktopResult<Vec<DiagnosticCheckDto>>>,
    },
    StartLcd {
        device_id: String,
        frames: Vec<[u8; LCD_FRAME_BYTES]>,
        pacing_ms: u64,
        resp: Sender<DesktopResult<String>>,
    },
    StopLcd {
        job_id: String,
        resp: Sender<DesktopResult<bool>>,
    },
    JobStatus {
        job_id: String,
        resp: Sender<DesktopResult<JobSnapshot>>,
    },
    RgbStatus {
        device_id: String,
        resp: Sender<DesktopResult<RgbStatusResult>>,
    },
    RgbPreview {
        device_id: String,
        config: LightingConfigDto,
        resp: Sender<DesktopResult<RgbPreviewResult>>,
    },
    RgbCommit {
        device_id: String,
        resp: Sender<DesktopResult<RgbCommitResult>>,
    },
    Shutdown,
}

struct ActiveJob {
    device_id: String,
    cancelled: Arc<AtomicBool>,
    state: JobState,
    frames_sent: usize,
    error: Option<DesktopErrorPayload>,
}

/// Handle to communicate with the background OS worker thread.
pub struct HardwareWorkerHandle {
    sender: Sender<HardwareWorkerCommand>,
    join_handle: Option<JoinHandle<()>>,
}

impl HardwareWorkerHandle {
    pub fn spawn<B: HardwareBackend>(backend: B) -> Self {
        let (tx, rx) = bounded::<HardwareWorkerCommand>(64);
        let join_handle = std::thread::Builder::new()
            .name("monkey-hw-worker".to_string())
            .spawn(move || {
                run_hardware_worker(backend, rx);
            })
            .expect("Failed to spawn hardware worker thread");

        Self {
            sender: tx,
            join_handle: Some(join_handle),
        }
    }

    pub fn spawn_mock(mock: MockHardwareBackend) -> Self {
        Self::spawn(mock)
    }

    pub fn list_devices(&self) -> DesktopResult<Vec<DeviceSnapshot>> {
        let (resp_tx, resp_rx) = bounded(1);
        self.sender
            .send(HardwareWorkerCommand::ListDevices { resp: resp_tx })
            .map_err(|e| DesktopError::Internal(format!("Worker unavailable: {e}")))?;
        resp_rx
            .recv()
            .map_err(|e| DesktopError::Internal(format!("Worker response dropped: {e}")))?
    }

    pub fn probe(&self, device_id: &str) -> DesktopResult<DeviceSnapshot> {
        let (resp_tx, resp_rx) = bounded(1);
        self.sender
            .send(HardwareWorkerCommand::Probe {
                device_id: device_id.to_string(),
                resp: resp_tx,
            })
            .map_err(|e| DesktopError::Internal(format!("Worker unavailable: {e}")))?;
        resp_rx
            .recv()
            .map_err(|e| DesktopError::Internal(format!("Worker response dropped: {e}")))?
    }

    pub fn diagnostics(&self) -> DesktopResult<Vec<DiagnosticCheckDto>> {
        let (resp_tx, resp_rx) = bounded(1);
        self.sender
            .send(HardwareWorkerCommand::Diagnostics { resp: resp_tx })
            .map_err(|e| DesktopError::Internal(format!("Worker unavailable: {e}")))?;
        resp_rx
            .recv()
            .map_err(|e| DesktopError::Internal(format!("Worker response dropped: {e}")))?
    }

    pub fn start_lcd(
        &self,
        device_id: &str,
        frames: Vec<[u8; LCD_FRAME_BYTES]>,
        pacing_ms: u64,
    ) -> DesktopResult<String> {
        let (resp_tx, resp_rx) = bounded(1);
        self.sender
            .send(HardwareWorkerCommand::StartLcd {
                device_id: device_id.to_string(),
                frames,
                pacing_ms,
                resp: resp_tx,
            })
            .map_err(|e| DesktopError::Internal(format!("Worker unavailable: {e}")))?;
        resp_rx
            .recv()
            .map_err(|e| DesktopError::Internal(format!("Worker response dropped: {e}")))?
    }

    pub fn stop_lcd(&self, job_id: &str) -> DesktopResult<bool> {
        let (resp_tx, resp_rx) = bounded(1);
        self.sender
            .send(HardwareWorkerCommand::StopLcd {
                job_id: job_id.to_string(),
                resp: resp_tx,
            })
            .map_err(|e| DesktopError::Internal(format!("Worker unavailable: {e}")))?;
        resp_rx
            .recv()
            .map_err(|e| DesktopError::Internal(format!("Worker response dropped: {e}")))?
    }

    pub fn job_status(&self, job_id: &str) -> DesktopResult<JobSnapshot> {
        let (resp_tx, resp_rx) = bounded(1);
        self.sender
            .send(HardwareWorkerCommand::JobStatus {
                job_id: job_id.to_string(),
                resp: resp_tx,
            })
            .map_err(|e| DesktopError::Internal(format!("Worker unavailable: {e}")))?;
        resp_rx
            .recv()
            .map_err(|e| DesktopError::Internal(format!("Worker response dropped: {e}")))?
    }

    pub fn rgb_status(&self, device_id: &str) -> DesktopResult<RgbStatusResult> {
        let (resp_tx, resp_rx) = bounded(1);
        self.sender
            .send(HardwareWorkerCommand::RgbStatus {
                device_id: device_id.to_string(),
                resp: resp_tx,
            })
            .map_err(|e| DesktopError::Internal(format!("Worker unavailable: {e}")))?;
        resp_rx
            .recv()
            .map_err(|e| DesktopError::Internal(format!("Worker response dropped: {e}")))?
    }

    pub fn rgb_preview(
        &self,
        device_id: &str,
        config: LightingConfigDto,
    ) -> DesktopResult<RgbPreviewResult> {
        let (resp_tx, resp_rx) = bounded(1);
        self.sender
            .send(HardwareWorkerCommand::RgbPreview {
                device_id: device_id.to_string(),
                config,
                resp: resp_tx,
            })
            .map_err(|e| DesktopError::Internal(format!("Worker unavailable: {e}")))?;
        resp_rx
            .recv()
            .map_err(|e| DesktopError::Internal(format!("Worker response dropped: {e}")))?
    }

    pub fn rgb_commit(&self, device_id: &str) -> DesktopResult<RgbCommitResult> {
        let (resp_tx, resp_rx) = bounded(1);
        self.sender
            .send(HardwareWorkerCommand::RgbCommit {
                device_id: device_id.to_string(),
                resp: resp_tx,
            })
            .map_err(|e| DesktopError::Internal(format!("Worker unavailable: {e}")))?;
        resp_rx
            .recv()
            .map_err(|e| DesktopError::Internal(format!("Worker response dropped: {e}")))?
    }

    pub fn shutdown(mut self) {
        let _ = self.sender.send(HardwareWorkerCommand::Shutdown);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for HardwareWorkerHandle {
    fn drop(&mut self) {
        let _ = self.sender.send(HardwareWorkerCommand::Shutdown);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

fn run_hardware_worker<B: HardwareBackend>(
    mut backend: B,
    receiver: Receiver<HardwareWorkerCommand>,
) {
    let mut discovered: HashMap<String, DiscoveredDeviceEntry> = HashMap::new();
    let mut pending_previews: HashMap<String, LightingConfigDto> = HashMap::new();
    let mut active_jobs: HashMap<String, ActiveJob> = HashMap::new();
    let mut last_commit_time: Option<Instant> = None;
    let mut next_job_id: usize = 1;

    while let Ok(cmd) = receiver.recv() {
        match cmd {
            HardwareWorkerCommand::Shutdown => {
                // Cancel all active jobs
                for job in active_jobs.values() {
                    job.cancelled.store(true, Ordering::Relaxed);
                }
                break;
            }
            HardwareWorkerCommand::ListDevices { resp } => {
                let res = backend.discover_devices().map(|entries| {
                    let mut snapshots = Vec::new();
                    discovered.clear();
                    for entry in entries {
                        discovered.insert(entry.opaque_id.clone(), entry.clone());
                        snapshots.push(snapshot_from_entry(&entry));
                    }
                    snapshots
                });
                let _ = resp.send(res);
            }
            HardwareWorkerCommand::Probe { device_id, resp } => {
                let entry_opt = discovered.get(&device_id).cloned();
                let res = match entry_opt {
                    Some(entry) => backend.probe_device(&entry).map(|details| {
                        let mut snap = snapshot_from_entry(&entry);
                        snap.firmware = Some(details.firmware_version);
                        if let Some(bat) = details.battery_percent {
                            snap.battery_percent = Some(bat);
                        }
                        snap
                    }),
                    None => Err(DesktopError::NotConnected(format!(
                        "Device '{}' not found in discovered set",
                        device_id
                    ))),
                };
                let _ = resp.send(res);
            }
            HardwareWorkerCommand::Diagnostics { resp } => {
                let is_mock = backend.is_mock();
                let report = run_doctor_checks(is_mock);
                let mut checks_dto = Vec::new();
                for check in report.checks {
                    let status = match check.status {
                        CheckStatus::Pass => DiagnosticStatusDto::Pass,
                        CheckStatus::Warn => DiagnosticStatusDto::Warn,
                        CheckStatus::Fail => DiagnosticStatusDto::Fail,
                    };
                    // Sanitize message: strip paths or sensitive details
                    let sanitized_message = sanitize_diagnostic_detail(&check.message);
                    checks_dto.push(DiagnosticCheckDto {
                        id: check.id,
                        label: check.name,
                        status,
                        detail: sanitized_message,
                        remediation: check.remediation,
                    });
                }
                let _ = resp.send(Ok(checks_dto));
            }
            HardwareWorkerCommand::StartLcd {
                device_id,
                frames,
                pacing_ms,
                resp,
            } => {
                let entry_opt = discovered.get(&device_id).cloned();
                match entry_opt {
                    None => {
                        let _ = resp.send(Err(DesktopError::NotConnected(format!(
                            "Device '{}' not connected",
                            device_id
                        ))));
                    }
                    Some(entry) => {
                        if !entry.has_interface_a {
                            let _ = resp.send(Err(DesktopError::Unsupported(
                                "Device lacks Interface A (bulk LCD pipe)".to_string(),
                            )));
                            continue;
                        }
                        // Check if device is already busy with an active job
                        let is_busy = active_jobs.values().any(|j| {
                            j.device_id == device_id
                                && matches!(j.state, JobState::Running | JobState::Queued)
                        });
                        if is_busy {
                            let _ = resp.send(Err(DesktopError::Busy(
                                "Another LCD streaming job is currently active on this device"
                                    .to_string(),
                            )));
                            continue;
                        }

                        let job_id = format!("job-{}", next_job_id);
                        next_job_id += 1;

                        let cancelled = Arc::new(AtomicBool::new(false));
                        active_jobs.insert(
                            job_id.clone(),
                            ActiveJob {
                                device_id: device_id.clone(),
                                cancelled: Arc::clone(&cancelled),
                                state: JobState::Running,
                                frames_sent: 0,
                                error: None,
                            },
                        );

                        let _ = resp.send(Ok(job_id.clone()));

                        // Stream frames sequentially on the hardware worker thread
                        let total_frames = frames.len();
                        for (idx, frame) in frames.iter().enumerate() {
                            if cancelled.load(Ordering::Relaxed)
                                || drain_commands_while_streaming(
                                    &receiver,
                                    &mut active_jobs,
                                    &discovered,
                                    &job_id,
                                )
                            {
                                if let Some(job) = active_jobs.get_mut(&job_id) {
                                    job.state = JobState::Cancelled;
                                }
                                break;
                            }

                            match backend.stream_lcd_frame(&entry, frame, &cancelled) {
                                Ok(()) => {
                                    if let Some(job) = active_jobs.get_mut(&job_id) {
                                        job.frames_sent = idx + 1;
                                        if idx + 1 == total_frames {
                                            job.state = JobState::Completed;
                                        }
                                    }
                                    if pacing_ms > 0 && idx + 1 < total_frames {
                                        let end_time =
                                            Instant::now() + Duration::from_millis(pacing_ms);
                                        while Instant::now() < end_time {
                                            if cancelled.load(Ordering::Relaxed)
                                                || drain_commands_while_streaming(
                                                    &receiver,
                                                    &mut active_jobs,
                                                    &discovered,
                                                    &job_id,
                                                )
                                            {
                                                if let Some(job) = active_jobs.get_mut(&job_id) {
                                                    job.state = JobState::Cancelled;
                                                }
                                                break;
                                            }
                                            std::thread::sleep(Duration::from_millis(2));
                                        }
                                        if cancelled.load(Ordering::Relaxed) {
                                            break;
                                        }
                                    }
                                }
                                Err(DesktopError::Cancelled(_)) => {
                                    if let Some(job) = active_jobs.get_mut(&job_id) {
                                        job.state = JobState::Cancelled;
                                    }
                                    break;
                                }
                                Err(e) => {
                                    if let Some(job) = active_jobs.get_mut(&job_id) {
                                        job.state = JobState::Failed;
                                        job.error = Some(e.to_payload());
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            HardwareWorkerCommand::StopLcd { job_id, resp } => {
                match active_jobs.get_mut(&job_id) {
                    Some(job) => {
                        job.cancelled.store(true, Ordering::Relaxed);
                        if matches!(job.state, JobState::Running | JobState::Queued) {
                            job.state = JobState::Cancelled;
                        }
                        let _ = resp.send(Ok(true));
                    }
                    None => {
                        // Idempotent stop
                        let _ = resp.send(Ok(true));
                    }
                }
            }
            HardwareWorkerCommand::JobStatus { job_id, resp } => match active_jobs.get(&job_id) {
                Some(job) => {
                    let _ = resp.send(Ok(JobSnapshot {
                        id: job_id.clone(),
                        device_id: job.device_id.clone(),
                        state: job.state,
                        frames_sent: job.frames_sent,
                        error: job.error.clone(),
                    }));
                }
                None => {
                    let _ = resp.send(Err(DesktopError::InvalidInput(format!(
                        "Job '{}' not found",
                        job_id
                    ))));
                }
            },
            HardwareWorkerCommand::RgbStatus { device_id, resp } => {
                let entry_opt = discovered.get(&device_id).cloned();
                match entry_opt {
                    None => {
                        let _ = resp.send(Err(DesktopError::NotConnected(format!(
                            "Device '{}' not connected",
                            device_id
                        ))));
                    }
                    Some(entry) => {
                        let core_config_res = backend.rgb_readback(&entry);
                        let config_dto = match core_config_res {
                            Ok(Some(cfg)) => Some(LightingConfigDto::from_core_config(&cfg)),
                            _ => pending_previews.get(&device_id).cloned(),
                        };
                        let persistence = if pending_previews.contains_key(&device_id) {
                            PersistenceState::Ram
                        } else if config_dto.is_some() {
                            PersistenceState::Verified
                        } else {
                            PersistenceState::Unknown
                        };
                        let supported_modes = LightingConfigDto::SUPPORTED_MODES
                            .iter()
                            .map(|s| s.to_string())
                            .collect();
                        let _ = resp.send(Ok(RgbStatusResult {
                            config: config_dto,
                            persistence,
                            supported_modes,
                        }));
                    }
                }
            }
            HardwareWorkerCommand::RgbPreview {
                device_id,
                config,
                resp,
            } => {
                let entry_opt = discovered.get(&device_id).cloned();
                match entry_opt {
                    None => {
                        let _ = resp.send(Err(DesktopError::NotConnected(format!(
                            "Device '{}' not connected",
                            device_id
                        ))));
                    }
                    Some(entry) => {
                        if !entry.has_interface_b {
                            let _ = resp.send(Err(DesktopError::Unsupported(
                                "Device lacks Interface B (config/RGB pipe)".to_string(),
                            )));
                            continue;
                        }
                        if let Err(e) = config.validate_for_preview() {
                            let _ = resp.send(Err(e));
                            continue;
                        }
                        match config.to_core_config() {
                            Ok(core_config) => match backend.rgb_preview(&entry, &core_config) {
                                Ok(()) => {
                                    pending_previews.insert(device_id, config.clone());
                                    let _ = resp.send(Ok(RgbPreviewResult {
                                        config,
                                        persistence: PersistenceState::Ram,
                                    }));
                                }
                                Err(e) => {
                                    let _ = resp.send(Err(e));
                                }
                            },
                            Err(e) => {
                                let _ = resp.send(Err(e));
                            }
                        }
                    }
                }
            }
            HardwareWorkerCommand::RgbCommit { device_id, resp } => {
                let entry_opt = discovered.get(&device_id).cloned();
                match entry_opt {
                    None => {
                        let _ = resp.send(Err(DesktopError::NotConnected(format!(
                            "Device '{}' not connected",
                            device_id
                        ))));
                    }
                    Some(entry) => {
                        let preview_config = match pending_previews.get(&device_id) {
                            Some(cfg) => cfg.clone(),
                            None => {
                                let _ = resp.send(Err(DesktopError::UnsafeOperation(
                                    "No pending RGB preview to commit. Apply an RGB preview first"
                                        .to_string(),
                                )));
                                continue;
                            }
                        };

                        // Enforce 500ms debounce since last commit
                        if let Some(last) = last_commit_time {
                            let elapsed = last.elapsed();
                            if elapsed < Duration::from_millis(500) {
                                std::thread::sleep(Duration::from_millis(500) - elapsed);
                            }
                        }

                        match preview_config.to_core_config() {
                            Ok(core_config) => {
                                match backend.rgb_commit(
                                    &entry,
                                    &core_config,
                                    entry.battery_percent,
                                ) {
                                    Ok(()) => {
                                        last_commit_time = Some(Instant::now());
                                        // Once committed, remove from pending preview so subsequent commit fails safely
                                        pending_previews.remove(&device_id);
                                        let _ = resp.send(Ok(RgbCommitResult {
                                            persistence: PersistenceState::Verified,
                                        }));
                                    }
                                    Err(e) => {
                                        let _ = resp.send(Err(e));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = resp.send(Err(e));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn drain_commands_while_streaming(
    receiver: &Receiver<HardwareWorkerCommand>,
    active_jobs: &mut HashMap<String, ActiveJob>,
    discovered: &HashMap<String, DiscoveredDeviceEntry>,
    current_job_id: &str,
) -> bool {
    let mut should_cancel_current = false;
    while let Ok(cmd) = receiver.try_recv() {
        match cmd {
            HardwareWorkerCommand::Shutdown => {
                for job in active_jobs.values() {
                    job.cancelled.store(true, Ordering::Relaxed);
                }
                should_cancel_current = true;
            }
            HardwareWorkerCommand::StopLcd { job_id, resp } => {
                if let Some(job) = active_jobs.get_mut(&job_id) {
                    job.cancelled.store(true, Ordering::Relaxed);
                    if matches!(job.state, JobState::Running | JobState::Queued) {
                        job.state = JobState::Cancelled;
                    }
                    if job_id == current_job_id {
                        should_cancel_current = true;
                    }
                }
                let _ = resp.send(Ok(true));
            }
            HardwareWorkerCommand::JobStatus { job_id, resp } => match active_jobs.get(&job_id) {
                Some(j) => {
                    let _ = resp.send(Ok(JobSnapshot {
                        id: job_id.clone(),
                        device_id: j.device_id.clone(),
                        state: j.state,
                        frames_sent: j.frames_sent,
                        error: j.error.clone(),
                    }));
                }
                None => {
                    let _ = resp.send(Err(DesktopError::InvalidInput(format!(
                        "Job '{job_id}' not found"
                    ))));
                }
            },
            HardwareWorkerCommand::ListDevices { resp } => {
                let snapshots: Vec<DeviceSnapshot> =
                    discovered.values().map(snapshot_from_entry).collect();
                let _ = resp.send(Ok(snapshots));
            }
            HardwareWorkerCommand::StartLcd { resp, .. } => {
                let _ = resp.send(Err(DesktopError::Busy(
                    "Another LCD streaming job is currently active".to_string(),
                )));
            }
            HardwareWorkerCommand::RgbPreview { resp, .. } => {
                let _ = resp.send(Err(DesktopError::Busy(
                    "Cannot perform RGB write while LCD streaming is active".to_string(),
                )));
            }
            HardwareWorkerCommand::RgbCommit { resp, .. } => {
                let _ = resp.send(Err(DesktopError::Busy(
                    "Cannot perform RGB commit while LCD streaming is active".to_string(),
                )));
            }
            HardwareWorkerCommand::Probe { resp, .. } => {
                let _ = resp.send(Err(DesktopError::Busy(
                    "Cannot probe while LCD streaming is active".to_string(),
                )));
            }
            HardwareWorkerCommand::Diagnostics { resp } => {
                let _ = resp.send(Err(DesktopError::Busy(
                    "Cannot run diagnostics while LCD streaming is active".to_string(),
                )));
            }
            HardwareWorkerCommand::RgbStatus { resp, .. } => {
                let _ = resp.send(Err(DesktopError::Busy(
                    "Cannot query RGB status while LCD streaming is active".to_string(),
                )));
            }
        }
    }
    should_cancel_current
}

fn snapshot_from_entry(entry: &DiscoveredDeviceEntry) -> DeviceSnapshot {
    let connection = if entry.is_wireless {
        ConnectionType::Wireless
    } else {
        ConnectionType::Wired
    };
    DeviceSnapshot {
        id: entry.opaque_id.clone(),
        label: entry.label.clone(),
        connection,
        interfaces: InterfacesDto {
            lcd: entry.has_interface_a,
            config: entry.has_interface_b,
        },
        capabilities: CapabilitiesDto {
            lcd: entry.has_interface_a,
            rgb: entry.has_interface_b,
            persist: entry.has_interface_b,
            remap: false,
            macro_: false,
            firmware: false,
        },
        battery_percent: entry.battery_percent,
        firmware: entry.firmware.clone(),
    }
}

fn sanitize_diagnostic_detail(detail: &str) -> String {
    // Sanitize any potential device paths or internal IDs
    let mut out = detail.to_string();
    if let Some(idx) = out.find("/dev/") {
        let end = out[idx..].find(' ').map(|sp| idx + sp).unwrap_or(out.len());
        out.replace_range(idx..end, "[redacted_device_path]");
    }
    out
}
