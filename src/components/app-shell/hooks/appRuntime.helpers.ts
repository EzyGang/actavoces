import type { AppRoute } from '../../../stores/route.store';
import type { WorkerSetupProgress } from '../../../types/desktop';

export const routeLabel: Record<AppRoute, string> = {
  dashboard: 'Dashboard',
  recordings: 'Recordings',
  jobs: 'Jobs',
  settings: 'Settings'
};

export const initialSetupProgress: WorkerSetupProgress = {
  status: 'missing',
  step: 'Preparing local worker runtime',
  error: null
};

export const isTauriRuntime = () => '__TAURI_INTERNALS__' in window;

export const errorMessage = (error: unknown, fallback: string): string => {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === 'string' && error.trim().length > 0) {
    return error;
  }

  if (error && typeof error === 'object') {
    return JSON.stringify(error);
  }

  return fallback;
};

export const diagnosticsMessage = (error: unknown): string => {
  if (error instanceof Error) {
    return error.stack ?? error.message;
  }

  return errorMessage(error, 'Unknown frontend error');
};
