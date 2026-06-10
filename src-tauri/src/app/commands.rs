use std::fs;
use std::path::PathBuf;

use tauri::{Emitter, Manager, PhysicalPosition, WebviewUrl};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::artifacts::{artifact, artifact_directory, stage_label};
use crate::capture::audio::AudioCaptureBackend;
use crate::domain::types::*;
use crate::settings::{
    clear_hugging_face_secret, clear_summary_provider_secret, read_hugging_face_token,
    read_summary_provider_api_key, update_hugging_face_token, update_summary_provider_api_key,
};
use crate::storage::repository::{AppRepository, NewRecording};
use crate::utils::{lock_error, pipeline_job_id, read_json_file, unix_timestamp};
use crate::worker::runtime::{
    bootstrap_worker, extract_model_inventory, extract_runtime_capabilities, model_install_message,
    persist_worker_setup_progress, run_diarization_setup, run_worker_command,
};
#[tauri::command]
pub fn get_app_snapshot(state: tauri::State<'_, ActavocesState>) -> Result<AppSnapshot, String> {
    let repository = state.repository()?;

    repository.snapshot().map_err(|error| error.to_string())
}

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

pub(crate) fn emit_snapshot_update(app: &tauri::AppHandle, snapshot: &AppSnapshot) {
    let _ = app.emit("app-snapshot-updated", snapshot);
}

pub(crate) fn spawn_pipeline_processing(app: tauri::AppHandle) {
    let state = app.state::<ActavocesState>();
    let mut running = match state.pipeline_running.lock() {
        Ok(running) => running,
        Err(_) => return,
    };

    if *running {
        return;
    }

    *running = true;
    drop(running);

    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<ActavocesState>();

        match run_pipeline_processing(&app, &state) {
            Ok(()) => (),
            Err(error) => {
                if let Ok(mut repository) = state.repository() {
                    let _ = repository.set_worker_error(&error);
                    if let Ok(snapshot) = repository.snapshot() {
                        emit_snapshot_update(&app, &snapshot);
                    }
                }
            }
        }

        if let Ok(mut running) = state.pipeline_running.lock() {
            *running = false;
        };
    });
}

pub(crate) fn run_pipeline_processing(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, ActavocesState>,
) -> Result<(), String> {
    let mut repository = state.repository()?;

    resume_pipeline_jobs(&mut repository, run_worker_command)?;
    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;
    emit_snapshot_update(app, &snapshot);

    Ok(())
}

#[tauri::command]
pub fn resume_pending_jobs(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    spawn_pipeline_processing(app.clone());
    let repository = state.repository()?;
    let snapshot = repository.snapshot().map_err(|error| error.to_string())?;

    sync_recording_overlay(
        &app,
        snapshot.active_recording.is_some(),
        snapshot.settings.overlay_position,
    )?;

    Ok(snapshot)
}

#[tauri::command]
pub async fn bootstrap_worker_runtime(app: tauri::AppHandle) -> Result<AppSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<ActavocesState>();

        match bootstrap_worker(&app, &state) {
            Ok(()) => {
                let repository = state.repository()?;

                let snapshot = repository.snapshot().map_err(|error| error.to_string())?;
                drop(repository);
                spawn_pipeline_processing(app.clone());

                Ok(snapshot)
            }
            Err(error) => {
                let progress = WorkerSetupProgress {
                    status: WorkerSetupStatus::Failed,
                    step: "Worker setup failed".to_owned(),
                    error: Some(error.clone()),
                };

                persist_worker_setup_progress(&state, &progress)?;
                app.emit("worker-setup-progress", progress)
                    .map_err(|emit_error| emit_error.to_string())?;

                Err(error)
            }
        }
    })
    .await
    .map_err(|error| format!("Worker setup task failed: {error}"))?
}

#[tauri::command]
pub fn get_worker_status(state: tauri::State<'_, ActavocesState>) -> Result<WorkerStatus, String> {
    state
        .worker_runtime
        .lock()
        .map(|runtime| runtime.status())
        .map_err(lock_error)
}

#[tauri::command]
pub fn start_worker(state: tauri::State<'_, ActavocesState>) -> Result<WorkerStatus, String> {
    let status = {
        let mut runtime = state.worker_runtime.lock().map_err(lock_error)?;

        runtime.running = true;
        runtime.status()
    };

    persist_worker_status(&state, &status)?;

    Ok(status)
}

#[tauri::command]
pub fn stop_worker(state: tauri::State<'_, ActavocesState>) -> Result<WorkerStatus, String> {
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
pub fn check_worker_health(
    state: tauri::State<'_, ActavocesState>,
) -> Result<WorkerStatus, String> {
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
pub fn refresh_model_inventory(
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let settings = {
        let repository = state.repository()?;

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
            let capabilities = run_worker_command("runtime.capabilities", serde_json::json!({}))
                .and_then(|events| extract_runtime_capabilities(&events))
                .ok();
            let mut repository = state.repository()?;

            repository
                .replace_model_inventory(&models)
                .map_err(|error| error.to_string())?;
            if let Some(capabilities) = capabilities {
                repository
                    .update_runtime_capabilities(&capabilities)
                    .map_err(|error| error.to_string())?;
            }
            repository
                .clear_worker_error()
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
        Err(error) => {
            let mut repository = state.repository()?;

            repository
                .set_worker_error(&error)
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
    }
}

#[tauri::command]
pub fn install_transcription_model(
    input: ModelInstallInput,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let (settings, cuda_available) = {
        let repository = state.repository()?;

        (
            repository.settings().map_err(|error| error.to_string())?,
            repository
                .desktop_runtime_status()
                .map_err(|error| error.to_string())?
                .cuda_available,
        )
    };
    let compute_type = match settings.compute_type.as_str() {
        "cuda" if !cuda_available => "cpu",
        value => value,
    };
    let result = run_worker_command(
        "models.install",
        serde_json::json!({
            "model": input.model,
            "computeType": compute_type,
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
            let mut repository = state.repository()?;

            repository
                .set_worker_error(&message)
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
        Err(error) => {
            let mut repository = state.repository()?;

            repository
                .set_worker_error(&error)
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
    }
}

pub(crate) fn start_recording_session(
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

pub(crate) fn stop_recording_session(
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

pub(crate) fn persist_worker_status(
    state: &tauri::State<'_, ActavocesState>,
    worker_status: &WorkerStatus,
) -> Result<(), String> {
    let mut repository = state.repository()?;

    repository
        .update_worker_runtime_status(worker_status)
        .map_err(|error| error.to_string())
}

pub(crate) fn resume_pipeline_jobs<F>(
    repository: &mut AppRepository,
    mut run_worker: F,
) -> Result<(), String>
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
            let diarization_api_key = if exact_one_speaker_diarization(&settings) {
                String::new()
            } else {
                match settings.diarization_backend {
                    DiarizationBackend::Pyannote => match read_hugging_face_token()? {
                        Some(token) => token,
                        None => {
                            mark_stage_needs_setup(
                                repository,
                                &recording.id,
                                PipelineStageId::Diarization,
                                "Hugging Face token is required for speaker diarization",
                            )?;
                            continue;
                        }
                    },
                }
            };
            run_pipeline_stage(
                repository,
                &recording,
                PipelineStageId::Diarization,
                diarization_payload(&recording, &settings, diarization_api_key),
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

pub(crate) fn run_pipeline_stage<F>(
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

pub(crate) fn apply_worker_events(
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
            mark_stage_needs_setup(
                repository,
                &recording.id,
                stage,
                &setup_message(stage, event),
            )?;
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

pub(crate) fn apply_complete_event(
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

    let message = event
        .payload
        .get("warning")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Worker stage complete");

    mark_stage_complete(repository, &recording.id, stage, message)
}

pub(crate) fn upsert_ready_artifact_from_event(
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

pub(crate) fn complete_alignment_stage(
    repository: &mut AppRepository,
    recording_id: &str,
) -> Result<(), String> {
    if !should_run_stage(repository, recording_id, PipelineStageId::Alignment)? {
        return Ok(());
    }

    mark_stage_skipped(
        repository,
        recording_id,
        PipelineStageId::Alignment,
        "No separate alignment pass is needed yet; transcript timings are used by diarization.",
    )
}

pub(crate) fn complete_disabled_summary_stage(
    repository: &mut AppRepository,
    recording_id: &str,
) -> Result<(), String> {
    mark_stage_skipped(
        repository,
        recording_id,
        PipelineStageId::Summary,
        "Summary generation is disabled",
    )
}

pub(crate) fn mark_stage_complete(
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

pub(crate) fn mark_stage_needs_setup(
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

pub(crate) fn setup_message(stage: PipelineStageId, event: &WorkerEvent) -> String {
    let dependency = event
        .payload
        .get("dependency")
        .and_then(serde_json::Value::as_str);

    match dependency {
        Some(dependency) => format!("{} requires {dependency} setup", stage_label(stage)),
        None => format!("{} needs additional setup", stage_label(stage)),
    }
}

pub(crate) fn mark_stage_skipped(
    repository: &mut AppRepository,
    recording_id: &str,
    stage: PipelineStageId,
    message: &str,
) -> Result<(), String> {
    repository
        .update_job(
            &pipeline_job_id(recording_id, stage)?,
            PipelineStageStatus::Skipped,
            100,
            message,
        )
        .and_then(|()| {
            repository.append_event(recording_id, stage, PipelineStageStatus::Skipped, message)
        })
        .map_err(|error| error.to_string())
}

pub(crate) fn mark_stage_failed(
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

pub(crate) fn should_run_stage(
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

pub(crate) fn stage_is_complete(
    repository: &AppRepository,
    recording_id: &str,
    stage: PipelineStageId,
) -> Result<bool, String> {
    let job = repository
        .job_for_recording_stage(recording_id, stage)
        .map_err(|error| error.to_string())?;

    Ok(job.status == PipelineStageStatus::Complete)
}

pub(crate) fn transcription_payload(
    recording: &Recording,
    settings: &AppSettings,
) -> serde_json::Value {
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

pub(crate) fn diarization_payload(
    recording: &Recording,
    settings: &AppSettings,
    api_key: String,
) -> serde_json::Value {
    let artifact_directory = PathBuf::from(&recording.artifact_directory);
    let segments = read_json_file(artifact_directory.join("raw-segments.json"))
        .and_then(|value| value.get("segments").cloned())
        .unwrap_or_else(|| serde_json::json!([]));

    serde_json::json!({
        "audioPath": artifact_directory.join("recording.wav"),
        "outputDirectory": artifact_directory,
        "segments": segments,
        "backend": settings.diarization_backend,
        "apiKey": api_key,
        "speakerCountMode": settings.speaker_count_mode,
        "exactSpeakers": settings.exact_speakers,
        "minSpeakers": settings.min_speakers,
        "maxSpeakers": settings.max_speakers,
    })
}

pub(crate) fn exact_one_speaker_diarization(settings: &AppSettings) -> bool {
    settings.speaker_count_mode == SpeakerCountMode::Exact && settings.exact_speakers == Some(1)
}

pub(crate) fn summary_payload(
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

pub(crate) fn rewrite_speaker_label(
    recording: &Recording,
    input: &SpeakerRenameInput,
) -> Result<(), String> {
    let current = input.speaker.trim();
    let replacement = input.replacement.trim();

    if current.is_empty() {
        return Err("Current speaker label is required".to_owned());
    }

    if replacement.is_empty() {
        return Err("Replacement speaker label is required".to_owned());
    }

    let artifact_directory = PathBuf::from(&recording.artifact_directory);
    let diarization_path = artifact_directory.join("diarization.json");
    let raw_segments_path = artifact_directory.join("raw-segments.json");
    let transcript_path = artifact_directory.join("diarized-transcript.md");
    let mut diarization = read_structured_artifact::<DiarizationArtifact>(&diarization_path)?;
    let segments = read_structured_artifact::<SegmentsArtifact>(&raw_segments_path)?.segments;
    let mut changed = false;

    for turn in &mut diarization.turns {
        if turn.speaker == current {
            turn.speaker = replacement.to_owned();
            changed = true;
        }
    }

    if !changed {
        return Err(format!("Speaker label not found: {current}"));
    }

    let content = serde_json::to_string_pretty(&diarization).map_err(|error| error.to_string())?;

    fs::write(&diarization_path, format!("{content}\n")).map_err(|error| error.to_string())?;
    fs::write(
        &transcript_path,
        render_diarized_transcript(&segments, &diarization.turns),
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn create_recording_overlay(app: &tauri::App) -> tauri::Result<()> {
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

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SegmentsArtifact {
    segments: Vec<TranscriptSegment>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiarizationArtifact {
    turns: Vec<SpeakerTurnArtifact>,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TranscriptSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpeakerTurnArtifact {
    speaker: String,
    start: f64,
    end: f64,
}

pub(crate) fn read_structured_artifact<T>(path: &std::path::Path) -> Result<T, String>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;

    serde_json::from_str(&content)
        .map_err(|error| format!("Unable to parse {}: {error}", path.display()))
}

pub(crate) fn render_diarized_transcript(
    segments: &[TranscriptSegment],
    turns: &[SpeakerTurnArtifact],
) -> String {
    let mut lines = vec!["# Diarized transcript".to_owned(), String::new()];

    for turn in turns {
        let text = segment_texts_in_turn(segments, turn.start, turn.end).join(" ");

        lines.push(format!("## {}", turn.speaker));
        lines.push(String::new());
        lines.push(
            format!(
                "[{} - {}] {text}",
                format_artifact_timestamp(turn.start),
                format_artifact_timestamp(turn.end)
            )
            .trim()
            .to_owned(),
        );
        lines.push(String::new());
    }

    lines.join("\n")
}

pub(crate) fn segment_texts_in_turn(
    segments: &[TranscriptSegment],
    start: f64,
    end: f64,
) -> Vec<String> {
    let mut texts = Vec::new();

    for segment in segments {
        if segment.start >= start && segment.end <= end {
            texts.push(segment.text.trim().to_owned());
        }
    }

    texts
}

pub(crate) fn format_artifact_timestamp(value: f64) -> String {
    let total_seconds = value as u64;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{minutes:02}:{seconds:02}")
}

pub(crate) fn sync_recording_overlay(
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

pub(crate) fn position_recording_overlay(
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

pub(crate) fn refresh_global_hotkey(
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

pub(crate) fn sync_launch_at_login(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
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

pub(crate) fn register_global_hotkey(app: &tauri::AppHandle, hotkey: &str) -> DesktopRuntimeStatus {
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

pub(crate) fn toggle_recording_lifecycle(
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
