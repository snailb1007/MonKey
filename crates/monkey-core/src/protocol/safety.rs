use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use crate::error::{MonkeyError, Result};
use crate::protocol::types::CommandId;

pub const SAFE_WRITE_COMMANDS: &[CommandId] = &[
    CommandId::GetVersion,
    CommandId::RgbControl,
    CommandId::LcdStartTransfer,
    CommandId::LcdChunkAck,
    CommandId::LcdEndTransfer,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    RamPreview,
    FlashCommit,
}

#[derive(Debug)]
pub struct SafetyRails {
    flash_debounce_duration: Duration,
    last_flash_write_ms: AtomicU64,
    start_time: Instant,
    blocked_count: AtomicU64,
}

impl SafetyRails {
    pub fn new() -> Self {
        Self::with_flash_debounce(Duration::from_millis(500))
    }

    pub fn with_flash_debounce(duration: Duration) -> Self {
        Self {
            flash_debounce_duration: duration,
            last_flash_write_ms: AtomicU64::new(u64::MAX),
            start_time: Instant::now(),
            blocked_count: AtomicU64::new(0),
        }
    }

    pub fn validate_command(&self, cmd: CommandId) -> Result<()> {
        if SAFE_WRITE_COMMANDS.contains(&cmd) {
            Ok(())
        } else {
            self.blocked_count.fetch_add(1, Ordering::Relaxed);
            Err(MonkeyError::SafetyViolation(format!(
                "Command opcode {:?} (0x{:02X}) is not in the safe write whitelist",
                cmd, cmd as u8
            )))
        }
    }

    pub fn check_write_allowed(&self, mode: WriteMode) -> Result<()> {
        match mode {
            WriteMode::RamPreview => Ok(()),
            WriteMode::FlashCommit => {
                let now_ms = self.start_time.elapsed().as_millis() as u64;
                let last_ms = self.last_flash_write_ms.load(Ordering::Relaxed);
                let debounce_ms = self.flash_debounce_duration.as_millis() as u64;

                if last_ms != u64::MAX && now_ms.saturating_sub(last_ms) < debounce_ms {
                    self.blocked_count.fetch_add(1, Ordering::Relaxed);
                    return Err(MonkeyError::SafetyViolation(format!(
                        "Flash write throttled: {}ms elapsed since last write, required debounce is {}ms",
                        now_ms.saturating_sub(last_ms),
                        debounce_ms
                    )));
                }

                self.last_flash_write_ms.store(now_ms, Ordering::Relaxed);
                Ok(())
            }
        }
    }

    pub fn blocked_count(&self) -> u64 {
        self.blocked_count.load(Ordering::Relaxed)
    }
}

impl Default for SafetyRails {
    fn default() -> Self {
        Self::new()
    }
}
