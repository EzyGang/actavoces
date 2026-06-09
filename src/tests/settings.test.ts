import { describe, expect, it } from 'vitest';
import type { AppSettingsUpdate } from '../types/desktop';
import { validateSettingsDraft } from '../utils/settings';

const validSettings: AppSettingsUpdate = {
  outputDirectory: '/tmp/actavoces/records',
  hotkey: 'CommandOrControl+Shift+Space',
  overlayPosition: 'topLeft',
  launchAtLogin: false,
  microphoneDevice: 'Default microphone',
  systemAudioSource: 'Default system output',
  sampleRate: 48000,
  whisperModel: 'medium.en',
  transcriptionLanguage: 'auto',
  computeType: 'auto',
  modelStorageDirectory: '/tmp/actavoces/models',
  diarizationBackend: 'nemoWhisper',
  speakerCountMode: 'automatic',
  exactSpeakers: null,
  minSpeakers: null,
  maxSpeakers: null,
  summaryEnabled: false,
  providerBaseUrl: 'https://api.openai.com/v1',
  providerModel: '',
  providerApiKey: '',
  titlePrompt: 'Title',
  summaryPrompt: 'Summary'
};

describe('settings validation', () => {
  it('accepts the default local-only settings payload', () => {
    expect(validateSettingsDraft(validSettings)).toEqual([]);
  });

  it('requires provider details when summaries are enabled', () => {
    expect(
      validateSettingsDraft({
        ...validSettings,
        summaryEnabled: true,
        providerBaseUrl: '',
        providerModel: ''
      })
    ).toEqual([
      'Provider base URL is required when summaries are enabled.',
      'Provider model is required when summaries are enabled.',
      'Provider API key is required when summaries are enabled.'
    ]);
  });

  it('accepts summaries when provider details and an existing key are present', () => {
    expect(
      validateSettingsDraft(
        {
          ...validSettings,
          summaryEnabled: true,
          providerModel: 'gpt-4o-mini'
        },
        true
      )
    ).toEqual([]);
  });

  it('validates exact and ranged speaker configuration', () => {
    expect(
      validateSettingsDraft({
        ...validSettings,
        speakerCountMode: 'exact',
        exactSpeakers: 0
      })
    ).toContain('Exact speaker count must be greater than zero.');

    expect(
      validateSettingsDraft({
        ...validSettings,
        speakerCountMode: 'range',
        minSpeakers: 4,
        maxSpeakers: 2
      })
    ).toContain('Speaker range must include a valid minimum and maximum.');
  });
});
