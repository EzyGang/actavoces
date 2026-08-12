import type { Signal } from '@preact/signals';
import { open } from '@tauri-apps/plugin-dialog';
import type { JSX } from 'preact';
import { appSnapshotSignal } from '../../../stores/app.store';
import type { AppSettingsUpdate } from '../../../types/desktop';
import { displayHotkey } from '../../../utils/hotkey';
import {
  captureDeviceOptions,
  contextFromGlossaryEntries,
  glossaryEntriesFromContext,
  normalizeGlossaryEntries,
  type SettingsFolderField,
  type SettingsGlossaryField,
  type SettingsHotkeyField,
  type SettingsNumberField,
  type SettingsSelectField,
  type SettingsTextareaField,
  type SettingsTextField
} from './settings.helpers';
import { selectFields, speakerCountField } from './settingsSelectFields';

export const buildSettingsFields = (
  draft: Signal<AppSettingsUpdate>,
  recordingHotkey: Signal<boolean>,
  recordingDictationHotkey: Signal<boolean>,
  glossaryInput: Signal<string>,
  dictationHintsInput: Signal<string>,
  onDashboardGlossaryChange?: (draft: AppSettingsUpdate) => void
) => {
  const inputHandlers = createInputHandlers(draft);

  return {
    folderFields: folderFields(draft),
    dashboardGlossaryField: glossaryField(
      draft,
      glossaryInput,
      'transcriptionContext',
      'Transcription glossary',
      onDashboardGlossaryChange
    ),
    glossaryField: glossaryField(
      draft,
      glossaryInput,
      'transcriptionContext',
      'Transcription glossary'
    ),
    dictationHintsField: glossaryField(
      draft,
      dictationHintsInput,
      'dictationContext',
      'Dictation hints'
    ),
    hotkeyField: hotkeyField(draft, recordingHotkey, 'hotkey', 'Global hotkey'),
    dictationHotkeyField: hotkeyField(
      draft,
      recordingDictationHotkey,
      'dictationHotkey',
      'Dictation shortcut'
    ),
    huggingFaceTokenField: {
      key: 'huggingFaceToken',
      label: 'Hugging Face token',
      inputType: 'password',
      value: draft.value.huggingFaceToken,
      onInput: inputHandlers.text('huggingFaceToken')
    } satisfies SettingsTextField,
    textFields: textFields(draft, inputHandlers.text),
    captureSelectFields: captureSelectFields(draft, inputHandlers.select),
    dictationSelectFields: dictationSelectFields(draft, inputHandlers.select),
    numberFields: numberFields(draft, inputHandlers.number, inputHandlers.optionalNumber),
    selectFields: selectFields(draft, inputHandlers.select),
    speakerCountField: speakerCountField(draft, inputHandlers.select),
    textareaFields: textareaFields(draft, inputHandlers.textarea),
    toggles: toggles(draft)
  };
};

const glossaryField = (
  draft: Signal<AppSettingsUpdate>,
  glossaryInput: Signal<string>,
  contextKey: 'transcriptionContext' | 'dictationContext',
  label: string,
  onChange?: (draft: AppSettingsUpdate) => void
): SettingsGlossaryField => {
  const entries = glossaryEntriesFromContext(draft.value[contextKey]);
  const updateEntries = (nextEntries: string[]) => {
    const nextDraft = {
      ...draft.value,
      [contextKey]: contextFromGlossaryEntries(normalizeGlossaryEntries(nextEntries))
    };

    draft.value = nextDraft;
    onChange?.(nextDraft);
  };
  const addEntry = () => {
    const nextEntry = glossaryInput.value.trim();

    if (nextEntry.length === 0) {
      return;
    }

    updateEntries([...entries, nextEntry]);
    glossaryInput.value = '';
  };

  return {
    label,
    hint: 'Optional words, names, products, acronyms, and short phrases to hint during transcription.',
    placeholder: 'Type a term or phrase, then press Enter',
    value: glossaryInput.value,
    entries: entries.map((entry) => ({
      value: entry,
      onRemove: () => {
        updateEntries(entries.filter((candidate) => candidate !== entry));
      }
    })),
    onInput: (event) => {
      glossaryInput.value = event.currentTarget.value;
    },
    onKeyDown: (event) => {
      if (event.key !== 'Enter') {
        return;
      }

      event.preventDefault();
      addEntry();
    },
    onAdd: addEntry
  };
};

const createInputHandlers = (draft: Signal<AppSettingsUpdate>) => ({
  text:
    (key: keyof AppSettingsUpdate): JSX.InputEventHandler<HTMLInputElement> =>
    (event) => {
      draft.value = {
        ...draft.value,
        [key]: event.currentTarget.value
      };
    },
  number:
    (key: keyof AppSettingsUpdate): JSX.InputEventHandler<HTMLInputElement> =>
    (event) => {
      const nextValue = Number(event.currentTarget.value);

      draft.value = {
        ...draft.value,
        [key]: Number.isNaN(nextValue) ? 0 : nextValue
      };
    },
  optionalNumber:
    (key: keyof AppSettingsUpdate): JSX.InputEventHandler<HTMLInputElement> =>
    (event) => {
      const rawValue = event.currentTarget.value.trim();

      draft.value = {
        ...draft.value,
        [key]: rawValue.length === 0 ? null : Number(rawValue)
      };
    },
  textarea:
    (key: keyof AppSettingsUpdate): JSX.InputEventHandler<HTMLTextAreaElement> =>
    (event) => {
      draft.value = {
        ...draft.value,
        [key]: event.currentTarget.value
      };
    },
  select: (key: keyof AppSettingsUpdate) => (value: string) => {
    draft.value = {
      ...draft.value,
      [key]: value
    };
  }
});

const folderFields = (draft: Signal<AppSettingsUpdate>): SettingsFolderField[] => [
  {
    key: 'outputDirectory',
    label: 'Output directory',
    value: draft.value.outputDirectory,
    onSelect: folderSelectHandler(draft, 'outputDirectory')
  },
  {
    key: 'modelStorageDirectory',
    label: 'Model storage',
    value: draft.value.modelStorageDirectory,
    onSelect: folderSelectHandler(draft, 'modelStorageDirectory')
  }
];

const folderSelectHandler =
  (draft: Signal<AppSettingsUpdate>, key: SettingsFolderField['key']) => async () => {
    const selectedPath = await open({
      defaultPath: draft.value[key],
      directory: true,
      multiple: false
    });

    if (typeof selectedPath !== 'string') {
      return;
    }

    draft.value = {
      ...draft.value,
      [key]: selectedPath
    };
  };

const hotkeyField = (
  draft: Signal<AppSettingsUpdate>,
  recordingHotkey: Signal<boolean>,
  key: 'hotkey' | 'dictationHotkey',
  label: string
): SettingsHotkeyField => ({
  label,
  value: draft.value[key],
  displayValue: displayHotkey(draft.value[key]),
  recording: recordingHotkey.value,
  onCapture: () => {
    recordingHotkey.value = true;
  }
});

const textFields = (
  draft: Signal<AppSettingsUpdate>,
  onInput: (key: keyof AppSettingsUpdate) => JSX.InputEventHandler<HTMLInputElement>
): SettingsTextField[] => [
  {
    key: 'providerBaseUrl',
    label: 'Provider base URL',
    value: draft.value.providerBaseUrl,
    onInput: onInput('providerBaseUrl')
  },
  {
    key: 'providerModel',
    label: 'Provider model',
    value: draft.value.providerModel,
    onInput: onInput('providerModel')
  },
  {
    key: 'providerApiKey',
    label: 'Provider API key',
    inputType: 'password',
    value: draft.value.providerApiKey,
    onInput: onInput('providerApiKey')
  }
];

const captureSelectFields = (
  draft: Signal<AppSettingsUpdate>,
  onValueChange: (key: keyof AppSettingsUpdate) => (value: string) => void
): SettingsSelectField[] => [
  {
    key: 'microphoneDevice',
    label: 'Microphone device',
    value: draft.value.microphoneDevice,
    options: captureDeviceOptions(
      appSnapshotSignal.value.captureDevices.microphones,
      draft.value.microphoneDevice
    ),
    onValueChange: onValueChange('microphoneDevice')
  },
  {
    key: 'systemAudioSource',
    label: 'System audio source',
    value: draft.value.systemAudioSource,
    options: captureDeviceOptions(
      appSnapshotSignal.value.captureDevices.systemSources,
      draft.value.systemAudioSource
    ),
    onValueChange: onValueChange('systemAudioSource')
  },
  {
    key: 'overlayPosition',
    label: 'Status window position',
    value: draft.value.overlayPosition,
    options: [
      { value: 'topLeft', label: 'Top left' },
      { value: 'topRight', label: 'Top right' },
      { value: 'bottomLeft', label: 'Bottom left' },
      { value: 'bottomRight', label: 'Bottom right' }
    ],
    onValueChange: onValueChange('overlayPosition')
  },
  {
    key: 'overlayDisplayMode',
    label: 'Status window',
    value: draft.value.overlayDisplayMode,
    options: [
      { value: 'full', label: 'Full' },
      { value: 'minimal', label: 'Minimal' },
      { value: 'none', label: 'None' }
    ],
    onValueChange: onValueChange('overlayDisplayMode')
  }
];

const dictationSelectFields = (
  draft: Signal<AppSettingsUpdate>,
  onValueChange: (key: keyof AppSettingsUpdate) => (value: string) => void
): SettingsSelectField[] => [
  {
    key: 'dictationShortcutMode',
    label: 'Shortcut mode',
    value: draft.value.dictationShortcutMode,
    options: [
      { value: 'toggle', label: 'Toggle' },
      { value: 'pushToTalk', label: 'Push to talk' }
    ],
    onValueChange: onValueChange('dictationShortcutMode')
  },
  {
    key: 'dictationWhisperModel',
    label: 'Whisper model',
    value: draft.value.dictationWhisperModel,
    options: [
      { value: 'small', label: 'small' },
      { value: 'medium', label: 'medium' },
      { value: 'large-v3', label: 'large-v3' },
      { value: 'distil-large-v3', label: 'distil-large-v3' }
    ],
    onValueChange: onValueChange('dictationWhisperModel')
  },
  {
    key: 'dictationLanguage',
    label: 'Language',
    value: draft.value.dictationLanguage,
    options: [
      { value: 'en', label: 'English' },
      { value: 'ru', label: 'Russian' },
      { value: 'uk', label: 'Ukrainian' },
      { value: 'es', label: 'Spanish' }
    ],
    onValueChange: onValueChange('dictationLanguage')
  },
  {
    key: 'dictationOverlayPosition',
    label: 'Status window position',
    value: draft.value.dictationOverlayPosition,
    options: [
      { value: 'topLeft', label: 'Top left' },
      { value: 'topRight', label: 'Top right' },
      { value: 'bottomLeft', label: 'Bottom left' },
      { value: 'bottomRight', label: 'Bottom right' }
    ],
    onValueChange: onValueChange('dictationOverlayPosition')
  },
  {
    key: 'dictationOverlayDisplayMode',
    label: 'Status window',
    value: draft.value.dictationOverlayDisplayMode,
    options: [
      { value: 'full', label: 'Full' },
      { value: 'minimal', label: 'Minimal' },
      { value: 'none', label: 'None' }
    ],
    onValueChange: onValueChange('dictationOverlayDisplayMode')
  }
];

const numberFields = (
  draft: Signal<AppSettingsUpdate>,
  onInput: (key: keyof AppSettingsUpdate) => JSX.InputEventHandler<HTMLInputElement>,
  onOptionalInput: (key: keyof AppSettingsUpdate) => JSX.InputEventHandler<HTMLInputElement>
): SettingsNumberField[] => [
  {
    key: 'sampleRate',
    label: 'Sample rate',
    value: draft.value.sampleRate,
    onInput: onInput('sampleRate')
  },
  {
    key: 'exactSpeakers',
    label: 'Exact speakers',
    value: draft.value.exactSpeakers ?? 0,
    onInput: onOptionalInput('exactSpeakers')
  },
  {
    key: 'minSpeakers',
    label: 'Minimum speakers',
    value: draft.value.minSpeakers ?? 0,
    onInput: onOptionalInput('minSpeakers')
  },
  {
    key: 'maxSpeakers',
    label: 'Maximum speakers',
    value: draft.value.maxSpeakers ?? 0,
    onInput: onOptionalInput('maxSpeakers')
  }
];

const textareaFields = (
  draft: Signal<AppSettingsUpdate>,
  onInput: (key: keyof AppSettingsUpdate) => JSX.InputEventHandler<HTMLTextAreaElement>
): SettingsTextareaField[] => [
  {
    key: 'summaryPrompt',
    label: 'Summary prompt',
    value: draft.value.summaryPrompt,
    onInput: onInput('summaryPrompt')
  }
];

const toggles = (draft: Signal<AppSettingsUpdate>) => ({
  closeToTray: {
    checked: draft.value.closeToTray,
    onCheckedChange: (checked: boolean) => {
      draft.value = {
        ...draft.value,
        closeToTray: checked
      };
    }
  },
  launchAtLogin: {
    checked: draft.value.launchAtLogin,
    onCheckedChange: (checked: boolean) => {
      draft.value = {
        ...draft.value,
        launchAtLogin: checked
      };
    }
  },
  summaryEnabled: {
    checked: draft.value.summaryEnabled,
    onCheckedChange: (checked: boolean) => {
      draft.value = {
        ...draft.value,
        summaryEnabled: checked
      };
    }
  }
});
