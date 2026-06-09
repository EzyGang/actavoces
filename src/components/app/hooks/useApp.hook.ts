import { useComputed, useSignal } from '@preact/signals';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { JSX } from 'preact';
import { useEffect } from 'preact/hooks';
import {
  checkWorkerHealth,
  clearSummaryProviderApiKey,
  deleteRecording,
  getAppSnapshot,
  installTranscriptionModel,
  openLocalPath,
  refreshModelInventory,
  resumePendingJobs,
  retryRecordingJobs,
  startRecording,
  stopRecording,
  toggleRecordingFromShortcut,
  updateAppSettings
} from '../../../services/desktop/app.service';
import { appErrorSignal, appSnapshotSignal } from '../../../stores/app.store';
import {
  type AppRoute,
  activeRouteSignal,
  navigationItems,
  setActiveRoute
} from '../../../stores/route.store';
import type {
  AppSettings,
  AppSettingsUpdate,
  CaptureDeviceInfo,
  PipelineJob,
  Recording
} from '../../../types/desktop';
import { formatDuration, formatTimestamp } from '../../../utils/format';
import { validateSettingsDraft } from '../../../utils/settings';

interface SettingsTextField {
  key: keyof AppSettingsUpdate;
  label: string;
  inputType?: 'text' | 'password';
  value: string;
  onInput: JSX.InputEventHandler<HTMLInputElement>;
}

interface SettingsFolderField {
  key: 'outputDirectory' | 'modelStorageDirectory';
  label: string;
  value: string;
  onSelect: () => void;
}

interface SettingsHotkeyField {
  label: string;
  value: string;
  recording: boolean;
  onCapture: () => void;
}

interface SettingsNumberField {
  key: keyof AppSettingsUpdate;
  label: string;
  value: number;
  onInput: JSX.InputEventHandler<HTMLInputElement>;
}

interface SettingsTextareaField {
  key: keyof AppSettingsUpdate;
  label: string;
  value: string;
  onInput: JSX.InputEventHandler<HTMLTextAreaElement>;
}

interface SettingsSelectField {
  key: keyof AppSettingsUpdate;
  label: string;
  value: string;
  options: string[];
  onChange: JSX.GenericEventHandler<HTMLSelectElement>;
}

const resolveLatestRecording = (recordings: Recording[]): Recording | null =>
  recordings.length > 0 ? recordings[0] : null;

const canRetryRecording = (recording: Recording): boolean =>
  recording.stages.some(
    (stage) =>
      stage.id !== 'recording' && (stage.status === 'failed' || stage.status === 'needsSetup')
  );

const recordingTitleForJob = (recordings: Recording[], job: PipelineJob): string =>
  recordings.find((recording) => recording.id === job.recordingId)?.title ?? job.recordingId;

const captureDeviceOptions = (devices: CaptureDeviceInfo[], selectedValue: string): string[] => {
  const options = devices.map((device) => device.name);

  if (selectedValue.trim().length > 0 && !options.includes(selectedValue)) {
    return [selectedValue, ...options];
  }

  return options;
};

const modifierKeys = new Set(['Alt', 'Control', 'Meta', 'Shift']);

const hotkeyKey = (event: KeyboardEvent): string | null => {
  if (event.key === ' ') {
    return 'Space';
  }

  if (modifierKeys.has(event.key)) {
    return null;
  }

  if (event.key.length === 1) {
    return event.key.toUpperCase();
  }

  return event.key;
};

const hotkeyFromKeyboardEvent = (event: KeyboardEvent): string | null => {
  const key = hotkeyKey(event);

  if (!key) {
    return null;
  }

  const modifiers = [
    event.ctrlKey || event.metaKey ? 'CommandOrControl' : null,
    event.altKey ? 'Alt' : null,
    event.shiftKey ? 'Shift' : null
  ].filter((modifier): modifier is string => modifier !== null);

  return [...modifiers, key].join('+');
};

const buildSettingsUpdate = (settings: AppSettings): AppSettingsUpdate => ({
  outputDirectory: settings.outputDirectory,
  hotkey: settings.hotkey,
  overlayPosition: settings.overlayPosition,
  launchAtLogin: settings.launchAtLogin,
  microphoneDevice: settings.microphoneDevice,
  systemAudioSource: settings.systemAudioSource,
  sampleRate: settings.sampleRate,
  whisperModel: settings.whisperModel,
  transcriptionLanguage: settings.transcriptionLanguage,
  computeType: settings.computeType,
  modelStorageDirectory: settings.modelStorageDirectory,
  diarizationBackend: settings.diarizationBackend,
  speakerCountMode: settings.speakerCountMode,
  exactSpeakers: settings.exactSpeakers,
  minSpeakers: settings.minSpeakers,
  maxSpeakers: settings.maxSpeakers,
  summaryEnabled: settings.summaryEnabled,
  providerBaseUrl: settings.providerBaseUrl,
  providerModel: settings.providerModel,
  providerApiKey: '',
  titlePrompt: settings.titlePrompt,
  summaryPrompt: settings.summaryPrompt
});

const settingsDraftChanged = (draft: AppSettingsUpdate, settings: AppSettings): boolean =>
  JSON.stringify(draft) !== JSON.stringify(buildSettingsUpdate(settings));

const routeLabel: Record<AppRoute, string> = {
  dashboard: 'Dashboard',
  recordings: 'Recordings',
  jobs: 'Jobs',
  settings: 'Settings'
};

const isTauriRuntime = () => '__TAURI_INTERNALS__' in window;

export const useApp = () => {
  const loading = useSignal(false);
  const savingSettings = useSignal(false);
  const installingModel = useSignal(false);
  const recordingHotkey = useSignal(false);
  const settingsDraft = useSignal<AppSettingsUpdate>(
    buildSettingsUpdate(appSnapshotSignal.value.settings)
  );

  const setSnapshot = (snapshot: typeof appSnapshotSignal.value) => {
    appSnapshotSignal.value = snapshot;
    settingsDraft.value = buildSettingsUpdate(snapshot.settings);
  };

  const loadSnapshot = async () => {
    loading.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await getAppSnapshot());
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
      setSnapshot(await startRecording());
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
      setSnapshot(await stopRecording());
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
      setSnapshot(await resumePendingJobs());
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to resume jobs';
    } finally {
      loading.value = false;
    }
  };

  const handleDeleteRecording = async (recording: Recording) => {
    const shouldDelete = window.confirm(`Delete ${recording.title} and its artifacts?`);

    if (!shouldDelete) {
      return;
    }

    loading.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await deleteRecording(recording.id));
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to delete recording';
    } finally {
      loading.value = false;
    }
  };

  const handleOpenPath = async (path: string) => {
    appErrorSignal.value = null;

    try {
      await openLocalPath(path);
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to open path';
    }
  };

  const handleRetryRecording = async (recording: Recording) => {
    loading.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await retryRecordingJobs(recording.id));
    } catch (error) {
      appErrorSignal.value =
        error instanceof Error ? error.message : 'Unable to retry recording jobs';
    } finally {
      loading.value = false;
    }
  };

  const handleToggleRecording = async () => {
    loading.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await toggleRecordingFromShortcut());
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to toggle recording';
    } finally {
      loading.value = false;
    }
  };

  const handleCheckWorker = async () => {
    loading.value = true;
    appErrorSignal.value = null;

    try {
      await checkWorkerHealth();
      setSnapshot(await getAppSnapshot());
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to check worker';
    } finally {
      loading.value = false;
    }
  };

  const handleRefreshModels = async () => {
    loading.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await refreshModelInventory());
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to refresh models';
    } finally {
      loading.value = false;
    }
  };

  const handleInstallSelectedModel = async () => {
    installingModel.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await installTranscriptionModel(settingsDraft.value.whisperModel));
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to install model';
    } finally {
      installingModel.value = false;
    }
  };

  const handleSaveSettings = async () => {
    const validationErrors = validateSettingsDraft(
      settingsDraft.value,
      appSnapshotSignal.value.settings.providerApiKeyConfigured
    );

    if (validationErrors.length > 0) {
      appErrorSignal.value = validationErrors.join(' ');

      return;
    }

    savingSettings.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await updateAppSettings(settingsDraft.value));
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to save settings';
    } finally {
      savingSettings.value = false;
    }
  };

  const handleClearProviderApiKey = async () => {
    savingSettings.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await clearSummaryProviderApiKey());
    } catch (error) {
      appErrorSignal.value =
        error instanceof Error ? error.message : 'Unable to clear provider API key';
    } finally {
      savingSettings.value = false;
    }
  };

  const makeFolderSelectHandler = (key: SettingsFolderField['key']) => async () => {
    appErrorSignal.value = null;

    try {
      const selectedPath = await open({
        defaultPath: settingsDraft.value[key],
        directory: true,
        multiple: false
      });

      if (typeof selectedPath !== 'string') {
        return;
      }

      settingsDraft.value = {
        ...settingsDraft.value,
        [key]: selectedPath
      };
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to select folder';
    }
  };

  const handleCaptureHotkey = () => {
    recordingHotkey.value = true;
  };

  const makeTextInputHandler =
    (key: keyof AppSettingsUpdate): JSX.InputEventHandler<HTMLInputElement> =>
    (event) => {
      settingsDraft.value = {
        ...settingsDraft.value,
        [key]: event.currentTarget.value
      };
    };

  const makeNumberInputHandler =
    (key: keyof AppSettingsUpdate): JSX.InputEventHandler<HTMLInputElement> =>
    (event) => {
      const nextValue = Number(event.currentTarget.value);

      settingsDraft.value = {
        ...settingsDraft.value,
        [key]: Number.isNaN(nextValue) ? 0 : nextValue
      };
    };

  const makeOptionalNumberInputHandler =
    (key: keyof AppSettingsUpdate): JSX.InputEventHandler<HTMLInputElement> =>
    (event) => {
      const rawValue = event.currentTarget.value.trim();

      settingsDraft.value = {
        ...settingsDraft.value,
        [key]: rawValue.length === 0 ? null : Number(rawValue)
      };
    };

  const makeTextareaInputHandler =
    (key: keyof AppSettingsUpdate): JSX.InputEventHandler<HTMLTextAreaElement> =>
    (event) => {
      settingsDraft.value = {
        ...settingsDraft.value,
        [key]: event.currentTarget.value
      };
    };

  const makeSelectChangeHandler =
    (key: keyof AppSettingsUpdate): JSX.GenericEventHandler<HTMLSelectElement> =>
    (event) => {
      settingsDraft.value = {
        ...settingsDraft.value,
        [key]: event.currentTarget.value
      };
    };

  const handleLaunchAtLoginChange: JSX.InputEventHandler<HTMLInputElement> = (event) => {
    settingsDraft.value = {
      ...settingsDraft.value,
      launchAtLogin: event.currentTarget.checked
    };
  };

  const handleSummaryEnabledChange: JSX.InputEventHandler<HTMLInputElement> = (event) => {
    settingsDraft.value = {
      ...settingsDraft.value,
      summaryEnabled: event.currentTarget.checked
    };
  };

  const hasUnsavedSettings = useComputed(() =>
    settingsDraftChanged(settingsDraft.value, appSnapshotSignal.value.settings)
  );

  const handleSelectRoute = (route: AppRoute) => {
    if (
      activeRouteSignal.value === 'settings' &&
      route !== 'settings' &&
      hasUnsavedSettings.value
    ) {
      const shouldLeave = window.confirm('Discard unsaved settings changes?');

      if (!shouldLeave) {
        return;
      }

      settingsDraft.value = buildSettingsUpdate(appSnapshotSignal.value.settings);
    }

    setActiveRoute(route);
  };

  useEffect(() => {
    void loadSnapshot();
  }, []);

  useEffect(() => {
    if (!recordingHotkey.value) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        recordingHotkey.value = false;

        return;
      }

      const nextHotkey = hotkeyFromKeyboardEvent(event);

      if (!nextHotkey) {
        return;
      }

      event.preventDefault();
      settingsDraft.value = {
        ...settingsDraft.value,
        hotkey: nextHotkey
      };
      recordingHotkey.value = false;
    };

    window.addEventListener('keydown', handleKeyDown, { capture: true });

    return () => {
      window.removeEventListener('keydown', handleKeyDown, { capture: true });
    };
  }, [recordingHotkey.value]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    const snapshotListener = listen<typeof appSnapshotSignal.value>(
      'app-snapshot-updated',
      (event) => {
        setSnapshot(event.payload);
      }
    );
    const errorListener = listen<string>('app-error', (event) => {
      appErrorSignal.value = event.payload;
    });

    return () => {
      void snapshotListener.then((unlisten) => unlisten());
      void errorListener.then((unlisten) => unlisten());
    };
  }, []);

  const latestRecording = useComputed(() =>
    resolveLatestRecording(appSnapshotSignal.value.recordings)
  );
  const recordingRows = useComputed(() =>
    appSnapshotSignal.value.recordings.map((recording) => ({
      recording,
      canRetry: canRetryRecording(recording),
      onOpenFolder: () => {
        void handleOpenPath(recording.artifactDirectory);
      },
      onRetry: () => {
        void handleRetryRecording(recording);
      },
      onDelete: () => {
        void handleDeleteRecording(recording);
      }
    }))
  );
  const latestArtifacts = useComputed(() =>
    latestRecording.value
      ? latestRecording.value.artifacts.map((artifact) => ({
          artifact,
          onOpen: () => {
            void handleOpenPath(artifact.path);
          }
        }))
      : []
  );
  const jobRows = useComputed(() =>
    appSnapshotSignal.value.jobs.map((job) => {
      const recording = appSnapshotSignal.value.recordings.find(
        (candidate) => candidate.id === job.recordingId
      );

      return {
        job,
        recordingTitle: recordingTitleForJob(appSnapshotSignal.value.recordings, job),
        canRetry: recording ? canRetryRecording(recording) : false,
        onRetry: recording
          ? () => {
              void handleRetryRecording(recording);
            }
          : undefined
      };
    })
  );
  const isRecording = useComputed(() => appSnapshotSignal.value.activeRecording !== null);
  const activeJobs = useComputed(() =>
    appSnapshotSignal.value.jobs.filter(
      (job: PipelineJob) => job.status === 'running' || job.status === 'pending'
    )
  );
  const settingsValidationErrors = useComputed(() =>
    validateSettingsDraft(
      settingsDraft.value,
      appSnapshotSignal.value.settings.providerApiKeyConfigured
    )
  );
  const selectedModel = useComputed(
    () =>
      appSnapshotSignal.value.models.find(
        (model) => model.name === settingsDraft.value.whisperModel
      ) ?? null
  );

  return {
    data: {
      snapshot: appSnapshotSignal,
      latestRecording,
      recordingRows,
      latestArtifacts,
      jobRows,
      activeJobs,
      selectedModel,
      settingsValidationErrors,
      routeLabel,
      formatDuration,
      formatTimestamp
    },
    status: {
      loading,
      savingSettings,
      installingModel,
      error: appErrorSignal,
      isRecording,
      activeRoute: activeRouteSignal,
      hasUnsavedSettings
    },
    navigation: navigationItems.map((item) => ({
      ...item,
      isActive: item.route === activeRouteSignal.value,
      onSelect: () => handleSelectRoute(item.route)
    })),
    settings: {
      draft: settingsDraft,
      folderFields: [
        {
          key: 'outputDirectory',
          label: 'Output directory',
          value: settingsDraft.value.outputDirectory,
          onSelect: makeFolderSelectHandler('outputDirectory')
        },
        {
          key: 'modelStorageDirectory',
          label: 'Model storage',
          value: settingsDraft.value.modelStorageDirectory,
          onSelect: makeFolderSelectHandler('modelStorageDirectory')
        }
      ] satisfies SettingsFolderField[],
      hotkeyField: {
        label: 'Global hotkey',
        value: settingsDraft.value.hotkey,
        recording: recordingHotkey.value,
        onCapture: handleCaptureHotkey
      } satisfies SettingsHotkeyField,
      textFields: [
        {
          key: 'providerBaseUrl',
          label: 'Provider base URL',
          value: settingsDraft.value.providerBaseUrl,
          onInput: makeTextInputHandler('providerBaseUrl')
        },
        {
          key: 'providerModel',
          label: 'Provider model',
          value: settingsDraft.value.providerModel,
          onInput: makeTextInputHandler('providerModel')
        },
        {
          key: 'providerApiKey',
          label: 'Provider API key',
          inputType: 'password',
          value: settingsDraft.value.providerApiKey,
          onInput: makeTextInputHandler('providerApiKey')
        }
      ] satisfies SettingsTextField[],
      captureSelectFields: [
        {
          key: 'microphoneDevice',
          label: 'Microphone device',
          value: settingsDraft.value.microphoneDevice,
          options: captureDeviceOptions(
            appSnapshotSignal.value.captureDevices.microphones,
            settingsDraft.value.microphoneDevice
          ),
          onChange: makeSelectChangeHandler('microphoneDevice')
        },
        {
          key: 'systemAudioSource',
          label: 'System audio source',
          value: settingsDraft.value.systemAudioSource,
          options: captureDeviceOptions(
            appSnapshotSignal.value.captureDevices.systemSources,
            settingsDraft.value.systemAudioSource
          ),
          onChange: makeSelectChangeHandler('systemAudioSource')
        }
      ] satisfies SettingsSelectField[],
      numberFields: [
        {
          key: 'sampleRate',
          label: 'Sample rate',
          value: settingsDraft.value.sampleRate,
          onInput: makeNumberInputHandler('sampleRate')
        },
        {
          key: 'exactSpeakers',
          label: 'Exact speakers',
          value: settingsDraft.value.exactSpeakers ?? 0,
          onInput: makeOptionalNumberInputHandler('exactSpeakers')
        },
        {
          key: 'minSpeakers',
          label: 'Minimum speakers',
          value: settingsDraft.value.minSpeakers ?? 0,
          onInput: makeOptionalNumberInputHandler('minSpeakers')
        },
        {
          key: 'maxSpeakers',
          label: 'Maximum speakers',
          value: settingsDraft.value.maxSpeakers ?? 0,
          onInput: makeOptionalNumberInputHandler('maxSpeakers')
        }
      ] satisfies SettingsNumberField[],
      selectFields: [
        {
          key: 'whisperModel',
          label: 'Whisper model',
          value: settingsDraft.value.whisperModel,
          options: ['small.en', 'medium.en', 'large-v3', 'distil-large-v3'],
          onChange: makeSelectChangeHandler('whisperModel')
        },
        {
          key: 'transcriptionLanguage',
          label: 'Language',
          value: settingsDraft.value.transcriptionLanguage,
          options: ['auto', 'en', 'ru', 'uk', 'es'],
          onChange: makeSelectChangeHandler('transcriptionLanguage')
        },
        {
          key: 'computeType',
          label: 'Compute type',
          value: settingsDraft.value.computeType,
          options: ['auto', 'cpu', 'cuda', 'metal'],
          onChange: makeSelectChangeHandler('computeType')
        },
        {
          key: 'diarizationBackend',
          label: 'Diarization backend',
          value: settingsDraft.value.diarizationBackend,
          options: ['nemoWhisper', 'pyannote'],
          onChange: makeSelectChangeHandler('diarizationBackend')
        },
        {
          key: 'speakerCountMode',
          label: 'Speaker count',
          value: settingsDraft.value.speakerCountMode,
          options: ['automatic', 'exact', 'range'],
          onChange: makeSelectChangeHandler('speakerCountMode')
        },
        {
          key: 'overlayPosition',
          label: 'Overlay position',
          value: settingsDraft.value.overlayPosition,
          options: ['topLeft', 'topRight', 'bottomLeft', 'bottomRight'],
          onChange: makeSelectChangeHandler('overlayPosition')
        }
      ] satisfies SettingsSelectField[],
      textareaFields: [
        {
          key: 'titlePrompt',
          label: 'Title prompt',
          value: settingsDraft.value.titlePrompt,
          onInput: makeTextareaInputHandler('titlePrompt')
        },
        {
          key: 'summaryPrompt',
          label: 'Summary prompt',
          value: settingsDraft.value.summaryPrompt,
          onInput: makeTextareaInputHandler('summaryPrompt')
        }
      ] satisfies SettingsTextareaField[],
      toggles: {
        launchAtLogin: {
          checked: settingsDraft.value.launchAtLogin,
          onInput: handleLaunchAtLoginChange
        },
        summaryEnabled: {
          checked: settingsDraft.value.summaryEnabled,
          onInput: handleSummaryEnabledChange
        }
      }
    },
    actions: {
      startRecording: handleStartRecording,
      stopRecording: handleStopRecording,
      toggleRecording: handleToggleRecording,
      resumeJobs: handleResumeJobs,
      checkWorker: handleCheckWorker,
      refreshModels: handleRefreshModels,
      installSelectedModel: handleInstallSelectedModel,
      clearProviderApiKey: handleClearProviderApiKey,
      saveSettings: handleSaveSettings
    }
  };
};
