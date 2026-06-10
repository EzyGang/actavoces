use std::fs;
use std::path::PathBuf;

use crate::artifacts::artifact_directory;
use crate::capture::audio::AudioCaptureBackend;
use crate::domain::types::*;
use crate::storage::repository::{AppRepository, NewRecording};
use crate::utils::{lock_error, unix_timestamp};

use super::overlay::sync_recording_overlay;
use super::pipeline::{emit_snapshot_update, spawn_pipeline_processing};
use super::speaker_labels::rewrite_speaker_label;

#[tauri::command]
pub fn start_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository()?;
    let mut capture_backend = state.capture_backend.lock().map_err(lock_error)?;

    start_recording_session(&mut repository, &mut *capture_backend)?;
    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
    )?;
    emit_snapshot_update(&app, &snapshot);

    Ok(snapshot)
}

#[tauri::command]
pub fn stop_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository()?;
    let mut capture_backend = state.capture_backend.lock().map_err(lock_error)?;
    stop_recording_session(&mut repository, &mut *capture_backend)?;
    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
    )?;
    emit_snapshot_update(&app, &snapshot);

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
    )?;
    emit_snapshot_update(&app, &snapshot);
    spawn_pipeline_processing(app);

    Ok(snapshot)
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
pub fn toggle_recording_from_shortcut(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    toggle_recording_lifecycle(&app, &state)
}

pub fn start_recording_session(
    repository: &mut AppRepository,
    capture_backend: &mut impl AudioCaptureBackend,
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
    let title = "Untitled meeting".to_owned();
    let recording_id = format!("recording-{started_at}");
    let artifact_directory = artifact_directory(&settings.output_directory, started_at, &title);

    fs::create_dir_all(&artifact_directory).map_err(|error| error.to_string())?;
    capture_backend.start(&recording_id, &settings)?;

    let recording = NewRecording {
        id: recording_id.clone(),
        title,
        started_at: started_at.to_string(),
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

pub fn toggle_recording_lifecycle(
    app: &tauri::AppHandle,
    state: &ActavocesState,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository()?;
    let mut capture_backend = state.capture_backend.lock().map_err(lock_error)?;

    match repository
        .active_recording()
        .map_err(|error| error.to_string())?
        .is_some()
    {
        true => stop_recording_session(&mut repository, &mut *capture_backend)?,
        false => start_recording_session(&mut repository, &mut *capture_backend)?,
    }

    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    sync_recording_overlay(
        app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
    )?;
    emit_snapshot_update(app, &snapshot);
    if snapshot.active_recording.is_none() {
        spawn_pipeline_processing(app.clone());
    }

    Ok(snapshot)
}
