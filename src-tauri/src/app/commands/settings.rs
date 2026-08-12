use std::path::PathBuf;

use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::diarization::prepare_sortformer_diarization;
use crate::dictation::{dispatch_shortcut, DictationShortcutEvent};
use crate::domain::types::*;
use crate::utils::lock_error;
use crate::worker::runtime::run_diarization_setup;

use super::overlay::sync_recording_overlay;
use super::pipeline::{emit_snapshot_update, spawn_pipeline_processing};
use super::recordings::toggle_recording_lifecycle_background;

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
        let _capture_admission = state.capture_admission.lock().map_err(lock_error)?;
        let mut repository = state.repository()?;
        let current_settings = repository.settings().map_err(|error| error.to_string())?;
        let shortcut_settings_changed = input.dictation_hotkey != current_settings.dictation_hotkey
            || input.dictation_shortcut_mode != current_settings.dictation_shortcut_mode;
        if shortcut_settings_changed
            && state
                .dictation_runtime
                .lock()
                .map_err(lock_error)?
                .is_capturing()
        {
            return Err(
                "Dictation shortcut settings cannot change during an active capture".to_owned(),
            );
        }

        repository
            .update_settings(input)
            .map_err(|error| error.to_string())?;
        drop(repository);
        refresh_global_hotkey(&app_for_settings)
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

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
        snapshot.settings.overlay_display_mode,
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
    let status = register_global_hotkeys(app, &settings);
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
    settings: &AppSettings,
) -> DesktopRuntimeStatus {
    let _ = app.global_shortcut().unregister_all();

    let registration = register_meeting_hotkey(app, &settings.hotkey)
        .and_then(|()| register_dictation_hotkey(app, &settings.dictation_hotkey));

    desktop_hotkey_status(registration)
}

fn register_meeting_hotkey(app: &tauri::AppHandle, hotkey: &str) -> Result<(), String> {
    app.global_shortcut()
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
        })
        .map_err(|error| format!("Unable to register meeting shortcut: {error}"))
}

fn register_dictation_hotkey(app: &tauri::AppHandle, hotkey: &str) -> Result<(), String> {
    validate_dictation_hotkey(hotkey)?;
    app.global_shortcut()
        .on_shortcut(hotkey, |app, _shortcut, event| {
            let event = match event.state {
                ShortcutState::Pressed => DictationShortcutEvent::Pressed,
                ShortcutState::Released => DictationShortcutEvent::Released,
            };
            dispatch_shortcut(app.clone(), event);
        })
        .map_err(|error| format!("Unable to register dictation shortcut: {error}"))
}

pub(crate) fn validate_dictation_hotkey(hotkey: &str) -> Result<(), String> {
    if cfg!(target_os = "linux")
        && (std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE")
                .is_ok_and(|session| session.eq_ignore_ascii_case("wayland")))
    {
        return Err(
            "Dictation shortcuts are unsupported on Wayland; use an X11 session".to_owned(),
        );
    }

    let components = hotkey
        .split('+')
        .map(str::trim)
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>();
    let key = components
        .last()
        .ok_or_else(|| "Dictation shortcut is required".to_owned())?;
    let normalized_key = key.to_ascii_lowercase();
    let unsupported = [
        "alt",
        "command",
        "commandorcontrol",
        "control",
        "ctrl",
        "fn",
        "meta",
        "option",
        "shift",
        "super",
    ];
    if unsupported.contains(&normalized_key.as_str())
        || normalized_key.starts_with("media")
        || normalized_key.starts_with("audio")
        || normalized_key.starts_with("browser")
    {
        return Err("Modifier-only, Fn, media, and system keys are unsupported".to_owned());
    }
    if components.len() == 1 && cfg!(target_os = "macos") {
        return Err(
            "Single-key dictation shortcuts are unsupported on macOS because input suppression is not reliable"
                .to_owned(),
        );
    }
    if components.len() == 1
        && !(key.len() == 1
            && key
                .chars()
                .all(|character| character.is_ascii_alphanumeric()))
    {
        return Err("Single-key dictation shortcuts must be a letter or number".to_owned());
    }

    Ok(())
}

fn desktop_hotkey_status(registration: Result<(), String>) -> DesktopRuntimeStatus {
    let (hotkey_registered, hotkey_error) = match registration {
        Ok(()) => (true, None),
        Err(error) => (false, Some(error)),
    };

    DesktopRuntimeStatus {
        overlay_visible: false,
        hotkey_registered,
        hotkey_error,
        worker_running: false,
        worker_health_ok: false,
        worker_error: None,
        worker_setup_status: WorkerSetupStatus::Missing,
        worker_setup_step: String::new(),
        worker_setup_error: None,
        cuda_available: false,
        cuda_error: None,
    }
}
