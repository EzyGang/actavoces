import type { Signal } from '@preact/signals';
import type { JSX } from 'preact';
import { appSnapshotSignal } from '../../../stores/app.store';
import type { AppSettingsUpdate } from '../../../types/desktop';
import type { SettingsSelectField } from './settings.helpers';

export const selectFields = (
  draft: Signal<AppSettingsUpdate>,
  onChange: (key: keyof AppSettingsUpdate) => JSX.GenericEventHandler<HTMLSelectElement>
): SettingsSelectField[] => [
  {
    key: 'whisperModel',
    label: 'Whisper model',
    value: draft.value.whisperModel,
    options: ['small.en', 'medium.en', 'large-v3', 'distil-large-v3'],
    onChange: onChange('whisperModel')
  },
  {
    key: 'transcriptionLanguage',
    label: 'Language',
    value: draft.value.transcriptionLanguage,
    options: ['auto', 'en', 'ru', 'uk', 'es'],
    onChange: onChange('transcriptionLanguage')
  },
  computeTypeField(draft, onChange),
  {
    key: 'diarizationBackend',
    label: 'Diarization backend',
    value: draft.value.diarizationBackend,
    options: ['pyannote'],
    onChange: onChange('diarizationBackend'),
    hint: {
      tone: 'muted',
      title: 'Local pyannote speaker diarization.',
      text: 'Requires pyannote.audio, bundled or system ffmpeg, accepted Hugging Face model terms, and a Hugging Face access token. pyannoteAI cloud API support is not implemented.'
    }
  },
  {
    key: 'speakerCountMode',
    label: 'Speaker count',
    value: draft.value.speakerCountMode,
    options: ['automatic', 'exact', 'range'],
    onChange: onChange('speakerCountMode')
  }
];

const computeTypeField = (
  draft: Signal<AppSettingsUpdate>,
  onChange: (key: keyof AppSettingsUpdate) => JSX.GenericEventHandler<HTMLSelectElement>
): SettingsSelectField => ({
  key: 'computeType',
  label: 'Compute type',
  value: draft.value.computeType,
  options: ['auto', 'cpu', 'cuda', 'metal'],
  onChange: onChange('computeType'),
  hint: {
    tone: 'warning',
    title: 'CUDA processing requires NVIDIA libraries.',
    text: `Install cuBLAS for CUDA 12 and cuDNN 9 for CUDA 12. Runtime check: ${
      appSnapshotSignal.value.desktop.cudaAvailable
        ? 'CUDA device and required libraries detected'
        : (appSnapshotSignal.value.desktop.cudaError ?? 'not ready')
    }`,
    links: [
      {
        href: 'https://developer.nvidia.com/cublas',
        label: 'cuBLAS'
      },
      {
        href: 'https://developer.nvidia.com/cudnn',
        label: 'cuDNN'
      }
    ]
  }
});
