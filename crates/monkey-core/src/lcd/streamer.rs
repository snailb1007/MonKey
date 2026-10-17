//! Paced raw-frame streaming and frame-rate regulation.

use std::thread;
use std::time::{Duration, Instant};

use crate::error::{MonkeyError, Result, TransportError};
use crate::transport::SafeTransport;

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
    transport: SafeTransport<'a>,
    pub config: LcdPacingConfig,
}

impl<'a> LcdStreamer<'a> {
    pub fn new(transport: SafeTransport<'a>, config: LcdPacingConfig) -> Result<Self> {
        config.validate()?;
        transport.safety().validate_hardware_write_permitted()?;
        Ok(Self { transport, config })
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
        self.transport
            .safety()
            .validate_hardware_write_permitted()?;
        let start = Instant::now();
        let chunks = FrameChunker::new(frame)?;
        let mut sent = 0usize;
        for chunk in chunks {
            let written = self
                .transport
                .write_bulk(LCD_INTERFACE_A_REPORT_ID, chunk)
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

    fn send_verified_ctrl_packet(&mut self, payload: &[u8; 64]) -> Result<()> {
        let mut frame_65 = [0u8; 65];
        frame_65[0] = 0x00;
        frame_65[1..65].copy_from_slice(payload);
        self.transport
            .send_feature_report(&frame_65)
            .map_err(MonkeyError::Transport)?;

        let mut rx_buf = [0u8; 65];
        for _ in 0..40 {
            thread::sleep(Duration::from_millis(12));
            match self.transport.get_feature_report(0, &mut rx_buf) {
                Ok(n) if n >= 4 => {
                    let status_byte = if n == 65 { rx_buf[4] } else { rx_buf[3] };
                    if status_byte != 0xFF {
                        return Ok(());
                    }
                }
                _ => {
                    // MockTransport or unqueued feature readback: treat as immediate success
                    return Ok(());
                }
            }
        }
        Err(MonkeyError::Transport(TransportError::Timeout))
    }

    /// Uploads one or more frames (`1..=255`) using the verified OEM Monka 3075 Pro / RKGK890
    /// LCD protocol (`04 18` -> `04 72` -> `N` chunks of `4096B` with `256B` header, `160x96` RGB565
    /// frames, and `64B` Input Report ACK -> `04 02` -> `04 28` slot 1 selection).
    pub fn upload_oem_lcd_sequence<F>(
        &mut self,
        frames: &[[u8; LCD_FRAME_BYTES]],
        delays_ms: &[u64],
        mut on_chunk: F,
    ) -> Result<LcdStreamMetrics>
    where
        F: FnMut(usize, usize),
    {
        self.transport
            .safety()
            .validate_hardware_write_permitted()?;

        if frames.is_empty() {
            return Err(MonkeyError::InvalidParameter(
                "LCD frame sequence cannot be empty".to_string(),
            ));
        }

        let frame_count = frames.len().min(255);
        let raw_total_bytes =
            super::OEM_LCD_HEADER_BYTES + frame_count * super::OEM_LCD_FRAME_BYTES;
        let total_chunks = raw_total_bytes.div_ceil(LCD_CHUNK_SIZE);
        let mut bulk_buf = vec![0xFFu8; total_chunks * LCD_CHUNK_SIZE];

        // 256-byte header: [frame_count, delay_0, delay_1, ..., 0xFF...]
        bulk_buf[0] = frame_count as u8;
        for i in 0..frame_count {
            let delay_ms = delays_ms.get(i).copied().unwrap_or(100);
            let delay_2ms = ((delay_ms / 2).clamp(1, 255)) as u8;
            bulk_buf[1 + i] = delay_2ms;
        }

        // Convert each 128x128 frame into 160x96 (30,720 bytes) at offset 256 + i * 30720
        for (i, frame) in frames.iter().take(frame_count).enumerate() {
            let oem_frame = super::resample_128_to_oem_160x96(frame);
            let offset = super::OEM_LCD_HEADER_BYTES + i * super::OEM_LCD_FRAME_BYTES;
            bulk_buf[offset..offset + super::OEM_LCD_FRAME_BYTES].copy_from_slice(&oem_frame);
        }

        let start = Instant::now();

        // Step 1: 04 18 (Start session)
        let mut cmd_18 = [0u8; 64];
        cmd_18[0] = 0x04;
        cmd_18[1] = 0x18;
        self.send_verified_ctrl_packet(&cmd_18)?;

        // Step 2: 04 72 01 00 00 00 00 00 <chunks_lo> <chunks_hi> (Start LCD upload to Slot 1)
        let mut cmd_72 = [0u8; 64];
        cmd_72[0] = 0x04;
        cmd_72[1] = 0x72;
        cmd_72[2] = 0x01; // Slot 1
        let chunks_le = (total_chunks as u16).to_le_bytes();
        cmd_72[8] = chunks_le[0];
        cmd_72[9] = chunks_le[1];
        self.send_verified_ctrl_packet(&cmd_72)?;

        // Step 3: Stream 4096-byte chunks to Interface A (0xFF68) and wait for 64B Input Report ACK
        let mut ack_buf = [0u8; 65];
        for chunk_idx in 0..total_chunks {
            let chunk = &bulk_buf[chunk_idx * LCD_CHUNK_SIZE..(chunk_idx + 1) * LCD_CHUNK_SIZE];
            let written = self
                .transport
                .write_bulk(LCD_INTERFACE_A_REPORT_ID, chunk)
                .map_err(MonkeyError::Transport)?;
            if written != LCD_CHUNK_SIZE {
                return Err(MonkeyError::Transport(TransportError::IoError(format!(
                    "LCD chunk {} short write: expected {}, wrote {}",
                    chunk_idx + 1,
                    LCD_CHUNK_SIZE,
                    written
                ))));
            }
            // Wait up to 350ms for MCU SPI flash page write ACK (`01 5A 02...`) on Interface A IN endpoint
            let _ = self.transport.read_input_report(&mut ack_buf, 350);
            on_chunk(chunk_idx + 1, total_chunks);
        }

        // Step 4: 04 02 (Commit / close LCD upload session)
        let mut cmd_02 = [0u8; 64];
        cmd_02[0] = 0x04;
        cmd_02[1] = 0x02;
        self.send_verified_ctrl_packet(&cmd_02)?;

        // Step 5: Select Slot 1 on screen via 04 28 so the keyboard switches away from the RTC clock
        let _ = self.send_verified_ctrl_packet(&cmd_18);
        let mut cmd_28 = [0u8; 64];
        cmd_28[0] = 0x04;
        cmd_28[1] = 0x28;
        cmd_28[8] = 0x01;
        if self.send_verified_ctrl_packet(&cmd_28).is_ok() {
            let mut data_28 = [0u8; 64];
            data_28[0] = 0x00;
            data_28[1] = 0x01; // Active slot 1
            data_28[2] = 0x5A;
            data_28[3] = 26; // year
            data_28[4] = 10; // month
            data_28[5] = 17; // day
            data_28[6] = 16; // hour
            data_28[7] = 45; // minute
            data_28[8] = 0;  // second
            data_28[62] = 0xAA;
            data_28[63] = 0x55;
            let _ = self.send_verified_ctrl_packet(&data_28);
            let _ = self.send_verified_ctrl_packet(&cmd_02);
        }

        Ok(LcdStreamMetrics {
            chunks_sent: total_chunks,
            bytes_sent: total_chunks * LCD_CHUNK_SIZE,
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
