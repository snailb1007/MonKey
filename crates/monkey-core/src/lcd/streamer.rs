//! Paced raw-frame streaming and frame-rate regulation.

use std::thread;
use std::time::{Duration, Instant};

use crate::error::{MonkeyError, Result, TransportError};
use crate::protocol::SafetyRails;
use crate::transport::Transport;

use super::chunker::FrameChunker;
use super::{LCD_CHUNK_COUNT, LCD_CHUNK_SIZE, LCD_FRAME_BYTES, LCD_INTERFACE_A_REPORT_ID};

/// Safe hardware frame rate boundary (10–15 FPS).
pub const MIN_SAFE_FPS: u32 = 10;
pub const MAX_SAFE_FPS: u32 = 15;

/// Hardware-safe minimum frame delay (at 15 FPS: 66,666 µs).
/// Ensures untrusted animation inputs cannot burst the MCU FIFO.
pub const MIN_SAFE_FRAME_DELAY: Duration = Duration::from_micros(1_000_000 / MAX_SAFE_FPS as u64);

/// Clamps an untrusted animation frame delay to ensure frame rate does not exceed MAX_SAFE_FPS (15 FPS).
pub fn clamp_safe_frame_delay(delay: Duration) -> Duration {
    delay.max(MIN_SAFE_FRAME_DELAY)
}

/// Rejects a frame rate outside the safe hardware band.
pub fn validate_safe_fps(target_fps: u32) -> Result<()> {
    if !(MIN_SAFE_FPS..=MAX_SAFE_FPS).contains(&target_fps) {
        return Err(MonkeyError::InvalidParameter(format!(
            "LCD target FPS must be between {MIN_SAFE_FPS} and {MAX_SAFE_FPS}, got {target_fps}"
        )));
    }
    Ok(())
}

/// Host pacing knobs for the raw LCD pipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LcdPacingConfig {
    /// Delay between consecutive 4096-byte writes. Default is 3 ms.
    pub inter_chunk_delay: Duration,
    /// Target animation rate. The supported safe band is 10–15 FPS.
    pub target_fps: u32,
}

impl Default for LcdPacingConfig {
    fn default() -> Self {
        Self {
            inter_chunk_delay: Duration::from_millis(3),
            target_fps: 12,
        }
    }
}

impl LcdPacingConfig {
    pub fn validate(self) -> Result<()> {
        validate_safe_fps(self.target_fps)?;
        if self.inter_chunk_delay > Duration::from_millis(8) {
            return Err(MonkeyError::InvalidParameter(
                "LCD inter-chunk delay must not exceed 8ms".to_string(),
            ));
        }
        Ok(())
    }

    pub const fn frame_period(self) -> Duration {
        // An unvalidated config must never pace *faster* than hardware allows,
        // so an out-of-range rate falls back to the slowest safe period.
        if self.target_fps == 0 {
            return MIN_SAFE_FRAME_DELAY;
        }
        Duration::from_micros(1_000_000 / self.target_fps as u64)
    }
}

/// Result of one complete frame transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LcdStreamMetrics {
    pub chunks_sent: usize,
    pub bytes_sent: usize,
    pub elapsed: Duration,
}

impl LcdStreamMetrics {
    pub fn meets_static_target(self) -> bool {
        self.elapsed <= Duration::from_millis(50)
    }
}

/// Sends raw, unnumbered 4096-byte chunks to Interface A.
pub struct LcdStreamer<'a> {
    transport: &'a mut dyn Transport,
    safety: &'a SafetyRails,
    pub config: LcdPacingConfig,
}

impl<'a> LcdStreamer<'a> {
    pub fn new(
        transport: &'a mut dyn Transport,
        safety: &'a SafetyRails,
        config: LcdPacingConfig,
    ) -> Result<Self> {
        config.validate()?;
        safety.validate_hardware_write_permitted()?;
        Ok(Self {
            transport,
            safety,
            config,
        })
    }

    /// Sends exactly eight borrowed chunks and paces all but the final write.
    pub fn send_frame(&mut self, frame: &[u8; LCD_FRAME_BYTES]) -> Result<LcdStreamMetrics> {
        self.send_frame_with_progress(frame, |_, _| {})
    }

    pub fn send_frame_with_progress<F>(
        &mut self,
        frame: &[u8; LCD_FRAME_BYTES],
        mut on_chunk: F,
    ) -> Result<LcdStreamMetrics>
    where
        F: FnMut(usize, usize),
    {
        self.safety.validate_hardware_write_permitted()?;
        let start = Instant::now();
        let chunks = FrameChunker::new(frame)?;
        let mut sent = 0usize;
        for chunk in chunks {
            let written = self
                .transport
                .write_bulk(LCD_INTERFACE_A_REPORT_ID, chunk, self.safety)
                .map_err(MonkeyError::Transport)?;
            if written != LCD_CHUNK_SIZE {
                return Err(MonkeyError::Transport(TransportError::IoError(format!(
                    "LCD chunk {} short write: expected {}, wrote {}",
                    sent + 1,
                    LCD_CHUNK_SIZE,
                    written
                ))));
            }
            sent += 1;
            on_chunk(sent, LCD_CHUNK_COUNT);
            if sent < LCD_CHUNK_COUNT && !self.config.inter_chunk_delay.is_zero() {
                thread::sleep(self.config.inter_chunk_delay);
            }
        }
        Ok(LcdStreamMetrics {
            chunks_sent: sent,
            bytes_sent: sent * LCD_CHUNK_SIZE,
            elapsed: start.elapsed(),
        })
    }
}

/// A monotonic, drift-resistant frame deadline regulator.
#[derive(Debug, Clone)]
pub struct FpsRegulator {
    period: Duration,
    next_deadline: Option<Instant>,
}

impl FpsRegulator {
    pub fn new(target_fps: u32) -> Result<Self> {
        validate_safe_fps(target_fps)?;
        Ok(Self {
            period: Duration::from_micros(1_000_000 / target_fps as u64),
            next_deadline: None,
        })
    }

    pub const fn frame_period(&self) -> Duration {
        self.period
    }

    /// Waits for the next slot. If work missed a deadline, starts immediately
    /// and resets the schedule rather than accumulating playback latency.
    pub fn wait_for_next_frame(&mut self) -> Duration {
        let now = Instant::now();
        let deadline = self.next_deadline.unwrap_or(now);
        let slept = deadline.saturating_duration_since(now);
        if !slept.is_zero() {
            thread::sleep(slept);
        }
        let after = Instant::now();
        self.next_deadline = Some(match self.next_deadline {
            Some(previous) if previous + self.period > after => previous + self.period,
            _ => after + self.period,
        });
        slept
    }

    pub fn reset(&mut self) {
        self.next_deadline = None;
    }

    pub fn is_late(&self) -> bool {
        self.next_deadline
            .is_some_and(|deadline| Instant::now() > deadline)
    }
}

/// More descriptive alias for callers that prefer the full name.
pub type FrameRateRegulator = FpsRegulator;
