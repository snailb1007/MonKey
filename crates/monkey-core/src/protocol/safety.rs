use crate::error::{MonkeyError, Result};
use crate::protocol::types::CommandId;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const SAFE_WRITE_COMMANDS: &[CommandId] = &[
    CommandId::StartTransaction,
    CommandId::RgbControl,
    CommandId::RgbMatrix,
    CommandId::SaveSettings,
    CommandId::EndTransaction,
    CommandId::StateReadback,
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
    hardware_writes_allowed: AtomicBool,
    active_transaction: AtomicBool,
    verified_ram_preview: AtomicBool,
}

impl SafetyRails {
    /// Helper to create `SafetyRails` with hardware writes permitted (primarily for tests).
    pub fn allow_hardware_writes() -> Self {
        Self::new().with_hardware_writes_permitted(true)
    }
    pub fn new() -> Self {
        Self::with_flash_debounce(Duration::from_millis(500))
    }

    pub fn with_flash_debounce(duration: Duration) -> Self {
        Self {
            flash_debounce_duration: duration,
            last_flash_write_ms: AtomicU64::new(u64::MAX),
            start_time: Instant::now(),
            blocked_count: AtomicU64::new(0),
            hardware_writes_allowed: AtomicBool::new(false),
            active_transaction: AtomicBool::new(false),
            verified_ram_preview: AtomicBool::new(false),
        }
    }

    /// Sets whether hardware writes/transfers are permitted.
    pub fn set_hardware_writes_allowed(&self, allowed: bool) {
        self.hardware_writes_allowed
            .store(allowed, Ordering::SeqCst);
    }

    /// Builder pattern helper to set hardware writes allowed.
    #[must_use]
    pub fn with_hardware_writes_permitted(self, allowed: bool) -> Self {
        self.set_hardware_writes_allowed(allowed);
        self
    }

    /// Sets whether a transaction is currently active.
    pub fn set_active_transaction(&self, active: bool) {
        self.active_transaction.store(active, Ordering::SeqCst);
    }

    /// Builder pattern helper to set active transaction state.
    #[must_use]
    pub fn with_active_transaction(self, active: bool) -> Self {
        self.set_active_transaction(active);
        self
    }

    /// Sets whether RAM preview state has been verified.
    pub fn set_verified_ram_preview(&self, verified: bool) {
        self.verified_ram_preview.store(verified, Ordering::SeqCst);
    }

    /// Builder pattern helper to set verified RAM preview state.
    #[must_use]
    pub fn with_verified_ram_preview(self, verified: bool) -> Self {
        self.set_verified_ram_preview(verified);
        self
    }

    /// Checks whether hardware writes/transfers are permitted.
    pub fn is_hardware_write_allowed(&self) -> bool {
        self.hardware_writes_allowed.load(Ordering::SeqCst)
    }

    /// Validates hardware write permission.
    pub fn validate_hardware_write_permitted(&self) -> Result<()> {
        if self.is_hardware_write_allowed() {
            Ok(())
        } else {
            self.blocked_count.fetch_add(1, Ordering::Relaxed);
            Err(MonkeyError::SafetyViolation(
                "Hardware writes are disabled. Explicit authorization is required.".into(),
            ))
        }
    }

    /// Marks that a transaction is active.
    pub fn set_transaction_active(&self, active: bool) {
        self.active_transaction.store(active, Ordering::SeqCst);
        if !active {
            self.verified_ram_preview.store(false, Ordering::SeqCst);
        }
    }

    /// Checks if a transaction is currently active.
    pub fn is_transaction_active(&self) -> bool {
        self.active_transaction.load(Ordering::SeqCst)
    }

    /// Marks that a valid RAM preview has been executed.
    pub fn set_ram_preview_verified(&self, verified: bool) {
        self.verified_ram_preview.store(verified, Ordering::SeqCst);
    }

    /// Checks whether a valid RAM preview was verified.
    pub fn is_ram_preview_verified(&self) -> bool {
        self.verified_ram_preview.load(Ordering::SeqCst)
    }

    /// Default-deny policy from Phase 2 CONTEXT.md; inspect the actual packet opcode.
    pub fn validate_command(&self, opcode: u8) -> Result<()> {
        if SAFE_WRITE_COMMANDS.iter().any(|cmd| *cmd as u8 == opcode) {
            Ok(())
        } else {
            self.blocked_count.fetch_add(1, Ordering::Relaxed);
            Err(MonkeyError::SafetyViolation(format!(
                "Command opcode 0x{opcode:02X} is not in the safe write whitelist"
            )))
        }
    }

    /// Bind persistence policy to the opcode, not a caller's independent label.
    pub fn validate_write(&self, opcode: u8, mode: WriteMode) -> Result<()> {
        self.validate_command(opcode)?;
        if (opcode == CommandId::SaveSettings as u8) != (mode == WriteMode::FlashCommit) {
            self.blocked_count.fetch_add(1, Ordering::Relaxed);
            return Err(MonkeyError::SafetyViolation(format!(
                "Opcode 0x{opcode:02X} is incompatible with write mode {mode:?}"
            )));
        }
        if mode == WriteMode::FlashCommit {
            if !self.is_transaction_active() {
                self.blocked_count.fetch_add(1, Ordering::Relaxed);
                return Err(MonkeyError::SafetyViolation(
                    "Flash commit rejected: no active transaction session".into(),
                ));
            }
            if !self.is_ram_preview_verified() {
                self.blocked_count.fetch_add(1, Ordering::Relaxed);
                return Err(MonkeyError::SafetyViolation(
                    "Flash commit rejected: RAM preview must be executed and verified before flash commit".into(),
                ));
            }
        }
        self.check_write_allowed(mode)
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
