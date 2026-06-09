import { useComputed, useSignal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import {
  getAppSnapshot,
  resumePendingJobs,
  startRecording,
  stopRecording
} from '../../../services/desktop/app.service';
import { appErrorSignal, appSnapshotSignal } from '../../../stores/app.store';
import type { Recording } from '../../../types/desktop';
import { formatDuration, formatTimestamp } from '../../../utils/format';

const resolveLatestRecording = (recordings: Recording[]): Recording | null =>
  recordings.length > 0 ? recordings[0] : null;

export const useApp = () => {
  const loading = useSignal(false);

  const loadSnapshot = async () => {
    loading.value = true;
    appErrorSignal.value = null;

    try {
      appSnapshotSignal.value = await getAppSnapshot();
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Desktop backend unavailable';
    } finally {
      loading.value = false;
    }
  };

  const handleStartRecording = async () => {
    loading.value = true;
    appErrorSignal.value = null;

    try {
      appSnapshotSignal.value = await startRecording();
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to start recording';
    } finally {
      loading.value = false;
    }
  };

  const handleStopRecording = async () => {
    loading.value = true;
    appErrorSignal.value = null;

    try {
      appSnapshotSignal.value = await stopRecording();
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to stop recording';
    } finally {
      loading.value = false;
    }
  };

  const handleResumeJobs = async () => {
    loading.value = true;
    appErrorSignal.value = null;

    try {
      appSnapshotSignal.value = await resumePendingJobs();
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to resume jobs';
    } finally {
      loading.value = false;
    }
  };

  useEffect(() => {
    void loadSnapshot();
  }, []);

  const latestRecording = useComputed(() =>
    resolveLatestRecording(appSnapshotSignal.value.recordings)
  );

  return {
    data: {
      snapshot: appSnapshotSignal,
      latestRecording,
      formatDuration,
      formatTimestamp
    },
    status: {
      loading,
      error: appErrorSignal,
      isRecording: useComputed(() => appSnapshotSignal.value.activeRecording !== null)
    },
    actions: {
      startRecording: handleStartRecording,
      stopRecording: handleStopRecording,
      resumeJobs: handleResumeJobs
    }
  };
};
