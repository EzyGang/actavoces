mod app;
mod artifacts;
mod capture;
mod domain;
mod settings;
mod storage;
mod utils;
mod worker;

#[cfg(test)]
mod tests;

use std::sync::{Mutex, OnceLock};

use crate::app::commands::{create_recording_overlay, sync_launch_at_login};
use crate::app::commands::{
    emit_snapshot_update, register_global_hotkey, spawn_pipeline_processing, sync_recording_overlay,
};
use crate::capture::audio::NativeAudioCaptureBackend;
use crate::domain::types::{ActavocesState, AppSettings};
use crate::storage::repository::AppRepository;
use crate::worker::runtime::WorkerRuntimeState;
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::MacosLauncher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ActavocesState {
            repository: OnceLock::new(),
            capture_backend: Mutex::new(NativeAudioCaptureBackend::default()),
            worker_runtime: Mutex::new(WorkerRuntimeState::default()),
            pipeline_running: Mutex::new(false),
        })
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            create_recording_overlay(app)?;
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match initialize_app_state(handle.clone()).await {
                    Ok(()) => {}
                    Err(error) => {
                        let _ = handle.emit("app-error", error);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::get_app_snapshot,
            app::commands::update_app_settings,
            app::commands::clear_summary_provider_api_key,
            app::commands::clear_hugging_face_token,
            app::commands::setup_diarization_runtime,
            app::commands::skip_diarization_setup,
            app::commands::start_recording,
            app::commands::stop_recording,
            app::commands::delete_recording,
            app::commands::open_local_path,
            app::commands::retry_recording_jobs,
            app::commands::toggle_recording_from_shortcut,
            app::commands::resume_pending_jobs,
            app::commands::bootstrap_worker_runtime,
            app::commands::get_worker_status,
            app::commands::start_worker,
            app::commands::stop_worker,
            app::commands::check_worker_health,
            app::commands::refresh_model_inventory,
            app::commands::install_transcription_model
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn initialize_app_state(handle: tauri::AppHandle) -> Result<(), String> {
    let database_path = handle
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?
        .join("actavoces.sqlite");
    let (repository, settings) =
        tauri::async_runtime::spawn_blocking(move || initialize_repository(database_path))
            .await
            .map_err(|error| format!("App initialization task failed: {error}"))??;
    let state = handle.state::<ActavocesState>();

    state
        .repository
        .set(Mutex::new(repository))
        .map_err(|_| "Repository was already initialized".to_owned())?;

    if let Err(error) = sync_launch_at_login(&handle, settings.launch_at_login) {
        let _ = handle.emit("app-error", error);
    }

    let status = register_global_hotkey(&handle, &settings.hotkey);
    let snapshot = {
        let mut repository = state.repository()?;

        repository
            .update_desktop_runtime_status(&status)
            .map_err(|error| error.to_string())?;
        repository.snapshot().map_err(|error| error.to_string())?
    };

    sync_recording_overlay(
        &handle,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
    )?;
    emit_snapshot_update(&handle, &snapshot);
    spawn_pipeline_processing(handle);

    Ok(())
}

fn initialize_repository(
    database_path: std::path::PathBuf,
) -> Result<(AppRepository, AppSettings), String> {
    let repository = AppRepository::open(&database_path).map_err(|error| error.to_string())?;
    let settings = repository.settings().map_err(|error| error.to_string())?;

    repository
        .ensure_current_storage_directories()
        .map_err(|error| error.to_string())?;

    Ok((repository, settings))
}
