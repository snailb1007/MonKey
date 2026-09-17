use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::error::TransportError;
use crate::transport::Transport;

/// Captures an invocation of a [`Transport`] method for deterministic test assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportCall {
    /// Invocations of [`Transport::write_bulk`].
    WriteBulk { report_id: u8, data: Vec<u8> },
    /// Invocations of [`Transport::send_feature_report`].
    SendFeature { data: Vec<u8> },
    /// Invocations of [`Transport::get_feature_report`].
    GetFeature { report_id: u8 },
    /// Invocations of [`Transport::read_input_report`].
    ReadInput { timeout_ms: i32 },
}

/// Deterministic, in-memory transport mock supporting call recording,
/// canned feature responses, FIFO input queues, and error injection.
#[derive(Debug, Default)]
pub struct MockTransport {
    calls: Vec<TransportCall>,
    feature_responses: HashMap<u8, Vec<u8>>,
    input_queue: VecDeque<Vec<u8>>,
    injected_error: Option<TransportError>,
    /// Inverted so that `Default` leaves recording enabled.
    suppress_recording: bool,
}

impl MockTransport {
    /// Creates a new empty `MockTransport`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a canned feature response returned by [`Transport::get_feature_report`] for a given `report_id`.
    pub fn set_feature_response(&mut self, report_id: u8, bytes: impl Into<Vec<u8>>) {
        self.feature_responses.insert(report_id, bytes.into());
    }

    /// Enqueues an input report returned in FIFO order by [`Transport::read_input_report`].
    pub fn queue_input_report(&mut self, bytes: impl Into<Vec<u8>>) {
        self.input_queue.push_back(bytes.into());
    }

    /// Injects an error that causes subsequent transport calls to fail immediately without side effects.
    pub fn inject_error(&mut self, error: TransportError) {
        self.injected_error = Some(error);
    }

    /// Clears any previously injected error.
    pub fn clear_injected_error(&mut self) {
        self.injected_error = None;
    }

    /// Toggles call recording.
    ///
    /// Recording is on by default. Long-running synthetic benchmarks disable it so the
    /// call history does not grow without bound; [`Self::assert_no_writes`] refuses to
    /// vouch for a transport whose history was suppressed.
    pub fn set_recording(&mut self, enabled: bool) {
        self.suppress_recording = !enabled;
    }

    /// Whether calls are currently being recorded.
    pub fn is_recording(&self) -> bool {
        !self.suppress_recording
    }

    /// Returns a slice of all recorded calls in FIFO invocation order.
    pub fn calls(&self) -> &[TransportCall] {
        &self.calls
    }

    /// Clears the recorded calls history.
    pub fn clear_calls(&mut self) {
        self.calls.clear();
    }

    /// Asserts that no write operations (`write_bulk` or `send_feature_report`) were invoked.
    ///
    /// Fulfills D-12 read-only probing safety invariant verification.
    pub fn assert_no_writes(&self) -> Result<(), String> {
        if self.suppress_recording {
            return Err("Cannot assert absence of writes: call recording was disabled".to_string());
        }

        let writes: Vec<&TransportCall> = self
            .calls
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    TransportCall::WriteBulk { .. } | TransportCall::SendFeature { .. }
                )
            })
            .collect();

        if writes.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Expected no write calls, but found {} write(s): {:?}",
                writes.len(),
                writes
            ))
        }
    }
}

impl Transport for MockTransport {
    fn write_bulk(&mut self, report_id: u8, data: &[u8]) -> Result<usize, TransportError> {
        if let Some(err) = &self.injected_error {
            return Err(err.clone());
        }

        if !self.suppress_recording {
            self.calls.push(TransportCall::WriteBulk {
                report_id,
                data: data.to_vec(),
            });
        }

        Ok(data.len())
    }

    fn send_feature_report(&mut self, data: &[u8]) -> Result<(), TransportError> {
        if let Some(err) = &self.injected_error {
            return Err(err.clone());
        }

        if !self.suppress_recording {
            self.calls.push(TransportCall::SendFeature {
                data: data.to_vec(),
            });
        }

        Ok(())
    }

    fn get_feature_report(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, TransportError> {
        if let Some(err) = &self.injected_error {
            return Err(err.clone());
        }

        if !self.suppress_recording {
            self.calls.push(TransportCall::GetFeature { report_id });
        }

        if buf.is_empty() {
            return Err(TransportError::BufferTooSmall {
                needed: 1,
                provided: 0,
            });
        }

        match self.feature_responses.get(&report_id) {
            Some(canned) => {
                // If canned already starts with report_id (pre-framed):
                if !canned.is_empty()
                    && canned[0] == report_id
                    && (canned.len() > 64 || report_id != 0 || canned.len() == buf.len())
                {
                    if buf.len() < canned.len() {
                        Err(TransportError::BufferTooSmall {
                            needed: canned.len(),
                            provided: buf.len(),
                        })
                    } else {
                        buf[..canned.len()].copy_from_slice(canned);
                        Ok(canned.len())
                    }
                } else {
                    // Canned is raw payload without Report ID prefix:
                    // Seed buf[0] = report_id and copy payload into buf[1..] to align with HidTransport hardware layout.
                    let needed = 1 + canned.len();
                    if buf.len() < needed {
                        Err(TransportError::BufferTooSmall {
                            needed,
                            provided: buf.len(),
                        })
                    } else {
                        buf[0] = report_id;
                        buf[1..needed].copy_from_slice(canned);
                        Ok(needed)
                    }
                }
            }
            None => Err(TransportError::InvalidReportId(report_id)),
        }
    }

    fn read_input_report(
        &mut self,
        buf: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, TransportError> {
        if let Some(err) = &self.injected_error {
            return Err(err.clone());
        }

        if !self.suppress_recording {
            self.calls.push(TransportCall::ReadInput { timeout_ms });
        }

        match self.input_queue.pop_front() {
            Some(report) => {
                if buf.len() < report.len() {
                    // Put report back to preserve queue consistency
                    self.input_queue.push_front(report.clone());
                    Err(TransportError::BufferTooSmall {
                        needed: report.len(),
                        provided: buf.len(),
                    })
                } else {
                    buf[..report.len()].copy_from_slice(&report);
                    Ok(report.len())
                }
            }
            None => Err(TransportError::Timeout),
        }
    }
}

/// A thread-safe shared wrapper around [`MockTransport`] allowing inspection after moving into a device coordinator.
#[derive(Debug, Clone, Default)]
pub struct SharedMockTransport(pub Arc<Mutex<MockTransport>>);

impl SharedMockTransport {
    /// Creates a new empty `SharedMockTransport`.
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(MockTransport::new())))
    }

    /// Obtains a lock guard on the underlying `MockTransport`.
    pub fn lock(&self) -> std::sync::MutexGuard<'_, MockTransport> {
        self.0.lock().unwrap()
    }
}

impl Transport for SharedMockTransport {
    fn write_bulk(&mut self, report_id: u8, data: &[u8]) -> Result<usize, TransportError> {
        self.0.lock().unwrap().write_bulk(report_id, data)
    }

    fn send_feature_report(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.0.lock().unwrap().send_feature_report(data)
    }

    fn get_feature_report(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, TransportError> {
        self.0.lock().unwrap().get_feature_report(report_id, buf)
    }

    fn read_input_report(
        &mut self,
        buf: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, TransportError> {
        self.0.lock().unwrap().read_input_report(buf, timeout_ms)
    }
}
