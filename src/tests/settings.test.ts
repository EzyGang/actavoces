import { describe, expect, it } from 'vitest';
import type { AppSettingsUpdate } from '../types/desktop';
import { validateSettingsDraft } from '../utils/settings';

const validSettings: AppSettingsUpdate = {
  outputDirectory: '/tmp/actavoces/records',
  hotkey: 'CommandOrControl+Shift+Space',
  overlayPosition: 'topLeft',
  overlayDisplayMode: 'full',
  closeToTray: true,
  launchAtLogin: false,
  microphoneDevice: 'Default microphone',
  systemAudioSource: 'Default system output',
  sampleRate: 48000,
  whisperModel: 'medium',
  transcriptionLanguage: 'auto',
  transcriptionContext: '',
  computeType: 'auto',
  modelStorageDirectory: '/tmp/actavoces/models',
  diarizationBackend: 'sortformer',
  speakerCountMode: 'automatic',
  exactSpeakers: null,
  minSpeakers: null,
  maxSpeakers: null,
  huggingFaceToken: '',
  diarizationSetupSkipped: true,
  summaryEnabled: false,
  providerBaseUrl: 'https://api.openai.com/v1',
  providerModel: '',
  providerApiKey: '',
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
      'Provider model is required when summaries are enabled.'
    ]);
  });

  it('accepts summaries when provider details are present without an API key', () => {
    expect(
      validateSettingsDraft({
        ...validSettings,
        summaryEnabled: true,
        providerModel: 'gpt-4o-mini'
      })
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

  it('requires CUDA runtime libraries when explicit CUDA compute is selected', () => {
    expect(
      validateSettingsDraft({
        ...validSettings,
        computeType: 'cuda'
      })
    ).toContain(
      'CUDA runtime is not ready. Install CUDA drivers, cuBLAS for CUDA 12, and cuDNN 9 for CUDA 12.'
    );

    expect(
      validateSettingsDraft(
        {
          ...validSettings,
          computeType: 'cuda'
        },
        false,
        true
      )
    ).toEqual([]);
  });
});
