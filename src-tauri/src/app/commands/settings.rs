use std::path::PathBuf;

use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::diarization::prepare_sortformer_diarization;
use crate::domain::types::*;
use crate::worker::runtime::run_diarization_setup;

use super::overlay::sync_recording_overlay;
use super::pipeline::{emit_snapshot_update, spawn_pipeline_processing};
use super::recordings::{
    set_recording_profile_active_background, toggle_recording_lifecycle_background,
    toggle_recording_lifecycle_for_profile_background,
};

#[tauri::command]
pub async fn update_app_settings(
    app: tauri::AppHandle,
    input: AppSettingsUpdate,
) -> Result<AppSnapshot, String> {
    let launch_at_login = input.launch_at_login;
    let prepare_sortformer = input.diarization_backend == DiarizationBackend::Sortformer;
    let model_storage_directory = input.model_storage_directory.clone();
    let app_for_settings = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let state = app_for_settings.state::<ActavocesState>();
        let mut repository = state.repository()?;

        repository
            .update_settings(input)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Settings update task failed: {error}"))??;

    refresh_global_hotkey(&app)?;
    sync_launch_at_login(&app, launch_at_login)?;

    let snapshot = {
        let state = app.state::<ActavocesState>();
        let repository = state.repository()?;

        repository.snapshot().map_err(|error| error.to_string())?
    };
    let (overlay_position, overlay_display_mode) = match snapshot.active_recording.as_ref() {
        Some(recording) if recording.profile == RecordingProfile::Dictation => (
            snapshot.settings.dictation_overlay_position,
            snapshot.settings.dictation_overlay_display_mode,
        ),
        Some(_) | None => (
            snapshot.settings.overlay_position,
            snapshot.settings.overlay_display_mode,
        ),
    };
    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        overlay_position,
        overlay_display_mode,
    )?;
    emit_snapshot_update(&app, &snapshot);
    if prepare_sortformer {
        spawn_sortformer_diarization_setup(app.clone(), model_storage_directory);
    }
    spawn_pipeline_processing(app);

    Ok(snapshot)
}

fn spawn_sortformer_diarization_setup(app: tauri::AppHandle, model_storage_directory: String) {
    tauri::async_runtime::spawn_blocking(move || {
        let model_storage_directory = PathBuf::from(model_storage_directory);
        let result = prepare_sortformer_diarization(&model_storage_directory, |progress| {
            let _ = app.emit("sortformer-diarization-progress", progress);
        });

        match result {
            Ok(()) => (),
            Err(error) => {
                let _ = app.emit(
                    "sortformer-diarization-progress",
                    SortformerSetupProgress {
                        status: SortformerSetupStatus::Failed,
                        step: "Sortformer voice attribution setup failed".to_owned(),
                        progress: None,
                        error: Some(error),
                    },
                );
            }
        }
    });
}

#[tauri::command]
pub fn clear_summary_provider_api_key(
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository()?;

    repository
        .clear_summary_provider_api_key()
        .map_err(|error| error.to_string())?;
    repository.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_hugging_face_token(
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository()?;

    repository
        .clear_hugging_face_token()
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
pub fn refresh_global_hotkey(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<ActavocesState>();
    let settings = {
        let repository = state.repository()?;

        repository.settings().map_err(|error| error.to_string())?
    };
    let status = register_global_hotkeys(
        app,
        &settings.hotkey,
        &settings.dictation_hotkey,
        settings.dictation_shortcut_mode,
    );
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

pub fn register_global_hotkeys(
    app: &tauri::AppHandle,
    hotkey: &str,
    dictation_hotkey: &str,
    dictation_shortcut_mode: DictationShortcutMode,
) -> DesktopRuntimeStatus {
    let _ = app.global_shortcut().unregister_all();

    let registration = app
        .global_shortcut()
        .on_shortcut(hotkey, |app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }

            let app = app.clone();

            tauri::async_runtime::spawn(async move {
                match toggle_recording_lifecycle_background(app.clone()).await {
                    Ok(_) => (),
                    Err(error) => {
                        let _ = app.emit("app-error", error);
                    }
                }
            });
        });
    let registration = match registration {
        Ok(()) if dictation_hotkey.is_empty() => Ok(()),
        Ok(()) => {
            app.global_shortcut()
                .on_shortcut(dictation_hotkey, move |app, _shortcut, event| {
                    let active = match dictation_shortcut_mode {
                        DictationShortcutMode::Toggle => event.state == ShortcutState::Pressed,
                        DictationShortcutMode::PushToTalk => match event.state {
                            ShortcutState::Pressed => true,
                            ShortcutState::Released => false,
                        },
                    };
                    if dictation_shortcut_mode == DictationShortcutMode::Toggle
                        && event.state != ShortcutState::Pressed
                    {
                        return;
                    }

                    let app = app.clone();

                    tauri::async_runtime::spawn(async move {
                        let result = match dictation_shortcut_mode {
                            DictationShortcutMode::Toggle => {
                                toggle_recording_lifecycle_for_profile_background(
                                    app.clone(),
                                    RecordingProfile::Dictation,
                                )
                                .await
                            }
                            DictationShortcutMode::PushToTalk => {
                                set_recording_profile_active_background(
                                    app.clone(),
                                    RecordingProfile::Dictation,
                                    active,
                                )
                                .await
                            }
                        };

                        match result {
                            Ok(_) => (),
                            Err(error) => {
                                let _ = app.emit("app-error", error);
                            }
                        }
                    });
                })
        }
        Err(error) => Err(error),
    };

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
