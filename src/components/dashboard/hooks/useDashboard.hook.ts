import type { useApp } from '../../app-shell/hooks/useApp.hook';

type App = ReturnType<typeof useApp>;

export interface DashboardMetric {
  badge?: {
    label: string;
    status: 'idle' | 'recording';
  };
  label: string;
  value: string;
}

export const useDashboard = (app: App) => {
  const snapshot = app.data.snapshot.value;
  const latestRecording = app.data.latestRecording.value;
  const latestRecordingActions = app.data.latestRecordingActions.value;
  const latestRecordingPipelineStatus = app.data.latestRecordingPipelineStatus.value;

  return {
    data: {
      metrics: [
        {
          label: 'Capture',
          value: app.status.isRecording.value ? 'Live' : 'Idle',
          badge: {
            label: app.status.isRecording.value ? 'Recording' : 'Ready',
            status: app.status.isRecording.value ? 'recording' : 'idle'
          }
        },
        {
          label: 'Recordings',
          value: snapshot.recordings.length.toString()
        },
        {
          label: 'Jobs',
          value: app.data.activeJobs.value.length.toString()
        },
        {
          label: 'Summary',
          value: snapshot.settings.summaryProviderConfigured ? 'Ready' : 'Off'
        }
      ] satisfies DashboardMetric[],
      updateNotice: {
        visible: app.data.updateNoticeVisible.value,
        status: app.data.updateStatus.value,
        updateAvailable: app.data.updateAvailable.value
      },
      showDiarizationWarning:
        snapshot.settings.diarizationBackend === 'pyannote' &&
        (snapshot.settings.diarizationSetupSkipped || !snapshot.settings.diarizationRuntimeReady),
      pipeline: {
        recording: latestRecording,
        actions: latestRecordingActions,
        progress: app.data.latestRecordingProgress.value,
        status: latestRecordingPipelineStatus
      },
      runtime: {
        rows: [
          {
            label: 'Worker',
            value: snapshot.desktop.workerHealthOk
              ? 'Healthy'
              : snapshot.desktop.workerRunning
                ? 'Running'
                : 'Stopped'
          },
          {
            label: 'Transcription setup',
            value: snapshot.desktop.workerSetupStatus,
            class: 'capitalize'
          },
          {
            label: 'CUDA',
            value: snapshot.desktop.cudaAvailable ? 'Available' : 'CPU fallback'
          },
          {
            label: 'Overlay',
            value:
              snapshot.settings.overlayDisplayMode === 'none'
                ? 'Off'
                : snapshot.settings.overlayDisplayMode === 'minimal'
                  ? 'Minimal'
                  : 'Full'
          },
          {
            label: 'Hotkey',
            value: `${snapshot.desktop.hotkeyRegistered ? 'Registered' : 'Pending'} - ${app.data.displayHotkey(snapshot.settings.hotkey)}`,
            class: 'font-mono text-xs'
          },
          {
            label: 'Capture',
            value: 'File backend',
            border: false
          }
        ],
        errors: [
          snapshot.desktop.hotkeyError,
          snapshot.desktop.workerError,
          snapshot.desktop.cudaError
        ].filter((error): error is string => error !== null)
      },
      glossaryField: app.settings.dashboardGlossaryField,
      storage: {
        outputDirectory: snapshot.settings.outputDirectory,
        databasePath: snapshot.settings.databasePath,
        modelStorageDirectory: snapshot.settings.modelStorageDirectory
      }
    },
    status: {
      loading: app.status.loading.value,
      savingSettings: app.status.savingSettings.value,
      updateChecking: app.status.updateChecking.value,
      updateInstalling: app.status.updateInstalling.value
    },
    actions: {
      checkForUpdates: app.actions.checkForUpdates,
      installUpdate: app.actions.installUpdate,
      checkWorker: app.actions.checkWorker
    }
  };
};

export type DashboardViewModel = ReturnType<typeof useDashboard>;
