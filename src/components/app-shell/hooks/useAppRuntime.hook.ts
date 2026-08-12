import { type Signal, useComputed, useSignal } from '@preact/signals';
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'preact/hooks';
import {
  bootstrapWorkerRuntime,
  cancelActiveDictation,
  getAppSnapshot,
  getDictationStatus,
  setupDiarizationRuntime,
  skipDiarizationSetup,
  writeDiagnosticLog
} from '../../../services/desktop/app.service';
import { appSnapshotSignal } from '../../../stores/app.store';
import type {
  AppSettingsUpdate,
  AppSnapshot,
  DictationStateUpdate,
  SortformerSetupProgress,
  WorkerSetupProgress
} from '../../../types/desktop';
import {
  diagnosticsMessage,
  errorMessage,
  initialSetupProgress,
  isTauriRuntime,
  setupProgressFromSnapshot,
  startupDelay
} from './appRuntime.helpers';

interface UseAppRuntimeInput {
  loading: Signal<boolean>;
  settingsDraft: Signal<AppSettingsUpdate>;
  setError: (message: string | null) => void;
  setSnapshot: (snapshot: AppSnapshot) => void;
}

export const useAppRuntime = ({
  loading,
  settingsDraft,
  setError,
  setSnapshot
}: UseAppRuntimeInput) => {
  const setupProgress = useSignal<WorkerSetupProgress>(initialSetupProgress);
  const sortformerProgress = useSignal<SortformerSetupProgress | null>(null);
  const setupRunning = useSignal(false);
  const bootstrapRequested = useSignal(false);
  const dictationStatus = useSignal<DictationStateUpdate | null>(null);
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

  const loadSnapshot = async (attempts = 1) => {
    loading.value = true;
    setError(null);

    for (let attempt = 1; attempt <= attempts; attempt += 1) {
      try {
        const snapshot = await getAppSnapshot();

        setSnapshot(snapshot);
        setupProgress.value = setupProgressFromSnapshot(snapshot);
        loading.value = false;

        return;
      } catch (error) {
        const message = errorMessage(error, 'Desktop backend unavailable');

        if (message === 'ActaVoces is still starting' && attempt < attempts) {
          await startupDelay(100);
          continue;
        }

        setError(message);
        loading.value = false;

        return;
      }
    }
  };

  const runBootstrap = async () => {
    setupRunning.value = true;
    setError(null);
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

      setError(message);
      setupProgress.value = {
        status: 'failed',
        step: 'Worker setup failed',
        error: message
      };
    } finally {
      setupRunning.value = false;
    }
  };

  const setupDiarization = async () => {
    setupRunning.value = true;
    setError(null);
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

      setError(message);
      setupProgress.value = {
        status: 'failed',
        step: 'Speaker diarization setup failed',
        error: message
      };
    } finally {
      setupRunning.value = false;
    }
  };

  const skipDiarization = async () => {
    setupRunning.value = true;
    setError(null);

    try {
      setSnapshot(await skipDiarizationSetup());
    } catch (error) {
      setError(errorMessage(error, 'Unable to skip speaker diarization setup'));
    } finally {
      setupRunning.value = false;
    }
  };

  const cancelDictation = async () => {
    try {
      dictationStatus.value = await cancelActiveDictation();
    } catch (error) {
      setError(errorMessage(error, 'Unable to cancel dictation'));
    }
  };

  useRuntimeEffects({
    bootstrapRequested,
    dictationStatus,
    loading,
    setupProgress,
    sortformerProgress,
    setError,
    setSnapshot,
    loadSnapshot,
    runBootstrap
  });

  return {
    setupProgress,
    sortformerProgress,
    setupRunning,
    setupReady,
    needsDiarizationSetup,
    dictationStatus,
    actions: {
      retrySetup: runBootstrap,
      setupDiarization,
      skipDiarizationSetup: skipDiarization,
      cancelDictation
    }
  };
};

const useRuntimeEffects = ({
  bootstrapRequested,
  dictationStatus,
  loading,
  setupProgress,
  sortformerProgress,
  setError,
  setSnapshot,
  loadSnapshot,
  runBootstrap
}: {
  bootstrapRequested: Signal<boolean>;
  dictationStatus: Signal<DictationStateUpdate | null>;
  loading: Signal<boolean>;
  setupProgress: Signal<WorkerSetupProgress>;
  sortformerProgress: Signal<SortformerSetupProgress | null>;
  setError: (message: string | null) => void;
  setSnapshot: (snapshot: AppSnapshot) => void;
  loadSnapshot: (attempts?: number) => Promise<void>;
  runBootstrap: () => Promise<void>;
}) => {
  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    void getDictationStatus()
      .then((status) => {
        dictationStatus.value = status;
      })
      .catch(() => {});

    const snapshotListener = listen<AppSnapshot>('app-snapshot-updated', (event) => {
      setSnapshot(event.payload);
      setupProgress.value = setupProgressFromSnapshot(event.payload);
      loading.value = false;
    });
    const dictationListener = listen<DictationStateUpdate>('dictation-state-update', (event) => {
      dictationStatus.value = event.payload;
    });
    const errorListener = listen<string>('app-error', (event) => {
      setError(event.payload);
    });
    const setupListener = listen<WorkerSetupProgress>('worker-setup-progress', (event) => {
      setupProgress.value = event.payload;
    });
    const sortformerListener = listen<SortformerSetupProgress>(
      'sortformer-diarization-progress',
      (event) => {
        sortformerProgress.value = event.payload;

        if (event.payload.status === 'ready') {
          window.setTimeout(() => {
            if (sortformerProgress.value?.status === 'ready') {
              sortformerProgress.value = null;
            }
          }, 4000);
        }
      }
    );

    return () => {
      void snapshotListener.then((unlisten) => unlisten());
      void dictationListener.then((unlisten) => unlisten());
      void errorListener.then((unlisten) => unlisten());
      void setupListener.then((unlisten) => unlisten());
      void sortformerListener.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    const handleWindowError = (event: ErrorEvent) => {
      void writeDiagnosticLog({
        event: 'frontend.error',
        message: diagnosticsMessage(event.error ?? event.message)
      });
    };
    const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
      void writeDiagnosticLog({
        event: 'frontend.unhandledRejection',
        message: diagnosticsMessage(event.reason)
      });
    };

    window.addEventListener('error', handleWindowError);
    window.addEventListener('unhandledrejection', handleUnhandledRejection);

    return () => {
      window.removeEventListener('error', handleWindowError);
      window.removeEventListener('unhandledrejection', handleUnhandledRejection);
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) {
      void loadSnapshot();

      return;
    }

    void loadSnapshot(20);
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
};
