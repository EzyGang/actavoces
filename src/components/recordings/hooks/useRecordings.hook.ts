import { type Signal, useComputed, useSignal } from '@preact/signals';
import type { JSX } from 'preact';
import {
  deleteRecording,
  openLocalPath,
  renameRecordingTitle,
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

interface TitleRenameRow {
  isRenaming: boolean;
  value: string;
  onStart: () => void;
  onInput: JSX.InputEventHandler<HTMLInputElement>;
  onSubmit: JSX.GenericEventHandler<HTMLFormElement>;
  onCancel: () => void;
}

interface UseRecordingsInput {
  loading: Signal<boolean>;
  setError: (message: string | null) => void;
  setSnapshot: (snapshot: AppSnapshot) => void;
}

export const useRecordings = ({ loading, setError, setSnapshot }: UseRecordingsInput) => {
  const titleRenameTarget = useSignal<string | null>(null);
  const titleRenameDraft = useSignal('');
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

  const renameTitle = async (recording: Recording) => {
    const title = titleRenameDraft.value.trim();

    if (title.length === 0) {
      setError('Recording title cannot be empty');

      return;
    }

    if (title === recording.title.trim()) {
      titleRenameTarget.value = null;
      titleRenameDraft.value = '';

      return;
    }

    loading.value = true;
    setError(null);

    try {
      setSnapshot(await renameRecordingTitle(recording.id, title));
      titleRenameTarget.value = null;
      titleRenameDraft.value = '';
    } catch (error) {
      setError(errorMessage(error, 'Unable to rename recording'));
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
  const latestRecordingActions = useComputed(() => {
    if (!latestRecording.value) {
      return null;
    }

    const recording = latestRecording.value;

    return {
      canRetry: canRetryRecording(recording),
      onOpenFolder: () => {
        void openPath(recording.artifactDirectory);
      },
      onRetry: () => {
        void retry(recording);
      }
    };
  });

  return {
    latestRecording,
    latestRecordingActions,
    recordingRows: recordingRows({
      titleRenameTarget,
      titleRenameDraft,
      speakerRenameTarget,
      speakerRenameDraft,
      openPath,
      retry,
      remove,
      renameTitle,
      renameSpeaker
    }),
    groupedJobRows: groupedJobRows(openPath, retry),
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
  titleRenameTarget,
  titleRenameDraft,
  speakerRenameTarget,
  speakerRenameDraft,
  openPath,
  retry,
  remove,
  renameTitle,
  renameSpeaker
}: {
  titleRenameTarget: Signal<string | null>;
  titleRenameDraft: Signal<string>;
  speakerRenameTarget: Signal<{ recordingId: string; speaker: string } | null>;
  speakerRenameDraft: Signal<string>;
  openPath: (path: string) => Promise<void>;
  retry: (recording: Recording) => Promise<void>;
  remove: (recording: Recording) => Promise<void>;
  renameTitle: (recording: Recording) => Promise<void>;
  renameSpeaker: (recording: Recording, speaker: string) => Promise<void>;
}) =>
  useComputed(() =>
    appSnapshotSignal.value.recordings.map((recording) => {
      const isTitleRenaming = titleRenameTarget.value === recording.id;
      const titleRow: TitleRenameRow = {
        isRenaming: isTitleRenaming,
        value: isTitleRenaming ? titleRenameDraft.value : recording.title,
        onStart: () => {
          titleRenameTarget.value = recording.id;
          titleRenameDraft.value = recording.title;
        },
        onInput: (event) => {
          titleRenameDraft.value = event.currentTarget.value;
        },
        onSubmit: (event) => {
          event.preventDefault();
          void renameTitle(recording);
        },
        onCancel: () => {
          titleRenameTarget.value = null;
          titleRenameDraft.value = '';
        }
      };
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
        titleRow,
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

const groupedJobRows = (
  openPath: (path: string) => Promise<void>,
  retry: (recording: Recording) => Promise<void>
) =>
  useComputed(() =>
    appSnapshotSignal.value.recordings
      .map((recording) => ({
        recording,
        progress: recordingProgress(recording),
        pipelineStatus: recordingPipelineStatus(recording),
        canRetry: canRetryRecording(recording),
        jobs: appSnapshotSignal.value.jobs.filter((job) => job.recordingId === recording.id),
        onOpenFolder: () => {
          void openPath(recording.artifactDirectory);
        },
        onRetry: () => {
          void retry(recording);
        }
      }))
      .filter((row) => row.jobs.length > 0)
  );
