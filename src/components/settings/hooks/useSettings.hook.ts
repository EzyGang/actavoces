import { useComputed, useSignal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import {
  clearHuggingFaceToken,
  clearSummaryProviderApiKey,
  updateAppSettings
} from '../../../services/desktop/app.service';
import { appSnapshotSignal } from '../../../stores/app.store';
import type { AppSettings, AppSettingsUpdate, AppSnapshot } from '../../../types/desktop';
import { validateSettingsDraft } from '../../../utils/settings';
import { errorMessage } from '../../app-shell/hooks/appRuntime.helpers';
import {
  buildSettingsUpdate,
  hotkeyFromKeyboardEvent,
  settingsDraftChanged
} from './settings.helpers';
import { buildSettingsFields } from './settingsFields';

interface UseSettingsInput {
  setError: (message: string | null) => void;
  setSnapshot: (snapshot: AppSnapshot) => void;
}

export const useSettings = ({ setError, setSnapshot }: UseSettingsInput) => {
  const savingSettings = useSignal(false);
  const recordingHotkey = useSignal(false);
  const draft = useSignal<AppSettingsUpdate>(buildSettingsUpdate(appSnapshotSignal.value.settings));
  const validationErrors = useComputed(() =>
    validateSettingsDraft(
      draft.value,
      appSnapshotSignal.value.settings.providerApiKeyConfigured,
      appSnapshotSignal.value.desktop.cudaAvailable
    )
  );
  const hasUnsavedSettings = useComputed(() =>
    settingsDraftChanged(draft.value, appSnapshotSignal.value.settings)
  );

  const resetDraft = (settings: AppSettings) => {
    draft.value = buildSettingsUpdate(settings);
  };

  const saveSettings = async () => {
    if (validationErrors.value.length > 0) {
      setError(validationErrors.value.join(' '));

      return;
    }

    savingSettings.value = true;
    setError(null);

    try {
      setSnapshot(await updateAppSettings(draft.value));
    } catch (error) {
      setError(errorMessage(error, 'Unable to save settings'));
    } finally {
      savingSettings.value = false;
    }
  };

  const clearProviderApiKey = async () => {
    savingSettings.value = true;
    setError(null);

    try {
      setSnapshot(await clearSummaryProviderApiKey());
    } catch (error) {
      setError(errorMessage(error, 'Unable to clear provider API key'));
    } finally {
      savingSettings.value = false;
    }
  };

  const clearHuggingFaceTokenAction = async () => {
    savingSettings.value = true;
    setError(null);

    try {
      setSnapshot(await clearHuggingFaceToken());
    } catch (error) {
      setError(errorMessage(error, 'Unable to clear Hugging Face token'));
    } finally {
      savingSettings.value = false;
    }
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
      draft.value = {
        ...draft.value,
        hotkey: nextHotkey
      };
      recordingHotkey.value = false;
    };

    window.addEventListener('keydown', handleKeyDown, { capture: true });

    return () => {
      window.removeEventListener('keydown', handleKeyDown, { capture: true });
    };
  }, [recordingHotkey.value]);

  return {
    draft,
    savingSettings,
    validationErrors,
    hasUnsavedSettings,
    resetDraft,
    fields: buildSettingsFields(draft, recordingHotkey),
    actions: {
      saveSettings,
      clearProviderApiKey,
      clearHuggingFaceToken: clearHuggingFaceTokenAction
    }
  };
};
