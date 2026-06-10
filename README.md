# ActaVoces

**ActaVoces records meetings into a folder you can inspect with normal tools.**

It captures desktop meeting audio, runs transcription and optional speaker
diarization, then writes Markdown and JSON artifacts beside the recording. The
result works in the app, in a text editor, in scripts, and in personal agent
harnesses such as OpenClaw, Hermes, or similar assistant setups.

Pronunciation: `AHK-tah VOH-kays`.

The name joins two Latin roots:

- `acta`: records or proceedings
- `voces`: voices

## Features

- **Local-first recording archive**
  Stores recordings and generated artifacts under your configured records
  folder. Existing recording folders keep stable paths after settings changes.

- **Inspectable file output**
  Produces `recording.wav`, Markdown transcripts, JSON segments, speaker turns,
  metadata, and a JSONL job log. You can search, diff, back up, sync, or process
  these files outside the app.

- **Desktop capture**
  Records microphone audio and available system audio through the desktop app.
  Capture can be controlled from the app, a global hotkey, and a small recording
  overlay.

- **Local transcription**
  Uses a Python worker with `faster-whisper`. Model status and install controls
  live in Settings.

- **Speaker diarization**
  Supports local `pyannote.audio` diarization after Hugging Face setup. Exact
  one-speaker mode works without a diarization model.

- **Speaker label editing**
  Completed recordings expose speaker labels in the Recordings view. Renaming a
  speaker rewrites `diarization.json` and regenerates `diarized-transcript.md`.

- **Optional summaries**
  Generates `summary.md` through an OpenAI-compatible provider when configured.
  Summary generation can be disabled without blocking transcripts.

- **Persistent job state**
  Uses SQLite for settings, recordings, pipeline jobs, providers, model
  inventory, and worker status. Failed downstream jobs can be retried from the
  app.

- **Self-updates**
  Tauri updater support is wired for signed public GitHub Release artifacts and
  a GitHub-hosted `latest.json`.

- **Agent-friendly records**
  The record folder is meant for humans and software. You can point an assistant
  at past meeting folders, ask questions over transcripts, extract action items,
  or feed records into planning workflows.

## Why ActaVoces Exists

Many meeting-note tools hide the useful record behind a product surface:

- A bot joins the call.
- Audio leaves the machine by default.
- Notes stay trapped in a web app.
- Exports omit raw segments, speaker turns, job logs, or source metadata.

ActaVoces optimizes for a durable local archive:

- **Own the files.**
- **Keep the raw transcript.**
- **Keep the machine-readable data.**
- **Use local processing where practical.**
- **Allow remote processing only through explicit provider settings.**

## Artifact Layout

Each recording gets its own directory under the configured records folder:

```text
~/actavoces/records/YYYY/MM/YYYY-MM-DD-HHMMSS-title/
```

Expected artifacts:

```text
recording.wav
raw-transcript.md
raw-segments.json
diarization.json
diarized-transcript.md
summary.md
metadata.json
job-log.jsonl
```

The raw transcript can appear before diarization and summary finish. Later
pipeline stages update their own files instead of rewriting one combined note.

## Processing Model

ActaVoces keeps storage local by default:

- **Transcription:** local Python worker with `faster-whisper`
- **Diarization:** local `pyannote.audio` after setup
- **Summaries:** optional OpenAI-compatible remote provider
- **Secrets:** OS keychain where possible

Remote processing for more stages can fit the same explicit provider model.
NeMo/Whisper-style diarization is planned, but not supported yet.

Users remain responsible for consent and recording-law compliance in their
jurisdiction.

## Platform Support

Beta releases target desktop use:

- **macOS:** supported beta target, tested before beta publishing
- **Windows:** supported beta target, tested before beta publishing
- **Linux:** expected target, not tested yet

Linux system-audio capture depends on available PipeWire/PulseAudio monitor or
loopback devices.

## Prerequisites

For local development:

- **Node.js** with `pnpm`
- **Rust stable** with Cargo
- **Tauri v2 system dependencies** for your operating system
- **Python 3.14**
- **uv** for the Python worker
- **OS microphone and audio-capture permissions**

Optional runtime setup:

- **ffmpeg** for `pyannote.audio` when the bundled runtime does not provide it
- **Hugging Face token** with access to
  `pyannote/speaker-diarization-community-1`
- **OpenAI-compatible API key and model** for summaries

## Setup

Install frontend and desktop dependencies:

```bash
pnpm install
```

Install the Python worker environment:

```bash
pnpm sync:py
```

Run the desktop app in development:

```bash
pnpm tauri dev
```

Run only the Vite renderer:

```bash
pnpm dev
```

The renderer dev server uses port `1420`.

## Common Commands

```bash
pnpm validate     # Biome + TypeScript
pnpm lint:all     # TypeScript, Rust, and Python checks
pnpm test:all     # Frontend, Rust, and Python tests
pnpm build:web    # Vite production build
pnpm build        # Tauri production build
```

Worker-only commands run from `worker/`:

```bash
uv run task ruff-lint
uv run task pyright-lint
uv run task tests
```

Rust-only checks run from the repository root through `pnpm`:

```bash
pnpm lint:rust
pnpm test:rust
```

## Releases

Releases are created through the GitHub Actions release workflow:

- **Manual dispatch:** run the workflow from the branch to publish
- **Version source:** root `package.json`
- **Full release tag:** `v<version>`
- **Alpha release tag:** `v<version>-alpha.N`
- **Artifacts:** platform bundles uploaded to the GitHub Release
- **Updater metadata:** signed updater artifacts plus `latest.json`

Updater releases require:

- `TAURI_SIGNING_PRIVATE_KEY` as a GitHub secret
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` as a GitHub secret when the key has a
  password
- `TAURI_UPDATER_PUBKEY` as a GitHub repository variable or secret

## Architecture

```text
src/                 Preact renderer
src-tauri/           Tauri shell, Rust commands, capture, storage
worker/              Python worker for transcription and AI processing
public/              Static renderer assets
```

Main stack:

- **Frontend:** Preact 10, TypeScript, `@preact/signals`, Tailwind CSS v4
- **Desktop backend:** Tauri v2, Rust, SQLite through `rusqlite`, native capture
  through `cpal`
- **Worker:** Python 3.14, `uv`, Pydantic, `faster-whisper`, optional
  `pyannote.audio`, `pydantic-ai`

The Rust app owns windows, hotkeys, capture, filesystem paths, SQLite state, and
worker process orchestration. The Python worker owns ML-heavy processing and
communicates with the desktop app through newline-delimited JSON.

## Project Status

ActaVoces has the main app path implemented:

- Capture lifecycle and durable WAV output
- SQLite-backed settings and recordings
- Route-backed Dashboard, Recordings, Jobs, and Settings views
- Local worker bootstrap and JSONL command protocol
- Transcription, diarization, summaries, job retry, artifact opening
- CI across TypeScript, Rust, and Python
- Release workflow with signed updater metadata

Before beta publishing, the remaining work is release hardening:

- Validate packaged macOS and Windows builds
- Validate real `faster-whisper`, pyannote, and provider runs
- Validate Linux behavior and document Linux audio setup if needed

## Contributing

Contributions are welcome. Keep changes focused, documented by tests where
practical, and aligned with the local-first file archive model.

Good first contributions include:

- Documentation fixes
- Platform setup notes
- Focused UI polish
- Worker error-message improvements
- Tests for existing behavior

Before larger changes, open an issue or draft pull request that explains:

- **Problem:** what user pain or project risk the change addresses
- **Approach:** how the change fits the existing architecture
- **Tradeoffs:** any added dependency, runtime cost, or behavior change
- **Validation:** how the change was tested

Before opening a pull request, run the checks that match your changes:

```bash
pnpm validate
pnpm test
pnpm lint:rust
pnpm test:rust
pnpm lint:py
pnpm test:py
```

Contribution guidelines:

- **Keep pull requests small.** One bug fix, feature, or cleanup per PR is
  easiest to review.
- **Preserve artifact compatibility.** Do not rename, move, or change generated
  files without a migration path and README update.
- **Follow existing architecture.** Frontend features use the hook/view/container
  triplet pattern. Rust domain logic stays out of `lib.rs`. Python worker
  imports stay rooted at `app.`.
- **Avoid hidden network behavior.** Remote processing must be explicit in
  Settings and documented in user-facing text.
- **Protect user data.** Do not log secrets, transcript content, or provider API
  keys. Store secrets through the OS keychain where possible.
- **Add tests for behavior changes.** Use Vitest for frontend behavior, Rust
  unit tests for desktop/storage logic, and pytest for worker contracts.
- **Document platform assumptions.** Capture, hotkeys, overlays, and updater
  behavior can vary by OS. Include the tested OS and version when reporting
  platform work.

## License

ActaVoces is licensed under `AGPL-3.0-or-later`. See [LICENSE](LICENSE).

Derivative source distributions should preserve attribution to ActaVoces and a
link to the original repository in their README, NOTICE, or equivalent source
distribution notice.
