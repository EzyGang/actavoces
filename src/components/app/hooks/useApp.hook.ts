import { useComputed, useSignal } from '@preact/signals';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { JSX } from 'preact';
import { useEffect } from 'preact/hooks';
import {
  bootstrapWorkerRuntime,
  checkWorkerHealth,
  clearHuggingFaceToken,
  clearSummaryProviderApiKey,
  deleteRecording,
  getAppSnapshot,
  installTranscriptionModel,
  openLocalPath,
  refreshModelInventory,
  retryRecordingJobs,
  setupDiarizationRuntime,
  skipDiarizationSetup,
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
  PipelineStage,
  Recording,
  WorkerSetupProgress
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
  hint?: SettingsFieldHint;
}

interface SettingsFieldHint {
  tone: 'muted' | 'warning';
  title?: string;
  text: string;
  links?: SettingsFieldHintLink[];
}

interface SettingsFieldHintLink {
  href: string;
  label: string;
}

const resolveLatestRecording = (recordings: Recording[]): Recording | null =>
  recordings.length > 0 ? recordings[0] : null;

const canRetryRecording = (recording: Recording): boolean =>
  recording.stages.some(
    (stage) =>
      stage.id !== 'recording' && (stage.status === 'failed' || stage.status === 'needsSetup')
  );

const stageProgressWeight = (stage: PipelineStage): number => {
  if (stage.status === 'complete' || stage.status === 'skipped') {
    return 100;
  }

  return stage.progress;
};

const recordingProgress = (recording: Recording): number => {
  if (recording.stages.length === 0) {
    return 0;
  }

  const progress = recording.stages.reduce((total, stage) => total + stageProgressWeight(stage), 0);

  return Math.round(progress / recording.stages.length);
};

const recordingPipelineStatus = (recording: Recording) => {
  const blocker = recording.stages.find(
    (stage) => stage.status === 'failed' || stage.status === 'needsSetup'
  );

  if (blocker) {
    return {
      status: blocker.status,
      label: blocker.status,
      message: blocker.message
    };
  }

  if (recording.stages.some((stage) => stage.status === 'running')) {
    return {
      status: 'running' as const,
      label: 'running',
      message: 'Processing recording'
    };
  }

  if (
    recording.stages.every((stage) => stage.status === 'complete' || stage.status === 'skipped')
  ) {
    return {
      status: 'complete' as const,
      label: 'complete',
      message: 'Processing complete'
    };
  }

  return {
    status: 'pending' as const,
    label: 'pending',
    message: 'Waiting for processing'
  };
};

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
  huggingFaceToken: '',
  diarizationSetupSkipped: settings.diarizationSetupSkipped,
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

const initialSetupProgress: WorkerSetupProgress = {
  status: 'missing',
  step: 'Preparing local worker runtime',
  error: null
};

const errorMessage = (error: unknown, fallback: string): string => {
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

export const useApp = () => {
  const loading = useSignal(false);
  const savingSettings = useSignal(false);
  const installingModel = useSignal(false);
  const recordingHotkey = useSignal(false);
  const setupProgress = useSignal<WorkerSetupProgress>(initialSetupProgress);
  const setupRunning = useSignal(false);
  const bootstrapRequested = useSignal(false);
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
      appErrorSignal.value = errorMessage(error, 'Desktop backend unavailable');
    } finally {
      loading.value = false;
    }
  };

  const runBootstrap = async () => {
    setupRunning.value = true;
    appErrorSignal.value = null;
    setupProgress.value = {
      ...setupProgress.value,
      status: 'installing',
      step: 'Preparing local worker runtime',
      error: null
    };

    try {
      setSnapshot(await bootstrapWorkerRuntime());
      setupProgress.value = {
        status: 'ready',
        step: 'Worker runtime ready',
        error: null
      };
    } catch (error) {
      const message = errorMessage(error, 'Unable to prepare worker runtime');

      appErrorSignal.value = message;
      setupProgress.value = {
        status: 'failed',
        step: 'Worker setup failed',
        error: message
      };
    } finally {
      setupRunning.value = false;
    }
  };

  const handleSetupDiarization = async () => {
    setupRunning.value = true;
    appErrorSignal.value = null;
    setupProgress.value = {
      status: 'installing',
      step: 'Preparing speaker diarization',
      error: null
    };

    try {
      setSnapshot(await setupDiarizationRuntime(settingsDraft.value.huggingFaceToken));
      setupProgress.value = {
        status: 'ready',
        step: 'Speaker diarization runtime ready',
        error: null
      };
    } catch (error) {
      const message = errorMessage(error, 'Unable to prepare speaker diarization');

      appErrorSignal.value = message;
      setupProgress.value = {
        status: 'failed',
        step: 'Speaker diarization setup failed',
        error: message
      };
    } finally {
      setupRunning.value = false;
    }
  };

  const handleSkipDiarizationSetup = async () => {
    setupRunning.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await skipDiarizationSetup());
    } catch (error) {
      appErrorSignal.value = errorMessage(error, 'Unable to skip speaker diarization setup');
    } finally {
      setupRunning.value = false;
    }
  };

  const handleStartRecording = async () => {
    loading.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await startRecording());
    } catch (error) {
      appErrorSignal.value = errorMessage(error, 'Unable to start recording');
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
      appErrorSignal.value = errorMessage(error, 'Unable to stop recording');
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
      appErrorSignal.value = errorMessage(error, 'Unable to delete recording');
    } finally {
      loading.value = false;
    }
  };

  const handleOpenPath = async (path: string) => {
    appErrorSignal.value = null;

    try {
      await openLocalPath(path);
    } catch (error) {
      appErrorSignal.value = errorMessage(error, 'Unable to open path');
    }
  };

  const handleRetryRecording = async (recording: Recording) => {
    loading.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await retryRecordingJobs(recording.id));
    } catch (error) {
      appErrorSignal.value = errorMessage(error, 'Unable to retry recording jobs');
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
      appErrorSignal.value = errorMessage(error, 'Unable to toggle recording');
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
      appErrorSignal.value = errorMessage(error, 'Unable to check worker');
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
      appErrorSignal.value = errorMessage(error, 'Unable to refresh models');
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
      appErrorSignal.value = errorMessage(error, 'Unable to install model');
    } finally {
      installingModel.value = false;
    }
  };

  const handleSaveSettings = async () => {
    const validationErrors = validateSettingsDraft(
      settingsDraft.value,
      appSnapshotSignal.value.settings.providerApiKeyConfigured,
      appSnapshotSignal.value.desktop.cudaAvailable
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
      appErrorSignal.value = errorMessage(error, 'Unable to save settings');
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
      appErrorSignal.value = errorMessage(error, 'Unable to clear provider API key');
    } finally {
      savingSettings.value = false;
    }
  };

  const handleClearHuggingFaceToken = async () => {
    savingSettings.value = true;
    appErrorSignal.value = null;

    try {
      setSnapshot(await clearHuggingFaceToken());
    } catch (error) {
      appErrorSignal.value = errorMessage(error, 'Unable to clear Hugging Face token');
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
      appErrorSignal.value = errorMessage(error, 'Unable to select folder');
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
        loading.value = false;
      }
    );
    const errorListener = listen<string>('app-error', (event) => {
      appErrorSignal.value = event.payload;
    });
    const setupListener = listen<WorkerSetupProgress>('worker-setup-progress', (event) => {
      setupProgress.value = event.payload;
    });

    return () => {
      void snapshotListener.then((unlisten) => unlisten());
      void errorListener.then((unlisten) => unlisten());
      void setupListener.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) {
      void loadSnapshot();

      return;
    }

    loading.value = true;
  }, []);

  useEffect(() => {
    if (
      !isTauriRuntime() ||
      bootstrapRequested.value ||
      appSnapshotSignal.value.settings.databasePath.trim().length === 0
    ) {
      return;
    }

    bootstrapRequested.value = true;
    void runBootstrap();
  }, [appSnapshotSignal.value.settings.databasePath]);

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
  const isRecording = useComputed(() => appSnapshotSignal.value.activeRecording !== null);
  const activeJobs = useComputed(() =>
    appSnapshotSignal.value.jobs.filter(
      (job) => job.status === 'running' || job.status === 'pending'
    )
  );
  const settingsValidationErrors = useComputed(() =>
    validateSettingsDraft(
      settingsDraft.value,
      appSnapshotSignal.value.settings.providerApiKeyConfigured,
      appSnapshotSignal.value.desktop.cudaAvailable
    )
  );
  const latestRecordingProgress = useComputed(() =>
    latestRecording.value ? recordingProgress(latestRecording.value) : 0
  );
  const latestRecordingPipelineStatus = useComputed(() =>
    latestRecording.value ? recordingPipelineStatus(latestRecording.value) : null
  );
  const recentRecordingRows = useComputed(() =>
    appSnapshotSignal.value.recordings.slice(0, 5).map((recording) => ({
      recording,
      progress: recordingProgress(recording),
      pipelineStatus: recordingPipelineStatus(recording),
      canRetry: canRetryRecording(recording),
      onOpenFolder: () => {
        void handleOpenPath(recording.artifactDirectory);
      },
      onRetry: () => {
        void handleRetryRecording(recording);
      }
    }))
  );
  const groupedJobRows = useComputed(() =>
    appSnapshotSignal.value.recordings
      .map((recording) => ({
        recording,
        progress: recordingProgress(recording),
        pipelineStatus: recordingPipelineStatus(recording),
        canRetry: canRetryRecording(recording),
        jobs: appSnapshotSignal.value.jobs.filter((job) => job.recordingId === recording.id),
        onRetry: () => {
          void handleRetryRecording(recording);
        }
      }))
      .filter((row) => row.jobs.length > 0)
  );
  const selectedModel = useComputed(
    () =>
      appSnapshotSignal.value.models.find(
        (model) => model.name === settingsDraft.value.whisperModel
      ) ?? null
  );
  const workerSetupReady = useComputed(
    () => appSnapshotSignal.value.desktop.workerSetupStatus === 'ready'
  );
  const needsDiarizationSetup = useComputed(
    () =>
      workerSetupReady.value &&
      appSnapshotSignal.value.settings.diarizationBackend === 'pyannote' &&
      !appSnapshotSignal.value.settings.diarizationRuntimeReady &&
      !appSnapshotSignal.value.settings.diarizationSetupSkipped
  );
  const setupReady = useComputed(() => workerSetupReady.value && !needsDiarizationSetup.value);

  return {
    data: {
      snapshot: appSnapshotSignal,
      latestRecording,
      recordingRows,
      groupedJobRows,
      recentRecordingRows,
      activeJobs,
      latestRecordingProgress,
      latestRecordingPipelineStatus,
      selectedModel,
      settingsValidationErrors,
      routeLabel,
      formatDuration,
      formatTimestamp,
      setupProgress
    },
    status: {
      loading,
      savingSettings,
      installingModel,
      setupReady,
      needsDiarizationSetup,
      setupRunning,
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
      huggingFaceTokenField: {
        key: 'huggingFaceToken',
        label: 'Hugging Face token',
        inputType: 'password',
        value: settingsDraft.value.huggingFaceToken,
        onInput: makeTextInputHandler('huggingFaceToken')
      } satisfies SettingsTextField,
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
        },
        {
          key: 'overlayPosition',
          label: 'Overlay position',
          value: settingsDraft.value.overlayPosition,
          options: ['topLeft', 'topRight', 'bottomLeft', 'bottomRight'],
          onChange: makeSelectChangeHandler('overlayPosition')
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
          onChange: makeSelectChangeHandler('computeType'),
          hint: {
            tone: 'warning',
            title: 'CUDA processing requires NVIDIA libraries.',
            text: `Install cuBLAS for CUDA 12 and cuDNN 9 for CUDA 12. Runtime check: ${
              appSnapshotSignal.value.desktop.cudaAvailable
                ? 'CUDA device and required libraries detected'
                : (appSnapshotSignal.value.desktop.cudaError ?? 'not ready')
            }`,
            links: [
              {
                href: 'https://developer.nvidia.com/cublas',
                label: 'cuBLAS'
              },
              {
                href: 'https://developer.nvidia.com/cudnn',
                label: 'cuDNN'
              }
            ]
          }
        },
        {
          key: 'diarizationBackend',
          label: 'Diarization backend',
          value: settingsDraft.value.diarizationBackend,
          options: ['pyannote'],
          onChange: makeSelectChangeHandler('diarizationBackend'),
          hint: {
            tone: 'muted',
            title: 'Local pyannote speaker diarization.',
            text: 'Requires pyannote.audio, bundled or system ffmpeg, accepted Hugging Face model terms, and a Hugging Face access token. pyannoteAI cloud API support is not implemented.'
          }
        },
        {
          key: 'speakerCountMode',
          label: 'Speaker count',
          value: settingsDraft.value.speakerCountMode,
          options: ['automatic', 'exact', 'range'],
          onChange: makeSelectChangeHandler('speakerCountMode')
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
      checkWorker: handleCheckWorker,
      refreshModels: handleRefreshModels,
      installSelectedModel: handleInstallSelectedModel,
      clearProviderApiKey: handleClearProviderApiKey,
      clearHuggingFaceToken: handleClearHuggingFaceToken,
      saveSettings: handleSaveSettings,
      retrySetup: runBootstrap,
      setupDiarization: handleSetupDiarization,
      skipDiarizationSetup: handleSkipDiarizationSetup
    }
  };
};
