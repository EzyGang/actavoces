import type { JSX } from 'preact';
import type { AppSettings, AppSettingsUpdate, CaptureDeviceInfo } from '../../../types/desktop';

export interface SettingsTextField {
  key: keyof AppSettingsUpdate;
  label: string;
  inputType?: 'text' | 'password';
  value: string;
  onInput: JSX.InputEventHandler<HTMLInputElement>;
}

export interface SettingsFolderField {
  key: 'outputDirectory' | 'modelStorageDirectory';
  label: string;
  value: string;
  onSelect: () => void;
}

export interface SettingsHotkeyField {
  label: string;
  value: string;
  displayValue: string;
  recording: boolean;
  onCapture: () => void;
}

export interface SettingsNumberField {
  key: keyof AppSettingsUpdate;
  label: string;
  value: number;
  onInput: JSX.InputEventHandler<HTMLInputElement>;
}

export interface SettingsTextareaField {
  key: keyof AppSettingsUpdate;
  label: string;
  value: string;
  onInput: JSX.InputEventHandler<HTMLTextAreaElement>;
}

export interface SettingsSelectField {
  key: keyof AppSettingsUpdate;
  label: string;
  value: string;
  options: SettingsSelectOption[];
  onChange: JSX.GenericEventHandler<HTMLSelectElement>;
  hint?: SettingsFieldHint;
}

export interface SettingsSelectOption {
  value: string;
  label: string;
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

export const buildSettingsUpdate = (settings: AppSettings): AppSettingsUpdate => ({
  outputDirectory: settings.outputDirectory,
  hotkey: settings.hotkey,
  overlayPosition: settings.overlayPosition,
  overlayDisplayMode: settings.overlayDisplayMode,
  closeToTray: settings.closeToTray,
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
  summaryPrompt: settings.summaryPrompt
});

export const settingsDraftChanged = (draft: AppSettingsUpdate, settings: AppSettings): boolean =>
  JSON.stringify(draft) !== JSON.stringify(buildSettingsUpdate(settings));

export const captureDeviceOptions = (
  devices: CaptureDeviceInfo[],
  selectedValue: string
): SettingsSelectOption[] => {
  const options = devices.map((device) => ({
    value: device.name,
    label: device.name
  }));

  if (
    selectedValue.trim().length > 0 &&
    !options.some((option) => option.value === selectedValue)
  ) {
    return [
      {
        value: selectedValue,
        label: selectedValue
      },
      ...options
    ];
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

export const hotkeyFromKeyboardEvent = (event: KeyboardEvent): string | null => {
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
