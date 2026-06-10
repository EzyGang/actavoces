import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import type { useApp } from '../hooks/useApp.hook';

interface AppSidebarProps {
  app: ReturnType<typeof useApp>;
}

export const AppSidebar = ({ app }: AppSidebarProps): JSX.Element => (
  <aside class='hidden min-h-0 flex-col gap-5 overflow-y-auto border-border-base border-l bg-bg-card p-5 xl:flex'>
    <section class='flex flex-col gap-4'>
      <h2 class='font-semibold text-xl'>Recent records</h2>
      {app.data.recentRecordingRows.value.length > 0 ? (
        <div class='flex flex-col gap-3'>
          {app.data.recentRecordingRows.value.map(
            ({ recording, progress, pipelineStatus, canRetry, onOpenFolder, onRetry }) => (
              <article
                class='flex flex-col gap-3 border border-border-base bg-bg-input p-3'
                key={recording.id}
              >
                <div class='flex items-center justify-between gap-3'>
                  <span class='truncate font-semibold text-sm'>{recording.title}</span>
                  <StatusBadge label={pipelineStatus.label} status={pipelineStatus.status} />
                </div>
                <div class='flex flex-col gap-2'>
                  <div class='h-1.5 bg-bg-page'>
                    <div class='h-full bg-text-primary' style={{ width: `${progress}%` }} />
                  </div>
                  <span class='font-mono text-text-muted text-xs'>
                    {progress}% - {pipelineStatus.message}
                  </span>
                  {pipelineStatus.status === 'running' ? (
                    <span class='inline-flex items-center gap-2 text-accent-light text-xs'>
                      <span class='h-2 w-2 animate-pulse bg-accent-light' />
                      Processing
                    </span>
                  ) : null}
                </div>
                <span class='font-mono text-text-muted text-xs'>
                  {app.data.formatTimestamp(recording.startedAt)}
                </span>
                <div class='flex gap-2'>
                  <Button class='h-8 px-3' onClick={onOpenFolder} variant='ghost'>
                    Open
                  </Button>
                  <Button
                    class='h-8 px-3'
                    disabled={app.status.loading.value || !canRetry}
                    onClick={onRetry}
                    variant='ghost'
                  >
                    Retry
                  </Button>
                </div>
              </article>
            )
          )}
        </div>
      ) : (
        <div class='border border-border-base bg-bg-input p-4 text-text-muted text-sm'>
          Records will appear after capture stops.
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
          <span>{app.data.snapshot.value.desktop.hotkeyRegistered ? 'Registered' : 'Pending'}</span>
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
);
