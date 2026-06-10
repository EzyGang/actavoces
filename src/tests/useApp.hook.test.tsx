import { open } from '@tauri-apps/plugin-dialog';
import { act, renderHook, waitFor } from '@testing-library/preact';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useApp } from '../components/app/hooks/useApp.hook';
import {
  getAppSnapshot,
  openLocalPath,
  resumePendingJobs,
  retryRecordingJobs,
  startRecording,
  stopRecording
} from '../services/desktop/app.service';
import { appErrorSignal, appSnapshotSignal } from '../stores/app.store';
import { setActiveRoute } from '../stores/route.store';
import type { AppSettings, AppSnapshot, Recording } from '../types/desktop';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => undefined))
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn()
}));

vi.mock('../services/desktop/app.service', () => ({
  bootstrapWorkerRuntime: vi.fn(),
  checkWorkerHealth: vi.fn(),
  clearSummaryProviderApiKey: vi.fn(),
  deleteRecording: vi.fn(),
  getAppSnapshot: vi.fn(),
  installTranscriptionModel: vi.fn(),
  openLocalPath: vi.fn(),
  refreshModelInventory: vi.fn(),
  resumePendingJobs: vi.fn(),
  retryRecordingJobs: vi.fn(),
  startRecording: vi.fn(),
  stopRecording: vi.fn(),
  toggleRecordingFromShortcut: vi.fn(),
  updateAppSettings: vi.fn()
}));

const baseSettings: AppSettings = {
  outputDirectory: '/tmp/actavoces/records',
  databasePath: '/tmp/actavoces/actavoces.sqlite',
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
  summaryProviderConfigured: false,
  providerApiKeyConfigured: false,
  summaryEnabled: false,
  providerBaseUrl: 'https://api.openai.com/v1',
  providerModel: '',
  titlePrompt: 'Title',
  summaryPrompt: 'Summary'
};

const makeRecording = (id: string): Recording => ({
  id,
  title: `Recording ${id}`,
  startedAt: '1717938012',
  endedAt: '1717938072',
  durationSeconds: 60,
  status: 'processing',
  artifactDirectory: `/tmp/actavoces/records/${id}`,
  captureErrors: [],
  stages: [
    {
      id: 'recording',
      label: 'Capture',
      status: 'complete',
      progress: 100,
      message: 'Audio capture complete'
    }
  ],
  artifacts: []
});

const makeSnapshot = (overrides: Partial<AppSnapshot> = {}): AppSnapshot => ({
  activeRecording: null,
  recordings: [],
  jobs: [],
  models: [],
  captureDevices: {
    microphones: [
      {
        name: 'Default microphone',
        label: 'Default microphone',
        default: true
      },
      {
        name: 'Studio mic',
        label: 'Studio mic',
        default: false
      }
    ],
    systemSources: [
      {
        name: 'Default system output',
        label: 'Default system output',
        default: true
      },
      {
        name: 'MacBook Speakers',
        label: 'MacBook Speakers',
        default: false
      }
    ]
  },
  desktop: {
    overlayVisible: false,
    hotkeyRegistered: true,
    hotkeyError: null,
    workerRunning: false,
    workerHealthOk: false,
    workerError: null,
    workerSetupStatus: 'ready',
    workerSetupStep: 'Worker runtime ready',
    workerSetupError: null
  },
  settings: baseSettings,
  ...overrides
});

const resetSignals = () => {
  appSnapshotSignal.value = makeSnapshot();
  appErrorSignal.value = null;
  setActiveRoute('dashboard');
};

describe('useApp hook', () => {
  beforeEach(() => {
    vi.mocked(getAppSnapshot).mockReset();
    vi.mocked(openLocalPath).mockReset();
    vi.mocked(startRecording).mockReset();
    vi.mocked(stopRecording).mockReset();
    vi.mocked(resumePendingJobs).mockReset();
    vi.mocked(retryRecordingJobs).mockReset();
    vi.mocked(open).mockReset();
    resetSignals();
  });

  it('loads the desktop snapshot and derives the latest recording', async () => {
    const loadedSnapshot = makeSnapshot({
      recordings: [makeRecording('recording-1')]
    });

    vi.mocked(getAppSnapshot).mockResolvedValue(loadedSnapshot);

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.data.snapshot.value.recordings).toHaveLength(1);
    });

    expect(result.current.data.latestRecording.value?.id).toBe('recording-1');
    expect(result.current.status.error.value).toBeNull();
  });

  it('surfaces load failures', async () => {
    vi.mocked(getAppSnapshot).mockRejectedValue(new Error('backend offline'));

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.status.error.value).toBe('backend offline');
    });
  });

  it('starts recording and applies the returned snapshot', async () => {
    const activeRecording = { ...makeRecording('recording-active'), status: 'recording' as const };

    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot());
    vi.mocked(startRecording).mockResolvedValue(
      makeSnapshot({
        activeRecording
      })
    );

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(vi.mocked(getAppSnapshot)).toHaveBeenCalled();
    });
    await act(async () => {
      await result.current.actions.startRecording();
    });

    expect(result.current.data.snapshot.value.activeRecording?.id).toBe('recording-active');
    expect(result.current.status.isRecording.value).toBe(true);
  });

  it('surfaces start failures', async () => {
    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot());
    vi.mocked(startRecording).mockRejectedValue(new Error('mic denied'));

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(vi.mocked(getAppSnapshot)).toHaveBeenCalled();
    });
    await act(async () => {
      await result.current.actions.startRecording();
    });

    expect(result.current.status.error.value).toBe('mic denied');
  });

  it('stops recording and clears the active recording', async () => {
    const activeRecording = { ...makeRecording('recording-active'), status: 'recording' as const };

    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot({ activeRecording }));
    vi.mocked(stopRecording).mockResolvedValue(
      makeSnapshot({
        recordings: [makeRecording('recording-active')]
      })
    );

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.status.isRecording.value).toBe(true);
    });
    await act(async () => {
      await result.current.actions.stopRecording();
    });

    expect(result.current.data.snapshot.value.activeRecording).toBeNull();
    expect(result.current.data.snapshot.value.recordings).toHaveLength(1);
  });

  it('surfaces stop failures', async () => {
    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot());
    vi.mocked(stopRecording).mockRejectedValue(new Error('stop failed'));

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(vi.mocked(getAppSnapshot)).toHaveBeenCalled();
    });
    await act(async () => {
      await result.current.actions.stopRecording();
    });

    expect(result.current.status.error.value).toBe('stop failed');
  });

  it('resumes jobs and applies job progress', async () => {
    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot());
    vi.mocked(resumePendingJobs).mockResolvedValue(
      makeSnapshot({
        jobs: [
          {
            id: 'job-1',
            recordingId: 'recording-1',
            stage: 'transcription',
            status: 'running',
            progress: 25,
            message: 'Worker stage running'
          }
        ]
      })
    );

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(vi.mocked(getAppSnapshot)).toHaveBeenCalled();
    });
    await act(async () => {
      await result.current.actions.resumeJobs();
    });

    expect(result.current.data.activeJobs.value).toHaveLength(1);
    expect(result.current.data.activeJobs.value[0].progress).toBe(25);
  });

  it('opens a recording folder from the row action', async () => {
    const recording = makeRecording('recording-1');

    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot({ recordings: [recording] }));
    vi.mocked(openLocalPath).mockResolvedValue(undefined);

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.data.recordingRows.value).toHaveLength(1);
    });
    await act(async () => {
      result.current.data.recordingRows.value[0].onOpenFolder();
    });

    expect(openLocalPath).toHaveBeenCalledWith(recording.artifactDirectory);
  });

  it('exposes capture device selectors from the snapshot', async () => {
    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot());

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.settings.captureSelectFields[0].options).toContain('Studio mic');
    });

    expect(result.current.settings.captureSelectFields[1].options).toContain('MacBook Speakers');
  });

  it('preserves configured capture devices missing from current enumeration', async () => {
    vi.mocked(getAppSnapshot).mockResolvedValue(
      makeSnapshot({
        settings: {
          ...baseSettings,
          microphoneDevice: 'Disconnected microphone',
          systemAudioSource: 'Missing loopback'
        }
      })
    );

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.settings.captureSelectFields[0].options[0]).toBe(
        'Disconnected microphone'
      );
    });

    expect(result.current.settings.captureSelectFields[1].options[0]).toBe('Missing loopback');
  });

  it('selects folder settings through the native dialog', async () => {
    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot());
    vi.mocked(open).mockResolvedValue('/tmp/actavoces/selected');

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.settings.folderFields[0].value).toBe('/tmp/actavoces/records');
    });
    await act(async () => {
      result.current.settings.folderFields[0].onSelect();
    });

    expect(open).toHaveBeenCalledWith({
      defaultPath: '/tmp/actavoces/records',
      directory: true,
      multiple: false
    });
    expect(result.current.settings.draft.value.outputDirectory).toBe('/tmp/actavoces/selected');
  });

  it('captures the next keypress as a global hotkey', async () => {
    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot());

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.settings.hotkeyField.value).toBe('CommandOrControl+Shift+Space');
    });
    act(() => {
      result.current.settings.hotkeyField.onCapture();
    });
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'k',
          ctrlKey: true,
          shiftKey: true
        })
      );
    });

    expect(result.current.settings.draft.value.hotkey).toBe('CommandOrControl+Shift+K');
    expect(result.current.settings.hotkeyField.recording).toBe(false);
  });

  it('keeps the user on settings when unsaved changes are not discarded', async () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);

    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot());
    setActiveRoute('settings');

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.status.activeRoute.value).toBe('settings');
    });
    act(() => {
      result.current.settings.hotkeyField.onCapture();
    });
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'k',
          ctrlKey: true,
          shiftKey: true
        })
      );
    });
    act(() => {
      result.current.navigation[0].onSelect();
    });

    expect(confirm).toHaveBeenCalledWith('Discard unsaved settings changes?');
    expect(result.current.status.activeRoute.value).toBe('settings');

    confirm.mockRestore();
  });

  it('discards unsaved settings when navigation away is confirmed', async () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);

    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot());
    setActiveRoute('settings');

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.status.activeRoute.value).toBe('settings');
    });
    act(() => {
      result.current.settings.hotkeyField.onCapture();
    });
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'k',
          ctrlKey: true,
          shiftKey: true
        })
      );
    });
    act(() => {
      result.current.navigation[0].onSelect();
    });

    expect(result.current.status.activeRoute.value).toBe('dashboard');
    expect(result.current.settings.draft.value.hotkey).toBe('CommandOrControl+Shift+Space');

    confirm.mockRestore();
  });

  it('does not retry failed capture stages', async () => {
    const recording = {
      ...makeRecording('recording-1'),
      stages: [
        {
          id: 'recording' as const,
          label: 'Capture',
          status: 'failed' as const,
          progress: 0,
          message: 'Capture failed'
        }
      ]
    };

    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot({ recordings: [recording] }));

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.data.recordingRows.value).toHaveLength(1);
    });

    expect(result.current.data.recordingRows.value[0].canRetry).toBe(false);
  });

  it('retries failed recording jobs', async () => {
    const recording = {
      ...makeRecording('recording-1'),
      stages: [
        {
          id: 'transcription' as const,
          label: 'Raw transcript',
          status: 'failed' as const,
          progress: 0,
          message: 'Worker missing'
        }
      ]
    };

    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot({ recordings: [recording] }));
    vi.mocked(retryRecordingJobs).mockResolvedValue(
      makeSnapshot({
        recordings: [
          {
            ...recording,
            stages: [
              {
                id: 'transcription',
                label: 'Raw transcript',
                status: 'running',
                progress: 25,
                message: 'Worker stage running'
              }
            ]
          }
        ]
      })
    );

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(result.current.data.recordingRows.value[0].canRetry).toBe(true);
    });
    await act(async () => {
      result.current.data.recordingRows.value[0].onRetry();
    });

    expect(retryRecordingJobs).toHaveBeenCalledWith('recording-1');
    expect(result.current.data.snapshot.value.recordings[0].stages[0].status).toBe('running');
  });

  it('surfaces resume failures', async () => {
    vi.mocked(getAppSnapshot).mockResolvedValue(makeSnapshot());
    vi.mocked(resumePendingJobs).mockRejectedValue(new Error('worker missing'));

    const { result } = renderHook(() => useApp());

    await waitFor(() => {
      expect(vi.mocked(getAppSnapshot)).toHaveBeenCalled();
    });
    await act(async () => {
      await result.current.actions.resumeJobs();
    });

    expect(result.current.status.error.value).toBe('worker missing');
  });
});
