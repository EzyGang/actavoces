import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';

interface JobsRouteProps {
  app: ReturnType<typeof useApp>;
}

export const JobsRoute = ({ app }: JobsRouteProps): JSX.Element => (
  <section class='flex flex-col gap-4'>
    <div class='flex flex-col gap-1'>
      <h1 class='font-semibold text-2xl'>Jobs</h1>
      <p class='text-sm text-text-muted'>Debug view grouped by recording.</p>
    </div>
    {app.data.groupedJobRows.value.length > 0 ? (
      <div class='grid gap-4'>
        {app.data.groupedJobRows.value.map(
          ({ recording, progress, pipelineStatus, canRetry, jobs, onRetry }) => (
            <article
              class='flex flex-col gap-4 border border-border-base bg-bg-card p-4'
              key={recording.id}
            >
              <div class='grid gap-4 lg:grid-cols-[minmax(0,1fr)_160px]'>
                <div class='flex min-w-0 flex-col gap-2'>
                  <div class='flex items-center gap-3'>
                    <h2 class='truncate font-semibold text-base'>{recording.title}</h2>
                    <StatusBadge label={pipelineStatus.label} status={pipelineStatus.status} />
                  </div>
                  <span class='font-mono text-text-muted text-xs'>
                    {app.data.formatTimestamp(recording.startedAt)}
                  </span>
                  <div class='flex flex-col gap-2'>
                    <div class='h-1.5 bg-bg-input'>
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
                </div>
                <Button
                  class='h-9 px-3 lg:justify-self-end'
                  disabled={app.status.loading.value || !canRetry}
                  onClick={onRetry}
                  variant='ghost'
                >
                  Retry failed jobs
                </Button>
              </div>
              <div class='grid gap-2 md:grid-cols-2 xl:grid-cols-3'>
                {jobs.map((job) => (
                  <article
                    class='flex flex-col gap-2 border border-border-base bg-bg-input p-3'
                    key={job.id}
                  >
                    <div class='flex items-center justify-between gap-3'>
                      <span class='font-semibold text-sm'>{job.stage}</span>
                      <StatusBadge label={job.status} status={job.status} />
                    </div>
                    <p class='line-clamp-2 text-text-muted text-xs'>{job.message}</p>
                    {job.status === 'running' ? (
                      <span class='inline-flex items-center gap-2 text-accent-light text-xs'>
                        <span class='h-2 w-2 animate-pulse bg-accent-light' />
                        Running
                      </span>
                    ) : null}
                    <div class='h-1 bg-bg-page'>
                      <div class='h-full bg-text-primary' style={{ width: `${job.progress}%` }} />
                    </div>
                  </article>
                ))}
              </div>
            </article>
          )
        )}
      </div>
    ) : (
      <div class='flex min-h-40 items-center justify-center border border-border-base bg-bg-card p-5 text-text-muted'>
        No jobs yet.
      </div>
    )}
  </section>
);
