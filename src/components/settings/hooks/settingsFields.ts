import type { Signal } from '@preact/signals';
import { open } from '@tauri-apps/plugin-dialog';
import type { JSX } from 'preact';
import { appSnapshotSignal } from '../../../stores/app.store';
import type { AppSettingsUpdate } from '../../../types/desktop';
import {
  captureDeviceOptions,
  type SettingsFolderField,
  type SettingsHotkeyField,
  type SettingsNumberField,
  type SettingsSelectField,
  type SettingsTextareaField,
  type SettingsTextField
} from './settings.helpers';
import { selectFields } from './settingsSelectFields';

export const buildSettingsFields = (
  draft: Signal<AppSettingsUpdate>,
  recordingHotkey: Signal<boolean>
) => {
  const inputHandlers = createInputHandlers(draft);

  return {
    folderFields: folderFields(draft),
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
    label: 'Overlay position',
    value: draft.value.overlayPosition,
    options: ['topLeft', 'topRight', 'bottomLeft', 'bottomRight'],
    onChange: onChange('overlayPosition')
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
    key: 'titlePrompt',
    label: 'Title prompt',
    value: draft.value.titlePrompt,
    onInput: onInput('titlePrompt')
  },
  {
    key: 'summaryPrompt',
    label: 'Summary prompt',
    value: draft.value.summaryPrompt,
    onInput: onInput('summaryPrompt')
  }
];

const toggles = (draft: Signal<AppSettingsUpdate>) => ({
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
