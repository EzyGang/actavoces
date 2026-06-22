import { type EventCallback, listen } from '@tauri-apps/api/event';
import { act, renderHook, waitFor } from '@testing-library/preact';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useRecordingOverlay } from '../components/recording-overlay/hooks/useRecordingOverlay.hook';
import { getAppSnapshot } from '../services/desktop/app.service';
import type { AppSettings, AppSnapshot } from '../types/desktop';

const eventListeners = vi.hoisted(() => new Map<string, EventCallback<unknown>>());

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn()
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
  modelRecommendation: {
    recommendedModel: 'medium',
    reason: 'CPU-only system has enough resources for medium',
    userOverridden: false
  },
  transcriptionLanguage: 'auto',
  transcriptionContext: '',
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

const makeSnapshot = (settings: Partial<AppSettings> = {}, overlayVisible = true): AppSnapshot => ({
  activeRecording: null,
  recordings: [],
  jobs: [],
  models: [],
  captureDevices: {
    microphones: [],
    systemSources: []
  },
  desktop: {
    overlayVisible,
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
    eventListeners.clear();
    vi.mocked(listen).mockImplementation((eventName: string, callback: EventCallback<unknown>) => {
      eventListeners.set(eventName, callback);

      return Promise.resolve(() => undefined);
    });
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
    expect(listen).toHaveBeenCalledWith('recording-overlay-sync', expect.any(Function));
  });

  it('uses no display mode when the snapshot says the overlay is hidden', async () => {
    vi.mocked(getAppSnapshot).mockResolvedValue(
      makeSnapshot({ overlayDisplayMode: 'full' }, false)
    );

    const { result } = renderHook(() => useRecordingOverlay());

    await waitFor(() => {
      expect(result.current.status.displayMode.value).toBe('none');
    });
  });

  it('updates display mode from overlay sync events', async () => {
    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot({ overlayDisplayMode: 'full' }));

    const { result } = renderHook(() => useRecordingOverlay());

    await waitFor(() => {
      expect(eventListeners.has('recording-overlay-sync')).toBe(true);
    });

    const overlaySync = eventListeners.get('recording-overlay-sync');

    overlaySync?.({
      event: 'recording-overlay-sync',
      id: 1,
      payload: { visible: true, displayMode: 'minimal' }
    });
    expect(result.current.status.displayMode.value).toBe('minimal');

    overlaySync?.({
      event: 'recording-overlay-sync',
      id: 2,
      payload: { visible: true, displayMode: 'full' }
    });
    expect(result.current.status.displayMode.value).toBe('full');

    overlaySync?.({
      event: 'recording-overlay-sync',
      id: 3,
      payload: { visible: false, displayMode: 'full' }
    });
    expect(result.current.status.displayMode.value).toBe('none');
  });

  it('does not let a stale initial snapshot overwrite overlay sync state', async () => {
    let resolveSnapshot: (snapshot: AppSnapshot) => void = () => undefined;
    const snapshotPromise = new Promise<AppSnapshot>((resolve) => {
      resolveSnapshot = resolve;
    });

    vi.mocked(getAppSnapshot).mockReturnValue(snapshotPromise);

    const { result } = renderHook(() => useRecordingOverlay());

    await waitFor(() => {
      expect(eventListeners.has('recording-overlay-sync')).toBe(true);
    });

    const overlaySync = eventListeners.get('recording-overlay-sync');

    overlaySync?.({
      event: 'recording-overlay-sync',
      id: 1,
      payload: { visible: true, displayMode: 'full' }
    });
    expect(result.current.status.displayMode.value).toBe('full');

    await act(async () => {
      resolveSnapshot(makeSnapshot({ overlayDisplayMode: 'minimal' }));
      await snapshotPromise;
    });

    expect(result.current.status.displayMode.value).toBe('full');
  });
});
