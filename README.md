# ActaVoces

**Pronunciation:** `AHK-tah VOH-kays`.

The name is styled as **ActaVoces**, from Latin roots: `acta` for records or
proceedings, and `voces` for voices. The intended feel is old Roman: written
records made from spoken voices.

## Purpose

ActaVoces is a small, stylish desktop app for local voice-call note taking. It
is meant to capture a user's microphone and the audio they hear from calls,
huddles, or meetings without adding a bot or assistant participant to the call.

The core workflow is:

- Press a global hotkey to start recording.
- Show a persistent recording overlay while capture is active.
- Press the hotkey again, or stop from the app, to end capture.
- Produce local transcription artifacts first.
- Continue diarization and summarization as background jobs.
- Save the results as Markdown files in a user-selected notes folder.

## Product Direction

ActaVoces is privacy-first by default. Local transcription should work without a
cloud provider. Cloud APIs are optional and should mainly be used for summaries,
titles, and richer note formatting through OpenAI-compatible providers.

The app is designed around separate artifacts instead of one mutable note:

- `recording.wav`
- `raw-transcript.md`
- `raw-segments.json`
- `diarization.json`
- `diarized-transcript.md`
- `summary.md`
- `job-log.jsonl`

This lets users work with the raw transcript immediately while slower
diarization and summary jobs continue in the background.

## Architecture

- **Desktop shell:** Tauri v2 and Rust.
- **Frontend:** Preact, TypeScript, `@preact/signals`, Tailwind CSS v4.
- **Native responsibilities:** capture orchestration, hotkeys, overlay windows,
  file paths, SQLite metadata, and worker process management.
- **Worker responsibilities:** local transcription, diarization, alignment,
  punctuation restoration, and AI summaries.
- **Worker runtime:** uv-managed Python sidecar.

The planned local transcription backend is `faster-whisper`. The planned free
diarization path is based on the NeMo/forced-alignment approach from
`MahmoudAshraf97/whisper-diarization`, with pyannote kept as an optional backend
for users who prefer it.

## Current Status

The repository currently contains the first functional foundation:

- A dark-first ActaVoces app shell.
- A staged recording pipeline UI.
- Tauri commands for starting, stopping, and resuming recording jobs.
- Separate artifact-writing stubs.
- A Python worker package scaffold with a JSONL command/event protocol.

Native system-audio capture, global hotkeys, SQLite persistence, and the real
ML worker are still to be wired in.

## Development

```bash
pnpm dev        # Vite renderer on port 1420
pnpm tauri dev  # Desktop shell
pnpm validate   # Biome + TypeScript
pnpm build:web  # Renderer production build
pnpm build      # Tauri production build
```

Rust backend checks:

```bash
cd src-tauri
cargo fmt --check
cargo check
```
