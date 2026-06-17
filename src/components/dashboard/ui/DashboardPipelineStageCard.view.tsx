import { IconRefresh } from '@tabler/icons-react';
import type { JSX } from 'preact';
import type { PipelineStage } from '../../../types/desktop';
import { Button } from '../../shared/ui/Button.view';
import { Panel } from '../../shared/ui/Panel.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import type { DashboardViewModel } from '../hooks/useDashboard.hook';

interface DashboardPipelineStageCardProps {
  actions: DashboardViewModel['data']['pipeline']['actions'];
  index: number;
  loading: boolean;
  stage: PipelineStage;
}

export const DashboardPipelineStageCard = ({
  actions,
  index,
  loading,
  stage
}: DashboardPipelineStageCardProps): JSX.Element => (
  <Panel class='min-h-36 justify-between' gap='md' padding='md' surface='input'>
    <div class='flex flex-col gap-3'>
      <div class='flex items-center justify-between gap-3'>
        <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
          {index + 1}
        </span>
        <div class='flex items-center gap-2'>
          {stage.id === 'summary' ? (
            <Button
              aria-label='Rerun summary'
              class='h-8 w-8 p-0!'
              disabled={loading || !actions?.canRerunSummary}
              onClick={actions?.onRerunSummary}
              title='Rerun summary'
              variant='ghost'
            >
              <IconRefresh aria-hidden='true' className='h-4 w-4' />
            </Button>
          ) : null}
          <StatusBadge label={stage.status} status={stage.status} />
        </div>
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
  </Panel>
);
