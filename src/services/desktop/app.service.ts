import { invoke } from '@tauri-apps/api/core';
import type { AppSnapshot } from '../../types/desktop';

export const getAppSnapshot = () => invoke<AppSnapshot>('get_app_snapshot');

export const startRecording = () => invoke<AppSnapshot>('start_recording');

export const stopRecording = () => invoke<AppSnapshot>('stop_recording');

export const resumePendingJobs = () => invoke<AppSnapshot>('resume_pending_jobs');
