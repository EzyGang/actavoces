import { signal } from '@preact/signals';
import type { AppSnapshot } from '../types/desktop';

const fallbackSnapshot: AppSnapshot = {
  activeRecording: null,
  recordings: [
    {
      id: 'demo-2026-06-09',
      title: 'Actavoces product pipeline',
      startedAt: '2026-06-09T15:40:00.000Z',
      endedAt: '2026-06-09T16:04:00.000Z',
      durationSeconds: 1440,
      status: 'processing',
      stages: [
        {
          id: 'recording',
          label: 'Capture',
          status: 'complete',
          progress: 100
        },
        {
          id: 'transcription',
          label: 'Raw transcript',
          status: 'complete',
          progress: 100
        },
        {
          id: 'alignment',
          label: 'Alignment',
          status: 'complete',
          progress: 100
        },
        {
          id: 'diarization',
          label: 'Diarization',
          status: 'running',
          progress: 62
        },
        {
          id: 'summary',
          label: 'Summary',
          status: 'pending',
          progress: 0
        }
      ],
      artifacts: [
        {
          kind: 'audio',
          label: 'Mixed WAV',
          path: 'Actavoces/2026/06/09/1540/recording.wav',
          ready: true
        },
        {
          kind: 'rawTranscript',
          label: 'Raw transcript',
          path: 'Actavoces/2026/06/09/1540/raw-transcript.md',
          ready: true
        },
        {
          kind: 'diarizedTranscript',
          label: 'Diarized transcript',
          path: 'Actavoces/2026/06/09/1540/diarized-transcript.md',
          ready: false
        }
      ]
    }
  ],
  settings: {
    outputDirectory: 'Actavoces',
    hotkey: 'CommandOrControl+Shift+Space',
    whisperModel: 'medium.en',
    diarizationBackend: 'nemoWhisper',
    summaryProviderConfigured: false
  }
};

export const appSnapshotSignal = signal<AppSnapshot>(fallbackSnapshot);
export const appErrorSignal = signal<string | null>(null);
