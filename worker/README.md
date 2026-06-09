# Actavoces Worker

The worker is the planned Python runtime for local transcription, diarization,
and summary generation. Tauri owns capture, hotkeys, windows, state, and file
paths; the worker owns ML-heavy processing.

## Protocol

Tauri will communicate with the worker through newline-delimited JSON:

- `health.check`
- `models.status`
- `models.install`
- `transcribe.run`
- `diarize.run`
- `summarize.run`

Every long-running command must emit progress events so recordings can expose
raw transcripts first and then add diarized and summary artifacts later.

## Diarization Backends

The default free backend should be based on
`MahmoudAshraf97/whisper-diarization`: faster-whisper, CTC forced alignment,
NeMo diarization, optional Demucs source separation, and punctuation
restoration.

Pyannote remains an optional backend because its local community model requires
external model access and token setup.
