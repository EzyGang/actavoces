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

The currently supported local backend is `pyannote.audio` with
`pyannote/speaker-diarization-community-1`.

It requires accepted Hugging Face model terms, a Hugging Face access token, and
FFmpeg on PATH or bundled with the desktop app.

NeMo/Whisper-style diarization is planned but not supported yet.
