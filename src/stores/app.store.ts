import { signal } from '@preact/signals';
import type { AppSnapshot } from '../types/desktop';

const initialSnapshot: AppSnapshot = {
  activeRecording: null,
  recordings: [],
  settings: {
    outputDirectory: 'Actavoces',
    hotkey: 'CommandOrControl+Shift+Space',
    whisperModel: 'medium.en',
    diarizationBackend: 'nemoWhisper',
    summaryProviderConfigured: false
  }
};

export const appSnapshotSignal = signal<AppSnapshot>(initialSnapshot);
export const appErrorSignal = signal<string | null>(null);
