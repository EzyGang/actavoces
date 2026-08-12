import { type Signal, useComputed, useSignal } from '@preact/signals';
import {
  checkWorkerHealth,
  getAppSnapshot,
  installTranscriptionModel,
  refreshModelInventory
} from '../../../services/desktop/app.service';
import { appSnapshotSignal } from '../../../stores/app.store';
import type { AppSettingsUpdate, AppSnapshot } from '../../../types/desktop';
import { errorMessage } from '../../app-shell/hooks/appRuntime.helpers';

interface UseWorkerRuntimeInput {
  loading: Signal<boolean>;
  settingsDraft: Signal<AppSettingsUpdate>;
  setError: (message: string | null) => void;
  setSnapshot: (snapshot: AppSnapshot) => void;
}

export const useWorkerRuntime = ({
  loading,
  settingsDraft,
  setError,
  setSnapshot
}: UseWorkerRuntimeInput) => {
  const installingModel = useSignal(false);
  const selectedModel = useComputed(
    () =>
      appSnapshotSignal.value.models.find(
        (model) => model.name === settingsDraft.value.whisperModel
      ) ?? null
  );
  const selectedDictationModel = useComputed(
    () =>
      appSnapshotSignal.value.models.find(
        (model) => model.name === settingsDraft.value.dictationWhisperModel
      ) ?? null
  );

  const checkWorker = async () => {
    loading.value = true;
    setError(null);

    try {
      await checkWorkerHealth();
      setSnapshot(await getAppSnapshot());
    } catch (error) {
      setError(errorMessage(error, 'Unable to check worker'));
    } finally {
      loading.value = false;
    }
  };

  const refreshModels = async () => {
    loading.value = true;
    setError(null);

    try {
      setSnapshot(await refreshModelInventory());
    } catch (error) {
      setError(errorMessage(error, 'Unable to refresh models'));
    } finally {
      loading.value = false;
    }
  };

  const installSelectedModel = async () => {
    installingModel.value = true;
    setError(null);

    try {
      setSnapshot(await installTranscriptionModel(settingsDraft.value.whisperModel));
    } catch (error) {
      setError(errorMessage(error, 'Unable to install model'));
    } finally {
      installingModel.value = false;
    }
  };
  const installSelectedDictationModel = async () => {
    installingModel.value = true;
    setError(null);

    try {
      setSnapshot(await installTranscriptionModel(settingsDraft.value.dictationWhisperModel));
    } catch (error) {
      setError(errorMessage(error, 'Unable to install dictation model'));
    } finally {
      installingModel.value = false;
    }
  };

  return {
    installingModel,
    selectedModel,
    selectedDictationModel,
    actions: {
      checkWorker,
      refreshModels,
      installSelectedModel,
      installSelectedDictationModel
    }
  };
};
