export type RecordingStatus = 'idle' | 'recording' | 'processing' | 'complete';

export type PipelineStageId = 'recording' | 'transcription' | 'diarization' | 'summary';

export type PipelineStageStatus =
  | 'pending'
  | 'running'
  | 'complete'
  | 'failed'
  | 'needsSetup'
  | 'skipped';

export type ArtifactKind =
  | 'audio'
  | 'microphoneAudio'
  | 'systemAudio'
  | 'rawTranscript'
  | 'segments'
  | 'rawWords'
  | 'diarization'
  | 'diarizedTranscript'
  | 'summary'
  | 'metadata'
  | 'jobLog';

export interface PipelineStage {
  id: PipelineStageId;
  label: string;
  status: PipelineStageStatus;
  progress: number;
  message: string;
}

export interface Artifact {
  kind: ArtifactKind;
  label: string;
  path: string;
  ready: boolean;
}

export interface CaptureError {
  source: 'microphone' | 'system';
  message: string;
}

export interface PipelineJob {
  id: string;
  recordingId: string;
  stage: PipelineStageId;
  status: PipelineStageStatus;
  progress: number;
  message: string;
}

export interface ModelInventoryItem {
  name: string;
  installed: boolean;
  setupRequired: boolean;
  dependency: string;
}

export interface CaptureDeviceInfo {
  name: string;
  label: string;
  default: boolean;
}

export interface CaptureDevices {
  microphones: CaptureDeviceInfo[];
  systemSources: CaptureDeviceInfo[];
}

export interface Recording {
  id: string;
  title: string;
  startedAt: string;
  endedAt: string | null;
  durationSeconds: number | null;
  status: RecordingStatus;
  artifactDirectory: string;
  captureErrors: CaptureError[];
  stages: PipelineStage[];
  artifacts: Artifact[];
  speakerLabels: SpeakerLabel[];
}

export interface SpeakerLabel {
  name: string;
}

export interface AppSettings {
  outputDirectory: string;
  databasePath: string;
  hotkey: string;
  overlayPosition: 'topLeft' | 'topRight' | 'bottomLeft' | 'bottomRight';
  overlayDisplayMode: 'full' | 'minimal' | 'none';
  closeToTray: boolean;
  launchAtLogin: boolean;
  microphoneDevice: string;
  systemAudioSource: string;
  sampleRate: number;
  whisperModel: string;
  transcriptionLanguage: string;
  computeType: string;
  modelStorageDirectory: string;
  diarizationBackend: 'pyannote' | 'sortformer';
  speakerCountMode: 'automatic' | 'exact' | 'range';
  exactSpeakers: number | null;
  minSpeakers: number | null;
  maxSpeakers: number | null;
  huggingFaceTokenConfigured: boolean;
  diarizationSetupSkipped: boolean;
  diarizationRuntimeReady: boolean;
  summaryProviderConfigured: boolean;
  providerApiKeyConfigured: boolean;
  summaryEnabled: boolean;
  providerBaseUrl: string;
  providerModel: string;
  summaryPrompt: string;
}

export type AppSettingsUpdate = Omit<
  AppSettings,
  | 'databasePath'
  | 'summaryProviderConfigured'
  | 'providerApiKeyConfigured'
  | 'huggingFaceTokenConfigured'
  | 'diarizationRuntimeReady'
> & {
  providerApiKey: string;
  huggingFaceToken: string;
};

export interface DesktopRuntimeStatus {
  overlayVisible: boolean;
  hotkeyRegistered: boolean;
  hotkeyError: string | null;
  workerRunning: boolean;
  workerHealthOk: boolean;
  workerError: string | null;
  workerSetupStatus: WorkerSetupStatus;
  workerSetupStep: string;
  workerSetupError: string | null;
  cudaAvailable: boolean;
  cudaError: string | null;
}

export type WorkerSetupStatus = 'missing' | 'installing' | 'ready' | 'failed';

export interface WorkerSetupProgress {
  status: WorkerSetupStatus;
  step: string;
  error: string | null;
}

export type SortformerSetupStatus = 'downloading' | 'ready' | 'failed';

export interface SortformerSetupProgress {
  status: SortformerSetupStatus;
  step: string;
  progress: number | null;
  error: string | null;
}

export interface WorkerStatus {
  running: boolean;
  healthOk: boolean;
  lastError: string | null;
  mode: 'cliJsonl';
}

export interface DiagnosticLogInput {
  event: string;
  message: string;
}

export interface AppSnapshot {
  activeRecording: Recording | null;
  recordings: Recording[];
  jobs: PipelineJob[];
  models: ModelInventoryItem[];
  captureDevices: CaptureDevices;
  desktop: DesktopRuntimeStatus;
  settings: AppSettings;
}
