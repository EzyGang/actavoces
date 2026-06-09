use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    active_recording: Option<Recording>,
    recordings: Vec<Recording>,
    settings: AppSettings,
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
    stages: Vec<PipelineStage>,
    artifacts: Vec<Artifact>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordingStatus {
    Idle,
    Recording,
    Processing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PipelineStageId {
    Recording,
    Transcription,
    Alignment,
    Diarization,
    Summary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKind {
    Audio,
    RawTranscript,
    Segments,
    Diarization,
    DiarizedTranscript,
    Summary,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    output_directory: String,
    hotkey: String,
    whisper_model: String,
    diarization_backend: DiarizationBackend,
    summary_provider_configured: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiarizationBackend {
    NemoWhisper,
    Pyannote,
}

#[derive(Debug)]
pub struct ActavocesState {
    snapshot: Mutex<AppSnapshot>,
}

#[tauri::command]
fn get_app_snapshot(state: tauri::State<'_, ActavocesState>) -> Result<AppSnapshot, String> {
    state
        .snapshot
        .lock()
        .map(|snapshot| snapshot.clone())
        .map_err(lock_error)
}

#[tauri::command]
fn start_recording(state: tauri::State<'_, ActavocesState>) -> Result<AppSnapshot, String> {
    let mut snapshot = state.snapshot.lock().map_err(lock_error)?;

    if snapshot.active_recording.is_some() {
        return Err("A recording is already active".to_owned());
    }

    let now = unix_timestamp();
    snapshot.active_recording = Some(Recording {
        id: format!("recording-{now}"),
        title: "Untitled meeting".to_owned(),
        started_at: now.to_string(),
        ended_at: None,
        duration_seconds: None,
        status: RecordingStatus::Recording,
        stages: recording_stages(),
        artifacts: Vec::new(),
    });

    Ok(snapshot.clone())
}

#[tauri::command]
fn stop_recording(state: tauri::State<'_, ActavocesState>) -> Result<AppSnapshot, String> {
    let mut snapshot = state.snapshot.lock().map_err(lock_error)?;
    let mut recording = snapshot
        .active_recording
        .take()
        .ok_or_else(|| "No recording is active".to_owned())?;
    let ended_at = unix_timestamp();
    let started_at = recording.started_at.parse::<u64>().unwrap_or(ended_at);
    let duration_seconds = ended_at.saturating_sub(started_at);
    let artifact_dir = artifact_directory(&snapshot.settings, &recording.id);

    fs::create_dir_all(&artifact_dir).map_err(|error| error.to_string())?;
    write_initial_artifacts(&artifact_dir, &recording)?;

    recording.ended_at = Some(ended_at.to_string());
    recording.duration_seconds = Some(duration_seconds);
    recording.status = RecordingStatus::Processing;
    recording.title = format!("Meeting {}", recording.id.replace("recording-", ""));
    recording.stages = transcribed_stages();
    recording.artifacts = initial_artifacts(&artifact_dir);

    snapshot.recordings.insert(0, recording);

    Ok(snapshot.clone())
}

#[tauri::command]
fn resume_pending_jobs(state: tauri::State<'_, ActavocesState>) -> Result<AppSnapshot, String> {
    let mut snapshot = state.snapshot.lock().map_err(lock_error)?;
    let settings = snapshot.settings.clone();

    for recording in &mut snapshot.recordings {
        if recording.status == RecordingStatus::Processing {
            let artifact_dir = artifact_directory(&settings, &recording.id);
            fs::create_dir_all(&artifact_dir).map_err(|error| error.to_string())?;
            write_completed_artifacts(&artifact_dir, recording)?;
            recording.stages = completed_stages();
            recording.artifacts = completed_artifacts(&artifact_dir);
            recording.status = RecordingStatus::Idle;
        }
    }

    Ok(snapshot.clone())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ActavocesState {
            snapshot: Mutex::new(default_snapshot()),
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_snapshot,
            start_recording,
            stop_recording,
            resume_pending_jobs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn default_snapshot() -> AppSnapshot {
    AppSnapshot {
        active_recording: None,
        recordings: Vec::new(),
        settings: AppSettings {
            output_directory: "Actavoces".to_owned(),
            hotkey: "CommandOrControl+Shift+Space".to_owned(),
            whisper_model: "medium.en".to_owned(),
            diarization_backend: DiarizationBackend::NemoWhisper,
            summary_provider_configured: false,
        },
    }
}

fn recording_stages() -> Vec<PipelineStage> {
    vec![
        stage(
            PipelineStageId::Recording,
            "Capture",
            PipelineStageStatus::Running,
            12,
        ),
        stage(
            PipelineStageId::Transcription,
            "Raw transcript",
            PipelineStageStatus::Pending,
            0,
        ),
        stage(
            PipelineStageId::Alignment,
            "Alignment",
            PipelineStageStatus::Pending,
            0,
        ),
        stage(
            PipelineStageId::Diarization,
            "Diarization",
            PipelineStageStatus::Pending,
            0,
        ),
        stage(
            PipelineStageId::Summary,
            "Summary",
            PipelineStageStatus::Pending,
            0,
        ),
    ]
}

fn transcribed_stages() -> Vec<PipelineStage> {
    vec![
        stage(
            PipelineStageId::Recording,
            "Capture",
            PipelineStageStatus::Complete,
            100,
        ),
        stage(
            PipelineStageId::Transcription,
            "Raw transcript",
            PipelineStageStatus::Complete,
            100,
        ),
        stage(
            PipelineStageId::Alignment,
            "Alignment",
            PipelineStageStatus::Running,
            45,
        ),
        stage(
            PipelineStageId::Diarization,
            "Diarization",
            PipelineStageStatus::Pending,
            0,
        ),
        stage(
            PipelineStageId::Summary,
            "Summary",
            PipelineStageStatus::Pending,
            0,
        ),
    ]
}

fn completed_stages() -> Vec<PipelineStage> {
    vec![
        stage(
            PipelineStageId::Recording,
            "Capture",
            PipelineStageStatus::Complete,
            100,
        ),
        stage(
            PipelineStageId::Transcription,
            "Raw transcript",
            PipelineStageStatus::Complete,
            100,
        ),
        stage(
            PipelineStageId::Alignment,
            "Alignment",
            PipelineStageStatus::Complete,
            100,
        ),
        stage(
            PipelineStageId::Diarization,
            "Diarization",
            PipelineStageStatus::Complete,
            100,
        ),
        stage(
            PipelineStageId::Summary,
            "Summary",
            PipelineStageStatus::Complete,
            100,
        ),
    ]
}

fn stage(
    id: PipelineStageId,
    label: &str,
    status: PipelineStageStatus,
    progress: u8,
) -> PipelineStage {
    PipelineStage {
        id,
        label: label.to_owned(),
        status,
        progress,
    }
}

fn initial_artifacts(path: &Path) -> Vec<Artifact> {
    vec![
        artifact(
            ArtifactKind::Audio,
            "Mixed WAV",
            path.join("recording.wav"),
            true,
        ),
        artifact(
            ArtifactKind::RawTranscript,
            "Raw transcript",
            path.join("raw-transcript.md"),
            true,
        ),
        artifact(
            ArtifactKind::Segments,
            "Raw segments",
            path.join("raw-segments.json"),
            true,
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
    ]
}

fn completed_artifacts(path: &Path) -> Vec<Artifact> {
    vec![
        artifact(
            ArtifactKind::Audio,
            "Mixed WAV",
            path.join("recording.wav"),
            true,
        ),
        artifact(
            ArtifactKind::RawTranscript,
            "Raw transcript",
            path.join("raw-transcript.md"),
            true,
        ),
        artifact(
            ArtifactKind::Segments,
            "Raw segments",
            path.join("raw-segments.json"),
            true,
        ),
        artifact(
            ArtifactKind::Diarization,
            "Diarization turns",
            path.join("diarization.json"),
            true,
        ),
        artifact(
            ArtifactKind::DiarizedTranscript,
            "Diarized transcript",
            path.join("diarized-transcript.md"),
            true,
        ),
        artifact(
            ArtifactKind::Summary,
            "Summary",
            path.join("summary.md"),
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

fn write_initial_artifacts(path: &Path, recording: &Recording) -> Result<(), String> {
    fs::write(path.join("recording.wav"), b"").map_err(|error| error.to_string())?;
    fs::write(
        path.join("raw-transcript.md"),
        format!(
            "# Raw transcript\n\nRecording `{}` has been captured. Native audio capture and faster-whisper processing are the next backend implementations.\n",
            recording.id
        ),
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        path.join("raw-segments.json"),
        format!("{{\"recordingId\":\"{}\",\"segments\":[]}}\n", recording.id),
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        path.join("job-log.jsonl"),
        "{\"stage\":\"transcription\",\"status\":\"complete\"}\n",
    )
    .map_err(|error| error.to_string())
}

fn write_completed_artifacts(path: &Path, recording: &Recording) -> Result<(), String> {
    fs::write(
        path.join("diarization.json"),
        format!(
            "{{\"recordingId\":\"{}\",\"backend\":\"nemoWhisper\",\"turns\":[]}}\n",
            recording.id
        ),
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        path.join("diarized-transcript.md"),
        "# Diarized transcript\n\nSpeaker labels will appear here once the NeMo worker backend is connected.\n",
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        path.join("summary.md"),
        "# Summary\n\nConfigure an OpenAI-compatible provider to generate summaries.\n",
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        path.join("job-log.jsonl"),
        "{\"stage\":\"transcription\",\"status\":\"complete\"}\n{\"stage\":\"diarization\",\"status\":\"complete\"}\n{\"stage\":\"summary\",\"status\":\"complete\"}\n",
    )
    .map_err(|error| error.to_string())
}

fn artifact_directory(settings: &AppSettings, recording_id: &str) -> PathBuf {
    PathBuf::from(&settings.output_directory).join(recording_id)
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
