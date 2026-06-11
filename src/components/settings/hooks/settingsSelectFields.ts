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
    options: [
      { value: 'medium', label: 'medium' },
      { value: 'small', label: 'small' },
      { value: 'large-v3', label: 'large-v3' },
      { value: 'distil-large-v3', label: 'distil-large-v3' }
    ],
    onChange: onChange('whisperModel')
  },
  {
    key: 'transcriptionLanguage',
    label: 'Language',
    value: draft.value.transcriptionLanguage,
    options: [
      { value: 'auto', label: 'Automatic' },
      { value: 'en', label: 'English' },
      { value: 'ru', label: 'Russian' },
      { value: 'uk', label: 'Ukrainian' },
      { value: 'es', label: 'Spanish' }
    ],
    onChange: onChange('transcriptionLanguage')
  },
  computeTypeField(draft, onChange),
  {
    key: 'diarizationBackend',
    label: 'Diarization backend (speaker labels)',
    value: draft.value.diarizationBackend,
    options: [
      { value: 'sortformer', label: 'Sortformer' },
      { value: 'pyannote', label: 'pyannote' }
    ],
    onChange: onChange('diarizationBackend'),
    hint: diarizationHint(draft.value.diarizationBackend)
  },
  {
    key: 'speakerCountMode',
    label: 'Speaker count',
    value: draft.value.speakerCountMode,
    options: [
      { value: 'automatic', label: 'Automatic' },
      { value: 'exact', label: 'Exact' },
      { value: 'range', label: 'Range' }
    ],
    onChange: onChange('speakerCountMode')
  }
];

const diarizationHint = (
  backend: AppSettingsUpdate['diarizationBackend']
): SettingsSelectField['hint'] => {
  if (backend === 'sortformer') {
    return {
      tone: 'muted',
      title: 'Local Sortformer voice attribution.',
      text: 'Adds speaker labels by detecting who spoke when. The ONNX model and ONNX Runtime download automatically on first use; no Hugging Face token or Python diarization runtime is required. Speaker count is automatic unless exact one speaker is selected.'
    };
  }

  return {
    tone: 'warning',
    title: 'Local pyannote voice attribution.',
    text: 'Adds speaker labels by detecting who spoke when. Requires pyannote.audio, accepted Hugging Face model terms, and a Hugging Face access token. pyannoteAI cloud API support is not implemented.'
  };
};

const computeTypeField = (
  draft: Signal<AppSettingsUpdate>,
  onChange: (key: keyof AppSettingsUpdate) => JSX.GenericEventHandler<HTMLSelectElement>
): SettingsSelectField => ({
  key: 'computeType',
  label: 'Compute type',
  value: draft.value.computeType,
  options: [
    { value: 'auto', label: 'Automatic' },
    { value: 'cpu', label: 'CPU' },
    { value: 'cuda', label: 'CUDA' },
    { value: 'metal', label: 'Metal' }
  ],
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
