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
#[allow(unused_imports)]
use tauri::Manager;
use tauri::State;

use crate::adapter::DesktopAdapter;
use crate::dto::{DesktopRequestEnvelope, DesktopResponseEnvelope};

pub struct AppState {
    pub adapter: DesktopAdapter,
}

#[tauri::command]
#[allow(dead_code)]
fn desktop_request(
    state: State<'_, Arc<AppState>>,
    request: DesktopRequestEnvelope,
) -> DesktopResponseEnvelope {
    state.adapter.handle_request(request)
}

#[cfg(not(test))]
pub fn run() {
    let backend = crate::worker::ProductionHardwareBackend::new();
    let adapter = DesktopAdapter::new_with_backend(backend);
    let state = Arc::new(AppState { adapter });

    let app = tauri::Builder::<tauri::Wry>::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![desktop_request])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            let state = app_handle.state::<Arc<AppState>>();
            state.adapter.shutdown();
        }
    });
}
