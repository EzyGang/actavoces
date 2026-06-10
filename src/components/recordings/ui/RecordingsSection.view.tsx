import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';

interface RecordingsSectionProps {
  app: ReturnType<typeof useApp>;
}

export const RecordingsSection = ({ app }: RecordingsSectionProps): JSX.Element => (
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
          ({ recording, canRetry, onDelete, onOpenFolder, speakerRows, onRetry }) => (
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
                <span class='wrap-break-word font-mono text-text-muted text-xs'>
                  {recording.artifactDirectory}
                </span>
                {speakerRows.length > 0 ? (
                  <div class='flex flex-wrap gap-2 pt-2'>
                    {speakerRows.map((speaker) =>
                      speaker.isRenaming ? (
                        <form
                          class='flex items-center gap-2'
                          key={speaker.name}
                          onSubmit={speaker.onRenameSubmit}
                        >
                          <input
                            aria-label={`Rename ${speaker.name}`}
                            class='h-8 min-w-36 border border-border-focus bg-bg-input px-2 font-mono text-text-primary text-xs outline-none'
                            disabled={app.status.loading.value}
                            onInput={speaker.onRenameInput}
                            value={speaker.renameValue}
                          />
                          <Button
                            class='h-8 px-3'
                            disabled={app.status.loading.value}
                            type='submit'
                            variant='primary'
                          >
                            Save
                          </Button>
                          <Button
                            class='h-8 px-3'
                            disabled={app.status.loading.value}
                            onClick={speaker.onCancelRename}
                            variant='ghost'
                          >
                            Cancel
                          </Button>
                        </form>
                      ) : (
                        <button
                          class='border border-border-base bg-bg-input px-2 py-1 font-mono text-text-secondary text-xs hover:border-border-focus hover:text-text-primary'
                          disabled={app.status.loading.value}
                          key={speaker.name}
                          onClick={speaker.onStartRename}
                          title='Rename speaker'
                          type='button'
                        >
                          {speaker.name}
                        </button>
                      )
                    )}
                  </div>
                ) : null}
              </div>
              <div class='flex flex-col gap-3 lg:items-end'>
                <div class='flex flex-col gap-1 lg:items-end'>
                  <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
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
);
