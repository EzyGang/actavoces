import { signal } from '@preact/signals';
import type { AppSnapshot } from '../types/desktop';

const initialSnapshot: AppSnapshot = {
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
      }
    ],
    systemSources: [
      {
        name: 'Default system output',
        label: 'Default system output',
        default: true
      }
    ]
  },
  desktop: {
    overlayVisible: false,
    hotkeyRegistered: false,
    hotkeyError: null,
    workerRunning: false,
    workerHealthOk: false,
    workerError: null,
    workerSetupStatus: 'missing',
    workerSetupStep: '',
    workerSetupError: null,
    cudaAvailable: false,
    cudaError: null
  },
  settings: {
    outputDirectory: '',
    databasePath: '',
    hotkey: 'CommandOrControl+Shift+Space',
    overlayPosition: 'topLeft',
    overlayDisplayMode: 'full',
    closeToTray: true,
    launchAtLogin: false,
    microphoneDevice: 'Default microphone',
    systemAudioSource: 'Default system output',
    sampleRate: 48000,
    whisperModel: 'small',
    modelRecommendation: {
      recommendedModel: 'small',
      reason: 'Small is the safest default until hardware capabilities are checked',
      userOverridden: false
    },
    transcriptionLanguage: 'auto',
    computeType: 'auto',
    modelStorageDirectory: '',
    diarizationBackend: 'sortformer',
    speakerCountMode: 'automatic',
    exactSpeakers: null,
    minSpeakers: null,
    maxSpeakers: null,
    huggingFaceTokenConfigured: false,
    diarizationSetupSkipped: false,
    diarizationRuntimeReady: false,
    summaryProviderConfigured: false,
    providerApiKeyConfigured: false,
    summaryEnabled: false,
    providerBaseUrl: 'https://api.openai.com/v1',
    providerModel: '',
    summaryPrompt: 'Summarize decisions, action items, risks, and unanswered questions.'
  }
};

export const appSnapshotSignal = signal<AppSnapshot>(initialSnapshot);
export const appErrorSignal = signal<string | null>(null);
