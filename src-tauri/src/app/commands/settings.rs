use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::domain::types::*;
use crate::settings::{
    clear_hugging_face_secret, clear_summary_provider_secret, update_hugging_face_token,
    update_summary_provider_api_key,
};
use crate::worker::runtime::run_diarization_setup;

use super::overlay::sync_recording_overlay;
use super::pipeline::{emit_snapshot_update, spawn_pipeline_processing};
use super::recordings::toggle_recording_lifecycle;

#[tauri::command]
pub fn update_app_settings(
    app: tauri::AppHandle,
    input: AppSettingsUpdate,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let provider_api_key_configured = update_summary_provider_api_key(&input)?;
    let hugging_face_token_configured =
        update_hugging_face_token(input.hugging_face_token.as_deref())?;
    let launch_at_login = input.launch_at_login;

    {
        let mut repository = state.repository()?;

        repository
            .update_settings(
                input,
                provider_api_key_configured,
                hugging_face_token_configured,
            )
            .map_err(|error| error.to_string())?;
    }

    refresh_global_hotkey(&app, &state)?;
    sync_launch_at_login(&app, launch_at_login)?;

    let repository = state.repository()?;
    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
    )?;
    emit_snapshot_update(&app, &snapshot);
    spawn_pipeline_processing(app);

    Ok(snapshot)
}

#[tauri::command]
pub fn clear_summary_provider_api_key(
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    clear_summary_provider_secret()?;

    let mut repository = state.repository()?;

    repository
        .update_summary_provider_status(false)
        .map_err(|error| error.to_string())?;
    repository.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_hugging_face_token(
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    clear_hugging_face_secret()?;

    let mut repository = state.repository()?;

    repository
        .update_hugging_face_token_status(false)
        .map_err(|error| error.to_string())?;
    repository.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn setup_diarization_runtime(
    app: tauri::AppHandle,
    input: DiarizationSetupInput,
) -> Result<AppSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<ActavocesState>();

        run_diarization_setup(&app, &state, input)?;

        let repository = state.repository()?;
        repository.snapshot().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Diarization setup task failed: {error}"))?
}

#[tauri::command]
pub fn skip_diarization_setup(
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository()?;

    repository
        .update_diarization_setup_skipped(true)
        .map_err(|error| error.to_string())?;
    repository.snapshot().map_err(|error| error.to_string())
}

pub fn refresh_global_hotkey(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, ActavocesState>,
) -> Result<(), String> {
    let hotkey = {
        let repository = state.repository()?;

        repository
            .settings()
            .map(|settings| settings.hotkey)
            .map_err(|error| error.to_string())?
    };
    let status = register_global_hotkey(app, &hotkey);
    let mut repository = state.repository()?;
    let mut desktop_status = repository
        .desktop_runtime_status()
        .map_err(|error| error.to_string())?;

    desktop_status.hotkey_registered = status.hotkey_registered;
    desktop_status.hotkey_error = status.hotkey_error;
    repository
        .update_desktop_runtime_status(&desktop_status)
        .map_err(|error| error.to_string())
}

pub fn sync_launch_at_login(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let autostart = app.autolaunch();
    let current = autostart
        .is_enabled()
        .map_err(|error| format!("Unable to read launch-at-login status: {error}"))?;

    if current == enabled {
        return Ok(());
    }

    if enabled {
        return autostart
            .enable()
            .map_err(|error| format!("Unable to enable launch at login: {error}"));
    }

    autostart
        .disable()
        .map_err(|error| format!("Unable to disable launch at login: {error}"))
}

pub fn register_global_hotkey(app: &tauri::AppHandle, hotkey: &str) -> DesktopRuntimeStatus {
    let _ = app.global_shortcut().unregister_all();

    let registration = app
        .global_shortcut()
        .on_shortcut(hotkey, |app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }

            let state = app.state::<ActavocesState>();

            match toggle_recording_lifecycle(app, &state) {
                Ok(_) => (),
                Err(error) => {
                    let _ = app.emit("app-error", error);
                }
            }
        });

    match registration {
        Ok(()) => DesktopRuntimeStatus {
            overlay_visible: false,
            hotkey_registered: true,
            hotkey_error: None,
            worker_running: false,
            worker_health_ok: false,
            worker_error: None,
            worker_setup_status: WorkerSetupStatus::Missing,
            worker_setup_step: String::new(),
            worker_setup_error: None,
            cuda_available: false,
            cuda_error: None,
        },
        Err(error) => DesktopRuntimeStatus {
            overlay_visible: false,
            hotkey_registered: false,
            hotkey_error: Some(error.to_string()),
            worker_running: false,
            worker_health_ok: false,
            worker_error: None,
            worker_setup_status: WorkerSetupStatus::Missing,
            worker_setup_step: String::new(),
            worker_setup_error: None,
            cuda_available: false,
            cuda_error: None,
        },
    }
}
