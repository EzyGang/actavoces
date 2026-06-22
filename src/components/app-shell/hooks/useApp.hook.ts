import { useSignal } from '@preact/signals';
import { getVersion } from '@tauri-apps/api/app';
import { useEffect } from 'preact/hooks';
import { appErrorSignal, appSnapshotSignal } from '../../../stores/app.store';
import { activeRouteSignal } from '../../../stores/route.store';
import type { AppSnapshot } from '../../../types/desktop';
import { formatDuration, formatTimestamp } from '../../../utils/format';
import { displayHotkey } from '../../../utils/hotkey';
import { useRecordings } from '../../recordings/hooks/useRecordings.hook';
import { useSettings } from '../../settings/hooks/useSettings.hook';
import { useUpdates } from '../../updates/hooks/useUpdates.hook';
import { useWorkerRuntime } from '../../worker-runtime/hooks/useWorkerRuntime.hook';
import { isTauriRuntime, routeLabel } from './appRuntime.helpers';
import { useAppNavigation } from './useAppNavigation.hook';
import { useAppRuntime } from './useAppRuntime.hook';

export const useApp = () => {
  const loading = useSignal(false);
  const appVersion = useSignal('unknown');
  const setError = (message: string | null) => {
    appErrorSignal.value = message;
  };
  let resetSettingsDraft = (_settings: AppSnapshot['settings']) => {};
  let canResetSettingsDraft = () => true;
  const setSnapshot = (snapshot: AppSnapshot) => {
    const shouldResetSettingsDraft = canResetSettingsDraft();

    appSnapshotSignal.value = snapshot;

    if (shouldResetSettingsDraft) {
      resetSettingsDraft(snapshot.settings);
    }
  };

  const settings = useSettings({
    setError,
    setSnapshot
  });
  resetSettingsDraft = settings.resetDraft;
  canResetSettingsDraft = () =>
    !settings.hasUnsavedSettings.value && !settings.savingSettings.value;

  const recordings = useRecordings({
    loading,
    setError,
    setSnapshot
  });
  const runtime = useAppRuntime({
    loading,
    settingsDraft: settings.draft,
    setError,
    setSnapshot
  });
  const workerRuntime = useWorkerRuntime({
    loading,
    settingsDraft: settings.draft,
    setError,
    setSnapshot
  });
  const updates = useUpdates({
    loading,
    setError,
    setupReady: runtime.setupReady
  });
  const navigation = useAppNavigation({
    hasUnsavedSettings: settings.hasUnsavedSettings,
    resetSettingsDraft
  });

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    void getVersion()
      .then((version) => {
        appVersion.value = version;
      })
      .catch(() => {
        appVersion.value = 'unknown';
      });
  }, []);

  return {
    data: {
      snapshot: appSnapshotSignal,
      latestRecording: recordings.latestRecording,
      latestRecordingActions: recordings.latestRecordingActions,
      recordingRows: recordings.recordingRows,
      groupedJobRows: recordings.groupedJobRows,
      recentRecordingRows: recordings.recentRecordingRows,
      activeJobs: recordings.activeJobs,
      latestRecordingProgress: recordings.latestRecordingProgress,
      latestRecordingPipelineStatus: recordings.latestRecordingPipelineStatus,
      selectedModel: workerRuntime.selectedModel,
      settingsValidationErrors: settings.validationErrors,
      routeLabel,
      displayHotkey,
      formatDuration,
      formatTimestamp,
      setupProgress: runtime.setupProgress,
      sortformerProgress: runtime.sortformerProgress,
      updateStatus: updates.updateStatus,
      updateAvailable: updates.updateAvailable,
      updateNoticeVisible: updates.updateNoticeVisible,
      updateToast: updates.updateToast,
      appVersion
    },
    status: {
      loading,
      savingSettings: settings.savingSettings,
      installingModel: workerRuntime.installingModel,
      updateChecking: updates.updateChecking,
      updateInstalling: updates.updateInstalling,
      setupReady: runtime.setupReady,
      needsDiarizationSetup: runtime.needsDiarizationSetup,
      setupRunning: runtime.setupRunning,
      error: appErrorSignal,
      isRecording: recordings.isRecording,
      activeRoute: activeRouteSignal,
      hasUnsavedSettings: settings.hasUnsavedSettings
    },
    navigation,
    settings: {
      draft: settings.draft,
      ...settings.fields
    },
    actions: {
      startRecording: recordings.actions.start,
      stopRecording: recordings.actions.stop,
      toggleRecording: recordings.actions.toggle,
      checkWorker: workerRuntime.actions.checkWorker,
      refreshModels: workerRuntime.actions.refreshModels,
      installSelectedModel: workerRuntime.actions.installSelectedModel,
      clearProviderApiKey: settings.actions.clearProviderApiKey,
      clearHuggingFaceToken: settings.actions.clearHuggingFaceToken,
      saveSettings: settings.actions.saveSettings,
      checkForUpdates: updates.actions.checkForUpdates,
      dismissUpdateToast: updates.actions.dismissUpdateToast,
      installUpdate: updates.actions.installUpdate,
      retrySetup: runtime.actions.retrySetup,
      setupDiarization: runtime.actions.setupDiarization,
      skipDiarizationSetup: runtime.actions.skipDiarizationSetup
    }
  };
};
