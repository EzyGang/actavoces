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
              {app.data.snapshot.value.settings.hotkey}
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

      <div class='grid min-h-0 grid-cols-[72px_minmax(0,1fr)] lg:grid-cols-[72px_minmax(0,1fr)_360px]'>
        <nav class='flex flex-col items-center justify-between border-border-base border-r bg-bg-page py-5'>
          <div class='flex flex-col gap-3'>
            <div class='h-9 w-9 border border-text-primary bg-text-primary' />
            <div class='h-9 w-9 border border-border-base bg-bg-card' />
            <div class='h-9 w-9 border border-border-base bg-bg-card' />
          </div>
          <div class='flex h-9 w-9 items-center justify-center border border-border-base bg-bg-card'>
            <span class='h-2 w-2 bg-success' />
          </div>
        </nav>

        <section class='min-h-0 overflow-y-auto bg-bg-page'>
          <div class='flex flex-col gap-6 p-5 lg:p-7'>
            <section class='grid gap-4 xl:grid-cols-[minmax(0,1fr)_280px]'>
              <article class='flex min-h-72 flex-col justify-between border border-border-base bg-bg-card p-6'>
                <div class='flex items-start justify-between gap-6'>
                  <div class='flex max-w-2xl flex-col gap-4'>
                    <StatusBadge
                      label={app.status.isRecording.value ? 'Live capture' : 'Standby'}
                      status={app.status.isRecording.value ? 'recording' : 'idle'}
                    />
                    <div class='flex flex-col gap-3'>
                      <h1 class='max-w-3xl font-semibold text-4xl leading-[1.05] md:text-6xl'>
                        Private meeting capture, staged into usable notes.
                      </h1>
                      <p class='max-w-2xl text-base text-text-secondary'>
                        ActaVoces records locally, creates a raw transcript first, then finishes
                        speaker labels and summaries as resumable jobs.
                      </p>
                    </div>
                  </div>
                  <div class='hidden h-24 w-24 items-center justify-center border border-border-base bg-bg-input xl:flex'>
                    <span
                      class={
                        app.status.isRecording.value
                          ? 'h-10 w-10 bg-error'
                          : 'h-10 w-10 bg-text-muted'
                      }
                    />
                  </div>
                </div>

                <div class='flex flex-wrap items-end justify-between gap-4 border-border-base border-t pt-5'>
                  <div class='grid grid-cols-3 gap-3'>
                    <div class='flex min-w-28 flex-col gap-1 border border-border-base bg-bg-input p-3'>
                      <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
                        Model
                      </span>
                      <span class='font-semibold text-sm'>
                        {app.data.snapshot.value.settings.whisperModel}
                      </span>
                    </div>
                    <div class='flex min-w-28 flex-col gap-1 border border-border-base bg-bg-input p-3'>
                      <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
                        Speakers
                      </span>
                      <span class='font-semibold text-sm'>
                        {app.data.snapshot.value.settings.diarizationBackend}
                      </span>
                    </div>
                    <div class='flex min-w-28 flex-col gap-1 border border-border-base bg-bg-input p-3'>
                      <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
                        Summary
                      </span>
                      <span class='font-semibold text-sm'>
                        {app.data.snapshot.value.settings.summaryProviderConfigured
                          ? 'Ready'
                          : 'Offline'}
                      </span>
                    </div>
                  </div>
                  <Button
                    disabled={app.status.loading.value}
                    onClick={app.actions.resumeJobs}
                    variant='secondary'
                  >
                    Resume jobs
                  </Button>
                </div>
              </article>

              <article class='flex min-h-72 flex-col justify-between border border-border-base bg-bg-card p-5'>
                <div class='flex flex-col gap-3'>
                  <span class='font-mono text-accent-light text-[11px] uppercase tracking-[0.05em]'>
                    Output vault
                  </span>
                  <h2 class='font-semibold text-2xl leading-tight'>
                    {app.data.snapshot.value.settings.outputDirectory}
                  </h2>
                  <p class='text-sm text-text-muted'>
                    Markdown artifacts are written separately for raw transcript, diarized
                    transcript, summary, and job diagnostics.
                  </p>
                </div>
                <div class='grid grid-cols-2 gap-3'>
                  <div class='border border-border-base bg-bg-input p-3'>
                    <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
                      Saved
                    </span>
                    <p class='font-semibold text-3xl'>
                      {app.data.snapshot.value.recordings.length}
                    </p>
                  </div>
                  <div class='border border-border-base bg-bg-input p-3'>
                    <span class='font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
                      Active
                    </span>
                    <p class='font-semibold text-3xl'>{app.status.isRecording.value ? '1' : '0'}</p>
                  </div>
                </div>
              </article>
            </section>

            {app.status.error.value ? (
              <section class='border border-error-border bg-error-bg p-4 text-error text-sm'>
                {app.status.error.value}
              </section>
            ) : null}

            <section class='flex flex-col gap-4'>
              <div class='flex items-end justify-between gap-4'>
                <div class='flex flex-col gap-1'>
                  <h2 class='font-semibold text-xl'>Pipeline</h2>
                  <p class='text-sm text-text-muted'>
                    Capture, transcription, alignment, diarization, and summary run as independent
                    stages.
                  </p>
                </div>
                {app.data.latestRecording.value ? (
                  <StatusBadge
                    label={app.data.latestRecording.value.status}
                    status={app.data.latestRecording.value.status}
                  />
                ) : null}
              </div>

              {app.data.latestRecording.value ? (
                <div class='grid gap-3 md:grid-cols-5'>
                  {app.data.latestRecording.value.stages.map((stage) => (
                    <article
                      class='flex min-h-36 flex-col justify-between border border-border-base bg-bg-card p-4'
                      key={stage.id}
                    >
                      <div class='flex flex-col gap-3'>
                        <StatusBadge label={stage.status} status={stage.status} />
                        <h3 class='font-semibold text-sm'>{stage.label}</h3>
                      </div>
                      <div class='flex flex-col gap-2'>
                        <div class='h-1.5 bg-bg-input'>
                          <div
                            class='h-full bg-text-primary transition-all duration-slow'
                            style={{ width: `${stage.progress}%` }}
                          />
                        </div>
                        <span class='font-mono text-text-muted text-xs'>{stage.progress}%</span>
                      </div>
                    </article>
                  ))}
                </div>
              ) : (
                <div class='flex min-h-36 items-center justify-center border border-border-base bg-bg-card p-5 text-text-muted'>
                  No active pipeline.
                </div>
              )}
            </section>

            <section class='flex flex-col gap-4'>
              <h2 class='font-semibold text-xl'>Artifacts</h2>
              {app.data.latestRecording.value ? (
                <div class='grid gap-3 md:grid-cols-3'>
                  {app.data.latestRecording.value.artifacts.map((artifact) => (
                    <article
                      class='flex min-h-36 flex-col justify-between border border-border-base bg-bg-card p-4'
                      key={artifact.kind}
                    >
                      <div class='flex flex-col gap-2'>
                        <StatusBadge
                          label={artifact.ready ? 'Ready' : 'Pending'}
                          status={artifact.ready ? 'complete' : 'pending'}
                        />
                        <h3 class='font-semibold text-sm'>{artifact.label}</h3>
                      </div>
                      <p class='break-words font-mono text-text-muted text-xs'>{artifact.path}</p>
                    </article>
                  ))}
                </div>
              ) : (
                <div class='flex min-h-32 items-center justify-center border border-border-base bg-bg-card p-5 text-text-muted'>
                  Artifacts will appear after capture stops.
                </div>
              )}
            </section>
          </div>
        </section>

        <aside class='hidden min-h-0 flex-col gap-6 overflow-y-auto border-border-base border-l bg-bg-card p-5 lg:flex'>
          <section class='flex flex-col gap-4'>
            <h2 class='font-semibold text-xl'>Library</h2>
            {app.data.snapshot.value.recordings.length > 0 ? (
              <div class='flex flex-col gap-3'>
                {app.data.snapshot.value.recordings.map((recording) => (
                  <article
                    class='flex flex-col gap-3 border border-border-base bg-bg-input p-4'
                    key={recording.id}
                  >
                    <div class='flex items-start justify-between gap-3'>
                      <div class='flex flex-col gap-1'>
                        <h3 class='font-semibold text-sm'>{recording.title}</h3>
                        <span class='text-text-muted text-xs'>
                          {app.data.formatTimestamp(recording.startedAt)}
                        </span>
                      </div>
                      <StatusBadge label={recording.status} status={recording.status} />
                    </div>
                    <span class='font-mono text-text-muted text-xs'>
                      {app.data.formatDuration(recording.durationSeconds)}
                    </span>
                  </article>
                ))}
              </div>
            ) : (
              <div class='border border-border-base bg-bg-input p-4 text-text-muted text-sm'>
                No saved recordings.
              </div>
            )}
          </section>

          <section class='flex flex-col gap-4'>
            <h2 class='font-semibold text-xl'>System</h2>
            <div class='flex flex-col gap-3 text-sm'>
              <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
                <span class='text-text-muted'>Capture</span>
                <span>Windows first</span>
              </div>
              <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
                <span class='text-text-muted'>Worker</span>
                <span>uv sidecar</span>
              </div>
              <div class='flex justify-between gap-4'>
                <span class='text-text-muted'>Database</span>
                <span>SQLite planned</span>
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
