use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::app::commands::{
    normalized_transcription_context, rename_recording_outputs, resume_pipeline_jobs,
    rewrite_speaker_label, start_recording_session, stop_recording_session,
};
use crate::artifacts::{
    artifact_directory, capture_artifacts_with_readiness, diarization_path,
    diarized_transcript_path, microphone_audio_path, mixed_audio_path, raw_segments_path,
    raw_transcript_path, raw_words_path, summary_path,
};
use crate::capture::audio::{
    dedupe_capture_devices, is_default_system_source_name, is_system_monitor_device_name,
    mixed_recording_source, AudioCaptureBackend, FileAudioCaptureBackend, FinalizedSource,
};
use crate::domain::types::{
    AppSettingsUpdate, ArtifactKind, CaptureDeviceInfo, DesktopRuntimeStatus, DiarizationBackend,
    ModelInventoryItem, OverlayDisplayMode, OverlayPosition, PipelineStageId, PipelineStageStatus,
    RecordingStatus, SpeakerCountMode, SpeakerRenameInput, WorkerEvent, WorkerSetupStatus,
};
use crate::settings::{
    default_settings,
    recommendation::{model_recommendation, ModelRecommendationInput},
};
use crate::storage::repository::{AppRepository, NewRecording};
use crate::utils::default_records_root;
use crate::worker::runtime::{
    apply_worker_current_dir, apply_worker_path_env, extract_model_inventory,
    find_worker_python_executable, hash_worker_source_directory, model_install_payload,
    model_install_step, parse_worker_events, resolve_worker_virtualenv_python_executable,
    worker_runtime_paths_from_local_data_directory, WorkerRuntimePaths, WorkerRuntimeState,
};

static TEST_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn default_records_root_uses_home_actavoces_records() {
    let root = default_records_root();

    assert!(Path::new(&root).ends_with(Path::new("actavoces").join("records")));
}

#[test]
fn artifact_directory_uses_date_layout_and_stable_slug() {
    let path = artifact_directory("/tmp/records", 1_717_938_012, "Untitled meeting");

    assert_eq!(
        path,
        Path::new("/tmp/records").join("2024-06-09-1300-untitled-meeting")
    );
}

#[test]
fn repository_restores_recordings_after_reopen() {
    let database_path = test_database_path("restore");
    let artifact_path = test_artifact_path("restore-recording");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let mut capture_backend = FileAudioCaptureBackend::default();
    repository
        .set_setting(
            "diarizationBackend",
            &serde_json::to_string(&DiarizationBackend::Pyannote).unwrap(),
        )
        .unwrap();
    assert_eq!(
        repository.settings().unwrap().diarization_backend,
        DiarizationBackend::Pyannote
    );
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
fn repository_removes_legacy_alignment_stage_rows() {
    let database_path = test_database_path("legacy-alignment-stage");
    let artifact_path = test_artifact_path("legacy-alignment-stage-artifacts");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let recording = NewRecording {
        id: "recording-1".to_owned(),
        title: "Untitled meeting".to_owned(),
        started_at: "1".to_owned(),
        artifact_directory: artifact_path.display().to_string(),
    };

    repository.create_recording(recording.clone()).unwrap();
    repository
        .connection
        .execute(
            "
            INSERT INTO pipeline_jobs (id, recording_id, stage, status, progress, message)
            VALUES (?1, ?2, 'alignment', 'skipped', 100, 'legacy alignment')
            ",
            rusqlite::params!["recording-1-alignment", recording.id],
        )
        .unwrap();
    repository
        .connection
        .execute(
            "
            INSERT INTO job_events (recording_id, stage, status, message, created_at)
            VALUES (?1, 'alignment', 'skipped', 'legacy alignment', '1')
            ",
            rusqlite::params![recording.id],
        )
        .unwrap();
    drop(repository);

    let repository = AppRepository::open(&database_path).unwrap();
    let legacy_jobs = repository
        .connection
        .query_row(
            "SELECT COUNT(*) FROM pipeline_jobs WHERE stage = 'alignment'",
            [],
            |row| row.get::<_, u32>(0),
        )
        .unwrap();
    let legacy_events = repository
        .connection
        .query_row(
            "SELECT COUNT(*) FROM job_events WHERE stage = 'alignment'",
            [],
            |row| row.get::<_, u32>(0),
        )
        .unwrap();

    assert_eq!(legacy_jobs, 0);
    assert_eq!(legacy_events, 0);
    let stages = repository
        .stages(&recording.id)
        .unwrap()
        .iter()
        .map(|stage| stage.id)
        .collect::<Vec<_>>();

    assert_eq!(
        stages,
        vec![
            PipelineStageId::Recording,
            PipelineStageId::Transcription,
            PipelineStageId::Diarization,
            PipelineStageId::Summary,
        ]
    );
}

#[test]
fn repository_seeds_and_updates_model_inventory() {
    let database_path = test_database_path("models");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let initial_models = repository.model_inventory().unwrap();

    assert!(initial_models
        .iter()
        .any(|model| model.name == "medium" && model.setup_required));

    repository
        .replace_model_inventory(&[ModelInventoryItem {
            name: "medium".to_owned(),
            installed: true,
            setup_required: false,
            dependency: "faster-whisper".to_owned(),
        }])
        .unwrap();

    let snapshot = repository.snapshot().unwrap();

    assert!(snapshot
        .models
        .iter()
        .any(|model| model.name == "medium" && model.installed && !model.setup_required));
}

#[test]
fn cpu_only_low_resource_recommends_small_model() {
    let recommendation = model_recommendation(
        ModelRecommendationInput {
            cuda_available: false,
            total_memory_bytes: Some(8 * 1024 * 1024 * 1024),
            cpu_count: Some(4),
        },
        "small",
        false,
    );

    assert_eq!(recommendation.recommended_model, "small");
    assert!(!recommendation.user_overridden);
}

#[test]
fn cpu_only_higher_resource_recommends_medium_model() {
    let recommendation = model_recommendation(
        ModelRecommendationInput {
            cuda_available: false,
            total_memory_bytes: Some(16 * 1024 * 1024 * 1024),
            cpu_count: Some(6),
        },
        "medium",
        false,
    );

    assert_eq!(recommendation.recommended_model, "medium");
    assert!(!recommendation.user_overridden);
}

#[test]
fn validated_cuda_recommends_distil_large_v3_model() {
    let recommendation = model_recommendation(
        ModelRecommendationInput {
            cuda_available: true,
            total_memory_bytes: Some(8 * 1024 * 1024 * 1024),
            cpu_count: Some(4),
        },
        "distil-large-v3",
        false,
    );

    assert_eq!(recommendation.recommended_model, "distil-large-v3");
    assert!(!recommendation.user_overridden);
}

#[test]
fn repository_preserves_existing_whisper_model_setting() {
    let database_path = test_database_path("existing-whisper-model");
    let mut repository = AppRepository::open(&database_path).unwrap();

    repository.set_setting("whisperModel", "large-v3").unwrap();

    let settings = repository.settings().unwrap();

    assert_eq!(settings.whisper_model, "large-v3");
    assert!(settings.model_recommendation.user_overridden);
}

#[test]
fn bootstrap_model_install_payload_uses_settings_model() {
    let database_path = test_database_path("bootstrap-recommended-model");
    let mut settings = default_settings(&database_path);

    settings.whisper_model = "distil-large-v3".to_owned();

    let payload = model_install_payload(&settings, "cuda");

    assert_eq!(
        model_install_step(&settings.whisper_model),
        "Installing distil-large-v3 model"
    );
    assert_eq!(payload["model"], "distil-large-v3");
    assert_eq!(payload["computeType"], "cuda");
}

#[test]
fn model_status_events_parse_worker_inventory() {
    let models = extract_model_inventory(&[worker_event(
        "models.status",
        serde_json::json!({
            "models": [
                {
                    "name": "medium",
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
            name: "medium".to_owned(),
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

    repository.update_settings(update).unwrap();

    let settings = repository.settings().unwrap();
    let artifact_path = artifact_directory(&settings.output_directory, 1_717_938_012, "Meeting");
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
        .update_settings(settings_update(output_b.display().to_string()))
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

    repository.update_settings(update).unwrap();

    assert!(output_directory.exists());
    assert!(model_directory.exists());
}

#[test]
fn settings_update_persists_overlay_display_mode() {
    let database_path = test_database_path("overlay-display-mode-settings");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let mut update = settings_update(
        test_artifact_path("overlay-display-mode-records")
            .display()
            .to_string(),
    );

    update.overlay_display_mode = OverlayDisplayMode::Minimal;

    repository.update_settings(update).unwrap();

    let settings = repository.settings().unwrap();

    assert_eq!(settings.overlay_display_mode, OverlayDisplayMode::Minimal);
}

#[test]
fn settings_update_persists_transcription_context() {
    let database_path = test_database_path("transcription-context-settings");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let mut update = settings_update(
        test_artifact_path("transcription-context-records")
            .display()
            .to_string(),
    );

    assert_eq!(repository.settings().unwrap().transcription_context, "");

    update.transcription_context = "ActaVoces\nProject Orion".to_owned();

    repository.update_settings(update).unwrap();

    let settings = repository.settings().unwrap();

    assert_eq!(settings.transcription_context, "ActaVoces\nProject Orion");
}

#[test]
fn transcription_context_normalization_trims_deduplicates_and_bounds() {
    let oversized = "a".repeat(4_100);
    let context = normalized_transcription_context(&format!(
        "\n ActaVoces \n\nKaneo\nActaVoces\n{oversized}"
    ))
    .unwrap();

    assert!(context.starts_with("ActaVoces\nKaneo\n"));
    assert_eq!(context.chars().count(), 4_000);
    assert_eq!(normalized_transcription_context("\n  \n"), None);
}

#[test]
fn settings_secrets_are_stored_in_database_settings() {
    let database_path = test_database_path("settings-secrets");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let mut update = settings_update(test_artifact_path("secret-records").display().to_string());

    update.summary_enabled = true;
    update.provider_model = "gpt-4o-mini".to_owned();
    update.provider_api_key = Some("provider-secret".to_owned());
    update.hugging_face_token = Some("hf-secret".to_owned());

    repository.update_settings(update).unwrap();

    let settings = repository.settings().unwrap();

    assert!(settings.provider_api_key_configured);
    assert!(settings.hugging_face_token_configured);
    assert_eq!(
        repository.read_summary_provider_api_key().unwrap(),
        Some("provider-secret".to_owned())
    );
    assert_eq!(
        repository.read_hugging_face_token().unwrap(),
        Some("hf-secret".to_owned())
    );

    repository.clear_summary_provider_api_key().unwrap();
    repository.clear_hugging_face_token().unwrap();

    let settings = repository.settings().unwrap();

    assert!(!settings.provider_api_key_configured);
    assert!(!settings.hugging_face_token_configured);
    assert_eq!(repository.read_summary_provider_api_key().unwrap(), None);
    assert_eq!(repository.read_hugging_face_token().unwrap(), None);
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

    let mixed_file_path = mixed_audio_path(&artifact_path);
    let microphone_file_path = microphone_audio_path(&artifact_path);

    assert!(mixed_file_path.exists());
    assert!(microphone_file_path.exists());
    assert!(fs::metadata(mixed_file_path).unwrap().len() > 44);
    assert!(fs::metadata(microphone_file_path).unwrap().len() > 44);
    assert!(!artifact_path.join("meta").join("system.wav").exists());
}

#[test]
fn capture_artifacts_include_mixed_and_microphone_audio() {
    let artifacts = capture_artifacts_with_readiness(&test_artifact_path("artifacts"), true, true);

    assert!(artifacts
        .iter()
        .any(|artifact| artifact.kind == ArtifactKind::Audio && artifact.ready));
    assert!(artifacts
        .iter()
        .any(|artifact| artifact.kind == ArtifactKind::MicrophoneAudio && artifact.ready));
    assert!(!artifacts
        .iter()
        .any(|artifact| artifact.kind == ArtifactKind::SystemAudio));
    assert!(artifacts
        .iter()
        .any(|artifact| artifact.kind == ArtifactKind::Metadata && artifact.ready));
}

#[test]
fn mixed_recording_source_includes_mono_microphone_and_stereo_system_audio() {
    let microphone = FinalizedSource {
        samples: vec![1_000, 1_000],
        sample_rate: 48_000,
        channels: 1,
        frames: 2,
    };
    let system = FinalizedSource {
        samples: vec![100, 200, 300, 400],
        sample_rate: 48_000,
        channels: 2,
        frames: 2,
    };

    let mixed = mixed_recording_source(Some(&microphone), Some(&system)).unwrap();

    assert_eq!(mixed.sample_rate, 48_000);
    assert_eq!(mixed.channels, 2);
    assert_eq!(mixed.frames, 2);
    assert_eq!(mixed.samples, vec![1_100, 1_200, 1_300, 1_400]);
}

#[test]
fn mixed_recording_source_resamples_before_mixing() {
    let microphone = FinalizedSource {
        samples: vec![10, 20],
        sample_rate: 1_000,
        channels: 1,
        frames: 2,
    };
    let system = FinalizedSource {
        samples: vec![1, 2, 3, 4],
        sample_rate: 2_000,
        channels: 1,
        frames: 4,
    };

    let mixed = mixed_recording_source(Some(&microphone), Some(&system)).unwrap();

    assert_eq!(mixed.sample_rate, 2_000);
    assert_eq!(mixed.channels, 1);
    assert_eq!(mixed.frames, 4);
    assert_eq!(mixed.samples, vec![11, 12, 23, 24]);
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
    let devices = dedupe_capture_devices(vec![
        CaptureDeviceInfo {
            name: "Default microphone".to_owned(),
            label: "Default microphone".to_owned(),
            default: true,
        },
        CaptureDeviceInfo {
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
    repository
        .set_setting(
            "diarizationBackend",
            &serde_json::to_string(&DiarizationBackend::Pyannote).unwrap(),
        )
        .unwrap();
    repository
        .set_setting(
            "speakerCountMode",
            &serde_json::to_string(&SpeakerCountMode::Exact).unwrap(),
        )
        .unwrap();
    repository.set_setting("exactSpeakers", "1").unwrap();
    let mut observed_transcription_running = false;
    let mut observed_diarization_words = false;

    resume_pipeline_jobs(
        &mut repository,
        |command, payload| {
            let output_directory = std::path::PathBuf::from(
                payload
                    .get("outputDirectory")
                    .and_then(serde_json::Value::as_str)
                    .unwrap(),
            );

            match command {
                "transcribe.run" => {
                    fs::write(
                        raw_segments_path(&output_directory),
                        "{\"segments\":[{\"start\":0,\"end\":1,\"text\":\"Hello\"}]}\n",
                    )
                    .unwrap();
                    fs::write(
                        raw_words_path(&output_directory),
                        "{\"words\":[{\"segment_id\":0,\"text\":\"Hello\",\"start\":0,\"end\":1,\"probability\":0.95}]}\n",
                    )
                    .unwrap();
                    fs::write(raw_transcript_path(&output_directory), "# Raw\n\nHello\n").unwrap();

                    Ok(vec![worker_event(
                        "transcribe.complete",
                        serde_json::json!({
                            "segmentsPath": raw_segments_path(&output_directory),
                            "transcriptPath": raw_transcript_path(&output_directory),
                            "wordsPath": raw_words_path(&output_directory),
                        }),
                    )])
                }
                "diarize.run" => {
                    observed_diarization_words = payload
                        .get("words")
                        .and_then(serde_json::Value::as_array)
                        .and_then(|words| words.first())
                        .and_then(|word| word.get("text"))
                        .and_then(serde_json::Value::as_str)
                        == Some("Hello");
                    fs::write(
                        diarization_path(&output_directory),
                        "{\"turns\":[{\"speaker\":\"Speaker 1\",\"start\":0,\"end\":1}]}\n",
                    )
                    .unwrap();
                    fs::write(
                        diarized_transcript_path(&output_directory),
                        "# Diarized\n\nHello\n",
                    )
                    .unwrap();

                    Ok(vec![worker_event(
                        "diarize.complete",
                        serde_json::json!({
                            "diarizationPath": diarization_path(&output_directory),
                            "transcriptPath": diarized_transcript_path(&output_directory),
                        }),
                    )])
                }
                other => Err(format!("unexpected worker command: {other}")),
            }
        },
        |repository| {
            let job = repository
                .job_for_recording_stage(&recording.id, PipelineStageId::Transcription)
                .unwrap();
            observed_transcription_running =
                observed_transcription_running || job.status == PipelineStageStatus::Running;

            Ok(())
        },
    )
    .unwrap();
    let snapshot = repository.snapshot().unwrap();
    let recording = &snapshot.recordings[0];

    assert_eq!(recording.status, RecordingStatus::Complete);
    assert!(recording.stages.iter().any(|stage| {
        stage.id == PipelineStageId::Transcription && stage.status == PipelineStageStatus::Complete
    }));
    assert!(observed_transcription_running);
    let diarization_stage = recording
        .stages
        .iter()
        .find(|stage| stage.id == PipelineStageId::Diarization)
        .unwrap();
    assert_eq!(diarization_stage.status, PipelineStageStatus::Complete);
    assert!(recording.stages.iter().any(|stage| {
        stage.id == PipelineStageId::Summary && stage.status == PipelineStageStatus::Skipped
    }));
    assert!(recording
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == ArtifactKind::RawTranscript && artifact.ready));
    assert!(recording
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == ArtifactKind::DiarizedTranscript && artifact.ready));
    assert!(observed_diarization_words);
}

#[test]
fn resume_pipeline_sends_transcription_context_when_configured() {
    let database_path = test_database_path("pipeline-transcription-context");
    let artifact_path = test_artifact_path("pipeline-transcription-context-recording");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let mut capture_backend = FileAudioCaptureBackend::default();
    let recording = NewRecording {
        id: "recording-transcription-context".to_owned(),
        title: "Meeting".to_owned(),
        started_at: "1".to_owned(),
        artifact_directory: artifact_path.display().to_string(),
    };

    repository
        .set_setting("transcriptionContext", " ActaVoces \n\nKaneo\nActaVoces")
        .unwrap();
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

    let mut observed_transcription_context = None;

    resume_pipeline_jobs(
        &mut repository,
        |command, payload| match command {
            "transcribe.run" => {
                let output_directory = std::path::PathBuf::from(
                    payload
                        .get("outputDirectory")
                        .and_then(serde_json::Value::as_str)
                        .unwrap(),
                );
                observed_transcription_context = payload
                    .get("transcriptionContext")
                    .and_then(serde_json::Value::as_str)
                    .map(|context| context.to_owned());
                fs::write(
                    raw_segments_path(&output_directory),
                    "{\"segments\":[{\"start\":0,\"end\":1,\"text\":\"Hello\"}]}\n",
                )
                .unwrap();
                fs::write(raw_words_path(&output_directory), "{\"words\":[]}\n").unwrap();
                fs::write(raw_transcript_path(&output_directory), "# Raw\n\nHello\n").unwrap();

                Ok(vec![worker_event(
                    "transcribe.complete",
                    serde_json::json!({
                        "segmentsPath": raw_segments_path(&output_directory),
                        "transcriptPath": raw_transcript_path(&output_directory),
                        "wordsPath": raw_words_path(&output_directory),
                    }),
                )])
            }
            other => Err(format!("unexpected worker command: {other}")),
        },
        |_| Ok(()),
    )
    .unwrap();

    assert_eq!(
        observed_transcription_context,
        Some("ActaVoces\nKaneo".to_owned())
    );
}

#[test]
fn resume_pipeline_sends_snake_case_summary_payload_without_required_api_key() {
    let database_path = test_database_path("pipeline-summary-payload");
    let artifact_path = test_artifact_path("pipeline-summary-payload-recording");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let mut capture_backend = FileAudioCaptureBackend::default();
    let recording = NewRecording {
        id: "recording-summary-payload".to_owned(),
        title: "Meeting".to_owned(),
        started_at: "1".to_owned(),
        artifact_directory: artifact_path.display().to_string(),
    };

    repository.set_setting("summaryEnabled", "true").unwrap();
    repository
        .set_setting("providerBaseUrl", "http://localhost:11434/v1")
        .unwrap();
    repository.set_setting("providerModel", "llama3").unwrap();
    repository
        .set_setting(
            "diarizationBackend",
            &serde_json::to_string(&DiarizationBackend::Pyannote).unwrap(),
        )
        .unwrap();
    repository
        .set_setting(
            "speakerCountMode",
            &serde_json::to_string(&SpeakerCountMode::Exact).unwrap(),
        )
        .unwrap();
    repository.set_setting("exactSpeakers", "1").unwrap();
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

    let mut observed_summary_payload = None;

    resume_pipeline_jobs(
        &mut repository,
        |command, payload| {
            let output_directory_key = match command {
                "summarize.run" => "output_directory",
                _ => "outputDirectory",
            };
            let output_directory = std::path::PathBuf::from(
                payload
                    .get(output_directory_key)
                    .and_then(serde_json::Value::as_str)
                    .unwrap(),
            );

            match command {
                "transcribe.run" => {
                    fs::write(
                        raw_segments_path(&output_directory),
                        "{\"segments\":[{\"start\":0,\"end\":1,\"text\":\"Hello\"}]}\n",
                    )
                    .unwrap();
                    fs::write(raw_transcript_path(&output_directory), "# Raw\n\nHello\n").unwrap();

                    Ok(vec![worker_event(
                        "transcribe.complete",
                        serde_json::json!({
                            "segmentsPath": raw_segments_path(&output_directory),
                            "transcriptPath": raw_transcript_path(&output_directory),
                        }),
                    )])
                }
                "diarize.run" => {
                    fs::write(
                        diarization_path(&output_directory),
                        "{\"turns\":[{\"speaker\":\"Speaker 1\",\"start\":0,\"end\":1}]}\n",
                    )
                    .unwrap();
                    fs::write(
                        diarized_transcript_path(&output_directory),
                        "# Diarized\n\nHello\n",
                    )
                    .unwrap();

                    Ok(vec![worker_event(
                        "diarize.complete",
                        serde_json::json!({
                            "diarizationPath": diarization_path(&output_directory),
                            "transcriptPath": diarized_transcript_path(&output_directory),
                        }),
                    )])
                }
                "summarize.run" => {
                    observed_summary_payload = Some(payload);
                    fs::write(summary_path(&output_directory), "# Summary\n\nDone\n").unwrap();

                    Ok(vec![worker_event(
                        "summarize.complete",
                        serde_json::json!({
                            "summaryPath": summary_path(&output_directory),
                            "title": "Launch",
                        }),
                    )])
                }
                other => Err(format!("unexpected worker command: {other}")),
            }
        },
        |_| Ok(()),
    )
    .unwrap();

    let payload = observed_summary_payload.unwrap();

    assert_eq!(payload["provider_base_url"], "http://localhost:11434/v1");
    assert_eq!(payload["api_key"], "");
    assert_eq!(payload["model"], "llama3");
    assert!(payload.get("output_directory").is_some());
    assert!(payload.get("diarized_transcript_path").is_some());
    assert!(payload.get("transcript_path").is_some());
    assert!(payload.get("summary_prompt").is_some());
    assert!(payload.get("providerBaseUrl").is_none());
    assert!(payload.get("apiKey").is_none());
    assert!(payload.get("titlePrompt").is_none());

    let snapshot = repository.snapshot().unwrap();
    let recording = &snapshot.recordings[0];

    assert_eq!(recording.title, "Launch");
    assert!(Path::new(&recording.artifact_directory).ends_with("1970-01-01-0000-launch"));
    assert!(summary_path(Path::new(&recording.artifact_directory)).exists());
    assert!(fs::read_to_string(raw_transcript_path(Path::new(
        &recording.artifact_directory
    )))
    .unwrap()
    .starts_with("# Raw transcript - Launch"));
    assert!(fs::read_to_string(diarized_transcript_path(Path::new(
        &recording.artifact_directory
    )))
    .unwrap()
    .starts_with("# Diarized transcript - Launch"));
}

#[test]
fn speaker_label_rename_rewrites_diarization_artifacts() {
    let database_path = test_database_path("speaker-rename");
    let artifact_path = test_artifact_path("speaker-rename-artifacts");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let mut capture_backend = FileAudioCaptureBackend::default();
    let recording = NewRecording {
        id: "recording-speakers".to_owned(),
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
    fs::write(
        raw_segments_path(&artifact_path),
        "{\"segments\":[{\"start\":0,\"end\":4,\"text\":\"Hello there\"}]}\n",
    )
    .unwrap();
    fs::write(
        diarization_path(&artifact_path),
        "{\"turns\":[{\"speaker\":\"Speaker 1\",\"start\":0,\"end\":4}]}\n",
    )
    .unwrap();

    let recording = repository.recording_by_id(&recording.id).unwrap().unwrap();
    rewrite_speaker_label(
        &recording,
        &SpeakerRenameInput {
            recording_id: recording.id.clone(),
            speaker: "Speaker 1".to_owned(),
            replacement: "Alice".to_owned(),
        },
    )
    .unwrap();

    let diarization = fs::read_to_string(diarization_path(&artifact_path)).unwrap();
    let transcript = fs::read_to_string(diarized_transcript_path(&artifact_path)).unwrap();
    let snapshot = repository.snapshot().unwrap();

    assert!(diarization.contains("\"speaker\": \"Alice\""));
    assert!(transcript.contains("## Alice"));
    assert!(transcript.contains("Hello there"));
    assert_eq!(snapshot.recordings[0].speaker_labels[0].name, "Alice");
}

#[test]
fn recording_title_rename_moves_artifact_folder_and_updates_transcript_headers() {
    let database_path = test_database_path("title-rename");
    let artifact_path = test_artifact_path("title-rename-artifacts");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let mut capture_backend = FileAudioCaptureBackend::default();
    let recording = NewRecording {
        id: "recording-title".to_owned(),
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
    fs::write(
        raw_transcript_path(&artifact_path),
        "# Raw transcript - Meeting\n\nHello\n",
    )
    .unwrap();
    fs::write(
        diarized_transcript_path(&artifact_path),
        "# Diarized transcript - Meeting\n\nHello\n",
    )
    .unwrap();

    let recording = repository.recording_by_id(&recording.id).unwrap().unwrap();

    rename_recording_outputs(&mut repository, &recording, "Weekly Planning").unwrap();

    let snapshot = repository.snapshot().unwrap();
    let recording = &snapshot.recordings[0];
    let artifact_directory = Path::new(&recording.artifact_directory);

    assert_eq!(recording.title, "Weekly Planning");
    assert!(artifact_directory.ends_with("1970-01-01-0000-weekly-planning"));
    assert!(!artifact_path.exists());
    assert!(mixed_audio_path(artifact_directory).exists());
    assert!(fs::read_to_string(raw_transcript_path(artifact_directory))
        .unwrap()
        .starts_with("# Raw transcript - Weekly Planning"));
    assert!(
        fs::read_to_string(diarized_transcript_path(artifact_directory))
            .unwrap()
            .starts_with("# Diarized transcript - Weekly Planning")
    );
    assert!(recording
        .artifacts
        .iter()
        .all(|artifact| artifact.path.contains("1970-01-01-0000-weekly-planning")));
}

#[test]
fn shortcut_lifecycle_uses_recording_start_stop_flow() {
    let database_path = test_database_path("shortcut-lifecycle");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let mut capture_backend = FileAudioCaptureBackend::default();

    repository
        .update_settings(settings_update(
            test_artifact_path("shortcut-records").display().to_string(),
        ))
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
fn startup_clears_stale_active_recording_state() {
    let database_path = test_database_path("stale-active-recording");
    let mut repository = AppRepository::open(&database_path).unwrap();
    let mut capture_backend = FileAudioCaptureBackend::default();

    repository
        .update_settings(settings_update(
            test_artifact_path("stale-active-records")
                .display()
                .to_string(),
        ))
        .unwrap();
    start_recording_session(&mut repository, &mut capture_backend).unwrap();

    repository.clear_stale_active_recordings().unwrap();

    let snapshot = repository.snapshot().unwrap();
    let recording_job = snapshot
        .jobs
        .iter()
        .find(|job| job.stage == PipelineStageId::Recording)
        .unwrap();

    assert!(snapshot.active_recording.is_none());
    assert!(!snapshot.desktop.overlay_visible);
    assert_eq!(snapshot.recordings[0].status, RecordingStatus::Idle);
    assert_eq!(recording_job.status, PipelineStageStatus::Failed);
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
            worker_setup_status: WorkerSetupStatus::Ready,
            worker_setup_step: "Worker runtime ready".to_owned(),
            worker_setup_error: None,
            cuda_available: false,
            cuda_error: None,
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

#[test]
fn worker_source_hash_changes_when_worker_files_change() {
    let root = test_artifact_path("worker-source-hash");
    let app_directory = root.join("app");

    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&app_directory).unwrap();
    fs::write(app_directory.join("main.py"), "print('one')").unwrap();
    fs::write(root.join("pyproject.toml"), "[project]\nname='worker'\n").unwrap();
    fs::write(root.join("uv.lock"), "version = 1\n").unwrap();

    let first_hash = hash_worker_source_directory(&root).unwrap();

    fs::write(app_directory.join("main.py"), "print('two')").unwrap();

    let second_hash = hash_worker_source_directory(&root).unwrap();

    assert_ne!(first_hash, second_hash);
}

#[test]
fn worker_runtime_paths_use_local_app_data_layout() {
    let local_app_data = test_artifact_path("worker-local-runtime-layout");
    let ffmpeg_directory = local_app_data.join("runtime").join("ffmpeg");

    let paths = worker_runtime_paths_from_local_data_directory(
        local_app_data.clone(),
        Some(ffmpeg_directory.clone()),
    );

    assert_eq!(paths.worker_directory, local_app_data.join("worker"));
    assert_eq!(paths.uv_state_directory, local_app_data.join("uv"));
    assert_eq!(
        paths.uv_executable,
        local_app_data
            .join("runtime")
            .join("uv")
            .join(test_uv_executable_name())
    );
    assert_eq!(paths.ffmpeg_directory, Some(ffmpeg_directory));
}

#[test]
fn worker_uv_environment_is_scoped_to_app_paths() {
    let root = test_artifact_path("worker-uv-env");
    let paths = WorkerRuntimePaths {
        uv_executable: root.join("runtime").join("uv.exe"),
        worker_directory: root.join("worker"),
        uv_state_directory: root.join("uv"),
        ffmpeg_directory: None,
    };
    let mut command = Command::new("uv");

    apply_worker_current_dir(&mut command, &paths).unwrap();
    apply_worker_path_env(&mut command, &paths).unwrap();

    assert_eq!(
        command.get_current_dir(),
        Some(paths.worker_directory.as_path())
    );
    assert!(paths.worker_directory.exists());
    assert_eq!(
        command_env(&command, "UV_LINK_MODE"),
        Some("copy".to_owned())
    );
    assert_eq!(
        command_env(&command, "UV_CACHE_DIR"),
        Some(paths.uv_state_directory.join("cache").display().to_string())
    );
    assert_eq!(
        command_env(&command, "UV_PYTHON_CACHE_DIR"),
        Some(
            paths
                .uv_state_directory
                .join("python-cache")
                .display()
                .to_string()
        )
    );
    assert_eq!(
        command_env(&command, "UV_PYTHON_INSTALL_DIR"),
        Some(
            paths
                .uv_state_directory
                .join("python")
                .display()
                .to_string()
        )
    );
    assert_eq!(command_env(&command, "UV_NO_CONFIG"), Some("1".to_owned()));
    assert_eq!(
        command_env(&command, "UV_NO_SYSTEM_CONFIG"),
        Some("1".to_owned())
    );
    assert_eq!(
        command_env(&command, "UV_PROJECT_ENVIRONMENT"),
        Some(paths.worker_directory.join(".venv").display().to_string())
    );
}

#[test]
fn worker_python_resolution_uses_concrete_patch_installation() {
    let root = test_artifact_path("worker-python-resolution");
    let paths = WorkerRuntimePaths {
        uv_executable: root.join("runtime").join("uv.exe"),
        worker_directory: root.join("worker"),
        uv_state_directory: root.join("uv"),
        ffmpeg_directory: None,
    };
    let older = paths
        .uv_state_directory
        .join("python")
        .join("cpython-3.14.6-windows-x86_64-none");
    let newer = paths
        .uv_state_directory
        .join("python")
        .join("cpython-3.14.10-windows-x86_64-none");
    let unrelated_minor_link = paths
        .uv_state_directory
        .join("python")
        .join("cpython-3.14-windows-x86_64-none");
    let older_executable = create_test_python_executable(&older);
    let newer_executable = create_test_python_executable(&newer);
    create_test_python_executable(&unrelated_minor_link);

    let resolved = find_worker_python_executable(&paths).unwrap().unwrap();

    assert_eq!(resolved, newer_executable);
    assert_ne!(resolved, older_executable);
}

#[test]
fn worker_virtualenv_python_resolution_uses_synced_environment() {
    let root = test_artifact_path("worker-venv-python-resolution");
    let paths = WorkerRuntimePaths {
        uv_executable: root.join("runtime").join("uv.exe"),
        worker_directory: root.join("worker"),
        uv_state_directory: root.join("uv"),
        ffmpeg_directory: None,
    };
    let executable =
        create_test_virtualenv_python_executable(&paths.worker_directory.join(".venv"));

    let resolved = resolve_worker_virtualenv_python_executable(&paths).unwrap();

    assert_eq!(
        resolved,
        fs::canonicalize(executable.parent().unwrap())
            .unwrap()
            .join(executable.file_name().unwrap())
    );
}

fn settings_update(output_directory: String) -> AppSettingsUpdate {
    AppSettingsUpdate {
        output_directory,
        hotkey: "CommandOrControl+Shift+Space".to_owned(),
        overlay_position: OverlayPosition::TopLeft,
        overlay_display_mode: OverlayDisplayMode::Full,
        close_to_tray: true,
        launch_at_login: false,
        microphone_device: "Default microphone".to_owned(),
        system_audio_source: "Default system output".to_owned(),
        sample_rate: 48_000,
        whisper_model: "medium".to_owned(),
        transcription_language: "auto".to_owned(),
        transcription_context: String::new(),
        compute_type: "auto".to_owned(),
        model_storage_directory: test_artifact_path("models").display().to_string(),
        diarization_backend: DiarizationBackend::Pyannote,
        speaker_count_mode: SpeakerCountMode::Automatic,
        exact_speakers: None,
        min_speakers: None,
        max_speakers: None,
        hugging_face_token: None,
        diarization_setup_skipped: true,
        summary_enabled: false,
        provider_base_url: "https://api.openai.com/v1".to_owned(),
        provider_model: String::new(),
        provider_api_key: None,
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

fn command_env(command: &Command, key: &str) -> Option<String> {
    command
        .get_envs()
        .find(|(name, _)| *name == key)
        .and_then(|(_, value)| value.map(|value| value.to_string_lossy().into_owned()))
}

fn create_test_python_executable(directory: &Path) -> std::path::PathBuf {
    let executable = directory.join(test_python_executable_relative_path());
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::write(&executable, "python").unwrap();

    executable
}

fn create_test_virtualenv_python_executable(directory: &Path) -> std::path::PathBuf {
    let executable = directory.join(test_virtualenv_python_executable_relative_path());
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::write(&executable, "python").unwrap();

    executable
}

fn test_python_executable_relative_path() -> std::path::PathBuf {
    match cfg!(windows) {
        true => std::path::PathBuf::from("python.exe"),
        false => std::path::PathBuf::from("bin").join("python3.14"),
    }
}

fn test_virtualenv_python_executable_relative_path() -> std::path::PathBuf {
    match cfg!(windows) {
        true => std::path::PathBuf::from("Scripts").join("python.exe"),
        false => std::path::PathBuf::from("bin").join("python"),
    }
}

fn test_uv_executable_name() -> &'static str {
    match cfg!(windows) {
        true => "uv.exe",
        false => "uv",
    }
}

fn test_database_path(name: &str) -> std::path::PathBuf {
    let directory = test_artifact_path(name);

    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();

    directory.join("actavoces.sqlite")
}

fn test_artifact_path(name: &str) -> std::path::PathBuf {
    let counter = TEST_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);

    env::temp_dir()
        .join("actavoces-tests")
        .join(format!("{}-{counter}", std::process::id()))
        .join(name)
}
