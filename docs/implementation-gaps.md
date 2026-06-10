# ActaVoces Implementation Gaps

This document records what remains between the current repository state and a
fully functional ActaVoces desktop app. It also explicitly marks the work that
has already been addressed so future passes do not repeat or overlook it.

Last implementation validation observed before this audit:

- `pnpm lint:all` passed
- `pnpm test` passed with 4 frontend test files and 18 tests
- `cargo test` passed with 17 Rust tests
- `uv run task format-and-lint` passed for the worker
- `uv run task tests` passed with 17 worker tests

## Current Reality

Status: partially implemented.

Originally, the app only had a styled shell, in-memory recording state, empty
audio files, placeholder transcript artifacts, and a worker with only
`health.check`. That is no longer the current state.

Addressed:

- Tauri commands now include snapshot, settings, recording lifecycle, recording
  delete, recording job retry, local path opening, worker health/start/stop,
  pending job resume, model inventory refresh, model install, hotkey toggle,
  and summary API key clearing.
- Recording and settings state moved out of in-memory `Mutex<AppSnapshot>` and
  into SQLite.
- Recording stop writes durable WAV artifacts instead of empty stubs.
- Worker JSONL commands now include health, model status/install,
  transcription, diarization fixture/contract output, and summary generation.
- Main app shell has route-backed Dashboard, Recordings, Jobs, and Settings
  views.
- Dev inspection/recovery is now supported from the app: recordings can open
  artifact folders, ready artifacts can be opened directly, failed/setup-blocked
  recording jobs can be retried, and pipeline stage messages are visible in the
  dashboard/jobs UI.

Remaining implementation work:

- Real multi-speaker diarization backend execution is still missing. The worker
  currently supports the `diarize.run` contract, fixture turns, and exact
  one-speaker fallback, but not NeMo/Whisper or pyannote execution.
- Release packaging still needs a bundled or bootstrapped worker runtime. The
  desktop app currently launches the worker with `uv run python -m app.main`
  from a nearby `worker` directory.
- CI workflow automation is not present yet for the TypeScript, Rust, and
  Python suites.
- Future schema version upgrade tests are still needed once schema version 2+
  exists.

Remaining QA / validation work:

- Manual recording QA has been reported for macOS and Windows; keep those paths
  in release smoke tests rather than treating them as missing capture code.
- Manual release checks are still needed for Windows, macOS, and Linux
  packages, permissions, install/update behavior, and app startup.
- Linux PulseAudio/PipeWire monitor-device behavior still needs OS-level
  validation and clearer user-facing setup guidance if Linux is a release
  target.
- Global hotkey and always-on-top overlay behavior should be validated per OS
  as part of release QA.
- Real end-to-end ML validation is still needed with installed models,
  optional acceleration, and real provider endpoints.

Accepted limitation:

- The worker remains a CLI JSONL runner rather than a local API server. This is
  an accepted implementation direction for now, but cancellation/shared model
  lifecycle remains weaker than an API server.

## Missing Product Work

### 1. Real Recording

Status: partially addressed.

Original gap: recording did not capture microphone audio, system audio, or a
mixed track and wrote empty/stub files.

Required work status:

- Addressed: Added a Rust `AudioCaptureBackend` trait.
- Addressed: Added `NativeAudioCaptureBackend` using CPAL input capture.
- Addressed: Added a test `FileAudioCaptureBackend` for deterministic lifecycle
  and artifact tests.
- Addressed: Writes durable canonical `recording.wav`, `metadata.json`, and
  `job-log.jsonl` under the recording directory.
- Addressed: Source-level mic/system WAV files are no longer written by default;
  `metadata.json` records source readiness while `recording.wav` remains the
  user-facing audio artifact.
- Addressed: Tracks capture errors per source with `CaptureError`.
- Addressed: Persists capture errors on recordings and writes capture failures
  to `job-log.jsonl`.
- Addressed: Ties global hotkey and overlay visibility to the recorder
  lifecycle, not only local UI state.
- Addressed: Native system capture now attempts CPAL output-device loopback for
  the default system source on macOS and Windows instead of immediately
  reporting setup required.
- Addressed: Linux/BSD system capture now attempts to discover monitor,
  loopback, "what u hear", or stereo-mix style input devices before reporting
  setup required.
- Addressed: Named system audio sources can resolve either matching input
  devices or matching output devices for native loopback-capable backends.
- Partial: Microphone capture uses the default/configured input device.
- Partial: System audio capture uses CPAL output-device loopback where the
  backend supports it and monitor/loopback input devices elsewhere.
- QA: Windows WASAPI loopback is wired through CPAL output-device capture and
  should stay covered by release QA.
- QA: macOS system capture currently depends on CPAL CoreAudio loopback support
  and OS version behavior. ScreenCaptureKit or virtual-audio setup remains a
  fallback/product decision, not a required code gap for the current CPAL path.
- QA: Linux PulseAudio/PipeWire behavior discovers monitor devices but still
  needs OS-level validation.
- Implementation: Linux setup guidance may need a clearer user-facing flow if
  Linux is a supported release target.

Acceptance criteria status:

- Partial: Starting capture creates a native active recording session when an
  input source is available.
- Addressed: Stopping capture writes non-empty WAV files for captured sources.
- Addressed: Capture failure is surfaced through errors and persisted to the
  job log.
- Addressed: Tests cover lifecycle transitions, artifact readiness, non-empty
  test WAV output, partial source readiness, and system-source monitor/default
  name helpers.
- QA: Platform-specific real-system-audio capture should be covered in release
  QA. Add OS-specific implementation only when that QA identifies a failing
  supported path.

### 2. Output Directory and File Layout

Status: mostly addressed.

Original gap: default output path was a relative `Actavoces` directory.

Required work status:

- Addressed: Default output root is `~/actavoces/records`.
- Addressed: Output directory is configurable from Settings with a native
  folder picker.
- Addressed: Configured path is stored in persistent SQLite settings.
- Addressed: Startup and settings save create/configure the records and model
  storage directories.
- Addressed: Recording directories use date-based organization:
  `YYYY/MM/YYYY-MM-DD-HHMMSS-slug`.
- Addressed: Each recording keeps its own stable artifact directory.
- Addressed: Existing recordings keep original artifact paths after settings
  changes.
- Addressed: Artifact set uses one canonical audio file:
  `recording.wav`, `raw-transcript.md`, `raw-segments.json`,
  `diarization.json`, `diarized-transcript.md`, `summary.md`,
  `metadata.json`, and `job-log.jsonl`.
- Addressed: Added recording cleanup that can delete the SQLite recording row
  and known artifact directory.
- Addressed: Recording artifact folders can be opened from the Recordings view.
- Addressed: Ready artifacts can be opened directly from the Recent artifacts
  panel.
- Addressed: Microphone and system-audio source settings now use enumerated
  dropdown selectors instead of free-form text fields, while preserving saved
  values that are not currently connected.
- Addressed: Output and model storage paths use native folder pickers through
  the Tauri dialog plugin.

Acceptance criteria status:

- Addressed: First launch creates/prepares `~/actavoces/records`.
- Addressed: Settings can change the output root.
- Addressed: Existing recordings keep original artifact paths after the setting
  changes.
- Addressed: Tests cover default path resolution, path migration behavior, and
  configured directory creation.
- Addressed: Dev users can inspect produced artifact files/folders from the UI
  without manually finding the records root.

### 3. SQLite Persistence

Status: mostly addressed.

Original gap: Rust backend used in-memory `Mutex<AppSnapshot>`.

Required work status:

- Addressed: Added SQLite-backed persistence through `AppRepository`.
- Addressed: Added tables for `settings`, `recordings`,
  `recording_artifacts`, `pipeline_jobs`, `providers`, `models`,
  `job_events`, and `schema_migrations`.
- Addressed: Database is created under Tauri app data directory as
  `actavoces.sqlite`, not under the records folder.
- Addressed: Snapshot is loaded from SQLite on startup.
- Addressed: Recording create/finish/update/delete operations persist state.
- Addressed: Settings persist in SQLite.
- Addressed: Provider metadata persists in SQLite while secrets go to OS
  keychain.
- Addressed: Model inventory persists in SQLite.
- Addressed: Worker errors are persisted through desktop runtime status
  settings.
- Addressed: Migration startup exists from day one through `schema_migrations`
  and `CREATE TABLE IF NOT EXISTS`.
- Partial: Some repository operations use multiple statements rather than a
  single transaction for every possible lifecycle change.
- Partial: Migration versioning exists, but future upgrade migrations are not
  yet represented beyond schema creation and additive column checks.

Acceptance criteria status:

- Addressed: Restarting the repository restores recordings and job progress.
- Addressed: Jobs can resume after app shutdown because jobs and artifacts are
  persisted.
- Addressed: Failed, setup-blocked, and interrupted downstream job rows can be
  reset to pending for a recording-level retry; capture jobs are intentionally
  excluded because audio capture cannot be regenerated.
- Addressed: Tests cover repository restore, model inventory persistence,
  deletion, retry reset behavior, migration startup behavior indirectly,
  snapshot reconstruction, and artifact path persistence.
- Implementation: Broader migration version upgrade tests are still needed once
  schema version 2+ exists.

### 4. Worker Design

Status: partially addressed with CLI JSONL runner.

Original gap: worker only had a protocol scaffold and `health.check`.

Decision status:

- Addressed: Chose the CLI JSONL worker path for the current implementation.
- Not chosen: Local API server with `127.0.0.1`, random launch token, Unix
  socket, SSE, WebSocket, or polling endpoints.

Required work status:

- Addressed: Tauri can run worker commands through `uv run python -m app.main`.
- Addressed: Worker launch path matches the current `worker/app` package layout.
- Addressed: Tauri can start, stop, and health-check worker runtime status.
- Addressed: Worker exposes command handlers for health, model status, model
  install, transcription, diarization, and summary.
- Addressed: Worker failures are captured and surfaced through persisted
  desktop runtime status and pipeline job state.
- Addressed: Progress events are parsed and reflected in persisted pipeline
  jobs.
- Addressed: Pipeline stages are resumable and consume representative worker
  events.
- Accepted limitation: CLI-per-command design has weaker cancellation and
  shared model lifecycle than the API-server recommendation.
- Accepted limitation: No long-running worker daemon, auth token, socket
  binding, streaming transport, or cancellation API exists.

Acceptance criteria status:

- Addressed: Tauri can start, stop, and health-check the worker status.
- Addressed: Worker failures are captured in SQLite-backed runtime/job state
  and surfaced in the UI.
- Addressed: Progress updates can be shown per pipeline stage from worker
  events.
- Addressed: Stage messages are included in snapshots and displayed in the UI
  for setup/failure/debug visibility.
- Addressed: Worker tests cover command validation and representative
  success/failure/setup events.

### 5. Local Transcription

Status: partially addressed.

Original gap: `faster-whisper` was not wired into the worker and there was no
model setup flow.

Required work status:

- Addressed: Worker dependencies include `faster-whisper`.
- Addressed: `transcribe.run` is implemented.
- Addressed: `transcribe.run` accepts fixture segments for deterministic tests.
- Addressed: `transcribe.run` can call `faster-whisper` when installed.
- Addressed: Stores raw segments as `raw-segments.json`.
- Addressed: Generates `raw-transcript.md` from the same segment data.
- Addressed: Model selection is configurable in Settings.
- Addressed: Supported model options are `small.en`, `medium.en`,
  `large-v3`, and `distil-large-v3`.
- Addressed: Worker supports `models.status` and `models.install`.
- Addressed: Tauri exposes model inventory refresh and model install commands.
- Addressed: Model inventory is persisted in SQLite and shown in Settings.
- Addressed: CPU/GPU compute type is configurable and passed to worker/model
  setup.
- Implementation: Progress reporting currently emits coarse progress, not
  detailed per-segment transcription progress.
- QA: Real transcription depends on installed worker dependencies and models;
  base dev/test coverage uses setup-required paths and mocks.
- QA: Full GPU acceleration QA is not complete.

Acceptance criteria status:

- QA: A real WAV can produce transcript artifacts when `faster-whisper` and
  the selected model are installed; this should stay covered by ML QA.
- Addressed: Progress/setup/complete events are emitted and parsed.
- Addressed: Model missing/setup-required states are visible in Settings and
  pipeline UI.
- Addressed: Transcription setup/failure states can be retried from the
  recording/jobs UI after installing dependencies or models.
- Addressed: Tests cover transcript formatting, worker protocol parsing,
  missing audio, missing dependency setup, mocked model execution, model status,
  and model install.

### 6. Diarization

Status: partially addressed.

Original gap: no diarization backend existed.

Required work status:

- Addressed: Implemented `diarize.run` worker command contract.
- Addressed: `diarize.run` writes `diarization.json` and
  `diarized-transcript.md` when fixture turns are provided.
- Addressed: `diarize.run` can synthesize a valid single-speaker diarization
  when speaker settings explicitly request exactly one speaker.
- Addressed: `diarize.run` reports `diarize.needs_setup` for missing backend
  setup, with backend-specific dependency names for NeMo and pyannote.
- Addressed: Pipeline can run diarization after transcription completes.
- Addressed: Diarization artifacts are persisted and marked ready.
- Addressed: Speaker count settings exist: automatic, exact, min/max range.
- Addressed: Diarization backend setting exists with `nemoWhisper` and
  `pyannote` options.
- Addressed: Speaker turns can be represented in `diarization.json`.
- Implementation: The first supported backend is represented as
  NeMo/Whisper-oriented setup, but actual NeMo diarization execution is not
  implemented.
- Implementation: Pyannote remains an option in settings but is not implemented
  as a real worker backend.
- Addressed: The UI exposes setup-blocked stage messages and retry.
- Addressed: Exact one-speaker diarization is handled without a real
  diarization model.
- Implementation: Multi-speaker detection still requires a real backend.
- Implementation: Speaker label editing/renaming is not implemented.

Acceptance criteria status:

- Addressed: Diarization runs as a resumable background job at the pipeline
  contract level.
- Addressed: Users can see raw transcript artifacts before diarized transcript
  artifacts are ready.
- Addressed: Speaker configuration is available from Settings before running a
  job.
- Addressed: Tests cover diarized transcript rendering from turns and segments.
- Addressed: Diarization setup-required states are inspectable and retryable
  after configuration changes.
- Addressed: Tests cover exact single-speaker diarization and backend-specific
  setup-required dependency reporting.
- Implementation: Real diarization backend execution is not complete.

### 7. Summary, Titles, and Providers

Status: mostly addressed.

Original gap: only a boolean `summaryProviderConfigured` existed.

Required work status:

- Addressed: Added OpenAI-compatible provider settings.
- Addressed: Supports base URL, API key, model, title prompt, summary prompt,
  and enabled/disabled state.
- Addressed: Stores provider API key in the OS keychain where possible through
  `keyring`, not plain SQLite.
- Addressed: SQLite stores only provider metadata and configured/status
  booleans.
- Addressed: UI allows setting and clearing the provider API key.
- Addressed: Summary/title generation is implemented as a worker job.
- Addressed: Summary worker uses OpenAI-compatible `/chat/completions`.
- Addressed: `summary.md` is written.
- Addressed: Recording title is updated from summary completion title when
  provided.
- Addressed: Summary is optional; disabled summary marks summary stage complete
  without blocking transcript/diarization availability.
- Addressed: Summary failures are persisted as job failures and do not remove
  transcript artifacts.
- Product decision: Multiple providers are not implemented; the app supports
  one OpenAI-compatible summary provider.
- QA: Manual verification against real provider endpoints is still needed.

Acceptance criteria status:

- Addressed: User can configure one OpenAI-compatible provider.
- Addressed: Summary generation can be disabled.
- Addressed: Failed summary does not block transcript availability.
- Addressed: Tests cover provider validation, prompt assembly, provider
  failure/setup handling, and Markdown output.

### 8. Settings Page

Status: mostly addressed.

Original gap: UI displayed settings-like values but did not let users change
them.

Required settings areas status:

- Addressed: General settings include folder-picked output directory, captured
  hotkey, configurable overlay corner, and launch at login toggle.
- Addressed: Capture settings include microphone device, system audio source,
  and sample rate.
- Addressed: Microphone and system audio source settings are selected from
  CPAL-enumerated device dropdowns.
- Addressed: Transcription settings include model, language, compute type, and
  folder-picked local model storage.
- Addressed: Language selector includes Auto, English, Russian, Ukrainian
  (`uk`), and Spanish.
- Addressed: Model inventory/status/install controls are present.
- Addressed: Speaker settings include backend, speaker count mode, exact
  speaker count, minimum speakers, and maximum speakers.
- Addressed: Summary provider settings include base URL, API key, model,
  title prompt, summary prompt, and enabled toggle.
- Addressed: Storage displays records folder and database location.
- Addressed: Recording cleanup exists from the Recordings view and backend
  command.
- Addressed: Recordings view includes open-folder and downstream retry-job
  actions for dev inspection and recovery.
- Partial: Cleanup is per-recording, not a full storage cleanup policy UI.
- Addressed: Output and model storage paths use native folder pickers through
  the Tauri dialog plugin.
- Addressed: Launch-at-login is persisted and synchronized with the OS through
  the Tauri autostart plugin.

Acceptance criteria status:

- Addressed: Settings page is reachable from left navigation.
- Addressed: Settings persist across app restarts through SQLite.
- Addressed: Invalid provider/model/path-style configuration is shown through
  validation errors and worker/setup status.
- Addressed: Pipeline setup/failure messages are visible enough to identify the
  missing worker/model/provider step without checking logs first.
- Addressed: Tests cover capture device selector options and disconnected saved
  value preservation.
- Addressed: Tests cover settings validation, provider form state, and hook
  payload/error behavior.
- Addressed: OS launch-at-login integration is implemented through the Tauri
  autostart plugin.

### 9. Navigation and App Shell

Status: mostly addressed.

Original gap: left navigation was decorative and the main page looked like a
landing page.

Required work status:

- Addressed: Replaced decorative nav blocks with real clickable buttons.
- Addressed: Added Dashboard, Recordings, Jobs, and Settings views.
- Addressed: Uses a signal-backed route store consistently.
- Addressed: Removed landing/hero-style product pitch from the main page.
- Addressed: First viewport is a dense app dashboard with capture status,
  recording count, active jobs, summary status, current pipeline, storage, and
  runtime panels.
- Addressed: Dashboard pipeline cards show backend stage messages, not only
  status/progress.
- Addressed: Jobs view identifies recordings by title and exposes retry for
  retryable downstream jobs.
- Addressed: Overlay UI is separate from the main window through a dedicated
  Tauri webview label.
- Partial: There is no separate Models route; model controls live in Settings.

Acceptance criteria status:

- Addressed: Every left-nav item is clickable and has active state.
- Addressed: Main page has no marketing/landing hero.
- Addressed: Dashboard is functional app surface.
- Addressed: Tests cover route changes and active nav state.

### 10. Persistent Recording Overlay and Hotkey

Status: mostly addressed, with manual OS QA remaining.

Original gap: overlay only existed as a fixed element inside the main window.

Required work status:

- Addressed: Added Tauri global shortcut support.
- Addressed: Added `toggle_recording_from_shortcut`.
- Addressed: Created a small always-on-top overlay webview window labeled
  `recording-overlay`.
- Addressed: Overlay state is synchronized with active recording state.
- Addressed: Hotkey is captured through a shortcut recorder in Settings instead
  of free-text entry.
- Addressed: Overlay position is configurable for the four screen corners.
- Addressed: Overlay control is stop-only while recording, so it can no longer
  start a second recording from stale state.
- Addressed: Hotkey registration conflicts/errors are captured in persisted
  runtime status and surfaced in UI.
- QA: OS-level hotkey behavior while another app is focused should stay covered
  by release QA.
- QA: Always-on-top overlay behavior across macOS, Windows, and Linux should
  stay covered by release QA.

Acceptance criteria status:

- QA: Hotkey toggles recording outside focus in implementation; OS-level
  behavior should be validated per release target.
- Addressed: Overlay is visible while recording is active.
- Addressed: Overlay hides when recording stops/fails through lifecycle sync.
- Addressed: Tests cover shortcut command/lifecycle state.
- QA: Manual release QA should cover OS-level hotkey and overlay behavior.

### 11. Test Coverage

Status: partially addressed.

Original gap: no frontend, Rust, or Python behavior tests existed.

Frontend tests status:

- Addressed: Hook behavior tests cover load/start/stop/resume success and
  failure.
- Addressed: Hook behavior tests cover recording folder opening and failed job
  retry.
- Addressed: Hook behavior tests cover capture device selector options.
- Addressed: Route/nav active state tests exist.
- Addressed: Settings validation and provider form state tests exist.
- Addressed: Formatting helper tests exist for duration and timestamp
  formatting.
- Implementation: There is not yet a full rendered UI interaction suite for
  every settings control and recording cleanup confirmation.

Rust tests status:

- Addressed: Settings default path resolution.
- Addressed: SQLite repository behavior.
- Addressed: Recording lifecycle state transitions.
- Addressed: Artifact path generation.
- Addressed: Worker process/API client behavior with mocked responses/events.
- Addressed: Model inventory persistence and parsing.
- Addressed: Storage directory creation and recording deletion.
- Addressed: Recording job retry reset behavior, excluding capture-stage jobs.
- Addressed: Capture device enumeration helper behavior.
- QA: Cross-platform native capture behavior is covered by manual OS QA rather
  than automated integration tests.

Python tests status:

- Addressed: Worker health command.
- Addressed: Transcription command contract with mocked model execution.
- Addressed: Diarization output formatting with fixture segments.
- Addressed: Summary prompt assembly and provider failure/setup handling.
- Addressed: Model install/status setup paths.
- Addressed: Worker package import path is configured for pytest, and worker
  type checking passes with typed protocol payloads.
- Addressed: Worker tests that mock behavior use `mocker: MockerFixture` rather
  than `monkeypatch`.
- QA: Real ML dependency integration tests are not present; current coverage
  uses fixtures and mocks.

Acceptance criteria status:

- Addressed: `pnpm test` runs frontend tests.
- Addressed: Rust unit tests run through `cargo test`.
- Addressed: Python worker tests run through `uv run pytest`.
- Addressed: Worker formatting/linting and basedpyright checks run through
  `uv run task format-and-lint`.
- Implementation: CI workflow is not present yet, so CI does not currently run
  the TypeScript, Rust, and Python suites automatically.

## Implementation Plan

### Phase 1: Make State Real

Status: mostly addressed.

- Addressed: Add SQLite migrations and Rust repositories.
- Addressed: Move settings and recordings out of `Mutex<AppSnapshot>`.
- Addressed: Set default records root to `~/actavoces/records`.
- Addressed: Add settings commands for output directory and configurable
  options.
- Addressed: Add native folder pickers for output and model storage settings.
- Addressed: Add tests for persistence, settings, and artifact paths.
- Partial: Future schema migrations beyond initial/additive columns still need
  explicit versioned upgrade tests.

### Phase 2: Make The Shell Functional

Status: mostly addressed.

- Addressed: Replace decorative left nav with real navigation.
- Addressed: Add Settings, Recordings, Jobs, and Dashboard screens.
- Addressed: Remove landing-style hero/header copy from the main page.
- Addressed: Wire settings forms to Tauri commands.
- Addressed: Add artifact open, recording folder open, job retry, and stage
  message visibility for dev inspection/recovery.
- Addressed: Add route and settings tests.
- Partial: Models are handled inside Settings rather than a dedicated Models
  screen.

### Phase 3: Real Capture

Status: partially addressed.

- Addressed: Implement native capture on the current CPAL-supported input
  path.
- Addressed: Attempt default system output capture through CPAL loopback-capable
  output devices on macOS/Windows and monitor-style inputs on Linux/BSD.
- Addressed: Save a canonical mixed `recording.wav` and track source readiness
  in metadata.
- Addressed: Add global hotkey and native overlay window.
- Addressed: Persist capture progress and failures.
- Addressed: Add Rust lifecycle tests.
- QA: Windows WASAPI loopback and macOS CoreAudio loopback are wired through
  CPAL and should stay covered by release QA.
- QA: Linux PulseAudio/PipeWire behavior discovers monitor devices but still
  needs OS-level validation.
- Implementation: Linux setup guidance may need a clearer user-facing flow if
  Linux is a supported release target.
- QA: Manual OS QA checklist execution.

### Phase 4: Worker Contract

Status: mostly addressed as CLI JSONL.

- Addressed: Decide API server versus CLI worker: current implementation uses
  CLI JSONL.
- Addressed: Implement Tauri worker manager/status commands.
- Addressed: Add worker health, model status, install, transcription,
  diarization, and summary commands.
- Addressed: Align Rust worker launch with the current `worker/app` Python
  package.
- Addressed: Add progress event parsing and persisted pipeline stage updates.
- Addressed: Add stage message propagation and recording-level retry for failed
  or setup-blocked downstream jobs.
- Addressed: Add integration-style Rust tests around mocked worker events.
- Accepted limitation: Cancellation and shared model lifecycle are limited by
  CLI design.

### Phase 5: ML Pipeline

Status: partially addressed.

- Addressed: Implement `faster-whisper` transcription adapter.
- Implementation: Real diarization backend integration is not implemented.
- Addressed: Implement speaker configuration and diarization command contract.
- Addressed: Implement exact one-speaker diarization fallback and
  backend-specific diarization setup reporting.
- Addressed: Implement OpenAI-compatible summary/title generation.
- Addressed: Write final Markdown artifacts from structured data/contracts.
- Addressed: Add fixture-based Python tests.
- Implementation: Real NeMo/Whisper or pyannote diarization execution.
- QA: Real end-to-end ML QA with installed ML dependencies and models.

### Phase 6: Packaging

Status: mostly remaining.

- Partial: Worker runtime is launched with `uv run`, and worker dependencies
  are managed with `uv sync`/`uv.lock`.
- Partial: Model storage and download behavior is defined at app/worker
  contract level.
- Implementation: Bundle or bootstrap the worker runtime for release builds.
- QA: Validate app permissions for mic/system audio on each OS.
- QA: Add and run release build checks for Windows, macOS, and Linux.
- Implementation: Add CI workflow to run TypeScript, Rust, and Python test
  suites.

## Open Questions

- First capture implementation target:
  - Current answer: implemented CPAL input capture on the current development
    path first.
  - QA: Windows WASAPI loopback and macOS system capture are implemented
    through the current CPAL path and should stay covered by release QA.
  - QA: Linux PulseAudio/PipeWire behavior still needs OS-level validation.
  - Implementation: Add Linux setup guidance or platform-specific capture work
    only if Linux QA shows the current monitor-device path is insufficient.
- Separate `mic.wav` and `system.wav` plus mixed `recording.wav`:
  - Answered: no for the user-facing layout. The app now stores only canonical
    `recording.wav`; source readiness remains in `metadata.json`.
- Worker API server or CLI job runner:
  - Answered for now: CLI JSONL job runner.
  - Accepted limitation: revisit only if cancellation/shared model lifecycle
    becomes a hard requirement.
- Pyannote in first diarization pass:
  - Current state: UI exposes `pyannote`, but first real backend execution is
    not implemented.
  - Product decision: decide whether to keep pyannote as first-pass supported
    backend or mark it future-only.
- Cloud summaries in Rust or Python worker:
  - Answered: Python worker handles OpenAI-compatible summary/title generation.
- Provider API keys in OS keychain from first settings implementation:
  - Answered: yes, provider API key storage uses OS keychain where possible.
- Automatic language detection or required language setting:
  - Current answer: language setting exists and defaults to `auto`.
