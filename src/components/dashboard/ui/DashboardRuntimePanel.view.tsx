import { clsx } from 'clsx';
import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { Panel } from '../../shared/ui/Panel.view';
import type { DashboardViewModel } from '../hooks/useDashboard.hook';

interface DashboardRuntimePanelProps {
  loading: boolean;
  onCheckWorker: () => void;
  runtime: DashboardViewModel['data']['runtime'];
}

export const DashboardRuntimePanel = ({
  loading,
  onCheckWorker,
  runtime
}: DashboardRuntimePanelProps): JSX.Element => (
  <Panel>
    <div class='flex items-center justify-between gap-4'>
      <h2 class='font-semibold text-xl'>Runtime</h2>
      <Button class='h-9 px-3' disabled={loading} onClick={onCheckWorker} variant='ghost'>
        Check worker
      </Button>
    </div>
    <div class='flex flex-col gap-3 text-sm'>
      {runtime.rows.map((row) => (
        <div
          class={clsx(
            'flex justify-between gap-4',
            row.border === false ? null : 'border-border-base border-b pb-3'
          )}
          key={row.label}
        >
          <span class='text-text-muted'>{row.label}</span>
          <span class={row.class}>{row.value}</span>
        </div>
      ))}
      {runtime.errors.map((error) => (
        <div
          class='border border-warning-border bg-warning-bg p-3 text-warning text-xs'
          key={error}
        >
          {error}
        </div>
      ))}
    </div>
  </Panel>
);
