import { invoke } from '@tauri-apps/api/core';
import type { AppSettingsUpdate, AppSnapshot, WorkerStatus } from '../../types/desktop';

export const getAppSnapshot = () => invoke<AppSnapshot>('get_app_snapshot');

export const updateAppSettings = (input: AppSettingsUpdate) =>
  invoke<AppSnapshot>('update_app_settings', { input });

export const clearSummaryProviderApiKey = () =>
  invoke<AppSnapshot>('clear_summary_provider_api_key');

export const clearHuggingFaceToken = () => invoke<AppSnapshot>('clear_hugging_face_token');

export const setupDiarizationRuntime = (huggingFaceToken: string) =>
  invoke<AppSnapshot>('setup_diarization_runtime', {
    input: { huggingFaceToken }
  });

export const skipDiarizationSetup = () => invoke<AppSnapshot>('skip_diarization_setup');

export const startRecording = () => invoke<AppSnapshot>('start_recording');

export const stopRecording = () => invoke<AppSnapshot>('stop_recording');

export const deleteRecording = (recordingId: string, deleteArtifacts = true) =>
  invoke<AppSnapshot>('delete_recording', {
    input: { recordingId, deleteArtifacts }
  });

export const openLocalPath = (path: string) =>
  invoke<void>('open_local_path', {
    input: { path }
  });

export const retryRecordingJobs = (recordingId: string) =>
  invoke<AppSnapshot>('retry_recording_jobs', {
    input: { recordingId }
  });

export const renameSpeakerLabel = (recordingId: string, speaker: string, replacement: string) =>
  invoke<AppSnapshot>('rename_speaker_label', {
    input: { recordingId, speaker, replacement }
  });

export const toggleRecordingFromShortcut = () =>
  invoke<AppSnapshot>('toggle_recording_from_shortcut');

export const resumePendingJobs = () => invoke<AppSnapshot>('resume_pending_jobs');

export const bootstrapWorkerRuntime = () => invoke<AppSnapshot>('bootstrap_worker_runtime');

export const refreshModelInventory = () => invoke<AppSnapshot>('refresh_model_inventory');

export const installTranscriptionModel = (model: string) =>
  invoke<AppSnapshot>('install_transcription_model', { input: { model } });

export const getWorkerStatus = () => invoke<WorkerStatus>('get_worker_status');

export const startWorker = () => invoke<WorkerStatus>('start_worker');

export const stopWorker = () => invoke<WorkerStatus>('stop_worker');

export const checkWorkerHealth = () => invoke<WorkerStatus>('check_worker_health');
