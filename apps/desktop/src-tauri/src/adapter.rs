use std::sync::Arc;

use crate::asset::{
    convert_image_bytes_to_asset, process_image_sequence, AssetStore, ConvertedAsset,
};
use crate::dto::*;
use crate::error::{DesktopError, DesktopResult};
use crate::profile::{export_profile_json, import_profile_json};
use crate::worker::{HardwareBackend, HardwareWorkerHandle};

pub struct DesktopAdapter {
    worker: HardwareWorkerHandle,
    asset_store: Arc<AssetStore>,
}

impl DesktopAdapter {
    pub fn new_with_backend<B: HardwareBackend>(backend: B) -> Self {
        let worker = HardwareWorkerHandle::spawn(backend);
        Self {
            worker,
            asset_store: Arc::new(AssetStore::new()),
        }
    }

    pub fn shutdown(&self) {
        self.worker.shutdown();
    }

    pub fn handle_request(&self, request: DesktopRequestEnvelope) -> DesktopResponseEnvelope {
        if let Err(e) = request.validate_envelope() {
            return DesktopResponseEnvelope::error(&request.request_id, e);
        }

        let res = match request.action {
            DesktopAction::ListDevices => self.handle_list_devices(&request.payload),
            DesktopAction::Probe => {
                self.handle_probe(request.device_id.as_deref(), &request.payload)
            }
            DesktopAction::Diagnostics => self.handle_diagnostics(&request.payload),
            DesktopAction::PickAsset => self.handle_pick_asset(&request.payload),
            DesktopAction::StartLcd => {
                self.handle_start_lcd(request.device_id.as_deref(), &request.payload)
            }
            DesktopAction::StopLcd => self.handle_stop_lcd(&request.payload),
            DesktopAction::JobStatus => self.handle_job_status(&request.payload),
            DesktopAction::RgbStatus => {
                self.handle_rgb_status(request.device_id.as_deref(), &request.payload)
            }
            DesktopAction::RgbPreview => {
                self.handle_rgb_preview(request.device_id.as_deref(), &request.payload)
            }
            DesktopAction::RgbCommit => {
                self.handle_rgb_commit(request.device_id.as_deref(), &request.payload)
            }
            DesktopAction::ExportProfile => self.handle_export_profile(&request.payload),
            DesktopAction::ImportProfile => self.handle_import_profile(&request.payload),
        };

        match res {
            Ok(value) => DesktopResponseEnvelope::ok(&request.request_id, value),
            Err(err) => DesktopResponseEnvelope::error(&request.request_id, err),
        }
    }

    fn handle_list_devices(
        &self,
        _payload: &serde_json::Value,
    ) -> DesktopResult<serde_json::Value> {
        let list = self.worker.list_devices()?;
        serde_json::to_value(ListDevicesResult { devices: list })
            .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }

    fn handle_probe(
        &self,
        device_id: Option<&str>,
        _payload: &serde_json::Value,
    ) -> DesktopResult<serde_json::Value> {
        let dev_id = device_id.ok_or_else(|| {
            DesktopError::InvalidInput("deviceId is required for probe action".to_string())
        })?;
        let snapshot = self.worker.probe(dev_id)?;
        serde_json::to_value(ProbeResult { device: snapshot })
            .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }

    fn handle_diagnostics(&self, _payload: &serde_json::Value) -> DesktopResult<serde_json::Value> {
        let checks = self.worker.diagnostics()?;
        serde_json::to_value(DiagnosticsResult { checks })
            .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }

    fn handle_pick_asset(&self, payload: &serde_json::Value) -> DesktopResult<serde_json::Value> {
        let req: PickAssetPayload = serde_json::from_value(payload.clone())
            .map_err(|e| DesktopError::InvalidInput(format!("Invalid pick_asset payload: {e}")))?;

        let asset: ConvertedAsset = match req.kind {
            AssetKind::Image => {
                let file = rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp"])
                    .pick_file();
                match file {
                    Some(path) => {
                        let bytes = std::fs::read(&path).map_err(|e| {
                            DesktopError::IoError(format!("Failed to read asset file: {e}"))
                        })?;
                        let name = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("image.png");
                        convert_image_bytes_to_asset(name, &bytes)?
                    }
                    None => {
                        return Err(DesktopError::Cancelled("File picker cancelled".to_string()))
                    }
                }
            }
            AssetKind::Animation => {
                let file = rfd::FileDialog::new()
                    .add_filter("GIF Animation", &["gif"])
                    .pick_file();
                match file {
                    Some(path) => {
                        let bytes = std::fs::read(&path).map_err(|e| {
                            DesktopError::IoError(format!("Failed to read GIF file: {e}"))
                        })?;
                        let name = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("animation.gif");
                        convert_image_bytes_to_asset(name, &bytes)?
                    }
                    None => {
                        return Err(DesktopError::Cancelled("File picker cancelled".to_string()))
                    }
                }
            }
            AssetKind::Sequence => {
                let folder = rfd::FileDialog::new().pick_folder();
                match folder {
                    Some(dir) => process_image_sequence(&dir)?,
                    None => {
                        return Err(DesktopError::Cancelled(
                            "Directory picker cancelled".to_string(),
                        ))
                    }
                }
            }
        };

        let id = self.asset_store.insert(asset.clone());
        let mut preview = asset.preview;
        preview.id = id;

        serde_json::to_value(PickAssetResult {
            asset: Some(preview),
        })
        .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }

    fn handle_start_lcd(
        &self,
        device_id: Option<&str>,
        payload: &serde_json::Value,
    ) -> DesktopResult<serde_json::Value> {
        let dev_id = device_id.ok_or_else(|| {
            DesktopError::InvalidInput("deviceId is required for start_lcd action".to_string())
        })?;

        let req: StartLcdPayload = serde_json::from_value(payload.clone())
            .map_err(|e| DesktopError::InvalidInput(format!("Invalid start_lcd payload: {e}")))?;
        req.validate()?;

        let (frames, pacing_ms) = match (&req.asset_id, &req.pattern) {
            (Some(asset_id), None) => {
                let asset = self.asset_store.get(asset_id).ok_or_else(|| {
                    DesktopError::InvalidInput(format!("Asset '{asset_id}' not found in store"))
                })?;
                let pacing = if asset.delays_ms.is_empty() {
                    100
                } else {
                    asset.delays_ms[0]
                };
                (asset.frames, pacing)
            }
            (None, Some(pat)) => {
                let frame = monkey_core::lcd::generate_test_pattern(pat.to_core_pattern());
                (vec![frame], 0)
            }
            _ => {
                return Err(DesktopError::InvalidInput(
                    "Exactly one of assetId or pattern must be provided".to_string(),
                ));
            }
        };

        let job_id = self.worker.start_lcd(dev_id, frames, pacing_ms)?;
        serde_json::to_value(StartLcdResult { job_id })
            .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }

    fn handle_stop_lcd(&self, payload: &serde_json::Value) -> DesktopResult<serde_json::Value> {
        let req: StopLcdPayload = serde_json::from_value(payload.clone())
            .map_err(|e| DesktopError::InvalidInput(format!("Invalid stop_lcd payload: {e}")))?;
        let stopped = self.worker.stop_lcd(&req.job_id)?;
        serde_json::to_value(StopLcdResult { stopped })
            .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }

    fn handle_job_status(&self, payload: &serde_json::Value) -> DesktopResult<serde_json::Value> {
        let req: JobStatusPayload = serde_json::from_value(payload.clone())
            .map_err(|e| DesktopError::InvalidInput(format!("Invalid job_status payload: {e}")))?;
        let job = self.worker.job_status(&req.job_id)?;
        serde_json::to_value(JobStatusResult { job })
            .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }

    fn handle_rgb_status(
        &self,
        device_id: Option<&str>,
        _payload: &serde_json::Value,
    ) -> DesktopResult<serde_json::Value> {
        let dev_id = device_id.ok_or_else(|| {
            DesktopError::InvalidInput("deviceId is required for rgb_status action".to_string())
        })?;
        let res = self.worker.rgb_status(dev_id)?;
        serde_json::to_value(res)
            .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }

    fn handle_rgb_preview(
        &self,
        device_id: Option<&str>,
        payload: &serde_json::Value,
    ) -> DesktopResult<serde_json::Value> {
        let dev_id = device_id.ok_or_else(|| {
            DesktopError::InvalidInput("deviceId is required for rgb_preview action".to_string())
        })?;
        let req: RgbPreviewPayload = serde_json::from_value(payload.clone())
            .map_err(|e| DesktopError::InvalidInput(format!("Invalid rgb_preview payload: {e}")))?;

        let res = self.worker.rgb_preview(dev_id, req.config)?;
        serde_json::to_value(res)
            .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }

    fn handle_rgb_commit(
        &self,
        device_id: Option<&str>,
        _payload: &serde_json::Value,
    ) -> DesktopResult<serde_json::Value> {
        let dev_id = device_id.ok_or_else(|| {
            DesktopError::InvalidInput("deviceId is required for rgb_commit action".to_string())
        })?;
        let res = self.worker.rgb_commit(dev_id)?;
        serde_json::to_value(res)
            .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }

    fn handle_export_profile(
        &self,
        payload: &serde_json::Value,
    ) -> DesktopResult<serde_json::Value> {
        let req: ExportProfilePayload = serde_json::from_value(payload.clone()).map_err(|e| {
            DesktopError::InvalidInput(format!("Invalid export_profile payload: {e}"))
        })?;
        let json_str = export_profile_json(
            "Monka 3075 Pro / RKGK890",
            &req.config,
            req.description.as_deref(),
        )?;
        serde_json::to_value(ExportProfileResult {
            profile_json: json_str,
        })
        .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }

    fn handle_import_profile(
        &self,
        payload: &serde_json::Value,
    ) -> DesktopResult<serde_json::Value> {
        let req: ImportProfilePayload = serde_json::from_value(payload.clone()).map_err(|e| {
            DesktopError::InvalidInput(format!("Invalid import_profile payload: {e}"))
        })?;
        let config = import_profile_json(&req.profile_json)?;
        serde_json::to_value(ImportProfileResult { config })
            .map_err(|e| DesktopError::Internal(format!("Serialization error: {e}")))
    }
}
