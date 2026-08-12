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
  const recordingDictationHotkey = useSignal(false);
  const glossaryInput = useSignal('');
  const dictationHintsInput = useSignal('');
  const activeTab = useSignal<'recording' | 'dictation'>('recording');
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
    glossaryInput.value = '';
    dictationHintsInput.value = '';
    recordingHotkey.value = false;
    recordingDictationHotkey.value = false;
  };

  const persistSettings = async (nextDraft: AppSettingsUpdate) => {
    const errors = validateSettingsDraft(
      nextDraft,
      appSnapshotSignal.value.settings.providerApiKeyConfigured,
      appSnapshotSignal.value.desktop.cudaAvailable
    );

    if (errors.length > 0) {
      setError(errors.join(' '));

      return;
    }

    savingSettings.value = true;
    setError(null);

    try {
      const snapshot = await updateAppSettings(nextDraft);

      setSnapshot(snapshot);
      resetDraft(snapshot.settings);
    } catch (error) {
      setError(errorMessage(error, 'Unable to save settings'));
    } finally {
      savingSettings.value = false;
    }
  };

  const saveSettings = async () => {
    await persistSettings(draft.value);
  };

  const clearProviderApiKey = async () => {
    savingSettings.value = true;
    setError(null);

    try {
      const snapshot = await clearSummaryProviderApiKey();

      setSnapshot(snapshot);
      resetDraft(snapshot.settings);
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
      const snapshot = await clearHuggingFaceToken();

      setSnapshot(snapshot);
      resetDraft(snapshot.settings);
    } catch (error) {
      setError(errorMessage(error, 'Unable to clear Hugging Face token'));
    } finally {
      savingSettings.value = false;
    }
  };

  useEffect(() => {
    if (!recordingHotkey.value && !recordingDictationHotkey.value) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        recordingHotkey.value = false;
        recordingDictationHotkey.value = false;

        return;
      }

      const nextHotkey = hotkeyFromKeyboardEvent(event);

      if (!nextHotkey) {
        return;
      }

      event.preventDefault();
      draft.value = {
        ...draft.value,
        [recordingDictationHotkey.value ? 'dictationHotkey' : 'hotkey']: nextHotkey
      };
      recordingHotkey.value = false;
      recordingDictationHotkey.value = false;
    };

    window.addEventListener('keydown', handleKeyDown, { capture: true });

    return () => {
      window.removeEventListener('keydown', handleKeyDown, { capture: true });
    };
  }, [recordingHotkey.value, recordingDictationHotkey.value]);

  return {
    draft,
    savingSettings,
    validationErrors,
    activeTab,
    hasUnsavedSettings,
    resetDraft,
    fields: buildSettingsFields(
      draft,
      recordingHotkey,
      recordingDictationHotkey,
      glossaryInput,
      dictationHintsInput,
      (nextDraft) => {
        void persistSettings(nextDraft);
      }
    ),
    actions: {
      showRecordingTab: () => {
        activeTab.value = 'recording';
      },
      showDictationTab: () => {
        activeTab.value = 'dictation';
      },
      saveSettings,
      clearProviderApiKey,
      clearHuggingFaceToken: clearHuggingFaceTokenAction
    }
  };
};
