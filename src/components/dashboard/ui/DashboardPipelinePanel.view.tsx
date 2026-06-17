import { IconFolderOpen, IconRefresh } from '@tabler/icons-react';
import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { Panel } from '../../shared/ui/Panel.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import type { DashboardViewModel } from '../hooks/useDashboard.hook';
import { DashboardPipelineStageCard } from './DashboardPipelineStageCard.view';

interface DashboardPipelinePanelProps {
  loading: boolean;
  pipeline: DashboardViewModel['data']['pipeline'];
}

export const DashboardPipelinePanel = ({
  loading,
  pipeline
}: DashboardPipelinePanelProps): JSX.Element => (
  <Panel gap='lg'>
    <div class='flex items-center justify-between gap-4'>
      <div class='flex flex-col gap-1'>
        <h1 class='font-semibold text-2xl'>Current pipeline</h1>
        <p class='text-sm text-text-muted'>Processing starts automatically after capture stops.</p>
      </div>
      {pipeline.status ? (
        <div class='flex shrink-0 items-center gap-2'>
          <StatusBadge label={pipeline.status.label} status={pipeline.status.status} />
          {pipeline.actions && pipeline.status.status === 'complete' ? (
            <Button
              aria-label='Open recording folder'
              class='h-9 w-9 p-0!'
              disabled={loading}
              onClick={pipeline.actions.onOpenFolder}
              title='Open recording folder'
              variant='ghost'
            >
              <IconFolderOpen aria-hidden='true' className='h-4 w-4' />
            </Button>
          ) : null}
          {pipeline.actions?.canRetry ? (
            <Button
              aria-label='Retry failed jobs'
              class='h-9 w-9 p-0!'
              disabled={loading}
              onClick={pipeline.actions.onRetry}
              title='Retry failed jobs'
              variant='ghost'
            >
              <IconRefresh aria-hidden='true' className='h-4 w-4' />
            </Button>
          ) : null}
        </div>
      ) : null}
    </div>

    {pipeline.recording ? (
      <div class='flex flex-col gap-5'>
        <div class='flex flex-col gap-2'>
          <div class='h-2 bg-bg-page'>
            <div
              class='h-full bg-text-primary transition-all duration-slow'
              style={{ width: `${pipeline.progress}%` }}
            />
          </div>
          <div class='flex items-center justify-between gap-3 font-mono text-text-muted text-xs'>
            <span>{pipeline.status?.message}</span>
            <span>{pipeline.progress}%</span>
          </div>
        </div>
        <div class='grid gap-3 md:grid-cols-4'>
          {pipeline.recording.stages.map((stage, index) => (
            <DashboardPipelineStageCard
              actions={pipeline.actions}
              index={index}
              key={stage.id}
              loading={loading}
              stage={stage}
            />
          ))}
        </div>
      </div>
    ) : (
      <div class='flex min-h-40 items-center justify-center border border-border-base bg-bg-input p-5 text-text-muted'>
        No pipeline has started.
      </div>
    )}
  </Panel>
);
