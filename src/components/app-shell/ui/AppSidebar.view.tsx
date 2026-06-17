import { IconFolderOpen, IconRefresh } from '@tabler/icons-react';
import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import type { useApp } from '../hooks/useApp.hook';

interface AppSidebarProps {
  app: ReturnType<typeof useApp>;
}

export const AppSidebar = ({ app }: AppSidebarProps): JSX.Element => (
  <aside class='hidden min-h-full flex-col gap-5 border-border-base border-l bg-bg-card p-5 xl:flex'>
    <section class='flex flex-col gap-4'>
      <h2 class='font-semibold text-xl'>Recent records</h2>
      {app.data.recentRecordingRows.value.length > 0 ? (
        <div class='flex flex-col gap-3'>
          {app.data.recentRecordingRows.value.map(
            ({
              recording,
              progress,
              pipelineStatus,
              canRetry,
              canRerunSummary,
              onOpenFolder,
              onRetry,
              onRerunSummary
            }) => (
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
                  {pipelineStatus.status === 'pending' ? (
                    <span class='inline-flex items-center gap-2 text-text-muted text-xs'>
                      <span class='h-2 w-2 animate-pulse bg-text-muted' />
                      Queued
                    </span>
                  ) : null}
                </div>
                <span class='font-mono text-text-muted text-xs'>
                  {app.data.formatTimestamp(recording.startedAt)}
                </span>
                <div class='flex gap-2'>
                  <Button
                    aria-label='Open recording folder'
                    class='h-8 w-8 p-0!'
                    onClick={onOpenFolder}
                    title='Open recording folder'
                    variant='ghost'
                  >
                    <IconFolderOpen aria-hidden='true' className='h-4 w-4' />
                  </Button>
                  <Button
                    aria-label='Retry failed jobs'
                    class='h-8 w-8 p-0!'
                    disabled={app.status.loading.value || !canRetry}
                    onClick={onRetry}
                    title='Retry failed jobs'
                    variant='ghost'
                  >
                    <IconRefresh aria-hidden='true' className='h-4 w-4' />
                  </Button>
                  <Button
                    class='h-8 px-3'
                    disabled={app.status.loading.value || !canRerunSummary}
                    onClick={onRerunSummary}
                    variant='ghost'
                  >
                    Rerun summary
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
  </aside>
);
