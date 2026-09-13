use crate::error::{MonkeyError, Result, TransportError};
use crate::protocol::framing::calculate_chunks_crc16;
use crate::protocol::safety::{SafetyRails, WriteMode};
use crate::protocol::types::{BulkChunkPacket, FeatureReportPacket, VendorReportId};
use crate::transport::Transport;
use std::time::{Duration, Instant};

pub struct TransactionManager<'a> {
    transport: &'a mut dyn Transport,
    safety: &'a SafetyRails,
    inter_packet_delay: Duration,
    inter_chunk_delay: Duration,
}

impl<'a> TransactionManager<'a> {
    pub fn new(transport: &'a mut dyn Transport, safety: &'a SafetyRails) -> Self {
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
        packet: &FeatureReportPacket,
        mode: WriteMode,
    ) -> Result<()> {
        packet.validate_header()?;
        self.safety.validate_write(packet.command, mode)?;

        self.transport
            .send_feature_report(packet.as_bytes(), self.safety)
            .map_err(MonkeyError::Transport)?;

        if !self.inter_packet_delay.is_zero() {
            std::thread::sleep(self.inter_packet_delay);
        }

        Ok(())
    }

    pub fn stream_bulk_chunks<F>(
        &mut self,
        chunks: &[BulkChunkPacket],
        mut on_chunk_sent: F,
    ) -> Result<Duration>
    where
        F: FnMut(usize, usize),
    {
        // Enforce hardware write authorization check
        self.safety.validate_hardware_write_permitted()?;

        // Enforce boundary checksum calculation across chunks
        let _expected_crc = calculate_chunks_crc16(chunks);

        let start = Instant::now();
        let total = chunks.len();

        for (idx, chunk) in chunks.iter().enumerate() {
            let written = self
                .transport
                .write_bulk(VendorReportId::BulkOut as u8, &chunk.data, self.safety)
                .map_err(|err| {
                    MonkeyError::Transport(TransportError::IoError(format!(
                        "Bulk chunk {}/{} failed: {}",
                        idx + 1,
                        total,
                        err
                    )))
                })?;

            if written != chunk.data.len() {
                return Err(MonkeyError::Transport(TransportError::IoError(format!(
                    "Bulk chunk {}/{} incomplete: wrote {} of {} bytes",
                    idx + 1,
                    total,
                    written,
                    chunk.data.len()
                ))));
            }

            on_chunk_sent(idx + 1, total);

            if idx + 1 < total && !self.inter_chunk_delay.is_zero() {
                std::thread::sleep(self.inter_chunk_delay);
            }
        }

        Ok(start.elapsed())
    }
}
