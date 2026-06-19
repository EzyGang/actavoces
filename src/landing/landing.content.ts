export const latestVersion: string = __LATEST_VERSION__;

export const repositoryUrl = 'https://github.com/EzyGang/actavoces';
export const releasesUrl = 'https://github.com/EzyGang/actavoces/releases';
export const issuesUrl = 'https://github.com/EzyGang/actavoces/issues';
export const feedbackUrl = 'https://insigh.to/b/actavoces';
export const creatorUrl = 'https://x.com/galtozzy';

export const featureCards = [
  {
    label: 'Capture',
    title: 'Desktop meeting recording',
    text: 'Record microphone and available system audio. Start capture from the app, tray, hotkey, or overlay.'
  },
  {
    label: 'Transcribe',
    title: 'Local transcription by default',
    text: 'The bundled Python worker uses faster-whisper. Choose small, medium, large-v3, or distil-large-v3.'
  },
  {
    label: 'Speakers',
    title: 'Speaker labels when useful',
    text: 'Sortformer runs locally by default. Use one-speaker mode for solo recordings or pyannote for a token-based local setup.'
  },
  {
    label: 'Files',
    title: 'Readable artifacts on disk',
    text: 'Each recording folder can include WAV audio, Markdown transcripts, JSON segments, speaker data, metadata, summaries, and logs.'
  },
  {
    label: 'Recovery',
    title: 'Pipeline state you can retry',
    text: 'SQLite tracks recordings, artifacts, settings, models, and jobs. Retry failed or setup-blocked stages from the app.'
  },
  {
    label: 'Summary',
    title: 'Optional summaries',
    text: 'Summaries stay off until configured. Use Ollama or any OpenAI-compatible provider.'
  }
];

export const workflowSteps = [
  'Start capture from the app, tray, global hotkey, or overlay.',
  'Stop capture. The processing pipeline starts automatically.',
  'Open the recording folder. Keep the Markdown, JSON, and WAV files.'
];

export const artifactItems = [
  'raw-transcript.md',
  'diarized-transcript.md',
  'meta/recording.wav',
  'meta/microphone.wav',
  'meta/raw-segments.json',
  'meta/raw-words.json',
  'meta/diarization.json',
  'summary.md',
  'meta/metadata.json',
  'meta/job-log.jsonl'
];

export const downloadRows = [
  {
    platform: 'Windows x64',
    links: [
      {
        label: 'Setup EXE',
        href: 'https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-windows-x64-setup.exe'
      },
      {
        label: 'MSI',
        href: 'https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-windows-x64.msi'
      }
    ]
  },
  {
    platform: 'macOS Apple Silicon',
    links: [
      {
        label: 'DMG',
        href: 'https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-macos-aarch64.dmg'
      }
    ]
  },
  {
    platform: 'macOS Intel',
    links: [
      {
        label: 'DMG',
        href: 'https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-macos-x64.dmg'
      }
    ]
  },
  {
    platform: 'Linux x64',
    links: [
      {
        label: 'AppImage',
        href: 'https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-linux-x64.AppImage'
      },
      {
        label: 'DEB',
        href: 'https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-linux-x64.deb'
      },
      {
        label: 'RPM',
        href: 'https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-linux-x64.rpm'
      }
    ]
  }
];

export const faqItems = [
  {
    question: 'Is ActaVoces fully local?',
    answer:
      'Recording, storage, transcription, and Sortformer speaker labels are local by default. Model downloads and optional summaries can use network. Summaries can also use local Ollama endpoints or any OpenAI-compatible API.'
  },
  {
    question: 'Where are recordings stored?',
    answer:
      'Recordings live under your configured records folder. The app writes Markdown and JSON artifacts beside the captured audio.'
  },
  {
    question: 'Does it support speaker labels?',
    answer:
      'Yes. Sortformer is the default local backend. pyannote.audio is optional and requires accepted Hugging Face model terms plus a token.'
  },
  {
    question: 'Does it summarize meetings?',
    answer:
      'Summaries run only when enabled. They use the OpenAI-compatible provider configured in settings, including local providers such as Ollama.'
  },
  {
    question: 'Is it production-ready?',
    answer:
      'It is usable and is being used. It is still open-source pre-1.0 software without comprehensive multi-platform QA. Windows has the most runtime, macOS has lighter runtime, and Linux lacks active QA.'
  },
  {
    question: 'Are builds code-signed?',
    answer:
      'Builds are unsigned today. Code signing is expensive for an independent open-source app. macOS builds may later move through App Store distribution and signing. Windows builds will likely stay unsigned for a while.'
  }
];
