import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';

interface DashboardRouteProps {
  app: ReturnType<typeof useApp>;
}

export const DashboardRoute = ({ app }: DashboardRouteProps): JSX.Element => (
  <div class='flex flex-col gap-5'>
    <section class='grid gap-4 lg:grid-cols-4'>
      <article class='flex min-h-32 flex-col justify-between border border-border-base bg-bg-card p-5'>
        <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>Capture</span>
        <div class='flex items-end justify-between gap-3'>
          <p class='font-semibold text-3xl'>{app.status.isRecording.value ? 'Live' : 'Idle'}</p>
          <StatusBadge
            label={app.status.isRecording.value ? 'Recording' : 'Ready'}
            status={app.status.isRecording.value ? 'recording' : 'idle'}
          />
        </div>
      </article>
      <article class='flex min-h-32 flex-col justify-between border border-border-base bg-bg-card p-5'>
        <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
          Recordings
        </span>
        <p class='font-semibold text-3xl'>{app.data.snapshot.value.recordings.length}</p>
      </article>
      <article class='flex min-h-32 flex-col justify-between border border-border-base bg-bg-card p-5'>
        <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>Jobs</span>
        <p class='font-semibold text-3xl'>{app.data.activeJobs.value.length}</p>
      </article>
      <article class='flex min-h-32 flex-col justify-between border border-border-base bg-bg-card p-5'>
        <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>Summary</span>
        <p class='font-semibold text-3xl'>
          {app.data.snapshot.value.settings.summaryProviderConfigured ? 'Ready' : 'Off'}
        </p>
      </article>
    </section>

    {app.data.snapshot.value.settings.diarizationBackend === 'pyannote' &&
    (app.data.snapshot.value.settings.diarizationSetupSkipped ||
      !app.data.snapshot.value.settings.diarizationRuntimeReady) ? (
      <section class='border border-warning-border bg-warning-bg p-4 text-warning text-sm'>
        Speaker diarization is not fully set up. Recordings will still transcribe, but speaker
        labels need pyannote setup from Settings.
      </section>
    ) : null}

    <section class='grid gap-4'>
      <article class='flex min-w-0 flex-col gap-5 border border-border-base bg-bg-card p-5'>
        <div class='flex items-center justify-between gap-4'>
          <div class='flex flex-col gap-1'>
            <h1 class='font-semibold text-2xl'>Current pipeline</h1>
            <p class='text-sm text-text-muted'>
              Processing starts automatically after capture stops.
            </p>
          </div>
          {app.data.latestRecordingPipelineStatus.value ? (
            <div class='flex shrink-0 items-center gap-2'>
              <StatusBadge
                label={app.data.latestRecordingPipelineStatus.value.label}
                status={app.data.latestRecordingPipelineStatus.value.status}
              />
              {app.data.latestRecordingActions.value ? (
                <Button
                  class='h-9 px-3'
                  disabled={
                    app.status.loading.value || !app.data.latestRecordingActions.value.canRetry
                  }
                  onClick={app.data.latestRecordingActions.value.onRetry}
                  variant='ghost'
                >
                  Retry
                </Button>
              ) : null}
            </div>
          ) : null}
        </div>

        {app.data.latestRecording.value ? (
          <div class='flex flex-col gap-5'>
            <div class='flex flex-col gap-2'>
              <div class='h-2 bg-bg-page'>
                <div
                  class='h-full bg-text-primary transition-all duration-slow'
                  style={{ width: `${app.data.latestRecordingProgress.value}%` }}
                />
              </div>
              <div class='flex items-center justify-between gap-3 font-mono text-text-muted text-xs'>
                <span>{app.data.latestRecordingPipelineStatus.value?.message}</span>
                <span>{app.data.latestRecordingProgress.value}%</span>
              </div>
            </div>
            <div class='grid gap-3 md:grid-cols-5'>
              {app.data.latestRecording.value.stages.map((stage, index) => (
                <article
                  class='flex min-h-36 flex-col justify-between gap-4 border border-border-base bg-bg-input p-4'
                  key={stage.id}
                >
                  <div class='flex flex-col gap-3'>
                    <div class='flex items-center justify-between gap-3'>
                      <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
                        {index + 1}
                      </span>
                      <StatusBadge label={stage.status} status={stage.status} />
                    </div>
                    <h2 class='font-semibold text-sm'>{stage.label}</h2>
                    <p class='text-text-muted text-xs'>{stage.message}</p>
                    {stage.status === 'running' ? (
                      <span class='inline-flex items-center gap-2 text-accent-light text-xs'>
                        <span class='h-2 w-2 animate-pulse bg-accent-light' />
                        Processing
                      </span>
                    ) : null}
                    {stage.status === 'pending' ? (
                      <span class='inline-flex items-center gap-2 text-text-muted text-xs'>
                        <span class='h-2 w-2 animate-pulse bg-text-muted' />
                        Queued
                      </span>
                    ) : null}
                  </div>
                  <div class='flex flex-col gap-2 pt-1'>
                    <div class='h-1.5 bg-bg-page'>
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
          </div>
        ) : (
          <div class='flex min-h-40 items-center justify-center border border-border-base bg-bg-input p-5 text-text-muted'>
            No pipeline has started.
          </div>
        )}
      </article>

      <div class='grid gap-4 lg:grid-cols-2'>
        <article class='flex min-w-0 flex-col gap-4 border border-border-base bg-bg-card p-5'>
          <div class='flex items-center justify-between gap-4'>
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
              <span class='text-text-muted'>Transcription setup</span>
              <span class='capitalize'>{app.data.snapshot.value.desktop.workerSetupStatus}</span>
            </div>
            <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
              <span class='text-text-muted'>CUDA</span>
              <span>
                {app.data.snapshot.value.desktop.cudaAvailable ? 'Available' : 'CPU fallback'}
              </span>
            </div>
            <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
              <span class='text-text-muted'>Overlay</span>
              <span>{app.data.snapshot.value.desktop.overlayVisible ? 'Visible' : 'Hidden'}</span>
            </div>
            <div class='flex justify-between gap-4 border-border-base border-b pb-3'>
              <span class='text-text-muted'>Hotkey</span>
              <span class='font-mono text-xs'>
                {app.data.snapshot.value.desktop.hotkeyRegistered ? 'Registered' : 'Pending'} -{' '}
                {app.data.displayHotkey(app.data.snapshot.value.settings.hotkey)}
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
            {app.data.snapshot.value.desktop.cudaError ? (
              <div class='border border-warning-border bg-warning-bg p-3 text-warning text-xs'>
                {app.data.snapshot.value.desktop.cudaError}
              </div>
            ) : null}
            <div class='flex justify-between gap-4'>
              <span class='text-text-muted'>Capture</span>
              <span>File backend</span>
            </div>
          </div>
        </article>

        <article class='flex min-w-0 flex-col gap-4 border border-border-base bg-bg-card p-5'>
          <h2 class='font-semibold text-xl'>Storage</h2>
          <div class='flex flex-col gap-3 text-sm'>
            <div class='flex flex-col gap-1 border-border-base border-b pb-3'>
              <span class='text-text-muted'>Records folder</span>
              <span class='wrap-break-word font-mono text-xs'>
                {app.data.snapshot.value.settings.outputDirectory}
              </span>
            </div>
            <div class='flex flex-col gap-1 border-border-base border-b pb-3'>
              <span class='text-text-muted'>Database</span>
              <span class='wrap-break-word font-mono text-xs'>
                {app.data.snapshot.value.settings.databasePath}
              </span>
            </div>
            <div class='flex flex-col gap-1'>
              <span class='text-text-muted'>Model folder</span>
              <span class='wrap-break-word font-mono text-xs'>
                {app.data.snapshot.value.settings.modelStorageDirectory}
              </span>
            </div>
          </div>
        </article>
      </div>
    </section>
  </div>
);
