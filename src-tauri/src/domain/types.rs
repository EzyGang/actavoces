use std::sync::{Mutex, MutexGuard, OnceLock};

use serde::{Deserialize, Serialize};

use crate::capture::audio::NativeAudioCaptureBackend;
use crate::storage::repository::AppRepository;
use crate::worker::runtime::WorkerRuntimeState;
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub(crate) active_recording: Option<Recording>,
    pub(crate) recordings: Vec<Recording>,
    pub(crate) jobs: Vec<PipelineJob>,
    pub(crate) models: Vec<ModelInventoryItem>,
    pub(crate) capture_devices: CaptureDevices,
    pub(crate) desktop: DesktopRuntimeStatus,
    pub(crate) settings: AppSettings,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDevices {
    pub(crate) microphones: Vec<CaptureDeviceInfo>,
    pub(crate) system_sources: Vec<CaptureDeviceInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDeviceInfo {
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) default: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRuntimeStatus {
    pub(crate) overlay_visible: bool,
    pub(crate) hotkey_registered: bool,
    pub(crate) hotkey_error: Option<String>,
    pub(crate) worker_running: bool,
    pub(crate) worker_health_ok: bool,
    pub(crate) worker_error: Option<String>,
    pub(crate) worker_setup_status: WorkerSetupStatus,
    pub(crate) worker_setup_step: String,
    pub(crate) worker_setup_error: Option<String>,
    pub(crate) cuda_available: bool,
    pub(crate) cuda_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerStatus {
    pub(crate) running: bool,
    pub(crate) health_ok: bool,
    pub(crate) last_error: Option<String>,
    pub(crate) mode: WorkerMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkerMode {
    CliJsonl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkerSetupStatus {
    Missing,
    Installing,
    Ready,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSetupProgress {
    pub(crate) status: WorkerSetupStatus,
    pub(crate) step: String,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortformerSetupStatus {
    Downloading,
    Ready,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortformerSetupProgress {
    pub(crate) status: SortformerSetupStatus,
    pub(crate) step: String,
    pub(crate) progress: Option<u8>,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerEvent {
    pub(crate) command_id: String,
    pub(crate) event: String,
    pub(crate) payload: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticLogInput {
    pub(crate) event: String,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInventoryItem {
    pub(crate) name: String,
    pub(crate) installed: bool,
    pub(crate) setup_required: bool,
    pub(crate) dependency: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilities {
    pub(crate) faster_whisper_available: bool,
    pub(crate) cuda_available: bool,
    pub(crate) cuda_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) started_at: String,
    pub(crate) ended_at: Option<String>,
    pub(crate) duration_seconds: Option<u64>,
    pub(crate) status: RecordingStatus,
    pub(crate) artifact_directory: String,
    pub(crate) capture_errors: Vec<CaptureError>,
    pub(crate) stages: Vec<PipelineStage>,
    pub(crate) artifacts: Vec<Artifact>,
    pub(crate) speaker_labels: Vec<SpeakerLabel>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordingStatus {
    Idle,
    Recording,
    Processing,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PipelineStageId {
    Recording,
    Transcription,
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
    Skipped,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStage {
    pub(crate) id: PipelineStageId,
    pub(crate) label: String,
    pub(crate) status: PipelineStageStatus,
    pub(crate) progress: u8,
    pub(crate) message: String,
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
    pub(crate) kind: ArtifactKind,
    pub(crate) label: String,
    pub(crate) path: String,
    pub(crate) ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineJob {
    pub(crate) id: String,
    pub(crate) recording_id: String,
    pub(crate) stage: PipelineStageId,
    pub(crate) status: PipelineStageStatus,
    pub(crate) progress: u8,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureError {
    pub(crate) source: CaptureSource,
    pub(crate) message: String,
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
    pub(crate) output_directory: String,
    pub(crate) database_path: String,
    pub(crate) hotkey: String,
    pub(crate) overlay_position: OverlayPosition,
    pub(crate) overlay_display_mode: OverlayDisplayMode,
    pub(crate) close_to_tray: bool,
    pub(crate) launch_at_login: bool,
    pub(crate) microphone_device: String,
    pub(crate) system_audio_source: String,
    pub(crate) sample_rate: u32,
    pub(crate) whisper_model: String,
    pub(crate) transcription_language: String,
    pub(crate) compute_type: String,
    pub(crate) model_storage_directory: String,
    pub(crate) diarization_backend: DiarizationBackend,
    pub(crate) speaker_count_mode: SpeakerCountMode,
    pub(crate) exact_speakers: Option<u8>,
    pub(crate) min_speakers: Option<u8>,
    pub(crate) max_speakers: Option<u8>,
    pub(crate) hugging_face_token_configured: bool,
    pub(crate) diarization_setup_skipped: bool,
    pub(crate) diarization_runtime_ready: bool,
    pub(crate) summary_provider_configured: bool,
    pub(crate) provider_api_key_configured: bool,
    pub(crate) summary_enabled: bool,
    pub(crate) provider_base_url: String,
    pub(crate) provider_model: String,
    pub(crate) title_prompt: String,
    pub(crate) summary_prompt: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsUpdate {
    pub(crate) output_directory: String,
    pub(crate) hotkey: String,
    pub(crate) overlay_position: OverlayPosition,
    pub(crate) overlay_display_mode: OverlayDisplayMode,
    pub(crate) close_to_tray: bool,
    pub(crate) launch_at_login: bool,
    pub(crate) microphone_device: String,
    pub(crate) system_audio_source: String,
    pub(crate) sample_rate: u32,
    pub(crate) whisper_model: String,
    pub(crate) transcription_language: String,
    pub(crate) compute_type: String,
    pub(crate) model_storage_directory: String,
    pub(crate) diarization_backend: DiarizationBackend,
    pub(crate) speaker_count_mode: SpeakerCountMode,
    pub(crate) exact_speakers: Option<u8>,
    pub(crate) min_speakers: Option<u8>,
    pub(crate) max_speakers: Option<u8>,
    pub(crate) hugging_face_token: Option<String>,
    pub(crate) diarization_setup_skipped: bool,
    pub(crate) summary_enabled: bool,
    pub(crate) provider_base_url: String,
    pub(crate) provider_model: String,
    pub(crate) provider_api_key: Option<String>,
    pub(crate) title_prompt: String,
    pub(crate) summary_prompt: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInstallInput {
    pub(crate) model: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiarizationSetupInput {
    pub(crate) hugging_face_token: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverlayPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverlayDisplayMode {
    Full,
    Minimal,
    None,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenPathInput {
    pub(crate) path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingDeleteInput {
    pub(crate) recording_id: String,
    pub(crate) delete_artifacts: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingRetryInput {
    pub(crate) recording_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingRenameInput {
    pub(crate) recording_id: String,
    pub(crate) title: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerLabel {
    pub(crate) name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerRenameInput {
    pub(crate) recording_id: String,
    pub(crate) speaker: String,
    pub(crate) replacement: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiarizationBackend {
    Pyannote,
    Sortformer,
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
    pub(crate) repository: OnceLock<Mutex<AppRepository>>,
    pub(crate) capture_backend: Mutex<NativeAudioCaptureBackend>,
    pub(crate) worker_runtime: Mutex<WorkerRuntimeState>,
    pub(crate) pipeline_running: Mutex<bool>,
}

impl ActavocesState {
    pub(crate) fn repository(&self) -> Result<MutexGuard<'_, AppRepository>, String> {
        self.repository
            .get()
            .ok_or_else(|| "ActaVoces is still starting".to_owned())?
            .lock()
            .map_err(|error| error.to_string())
    }
}
