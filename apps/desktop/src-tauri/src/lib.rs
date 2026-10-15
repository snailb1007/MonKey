pub mod adapter;
pub mod asset;
pub mod dto;
pub mod error;
pub mod fixtures;
pub mod profile;
pub mod worker;

#[cfg(test)]
mod adapter_tests;

#[cfg(test)]
mod asset_tests;

#[cfg(test)]
mod contract_tests;

#[cfg(test)]
mod rgb_tests;

#[cfg(test)]
mod worker_tests;

use std::sync::Arc;
use tauri::State;

use crate::adapter::DesktopAdapter;
use crate::dto::{DesktopRequestEnvelope, DesktopResponseEnvelope};
use crate::worker::MockHardwareBackend;
use monkey_core::transport::MockTransport;

pub struct AppState {
    pub adapter: DesktopAdapter,
}

#[tauri::command]
fn desktop_request(
    state: State<'_, Arc<AppState>>,
    request: DesktopRequestEnvelope,
) -> DesktopResponseEnvelope {
    state.adapter.handle_request(request)
}

#[cfg(not(test))]
pub fn run() {
    let mock = MockTransport::new();
    let (backend, _) = MockHardwareBackend::with_mock_transport(mock);
    let adapter = DesktopAdapter::new_with_backend(backend);
    let state = Arc::new(AppState { adapter });

    let app = tauri::Builder::<tauri::Wry>::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![desktop_request])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, _event| {});
}
