use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use tauri::{Emitter, Manager};

use crate::artifacts::{
    artifact, clean_transcript_path, diarized_transcript_path, mixed_audio_path, raw_segments_path,
    raw_transcript_read_path, raw_words_path, stage_label,
};
use crate::diarization::{
    run_single_speaker_diarization, run_sortformer_diarization, TranscriptSegment, TranscriptWord,
};
use crate::domain::types::*;
use crate::storage::repository::AppRepository;
use crate::utils::{pipeline_job_id, read_json_file};
use crate::worker::runtime::run_worker_command;

use super::overlay::sync_recording_overlay;
use super::recordings::rename_recording_outputs;

const MAX_TRANSCRIPTION_CONTEXT_CHARS: usize = 4000;
// Keep this limit in sync with worker/app/models.py.

pub fn emit_snapshot_update(app: &tauri::AppHandle, snapshot: &AppSnapshot) {
    let _ = app.emit("app-snapshot-updated", snapshot);
}

pub fn spawn_pipeline_processing(app: tauri::AppHandle) {
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

pub fn run_pipeline_processing(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, ActavocesState>,
) -> Result<(), String> {
    let mut repository = state.repository()?;

    resume_pipeline_jobs(&mut repository, run_worker_command, |repository| {
        let snapshot = repository.snapshot().map_err(|error| error.to_string())?;
        emit_snapshot_update(app, &snapshot);

        Ok(())
    })?;
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
        snapshot.settings.overlay_display_mode,
    )?;

    Ok(snapshot)
}

pub fn resume_pipeline_jobs<F>(
    repository: &mut AppRepository,
    mut run_worker: F,
    mut on_update: impl FnMut(&mut AppRepository) -> Result<(), String>,
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
                &mut on_update,
            )?;
        }

        if stage_is_complete(repository, &recording.id, PipelineStageId::Transcription)?
            && should_run_stage(repository, &recording.id, PipelineStageId::Diarization)?
        {
            match settings.diarization_backend {
                DiarizationBackend::Pyannote => {
                    let diarization_api_key = if exact_one_speaker_diarization(&settings) {
                        String::new()
                    } else {
                        match repository
                            .read_hugging_face_token()
                            .map_err(|error| error.to_string())?
                        {
                            Some(token) => token,
                            None => {
                                mark_stage_needs_setup(
                                    repository,
                                    &recording.id,
                                    PipelineStageId::Diarization,
                                    "Hugging Face token is required for speaker diarization",
                                )?;
                                on_update(repository)?;
                                continue;
                            }
                        }
                    };
                    run_pipeline_stage(
                        repository,
                        &recording,
                        PipelineStageId::Diarization,
                        diarization_payload(&recording, &settings, diarization_api_key),
                        "diarize.run",
                        &mut run_worker,
                        &mut on_update,
                    )?;
                }
                DiarizationBackend::Sortformer => {
                    run_sortformer_pipeline_stage(
                        repository,
                        &recording,
                        &settings,
                        &mut on_update,
                    )?;
                }
            }
        }

        if should_run_stage(repository, &recording.id, PipelineStageId::Summary)? {
            if !settings.summary_enabled {
                complete_disabled_summary_stage(repository, &recording.id)?;
                repository
                    .complete_recording_if_pipeline_done(&recording.id)
                    .map_err(|error| error.to_string())?;
                on_update(repository)?;
                continue;
            }

            let api_key = repository
                .read_summary_provider_api_key()
                .map_err(|error| error.to_string())?
                .unwrap_or_default();

            run_pipeline_stage(
                repository,
                &recording,
                PipelineStageId::Summary,
                summary_payload(&recording, &settings, api_key),
                "summarize.run",
                &mut run_worker,
                &mut on_update,
            )?;
        }

        repository
            .complete_recording_if_pipeline_done(&recording.id)
            .map_err(|error| error.to_string())?;
        on_update(repository)?;
    }

    Ok(())
}

fn run_sortformer_pipeline_stage(
    repository: &mut AppRepository,
    recording: &Recording,
    settings: &AppSettings,
    on_update: &mut impl FnMut(&mut AppRepository) -> Result<(), String>,
) -> Result<(), String> {
    let stage = PipelineStageId::Diarization;
    let job_id = pipeline_job_id(&recording.id, stage)?;
    let artifact_directory = PathBuf::from(&recording.artifact_directory);
    let segments = read_transcript_segments(&artifact_directory);
    let words = read_transcript_words(&artifact_directory);

    repository
        .update_job(
            &job_id,
            PipelineStageStatus::Running,
            5,
            "Sortformer diarization started",
        )
        .and_then(|()| {
            repository.append_event(
                &recording.id,
                stage,
                PipelineStageStatus::Running,
                "Sortformer diarization started",
            )
        })
        .map_err(|error| error.to_string())?;
    on_update(repository)?;

    let output = match run_local_diarization(
        &artifact_directory,
        settings,
        &segments,
        &words,
        &recording.title,
    ) {
        Ok(output) => output,
        Err(error) => {
            mark_stage_failed(repository, &recording.id, stage, &error)?;
            on_update(repository)?;

            return Ok(());
        }
    };

    repository
        .upsert_artifact(
            &recording.id,
            &artifact(
                ArtifactKind::Diarization,
                "Diarization turns",
                output.diarization_path,
                true,
            ),
        )
        .and_then(|()| {
            repository.upsert_artifact(
                &recording.id,
                &artifact(
                    ArtifactKind::DiarizedTranscript,
                    "Diarized transcript",
                    output.transcript_path,
                    true,
                ),
            )
        })
        .and_then(|()| {
            repository.upsert_artifact(
                &recording.id,
                &artifact(
                    ArtifactKind::CleanTranscript,
                    "Clean transcript",
                    output.clean_transcript_path,
                    true,
                ),
            )
        })
        .map_err(|error| error.to_string())?;

    mark_stage_complete(
        repository,
        &recording.id,
        stage,
        "Sortformer diarization complete",
    )?;
    on_update(repository)
}

fn run_local_diarization(
    artifact_directory: &Path,
    settings: &AppSettings,
    segments: &[TranscriptSegment],
    words: &[TranscriptWord],
    title: &str,
) -> Result<crate::diarization::SortformerDiarizationOutput, String> {
    if exact_one_speaker_diarization(settings) {
        return run_single_speaker_diarization(artifact_directory, segments, words, title);
    }

    run_sortformer_diarization(
        &mixed_audio_path(artifact_directory),
        artifact_directory,
        &PathBuf::from(&settings.model_storage_directory),
        segments,
        words,
        title,
    )
}

fn run_pipeline_stage<F>(
    repository: &mut AppRepository,
    recording: &Recording,
    stage: PipelineStageId,
    payload: serde_json::Value,
    command_name: &str,
    run_worker: &mut F,
    on_update: &mut impl FnMut(&mut AppRepository) -> Result<(), String>,
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
    on_update(repository)?;

    let events = match run_worker(command_name, payload) {
        Ok(events) => events,
        Err(error) => {
            mark_stage_failed(repository, &recording.id, stage, &error)?;
            on_update(repository)?;

            return Ok(());
        }
    };

    apply_worker_events(repository, recording, stage, &events)?;
    on_update(repository)
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
                ArtifactKind::CleanTranscript,
                "Clean transcript",
                event,
                "cleanTranscriptPath",
            )?;
            upsert_ready_artifact_from_event(
                repository,
                &recording.id,
                ArtifactKind::RawTranscript,
                "Raw ASR transcript",
                event,
                "transcriptPath",
            )?;
            upsert_ready_artifact_from_event(
                repository,
                &recording.id,
                ArtifactKind::RawWords,
                "Raw words",
                event,
                "wordsPath",
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
            upsert_ready_artifact_from_event(
                repository,
                &recording.id,
                ArtifactKind::CleanTranscript,
                "Clean transcript",
                event,
                "cleanTranscriptPath",
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
                    rename_recording_outputs(repository, recording, title.trim())?;
                }
            }
        }
        PipelineStageId::Recording => {}
    }

    let message = event
        .payload
        .get("warning")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Worker stage complete");

    mark_stage_complete(repository, &recording.id, stage, message)
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

fn complete_disabled_summary_stage(
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

fn setup_message(stage: PipelineStageId, event: &WorkerEvent) -> String {
    let dependency = event
        .payload
        .get("dependency")
        .and_then(serde_json::Value::as_str);

    match dependency {
        Some(dependency) => format!("{} requires {dependency} setup", stage_label(stage)),
        None => format!("{} needs additional setup", stage_label(stage)),
    }
}

fn mark_stage_skipped(
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
    let mut payload = serde_json::json!({
        "audioPath": mixed_audio_path(&artifact_directory),
        "outputDirectory": artifact_directory,
        "title": recording.title,
        "model": settings.whisper_model,
        "language": settings.transcription_language,
        "computeType": settings.compute_type,
        "modelStorageDirectory": settings.model_storage_directory,
    });

    if let Some(context) = normalized_transcription_context(&settings.transcription_context) {
        payload["transcriptionContext"] = serde_json::json!(context);
    }

    payload
}

pub(crate) fn normalized_transcription_context(input: &str) -> Option<String> {
    let mut seen = HashSet::new();
    let mut values = Vec::new();

    for line in input.lines() {
        let value = line.trim();

        if value.is_empty() || !seen.insert(value) {
            continue;
        }

        values.push(value);
    }

    let context = values.join("\n");

    if context.is_empty() {
        return None;
    }

    Some(
        context
            .chars()
            .take(MAX_TRANSCRIPTION_CONTEXT_CHARS)
            .collect(),
    )
}

fn diarization_payload(
    recording: &Recording,
    settings: &AppSettings,
    api_key: String,
) -> serde_json::Value {
    let artifact_directory = PathBuf::from(&recording.artifact_directory);
    let segments = read_json_file(raw_segments_path(&artifact_directory))
        .and_then(|value| value.get("segments").cloned())
        .unwrap_or_else(|| serde_json::json!([]));
    let words = read_json_file(raw_words_path(&artifact_directory))
        .and_then(|value| value.get("words").cloned())
        .unwrap_or_else(|| serde_json::json!([]));

    serde_json::json!({
        "audioPath": mixed_audio_path(&artifact_directory),
        "outputDirectory": artifact_directory,
        "segments": segments,
        "words": words,
        "title": recording.title,
        "backend": settings.diarization_backend,
        "apiKey": api_key,
        "speakerCountMode": settings.speaker_count_mode,
        "exactSpeakers": settings.exact_speakers,
        "minSpeakers": settings.min_speakers,
        "maxSpeakers": settings.max_speakers,
    })
}

fn read_transcript_segments(artifact_directory: &Path) -> Vec<TranscriptSegment> {
    read_json_file(raw_segments_path(artifact_directory))
        .and_then(|value| value.get("segments").cloned())
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

fn read_transcript_words(artifact_directory: &Path) -> Vec<TranscriptWord> {
    read_json_file(raw_words_path(artifact_directory))
        .and_then(|value| value.get("words").cloned())
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

fn exact_one_speaker_diarization(settings: &AppSettings) -> bool {
    settings.speaker_count_mode == SpeakerCountMode::Exact && settings.exact_speakers == Some(1)
}

fn summary_payload(
    recording: &Recording,
    settings: &AppSettings,
    api_key: String,
) -> serde_json::Value {
    let artifact_directory = PathBuf::from(&recording.artifact_directory);

    serde_json::json!({
        "output_directory": artifact_directory,
        "provider_base_url": settings.provider_base_url,
        "api_key": api_key,
        "model": settings.provider_model,
        "clean_transcript_path": clean_transcript_path(&artifact_directory),
        "diarized_transcript_path": diarized_transcript_path(&artifact_directory),
        "transcript_path": raw_transcript_read_path(&artifact_directory),
        "summary_prompt": settings.summary_prompt,
    })
}
