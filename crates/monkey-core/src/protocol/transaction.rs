use std::time::{Duration, Instant};
use zerocopy::IntoBytes;
use crate::error::{MonkeyError, Result, TransportError};
use crate::protocol::codecs::{BulkPacket, FeaturePacket};
use crate::protocol::safety::{SafetyRails, WriteMode};
use crate::protocol::types::{CommandId, VendorReportId};
use crate::transport::Transport;

pub struct TransactionManager<'a> {
    transport: &'a mut dyn Transport,
    safety: &'a SafetyRails,
    inter_packet_delay: Duration,
    inter_chunk_delay: Duration,
}

impl<'a> TransactionManager<'a> {
    pub fn new(
        transport: &'a mut dyn Transport,
        safety: &'a SafetyRails,
    ) -> Self {
        Self {
            transport,
            safety,
            inter_packet_delay: Duration::from_millis(5),
            inter_chunk_delay: Duration::from_millis(15),
        }
    }

    pub fn with_delays(mut self, packet_delay: Duration, chunk_delay: Duration) -> Self {
        self.inter_packet_delay = packet_delay;
        self.inter_chunk_delay = chunk_delay;
        self
    }

    pub fn send_feature_command(
        &mut self,
        cmd: CommandId,
        packet: &FeaturePacket,
        mode: WriteMode,
    ) -> Result<()> {
        self.safety.validate_command(cmd)?;
        self.safety.check_write_allowed(mode)?;

        self.transport
            .send_feature_report(packet.as_bytes())
            .map_err(MonkeyError::Transport)?;

        if !self.inter_packet_delay.is_zero() {
            std::thread::sleep(self.inter_packet_delay);
        }

        Ok(())
    }

    pub fn stream_bulk_chunks<F>(
        &mut self,
        chunks: &[BulkPacket],
        mut on_chunk_sent: F,
    ) -> Result<Duration>
    where
        F: FnMut(usize, usize),
    {
        let start = Instant::now();
        let total = chunks.len();

        for (idx, chunk) in chunks.iter().enumerate() {
            let bytes = chunk.to_bytes();
            self.transport
                .write_bulk(VendorReportId::BulkOut as u8, &bytes)
                .map_err(|err| {
                    MonkeyError::Transport(TransportError::IoError(format!(
                        "Bulk chunk {}/{} failed: {}",
                        idx + 1,
                        total,
                        err
                    )))
                })?;

            on_chunk_sent(idx + 1, total);

            if idx + 1 < total && !self.inter_chunk_delay.is_zero() {
                std::thread::sleep(self.inter_chunk_delay);
            }
        }

        Ok(start.elapsed())
    }
}
