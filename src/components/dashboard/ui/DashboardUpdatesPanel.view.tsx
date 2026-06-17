import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { Panel } from '../../shared/ui/Panel.view';
import type { DashboardViewModel } from '../hooks/useDashboard.hook';

interface DashboardUpdatesPanelProps {
  actions: DashboardViewModel['actions'];
  notice: DashboardViewModel['data']['updateNotice'];
  status: DashboardViewModel['status'];
}

export const DashboardUpdatesPanel = ({
  actions,
  notice,
  status
}: DashboardUpdatesPanelProps): JSX.Element => (
  <Panel>
    <div class='flex items-center justify-between gap-4'>
      <div class='flex flex-col gap-1'>
        <h2 class='font-semibold text-xl'>Updates</h2>
        <p class='text-sm text-text-muted'>{notice.status}</p>
      </div>
      <div class='flex gap-2'>
        <Button
          class='h-9 px-3'
          disabled={status.updateChecking || status.updateInstalling}
          onClick={actions.checkForUpdates}
          variant='ghost'
        >
          Check
        </Button>
        <Button
          class='h-9 px-3'
          disabled={status.updateInstalling || !notice.updateAvailable}
          onClick={actions.installUpdate}
          variant='secondary'
        >
          Install
        </Button>
      </div>
    </div>
  </Panel>
);
