use std::env;
use std::fs;
use std::path::Path;

use crate::app::commands::{
    resume_pipeline_jobs, rewrite_speaker_label, start_recording_session, stop_recording_session,
};
use crate::artifacts::{artifact_directory, capture_artifacts_with_readiness};
use crate::capture::audio::{
    dedupe_capture_devices, is_default_system_source_name, is_system_monitor_device_name,
    mixed_recording_source, AudioCaptureBackend, FileAudioCaptureBackend, FinalizedSource,
};
use crate::domain::types::{
    AppSettingsUpdate, ArtifactKind, CaptureDeviceInfo, DesktopRuntimeStatus, DiarizationBackend,
    ModelInventoryItem, OverlayDisplayMode, OverlayPosition, PipelineStageId, PipelineStageStatus,
    RecordingStatus, SpeakerCountMode, SpeakerRenameInput, WorkerEvent, WorkerSetupStatus,
};
use crate::settings::default_settings;
use crate::storage::repository::{AppRepository, NewRecording};
use crate::utils::default_records_root;
use crate::worker::runtime::{
    extract_model_inventory, hash_worker_source_directory, parse_worker_events, WorkerRuntimeState,
};

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
        Path::new("/tmp/records")
            .join("2024")
            .join("06")
            .join("2024-06-09-130012-untitled-meeting")
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

    let mixed_file_path = artifact_path.join("recording.wav");
    let microphone_file_path = artifact_path.join("microphone.wav");

    assert!(mixed_file_path.exists());
    assert!(microphone_file_path.exists());
    assert!(fs::metadata(mixed_file_path).unwrap().len() > 44);
    assert!(fs::metadata(microphone_file_path).unwrap().len() > 44);
    assert!(!artifact_path.join("system.wav").exists());
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
        artifact_path.join("raw-segments.json"),
        "{\"segments\":[{\"start\":0,\"end\":4,\"text\":\"Hello there\"}]}\n",
    )
    .unwrap();
    fs::write(
        artifact_path.join("diarization.json"),
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

    let diarization = fs::read_to_string(artifact_path.join("diarization.json")).unwrap();
    let transcript = fs::read_to_string(artifact_path.join("diarized-transcript.md")).unwrap();
    let snapshot = repository.snapshot().unwrap();

    assert!(diarization.contains("\"speaker\": \"Alice\""));
    assert!(transcript.contains("## Alice"));
    assert!(transcript.contains("Hello there"));
    assert_eq!(snapshot.recordings[0].speaker_labels[0].name, "Alice");
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
