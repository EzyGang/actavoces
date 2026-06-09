import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import type { useApp } from '../hooks/useApp.hook';

interface AppViewProps {
  app: ReturnType<typeof useApp>;
}

export const AppView = ({ app }: AppViewProps): JSX.Element => (
  <main class='min-h-screen bg-bg-page text-text-primary'>
    <section class='flex min-h-screen flex-col'>
      <header class='flex min-h-16 items-center justify-between border-border-base border-b px-5'>
        <div class='flex items-center gap-3'>
          <div class='flex h-9 w-9 items-center justify-center border border-text-primary font-semibold text-sm'>
            AV
          </div>
          <div class='flex flex-col gap-0.5'>
            <span class='font-semibold text-sm uppercase tracking-[0.05em]'>Actavoces</span>
            <span class='text-text-muted text-xs'>Local meeting notes</span>
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
            {app.status.isRecording.value ? 'Stop' : 'Record'}
          </Button>
        </div>
      </header>

      <div class='grid flex-1 grid-cols-1 gap-0 lg:grid-cols-[minmax(0,1fr)_360px]'>
        <section class='flex flex-col gap-8 px-5 py-8 lg:px-8'>
          <div class='flex max-w-4xl flex-col gap-5'>
            <p class='font-mono text-accent-light text-xs uppercase tracking-[0.05em]'>
              {app.data.snapshot.value.settings.hotkey}
            </p>
            <h1 class='max-w-3xl font-bold text-4xl leading-[1.05] md:text-6xl'>
              Capture calls locally, then let the pipeline finish in stages.
            </h1>
            <p class='max-w-2xl text-lg text-text-secondary'>
              Raw transcripts become available first. Diarization and summaries run as resumable
              jobs and write separate artifacts when ready.
            </p>
            <div class='flex flex-wrap gap-3'>
              <Button
                disabled={app.status.loading.value}
                onClick={app.actions.resumeJobs}
                variant='secondary'
              >
                Resume jobs
              </Button>
              <Button variant='ghost'>Open library</Button>
            </div>
            {app.status.error.value ? (
              <div class='border border-warning-border bg-warning-bg p-4 text-sm text-warning'>
                {app.status.error.value}. Showing local design fallback.
              </div>
            ) : null}
          </div>

          <section class='flex flex-col gap-4'>
            <div class='flex items-end justify-between gap-4'>
              <div class='flex flex-col gap-1'>
                <h2 class='font-semibold text-xl'>Latest pipeline</h2>
                <p class='text-sm text-text-muted'>
                  Capture, transcription, diarization, and summary are independent stages.
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
                    class='flex min-h-32 flex-col justify-between border border-border-base bg-bg-card p-4'
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
              <div class='border border-border-base bg-bg-card p-5 text-text-muted'>
                No recordings yet.
              </div>
            )}
          </section>

          <section class='flex flex-col gap-4'>
            <h2 class='font-semibold text-xl'>Artifacts</h2>
            <div class='grid gap-3 md:grid-cols-3'>
              {app.data.latestRecording.value?.artifacts.map((artifact) => (
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
          </section>
        </section>

        <aside class='flex flex-col gap-6 border-border-base border-t bg-bg-card p-5 lg:border-l lg:border-t-0'>
          <section class='flex flex-col gap-4'>
            <h2 class='font-semibold text-xl'>Settings</h2>
            <div class='flex flex-col gap-3 text-sm'>
              <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
                <span class='text-text-muted'>Output</span>
                <span class='text-right'>{app.data.snapshot.value.settings.outputDirectory}</span>
              </div>
              <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
                <span class='text-text-muted'>Whisper</span>
                <span>{app.data.snapshot.value.settings.whisperModel}</span>
              </div>
              <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
                <span class='text-text-muted'>Diarization</span>
                <span>{app.data.snapshot.value.settings.diarizationBackend}</span>
              </div>
              <div class='flex justify-between gap-4'>
                <span class='text-text-muted'>Summary API</span>
                <span>
                  {app.data.snapshot.value.settings.summaryProviderConfigured
                    ? 'Configured'
                    : 'Not configured'}
                </span>
              </div>
            </div>
          </section>

          <section class='flex flex-col gap-4'>
            <h2 class='font-semibold text-xl'>Library</h2>
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
          </section>
        </aside>
      </div>

      {app.status.isRecording.value ? (
        <div class='fixed right-4 bottom-4 flex items-center gap-3 border border-error-border bg-bg-page px-4 py-3 text-error'>
          <span class='h-2.5 w-2.5 bg-error' />
          <span class='font-mono text-xs uppercase tracking-[0.05em]'>Actavoces is recording</span>
        </div>
      ) : null}
    </section>
  </main>
);
