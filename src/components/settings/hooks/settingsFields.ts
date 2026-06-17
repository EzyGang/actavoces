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
import { selectFields } from './settingsSelectFields';

export const buildSettingsFields = (
  draft: Signal<AppSettingsUpdate>,
  recordingHotkey: Signal<boolean>,
  glossaryInput: Signal<string>,
  onDashboardGlossaryChange?: (draft: AppSettingsUpdate) => void
) => {
  const inputHandlers = createInputHandlers(draft);

  return {
    folderFields: folderFields(draft),
    dashboardGlossaryField: glossaryField(draft, glossaryInput, onDashboardGlossaryChange),
    glossaryField: glossaryField(draft, glossaryInput),
    hotkeyField: hotkeyField(draft, recordingHotkey),
    huggingFaceTokenField: {
      key: 'huggingFaceToken',
      label: 'Hugging Face token',
      inputType: 'password',
      value: draft.value.huggingFaceToken,
      onInput: inputHandlers.text('huggingFaceToken')
    } satisfies SettingsTextField,
    textFields: textFields(draft, inputHandlers.text),
    captureSelectFields: captureSelectFields(draft, inputHandlers.select),
    numberFields: numberFields(draft, inputHandlers.number, inputHandlers.optionalNumber),
    selectFields: selectFields(draft, inputHandlers.select),
    textareaFields: textareaFields(draft, inputHandlers.textarea),
    toggles: toggles(draft)
  };
};

const glossaryField = (
  draft: Signal<AppSettingsUpdate>,
  glossaryInput: Signal<string>,
  onChange?: (draft: AppSettingsUpdate) => void
): SettingsGlossaryField => {
  const entries = glossaryEntriesFromContext(draft.value.transcriptionContext);
  const updateEntries = (nextEntries: string[]) => {
    const nextDraft = {
      ...draft.value,
      transcriptionContext: contextFromGlossaryEntries(normalizeGlossaryEntries(nextEntries))
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
    label: 'Transcription glossary',
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
  select:
    (key: keyof AppSettingsUpdate): JSX.GenericEventHandler<HTMLSelectElement> =>
    (event) => {
      draft.value = {
        ...draft.value,
        [key]: event.currentTarget.value
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
  recordingHotkey: Signal<boolean>
): SettingsHotkeyField => ({
  label: 'Global hotkey',
  value: draft.value.hotkey,
  displayValue: displayHotkey(draft.value.hotkey),
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
  onChange: (key: keyof AppSettingsUpdate) => JSX.GenericEventHandler<HTMLSelectElement>
): SettingsSelectField[] => [
  {
    key: 'microphoneDevice',
    label: 'Microphone device',
    value: draft.value.microphoneDevice,
    options: captureDeviceOptions(
      appSnapshotSignal.value.captureDevices.microphones,
      draft.value.microphoneDevice
    ),
    onChange: onChange('microphoneDevice')
  },
  {
    key: 'systemAudioSource',
    label: 'System audio source',
    value: draft.value.systemAudioSource,
    options: captureDeviceOptions(
      appSnapshotSignal.value.captureDevices.systemSources,
      draft.value.systemAudioSource
    ),
    onChange: onChange('systemAudioSource')
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
    onChange: onChange('overlayPosition')
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
    onChange: onChange('overlayDisplayMode')
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
    onInput: ((event) => {
      draft.value = {
        ...draft.value,
        closeToTray: event.currentTarget.checked
      };
    }) satisfies JSX.InputEventHandler<HTMLInputElement>
  },
  launchAtLogin: {
    checked: draft.value.launchAtLogin,
    onInput: ((event) => {
      draft.value = {
        ...draft.value,
        launchAtLogin: event.currentTarget.checked
      };
    }) satisfies JSX.InputEventHandler<HTMLInputElement>
  },
  summaryEnabled: {
    checked: draft.value.summaryEnabled,
    onInput: ((event) => {
      draft.value = {
        ...draft.value,
        summaryEnabled: event.currentTarget.checked
      };
    }) satisfies JSX.InputEventHandler<HTMLInputElement>
  }
});
