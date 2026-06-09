export type RecordingStatus = 'idle' | 'recording' | 'processing';

export type PipelineStageId =
  | 'recording'
  | 'transcription'
  | 'alignment'
  | 'diarization'
  | 'summary';

export type PipelineStageStatus = 'pending' | 'running' | 'complete' | 'failed' | 'needsSetup';

export type ArtifactKind =
  | 'audio'
  | 'rawTranscript'
  | 'segments'
  | 'diarization'
  | 'diarizedTranscript'
  | 'summary'
  | 'jobLog';

export interface PipelineStage {
  id: PipelineStageId;
  label: string;
  status: PipelineStageStatus;
  progress: number;
}

export interface Artifact {
  kind: ArtifactKind;
  label: string;
  path: string;
  ready: boolean;
}

export interface Recording {
  id: string;
  title: string;
  startedAt: string;
  endedAt: string | null;
  durationSeconds: number | null;
  status: RecordingStatus;
  stages: PipelineStage[];
  artifacts: Artifact[];
}

export interface AppSettings {
  outputDirectory: string;
  hotkey: string;
  whisperModel: string;
  diarizationBackend: 'nemoWhisper' | 'pyannote';
  summaryProviderConfigured: boolean;
}

export interface AppSnapshot {
  activeRecording: Recording | null;
  recordings: Recording[];
  settings: AppSettings;
}
