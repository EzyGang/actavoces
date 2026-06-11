import { renderHook, waitFor } from '@testing-library/preact';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useRecordingOverlay } from '../components/recording-overlay/hooks/useRecordingOverlay.hook';
import { getAppSnapshot } from '../services/desktop/app.service';
import type { AppSettings, AppSnapshot } from '../types/desktop';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => undefined))
}));

vi.mock('../services/desktop/app.service', () => ({
  getAppSnapshot: vi.fn(),
  stopRecording: vi.fn()
}));

const baseSettings: AppSettings = {
  outputDirectory: '/tmp/actavoces/records',
  databasePath: '/tmp/actavoces/actavoces.sqlite',
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
  computeType: 'auto',
  modelStorageDirectory: '/tmp/actavoces/models',
  diarizationBackend: 'sortformer',
  speakerCountMode: 'automatic',
  exactSpeakers: null,
  minSpeakers: null,
  maxSpeakers: null,
  huggingFaceTokenConfigured: false,
  diarizationSetupSkipped: true,
  diarizationRuntimeReady: false,
  summaryProviderConfigured: false,
  providerApiKeyConfigured: false,
  summaryEnabled: false,
  providerBaseUrl: 'https://api.openai.com/v1',
  providerModel: '',
  summaryPrompt: 'Summary'
};

const makeSnapshot = (settings: Partial<AppSettings> = {}): AppSnapshot => ({
  activeRecording: null,
  recordings: [],
  jobs: [],
  models: [],
  captureDevices: {
    microphones: [],
    systemSources: []
  },
  desktop: {
    overlayVisible: true,
    hotkeyRegistered: true,
    hotkeyError: null,
    workerRunning: false,
    workerHealthOk: false,
    workerError: null,
    workerSetupStatus: 'ready',
    workerSetupStep: '',
    workerSetupError: null,
    cudaAvailable: false,
    cudaError: null
  },
  settings: {
    ...baseSettings,
    ...settings
  }
});

describe('useRecordingOverlay hook', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {}
    });
    Object.defineProperty(window, 'innerWidth', {
      configurable: true,
      value: 64
    });
  });

  it('uses the saved full display mode after snapshot even in a narrow window', async () => {
    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot({ overlayDisplayMode: 'full' }));

    const { result } = renderHook(() => useRecordingOverlay());

    await waitFor(() => {
      expect(result.current.status.displayMode.value).toBe('full');
    });
  });
});
