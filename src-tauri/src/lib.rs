mod app;
mod artifacts;
mod capture;
mod diagnostics;
mod diarization;
mod domain;
mod settings;
mod storage;
mod utils;
mod worker;

#[cfg(test)]
mod tests;

use std::sync::{Mutex, OnceLock};

use crate::app::commands::{create_recording_overlay, init_tray, sync_launch_at_login};
use crate::app::commands::{
    emit_snapshot_update, register_global_hotkey, spawn_pipeline_processing,
    sync_recording_overlay, sync_tray_recording_icon,
};
use crate::capture::audio::NativeAudioCaptureBackend;
use crate::domain::types::{ActavocesState, AppSettings};
use crate::storage::repository::AppRepository;
use crate::worker::runtime::WorkerRuntimeState;
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

fn should_close_to_tray(window: &tauri::Window) -> bool {
    let app = window.app_handle();
    let state = app.state::<ActavocesState>();
    let Ok(repository) = state.repository() else {
        return false;
    };
    let Ok(settings) = repository.settings() else {
        return false;
    };

    settings.close_to_tray
}

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
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            match event {
                WindowEvent::CloseRequested { api, .. } if should_close_to_tray(window) => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                _ => (),
            }
        })
        .setup(|app| {
            if let Ok(app_data_directory) = app.path().app_data_dir() {
                let log_directory = app_data_directory.join("logs");
                let _ = diagnostics::initialize(&log_directory);
                diagnostics::info("app.start", env!("CARGO_PKG_VERSION"));
            }

            create_recording_overlay(app)?;
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match initialize_app_state(handle.clone()).await {
                    Ok(()) => {}
                    Err(error) => {
                        diagnostics::error("app.initialize.error", &error);
                        let _ = handle.emit("app-error", error);
                    }
                }
            });

            init_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::snapshot::get_app_snapshot,
            app::commands::settings::update_app_settings,
            app::commands::settings::clear_summary_provider_api_key,
            app::commands::settings::clear_hugging_face_token,
            app::commands::settings::setup_diarization_runtime,
            app::commands::settings::skip_diarization_setup,
            app::commands::recordings::start_recording,
            app::commands::recordings::stop_recording,
            app::commands::recordings::delete_recording,
            app::commands::recordings::open_local_path,
            app::commands::recordings::retry_recording_jobs,
            app::commands::recordings::rename_recording_title,
            app::commands::recordings::rename_speaker_label,
            app::commands::recordings::toggle_recording_from_shortcut,
            app::commands::pipeline::resume_pending_jobs,
            app::commands::worker::bootstrap_worker_runtime,
            app::commands::worker::get_worker_status,
            app::commands::worker::start_worker,
            app::commands::worker::stop_worker,
            app::commands::worker::check_worker_health,
            app::commands::models::refresh_model_inventory,
            app::commands::models::install_transcription_model,
            app::commands::snapshot::write_diagnostic_log
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
        snapshot.settings.overlay_display_mode,
    )?;
    sync_tray_recording_icon(&handle, snapshot.active_recording.is_some());
    emit_snapshot_update(&handle, &snapshot);
    spawn_pipeline_processing(handle);
    diagnostics::info("app.initialize.ready", "Application state initialized");

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
