# ActaVoces

[![CI](https://github.com/EzyGang/actavoces/actions/workflows/ci.yml/badge.svg)](https://github.com/EzyGang/actavoces/actions/workflows/ci.yml)
![AGPL-3.0-or-later](https://img.shields.io/badge/license-AGPL--3.0--or--later-green)
![GitHub release](https://img.shields.io/github/v/release/EzyGang/actavoces)

**ActaVoces is a local-first desktop meeting recorder that produces files you can inspect, search, back up, and reuse outside the app.**

It captures microphone and system audio, transcribes recordings locally, adds optional speaker labels, and writes Markdown and JSON artifacts beside each recording. The app is built for people who want a durable meeting archive instead of notes locked inside a hosted product.

Pronunciation: `AHK-tah VOH-kays`.

The name joins two Latin roots:

- `acta`: records or proceedings
- `voces`: voices

## Navigation

- [Features](#features)
- [How It Works](#how-it-works)
- [Available Backends](#available-backends)
- [Artifact Layout](#artifact-layout)
- [Platform Support](#platform-support)
- [Installation](#installation)
- [Development](#development)
- [Architecture](#architecture)
- [Releases](#releases)
- [Project Status](#project-status)
- [Contributing](#contributing)
- [License](#license)

## Features

- **Local recording archive**
  Recordings are stored under your configured records folder. Existing recording folders keep stable paths when settings change.

- **Inspectable artifacts**
  ActaVoces writes Markdown transcripts, WAV audio, JSON segment data, diarization turns, metadata, and a JSONL job log.

- **Desktop capture**
  Capture microphone audio and available system audio from the Tauri desktop app. Recording can be controlled from the app, tray, global hotkey, or floating overlay.

- **Local transcription**
  Transcription runs through the bundled Python worker with `faster-whisper`. The app tracks model status and can install supported models from Settings.

- **Speaker labels**
  Speaker attribution supports the default local Sortformer backend and an optional `pyannote.audio` backend. A one-speaker mode works without downloading a diarization model.

- **Speaker label editing**
  Rename speakers after processing. ActaVoces rewrites `meta/diarization.json` and regenerates `diarized-transcript.md`.

- **Optional summaries**
  Summary generation is disabled by default and can be enabled with an OpenAI-compatible provider URL, model, API key, and prompt.

- **Pipeline recovery**
  SQLite tracks recordings, settings, artifacts, models, and pipeline jobs. Failed or setup-blocked stages can be retried from the app.

- **Self-updates**
  Release builds produce Tauri updater artifacts and `latest.json` for GitHub Releases.

## How It Works

ActaVoces keeps the source recording and primary processing local by default:

| Stage | Implementation | Notes |
| --- | --- | --- |
| Capture | Rust + `cpal` | Microphone plus system source when the OS exposes one. |
| Transcription | Python worker + `faster-whisper` | Supported models: `small`, `medium`, `large-v3`, `distil-large-v3`. |
| Diarization | Rust Sortformer/ONNX or Python `pyannote.audio` | Sortformer is the default; pyannote requires setup and a Hugging Face token. |
| Summary | OpenAI-compatible provider | Optional remote call; disabled by default. |
| Storage | SQLite + files on disk | Records, jobs, settings, and artifact readiness are tracked locally. |

## Available Backends

### Transcription

ActaVoces uses `faster-whisper` through the Python worker. The app exposes model inventory and installation controls for:

- `small`
- `medium`
- `large-v3`
- `distil-large-v3`

Compute mode can be set to automatic, CPU, CUDA, or Metal. CUDA mode requires the NVIDIA runtime libraries shown in Settings.

### Diarization

ActaVoces currently supports two speaker-label backends:

- **Sortformer**: the default local backend. It uses a bundled Rust path with ONNX Runtime and downloads the Sortformer ONNX model on first use. It does not require a Hugging Face token.
- **pyannote**: optional Python-worker backend using `pyannote.audio` and `pyannote/speaker-diarization-community-1`. It requires accepted Hugging Face model terms, a Hugging Face token, and FFmpeg.

pyannoteAI cloud API support is not implemented.

### Summaries

Summaries use an OpenAI-compatible provider through `pydantic-ai`. You can point the provider settings at OpenAI or another compatible endpoint by configuring:

- Base URL
- Model
- API key
- Summary prompt

Summary generation can stay disabled without blocking recording, transcription, or speaker labels.

## Artifact Layout

Each recording gets a folder under the configured records directory:

```text
~/actavoces/records/YYYY-MM-DD-HHMM-title/
```

Human-readable artifacts are kept at the recording folder root:

```text
raw-transcript.md
diarized-transcript.md
```

Machine-readable artifacts and audio files live under `meta/`:

```text
meta/
  recording.wav
  microphone.wav
  raw-segments.json
  diarization.json
  summary.md
  metadata.json
  job-log.jsonl
```

The raw transcript can appear before diarization and summary finish. Later stages update their own files instead of rewriting one combined note.

## Platform Support

ActaVoces is a desktop app built with Tauri v2.

| Platform | Status | Notes |
| --- | --- | --- |
| Windows | Build target | CI and release workflow build Windows x64. |
| macOS | Build target | CI plus Apple Silicon and Intel release builds. |
| Linux | Build target | CI and release workflow build Linux x64. System audio requires a PipeWire/PulseAudio monitor or loopback input device. |

Users are responsible for consent and recording-law compliance in their jurisdiction.

## Installation

Published builds are available from the [GitHub Releases page](https://github.com/EzyGang/actavoces/releases). Latest direct downloads:

| Platform | Downloads |
| --- | --- |
| Windows x64 | [Setup EXE](https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-windows-x64-setup.exe) · [MSI](https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-windows-x64.msi) |
| macOS Apple Silicon | [DMG](https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-macos-aarch64.dmg) |
| macOS Intel | [DMG](https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-macos-x64.dmg) |
| Linux x64 | [AppImage](https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-linux-x64.AppImage) · [DEB](https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-linux-x64.deb) · [RPM](https://github.com/EzyGang/actavoces/releases/latest/download/ActaVoces-linux-x64.rpm) |

If no build is available for your platform yet, use the development setup below to run from source.

On first run, ActaVoces prepares the local worker runtime and installs the default transcription model. Some model and backend setup steps require network access.

## Development

### Prerequisites

- Node.js 22 with `pnpm`
- Rust stable with Cargo
- Tauri v2 system dependencies for your OS
- Python 3.14
- `uv`
- OS microphone and audio-capture permissions

### Setup

Install dependencies:

```bash
pnpm install
pnpm sync:py
```

Run the desktop app:

```bash
pnpm tauri dev
```

Run only the Vite renderer:

```bash
pnpm dev
```

The renderer dev server uses port `1420`.

### Common Commands

```bash
pnpm dev          # Vite renderer dev server
pnpm tauri dev    # Tauri desktop app
pnpm validate     # Biome + TypeScript checks
pnpm lint:ts      # Biome + TypeScript
pnpm lint:rust    # cargo fmt, clippy, and cargo check
pnpm lint:py      # Ruff and basedpyright
pnpm lint:all     # TypeScript, Rust, and Python lint/type checks
pnpm test         # Vitest
pnpm test:rust    # Rust tests
pnpm test:py      # Python worker tests
pnpm test:all     # Frontend, Rust, and Python tests
pnpm build:web    # Vite production build
pnpm build        # Tauri production build
```

Worker commands run from `worker/`:

```bash
uv run task ruff-lint
uv run task pyright-lint
uv run task tests
```

Formatting commands:

```bash
pnpm format       # Format TypeScript
pnpm format:rust  # Format Rust and apply safe clippy fixes
pnpm format:py    # Format Python worker
pnpm format:all   # Format all code
```

## Architecture

```text
src/                 Preact renderer
src-tauri/           Tauri shell, Rust commands, capture, storage, updater
worker/              Python worker for transcription, pyannote, and summaries
public/              Static renderer assets
```

Main stack:

- **Frontend:** Preact 10, TypeScript, `@preact/signals`, Tailwind CSS v4, Base UI
- **Desktop backend:** Tauri v2, Rust, SQLite through `rusqlite`, native capture through `cpal`
- **Diarization:** Rust Sortformer/ONNX path plus optional Python `pyannote.audio`
- **Worker:** Python 3.14, `uv`, Pydantic, `faster-whisper`, `pydantic-ai`

The Rust app owns windows, tray behavior, hotkeys, capture, filesystem paths, SQLite state, updater integration, and worker orchestration. The Python worker owns ML-heavy transcription, optional pyannote diarization, and summary calls.

### Frontend Shape

The renderer is organized by feature under `src/components/`. Feature code follows a hook/view/container split:

- Hooks own state, effects, services, and callback binding.
- Views are presentational JSX and styling.
- Containers connect hook output to views.

Shared renderer state lives in signal stores under `src/stores/`. Tauri calls are wrapped in service modules under `src/services/`.

### Desktop Shape

Rust command handlers live under `src-tauri/src/app/commands/`. Non-command logic is kept in domain folders:

- `capture/`: native audio device discovery, capture, WAV writing, and mixing
- `storage/`: SQLite repository, settings, recordings, artifacts, and jobs
- `worker/`: Python worker bootstrap and JSONL command execution
- `diarization/`: Sortformer setup and local diarization output
- `artifacts/`: recording folder paths and generated file paths

SQLite setup is additive and defensive. Generated artifact paths should be treated as part of the user-facing contract.

### Worker Shape

The worker receives newline-delimited JSON commands from the Rust app and emits JSON events. It owns:

- `faster-whisper` transcription
- model status and model installation checks
- optional `pyannote.audio` diarization
- OpenAI-compatible summary generation

Worker code lives under `worker/app/`; worker tests live under `worker/tests/`.

## Releases

Releases are created with the GitHub Actions release workflow.

- Manual dispatch publishes either a full release or alpha release.
- The root `package.json` version is the release source of truth.
- Build artifacts are uploaded to GitHub Releases.
- Tauri updater metadata is generated as part of the release.

## Project Status

ActaVoces has the main desktop workflow implemented:

- Recording lifecycle with durable WAV output
- SQLite-backed settings, recordings, artifacts, and jobs
- Dashboard, Recordings, Jobs, and Settings views
- Floating recording overlay, tray integration, global hotkey, and launch-at-login setting
- Worker bootstrap and newline-delimited JSON command protocol
- Local transcription, Sortformer diarization, optional pyannote diarization, optional summaries, retries, and artifact opening
- CI across TypeScript, Rust, and Python
- Release workflow for Windows, macOS, and Linux bundles

The project is still pre-1.0. Packaged build behavior, real-world audio devices, model setup, and platform-specific capture paths should be validated carefully before relying on it for critical meetings.

## Contributing

Contributions are welcome. Keep changes focused, tested where practical, and aligned with the local-first file archive model.

### Good First Contributions

Good places to start:

- Documentation fixes
- Platform setup notes
- Focused UI polish
- Worker error-message improvements
- Tests for existing behavior

### Before Larger Changes

Before starting a larger change, open an issue or draft pull request that explains:

- **Problem:** what user pain or project risk the change addresses
- **Approach:** how the change fits the current architecture
- **Tradeoffs:** any dependency, runtime cost, model behavior, storage change, or compatibility risk
- **Validation:** how the change will be tested

This matters most for capture behavior, artifact formats, database schema, pipeline job semantics, worker setup, release packaging, and any networked provider behavior.

### Pull Request Checklist

Before opening a pull request, run the checks that match your changes:

```bash
pnpm validate
pnpm test
pnpm lint:rust
pnpm test:rust
pnpm lint:py
pnpm test:py
```

Use narrower checks while iterating and broader checks before review:

```bash
pnpm lint:ts
pnpm test
pnpm lint:rust
pnpm test:rust
pnpm lint:py
pnpm test:py
```

### Engineering Guidelines

- Keep pull requests small.
- Preserve artifact compatibility. Do not rename, move, or change generated files without a migration path and README update.
- Keep remote processing explicit in Settings and user-facing text.
- Do not log secrets, transcript content, provider API keys, or Hugging Face tokens.
- Keep frontend work aligned with the hook/view/container pattern.
- Keep Rust command handlers thin and put domain behavior in the owning module.
- Keep Python imports rooted at `app.` and keep worker commands typed.
- Add tests for behavior changes where practical.
- Document tested operating systems for capture, hotkey, overlay, autostart, and updater changes.
- Treat SQLite migrations as additive by default. Do not drop user data without explicit approval and a migration plan.
- Keep generated artifacts useful outside the app. Markdown should be readable; JSON should stay machine-friendly.

## License

ActaVoces is licensed under `AGPL-3.0-or-later`. See [LICENSE](LICENSE).
