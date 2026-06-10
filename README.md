# ActaVoces

ActaVoces records meeting audio and writes transcripts, summaries, and job
metadata into a folder people and tools can inspect.

The app follows a few plain goals:

- Record microphone audio and available system audio from the desktop.
- Produce Markdown and JSON artifacts that work outside the app.
- Run local processing by default where practical.
- Allow remote processing through explicit provider settings.
- Keep the record folder useful for humans, scripts, and personal AI agents.

Pronunciation: `AHK-tah VOH-kays`.

The name joins two Latin roots: `acta`, meaning records or proceedings, and
`voces`, meaning voices.

## Why it exists

Many meeting-note tools make the useful record hard to reuse:

- A bot joins the call.
- Audio goes to a remote pipeline by default.
- Notes stay locked inside a product UI.
- Exports lose the raw transcript, segments, job log, or source metadata.

ActaVoces serves people who want a durable meeting archive:

- Searchable transcripts.
- Markdown summaries.
- JSON segment and speaker data.
- A folder that can be opened with normal tools.
- Files that can be handed to personal agent harnesses such as OpenClaw,
  Hermes, or similar local assistant setups.

The record folder drives the design. A user should be able to ask an assistant
about past meetings, feed transcripts into planning workflows, or process files
with their own scripts.

## What it does

ActaVoces aims to support this workflow:

1. Start capture from the app or a global hotkey.
2. Record microphone audio and available system audio.
3. Stop capture and write a durable recording folder.
4. Transcribe the audio through the configured worker path.
5. Add speaker labels when diarization has been configured.
6. Generate optional summaries through an OpenAI-compatible provider.
7. Keep every output as a file beside the recording.

Generated artifact folders use stable paths, so later settings changes do not
move older recordings.

## Current status

Current state: working foundation for contributors, with release hardening still
pending.

Implemented pieces:

- Tauri v2 desktop shell with Dashboard, Recordings, Jobs, and Settings views.
- Native recording lifecycle with persistent WAV artifacts.
- SQLite-backed settings, recordings, pipeline jobs, providers, and model
  inventory.
- Global-hotkey and recording-overlay plumbing.
- Python JSONL worker commands for health checks, model status/install,
  transcription, diarization, and summaries.
- `faster-whisper` transcription integration.
- Optional `pyannote.audio` diarization path with Hugging Face token setup.
- OpenAI-compatible summary provider settings.
- CI workflow coverage for Windows, macOS, and Linux.
- Release workflow that tags, creates a GitHub release, and uploads platform
  artifacts.

Known work before a public release:

- Real ML runs need validation with installed models, pyannote, and provider
  endpoints.
- Linux system-audio behavior needs OS-level validation and user-facing setup
  guidance.

See [docs/implementation-gaps.md](docs/implementation-gaps.md) for the detailed
implementation audit.

## Beta support

Beta releases target desktop use with this support matrix:

- macOS: supported target, tested before beta publishing.
- Windows: supported target, tested before beta publishing.
- Linux: expected target, not tested yet.

Linux builds are expected to work, but system-audio capture depends on the
available PipeWire/PulseAudio monitor or loopback devices.

## Prerequisites

For local development:

- Node.js with `pnpm`.
- Rust stable with Cargo.
- Tauri v2 system dependencies for your operating system.
- Python 3.14.
- `uv` for the Python worker.
- OS microphone and audio-capture permissions.

Optional runtime features:

- `ffmpeg` for `pyannote.audio` diarization when the bundled runtime does not
  provide it.
- A Hugging Face token with access to
  `pyannote/speaker-diarization-community-1` for speaker diarization.
- An OpenAI-compatible API key and model name for summaries.

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

## Common commands

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

- Manually dispatch the workflow from the branch to publish.
- The workflow reads `version` from the root `package.json`.
- The same version is stamped into the Tauri config before packaging.
- The workflow builds platform bundles, creates tag `v<version>`, creates the
  GitHub release, and uploads the generated artifacts.

## Architecture

```text
src/                 Preact renderer
src-tauri/           Tauri shell, Rust commands, capture, storage
worker/              Python worker for transcription and AI processing
docs/                Project notes and implementation audits
public/              Static renderer assets
```

Main stack:

- Frontend: Preact 10, TypeScript, `@preact/signals`, Tailwind CSS v4.
- Desktop backend: Tauri v2, Rust, SQLite through `rusqlite`, native capture
  through `cpal`.
- Worker: Python 3.14, `uv`, Pydantic, `faster-whisper`, optional
  `pyannote.audio`, `pydantic-ai`.

The Rust app owns windows, hotkeys, capture, filesystem paths, SQLite state, and
worker process orchestration. The Python worker owns ML-heavy processing and
communicates with the desktop app through newline-delimited JSON.

## Artifact layout

Each recording gets its own directory under the configured records folder. The
default records root follows this shape:

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

The raw transcript can appear before diarization and summary stages finish.
Later stages update their own files instead of rewriting one combined note.

## Processing model

ActaVoces stores recordings, transcripts, job logs, and metadata on the local
machine by default.

Processing stays local-first, with remote stages available through explicit
provider configuration:

- Local transcription uses the Python worker and `faster-whisper`.
- Local speaker diarization can use `pyannote.audio` after setup.
- NeMo/Whisper-style diarization is planned but not supported yet.
- Remote summaries use an OpenAI-compatible provider.
- Future remote transcription or formatting stages should follow the same
  explicit provider model.

Secrets are stored through the operating system keychain where possible.

Users remain responsible for consent and recording-law compliance in their
jurisdiction.

## Contributing

The project remains early. Small, focused changes are easiest to review.

Before opening a pull request, run the checks that match your changes:

```bash
pnpm validate
pnpm test
pnpm lint:rust
pnpm test:rust
pnpm lint:py
pnpm test:py
```

Use the existing frontend triplet pattern, keep Rust domain logic out of
`lib.rs`, and keep worker imports rooted at `app.`.

## License

ActaVoces is licensed under `AGPL-3.0-or-later`. See [LICENSE](LICENSE).

Derivative source distributions should preserve attribution to ActaVoces and a
link to the original repository in their README, NOTICE, or equivalent source
distribution notice.
