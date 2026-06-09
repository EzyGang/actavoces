import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import type { useApp } from '../hooks/useApp.hook';

interface AppViewProps {
  app: ReturnType<typeof useApp>;
}

export const AppView = ({ app }: AppViewProps): JSX.Element => (
  <main class='min-h-screen overflow-hidden bg-bg-page text-text-primary'>
    <section class='grid min-h-screen grid-rows-[64px_minmax(0,1fr)]'>
      <header class='flex items-center justify-between border-border-base border-b bg-bg-page px-5'>
        <div class='flex items-center gap-4'>
          <div class='flex h-10 w-10 items-center justify-center border border-text-primary bg-text-primary font-semibold text-bg-page text-sm'>
            AV
          </div>
          <div class='flex flex-col gap-0.5'>
            <span class='font-semibold text-sm uppercase tracking-[0.05em]'>ActaVoces</span>
            <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
              {app.data.routeLabel[app.status.activeRoute.value]}
            </span>
          </div>
        </div>

        <div class='flex items-center gap-3'>
          <StatusBadge
            label={app.status.isRecording.value ? 'Recording' : 'Ready'}
            status={app.status.isRecording.value ? 'recording' : 'idle'}
          />
          <Button
            disabled={app.status.loading.value}
            onClick={
              app.status.isRecording.value ? app.actions.stopRecording : app.actions.startRecording
            }
            variant={app.status.isRecording.value ? 'secondary' : 'primary'}
          >
            {app.status.isRecording.value ? 'Stop capture' : 'Start capture'}
          </Button>
        </div>
      </header>

      <div class='grid min-h-0 grid-cols-[176px_minmax(0,1fr)] xl:grid-cols-[176px_minmax(0,1fr)_360px]'>
        <nav class='flex min-h-0 flex-col justify-between border-border-base border-r bg-bg-page p-3'>
          <div class='flex flex-col gap-2'>
            {app.navigation.map((item) => (
              <button
                aria-current={item.isActive ? 'page' : undefined}
                class={
                  item.isActive
                    ? 'border border-text-primary bg-text-primary px-3 py-3 text-left font-semibold text-bg-page text-xs uppercase tracking-[0.05em]'
                    : 'border border-border-base bg-bg-card px-3 py-3 text-left font-semibold text-text-secondary text-xs uppercase tracking-[0.05em] hover:border-text-muted hover:bg-bg-hover hover:text-text-primary'
                }
                key={item.route}
                onClick={item.onSelect}
                type='button'
              >
                {item.label}
              </button>
            ))}
          </div>
          <div class='flex flex-col gap-2 border border-border-base bg-bg-card p-3'>
            <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
              Hotkey
            </span>
            <span class='break-words font-mono text-xs'>
              {app.data.snapshot.value.settings.hotkey}
            </span>
          </div>
        </nav>

        <section class='min-h-0 overflow-y-auto bg-bg-page'>
          <div class='flex flex-col gap-5 p-5 lg:p-7'>
            {app.status.error.value ? (
              <section class='border border-error-border bg-error-bg p-4 text-error text-sm'>
                {app.status.error.value}
              </section>
            ) : null}

            {app.status.activeRoute.value === 'dashboard' ? (
              <div class='flex flex-col gap-5'>
                <section class='grid gap-4 lg:grid-cols-4'>
                  <article class='flex min-h-32 flex-col justify-between border border-border-base bg-bg-card p-5'>
                    <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
                      Capture
                    </span>
                    <div class='flex items-end justify-between gap-3'>
                      <p class='font-semibold text-3xl'>
                        {app.status.isRecording.value ? 'Live' : 'Idle'}
                      </p>
                      <StatusBadge
                        label={app.status.isRecording.value ? 'Recording' : 'Ready'}
                        status={app.status.isRecording.value ? 'recording' : 'idle'}
                      />
                    </div>
                  </article>
                  <article class='flex min-h-32 flex-col justify-between border border-border-base bg-bg-card p-5'>
                    <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
                      Recordings
                    </span>
                    <p class='font-semibold text-3xl'>
                      {app.data.snapshot.value.recordings.length}
                    </p>
                  </article>
                  <article class='flex min-h-32 flex-col justify-between border border-border-base bg-bg-card p-5'>
                    <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
                      Jobs
                    </span>
                    <p class='font-semibold text-3xl'>{app.data.activeJobs.value.length}</p>
                  </article>
                  <article class='flex min-h-32 flex-col justify-between border border-border-base bg-bg-card p-5'>
                    <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
                      Summary
                    </span>
                    <p class='font-semibold text-3xl'>
                      {app.data.snapshot.value.settings.summaryProviderConfigured ? 'Ready' : 'Off'}
                    </p>
                  </article>
                </section>

                <section class='grid gap-4 xl:grid-cols-[minmax(0,1fr)_320px]'>
                  <article class='flex flex-col gap-5 border border-border-base bg-bg-card p-5'>
                    <div class='flex items-center justify-between gap-4'>
                      <div class='flex flex-col gap-1'>
                        <h1 class='font-semibold text-2xl'>Current pipeline</h1>
                        <p class='text-sm text-text-muted'>
                          Audio capture persists immediately; worker stages wait for local setup.
                        </p>
                      </div>
                      <Button
                        disabled={app.status.loading.value}
                        onClick={app.actions.resumeJobs}
                        variant='secondary'
                      >
                        Resume jobs
                      </Button>
                    </div>

                    {app.data.latestRecording.value ? (
                      <div class='grid gap-3 md:grid-cols-5'>
                        {app.data.latestRecording.value.stages.map((stage) => (
                          <article
                            class='flex min-h-36 flex-col justify-between gap-4 border border-border-base bg-bg-input p-4'
                            key={stage.id}
                          >
                            <div class='flex flex-col gap-3'>
                              <StatusBadge label={stage.status} status={stage.status} />
                              <h2 class='font-semibold text-sm'>{stage.label}</h2>
                              <p class='text-text-muted text-xs'>{stage.message}</p>
                            </div>
                            <div class='flex flex-col gap-2 pt-1'>
                              <div class='h-1.5 bg-bg-page'>
                                <div
                                  class='h-full bg-text-primary transition-all duration-slow'
                                  style={{ width: `${stage.progress}%` }}
                                />
                              </div>
                              <span class='font-mono text-text-muted text-xs'>
                                {stage.progress}%
                              </span>
                            </div>
                          </article>
                        ))}
                      </div>
                    ) : (
                      <div class='flex min-h-40 items-center justify-center border border-border-base bg-bg-input p-5 text-text-muted'>
                        No pipeline has started.
                      </div>
                    )}
                  </article>

                  <article class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
                    <h2 class='font-semibold text-xl'>Storage</h2>
                    <div class='flex flex-col gap-3 text-sm'>
                      <div class='flex flex-col gap-1 border-border-base border-b pb-3'>
                        <span class='text-text-muted'>Records folder</span>
                        <span class='break-words font-mono text-xs'>
                          {app.data.snapshot.value.settings.outputDirectory}
                        </span>
                      </div>
                      <div class='flex flex-col gap-1 border-border-base border-b pb-3'>
                        <span class='text-text-muted'>Database</span>
                        <span class='break-words font-mono text-xs'>
                          {app.data.snapshot.value.settings.databasePath}
                        </span>
                      </div>
                      <div class='flex flex-col gap-1'>
                        <span class='text-text-muted'>Model folder</span>
                        <span class='break-words font-mono text-xs'>
                          {app.data.snapshot.value.settings.modelStorageDirectory}
                        </span>
                      </div>
                    </div>
                  </article>
                </section>
              </div>
            ) : null}

            {app.status.activeRoute.value === 'recordings' ? (
              <section class='flex flex-col gap-4'>
                <div class='flex flex-col gap-1'>
                  <h1 class='font-semibold text-2xl'>Recordings</h1>
                  <p class='text-sm text-text-muted'>
                    Existing artifact paths stay fixed after settings changes.
                  </p>
                </div>
                {app.data.recordingRows.value.length > 0 ? (
                  <div class='grid gap-3'>
                    {app.data.recordingRows.value.map(
                      ({ recording, canRetry, onDelete, onOpenFolder, onRetry }) => (
                        <article
                          class='grid gap-4 border border-border-base bg-bg-card p-4 lg:grid-cols-[minmax(0,1fr)_180px]'
                          key={recording.id}
                        >
                          <div class='flex flex-col gap-2'>
                            <div class='flex items-center gap-3'>
                              <h2 class='font-semibold text-base'>{recording.title}</h2>
                              <StatusBadge label={recording.status} status={recording.status} />
                            </div>
                            <span class='font-mono text-text-muted text-xs'>
                              {app.data.formatTimestamp(recording.startedAt)}
                            </span>
                            <span class='break-words font-mono text-text-muted text-xs'>
                              {recording.artifactDirectory}
                            </span>
                          </div>
                          <div class='flex flex-col gap-3 lg:items-end'>
                            <div class='flex flex-col gap-1 lg:items-end'>
                              <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
                                Duration
                              </span>
                              <span class='font-semibold'>
                                {app.data.formatDuration(recording.durationSeconds)}
                              </span>
                            </div>
                            <Button
                              class='h-9 px-3'
                              disabled={app.status.loading.value}
                              onClick={onOpenFolder}
                              variant='secondary'
                            >
                              Open folder
                            </Button>
                            <Button
                              class='h-9 px-3'
                              disabled={app.status.loading.value || !canRetry}
                              onClick={onRetry}
                              variant='secondary'
                            >
                              Retry jobs
                            </Button>
                            <Button
                              class='h-9 px-3'
                              disabled={app.status.loading.value}
                              onClick={onDelete}
                              variant='ghost'
                            >
                              Delete
                            </Button>
                          </div>
                        </article>
                      )
                    )}
                  </div>
                ) : (
                  <div class='flex min-h-40 items-center justify-center border border-border-base bg-bg-card p-5 text-text-muted'>
                    No saved recordings.
                  </div>
                )}
              </section>
            ) : null}

            {app.status.activeRoute.value === 'jobs' ? (
              <section class='flex flex-col gap-4'>
                <div class='flex items-center justify-between gap-4'>
                  <div class='flex flex-col gap-1'>
                    <h1 class='font-semibold text-2xl'>Jobs</h1>
                    <p class='text-sm text-text-muted'>
                      Worker-backed stages are persisted and resumable after restart.
                    </p>
                  </div>
                  <Button
                    disabled={app.status.loading.value}
                    onClick={app.actions.resumeJobs}
                    variant='secondary'
                  >
                    Resume jobs
                  </Button>
                </div>
                {app.data.snapshot.value.jobs.length > 0 ? (
                  <div class='grid gap-3 md:grid-cols-2'>
                    {app.data.jobRows.value.map(({ job, recordingTitle, canRetry, onRetry }) => (
                      <article
                        class='flex flex-col gap-3 border border-border-base bg-bg-card p-4'
                        key={job.id}
                      >
                        <div class='flex items-center justify-between gap-3'>
                          <h2 class='font-semibold text-sm'>{job.stage}</h2>
                          <StatusBadge label={job.status} status={job.status} />
                        </div>
                        <p class='text-text-muted text-sm'>{job.message}</p>
                        <div class='flex flex-col gap-2'>
                          <div class='h-1.5 bg-bg-input'>
                            <div
                              class='h-full bg-text-primary'
                              style={{ width: `${job.progress}%` }}
                            />
                          </div>
                          <span class='font-mono text-text-muted text-xs'>{recordingTitle}</span>
                        </div>
                        <Button
                          class='h-9 px-3'
                          disabled={app.status.loading.value || !canRetry || !onRetry}
                          onClick={onRetry}
                          variant='ghost'
                        >
                          Retry failed jobs
                        </Button>
                      </article>
                    ))}
                  </div>
                ) : (
                  <div class='flex min-h-40 items-center justify-center border border-border-base bg-bg-card p-5 text-text-muted'>
                    No jobs yet.
                  </div>
                )}
              </section>
            ) : null}

            {app.status.activeRoute.value === 'settings' ? (
              <section class='flex flex-col gap-5'>
                <div class='flex items-center justify-between gap-4'>
                  <div class='flex flex-col gap-1'>
                    <h1 class='font-semibold text-2xl'>Settings</h1>
                    <p class='text-sm text-text-muted'>
                      Paths, capture defaults, worker setup, speakers, and provider settings.
                    </p>
                  </div>
                  <Button
                    disabled={
                      app.status.savingSettings.value ||
                      app.data.settingsValidationErrors.value.length > 0
                    }
                    onClick={app.actions.saveSettings}
                    variant='primary'
                  >
                    Save settings
                  </Button>
                </div>

                {app.data.settingsValidationErrors.value.length > 0 ? (
                  <div class='flex flex-col gap-2 border border-warning-border bg-warning-bg p-4 text-sm text-warning'>
                    {app.data.settingsValidationErrors.value.map((error) => (
                      <span key={error}>{error}</span>
                    ))}
                  </div>
                ) : null}

                <section class='grid gap-4 xl:grid-cols-2'>
                  <article class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
                    <h2 class='font-semibold text-xl'>General and capture</h2>
                    <div class='grid gap-3 md:grid-cols-2'>
                      {app.settings.folderFields.map((field) => (
                        <label class='flex flex-col gap-2 text-sm' key={field.key}>
                          <span class='text-text-muted'>{field.label}</span>
                          <div class='flex gap-2'>
                            <input
                              class='h-11 min-w-0 flex-1 border border-border-base bg-bg-input px-3 font-mono text-xs outline-none focus:border-border-focus'
                              readOnly
                              value={field.value}
                            />
                            <Button
                              class='h-11 px-3'
                              onClick={field.onSelect}
                              type='button'
                              variant='secondary'
                            >
                              Choose
                            </Button>
                          </div>
                        </label>
                      ))}
                      <label class='flex flex-col gap-2 text-sm'>
                        <span class='text-text-muted'>{app.settings.hotkeyField.label}</span>
                        <button
                          class='h-11 border border-border-base bg-bg-input px-3 text-left font-mono text-xs outline-none hover:border-border-focus focus:border-border-focus'
                          onClick={app.settings.hotkeyField.onCapture}
                          type='button'
                        >
                          {app.settings.hotkeyField.recording
                            ? 'Press shortcut'
                            : app.settings.hotkeyField.value}
                        </button>
                      </label>
                      {app.settings.captureSelectFields.map((field) => (
                        <label class='flex flex-col gap-2 text-sm' key={field.key}>
                          <span class='text-text-muted'>{field.label}</span>
                          <select
                            class='h-11 border border-border-base bg-bg-input px-3 font-mono text-xs outline-none focus:border-border-focus'
                            onChange={field.onChange}
                            value={field.value}
                          >
                            {field.options.map((option) => (
                              <option key={option} value={option}>
                                {option}
                              </option>
                            ))}
                          </select>
                        </label>
                      ))}
                      {app.settings.numberFields.slice(0, 1).map((field) => (
                        <label class='flex flex-col gap-2 text-sm' key={field.key}>
                          <span class='text-text-muted'>{field.label}</span>
                          <input
                            class='h-11 border border-border-base bg-bg-input px-3 font-mono text-xs outline-none focus:border-border-focus'
                            min='1'
                            onInput={field.onInput}
                            type='number'
                            value={field.value}
                          />
                        </label>
                      ))}
                    </div>
                    <label class='flex items-center gap-3 text-sm'>
                      <input
                        checked={app.settings.toggles.launchAtLogin.checked}
                        class='h-4 w-4'
                        onInput={app.settings.toggles.launchAtLogin.onInput}
                        type='checkbox'
                      />
                      <span>Launch at login</span>
                    </label>
                  </article>

                  <article class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
                    <h2 class='font-semibold text-xl'>Transcription and speakers</h2>
                    <div class='grid gap-3 md:grid-cols-2'>
                      {app.settings.selectFields.map((field) => (
                        <label class='flex flex-col gap-2 text-sm' key={field.key}>
                          <span class='text-text-muted'>{field.label}</span>
                          <select
                            class='h-11 border border-border-base bg-bg-input px-3 font-mono text-xs outline-none focus:border-border-focus'
                            onChange={field.onChange}
                            value={field.value}
                          >
                            {field.options.map((option) => (
                              <option key={option} value={option}>
                                {option}
                              </option>
                            ))}
                          </select>
                        </label>
                      ))}
                      {app.settings.numberFields.slice(1).map((field) => (
                        <label class='flex flex-col gap-2 text-sm' key={field.key}>
                          <span class='text-text-muted'>{field.label}</span>
                          <input
                            class='h-11 border border-border-base bg-bg-input px-3 font-mono text-xs outline-none focus:border-border-focus'
                            min='0'
                            onInput={field.onInput}
                            type='number'
                            value={field.value}
                          />
                        </label>
                      ))}
                    </div>
                    <div class='flex flex-col gap-3 border border-border-base bg-bg-input p-3 text-sm'>
                      <div class='flex items-center justify-between gap-3'>
                        <div class='flex flex-col gap-1'>
                          <span class='text-text-muted'>Selected model</span>
                          <span class='font-mono text-xs'>
                            {app.settings.draft.value.whisperModel}
                          </span>
                        </div>
                        <StatusBadge
                          label={
                            app.data.selectedModel.value?.installed
                              ? 'Installed'
                              : app.data.selectedModel.value?.setupRequired
                                ? 'Setup required'
                                : 'Missing'
                          }
                          status={
                            app.data.selectedModel.value?.installed
                              ? 'complete'
                              : app.data.selectedModel.value?.setupRequired
                                ? 'needsSetup'
                                : 'pending'
                          }
                        />
                      </div>
                      <div class='grid gap-2 sm:grid-cols-2'>
                        <Button
                          class='h-9 px-3'
                          disabled={app.status.loading.value}
                          onClick={app.actions.refreshModels}
                          variant='ghost'
                        >
                          Refresh models
                        </Button>
                        <Button
                          class='h-9 px-3'
                          disabled={
                            app.status.installingModel.value ||
                            app.data.selectedModel.value?.installed
                          }
                          onClick={app.actions.installSelectedModel}
                          variant='secondary'
                        >
                          Install selected
                        </Button>
                      </div>
                      {app.data.snapshot.value.models.length > 0 ? (
                        <div class='grid gap-2'>
                          {app.data.snapshot.value.models.map((model) => (
                            <div
                              class='flex items-center justify-between gap-3 border border-border-base bg-bg-card px-3 py-2'
                              key={model.name}
                            >
                              <span class='font-mono text-xs'>{model.name}</span>
                              <span class='text-text-muted text-xs'>
                                {model.installed
                                  ? 'installed'
                                  : model.setupRequired
                                    ? `${model.dependency} required`
                                    : 'not installed'}
                              </span>
                            </div>
                          ))}
                        </div>
                      ) : null}
                    </div>
                  </article>
                </section>

                <section class='grid gap-4 xl:grid-cols-2'>
                  <article class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
                    <h2 class='font-semibold text-xl'>Summary provider</h2>
                    <label class='flex items-center gap-3 text-sm'>
                      <input
                        checked={app.settings.toggles.summaryEnabled.checked}
                        class='h-4 w-4'
                        onInput={app.settings.toggles.summaryEnabled.onInput}
                        type='checkbox'
                      />
                      <span>Enable summary generation</span>
                    </label>
                    <div class='grid gap-3'>
                      {app.settings.textFields.map((field) => (
                        <label class='flex flex-col gap-2 text-sm' key={field.key}>
                          <span class='text-text-muted'>{field.label}</span>
                          <input
                            class='h-11 border border-border-base bg-bg-input px-3 font-mono text-xs outline-none focus:border-border-focus'
                            onInput={field.onInput}
                            type={field.inputType ?? 'text'}
                            value={field.value}
                          />
                        </label>
                      ))}
                    </div>
                    <div class='flex items-center justify-between gap-3 border border-border-base bg-bg-input p-3 text-sm'>
                      <div class='flex flex-col gap-1'>
                        <span class='text-text-muted'>API key status</span>
                        <span>
                          {app.data.snapshot.value.settings.providerApiKeyConfigured
                            ? 'Saved in keychain'
                            : 'Missing'}
                        </span>
                      </div>
                      <Button
                        class='h-9 px-3'
                        disabled={
                          app.status.savingSettings.value ||
                          !app.data.snapshot.value.settings.providerApiKeyConfigured
                        }
                        onClick={app.actions.clearProviderApiKey}
                        variant='ghost'
                      >
                        Clear key
                      </Button>
                    </div>
                  </article>

                  <article class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
                    <h2 class='font-semibold text-xl'>Prompts</h2>
                    {app.settings.textareaFields.map((field) => (
                      <label class='flex flex-col gap-2 text-sm' key={field.key}>
                        <span class='text-text-muted'>{field.label}</span>
                        <textarea
                          class='min-h-28 resize-y border border-border-base bg-bg-input p-3 font-mono text-xs outline-none focus:border-border-focus'
                          onInput={field.onInput}
                          value={field.value}
                        />
                      </label>
                    ))}
                  </article>
                </section>
              </section>
            ) : null}
          </div>
        </section>

        <aside class='hidden min-h-0 flex-col gap-5 overflow-y-auto border-border-base border-l bg-bg-card p-5 xl:flex'>
          <section class='flex flex-col gap-4'>
            <h2 class='font-semibold text-xl'>Recent artifacts</h2>
            {app.data.latestRecording.value ? (
              <div class='flex flex-col gap-3'>
                {app.data.latestArtifacts.value.map(({ artifact, onOpen }) => (
                  <article
                    class='flex flex-col gap-2 border border-border-base bg-bg-input p-3'
                    key={artifact.kind}
                  >
                    <div class='flex items-center justify-between gap-3'>
                      <span class='font-semibold text-sm'>{artifact.label}</span>
                      <StatusBadge
                        label={artifact.ready ? 'Ready' : 'Pending'}
                        status={artifact.ready ? 'complete' : 'pending'}
                      />
                    </div>
                    <span class='break-words font-mono text-text-muted text-xs'>
                      {artifact.path}
                    </span>
                    <Button
                      class='h-8 px-3'
                      disabled={!artifact.ready}
                      onClick={onOpen}
                      variant='ghost'
                    >
                      Open
                    </Button>
                  </article>
                ))}
              </div>
            ) : (
              <div class='border border-border-base bg-bg-input p-4 text-text-muted text-sm'>
                Artifacts will appear after capture stops.
              </div>
            )}
          </section>

          <section class='flex flex-col gap-4'>
            <div class='flex items-center justify-between gap-3'>
              <h2 class='font-semibold text-xl'>Runtime</h2>
              <Button
                class='h-9 px-3'
                disabled={app.status.loading.value}
                onClick={app.actions.checkWorker}
                variant='ghost'
              >
                Check worker
              </Button>
            </div>
            <div class='flex flex-col gap-3 text-sm'>
              <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
                <span class='text-text-muted'>Capture</span>
                <span>File backend</span>
              </div>
              <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
                <span class='text-text-muted'>Worker</span>
                <span>
                  {app.data.snapshot.value.desktop.workerHealthOk
                    ? 'Healthy'
                    : app.data.snapshot.value.desktop.workerRunning
                      ? 'Running'
                      : 'Stopped'}
                </span>
              </div>
              <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
                <span class='text-text-muted'>Overlay</span>
                <span>{app.data.snapshot.value.desktop.overlayVisible ? 'Visible' : 'Hidden'}</span>
              </div>
              <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
                <span class='text-text-muted'>Hotkey</span>
                <span>
                  {app.data.snapshot.value.desktop.hotkeyRegistered ? 'Registered' : 'Pending'}
                </span>
              </div>
              {app.data.snapshot.value.desktop.hotkeyError ? (
                <div class='border border-warning-border bg-warning-bg p-3 text-warning text-xs'>
                  {app.data.snapshot.value.desktop.hotkeyError}
                </div>
              ) : null}
              {app.data.snapshot.value.desktop.workerError ? (
                <div class='border border-warning-border bg-warning-bg p-3 text-warning text-xs'>
                  {app.data.snapshot.value.desktop.workerError}
                </div>
              ) : null}
              <div class='flex justify-between gap-4'>
                <span class='text-text-muted'>Database</span>
                <span>SQLite</span>
              </div>
            </div>
          </section>
        </aside>
      </div>

      {app.status.isRecording.value ? (
        <div class='fixed right-4 bottom-4 flex items-center gap-3 border border-error-border bg-bg-page px-4 py-3 text-error'>
          <span class='h-2.5 w-2.5 bg-error' />
          <span class='font-mono text-xs uppercase tracking-[0.05em]'>ActaVoces is recording</span>
        </div>
      ) : null}
    </section>
  </main>
);
