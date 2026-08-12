use std::fs;
use std::path::PathBuf;

use tauri::Manager;

use crate::artifacts::{
    artifact_directory, meta_directory, rename_artifact_directory, renamed_artifact_directory,
    rewrite_diarized_transcript_title, rewrite_raw_transcript_title,
};
use crate::capture::audio::AudioCaptureBackend;
use crate::domain::types::*;
use crate::storage::repository::{AppRepository, NewRecording};
use crate::utils::{lock_error, unix_timestamp};

use super::overlay::sync_recording_overlay;
use super::pipeline::{emit_snapshot_update, spawn_pipeline_processing};
use super::speaker_labels::rewrite_speaker_label;
use super::tray::sync_tray_recording_icon;

#[tauri::command]
pub async fn start_recording(app: tauri::AppHandle) -> Result<AppSnapshot, String> {
    let app_for_start = app.clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || -> Result<AppSnapshot, String> {
        let state = app_for_start.state::<ActavocesState>();
        let mut repository = state.repository()?;
        let mut capture_backend = state.capture_backend.lock().map_err(lock_error)?;

        start_recording_session(&mut repository, &mut *capture_backend)?;
        repository.snapshot().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Recording start task failed: {error}"))??;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
        snapshot.settings.overlay_display_mode,
    )?;
    sync_tray_recording_icon(&app, true);
    emit_snapshot_update(&app, &snapshot);

    Ok(snapshot)
}

#[tauri::command]
pub async fn stop_recording(app: tauri::AppHandle) -> Result<AppSnapshot, String> {
    let app_for_stop = app.clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || -> Result<AppSnapshot, String> {
        let state = app_for_stop.state::<ActavocesState>();
        let mut repository = state.repository()?;
        let mut capture_backend = state.capture_backend.lock().map_err(lock_error)?;

        stop_recording_session(&mut repository, &mut *capture_backend)?;
        repository.snapshot().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Recording stop task failed: {error}"))??;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
        snapshot.settings.overlay_display_mode,
    )?;
    sync_tray_recording_icon(&app, false);
    emit_snapshot_update(&app, &snapshot);
    spawn_pipeline_processing(app);

    Ok(snapshot)
}

#[tauri::command]
pub fn delete_recording(
    input: RecordingDeleteInput,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository()?;

    repository
        .delete_recording(&input.recording_id, input.delete_artifacts)
        .map_err(|error| error.to_string())?;
    repository.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_local_path(input: OpenPathInput) -> Result<(), String> {
    let path = PathBuf::from(input.path);

    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn retry_recording_jobs(
    app: tauri::AppHandle,
    input: RecordingRetryInput,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository()?;

    repository
        .reset_retryable_jobs(&input.recording_id)
        .map_err(|error| error.to_string())?;

    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
        snapshot.settings.overlay_display_mode,
    )?;
    emit_snapshot_update(&app, &snapshot);
    spawn_pipeline_processing(app);

    Ok(snapshot)
}

#[tauri::command]
pub fn rerun_summary_job(
    app: tauri::AppHandle,
    input: RecordingRetryInput,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository()?;

    repository
        .reset_summary_job(&input.recording_id)
        .map_err(|error| error.to_string())?;

    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    emit_snapshot_update(&app, &snapshot);
    spawn_pipeline_processing(app);

    Ok(snapshot)
}

#[tauri::command]
pub fn rename_recording_title(
    app: tauri::AppHandle,
    input: RecordingRenameInput,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let title = input.title.trim();

    if title.is_empty() {
        return Err("Recording title cannot be empty".to_owned());
    }

    let mut repository = state.repository()?;

    let recording = repository
        .recording_by_id(&input.recording_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Recording not found".to_owned())?;

    rename_recording_outputs(&mut repository, &recording, title)?;
    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    emit_snapshot_update(&app, &snapshot);

    Ok(snapshot)
}

pub(crate) fn rename_recording_outputs(
    repository: &mut AppRepository,
    recording: &Recording,
    title: &str,
) -> Result<(), String> {
    let current_directory = PathBuf::from(&recording.artifact_directory);
    let target_directory =
        renamed_artifact_directory(&current_directory, &recording.started_at, title);
    let artifact_directory = rename_artifact_directory(&current_directory, &target_directory)?;

    repository
        .update_recording_title_and_artifact_directory(
            &recording.id,
            title,
            &current_directory,
            &artifact_directory,
        )
        .map_err(|error| error.to_string())?;
    rewrite_raw_transcript_title(&artifact_directory, title)?;
    rewrite_diarized_transcript_title(&artifact_directory, title)
}

#[tauri::command]
pub fn rename_speaker_label(
    input: SpeakerRenameInput,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository()?;
    let recording = repository
        .recording_by_id(&input.recording_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Recording not found".to_owned())?;

    rewrite_speaker_label(&recording, &input)?;
    repository
        .append_event(
            &recording.id,
            PipelineStageId::Diarization,
            PipelineStageStatus::Complete,
            "Speaker label updated",
        )
        .map_err(|error| error.to_string())?;
    repository.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn toggle_recording_from_shortcut(app: tauri::AppHandle) -> Result<AppSnapshot, String> {
    toggle_recording_lifecycle_background(app).await
}

pub fn start_recording_session(
    repository: &mut AppRepository,
    capture_backend: &mut impl AudioCaptureBackend,
) -> Result<(), String> {
    start_recording_session_for_profile(repository, capture_backend, RecordingProfile::Meeting)
}

pub fn start_recording_session_for_profile(
    repository: &mut AppRepository,
    capture_backend: &mut impl AudioCaptureBackend,
    profile: RecordingProfile,
) -> Result<(), String> {
    if repository
        .active_recording()
        .map_err(|error| error.to_string())?
        .is_some()
    {
        return Err("A recording is already active".to_owned());
    }

    let settings = repository.settings().map_err(|error| error.to_string())?;
    let started_at = unix_timestamp();
    let title = match profile {
        RecordingProfile::Meeting => "Untitled meeting".to_owned(),
        RecordingProfile::Dictation => "Untitled dictation".to_owned(),
    };
    let recording_id = format!("recording-{started_at}");
    let artifact_directory = artifact_directory(&settings.output_directory, started_at, &title);

    fs::create_dir_all(meta_directory(&artifact_directory)).map_err(|error| error.to_string())?;
    capture_backend.start(&recording_id, &settings)?;

    let recording = NewRecording {
        id: recording_id.clone(),
        title,
        started_at: started_at.to_string(),
        profile,
        artifact_directory: artifact_directory.display().to_string(),
    };

    repository
        .create_recording(recording)
        .and_then(|()| {
            repository.append_event(
                &recording_id,
                PipelineStageId::Recording,
                PipelineStageStatus::Running,
                "Capture session started",
            )
        })
        .map_err(|error| error.to_string())
}

pub fn stop_recording_session(
    repository: &mut AppRepository,
    capture_backend: &mut impl AudioCaptureBackend,
) -> Result<(), String> {
    let recording = repository
        .active_recording()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No recording is active".to_owned())?;
    let ended_at = unix_timestamp();
    let started_at = recording.started_at.parse::<u64>().unwrap_or(ended_at);
    let duration_seconds = ended_at.saturating_sub(started_at);
    let artifact_directory = PathBuf::from(&recording.artifact_directory);
    let capture_result = capture_backend.stop(&recording.id, &artifact_directory)?;

    repository
        .finish_recording(
            &recording.id,
            ended_at.to_string(),
            duration_seconds,
            capture_result.errors,
            &capture_result.artifacts,
        )
        .and_then(|()| {
            repository.append_event(
                &recording.id,
                PipelineStageId::Recording,
                PipelineStageStatus::Complete,
                "Capture session stopped",
            )
        })
        .and_then(|()| {
            repository.append_event(
                &recording.id,
                PipelineStageId::Transcription,
                PipelineStageStatus::Pending,
                "Local transcription is ready to run",
            )
        })
        .map_err(|error| error.to_string())
}

pub async fn toggle_recording_lifecycle_background(
    app: tauri::AppHandle,
) -> Result<AppSnapshot, String> {
    toggle_recording_lifecycle_for_profile_background(app, RecordingProfile::Meeting).await
}

pub async fn toggle_recording_lifecycle_for_profile_background(
    app: tauri::AppHandle,
    profile: RecordingProfile,
) -> Result<AppSnapshot, String> {
    let app_for_toggle = app.clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || -> Result<AppSnapshot, String> {
        let state = app_for_toggle.state::<ActavocesState>();
        let mut repository = state.repository()?;
        let mut capture_backend = state.capture_backend.lock().map_err(lock_error)?;
        let active_recording = repository
            .active_recording()
            .map_err(|error| error.to_string())?;

        match active_recording {
            Some(recording) if recording.profile == profile => {
                stop_recording_session(&mut repository, &mut *capture_backend)?
            }
            Some(_) => return Err("A different recording profile is already active".to_owned()),
            None => start_recording_session_for_profile(
                &mut repository,
                &mut *capture_backend,
                profile,
            )?,
        }

        repository.snapshot().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Recording toggle task failed: {error}"))??;

    let (position, display_mode) = match profile {
        RecordingProfile::Meeting => (
            snapshot.settings.overlay_position,
            snapshot.settings.overlay_display_mode,
        ),
        RecordingProfile::Dictation => (
            snapshot.settings.dictation_overlay_position,
            snapshot.settings.dictation_overlay_display_mode,
        ),
    };
    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        position,
        display_mode,
    )?;
    sync_tray_recording_icon(&app, snapshot.active_recording.is_some());
    emit_snapshot_update(&app, &snapshot);
    if snapshot.active_recording.is_none() {
        spawn_pipeline_processing(app);
    }

    Ok(snapshot)
}

pub async fn set_recording_profile_active_background(
    app: tauri::AppHandle,
    profile: RecordingProfile,
    active: bool,
) -> Result<AppSnapshot, String> {
    let app_for_update = app.clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || -> Result<AppSnapshot, String> {
        let state = app_for_update.state::<ActavocesState>();
        let mut repository = state.repository()?;
        let mut capture_backend = state.capture_backend.lock().map_err(lock_error)?;
        let active_recording = repository
            .active_recording()
            .map_err(|error| error.to_string())?;

        match (active, active_recording) {
            (true, None) => start_recording_session_for_profile(
                &mut repository,
                &mut *capture_backend,
                profile,
            )?,
            (false, Some(recording)) if recording.profile == profile => {
                stop_recording_session(&mut repository, &mut *capture_backend)?
            }
            (true, Some(_)) => {
                return Err("A different recording profile is already active".to_owned())
            }
            (false, None) | (false, Some(_)) => (),
        }

        repository.snapshot().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Recording update task failed: {error}"))??;

    let (position, display_mode) = match profile {
        RecordingProfile::Meeting => (
            snapshot.settings.overlay_position,
            snapshot.settings.overlay_display_mode,
        ),
        RecordingProfile::Dictation => (
            snapshot.settings.dictation_overlay_position,
            snapshot.settings.dictation_overlay_display_mode,
        ),
    };
    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        position,
        display_mode,
    )?;
    sync_tray_recording_icon(&app, snapshot.active_recording.is_some());
    emit_snapshot_update(&app, &snapshot);
    if snapshot.active_recording.is_none() {
        spawn_pipeline_processing(app);
    }

    Ok(snapshot)
}
