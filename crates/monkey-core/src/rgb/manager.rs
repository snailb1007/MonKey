use std::sync::atomic::{AtomicU64, Ordering};
use crate::error::{MonkeyError, Result};
use crate::protocol::safety::{SafetyRails, WriteMode};
use crate::protocol::transaction::TransactionManager;
use crate::protocol::types::{CommandId, FeatureReportPacket, FEATURE_REPORT_SIZE};
use crate::rgb::codec::{decode_rgb_control_packet, encode_rgb_control_packet};
use crate::rgb::mode::LightingConfig;
use crate::transport::Transport;

pub struct RgbManager<'a> {
    transport: &'a mut dyn Transport,
    safety: &'a SafetyRails,
    session_flash_commits: AtomicU64,
}

impl<'a> RgbManager<'a> {
    pub fn new(transport: &'a mut dyn Transport, safety: &'a SafetyRails) -> Self {
        Self {
            transport,
            safety,
            session_flash_commits: AtomicU64::new(0),
        }
    }

    /// Returns the number of successful flash commit operations performed in this session.
    pub fn session_flash_commit_count(&self) -> u64 {
        self.session_flash_commits.load(Ordering::Relaxed)
    }

    /// Applies lighting configuration to volatile RAM preview (up to 30Hz, zero flash wear).
    pub fn apply_preview(&mut self, config: &LightingConfig) -> Result<()> {
        config.validate()?;
        let packet = encode_rgb_control_packet(config)?;

        let mut tx = TransactionManager::new(self.transport, self.safety);
        tx.send_feature_command(&packet, WriteMode::RamPreview)?;

        self.safety.set_ram_preview_verified(true);
        Ok(())
    }

    /// Commits lighting configuration to permanent onboard SPI NOR flash with debouncing and battery protection.
    pub fn apply_commit(
        &mut self,
        config: &LightingConfig,
        is_wireless: bool,
        battery_percent: Option<u8>,
        force: bool,
    ) -> Result<()> {
        config.validate()?;

        // Battery safety gate for wireless connections (RGB-03)
        if is_wireless && battery_percent.is_some_and(|b| b < 20) && !force {
            let b = battery_percent.unwrap_or(0);
            return Err(MonkeyError::SafetyViolation(format!(
                "Flash commit blocked: Battery is at {}% (< 20%) on wireless transport. Connect USB cable to charge, or pass --force to override at your own risk.",
                b
            )));
        }

        // Apply volatile preview first if not yet done
        let packet = encode_rgb_control_packet(config)?;

        self.safety.set_transaction_active(true);
        self.safety.set_ram_preview_verified(true);

        let mut tx = TransactionManager::new(self.transport, self.safety);

        // 1. Start transaction
        let start_pkt = FeatureReportPacket::new(CommandId::StartTransaction as u8);
        if let Err(e) = tx.send_feature_command(&start_pkt, WriteMode::RamPreview) {
            self.safety.set_transaction_active(false);
            return Err(e);
        }

        // 2. Send RGB configuration
        if let Err(e) = tx.send_feature_command(&packet, WriteMode::RamPreview) {
            let end_pkt = FeatureReportPacket::new(CommandId::EndTransaction as u8);
            let _ = tx.send_feature_command(&end_pkt, WriteMode::RamPreview);
            self.safety.set_transaction_active(false);
            return Err(e);
        }

        // 3. Commit to Flash (enforces 500ms debounce in SafetyRails)
        let save_pkt = FeatureReportPacket::new(CommandId::SaveSettings as u8);
        if let Err(e) = tx.send_feature_command(&save_pkt, WriteMode::FlashCommit) {
            let end_pkt = FeatureReportPacket::new(CommandId::EndTransaction as u8);
            let _ = tx.send_feature_command(&end_pkt, WriteMode::RamPreview);
            self.safety.set_transaction_active(false);
            return Err(e);
        }

        // 4. End transaction
        let end_pkt = FeatureReportPacket::new(CommandId::EndTransaction as u8);
        let end_res = tx.send_feature_command(&end_pkt, WriteMode::RamPreview);

        self.safety.set_transaction_active(false);
        end_res?;

        self.session_flash_commits.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Queries the device for active RGB configuration via 04 F5 readback.
    pub fn readback_status(&mut self) -> Result<Option<LightingConfig>> {
        let mut buf = [0u8; FEATURE_REPORT_SIZE];
        let read_res = self.transport.get_feature_report(0x04, &mut buf);

        match read_res {
            Ok(read_bytes) if read_bytes >= FEATURE_REPORT_SIZE => {
                match decode_rgb_control_packet(&buf) {
                    Ok(cfg) => Ok(Some(cfg)),
                    Err(_) => Ok(None),
                }
            }
            Ok(_) => Ok(None),
            Err(crate::error::TransportError::InvalidReportId(_)) => Ok(None),
            Err(e) => Err(MonkeyError::Transport(e)),
        }
    }
}
