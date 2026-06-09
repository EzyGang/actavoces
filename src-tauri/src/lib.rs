use std::collections::HashMap;
#[cfg(test)]
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, PhysicalPosition, WebviewUrl};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use keyring_core::{Entry, Error as KeyringError};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    active_recording: Option<Recording>,
    recordings: Vec<Recording>,
    jobs: Vec<PipelineJob>,
    models: Vec<ModelInventoryItem>,
    capture_devices: CaptureDevices,
    desktop: DesktopRuntimeStatus,
    settings: AppSettings,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDevices {
    microphones: Vec<CaptureDeviceInfo>,
    system_sources: Vec<CaptureDeviceInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDeviceInfo {
    name: String,
    label: String,
    default: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRuntimeStatus {
    overlay_visible: bool,
    hotkey_registered: bool,
    hotkey_error: Option<String>,
    worker_running: bool,
    worker_health_ok: bool,
    worker_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerStatus {
    running: bool,
    health_ok: bool,
    last_error: Option<String>,
    mode: WorkerMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkerMode {
    CliJsonl,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerEvent {
    command_id: String,
    event: String,
    payload: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInventoryItem {
    name: String,
    installed: bool,
    setup_required: bool,
    dependency: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    id: String,
    title: String,
    started_at: String,
    ended_at: Option<String>,
    duration_seconds: Option<u64>,
    status: RecordingStatus,
    artifact_directory: String,
    capture_errors: Vec<CaptureError>,
    stages: Vec<PipelineStage>,
    artifacts: Vec<Artifact>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordingStatus {
    Idle,
    Recording,
    Processing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PipelineStageId {
    Recording,
    Transcription,
    Alignment,
    Diarization,
    Summary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PipelineStageStatus {
    Pending,
    Running,
    Complete,
    Failed,
    NeedsSetup,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStage {
    id: PipelineStageId,
    label: String,
    status: PipelineStageStatus,
    progress: u8,
    message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKind {
    Audio,
    MicrophoneAudio,
    SystemAudio,
    RawTranscript,
    Segments,
    Diarization,
    DiarizedTranscript,
    Summary,
    Metadata,
    JobLog,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    kind: ArtifactKind,
    label: String,
    path: String,
    ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineJob {
    id: String,
    recording_id: String,
    stage: PipelineStageId,
    status: PipelineStageStatus,
    progress: u8,
    message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureError {
    source: CaptureSource,
    message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureSource {
    Microphone,
    System,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    output_directory: String,
    database_path: String,
    hotkey: String,
    overlay_position: OverlayPosition,
    launch_at_login: bool,
    microphone_device: String,
    system_audio_source: String,
    sample_rate: u32,
    whisper_model: String,
    transcription_language: String,
    compute_type: String,
    model_storage_directory: String,
    diarization_backend: DiarizationBackend,
    speaker_count_mode: SpeakerCountMode,
    exact_speakers: Option<u8>,
    min_speakers: Option<u8>,
    max_speakers: Option<u8>,
    summary_provider_configured: bool,
    provider_api_key_configured: bool,
    summary_enabled: bool,
    provider_base_url: String,
    provider_model: String,
    title_prompt: String,
    summary_prompt: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsUpdate {
    output_directory: String,
    hotkey: String,
    overlay_position: OverlayPosition,
    launch_at_login: bool,
    microphone_device: String,
    system_audio_source: String,
    sample_rate: u32,
    whisper_model: String,
    transcription_language: String,
    compute_type: String,
    model_storage_directory: String,
    diarization_backend: DiarizationBackend,
    speaker_count_mode: SpeakerCountMode,
    exact_speakers: Option<u8>,
    min_speakers: Option<u8>,
    max_speakers: Option<u8>,
    summary_enabled: bool,
    provider_base_url: String,
    provider_model: String,
    provider_api_key: Option<String>,
    title_prompt: String,
    summary_prompt: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInstallInput {
    model: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverlayPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenPathInput {
    path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingDeleteInput {
    recording_id: String,
    delete_artifacts: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingRetryInput {
    recording_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiarizationBackend {
    NemoWhisper,
    Pyannote,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpeakerCountMode {
    Automatic,
    Exact,
    Range,
}

#[derive(Debug)]
pub struct ActavocesState {
    repository: Mutex<AppRepository>,
    capture_backend: Mutex<NativeAudioCaptureBackend>,
    worker_runtime: Mutex<WorkerRuntimeState>,
}

#[tauri::command]
fn get_app_snapshot(state: tauri::State<'_, ActavocesState>) -> Result<AppSnapshot, String> {
    let repository = state.repository.lock().map_err(lock_error)?;

    repository.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
fn update_app_settings(
    app: tauri::AppHandle,
    input: AppSettingsUpdate,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let provider_api_key_configured = update_summary_provider_api_key(&input)?;

    {
        let mut repository = state.repository.lock().map_err(lock_error)?;

        repository
            .update_settings(input, provider_api_key_configured)
            .map_err(|error| error.to_string())?;
    }

    refresh_global_hotkey(&app, &state)?;

    let repository = state.repository.lock().map_err(lock_error)?;
    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
    )?;

    Ok(snapshot)
}

#[tauri::command]
fn clear_summary_provider_api_key(
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    clear_summary_provider_secret()?;

    let mut repository = state.repository.lock().map_err(lock_error)?;

    repository
        .update_summary_provider_status(false)
        .map_err(|error| error.to_string())?;
    repository.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
fn start_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository.lock().map_err(lock_error)?;
    let mut capture_backend = state.capture_backend.lock().map_err(lock_error)?;

    start_recording_session(&mut repository, &mut *capture_backend)?;
    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
    )?;

    Ok(snapshot)
}

#[tauri::command]
fn stop_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository.lock().map_err(lock_error)?;
    let mut capture_backend = state.capture_backend.lock().map_err(lock_error)?;
    stop_recording_session(&mut repository, &mut *capture_backend)?;
    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
    )?;

    Ok(snapshot)
}

#[tauri::command]
fn delete_recording(
    input: RecordingDeleteInput,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository.lock().map_err(lock_error)?;

    repository
        .delete_recording(&input.recording_id, input.delete_artifacts)
        .map_err(|error| error.to_string())?;
    repository.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
fn open_local_path(input: OpenPathInput) -> Result<(), String> {
    let path = PathBuf::from(input.path);

    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|error| error.to_string())
}

#[tauri::command]
fn retry_recording_jobs(
    app: tauri::AppHandle,
    input: RecordingRetryInput,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository.lock().map_err(lock_error)?;

    repository
        .reset_retryable_jobs(&input.recording_id)
        .map_err(|error| error.to_string())?;
    resume_pipeline_jobs(&mut repository, run_worker_command)?;

    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
    )?;

    Ok(snapshot)
}

#[tauri::command]
fn toggle_recording_from_shortcut(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    toggle_recording_lifecycle(&app, &state)
}

#[tauri::command]
fn resume_pending_jobs(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository.lock().map_err(lock_error)?;

    resume_pipeline_jobs(&mut repository, run_worker_command)?;

    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
    )?;

    Ok(snapshot)
}

#[tauri::command]
fn get_worker_status(state: tauri::State<'_, ActavocesState>) -> Result<WorkerStatus, String> {
    state
        .worker_runtime
        .lock()
        .map(|runtime| runtime.status())
        .map_err(lock_error)
}

#[tauri::command]
fn start_worker(state: tauri::State<'_, ActavocesState>) -> Result<WorkerStatus, String> {
    let status = {
        let mut runtime = state.worker_runtime.lock().map_err(lock_error)?;

        runtime.running = true;
        runtime.status()
    };

    persist_worker_status(&state, &status)?;

    Ok(status)
}

#[tauri::command]
fn stop_worker(state: tauri::State<'_, ActavocesState>) -> Result<WorkerStatus, String> {
    let status = {
        let mut runtime = state.worker_runtime.lock().map_err(lock_error)?;

        runtime.running = false;
        runtime.health_ok = false;
        runtime.status()
    };

    persist_worker_status(&state, &status)?;

    Ok(status)
}

#[tauri::command]
fn check_worker_health(state: tauri::State<'_, ActavocesState>) -> Result<WorkerStatus, String> {
    let status = {
        let mut runtime = state.worker_runtime.lock().map_err(lock_error)?;

        runtime.running = true;

        match run_worker_command("health.check", serde_json::json!({})) {
            Ok(events) if events.iter().any(|event| event.event == "health.ok") => {
                runtime.health_ok = true;
                runtime.last_error = None;
            }
            Ok(events) => {
                runtime.health_ok = false;
                runtime.last_error = Some(format!("Unexpected worker events: {}", events.len()));
            }
            Err(error) => {
                runtime.health_ok = false;
                runtime.last_error = Some(error);
            }
        }

        runtime.status()
    };

    persist_worker_status(&state, &status)?;

    Ok(status)
}

#[tauri::command]
fn refresh_model_inventory(state: tauri::State<'_, ActavocesState>) -> Result<AppSnapshot, String> {
    let settings = {
        let repository = state.repository.lock().map_err(lock_error)?;

        repository.settings().map_err(|error| error.to_string())?
    };
    let result = run_worker_command(
        "models.status",
        serde_json::json!({
            "modelStorageDirectory": settings.model_storage_directory,
        }),
    );

    match result {
        Ok(events) => {
            let models = extract_model_inventory(&events)?;
            let mut repository = state.repository.lock().map_err(lock_error)?;

            repository
                .replace_model_inventory(&models)
                .map_err(|error| error.to_string())?;
            repository
                .clear_worker_error()
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
        Err(error) => {
            let mut repository = state.repository.lock().map_err(lock_error)?;

            repository
                .set_worker_error(&error)
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
    }
}

#[tauri::command]
fn install_transcription_model(
    input: ModelInstallInput,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let settings = {
        let repository = state.repository.lock().map_err(lock_error)?;

        repository.settings().map_err(|error| error.to_string())?
    };
    let result = run_worker_command(
        "models.install",
        serde_json::json!({
            "model": input.model,
            "computeType": settings.compute_type,
            "modelStorageDirectory": settings.model_storage_directory,
        }),
    );

    match result {
        Ok(events)
            if events
                .iter()
                .any(|event| event.event == "models.install.complete") =>
        {
            refresh_model_inventory(state)
        }
        Ok(events) => {
            let message = model_install_message(&events);
            let mut repository = state.repository.lock().map_err(lock_error)?;

            repository
                .set_worker_error(&message)
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
        Err(error) => {
            let mut repository = state.repository.lock().map_err(lock_error)?;

            repository
                .set_worker_error(&error)
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
    }
}

fn start_recording_session(
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

fn stop_recording_session(
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

fn persist_worker_status(
    state: &tauri::State<'_, ActavocesState>,
    worker_status: &WorkerStatus,
) -> Result<(), String> {
    let mut repository = state.repository.lock().map_err(lock_error)?;

    repository
        .update_worker_runtime_status(worker_status)
        .map_err(|error| error.to_string())
}

fn resume_pipeline_jobs<F>(repository: &mut AppRepository, mut run_worker: F) -> Result<(), String>
where
    F: FnMut(&str, serde_json::Value) -> Result<Vec<WorkerEvent>, String>,
{
    let settings = repository.settings().map_err(|error| error.to_string())?;
    let recordings = repository.recordings().map_err(|error| error.to_string())?;

    for recording in recordings {
        if recording.status != RecordingStatus::Processing {
            continue;
        }

        if should_run_stage(repository, &recording.id, PipelineStageId::Transcription)? {
            run_pipeline_stage(
                repository,
                &recording,
                PipelineStageId::Transcription,
                transcription_payload(&recording, &settings),
                "transcribe.run",
                &mut run_worker,
            )?;
        }

        if stage_is_complete(repository, &recording.id, PipelineStageId::Transcription)? {
            complete_alignment_stage(repository, &recording.id)?;
        }

        if stage_is_complete(repository, &recording.id, PipelineStageId::Transcription)?
            && should_run_stage(repository, &recording.id, PipelineStageId::Diarization)?
        {
            run_pipeline_stage(
                repository,
                &recording,
                PipelineStageId::Diarization,
                diarization_payload(&recording, &settings),
                "diarize.run",
                &mut run_worker,
            )?;
        }

        if should_run_stage(repository, &recording.id, PipelineStageId::Summary)? {
            if !settings.summary_enabled {
                complete_disabled_summary_stage(repository, &recording.id)?;
                continue;
            }

            let Some(api_key) = read_summary_provider_api_key()? else {
                mark_stage_needs_setup(
                    repository,
                    &recording.id,
                    PipelineStageId::Summary,
                    "Summary provider API key is required",
                )?;
                continue;
            };

            run_pipeline_stage(
                repository,
                &recording,
                PipelineStageId::Summary,
                summary_payload(&recording, &settings, api_key),
                "summarize.run",
                &mut run_worker,
            )?;
        }
    }

    Ok(())
}

fn run_pipeline_stage<F>(
    repository: &mut AppRepository,
    recording: &Recording,
    stage: PipelineStageId,
    payload: serde_json::Value,
    command_name: &str,
    run_worker: &mut F,
) -> Result<(), String>
where
    F: FnMut(&str, serde_json::Value) -> Result<Vec<WorkerEvent>, String>,
{
    let job_id = pipeline_job_id(&recording.id, stage)?;

    repository
        .update_job(
            &job_id,
            PipelineStageStatus::Running,
            5,
            "Worker stage started",
        )
        .and_then(|()| {
            repository.append_event(
                &recording.id,
                stage,
                PipelineStageStatus::Running,
                "Worker stage started",
            )
        })
        .map_err(|error| error.to_string())?;

    let events = match run_worker(command_name, payload) {
        Ok(events) => events,
        Err(error) => {
            mark_stage_failed(repository, &recording.id, stage, &error)?;

            return Ok(());
        }
    };

    apply_worker_events(repository, recording, stage, &events)
}

fn apply_worker_events(
    repository: &mut AppRepository,
    recording: &Recording,
    stage: PipelineStageId,
    events: &[WorkerEvent],
) -> Result<(), String> {
    for event in events {
        if event.event.ends_with(".progress") {
            let progress = event
                .payload
                .get("progress")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(5)
                .min(100) as u8;

            repository
                .update_job(
                    &pipeline_job_id(&recording.id, stage)?,
                    PipelineStageStatus::Running,
                    progress,
                    "Worker stage running",
                )
                .map_err(|error| error.to_string())?;
            continue;
        }

        if event.event.ends_with(".needs_setup") {
            mark_stage_needs_setup(repository, &recording.id, stage, "Worker setup is required")?;
            continue;
        }

        if event.event == "command.failed" {
            let message = event
                .payload
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Worker command failed");

            mark_stage_failed(repository, &recording.id, stage, message)?;
            continue;
        }

        if event.event.ends_with(".complete") {
            apply_complete_event(repository, recording, stage, event)?;
        }
    }

    Ok(())
}

fn apply_complete_event(
    repository: &mut AppRepository,
    recording: &Recording,
    stage: PipelineStageId,
    event: &WorkerEvent,
) -> Result<(), String> {
    match stage {
        PipelineStageId::Transcription => {
            upsert_ready_artifact_from_event(
                repository,
                &recording.id,
                ArtifactKind::Segments,
                "Raw segments",
                event,
                "segmentsPath",
            )?;
            upsert_ready_artifact_from_event(
                repository,
                &recording.id,
                ArtifactKind::RawTranscript,
                "Raw transcript",
                event,
                "transcriptPath",
            )?;
        }
        PipelineStageId::Diarization => {
            upsert_ready_artifact_from_event(
                repository,
                &recording.id,
                ArtifactKind::Diarization,
                "Diarization turns",
                event,
                "diarizationPath",
            )?;
            upsert_ready_artifact_from_event(
                repository,
                &recording.id,
                ArtifactKind::DiarizedTranscript,
                "Diarized transcript",
                event,
                "transcriptPath",
            )?;
        }
        PipelineStageId::Summary => {
            upsert_ready_artifact_from_event(
                repository,
                &recording.id,
                ArtifactKind::Summary,
                "Summary",
                event,
                "summaryPath",
            )?;

            if let Some(title) = event
                .payload
                .get("title")
                .and_then(serde_json::Value::as_str)
            {
                if !title.trim().is_empty() {
                    repository
                        .update_recording_title(&recording.id, title.trim())
                        .map_err(|error| error.to_string())?;
                }
            }
        }
        PipelineStageId::Recording | PipelineStageId::Alignment => {}
    }

    mark_stage_complete(repository, &recording.id, stage, "Worker stage complete")
}

fn upsert_ready_artifact_from_event(
    repository: &mut AppRepository,
    recording_id: &str,
    kind: ArtifactKind,
    label: &str,
    event: &WorkerEvent,
    payload_key: &str,
) -> Result<(), String> {
    let Some(path) = event
        .payload
        .get(payload_key)
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(());
    };

    repository
        .upsert_artifact(
            recording_id,
            &artifact(kind, label, PathBuf::from(path), true),
        )
        .map_err(|error| error.to_string())
}

fn complete_alignment_stage(
    repository: &mut AppRepository,
    recording_id: &str,
) -> Result<(), String> {
    if !should_run_stage(repository, recording_id, PipelineStageId::Alignment)? {
        return Ok(());
    }

    mark_stage_complete(
        repository,
        recording_id,
        PipelineStageId::Alignment,
        "Alignment is included in diarization",
    )
}

fn complete_disabled_summary_stage(
    repository: &mut AppRepository,
    recording_id: &str,
) -> Result<(), String> {
    mark_stage_complete(
        repository,
        recording_id,
        PipelineStageId::Summary,
        "Summary generation is disabled",
    )
}

fn mark_stage_complete(
    repository: &mut AppRepository,
    recording_id: &str,
    stage: PipelineStageId,
    message: &str,
) -> Result<(), String> {
    repository
        .update_job(
            &pipeline_job_id(recording_id, stage)?,
            PipelineStageStatus::Complete,
            100,
            message,
        )
        .and_then(|()| {
            repository.append_event(recording_id, stage, PipelineStageStatus::Complete, message)
        })
        .map_err(|error| error.to_string())
}

fn mark_stage_needs_setup(
    repository: &mut AppRepository,
    recording_id: &str,
    stage: PipelineStageId,
    message: &str,
) -> Result<(), String> {
    repository
        .update_job(
            &pipeline_job_id(recording_id, stage)?,
            PipelineStageStatus::NeedsSetup,
            0,
            message,
        )
        .and_then(|()| {
            repository.append_event(
                recording_id,
                stage,
                PipelineStageStatus::NeedsSetup,
                message,
            )
        })
        .map_err(|error| error.to_string())
}

fn mark_stage_failed(
    repository: &mut AppRepository,
    recording_id: &str,
    stage: PipelineStageId,
    message: &str,
) -> Result<(), String> {
    repository
        .update_job(
            &pipeline_job_id(recording_id, stage)?,
            PipelineStageStatus::Failed,
            0,
            message,
        )
        .and_then(|()| {
            repository.append_event(recording_id, stage, PipelineStageStatus::Failed, message)
        })
        .map_err(|error| error.to_string())
}

fn should_run_stage(
    repository: &AppRepository,
    recording_id: &str,
    stage: PipelineStageId,
) -> Result<bool, String> {
    let job = repository
        .job_for_recording_stage(recording_id, stage)
        .map_err(|error| error.to_string())?;

    Ok(matches!(
        job.status,
        PipelineStageStatus::Pending
            | PipelineStageStatus::Running
            | PipelineStageStatus::NeedsSetup
    ))
}

fn stage_is_complete(
    repository: &AppRepository,
    recording_id: &str,
    stage: PipelineStageId,
) -> Result<bool, String> {
    let job = repository
        .job_for_recording_stage(recording_id, stage)
        .map_err(|error| error.to_string())?;

    Ok(job.status == PipelineStageStatus::Complete)
}

fn transcription_payload(recording: &Recording, settings: &AppSettings) -> serde_json::Value {
    let artifact_directory = PathBuf::from(&recording.artifact_directory);

    serde_json::json!({
        "audioPath": artifact_directory.join("recording.wav"),
        "outputDirectory": artifact_directory,
        "model": settings.whisper_model,
        "language": settings.transcription_language,
        "computeType": settings.compute_type,
        "modelStorageDirectory": settings.model_storage_directory,
    })
}

fn diarization_payload(recording: &Recording, settings: &AppSettings) -> serde_json::Value {
    let artifact_directory = PathBuf::from(&recording.artifact_directory);
    let segments = read_json_file(artifact_directory.join("raw-segments.json"))
        .and_then(|value| value.get("segments").cloned())
        .unwrap_or_else(|| serde_json::json!([]));

    serde_json::json!({
        "outputDirectory": artifact_directory,
        "segments": segments,
        "backend": settings.diarization_backend,
        "speakerCountMode": settings.speaker_count_mode,
        "exactSpeakers": settings.exact_speakers,
        "minSpeakers": settings.min_speakers,
        "maxSpeakers": settings.max_speakers,
    })
}

fn summary_payload(
    recording: &Recording,
    settings: &AppSettings,
    api_key: String,
) -> serde_json::Value {
    let artifact_directory = PathBuf::from(&recording.artifact_directory);

    serde_json::json!({
        "outputDirectory": artifact_directory,
        "providerBaseUrl": settings.provider_base_url,
        "apiKey": api_key,
        "model": settings.provider_model,
        "diarizedTranscriptPath": artifact_directory.join("diarized-transcript.md"),
        "transcriptPath": artifact_directory.join("raw-transcript.md"),
        "titlePrompt": settings.title_prompt,
        "summaryPrompt": settings.summary_prompt,
    })
}

fn create_recording_overlay(app: &tauri::App) -> tauri::Result<()> {
    tauri::WebviewWindowBuilder::new(
        app,
        "recording-overlay",
        WebviewUrl::App("index.html".into()),
    )
    .title("ActaVoces recording")
    .inner_size(260.0, 84.0)
    .position(24.0, 24.0)
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()?;

    Ok(())
}

fn sync_recording_overlay(
    app: &tauri::AppHandle,
    visible: bool,
    position: OverlayPosition,
) -> Result<(), String> {
    let Some(overlay) = app.get_webview_window("recording-overlay") else {
        return Ok(());
    };

    if visible {
        position_recording_overlay(&overlay, position)?;
        overlay.show().map_err(|error| error.to_string())?;
        return Ok(());
    }

    overlay.hide().map_err(|error| error.to_string())
}

fn position_recording_overlay(
    overlay: &tauri::WebviewWindow,
    position: OverlayPosition,
) -> Result<(), String> {
    let margin = 24;
    let size = overlay.outer_size().map_err(|error| error.to_string())?;
    let monitor = overlay
        .current_monitor()
        .map_err(|error| error.to_string())?
        .or_else(|| overlay.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        overlay
            .set_position(PhysicalPosition::new(margin, margin))
            .map_err(|error| error.to_string())?;

        return Ok(());
    };
    let monitor_origin = monitor.position();
    let monitor_size = monitor.size();
    let left = monitor_origin.x + margin;
    let right = monitor_origin.x + monitor_size.width as i32 - size.width as i32 - margin;
    let top = monitor_origin.y + margin;
    let bottom = monitor_origin.y + monitor_size.height as i32 - size.height as i32 - margin;
    let next_position = match position {
        OverlayPosition::TopLeft => PhysicalPosition::new(left, top),
        OverlayPosition::TopRight => PhysicalPosition::new(right, top),
        OverlayPosition::BottomLeft => PhysicalPosition::new(left, bottom),
        OverlayPosition::BottomRight => PhysicalPosition::new(right, bottom),
    };

    overlay
        .set_position(next_position)
        .map_err(|error| error.to_string())
}

fn refresh_global_hotkey(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, ActavocesState>,
) -> Result<(), String> {
    let hotkey = {
        let repository = state.repository.lock().map_err(lock_error)?;

        repository
            .settings()
            .map(|settings| settings.hotkey)
            .map_err(|error| error.to_string())?
    };
    let status = register_global_hotkey(app, &hotkey);
    let mut repository = state.repository.lock().map_err(lock_error)?;
    let mut desktop_status = repository
        .desktop_runtime_status()
        .map_err(|error| error.to_string())?;

    desktop_status.hotkey_registered = status.hotkey_registered;
    desktop_status.hotkey_error = status.hotkey_error;
    repository
        .update_desktop_runtime_status(&desktop_status)
        .map_err(|error| error.to_string())
}

fn register_global_hotkey(app: &tauri::AppHandle, hotkey: &str) -> DesktopRuntimeStatus {
    let _ = app.global_shortcut().unregister_all();

    let registration = app
        .global_shortcut()
        .on_shortcut(hotkey, |app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }

            let state = app.state::<ActavocesState>();

            match toggle_recording_lifecycle(app, &state) {
                Ok(snapshot) => {
                    let _ = app.emit("app-snapshot-updated", snapshot);
                }
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
        },
        Err(error) => DesktopRuntimeStatus {
            overlay_visible: false,
            hotkey_registered: false,
            hotkey_error: Some(error.to_string()),
            worker_running: false,
            worker_health_ok: false,
            worker_error: None,
        },
    }
}

fn toggle_recording_lifecycle(
    app: &tauri::AppHandle,
    state: &ActavocesState,
) -> Result<AppSnapshot, String> {
    let mut repository = state.repository.lock().map_err(lock_error)?;
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

    Ok(snapshot)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let database_path = match app.path().app_data_dir() {
                Ok(path) => path.join("actavoces.sqlite"),
                Err(error) => return Err(Box::new(error)),
            };
            let repository = match AppRepository::open(&database_path) {
                Ok(repository) => repository,
                Err(error) => return Err(Box::new(error)),
            };
            let hotkey = match repository.settings() {
                Ok(settings) => settings.hotkey,
                Err(error) => return Err(Box::new(error)),
            };
            match repository.ensure_current_storage_directories() {
                Ok(()) => {}
                Err(error) => return Err(Box::new(error)),
            }
            create_recording_overlay(app)?;

            app.manage(ActavocesState {
                repository: Mutex::new(repository),
                capture_backend: Mutex::new(NativeAudioCaptureBackend::default()),
                worker_runtime: Mutex::new(WorkerRuntimeState::default()),
            });
            let state = app.state::<ActavocesState>();
            let status = register_global_hotkey(app.handle(), &hotkey);

            match state.repository.lock() {
                Ok(mut repository) => {
                    if let Err(error) = repository.update_desktop_runtime_status(&status) {
                        return Err(Box::new(error));
                    }
                }
                Err(error) => {
                    return Err(Box::new(std::io::Error::other(error.to_string())));
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_snapshot,
            update_app_settings,
            clear_summary_provider_api_key,
            start_recording,
            stop_recording,
            delete_recording,
            open_local_path,
            retry_recording_jobs,
            toggle_recording_from_shortcut,
            resume_pending_jobs,
            get_worker_status,
            start_worker,
            stop_worker,
            check_worker_health,
            refresh_model_inventory,
            install_transcription_model
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug)]
struct AppRepository {
    connection: Connection,
    database_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NewRecording {
    id: String,
    title: String,
    started_at: String,
    artifact_directory: String,
}

impl AppRepository {
    fn open(database_path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }

        let connection = Connection::open(database_path)?;
        let repository = Self {
            connection,
            database_path: database_path.to_path_buf(),
        };

        repository.migrate()?;
        repository.seed_defaults()?;

        Ok(repository)
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        self.connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS recordings (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                duration_seconds INTEGER,
                status TEXT NOT NULL,
                artifact_directory TEXT NOT NULL,
                capture_errors TEXT NOT NULL DEFAULT '[]'
            );

            CREATE TABLE IF NOT EXISTS recording_artifacts (
                recording_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                label TEXT NOT NULL,
                path TEXT NOT NULL,
                ready INTEGER NOT NULL,
                PRIMARY KEY (recording_id, kind),
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS pipeline_jobs (
                id TEXT PRIMARY KEY,
                recording_id TEXT NOT NULL,
                stage TEXT NOT NULL,
                status TEXT NOT NULL,
                progress INTEGER NOT NULL,
                message TEXT NOT NULL,
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS providers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                base_url TEXT NOT NULL,
                model TEXT NOT NULL,
                enabled INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS models (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                name TEXT NOT NULL,
                installed INTEGER NOT NULL,
                setup_required INTEGER NOT NULL DEFAULT 1,
                dependency TEXT NOT NULL DEFAULT 'faster-whisper'
            );

            CREATE TABLE IF NOT EXISTS job_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                recording_id TEXT NOT NULL,
                stage TEXT NOT NULL,
                status TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
            );

            INSERT OR IGNORE INTO schema_migrations (version, applied_at)
            VALUES (1, strftime('%s', 'now'));
            ",
        )?;
        self.ensure_column(
            "models",
            "setup_required",
            "ALTER TABLE models ADD COLUMN setup_required INTEGER NOT NULL DEFAULT 1",
        )?;
        self.ensure_column(
            "models",
            "dependency",
            "ALTER TABLE models ADD COLUMN dependency TEXT NOT NULL DEFAULT 'faster-whisper'",
        )
    }

    fn ensure_column(&self, table: &str, column: &str, sql: &str) -> rusqlite::Result<()> {
        let mut statement = self
            .connection
            .prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = statement.query_map([], |row| row.get::<_, String>(1))?;

        for existing_column in rows {
            if existing_column? == column {
                return Ok(());
            }
        }

        self.connection.execute(sql, [])?;

        Ok(())
    }

    fn seed_defaults(&self) -> rusqlite::Result<()> {
        let settings = default_settings(&self.database_path);

        self.upsert_setting("outputDirectory", &settings.output_directory)?;
        self.upsert_setting("databasePath", &settings.database_path)?;
        self.upsert_setting("hotkey", &settings.hotkey)?;
        self.upsert_setting("overlayPosition", &json_string(&settings.overlay_position)?)?;
        self.upsert_setting("launchAtLogin", &settings.launch_at_login.to_string())?;
        self.upsert_setting("microphoneDevice", &settings.microphone_device)?;
        self.upsert_setting("systemAudioSource", &settings.system_audio_source)?;
        self.upsert_setting("sampleRate", &settings.sample_rate.to_string())?;
        self.upsert_setting("whisperModel", &settings.whisper_model)?;
        self.upsert_setting("transcriptionLanguage", &settings.transcription_language)?;
        self.upsert_setting("computeType", &settings.compute_type)?;
        self.upsert_setting("modelStorageDirectory", &settings.model_storage_directory)?;
        self.upsert_setting(
            "diarizationBackend",
            &json_string(&settings.diarization_backend)?,
        )?;
        self.upsert_setting(
            "speakerCountMode",
            &json_string(&settings.speaker_count_mode)?,
        )?;
        self.upsert_setting(
            "exactSpeakers",
            &option_number_to_string(settings.exact_speakers),
        )?;
        self.upsert_setting(
            "minSpeakers",
            &option_number_to_string(settings.min_speakers),
        )?;
        self.upsert_setting(
            "maxSpeakers",
            &option_number_to_string(settings.max_speakers),
        )?;
        self.upsert_setting(
            "summaryProviderConfigured",
            &settings.summary_provider_configured.to_string(),
        )?;
        self.upsert_setting(
            "providerApiKeyConfigured",
            &settings.provider_api_key_configured.to_string(),
        )?;
        self.upsert_setting("overlayVisible", "false")?;
        self.upsert_setting("hotkeyRegistered", "false")?;
        self.upsert_setting("hotkeyError", "")?;
        self.upsert_setting("workerRunning", "false")?;
        self.upsert_setting("workerHealthOk", "false")?;
        self.upsert_setting("workerError", "")?;
        self.upsert_setting("summaryEnabled", &settings.summary_enabled.to_string())?;
        self.upsert_setting("providerBaseUrl", &settings.provider_base_url)?;
        self.upsert_setting("providerModel", &settings.provider_model)?;
        self.upsert_setting("titlePrompt", &settings.title_prompt)?;
        self.upsert_setting("summaryPrompt", &settings.summary_prompt)?;
        self.seed_default_models()
    }

    fn upsert_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            INSERT OR IGNORE INTO settings (key, value)
            VALUES (?1, ?2)
            ",
            params![key, value],
        )?;

        Ok(())
    }

    fn seed_default_models(&self) -> rusqlite::Result<()> {
        for model in default_model_inventory() {
            self.connection.execute(
                "
                INSERT OR IGNORE INTO models (id, provider, name, installed, setup_required, dependency)
                VALUES (?1, 'faster-whisper', ?2, ?3, ?4, ?5)
                ",
                params![
                    model.name,
                    model.name,
                    model.installed,
                    model.setup_required,
                    model.dependency,
                ],
            )?;
        }

        Ok(())
    }

    fn settings(&self) -> rusqlite::Result<AppSettings> {
        let mut rows = self.connection.prepare("SELECT key, value FROM settings")?;
        let setting_pairs = rows.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut values = Vec::new();

        for pair in setting_pairs {
            values.push(pair?);
        }

        let get_value = |key: &str, fallback: &str| -> String {
            for (candidate_key, value) in &values {
                if candidate_key == key {
                    return value.clone();
                }
            }

            fallback.to_owned()
        };
        let summary_enabled = parse_bool(&get_value("summaryEnabled", "false"));
        let provider_base_url = get_value("providerBaseUrl", "https://api.openai.com/v1");
        let provider_model = get_value("providerModel", "");
        let provider_api_key_configured = summary_provider_api_key_configured();
        let summary_provider_configured = summary_provider_configured_for(
            summary_enabled,
            &provider_base_url,
            &provider_model,
            provider_api_key_configured,
        );

        Ok(AppSettings {
            output_directory: get_value("outputDirectory", &default_records_root()),
            database_path: self.database_path.display().to_string(),
            hotkey: get_value("hotkey", "CommandOrControl+Shift+Space"),
            overlay_position: serde_json::from_str(&get_value("overlayPosition", "\"topLeft\""))
                .unwrap_or(OverlayPosition::TopLeft),
            launch_at_login: parse_bool(&get_value("launchAtLogin", "false")),
            microphone_device: get_value("microphoneDevice", "Default microphone"),
            system_audio_source: get_value("systemAudioSource", "Default system output"),
            sample_rate: get_value("sampleRate", "48000").parse().unwrap_or(48_000),
            whisper_model: get_value("whisperModel", "medium.en"),
            transcription_language: get_value("transcriptionLanguage", "auto"),
            compute_type: get_value("computeType", "auto"),
            model_storage_directory: get_value(
                "modelStorageDirectory",
                &default_model_storage_root(),
            ),
            diarization_backend: serde_json::from_str(&get_value(
                "diarizationBackend",
                "\"nemoWhisper\"",
            ))
            .unwrap_or(DiarizationBackend::NemoWhisper),
            speaker_count_mode: serde_json::from_str(&get_value(
                "speakerCountMode",
                "\"automatic\"",
            ))
            .unwrap_or(SpeakerCountMode::Automatic),
            exact_speakers: parse_optional_number(&get_value("exactSpeakers", "")),
            min_speakers: parse_optional_number(&get_value("minSpeakers", "")),
            max_speakers: parse_optional_number(&get_value("maxSpeakers", "")),
            summary_provider_configured,
            provider_api_key_configured,
            summary_enabled,
            provider_base_url,
            provider_model,
            title_prompt: get_value("titlePrompt", DEFAULT_TITLE_PROMPT),
            summary_prompt: get_value("summaryPrompt", DEFAULT_SUMMARY_PROMPT),
        })
    }

    fn update_settings(
        &mut self,
        input: AppSettingsUpdate,
        provider_api_key_configured: bool,
    ) -> rusqlite::Result<()> {
        validate_settings(&input, provider_api_key_configured)?;
        ensure_configured_storage_directories(
            &input.output_directory,
            &input.model_storage_directory,
        )?;

        let transaction = self.connection.transaction()?;
        let provider_configured = summary_provider_configured_for(
            input.summary_enabled,
            &input.provider_base_url,
            &input.provider_model,
            provider_api_key_configured,
        );

        for (key, value) in settings_pairs(&input, provider_configured, provider_api_key_configured)
        {
            transaction.execute(
                "
                INSERT INTO settings (key, value)
                VALUES (?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value
                ",
                params![key, value],
            )?;
        }

        transaction.execute(
            "
            INSERT INTO providers (id, name, base_url, model, enabled)
            VALUES ('summary', 'OpenAI-compatible summary', ?1, ?2, ?3)
            ON CONFLICT(id) DO UPDATE SET
                base_url = excluded.base_url,
                model = excluded.model,
                enabled = excluded.enabled
            ",
            params![
                input.provider_base_url,
                input.provider_model,
                provider_configured
            ],
        )?;
        transaction.commit()
    }

    fn ensure_current_storage_directories(&self) -> rusqlite::Result<()> {
        let settings = self.settings()?;

        ensure_configured_storage_directories(
            &settings.output_directory,
            &settings.model_storage_directory,
        )
    }

    fn update_summary_provider_status(
        &mut self,
        provider_api_key_configured: bool,
    ) -> rusqlite::Result<()> {
        let settings = self.settings()?;
        let provider_configured = summary_provider_configured_for(
            settings.summary_enabled,
            &settings.provider_base_url,
            &settings.provider_model,
            provider_api_key_configured,
        );
        let transaction = self.connection.transaction()?;

        for (key, value) in [
            ("summaryProviderConfigured", provider_configured.to_string()),
            (
                "providerApiKeyConfigured",
                provider_api_key_configured.to_string(),
            ),
        ] {
            transaction.execute(
                "
                INSERT INTO settings (key, value)
                VALUES (?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value
                ",
                params![key, value],
            )?;
        }

        transaction.execute(
            "
            UPDATE providers
            SET enabled = ?1
            WHERE id = 'summary'
            ",
            params![provider_configured],
        )?;
        transaction.commit()
    }

    fn update_desktop_runtime_status(
        &mut self,
        status: &DesktopRuntimeStatus,
    ) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;
        let values = [
            ("overlayVisible", status.overlay_visible.to_string()),
            ("hotkeyRegistered", status.hotkey_registered.to_string()),
            (
                "hotkeyError",
                status.hotkey_error.clone().unwrap_or_default(),
            ),
            ("workerRunning", status.worker_running.to_string()),
            ("workerHealthOk", status.worker_health_ok.to_string()),
            (
                "workerError",
                status.worker_error.clone().unwrap_or_default(),
            ),
        ];

        for (key, value) in values {
            transaction.execute(
                "
                INSERT INTO settings (key, value)
                VALUES (?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value
                ",
                params![key, value],
            )?;
        }

        transaction.commit()
    }

    fn update_worker_runtime_status(&mut self, status: &WorkerStatus) -> rusqlite::Result<()> {
        let mut desktop_status = self.desktop_runtime_status()?;

        desktop_status.worker_running = status.running;
        desktop_status.worker_health_ok = status.health_ok;
        desktop_status.worker_error = status.last_error.clone();

        self.update_desktop_runtime_status(&desktop_status)
    }

    fn set_worker_error(&mut self, message: &str) -> rusqlite::Result<()> {
        let mut desktop_status = self.desktop_runtime_status()?;

        desktop_status.worker_running = true;
        desktop_status.worker_health_ok = false;
        desktop_status.worker_error = Some(message.to_owned());

        self.update_desktop_runtime_status(&desktop_status)
    }

    fn clear_worker_error(&mut self) -> rusqlite::Result<()> {
        let mut desktop_status = self.desktop_runtime_status()?;

        desktop_status.worker_running = true;
        desktop_status.worker_health_ok = true;
        desktop_status.worker_error = None;

        self.update_desktop_runtime_status(&desktop_status)
    }

    fn replace_model_inventory(&mut self, models: &[ModelInventoryItem]) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;

        for model in models {
            transaction.execute(
                "
                INSERT INTO models (id, provider, name, installed, setup_required, dependency)
                VALUES (?1, 'faster-whisper', ?2, ?3, ?4, ?5)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    installed = excluded.installed,
                    setup_required = excluded.setup_required,
                    dependency = excluded.dependency
                ",
                params![
                    model.name,
                    model.name,
                    model.installed,
                    model.setup_required,
                    model.dependency,
                ],
            )?;
        }

        transaction.commit()
    }

    fn active_recording(&self) -> rusqlite::Result<Option<Recording>> {
        self.recording_by_status(RecordingStatus::Recording)
    }

    fn create_recording(&mut self, recording: NewRecording) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;

        transaction.execute(
            "
            INSERT INTO recordings (
                id,
                title,
                started_at,
                ended_at,
                duration_seconds,
                status,
                artifact_directory,
                capture_errors
            )
            VALUES (?1, ?2, ?3, NULL, NULL, ?4, ?5, '[]')
            ",
            params![
                recording.id,
                recording.title,
                recording.started_at,
                enum_value(RecordingStatus::Recording)?,
                recording.artifact_directory
            ],
        )?;

        for stage in recording_stages() {
            transaction.execute(
                "
                INSERT INTO pipeline_jobs (id, recording_id, stage, status, progress, message)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![
                    format!("{}-{}", recording.id, enum_value(stage.id)?),
                    recording.id,
                    enum_value(stage.id)?,
                    enum_value(stage.status)?,
                    stage.progress,
                    stage.label,
                ],
            )?;
        }

        transaction.commit()
    }

    fn finish_recording(
        &mut self,
        recording_id: &str,
        ended_at: String,
        duration_seconds: u64,
        capture_errors: Vec<CaptureError>,
        artifacts: &[Artifact],
    ) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;
        let capture_errors = json_string(&capture_errors)?;

        transaction.execute(
            "
            UPDATE recordings
            SET ended_at = ?1,
                duration_seconds = ?2,
                status = ?3,
                capture_errors = ?4
            WHERE id = ?5
            ",
            params![
                ended_at,
                duration_seconds,
                enum_value(RecordingStatus::Processing)?,
                capture_errors,
                recording_id,
            ],
        )?;

        for artifact in artifacts {
            transaction.execute(
                "
                INSERT INTO recording_artifacts (recording_id, kind, label, path, ready)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(recording_id, kind) DO UPDATE SET
                    label = excluded.label,
                    path = excluded.path,
                    ready = excluded.ready
                ",
                params![
                    recording_id,
                    enum_value(artifact.kind)?,
                    artifact.label,
                    artifact.path,
                    artifact.ready,
                ],
            )?;
        }

        let stage_updates = [
            (
                PipelineStageId::Recording,
                PipelineStageStatus::Complete,
                100,
            ),
            (
                PipelineStageId::Transcription,
                PipelineStageStatus::Pending,
                0,
            ),
            (PipelineStageId::Alignment, PipelineStageStatus::Pending, 0),
            (
                PipelineStageId::Diarization,
                PipelineStageStatus::Pending,
                0,
            ),
            (PipelineStageId::Summary, PipelineStageStatus::Pending, 0),
        ];

        for (stage, status, progress) in stage_updates {
            transaction.execute(
                "
                UPDATE pipeline_jobs
                SET status = ?1,
                    progress = ?2,
                    message = ?3
                WHERE recording_id = ?4
                    AND stage = ?5
                ",
                params![
                    enum_value(status)?,
                    progress,
                    stage_message(stage, status),
                    recording_id,
                    enum_value(stage)?,
                ],
            )?;
        }

        transaction.commit()
    }

    fn delete_recording(
        &mut self,
        recording_id: &str,
        delete_artifacts: bool,
    ) -> rusqlite::Result<()> {
        let recording = self.recording_by_id(recording_id)?.ok_or_else(|| {
            rusqlite::Error::InvalidParameterName("Recording not found".to_owned())
        })?;

        if recording.status == RecordingStatus::Recording {
            return Err(rusqlite::Error::InvalidParameterName(
                "Active recordings cannot be deleted".to_owned(),
            ));
        }

        self.connection.execute(
            "
            DELETE FROM recordings
            WHERE id = ?1
            ",
            params![recording_id],
        )?;

        if delete_artifacts {
            remove_artifact_directory(&recording.artifact_directory)?;
        }

        Ok(())
    }

    fn reset_retryable_jobs(&mut self, recording_id: &str) -> rusqlite::Result<()> {
        let recording = self.recording_by_id(recording_id)?.ok_or_else(|| {
            rusqlite::Error::InvalidParameterName("Recording not found".to_owned())
        })?;

        if recording.status != RecordingStatus::Processing {
            return Err(rusqlite::Error::InvalidParameterName(
                "Only processed recordings can retry jobs".to_owned(),
            ));
        }

        self.connection.execute(
            "
            UPDATE pipeline_jobs
            SET status = ?1,
                progress = 0,
                message = ?2
            WHERE recording_id = ?3
                AND status IN (?4, ?5, ?6)
                AND stage != ?7
            ",
            params![
                enum_value(PipelineStageStatus::Pending)?,
                "Retry queued",
                recording_id,
                enum_value(PipelineStageStatus::Failed)?,
                enum_value(PipelineStageStatus::NeedsSetup)?,
                enum_value(PipelineStageStatus::Running)?,
                enum_value(PipelineStageId::Recording)?,
            ],
        )?;

        self.append_event(
            recording_id,
            PipelineStageId::Transcription,
            PipelineStageStatus::Pending,
            "Retry queued",
        )
    }

    fn update_job(
        &mut self,
        job_id: &str,
        status: PipelineStageStatus,
        progress: u8,
        message: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            UPDATE pipeline_jobs
            SET status = ?1,
                progress = ?2,
                message = ?3
            WHERE id = ?4
            ",
            params![enum_value(status)?, progress, message, job_id],
        )?;

        Ok(())
    }

    fn job_for_recording_stage(
        &self,
        recording_id: &str,
        stage: PipelineStageId,
    ) -> rusqlite::Result<PipelineJob> {
        self.connection.query_row(
            "
            SELECT id, recording_id, stage, status, progress, message
            FROM pipeline_jobs
            WHERE recording_id = ?1
                AND stage = ?2
            ",
            params![recording_id, enum_value(stage)?],
            row_to_pipeline_job,
        )
    }

    fn upsert_artifact(&mut self, recording_id: &str, artifact: &Artifact) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            INSERT INTO recording_artifacts (recording_id, kind, label, path, ready)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(recording_id, kind) DO UPDATE SET
                label = excluded.label,
                path = excluded.path,
                ready = excluded.ready
            ",
            params![
                recording_id,
                enum_value(artifact.kind)?,
                artifact.label,
                artifact.path,
                artifact.ready,
            ],
        )?;

        Ok(())
    }

    fn update_recording_title(&mut self, recording_id: &str, title: &str) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            UPDATE recordings
            SET title = ?1
            WHERE id = ?2
            ",
            params![title, recording_id],
        )?;

        Ok(())
    }

    fn append_event(
        &mut self,
        recording_id: &str,
        stage: PipelineStageId,
        status: PipelineStageStatus,
        message: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            INSERT INTO job_events (recording_id, stage, status, message, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                recording_id,
                enum_value(stage)?,
                enum_value(status)?,
                message,
                unix_timestamp().to_string(),
            ],
        )?;

        Ok(())
    }

    fn snapshot(&self) -> rusqlite::Result<AppSnapshot> {
        let settings = self.settings()?;
        let recordings = self.recordings()?;
        let active_recording = self.active_recording()?;
        let jobs = self.jobs()?;
        let models = self.model_inventory()?;
        let capture_devices = capture_devices();
        let mut desktop = self.desktop_runtime_status()?;

        desktop.overlay_visible = active_recording.is_some();

        Ok(AppSnapshot {
            active_recording,
            recordings,
            jobs,
            models,
            capture_devices,
            desktop,
            settings,
        })
    }

    fn model_inventory(&self) -> rusqlite::Result<Vec<ModelInventoryItem>> {
        let mut statement = self.connection.prepare(
            "
            SELECT name, installed, setup_required, dependency
            FROM models
            ORDER BY name
            ",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ModelInventoryItem {
                name: row.get(0)?,
                installed: row.get(1)?,
                setup_required: row.get(2)?,
                dependency: row.get(3)?,
            })
        })?;
        let mut models = Vec::new();

        for model in rows {
            models.push(model?);
        }

        Ok(models)
    }

    fn desktop_runtime_status(&self) -> rusqlite::Result<DesktopRuntimeStatus> {
        Ok(DesktopRuntimeStatus {
            overlay_visible: parse_bool(&self.setting_value("overlayVisible", "false")?),
            hotkey_registered: parse_bool(&self.setting_value("hotkeyRegistered", "false")?),
            hotkey_error: empty_string_to_none(self.setting_value("hotkeyError", "")?),
            worker_running: parse_bool(&self.setting_value("workerRunning", "false")?),
            worker_health_ok: parse_bool(&self.setting_value("workerHealthOk", "false")?),
            worker_error: empty_string_to_none(self.setting_value("workerError", "")?),
        })
    }

    fn setting_value(&self, key: &str, fallback: &str) -> rusqlite::Result<String> {
        self.connection
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map(|value| value.unwrap_or_else(|| fallback.to_owned()))
    }

    fn recordings(&self) -> rusqlite::Result<Vec<Recording>> {
        let mut statement = self.connection.prepare(
            "
            SELECT id, title, started_at, ended_at, duration_seconds, status, artifact_directory, capture_errors
            FROM recordings
            WHERE status != ?1
            ORDER BY started_at DESC
            ",
        )?;
        let rows = statement
            .query_map(params![enum_value(RecordingStatus::Recording)?], |row| {
                self.row_to_recording(row)
            })?;
        let mut recordings = Vec::new();

        for recording in rows {
            recordings.push(recording?);
        }

        Ok(recordings)
    }

    fn jobs(&self) -> rusqlite::Result<Vec<PipelineJob>> {
        let mut statement = self.connection.prepare(
            "
            SELECT id, recording_id, stage, status, progress, message
            FROM pipeline_jobs
            ORDER BY recording_id DESC, stage
            ",
        )?;
        let rows = statement.query_map([], row_to_pipeline_job)?;
        let mut jobs = Vec::new();

        for job in rows {
            jobs.push(job?);
        }

        Ok(jobs)
    }

    fn recording_by_status(&self, status: RecordingStatus) -> rusqlite::Result<Option<Recording>> {
        self.connection
            .query_row(
                "
                SELECT id, title, started_at, ended_at, duration_seconds, status, artifact_directory, capture_errors
                FROM recordings
                WHERE status = ?1
                ORDER BY started_at DESC
                LIMIT 1
                ",
                params![enum_value(status)?],
                |row| self.row_to_recording(row),
            )
            .optional()
    }

    fn recording_by_id(&self, recording_id: &str) -> rusqlite::Result<Option<Recording>> {
        self.connection
            .query_row(
                "
                SELECT id, title, started_at, ended_at, duration_seconds, status, artifact_directory, capture_errors
                FROM recordings
                WHERE id = ?1
                ",
                params![recording_id],
                |row| self.row_to_recording(row),
            )
            .optional()
    }

    fn row_to_recording(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<Recording> {
        let id = row.get::<_, String>(0)?;
        let artifact_directory = row.get::<_, String>(6)?;
        let capture_errors = serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default();

        Ok(Recording {
            id: id.clone(),
            title: row.get(1)?,
            started_at: row.get(2)?,
            ended_at: row.get(3)?,
            duration_seconds: row.get(4)?,
            status: enum_from_value(&row.get::<_, String>(5)?)?,
            artifact_directory,
            capture_errors,
            stages: self.stages(&id)?,
            artifacts: self.artifacts(&id)?,
        })
    }

    fn stages(&self, recording_id: &str) -> rusqlite::Result<Vec<PipelineStage>> {
        let mut statement = self.connection.prepare(
            "
            SELECT stage, status, progress, message
            FROM pipeline_jobs
            WHERE recording_id = ?1
            ORDER BY stage
            ",
        )?;
        let rows = statement.query_map(params![recording_id], |row| {
            let id = enum_from_value::<PipelineStageId>(&row.get::<_, String>(0)?)?;

            Ok(PipelineStage {
                id,
                label: stage_label(id).to_owned(),
                status: enum_from_value(&row.get::<_, String>(1)?)?,
                progress: row.get(2)?,
                message: row.get(3)?,
            })
        })?;
        let mut stages = Vec::new();

        for stage in rows {
            stages.push(stage?);
        }

        Ok(stages)
    }

    fn artifacts(&self, recording_id: &str) -> rusqlite::Result<Vec<Artifact>> {
        let mut statement = self.connection.prepare(
            "
            SELECT kind, label, path, ready
            FROM recording_artifacts
            WHERE recording_id = ?1
            ORDER BY kind
            ",
        )?;
        let rows = statement.query_map(params![recording_id], |row| {
            Ok(Artifact {
                kind: enum_from_value(&row.get::<_, String>(0)?)?,
                label: row.get(1)?,
                path: row.get(2)?,
                ready: row.get::<_, bool>(3)?,
            })
        })?;
        let mut artifacts = Vec::new();

        for artifact in rows {
            artifacts.push(artifact?);
        }

        Ok(artifacts)
    }
}

trait AudioCaptureBackend {
    fn start(&mut self, recording_id: &str, settings: &AppSettings) -> Result<(), String>;

    fn stop(
        &mut self,
        recording_id: &str,
        artifact_directory: &Path,
    ) -> Result<CaptureResult, String>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct WorkerRuntimeState {
    running: bool,
    health_ok: bool,
    last_error: Option<String>,
}

impl WorkerRuntimeState {
    fn status(&self) -> WorkerStatus {
        WorkerStatus {
            running: self.running,
            health_ok: self.health_ok,
            last_error: self.last_error.clone(),
            mode: WorkerMode::CliJsonl,
        }
    }
}

fn run_worker_command(
    command_name: &str,
    payload: serde_json::Value,
) -> Result<Vec<WorkerEvent>, String> {
    let worker_directory = resolve_worker_directory()?;
    let command = serde_json::json!({
        "id": format!("rust-{}", unix_timestamp()),
        "name": command_name,
        "payload": payload,
    });
    let mut process = Command::new("uv")
        .arg("run")
        .arg("python")
        .arg("-m")
        .arg("app.main")
        .current_dir(&worker_directory)
        .env("PYTHONPATH", &worker_directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Unable to start Python worker: {error}"))?;

    match process.stdin.as_mut() {
        Some(stdin) => {
            writeln!(stdin, "{command}")
                .map_err(|error| format!("Unable to write worker command: {error}"))?;
        }
        None => return Err("Worker stdin is unavailable".to_owned()),
    }

    let output = process
        .wait_with_output()
        .map_err(|error| format!("Unable to read worker output: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }

    parse_worker_events(&String::from_utf8_lossy(&output.stdout))
}

fn parse_worker_events(output: &str) -> Result<Vec<WorkerEvent>, String> {
    let mut events = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        events.push(
            serde_json::from_str(line)
                .map_err(|error| format!("Unable to parse worker event: {error}"))?,
        );
    }

    Ok(events)
}

fn extract_model_inventory(events: &[WorkerEvent]) -> Result<Vec<ModelInventoryItem>, String> {
    for event in events {
        if event.event != "models.status" {
            continue;
        }

        return serde_json::from_value(
            event
                .payload
                .get("models")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
        )
        .map_err(|error| format!("Unable to parse model status: {error}"));
    }

    Err("Worker did not return model status".to_owned())
}

fn model_install_message(events: &[WorkerEvent]) -> String {
    for event in events {
        if event.event == "models.install.needs_setup" {
            let dependency = event
                .payload
                .get("dependency")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("faster-whisper");

            return format!("Model installation requires {dependency}");
        }

        if event.event == "command.failed" {
            return event
                .payload
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Model installation failed")
                .to_owned();
        }
    }

    "Model installation did not complete".to_owned()
}

fn resolve_worker_directory() -> Result<PathBuf, String> {
    let current_directory = env::current_dir()
        .map_err(|error| format!("Unable to resolve current directory: {error}"))?;
    let candidates = [
        current_directory.join("worker"),
        current_directory.join("..").join("worker"),
    ];

    for candidate in candidates {
        if candidate.join("app").join("main.py").exists() {
            return Ok(candidate);
        }
    }

    Err("Unable to find worker directory".to_owned())
}

#[cfg(test)]
#[derive(Debug, Default)]
struct FileAudioCaptureBackend {
    active_recordings: HashSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CaptureResult {
    artifacts: Vec<Artifact>,
    errors: Vec<CaptureError>,
}

#[cfg(test)]
impl AudioCaptureBackend for FileAudioCaptureBackend {
    fn start(&mut self, recording_id: &str, _settings: &AppSettings) -> Result<(), String> {
        self.active_recordings.insert(recording_id.to_owned());

        Ok(())
    }

    fn stop(
        &mut self,
        recording_id: &str,
        artifact_directory: &Path,
    ) -> Result<CaptureResult, String> {
        if !self.active_recordings.remove(recording_id) {
            return Err("Capture backend does not have an active session".to_owned());
        }

        fs::create_dir_all(artifact_directory).map_err(|error| error.to_string())?;
        write_test_wav_file(&artifact_directory.join("recording.wav"), 440)?;
        write_capture_metadata(
            artifact_directory,
            "file",
            &[
                CaptureFileMetadata::ready(CaptureSource::Microphone, "recording.wav"),
                CaptureFileMetadata::ready(CaptureSource::System, "recording.wav"),
            ],
        )?;
        fs::write(
            artifact_directory.join("job-log.jsonl"),
            "{\"stage\":\"recording\",\"status\":\"complete\",\"message\":\"capture stopped\"}\n",
        )
        .map_err(|error| error.to_string())?;

        Ok(CaptureResult {
            artifacts: capture_artifacts_with_readiness(artifact_directory, true),
            errors: Vec::new(),
        })
    }
}

#[derive(Default)]
struct NativeAudioCaptureBackend {
    active_recordings: HashMap<String, NativeCaptureSession>,
}

impl std::fmt::Debug for NativeAudioCaptureBackend {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeAudioCaptureBackend")
            .field("active_recordings", &self.active_recordings.keys())
            .finish()
    }
}

struct NativeCaptureSession {
    microphone: Option<CapturedSource>,
    system: Option<CapturedSource>,
    errors: Vec<CaptureError>,
}

struct CapturedSource {
    source: CaptureSource,
    _stream: cpal::Stream,
    samples: Arc<Mutex<Vec<i16>>>,
    stream_errors: Arc<Mutex<Vec<String>>>,
    sample_rate: u32,
    channels: u16,
}

struct CaptureDevice {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureFileMetadata {
    source: CaptureSource,
    path: String,
    ready: bool,
    sample_rate: Option<u32>,
    channels: Option<u16>,
    frames: usize,
}

impl CaptureFileMetadata {
    #[cfg(test)]
    fn ready(source: CaptureSource, path: &str) -> Self {
        Self {
            source,
            path: path.to_owned(),
            ready: true,
            sample_rate: None,
            channels: None,
            frames: 0,
        }
    }
}

impl AudioCaptureBackend for NativeAudioCaptureBackend {
    fn start(&mut self, recording_id: &str, settings: &AppSettings) -> Result<(), String> {
        if self.active_recordings.contains_key(recording_id) {
            return Err("Capture backend already has an active session".to_owned());
        }

        let host = cpal::default_host();
        let mut errors = Vec::new();
        let microphone = match start_native_source(
            &host,
            CaptureSource::Microphone,
            &settings.microphone_device,
        ) {
            Ok(source) => Some(source),
            Err(error) => {
                errors.push(CaptureError {
                    source: CaptureSource::Microphone,
                    message: error,
                });
                None
            }
        };
        let system = match start_native_source(
            &host,
            CaptureSource::System,
            &settings.system_audio_source,
        ) {
            Ok(source) => Some(source),
            Err(error) => {
                errors.push(CaptureError {
                    source: CaptureSource::System,
                    message: error,
                });
                None
            }
        };

        if microphone.is_none() && system.is_none() {
            return Err(capture_errors_message(&errors));
        }

        self.active_recordings.insert(
            recording_id.to_owned(),
            NativeCaptureSession {
                microphone,
                system,
                errors,
            },
        );

        Ok(())
    }

    fn stop(
        &mut self,
        recording_id: &str,
        artifact_directory: &Path,
    ) -> Result<CaptureResult, String> {
        let session = self
            .active_recordings
            .remove(recording_id)
            .ok_or_else(|| "Capture backend does not have an active session".to_owned())?;

        fs::create_dir_all(artifact_directory).map_err(|error| error.to_string())?;

        let mut errors = session.errors;
        let microphone = finalize_native_source(&session.microphone)?;
        let system = finalize_native_source(&session.system)?;

        append_stream_errors(&mut errors, &session.microphone);
        append_stream_errors(&mut errors, &session.system);
        write_mixed_recording(
            artifact_directory,
            microphone.as_ref(),
            system.as_ref(),
            "recording.wav",
        )?;
        write_job_log(artifact_directory, &errors)?;
        write_capture_metadata(
            artifact_directory,
            "native-cpal",
            &[
                metadata_for_source(
                    CaptureSource::Microphone,
                    "recording.wav",
                    microphone.as_ref(),
                ),
                metadata_for_source(CaptureSource::System, "recording.wav", system.as_ref()),
            ],
        )?;

        Ok(CaptureResult {
            artifacts: capture_artifacts_with_readiness(
                artifact_directory,
                microphone.is_some() || system.is_some(),
            ),
            errors,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FinalizedSource {
    samples: Vec<i16>,
    sample_rate: u32,
    channels: u16,
    frames: usize,
}

fn start_native_source(
    host: &cpal::Host,
    source: CaptureSource,
    configured_name: &str,
) -> Result<CapturedSource, String> {
    let capture_device = resolve_capture_device(host, source, configured_name)?;
    let device = capture_device.device;
    let config = capture_device.config;
    let stream_config = config.config();
    let sample_rate = stream_config.sample_rate;
    let channels = stream_config.channels;
    let samples = Arc::new(Mutex::new(Vec::<i16>::new()));
    let stream_errors = Arc::new(Mutex::new(Vec::<String>::new()));
    let stream = build_input_stream(
        &device,
        config.sample_format(),
        stream_config,
        &samples,
        &stream_errors,
    )?;

    stream
        .play()
        .map_err(|error| format!("Unable to start input stream: {error}"))?;

    Ok(CapturedSource {
        source,
        _stream: stream,
        samples,
        stream_errors,
        sample_rate,
        channels,
    })
}

fn build_input_stream(
    device: &cpal::Device,
    sample_format: cpal::SampleFormat,
    stream_config: cpal::StreamConfig,
    samples: &Arc<Mutex<Vec<i16>>>,
    stream_errors: &Arc<Mutex<Vec<String>>>,
) -> Result<cpal::Stream, String> {
    match sample_format {
        cpal::SampleFormat::F32 => {
            build_typed_input_stream::<f32>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::F64 => {
            build_typed_input_stream::<f64>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::I8 => {
            build_typed_input_stream::<i8>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::I16 => {
            build_typed_input_stream::<i16>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::I32 => {
            build_typed_input_stream::<i32>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::I64 => {
            build_typed_input_stream::<i64>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::U8 => {
            build_typed_input_stream::<u8>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::U16 => {
            build_typed_input_stream::<u16>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::U32 => {
            build_typed_input_stream::<u32>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::U64 => {
            build_typed_input_stream::<u64>(device, stream_config, samples, stream_errors)
        }
        format => Err(format!("Unsupported input sample format: {format:?}")),
    }
}

fn build_typed_input_stream<T>(
    device: &cpal::Device,
    stream_config: cpal::StreamConfig,
    samples: &Arc<Mutex<Vec<i16>>>,
    stream_errors: &Arc<Mutex<Vec<String>>>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + ToI16Sample,
{
    let samples = Arc::clone(samples);
    let stream_errors = Arc::clone(stream_errors);

    device
        .build_input_stream(
            stream_config,
            move |data: &[T], _info| {
                if let Ok(mut captured_samples) = samples.lock() {
                    for sample in data {
                        captured_samples.push(sample.to_i16_sample());
                    }
                }
            },
            move |error| {
                if let Ok(mut errors) = stream_errors.lock() {
                    errors.push(error.to_string());
                }
            },
            None,
        )
        .map_err(|error| format!("Unable to build input stream: {error}"))
}

trait ToI16Sample {
    fn to_i16_sample(&self) -> i16;
}

impl ToI16Sample for f32 {
    fn to_i16_sample(&self) -> i16 {
        (self.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
    }
}

impl ToI16Sample for f64 {
    fn to_i16_sample(&self) -> i16 {
        (self.clamp(-1.0, 1.0) * i16::MAX as f64) as i16
    }
}

impl ToI16Sample for i8 {
    fn to_i16_sample(&self) -> i16 {
        (*self as i16) << 8
    }
}

impl ToI16Sample for i16 {
    fn to_i16_sample(&self) -> i16 {
        *self
    }
}

impl ToI16Sample for i32 {
    fn to_i16_sample(&self) -> i16 {
        (*self >> 16) as i16
    }
}

impl ToI16Sample for i64 {
    fn to_i16_sample(&self) -> i16 {
        (*self >> 48) as i16
    }
}

impl ToI16Sample for u8 {
    fn to_i16_sample(&self) -> i16 {
        ((*self as i16) - 128) << 8
    }
}

impl ToI16Sample for u16 {
    fn to_i16_sample(&self) -> i16 {
        (*self as i32 - 32_768) as i16
    }
}

impl ToI16Sample for u32 {
    fn to_i16_sample(&self) -> i16 {
        ((*self >> 16) as i32 - 32_768) as i16
    }
}

impl ToI16Sample for u64 {
    fn to_i16_sample(&self) -> i16 {
        ((*self >> 48) as i32 - 32_768) as i16
    }
}

fn resolve_capture_device(
    host: &cpal::Host,
    source: CaptureSource,
    configured_name: &str,
) -> Result<CaptureDevice, String> {
    let device = match source {
        CaptureSource::Microphone => resolve_microphone_device(host, configured_name)?,
        CaptureSource::System => resolve_system_device(host, configured_name)?,
    };
    let config = capture_config_for_device(&device, source)?;

    Ok(CaptureDevice { device, config })
}

fn resolve_microphone_device(
    host: &cpal::Host,
    configured_name: &str,
) -> Result<cpal::Device, String> {
    let configured_name = configured_name.trim();

    if is_default_microphone_name(configured_name) {
        return host
            .default_input_device()
            .ok_or_else(|| "No default microphone input device is available".to_owned());
    }

    resolve_named_device(host, configured_name, DeviceSearchMode::Input)
}

fn resolve_system_device(host: &cpal::Host, configured_name: &str) -> Result<cpal::Device, String> {
    let configured_name = configured_name.trim();

    if is_default_system_source_name(configured_name) {
        return resolve_default_system_device(host);
    }

    resolve_named_device(host, configured_name, DeviceSearchMode::System)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn resolve_default_system_device(host: &cpal::Host) -> Result<cpal::Device, String> {
    host.default_output_device().ok_or_else(|| {
        "No default output device is available for native system audio capture".to_owned()
    })
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn resolve_default_system_device(host: &cpal::Host) -> Result<cpal::Device, String> {
    for device in host
        .input_devices()
        .map_err(|error| format!("Unable to list input devices: {error}"))?
    {
        if is_system_monitor_device_name(&device.to_string()) {
            return Ok(device);
        }
    }

    Err("System audio capture on Linux requires a PipeWire/PulseAudio monitor or loopback input device in Capture settings".to_owned())
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd"
)))]
fn resolve_default_system_device(_host: &cpal::Host) -> Result<cpal::Device, String> {
    Err("Native system audio capture is not supported on this platform".to_owned())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeviceSearchMode {
    Input,
    System,
}

fn resolve_named_device(
    host: &cpal::Host,
    configured_name: &str,
    mode: DeviceSearchMode,
) -> Result<cpal::Device, String> {
    let configured_name_lower = configured_name.to_lowercase();

    for device in host
        .input_devices()
        .map_err(|error| format!("Unable to list input devices: {error}"))?
    {
        let device_name = device.to_string();

        if device_name.to_lowercase().contains(&configured_name_lower)
            || (mode == DeviceSearchMode::System
                && is_monitor_search_name(&configured_name_lower)
                && is_system_monitor_device_name(&device_name))
        {
            return Ok(device);
        }
    }

    if mode == DeviceSearchMode::System {
        for device in host
            .output_devices()
            .map_err(|error| format!("Unable to list output devices: {error}"))?
        {
            if device
                .to_string()
                .to_lowercase()
                .contains(&configured_name_lower)
            {
                return Ok(device);
            }
        }
    }

    Err(format!("Audio device not found: {configured_name}"))
}

fn capture_config_for_device(
    device: &cpal::Device,
    source: CaptureSource,
) -> Result<cpal::SupportedStreamConfig, String> {
    if device.supports_input() {
        return device
            .default_input_config()
            .map_err(|error| format!("Unable to read input config: {error}"));
    }

    if source == CaptureSource::System && device.supports_output() {
        return device
            .default_output_config()
            .map_err(|error| format!("Unable to read output loopback config: {error}"));
    }

    Err(format!("Audio device cannot capture {source:?} audio"))
}

fn is_default_microphone_name(name: &str) -> bool {
    name.is_empty() || name == "Default microphone"
}

fn is_default_system_source_name(name: &str) -> bool {
    name.is_empty() || name == "Default system output"
}

fn is_system_monitor_device_name(name: &str) -> bool {
    let normalized = name.to_lowercase();

    normalized.contains("monitor")
        || normalized.contains("loopback")
        || normalized.contains("what u hear")
        || normalized.contains("stereo mix")
}

fn is_monitor_search_name(name: &str) -> bool {
    name == "monitor" || name == "loopback" || name == "stereo mix"
}

fn capture_devices() -> CaptureDevices {
    let host = cpal::default_host();

    CaptureDevices {
        microphones: microphone_devices(&host),
        system_sources: system_source_devices(&host),
    }
}

fn microphone_devices(host: &cpal::Host) -> Vec<CaptureDeviceInfo> {
    let mut devices = vec![CaptureDeviceInfo {
        name: "Default microphone".to_owned(),
        label: "Default microphone".to_owned(),
        default: true,
    }];

    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            devices.push(capture_device_info(device.to_string(), false));
        }
    }

    dedupe_capture_devices(devices)
}

fn system_source_devices(host: &cpal::Host) -> Vec<CaptureDeviceInfo> {
    let mut devices = vec![CaptureDeviceInfo {
        name: "Default system output".to_owned(),
        label: "Default system output".to_owned(),
        default: true,
    }];

    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            let name = device.to_string();

            if is_system_monitor_device_name(&name) {
                devices.push(capture_device_info(name, false));
            }
        }
    }

    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            devices.push(capture_device_info(device.to_string(), false));
        }
    }

    dedupe_capture_devices(devices)
}

fn capture_device_info(name: String, default: bool) -> CaptureDeviceInfo {
    CaptureDeviceInfo {
        label: name.clone(),
        name,
        default,
    }
}

fn dedupe_capture_devices(devices: Vec<CaptureDeviceInfo>) -> Vec<CaptureDeviceInfo> {
    let mut unique_devices = Vec::new();

    for device in devices {
        if unique_devices
            .iter()
            .any(|existing: &CaptureDeviceInfo| existing.name == device.name)
        {
            continue;
        }

        unique_devices.push(device);
    }

    unique_devices
}

fn finalize_native_source(
    source: &Option<CapturedSource>,
) -> Result<Option<FinalizedSource>, String> {
    let Some(source) = source else {
        return Ok(None);
    };
    let samples = match source.samples.lock() {
        Ok(samples) => samples.clone(),
        Err(error) => return Err(error.to_string()),
    };

    if samples.is_empty() {
        return Ok(None);
    }

    let frames = samples.len() / source.channels.max(1) as usize;

    Ok(Some(FinalizedSource {
        samples,
        sample_rate: source.sample_rate,
        channels: source.channels,
        frames,
    }))
}

fn append_stream_errors(errors: &mut Vec<CaptureError>, source: &Option<CapturedSource>) {
    let Some(source) = source else {
        return;
    };
    let Ok(stream_errors) = source.stream_errors.lock() else {
        return;
    };

    for message in stream_errors.iter() {
        errors.push(CaptureError {
            source: source.source,
            message: message.clone(),
        });
    }
}

fn write_mixed_recording(
    artifact_directory: &Path,
    microphone: Option<&FinalizedSource>,
    system: Option<&FinalizedSource>,
    file_name: &str,
) -> Result<(), String> {
    let Some(primary) = microphone.or(system) else {
        return Err("No captured audio source is available for mixed recording".to_owned());
    };
    let sample_rate = primary.sample_rate;
    let channels = primary.channels;
    let mut mixed_samples = primary.samples.clone();

    if let Some(secondary) = system {
        if microphone.is_some()
            && secondary.channels == channels
            && secondary.sample_rate == sample_rate
        {
            for (index, sample) in secondary.samples.iter().enumerate() {
                if let Some(target) = mixed_samples.get_mut(index) {
                    let mixed = *target as i32 + *sample as i32;
                    *target = mixed.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                }
            }
        }
    }

    write_pcm_wav_file(
        &artifact_directory.join(file_name),
        &FinalizedSource {
            frames: mixed_samples.len() / channels.max(1) as usize,
            samples: mixed_samples,
            sample_rate,
            channels,
        },
    )
}

fn write_pcm_wav_file(path: &Path, source: &FinalizedSource) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: source.channels,
        sample_rate: source.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|error| error.to_string())?;

    for sample in &source.samples {
        writer
            .write_sample(*sample)
            .map_err(|error| error.to_string())?;
    }

    writer.finalize().map_err(|error| error.to_string())
}

fn write_job_log(path: &Path, errors: &[CaptureError]) -> Result<(), String> {
    let mut lines = Vec::new();

    lines.push(serde_json::json!({
        "stage": "recording",
        "status": "complete",
        "message": "capture stopped",
    }));

    for error in errors {
        lines.push(serde_json::json!({
            "stage": "recording",
            "status": "failed",
            "source": error.source,
            "message": error.message,
        }));
    }

    let content = lines
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(path.join("job-log.jsonl"), format!("{content}\n")).map_err(|error| error.to_string())
}

fn write_capture_metadata(
    artifact_directory: &Path,
    backend: &str,
    sources: &[CaptureFileMetadata],
) -> Result<(), String> {
    let metadata = serde_json::json!({
        "backend": backend,
        "sources": sources,
    });

    fs::write(
        artifact_directory.join("metadata.json"),
        serde_json::to_string_pretty(&metadata).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn metadata_for_source(
    source: CaptureSource,
    path: &str,
    finalized_source: Option<&FinalizedSource>,
) -> CaptureFileMetadata {
    match finalized_source {
        Some(finalized_source) => CaptureFileMetadata {
            source,
            path: path.to_owned(),
            ready: true,
            sample_rate: Some(finalized_source.sample_rate),
            channels: Some(finalized_source.channels),
            frames: finalized_source.frames,
        },
        None => CaptureFileMetadata {
            source,
            path: path.to_owned(),
            ready: false,
            sample_rate: None,
            channels: None,
            frames: 0,
        },
    }
}

fn capture_errors_message(errors: &[CaptureError]) -> String {
    errors
        .iter()
        .map(|error| format!("{:?}: {}", error.source, error.message))
        .collect::<Vec<_>>()
        .join("; ")
}

const DEFAULT_TITLE_PROMPT: &str = "Create a concise meeting title from the transcript.";
const DEFAULT_SUMMARY_PROMPT: &str =
    "Summarize decisions, action items, risks, and unanswered questions.";
const KEYCHAIN_SERVICE: &str = "com.actavoces.desktop";
const SUMMARY_PROVIDER_API_KEY_ACCOUNT: &str = "summary-provider-api-key";

fn default_settings(database_path: &Path) -> AppSettings {
    AppSettings {
        output_directory: default_records_root(),
        database_path: database_path.display().to_string(),
        hotkey: "CommandOrControl+Shift+Space".to_owned(),
        overlay_position: OverlayPosition::TopLeft,
        launch_at_login: false,
        microphone_device: "Default microphone".to_owned(),
        system_audio_source: "Default system output".to_owned(),
        sample_rate: 48_000,
        whisper_model: "medium.en".to_owned(),
        transcription_language: "auto".to_owned(),
        compute_type: "auto".to_owned(),
        model_storage_directory: default_model_storage_root(),
        diarization_backend: DiarizationBackend::NemoWhisper,
        speaker_count_mode: SpeakerCountMode::Automatic,
        exact_speakers: None,
        min_speakers: None,
        max_speakers: None,
        summary_provider_configured: false,
        provider_api_key_configured: false,
        summary_enabled: false,
        provider_base_url: "https://api.openai.com/v1".to_owned(),
        provider_model: String::new(),
        title_prompt: DEFAULT_TITLE_PROMPT.to_owned(),
        summary_prompt: DEFAULT_SUMMARY_PROMPT.to_owned(),
    }
}

fn default_model_inventory() -> Vec<ModelInventoryItem> {
    ["small.en", "medium.en", "large-v3", "distil-large-v3"]
        .iter()
        .map(|model| ModelInventoryItem {
            name: (*model).to_owned(),
            installed: false,
            setup_required: true,
            dependency: "faster-whisper".to_owned(),
        })
        .collect()
}

fn settings_pairs(
    input: &AppSettingsUpdate,
    summary_provider_configured: bool,
    provider_api_key_configured: bool,
) -> Vec<(&'static str, String)> {
    vec![
        ("outputDirectory", input.output_directory.clone()),
        ("hotkey", input.hotkey.clone()),
        (
            "overlayPosition",
            serde_json::to_string(&input.overlay_position).unwrap_or_default(),
        ),
        ("launchAtLogin", input.launch_at_login.to_string()),
        ("microphoneDevice", input.microphone_device.clone()),
        ("systemAudioSource", input.system_audio_source.clone()),
        ("sampleRate", input.sample_rate.to_string()),
        ("whisperModel", input.whisper_model.clone()),
        (
            "transcriptionLanguage",
            input.transcription_language.clone(),
        ),
        ("computeType", input.compute_type.clone()),
        (
            "modelStorageDirectory",
            input.model_storage_directory.clone(),
        ),
        (
            "diarizationBackend",
            serde_json::to_string(&input.diarization_backend).unwrap_or_default(),
        ),
        (
            "speakerCountMode",
            serde_json::to_string(&input.speaker_count_mode).unwrap_or_default(),
        ),
        (
            "exactSpeakers",
            option_number_to_string(input.exact_speakers),
        ),
        ("minSpeakers", option_number_to_string(input.min_speakers)),
        ("maxSpeakers", option_number_to_string(input.max_speakers)),
        (
            "summaryProviderConfigured",
            summary_provider_configured.to_string(),
        ),
        (
            "providerApiKeyConfigured",
            provider_api_key_configured.to_string(),
        ),
        ("summaryEnabled", input.summary_enabled.to_string()),
        ("providerBaseUrl", input.provider_base_url.clone()),
        ("providerModel", input.provider_model.clone()),
        ("titlePrompt", input.title_prompt.clone()),
        ("summaryPrompt", input.summary_prompt.clone()),
    ]
}

fn validate_settings(
    input: &AppSettingsUpdate,
    provider_api_key_configured: bool,
) -> rusqlite::Result<()> {
    if input.output_directory.trim().is_empty() {
        return Err(rusqlite::Error::InvalidParameterName(
            "Output directory is required".to_owned(),
        ));
    }

    if input.hotkey.trim().is_empty() {
        return Err(rusqlite::Error::InvalidParameterName(
            "Hotkey is required".to_owned(),
        ));
    }

    if input.sample_rate == 0 {
        return Err(rusqlite::Error::InvalidParameterName(
            "Sample rate must be greater than zero".to_owned(),
        ));
    }

    if input.summary_enabled {
        if input.provider_base_url.trim().is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Provider base URL is required when summaries are enabled".to_owned(),
            ));
        }

        if input.provider_model.trim().is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Provider model is required when summaries are enabled".to_owned(),
            ));
        }

        if !provider_api_key_configured {
            return Err(rusqlite::Error::InvalidParameterName(
                "Provider API key is required when summaries are enabled".to_owned(),
            ));
        }
    }

    match input.speaker_count_mode {
        SpeakerCountMode::Automatic => Ok(()),
        SpeakerCountMode::Exact => match input.exact_speakers {
            Some(value) if value > 0 => Ok(()),
            _ => Err(rusqlite::Error::InvalidParameterName(
                "Exact speaker count must be greater than zero".to_owned(),
            )),
        },
        SpeakerCountMode::Range => match (input.min_speakers, input.max_speakers) {
            (Some(min), Some(max)) if min > 0 && max >= min => Ok(()),
            _ => Err(rusqlite::Error::InvalidParameterName(
                "Speaker range must include a valid minimum and maximum".to_owned(),
            )),
        },
    }
}

fn summary_provider_configured_for(
    summary_enabled: bool,
    provider_base_url: &str,
    provider_model: &str,
    provider_api_key_configured: bool,
) -> bool {
    summary_enabled
        && !provider_base_url.trim().is_empty()
        && !provider_model.trim().is_empty()
        && provider_api_key_configured
}

fn update_summary_provider_api_key(input: &AppSettingsUpdate) -> Result<bool, String> {
    let provider_api_key = input.provider_api_key.as_deref().unwrap_or_default().trim();

    if provider_api_key.is_empty() {
        return Ok(summary_provider_api_key_configured());
    }

    summary_provider_entry()?
        .set_password(provider_api_key)
        .map_err(|error| format!("Unable to store provider API key: {error}"))?;

    Ok(true)
}

fn clear_summary_provider_secret() -> Result<(), String> {
    let entry = summary_provider_entry()?;

    match entry.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(format!("Unable to clear provider API key: {error}")),
    }
}

fn summary_provider_api_key_configured() -> bool {
    match read_summary_provider_api_key() {
        Ok(Some(api_key)) => !api_key.trim().is_empty(),
        Ok(None) | Err(_) => false,
    }
}

fn read_summary_provider_api_key() -> Result<Option<String>, String> {
    let entry = summary_provider_entry()?;

    match entry.get_password() {
        Ok(api_key) => Ok(Some(api_key)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(format!("Unable to read provider API key: {error}")),
    }
}

fn summary_provider_entry() -> Result<Entry, String> {
    keyring::use_native_store(false)
        .map_err(|error| format!("Unable to access the native keychain: {error}"))?;
    Entry::new(KEYCHAIN_SERVICE, SUMMARY_PROVIDER_API_KEY_ACCOUNT)
        .map_err(|error| format!("Unable to open provider API key entry: {error}"))
}

fn recording_stages() -> Vec<PipelineStage> {
    vec![
        stage(PipelineStageId::Recording, PipelineStageStatus::Running, 10),
        stage(
            PipelineStageId::Transcription,
            PipelineStageStatus::Pending,
            0,
        ),
        stage(PipelineStageId::Alignment, PipelineStageStatus::Pending, 0),
        stage(
            PipelineStageId::Diarization,
            PipelineStageStatus::Pending,
            0,
        ),
        stage(PipelineStageId::Summary, PipelineStageStatus::Pending, 0),
    ]
}

fn stage(id: PipelineStageId, status: PipelineStageStatus, progress: u8) -> PipelineStage {
    PipelineStage {
        id,
        label: stage_label(id).to_owned(),
        status,
        progress,
        message: stage_message(id, status).to_owned(),
    }
}

fn stage_label(stage: PipelineStageId) -> &'static str {
    match stage {
        PipelineStageId::Recording => "Capture",
        PipelineStageId::Transcription => "Raw transcript",
        PipelineStageId::Alignment => "Alignment",
        PipelineStageId::Diarization => "Diarization",
        PipelineStageId::Summary => "Summary",
    }
}

fn stage_message(stage: PipelineStageId, status: PipelineStageStatus) -> &'static str {
    match (stage, status) {
        (PipelineStageId::Recording, PipelineStageStatus::Complete) => "Audio capture complete",
        (PipelineStageId::Transcription, PipelineStageStatus::NeedsSetup) => {
            "Local transcription setup required"
        }
        _ => "Waiting for worker",
    }
}

fn capture_artifacts_with_readiness(path: &Path, mixed_ready: bool) -> Vec<Artifact> {
    vec![
        artifact(
            ArtifactKind::Audio,
            "Mixed WAV",
            path.join("recording.wav"),
            mixed_ready,
        ),
        artifact(
            ArtifactKind::RawTranscript,
            "Raw transcript",
            path.join("raw-transcript.md"),
            false,
        ),
        artifact(
            ArtifactKind::Segments,
            "Raw segments",
            path.join("raw-segments.json"),
            false,
        ),
        artifact(
            ArtifactKind::Diarization,
            "Diarization turns",
            path.join("diarization.json"),
            false,
        ),
        artifact(
            ArtifactKind::DiarizedTranscript,
            "Diarized transcript",
            path.join("diarized-transcript.md"),
            false,
        ),
        artifact(
            ArtifactKind::Summary,
            "Summary",
            path.join("summary.md"),
            false,
        ),
        artifact(
            ArtifactKind::Metadata,
            "Metadata",
            path.join("metadata.json"),
            true,
        ),
        artifact(
            ArtifactKind::JobLog,
            "Job log",
            path.join("job-log.jsonl"),
            true,
        ),
    ]
}

fn artifact(kind: ArtifactKind, label: &str, path: PathBuf, ready: bool) -> Artifact {
    Artifact {
        kind,
        label: label.to_owned(),
        path: path.display().to_string(),
        ready,
    }
}

fn artifact_directory(output_directory: &str, started_at: u64, title: &str) -> PathBuf {
    let date = civil_datetime(started_at);
    let slug = slugify(title);

    PathBuf::from(output_directory)
        .join(format!("{:04}", date.year))
        .join(format!("{:02}", date.month))
        .join(format!(
            "{:04}-{:02}-{:02}-{:02}{:02}{:02}-{slug}",
            date.year, date.month, date.day, date.hour, date.minute, date.second,
        ))
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            continue;
        }

        if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    slug.trim_matches('-').to_owned()
}

#[cfg(test)]
fn write_test_wav_file(path: &Path, tone_hz: u32) -> Result<(), String> {
    let sample_rate = 8_000u32;
    let duration_samples = sample_rate / 5;
    let data_bytes = duration_samples * 2;
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());

    for index in 0..duration_samples {
        let phase = (index * tone_hz) % sample_rate;
        let sample = match phase < sample_rate / 2 {
            true => 2_000i16,
            false => -2_000i16,
        };
        bytes.extend_from_slice(&sample.to_le_bytes());
    }

    fs::write(path, bytes).map_err(|error| error.to_string())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CivilDateTime {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

fn civil_datetime(timestamp: u64) -> CivilDateTime {
    let days = (timestamp / 86_400) as i64;
    let seconds_of_day = (timestamp % 86_400) as u32;
    let (year, month, day) = civil_from_days(days);

    CivilDateTime {
        year,
        month,
        day,
        hour: seconds_of_day / 3_600,
        minute: (seconds_of_day % 3_600) / 60,
        second: seconds_of_day % 60,
    }
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let shifted_days = days + 719_468;
    let era = match shifted_days >= 0 {
        true => shifted_days,
        false => shifted_days - 146_096,
    } / 146_097;
    let day_of_era = shifted_days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = (year_of_era + era * 400) as i32;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_parameter = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_parameter + 2) / 5 + 1;
    let month = month_parameter
        + match month_parameter < 10 {
            true => 3,
            false => -9,
        };

    if month <= 2 {
        year += 1;
    }

    (year, month as u32, day as u32)
}

fn default_records_root() -> String {
    home_directory()
        .join("actavoces")
        .join("records")
        .display()
        .to_string()
}

fn default_model_storage_root() -> String {
    home_directory()
        .join("actavoces")
        .join("models")
        .display()
        .to_string()
}

fn ensure_configured_storage_directories(
    output_directory: &str,
    model_storage_directory: &str,
) -> rusqlite::Result<()> {
    ensure_directory(output_directory)?;
    ensure_directory(model_storage_directory)
}

fn ensure_directory(path: &str) -> rusqlite::Result<()> {
    fs::create_dir_all(path)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn remove_artifact_directory(path: &str) -> rusqlite::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(rusqlite::Error::ToSqlConversionFailure(Box::new(error))),
    }
}

fn home_directory() -> PathBuf {
    match env::var_os("HOME") {
        Some(home) => PathBuf::from(home),
        None => match env::var_os("USERPROFILE") {
            Some(home) => PathBuf::from(home),
            None => PathBuf::from("."),
        },
    }
}

fn option_number_to_string(value: Option<u8>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => String::new(),
    }
}

fn parse_optional_number(value: &str) -> Option<u8> {
    if value.trim().is_empty() {
        return None;
    }

    value.parse().ok()
}

fn parse_bool(value: &str) -> bool {
    value == "true"
}

fn empty_string_to_none(value: String) -> Option<String> {
    match value.is_empty() {
        true => None,
        false => Some(value),
    }
}

fn json_string<T>(value: &T) -> rusqlite::Result<String>
where
    T: Serialize,
{
    serde_json::to_string(value)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn enum_value<T>(value: T) -> rusqlite::Result<String>
where
    T: Serialize,
{
    serde_json::to_value(value)
        .and_then(|value| match value.as_str() {
            Some(value) => Ok(value.to_owned()),
            None => Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "enum did not serialize as a string",
            ))),
        })
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn enum_from_value<T>(value: &str) -> rusqlite::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(&format!("\"{value}\"")).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn row_to_pipeline_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<PipelineJob> {
    Ok(PipelineJob {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        stage: enum_from_value(&row.get::<_, String>(2)?)?,
        status: enum_from_value(&row.get::<_, String>(3)?)?,
        progress: row.get(4)?,
        message: row.get(5)?,
    })
}

fn pipeline_job_id(recording_id: &str, stage: PipelineStageId) -> Result<String, String> {
    let stage = enum_value(stage).map_err(|error| error.to_string())?;

    Ok(format!("{recording_id}-{stage}"))
}

fn read_json_file(path: PathBuf) -> Option<serde_json::Value> {
    let content = fs::read_to_string(path).ok()?;

    serde_json::from_str(&content).ok()
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;

    use crate::{
        artifact_directory, capture_artifacts_with_readiness, default_records_root,
        default_settings, is_default_system_source_name, is_system_monitor_device_name,
        AppRepository, AppSettingsUpdate, ArtifactKind, DesktopRuntimeStatus, DiarizationBackend,
        FileAudioCaptureBackend, ModelInventoryItem, NewRecording, OverlayPosition,
        PipelineStageStatus, RecordingStatus, SpeakerCountMode,
    };
    use crate::{
        extract_model_inventory, parse_worker_events, resume_pipeline_jobs,
        start_recording_session, stop_recording_session, AudioCaptureBackend, PipelineStageId,
        WorkerEvent, WorkerRuntimeState,
    };

    #[test]
    fn default_records_root_uses_home_actavoces_records() {
        let root = default_records_root();

        assert!(root.ends_with("actavoces/records"));
    }

    #[test]
    fn artifact_directory_uses_date_layout_and_stable_slug() {
        let path = artifact_directory("/tmp/records", 1_717_938_012, "Untitled meeting");

        assert_eq!(
            path.display().to_string(),
            "/tmp/records/2024/06/2024-06-09-130012-untitled-meeting"
        );
    }

    #[test]
    fn repository_restores_recordings_after_reopen() {
        let database_path = test_database_path("restore");
        let artifact_path = test_artifact_path("restore-recording");
        let mut repository = AppRepository::open(&database_path).unwrap();
        let mut capture_backend = FileAudioCaptureBackend::default();
        let recording = NewRecording {
            id: "recording-1".to_owned(),
            title: "Untitled meeting".to_owned(),
            started_at: "1".to_owned(),
            artifact_directory: artifact_path.display().to_string(),
        };

        capture_backend
            .start(&recording.id, &repository.settings().unwrap())
            .unwrap();
        repository.create_recording(recording.clone()).unwrap();
        let capture_result = capture_backend.stop(&recording.id, &artifact_path).unwrap();
        repository
            .finish_recording(
                &recording.id,
                "2".to_owned(),
                1,
                capture_result.errors,
                &capture_result.artifacts,
            )
            .unwrap();
        drop(repository);

        let restored = AppRepository::open(&database_path)
            .unwrap()
            .snapshot()
            .unwrap();

        assert_eq!(restored.recordings.len(), 1);
        assert_eq!(restored.recordings[0].status, RecordingStatus::Processing);
        assert!(restored.recordings[0]
            .artifacts
            .iter()
            .any(|artifact| artifact.path.ends_with("recording.wav") && artifact.ready));
    }

    #[test]
    fn repository_seeds_and_updates_model_inventory() {
        let database_path = test_database_path("models");
        let mut repository = AppRepository::open(&database_path).unwrap();
        let initial_models = repository.model_inventory().unwrap();

        assert!(initial_models
            .iter()
            .any(|model| model.name == "medium.en" && model.setup_required));

        repository
            .replace_model_inventory(&[ModelInventoryItem {
                name: "medium.en".to_owned(),
                installed: true,
                setup_required: false,
                dependency: "faster-whisper".to_owned(),
            }])
            .unwrap();

        let snapshot = repository.snapshot().unwrap();

        assert!(snapshot
            .models
            .iter()
            .any(|model| model.name == "medium.en" && model.installed && !model.setup_required));
    }

    #[test]
    fn model_status_events_parse_worker_inventory() {
        let models = extract_model_inventory(&[worker_event(
            "models.status",
            serde_json::json!({
                "models": [
                    {
                        "name": "small.en",
                        "installed": true,
                        "setupRequired": false,
                        "dependency": "faster-whisper",
                    },
                ],
            }),
        )])
        .unwrap();

        assert_eq!(
            models,
            vec![ModelInventoryItem {
                name: "small.en".to_owned(),
                installed: true,
                setup_required: false,
                dependency: "faster-whisper".to_owned(),
            }]
        );
    }

    #[test]
    fn settings_changes_do_not_rewrite_existing_artifact_paths() {
        let database_path = test_database_path("settings-migration");
        let output_a = test_artifact_path("records-a");
        let output_b = test_artifact_path("records-b");
        let mut repository = AppRepository::open(&database_path).unwrap();
        let update = settings_update(output_a.display().to_string());

        repository.update_settings(update, false).unwrap();

        let settings = repository.settings().unwrap();
        let artifact_path =
            artifact_directory(&settings.output_directory, 1_717_938_012, "Meeting");
        let recording = NewRecording {
            id: "recording-2".to_owned(),
            title: "Meeting".to_owned(),
            started_at: "1717938012".to_owned(),
            artifact_directory: artifact_path.display().to_string(),
        };
        let mut capture_backend = FileAudioCaptureBackend::default();

        capture_backend
            .start(&recording.id, &repository.settings().unwrap())
            .unwrap();
        repository.create_recording(recording.clone()).unwrap();
        let capture_result = capture_backend.stop(&recording.id, &artifact_path).unwrap();
        repository
            .finish_recording(
                &recording.id,
                "1717938013".to_owned(),
                1,
                capture_result.errors,
                &capture_result.artifacts,
            )
            .unwrap();
        repository
            .update_settings(settings_update(output_b.display().to_string()), false)
            .unwrap();

        let snapshot = repository.snapshot().unwrap();

        assert!(snapshot.recordings[0]
            .artifact_directory
            .starts_with(&output_a.display().to_string()));
        assert_eq!(
            snapshot.settings.output_directory,
            output_b.display().to_string()
        );
    }

    #[test]
    fn settings_update_creates_configured_storage_directories() {
        let database_path = test_database_path("settings-directories");
        let output_directory = test_artifact_path("created-records-root");
        let model_directory = test_artifact_path("created-models-root");
        let mut repository = AppRepository::open(&database_path).unwrap();
        let mut update = settings_update(output_directory.display().to_string());

        let _ = fs::remove_dir_all(&output_directory);
        let _ = fs::remove_dir_all(&model_directory);
        update.model_storage_directory = model_directory.display().to_string();

        repository.update_settings(update, false).unwrap();

        assert!(output_directory.exists());
        assert!(model_directory.exists());
    }

    #[test]
    fn delete_recording_removes_database_rows_and_artifacts() {
        let database_path = test_database_path("delete-recording");
        let artifact_path = test_artifact_path("delete-recording-artifacts");
        let mut repository = AppRepository::open(&database_path).unwrap();
        let mut capture_backend = FileAudioCaptureBackend::default();
        let recording = NewRecording {
            id: "recording-delete".to_owned(),
            title: "Delete me".to_owned(),
            started_at: "1".to_owned(),
            artifact_directory: artifact_path.display().to_string(),
        };

        capture_backend
            .start(&recording.id, &repository.settings().unwrap())
            .unwrap();
        repository.create_recording(recording.clone()).unwrap();
        let capture_result = capture_backend.stop(&recording.id, &artifact_path).unwrap();
        repository
            .finish_recording(
                &recording.id,
                "2".to_owned(),
                1,
                capture_result.errors,
                &capture_result.artifacts,
            )
            .unwrap();

        repository.delete_recording(&recording.id, true).unwrap();

        let snapshot = repository.snapshot().unwrap();

        assert!(snapshot.recordings.is_empty());
        assert!(snapshot.jobs.is_empty());
        assert!(!artifact_path.exists());
    }

    #[test]
    fn retryable_jobs_reset_to_pending() {
        let database_path = test_database_path("retry-recording");
        let artifact_path = test_artifact_path("retry-recording-artifacts");
        let mut repository = AppRepository::open(&database_path).unwrap();
        let mut capture_backend = FileAudioCaptureBackend::default();
        let recording = NewRecording {
            id: "recording-retry".to_owned(),
            title: "Retry me".to_owned(),
            started_at: "1".to_owned(),
            artifact_directory: artifact_path.display().to_string(),
        };

        capture_backend
            .start(&recording.id, &repository.settings().unwrap())
            .unwrap();
        repository.create_recording(recording.clone()).unwrap();
        let capture_result = capture_backend.stop(&recording.id, &artifact_path).unwrap();
        repository
            .finish_recording(
                &recording.id,
                "2".to_owned(),
                1,
                capture_result.errors,
                &capture_result.artifacts,
            )
            .unwrap();
        repository
            .update_job(
                "recording-retry-recording",
                PipelineStageStatus::Failed,
                0,
                "Capture failed",
            )
            .unwrap();
        repository
            .update_job(
                "recording-retry-transcription",
                PipelineStageStatus::Failed,
                0,
                "Worker missing",
            )
            .unwrap();

        repository.reset_retryable_jobs(&recording.id).unwrap();

        let job = repository
            .job_for_recording_stage(&recording.id, PipelineStageId::Transcription)
            .unwrap();
        let recording_job = repository
            .job_for_recording_stage(&recording.id, PipelineStageId::Recording)
            .unwrap();

        assert_eq!(job.status, PipelineStageStatus::Pending);
        assert_eq!(job.message, "Retry queued");
        assert_eq!(recording_job.status, PipelineStageStatus::Failed);
        assert_eq!(recording_job.message, "Capture failed");
    }

    #[test]
    fn capture_backend_writes_non_empty_audio_files() {
        let artifact_path = test_artifact_path("capture");
        let mut capture_backend = FileAudioCaptureBackend::default();

        let _ = fs::remove_dir_all(&artifact_path);
        capture_backend
            .start(
                "recording-3",
                &default_settings(&test_database_path("capture-settings")),
            )
            .unwrap();
        capture_backend.stop("recording-3", &artifact_path).unwrap();

        let file_path = artifact_path.join("recording.wav");

        assert!(file_path.exists());
        assert!(fs::metadata(file_path).unwrap().len() > 44);
        assert!(!artifact_path.join("mic.wav").exists());
        assert!(!artifact_path.join("system.wav").exists());
    }

    #[test]
    fn capture_artifacts_include_only_canonical_audio() {
        let artifacts = capture_artifacts_with_readiness(&test_artifact_path("artifacts"), true);

        assert!(artifacts
            .iter()
            .any(|artifact| artifact.kind == ArtifactKind::Audio && artifact.ready));
        assert!(!artifacts.iter().any(|artifact| matches!(
            artifact.kind,
            ArtifactKind::MicrophoneAudio | ArtifactKind::SystemAudio
        )));
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.kind == ArtifactKind::Metadata && artifact.ready));
    }

    #[test]
    fn system_source_name_helpers_cover_default_and_monitor_devices() {
        assert!(is_default_system_source_name(""));
        assert!(is_default_system_source_name("Default system output"));
        assert!(is_system_monitor_device_name(
            "alsa_output.pci-0000_00_1b.0.analog-stereo.monitor",
        ));
        assert!(is_system_monitor_device_name("BlackHole 2ch Loopback"));
        assert!(is_system_monitor_device_name("Stereo Mix"));
        assert!(!is_system_monitor_device_name("Built-in Microphone"));
    }

    #[test]
    fn capture_device_dedupe_preserves_first_entry() {
        let devices = super::dedupe_capture_devices(vec![
            super::CaptureDeviceInfo {
                name: "Default microphone".to_owned(),
                label: "Default microphone".to_owned(),
                default: true,
            },
            super::CaptureDeviceInfo {
                name: "Default microphone".to_owned(),
                label: "Duplicate".to_owned(),
                default: false,
            },
        ]);

        assert_eq!(devices.len(), 1);
        assert!(devices[0].default);
        assert_eq!(devices[0].label, "Default microphone");
    }

    #[test]
    fn resume_pipeline_runs_worker_events_and_persists_artifacts() {
        let database_path = test_database_path("pipeline-resume");
        let artifact_path = test_artifact_path("pipeline-recording");
        let mut repository = AppRepository::open(&database_path).unwrap();
        let mut capture_backend = FileAudioCaptureBackend::default();
        let recording = NewRecording {
            id: "recording-4".to_owned(),
            title: "Meeting".to_owned(),
            started_at: "1".to_owned(),
            artifact_directory: artifact_path.display().to_string(),
        };

        capture_backend
            .start(&recording.id, &repository.settings().unwrap())
            .unwrap();
        repository.create_recording(recording.clone()).unwrap();
        let capture_result = capture_backend.stop(&recording.id, &artifact_path).unwrap();
        repository
            .finish_recording(
                &recording.id,
                "2".to_owned(),
                1,
                capture_result.errors,
                &capture_result.artifacts,
            )
            .unwrap();

        resume_pipeline_jobs(&mut repository, |command, payload| {
            let output_directory = std::path::PathBuf::from(
                payload
                    .get("outputDirectory")
                    .and_then(serde_json::Value::as_str)
                    .unwrap(),
            );

            match command {
                "transcribe.run" => {
                    fs::write(
                        output_directory.join("raw-segments.json"),
                        "{\"segments\":[{\"start\":0,\"end\":1,\"text\":\"Hello\"}]}\n",
                    )
                    .unwrap();
                    fs::write(
                        output_directory.join("raw-transcript.md"),
                        "# Raw\n\nHello\n",
                    )
                    .unwrap();

                    Ok(vec![worker_event(
                        "transcribe.complete",
                        serde_json::json!({
                            "segmentsPath": output_directory.join("raw-segments.json"),
                            "transcriptPath": output_directory.join("raw-transcript.md"),
                        }),
                    )])
                }
                "diarize.run" => {
                    fs::write(
                        output_directory.join("diarization.json"),
                        "{\"turns\":[{\"speaker\":\"Speaker 1\",\"start\":0,\"end\":1}]}\n",
                    )
                    .unwrap();
                    fs::write(
                        output_directory.join("diarized-transcript.md"),
                        "# Diarized\n\nHello\n",
                    )
                    .unwrap();

                    Ok(vec![worker_event(
                        "diarize.complete",
                        serde_json::json!({
                            "diarizationPath": output_directory.join("diarization.json"),
                            "transcriptPath": output_directory.join("diarized-transcript.md"),
                        }),
                    )])
                }
                other => Err(format!("unexpected worker command: {other}")),
            }
        })
        .unwrap();
        let snapshot = repository.snapshot().unwrap();
        let recording = &snapshot.recordings[0];

        assert!(recording.stages.iter().any(|stage| {
            stage.id == PipelineStageId::Transcription
                && stage.status == PipelineStageStatus::Complete
        }));
        assert!(recording.stages.iter().any(|stage| {
            stage.id == PipelineStageId::Diarization
                && stage.status == PipelineStageStatus::Complete
        }));
        assert!(recording.stages.iter().any(|stage| {
            stage.id == PipelineStageId::Summary && stage.status == PipelineStageStatus::Complete
        }));
        assert!(recording
            .artifacts
            .iter()
            .any(|artifact| artifact.kind == ArtifactKind::RawTranscript && artifact.ready));
        assert!(recording
            .artifacts
            .iter()
            .any(|artifact| artifact.kind == ArtifactKind::DiarizedTranscript && artifact.ready));
    }

    #[test]
    fn shortcut_lifecycle_uses_recording_start_stop_flow() {
        let database_path = test_database_path("shortcut-lifecycle");
        let mut repository = AppRepository::open(&database_path).unwrap();
        let mut capture_backend = FileAudioCaptureBackend::default();

        repository
            .update_settings(
                settings_update(test_artifact_path("shortcut-records").display().to_string()),
                false,
            )
            .unwrap();
        start_recording_session(&mut repository, &mut capture_backend).unwrap();

        let active_snapshot = repository.snapshot().unwrap();

        assert!(active_snapshot.active_recording.is_some());
        assert!(active_snapshot.desktop.overlay_visible);

        stop_recording_session(&mut repository, &mut capture_backend).unwrap();

        let stopped_snapshot = repository.snapshot().unwrap();

        assert!(stopped_snapshot.active_recording.is_none());
        assert!(!stopped_snapshot.desktop.overlay_visible);
        assert_eq!(stopped_snapshot.recordings.len(), 1);
    }

    #[test]
    fn desktop_runtime_status_is_included_in_snapshot() {
        let database_path = test_database_path("desktop-status");
        let mut repository = AppRepository::open(&database_path).unwrap();

        repository
            .update_desktop_runtime_status(&DesktopRuntimeStatus {
                overlay_visible: false,
                hotkey_registered: true,
                hotkey_error: None,
                worker_running: true,
                worker_health_ok: true,
                worker_error: None,
            })
            .unwrap();

        let snapshot = repository.snapshot().unwrap();

        assert!(snapshot.desktop.hotkey_registered);
        assert_eq!(snapshot.desktop.hotkey_error, None);
        assert!(snapshot.desktop.worker_running);
        assert!(snapshot.desktop.worker_health_ok);
    }

    #[test]
    fn worker_event_parser_reads_jsonl_events() {
        let events = parse_worker_events(
            "{\"commandId\":\"1\",\"event\":\"health.ok\",\"payload\":{\"worker\":\"test\"}}\n",
        )
        .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].command_id, "1");
        assert_eq!(events[0].event, "health.ok");
        assert_eq!(events[0].payload["worker"], "test");
    }

    #[test]
    fn worker_runtime_status_tracks_cli_mode() {
        let runtime = WorkerRuntimeState {
            running: true,
            health_ok: true,
            last_error: None,
        };
        let status = runtime.status();

        assert!(status.running);
        assert!(status.health_ok);
        assert_eq!(status.last_error, None);
    }

    fn settings_update(output_directory: String) -> AppSettingsUpdate {
        AppSettingsUpdate {
            output_directory,
            hotkey: "CommandOrControl+Shift+Space".to_owned(),
            overlay_position: OverlayPosition::TopLeft,
            launch_at_login: false,
            microphone_device: "Default microphone".to_owned(),
            system_audio_source: "Default system output".to_owned(),
            sample_rate: 48_000,
            whisper_model: "medium.en".to_owned(),
            transcription_language: "auto".to_owned(),
            compute_type: "auto".to_owned(),
            model_storage_directory: test_artifact_path("models").display().to_string(),
            diarization_backend: DiarizationBackend::NemoWhisper,
            speaker_count_mode: SpeakerCountMode::Automatic,
            exact_speakers: None,
            min_speakers: None,
            max_speakers: None,
            summary_enabled: false,
            provider_base_url: "https://api.openai.com/v1".to_owned(),
            provider_model: String::new(),
            provider_api_key: None,
            title_prompt: "Title".to_owned(),
            summary_prompt: "Summary".to_owned(),
        }
    }

    fn worker_event(event: &str, payload: serde_json::Value) -> WorkerEvent {
        WorkerEvent {
            command_id: "test-command".to_owned(),
            event: event.to_owned(),
            payload,
        }
    }

    fn test_database_path(name: &str) -> std::path::PathBuf {
        let directory = test_artifact_path(name);

        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();

        directory.join("actavoces.sqlite")
    }

    fn test_artifact_path(name: &str) -> std::path::PathBuf {
        env::temp_dir().join("actavoces-tests").join(name)
    }
}
