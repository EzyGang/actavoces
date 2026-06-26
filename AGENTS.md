# Agent Instructions

Project-wide reference guide. Ingest once and treat as implicit context for future work in this repository.

## Development Environment

Prerequisites for a fresh checkout:

- Node.js 22 with `pnpm`
- Rust stable with Cargo
- Tauri v2 system dependencies for the target OS
- Python 3.14
- `uv`
- OS microphone and audio-capture permissions

### Initial setup:

```bash
pnpm install
pnpm sync:py
```

### Validation shortcuts:

```bash
pnpm format && pnpm validate
pnpm test:all
```

## Core Principles

- Read the relevant folder structure before implementing changes.
- Prefer simple, readable, maintainable code. Code is a liability; avoid over-abstraction and unnecessary indirection.
- Follow DRY. Do not duplicate logic unless there is no reasonable shared shape.
- Reuse existing code and patterns. If unsure whether something exists, search first and implement only if it does not.
- Keep edits scoped to the user request and the affected domain.
- Do not create comments or documentation unless they clarify something genuinely non-obvious.
- Do not create `index.ts` files or re-export barrels.

## Tech Stack

### Frontend

- TypeScript
- Preact 10
- `@preact/signals` for state
- React compatibility through `preact/compat`
- `@base-ui/react` unstyled components
- Vite
- Tailwind CSS v4
- `wouter-preact` routing
- `clsx` for class composition

### Rust Backend / Desktop

- Tauri v2
- Rust code in `src-tauri/src`
- Desktop config in `src-tauri/tauri.conf.json`
- SQLite through `rusqlite`
- Native capture through `cpal`

### Python Worker

- Python 3.14
- `uv`
- `taskipy`
- `pydantic`
- `pydantic-ai`

## Commands

### Frontend / Desktop

```bash
pnpm dev              # Vite dev server
pnpm tauri dev        # Tauri dev app (uses port 1420)
pnpm build            # Production Tauri build
pnpm build:web        # Vite-only production build -> dist/
pnpm preview          # Preview production build

pnpm validate         # Lint + type-check
pnpm fix              # Format all + lint all
pnpm lint             # Biome check
pnpm lint:ts          # Biome check + type-check
pnpm lint:rust        # Cargo fmt check + clippy + cargo check
pnpm lint:all         # TypeScript, Rust, and Python lint
pnpm sync:py          # Sync locked Python worker dependencies
pnpm format           # Biome check --write --unsafe
pnpm format:rust      # Format and fix Rust where possible
pnpm format:py        # Format Python worker
pnpm format:all       # Format TypeScript, Rust, and Python
pnpm type-check       # tsc -b

pnpm test             # Vitest run
pnpm test:rust        # Rust tests
pnpm test:py          # Python worker tests
pnpm test:all         # Frontend, Rust, and Python tests
pnpm test:watch       # Vitest watch mode

pnpm ci               # Sync Python deps, lint all, test all
pnpm ci-run           # Biome CI check
pnpm ver:patch        # Bump root package.json patch version without tag
pnpm ver:minor        # Bump root package.json minor version without tag
pnpm ver:major        # Bump root package.json major version without tag
```

### Python Worker

Run worker commands from the `worker/` directory unless the command explicitly sets another working directory.

```bash
uv run task format-and-lint
uv run task ruff
uv run task ruff-lint
uv run task pyright-lint
uv run task tests
uv run pytest tests/test_main.py -v
uv run pytest tests/test_main.py::test_sync_runs -v
uv run pytest tests/ -k "asyncio" -v
```

## Repository Layout

```text
src/                 TypeScript/Preact frontend
src-tauri/           Rust/Tauri backend and desktop shell
worker/              Python AI worker
public/              Static frontend assets
```

## TypeScript Frontend

### Frontend Directory Structure

```text
src/
  components/      feature-oriented UI and shared UI primitives
  pages/           thin route wrappers
  hooks/           shared generic hooks
  stores/          global signal stores
  services/        Tauri/API request helpers
  utils/           cross-feature helpers
  routes/          route definitions
  types/           TypeScript type definitions
```

Current feature layout:

```text
src/components/app-shell/
  containers/
    App.container.tsx
  hooks/
    useApp.hook.ts
    useAppNavigation.hook.ts
    useAppRuntime.hook.ts
    appRuntime.helpers.ts
  ui/
    App.view.tsx                              app shell and route selection
    AppSidebar.view.tsx                       desktop sidebar

src/components/dashboard/
  ui/
    DashboardRoute.view.tsx                   dashboard route

src/components/jobs/
  ui/
    JobsRoute.view.tsx                        job monitor route

src/components/recordings/
  hooks/
    useRecordings.hook.ts
    recording.helpers.ts
  ui/
    RecordingsSection.view.tsx                recordings route

src/components/settings/
  hooks/
    useSettings.hook.ts
    settings.helpers.ts
    settingsFields.ts
    settingsSelectFields.ts
  ui/
    SettingsRoute.view.tsx                    settings route shell
    SettingsGeneralCapturePanel.view.tsx      settings panel
    SettingsTranscriptionSpeakersPanel.view.tsx
    SettingsSummaryProviderPanel.view.tsx
    SettingsPromptsPanel.view.tsx

src/components/setup/
  ui/
    SetupRoute.view.tsx                       first-run/setup route

src/components/recording-overlay/
  containers/
    RecordingOverlay.container.tsx
  hooks/
    useRecordingOverlay.hook.ts
  ui/
    RecordingOverlay.view.tsx                 floating capture overlay

src/components/updates/
  hooks/
    useUpdates.hook.ts

src/components/worker-runtime/
  hooks/
    useWorkerRuntime.hook.ts
```

Shared UI primitives live in `src/components/shared/ui/`.

As features grow, group related files into semantic subfolders:

```text
components/<feature>/
  ui/
    list-page/
    form/
    table/
    timeline/
  containers/
    list-page/
    form/
  hooks/
    form/
    list-page/
  context/
```

### Strict Triplet Pattern

Every feature should use this shape:

- `hooks/useFeature.hook.ts`: data fetching, side effects, signals, pre-bound callbacks
- `ui/Feature.view.tsx`: pure JSX, styles, layout
- `containers/Feature.container.tsx`: hook-to-view glue, ideally 20 lines or less

Rules:

- Use exact naming: `Foo.view.tsx`, `useFoo.hook.ts`, `Foo.container.tsx`.
- Views contain no side effects, no inline handlers, no business logic, and no local helper definitions.
- Views should stay under 300 lines. Split presentational subcomponents when needed.
- Hooks own side effects, service calls, signals, derived data, and callback binding.
- Containers only pass hook output into views.
- Group container-to-view props into `{ data, status, actions }`.
- Keep page components in `src/pages` thin: route handling plus one container.
- Keep `app-shell` composition-focused. Put route-specific state, field builders, row builders, and actions in the owning feature folder.
- If a container would pass more than 6-8 props, introduce context.
- Views consuming context should receive zero props.

### Context Pattern

Use Preact context when prop drilling gets heavy.

- File naming: `FooContext.tsx`
- Exports: `FooProvider` and `useFooContext()`
- Keep `FooContextValue` in the context file
- The feature hook should return the full context value:

```tsx
export const FooContainer = ({ id }: { id: string }): JSX.Element => (
  <FooProvider value={useFooForm(id)}>
    <FooView />
  </FooProvider>
);
```

### TypeScript Rules

<very_important_block>

- `*.view.tsx` files should contain only styles and layout. No logic, functions, side effects, or variable definitions.
- Use `export const foo = () => {}`. Do not put export lists at the bottom of the file.
- Do not use `function App()`. Use `const App = () => {}` style definitions.
- Use `class=""` in Preact JSX. Use `className=""` only for specific compatibility edge cases.
- Use `@preact/signals` for reactivity. Global state uses `signal()`. Component-scoped state uses `useSignal()`.
- Do not use `useState`.
- Use strict TypeScript. Do not rely on implicit `any`.
- Use `for...of` instead of `forEach`.
- Reuse instead of reimplementing.
- Do not create `index.ts` re-export files.

Bad:

```ts
objs.forEach((obj) => console.log(JSON.stringify(obj)));
```

Good:

```ts
for (const obj of objs) {
  console.log(JSON.stringify(obj));
}
```

</very_important_block>

### Stores

Stores are plain objects with signals. Do not use classes, getters, or setters.

```ts
import { signal, type Signal } from "@preact/signals";

interface FeatureStore {
  data: Signal<string[]>;
  loading: Signal<boolean>;
}

export const featureStore: FeatureStore = {
  data: signal([]),
  loading: signal(false),
};
```

### Services

Service files live in `src/services/<domain>/`.

- Keep service functions pure request/response wrappers.
- Return typed data matching backend models.
- Keep Tauri `invoke` calls in desktop service modules.
- Keep HTTP wrappers in `utils/api/*` if HTTP APIs are introduced.

### Testing

- Use Vitest for frontend tests.
- Tests live under `src/tests`.
- Prefer testing hooks/helpers for logic and rendered interaction tests for UI behavior.

## Frontend Design System

### Visual Direction

- Dark-first minimalist UI.
- Pure black body background.
- Deep charcoal surfaces for cards and panels.
- High-contrast white typography.
- Border-heavy depth using faint white-opacity borders.
- Avoid heavy shadows and gradients.
- All theme colors are defined with OKLCH in `style.css`.
- Do not use HEX or RGBA in theme tokens.

### Typography

- Sans-serif: `Inter`
- Monospace: `JetBrains Mono`
- Body text: `1rem`, line-height `1.5`, `oklch(100% 0 0 / 0.8)`
- Use uppercase only for small labels, nav, and CTAs.

### Color Palette

- Background: `oklch(0% 0 0)`
- Surface: `oklch(14% 0 0)`
- Card panels: `oklch(20% 0 0)`
- Subtle panels: `oklch(22% 0 0)`
- Raised panels: `oklch(23% 0 0)`
- Primary text: `oklch(100% 0 0)`
- Secondary text: `oklch(100% 0 0 / 0.8)`
- Muted text: `oklch(65% 0 0)`
- Borders: `oklch(100% 0 0 / 0.1)` or `oklch(26% 0 0)`
- Success green: `oklch(82% 0.2 145)`
- Emerald accent: `oklch(78% 0.18 165)`
- Error red: `oklch(55% 0.2 25)`
- Warning orange: `oklch(65% 0.2 45)`

### Components

- Cards use `bg-surface`, `border border-white/10`, no shadow, and no radius beyond `0.25rem`.
- Inputs use `bg-surface`, subtle borders, and clear focus borders.
- Tags and badges are small, uppercase, and border-based.
- Primary buttons are white on black with square edges.
- Secondary buttons are transparent with subtle borders.

### Layout

Always use flexbox with `gap` for sibling spacing. Do not use `space-y-*` or child margins for layout spacing.

```tsx
<div class="flex flex-col gap-4">
  <Item />
  <Item />
</div>
```

Parent containers control spacing.

### Theme

- Theme modes: `system`, `light`, `dark`
- Persist theme mode in localStorage.
- Use `data-theme` on `<html>`.

## Rust Backend

### Rust Directory Structure

```text
src-tauri/src/
  lib.rs              Tauri builder, plugin setup, command registration
  main.rs             binary entrypoint
  app/                Tauri command registration and app orchestration
  domain/             shared DTOs and state types
  storage/            SQLite repository and persistence
  worker/             worker bootstrap/runtime command execution
  capture/            native audio capture and mixing
  settings/           settings defaults, validation, keyring secrets
  artifacts/          artifact/stage/path helpers
  diagnostics.rs      local diagnostic log writer
  utils/              shared conversion and filesystem helpers
  tests.rs            Rust unit tests
```

Current app command layout:

```text
src-tauri/src/app/
  mod.rs
  commands.rs             command module hub and init-facing re-exports
  commands/
    models.rs             model inventory and install commands
    overlay.rs            recording overlay window positioning
    pipeline.rs           pipeline resume and stage processing
    recordings.rs         recording lifecycle and recording commands
    settings.rs           settings, autostart, hotkey, diarization setup
    snapshot.rs           snapshot and diagnostics commands
    speaker_labels.rs     speaker label artifact rewriting
    worker.rs             worker bootstrap, health, and status commands
```

### Rust Rules

- Keep `lib.rs` focused on initialization, plugins, managed state, and command registration.
- Keep `src-tauri/tauri.conf.json` `version` pointed at `../package.json`; the root package version is the release source of truth.
- Keep `src-tauri/src/app/commands.rs` as a small hub. Put command handlers in focused modules under `src-tauri/src/app/commands/`.
- Put non-command domain logic in modules under `src-tauri/src/<domain>/`.
- Keep `tauri::generate_handler!` pointed at the module that defines each `#[tauri::command]`; Tauri command macro helpers are not available through re-exports.
- Tauri command handlers marked with `#[tauri::command]` should be `pub`.
- Internal cross-module helpers should be `pub(crate)`.
- Prefer async setup for expensive startup work. Manage state immediately, spawn initialization, then emit readiness/snapshot events.
- Do not block `.setup()` with heavy work unless it is required before the window can exist.
- Keep repository/database access behind the storage layer.
- Keep worker process concerns in `worker/`.
- Keep native audio capture concerns in `capture/`.
- Use `cargo fmt`.
- Run Rust checks after backend changes.

### SQLite Migrations

- Keep SQLite schema changes additive, idempotent, and non-destructive by default.
- Prefer defensive setup operations: `CREATE TABLE IF NOT EXISTS`, column-existence checks before `ALTER TABLE ADD COLUMN`, `INSERT OR IGNORE` defaults, and read-time fallbacks for settings.
- Treat `schema_migrations` as informational unless a proper ordered migration runner is introduced. Do not assume versioned migration history exists.
- Do not drop tables, drop columns, rewrite existing data destructively, or use reset-style migrations without explicit user approval.
- If a replacement schema is needed, create new tables alongside old tables, copy/transform data safely, and keep older data available unless removal is explicitly requested.
- Settings are stored as key-value rows; new settings should usually be added with defaults plus safe fallback parsing for existing databases.

### Rust Style

- Put trait bounds in `where` clauses.
- Prefer `Self` in impl blocks where applicable.
- Use early returns for guard clauses.
- Avoid `if let ... else`; prefer `match` when both branches matter.
- Use full logging macro paths if logging is introduced.
- Prefer `.to_owned()` for `&str` to `String`.
- Max file size is 350 lines.
- Keep imports grouped in this order: std, external crates, current crate.
- Use one `use` per crate group and let `cargo fmt` handle ordering.
- Don't write tests inline, always put them into a separate file.

## Python Worker

Before Python implementation, inspect the worker folder structure. The worker follows a clean/domain-oriented style.

### Python Rules

<important_rules>

- Do not create comments unless absolutely necessary.
- Do not add docstrings unless the behavior is critical, confusing, or explicitly requested.
- Use `uv` as the package manager.
- Use task commands from `pyproject.toml`.
- Use absolute imports from the `app.` prefix.
- No local imports except when strictly necessary to avoid circular imports.
- Import order is enforced by Ruff/isort: standard library, third-party, first-party.
- All function arguments and return types must be annotated.
- Use `list[str]`, `dict[str, Any]`, and other parameterized generics.
- Use `Any` instead of `object` for generic values.
- Use `str | None`, not `Optional[str]`.
- Never silence the type checker unless the suppression already existed.
- Prefer keyword arguments: `func(a=1, b=2)`.
- Keep functions under 30 lines unless there is a strong reason.
- Keep files under 200 lines except database model files.
- Do not use `__all__`.
- Do not re-export from `__init__.py`; import directly from the defining module.
- Use Python 3.14 generic syntax, such as `def foo[**P, T](...)`.
- Use f-strings only. Do not use `.format()` or `%` formatting.
- When creating Pydantic models from existing dicts, JSON, or ORM objects with matching fields, use `.model_validate()`, `.model_validate_json()`, or `.model_validate(obj, from_attributes=True)`, make sure to avoid the following:
```python
class TranscriptionChunkReference(BaseModel): ...

# Bad, manual recreation where each field is 1 to 1 mapped
TranscriptionChunkReference(
    chunk_id=chunk.chunk_id,
    source_start=chunk.source_start,
    source_end=chunk.source_end,
    segment_id_start=chunk.segment_id_start,
    segment_id_end=chunk.segment_id_end,
    word_id_start=chunk.word_id_start,
    word_id_end=chunk.word_id_end,
)

# Good, using .model_validate()
TranscriptionChunkReference.model_validate(chunk)
```
- Do not return raw `dict` or `list[dict]` for complex objects. Use `BaseModel` DTOs.

</important_rules>

### Python Testing

- Use pytest.
- Use `mocker: MockerFixture` for mocks.
- Tests should live under the worker test structure.
- Run `uv run task tests` after worker changes.
- Run `uv run task ruff-lint` and `uv run task pyright-lint` after typed Python changes.
