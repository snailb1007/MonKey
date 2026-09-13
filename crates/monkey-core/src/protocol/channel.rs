use crate::error::{MonkeyError, Result, TransportError};
use crate::protocol::codecs::{BulkPacket, FeaturePacket};
use crate::protocol::safety::{SafetyRails, WriteMode};
use crate::protocol::transaction::TransactionManager;
use crate::protocol::types::CommandId;
use crate::transport::Transport;
use crossbeam_channel::{bounded, Sender};
use std::sync::Arc;
use std::time::Duration;

pub enum HardwareCommand {
    SendFeature {
        cmd: CommandId,
        packet: Box<FeaturePacket>,
        mode: WriteMode,
        responder: Sender<Result<()>>,
    },
    StreamBulk {
        chunks: Vec<BulkPacket>,
        responder: Sender<Result<Duration>>,
    },
    Close,
}

#[derive(Clone)]
pub struct HardwareChannel {
    tx: Sender<HardwareCommand>,
}

impl HardwareChannel {
    pub fn spawn<T: Transport + 'static>(mut transport: T, safety: Arc<SafetyRails>) -> Self {
        let (tx, rx) = bounded::<HardwareCommand>(32);

        std::thread::Builder::new()
            .name("monkey-hardware-worker".to_string())
            .spawn(move || {
                let mut manager = TransactionManager::new(&mut transport, &safety);

                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        HardwareCommand::SendFeature {
                            cmd,
                            packet,
                            mode,
                            responder,
                        } => {
                            let res = manager.send_feature_command(cmd, &packet, mode);
                            let _ = responder.send(res);
                        }
                        HardwareCommand::StreamBulk { chunks, responder } => {
                            let res = manager.stream_bulk_chunks(&chunks, |_, _| {});
                            let _ = responder.send(res);
                        }
                        HardwareCommand::Close => {
                            break;
                        }
                    }
                }
            })
            .expect("failed to spawn hardware worker thread");

        Self { tx }
    }

    pub fn send_feature(
        &self,
        cmd: CommandId,
        packet: FeaturePacket,
        mode: WriteMode,
    ) -> Result<()> {
        let (resp_tx, resp_rx) = bounded(1);
        self.tx
            .send(HardwareCommand::SendFeature {
                cmd,
                packet: Box::new(packet),
                mode,
                responder: resp_tx,
            })
            .map_err(|e| MonkeyError::Transport(TransportError::IoError(e.to_string())))?;

        resp_rx
            .recv()
            .map_err(|e| MonkeyError::Transport(TransportError::IoError(e.to_string())))?
    }

    pub fn stream_bulk(&self, chunks: Vec<BulkPacket>) -> Result<Duration> {
        let (resp_tx, resp_rx) = bounded(1);
        self.tx
            .send(HardwareCommand::StreamBulk {
                chunks,
                responder: resp_tx,
            })
            .map_err(|e| MonkeyError::Transport(TransportError::IoError(e.to_string())))?;

        resp_rx
            .recv()
            .map_err(|e| MonkeyError::Transport(TransportError::IoError(e.to_string())))?
    }

    pub fn close(&self) {
        let _ = self.tx.send(HardwareCommand::Close);
    }
}
