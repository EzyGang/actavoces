import { type Signal, useComputed, useSignal } from '@preact/signals';
import type { JSX } from 'preact';
import {
  deleteRecording,
  openLocalPath,
  renameSpeakerLabel,
  retryRecordingJobs,
  startRecording,
  stopRecording,
  toggleRecordingFromShortcut
} from '../../../services/desktop/app.service';
import { appSnapshotSignal } from '../../../stores/app.store';
import type { AppSnapshot, Recording } from '../../../types/desktop';
import { errorMessage } from '../../app-shell/hooks/appRuntime.helpers';
import {
  canRetryRecording,
  recordingPipelineStatus,
  recordingProgress,
  resolveLatestRecording
} from './recording.helpers';

interface SpeakerLabelRow {
  name: string;
  isRenaming: boolean;
  renameValue: string;
  onStartRename: () => void;
  onRenameInput: JSX.InputEventHandler<HTMLInputElement>;
  onRenameSubmit: JSX.GenericEventHandler<HTMLFormElement>;
  onCancelRename: () => void;
}

interface UseRecordingsInput {
  loading: Signal<boolean>;
  setError: (message: string | null) => void;
  setSnapshot: (snapshot: AppSnapshot) => void;
}

export const useRecordings = ({ loading, setError, setSnapshot }: UseRecordingsInput) => {
  const speakerRenameTarget = useSignal<{ recordingId: string; speaker: string } | null>(null);
  const speakerRenameDraft = useSignal('');

  const start = async () => {
    loading.value = true;
    setError(null);

    try {
      setSnapshot(await startRecording());
    } catch (error) {
      setError(errorMessage(error, 'Unable to start recording'));
    } finally {
      loading.value = false;
    }
  };

  const stop = async () => {
    loading.value = true;
    setError(null);

    try {
      setSnapshot(await stopRecording());
    } catch (error) {
      setError(errorMessage(error, 'Unable to stop recording'));
    } finally {
      loading.value = false;
    }
  };

  const toggle = async () => {
    loading.value = true;
    setError(null);

    try {
      setSnapshot(await toggleRecordingFromShortcut());
    } catch (error) {
      setError(errorMessage(error, 'Unable to toggle recording'));
    } finally {
      loading.value = false;
    }
  };

  const openPath = async (path: string) => {
    setError(null);

    try {
      await openLocalPath(path);
    } catch (error) {
      setError(errorMessage(error, 'Unable to open path'));
    }
  };

  const retry = async (recording: Recording) => {
    loading.value = true;
    setError(null);

    try {
      setSnapshot(await retryRecordingJobs(recording.id));
    } catch (error) {
      setError(errorMessage(error, 'Unable to retry recording jobs'));
    } finally {
      loading.value = false;
    }
  };

  const remove = async (recording: Recording) => {
    const shouldDelete = window.confirm(`Delete ${recording.title} and its artifacts?`);

    if (!shouldDelete) {
      return;
    }

    loading.value = true;
    setError(null);

    try {
      setSnapshot(await deleteRecording(recording.id));
    } catch (error) {
      setError(errorMessage(error, 'Unable to delete recording'));
    } finally {
      loading.value = false;
    }
  };

  const renameSpeaker = async (recording: Recording, speaker: string) => {
    const replacement = speakerRenameDraft.value.trim();

    if (replacement.length === 0) {
      setError('Speaker label cannot be empty');

      return;
    }

    if (replacement === speaker.trim()) {
      speakerRenameTarget.value = null;
      speakerRenameDraft.value = '';

      return;
    }

    loading.value = true;
    setError(null);

    try {
      setSnapshot(await renameSpeakerLabel(recording.id, speaker, replacement));
      speakerRenameTarget.value = null;
      speakerRenameDraft.value = '';
    } catch (error) {
      setError(errorMessage(error, 'Unable to rename speaker'));
    } finally {
      loading.value = false;
    }
  };

  const latestRecording = useComputed(() =>
    resolveLatestRecording(appSnapshotSignal.value.recordings)
  );
  const isRecording = useComputed(() => appSnapshotSignal.value.activeRecording !== null);
  const activeJobs = useComputed(() =>
    appSnapshotSignal.value.jobs.filter(
      (job) => job.status === 'running' || job.status === 'pending'
    )
  );
  const latestRecordingProgress = useComputed(() =>
    latestRecording.value ? recordingProgress(latestRecording.value) : 0
  );
  const latestRecordingPipelineStatus = useComputed(() =>
    latestRecording.value ? recordingPipelineStatus(latestRecording.value) : null
  );

  return {
    latestRecording,
    recordingRows: recordingRows({
      speakerRenameTarget,
      speakerRenameDraft,
      openPath,
      retry,
      remove,
      renameSpeaker
    }),
    groupedJobRows: groupedJobRows(retry),
    recentRecordingRows: recentRecordingRows(openPath, retry),
    activeJobs,
    latestRecordingProgress,
    latestRecordingPipelineStatus,
    isRecording,
    actions: {
      start,
      stop,
      toggle
    }
  };
};

const recordingRows = ({
  speakerRenameTarget,
  speakerRenameDraft,
  openPath,
  retry,
  remove,
  renameSpeaker
}: {
  speakerRenameTarget: Signal<{ recordingId: string; speaker: string } | null>;
  speakerRenameDraft: Signal<string>;
  openPath: (path: string) => Promise<void>;
  retry: (recording: Recording) => Promise<void>;
  remove: (recording: Recording) => Promise<void>;
  renameSpeaker: (recording: Recording, speaker: string) => Promise<void>;
}) =>
  useComputed(() =>
    appSnapshotSignal.value.recordings.map((recording) => {
      const speakerRows: SpeakerLabelRow[] = recording.speakerLabels.map((speaker) => {
        const isRenaming =
          speakerRenameTarget.value?.recordingId === recording.id &&
          speakerRenameTarget.value.speaker === speaker.name;

        return {
          name: speaker.name,
          isRenaming,
          renameValue: isRenaming ? speakerRenameDraft.value : speaker.name,
          onStartRename: () => {
            speakerRenameTarget.value = {
              recordingId: recording.id,
              speaker: speaker.name
            };
            speakerRenameDraft.value = speaker.name;
          },
          onRenameInput: (event) => {
            speakerRenameDraft.value = event.currentTarget.value;
          },
          onRenameSubmit: (event) => {
            event.preventDefault();
            void renameSpeaker(recording, speaker.name);
          },
          onCancelRename: () => {
            speakerRenameTarget.value = null;
            speakerRenameDraft.value = '';
          }
        };
      });

      return {
        recording,
        canRetry: canRetryRecording(recording),
        speakerRows,
        onOpenFolder: () => {
          void openPath(recording.artifactDirectory);
        },
        onRetry: () => {
          void retry(recording);
        },
        onDelete: () => {
          void remove(recording);
        }
      };
    })
  );

const recentRecordingRows = (
  openPath: (path: string) => Promise<void>,
  retry: (recording: Recording) => Promise<void>
) =>
  useComputed(() =>
    appSnapshotSignal.value.recordings.slice(0, 5).map((recording) => ({
      recording,
      progress: recordingProgress(recording),
      pipelineStatus: recordingPipelineStatus(recording),
      canRetry: canRetryRecording(recording),
      onOpenFolder: () => {
        void openPath(recording.artifactDirectory);
      },
      onRetry: () => {
        void retry(recording);
      }
    }))
  );

const groupedJobRows = (retry: (recording: Recording) => Promise<void>) =>
  useComputed(() =>
    appSnapshotSignal.value.recordings
      .map((recording) => ({
        recording,
        progress: recordingProgress(recording),
        pipelineStatus: recordingPipelineStatus(recording),
        canRetry: canRetryRecording(recording),
        jobs: appSnapshotSignal.value.jobs.filter((job) => job.recordingId === recording.id),
        onRetry: () => {
          void retry(recording);
        }
      }))
      .filter((row) => row.jobs.length > 0)
  );
