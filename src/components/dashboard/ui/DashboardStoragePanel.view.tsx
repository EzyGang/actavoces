import type { JSX } from 'preact';
import { Panel } from '../../shared/ui/Panel.view';
import type { DashboardViewModel } from '../hooks/useDashboard.hook';

interface DashboardStoragePanelProps {
  storage: DashboardViewModel['data']['storage'];
}

export const DashboardStoragePanel = ({ storage }: DashboardStoragePanelProps): JSX.Element => (
  <Panel>
    <h2 class='font-semibold text-xl'>Storage</h2>
    <div class='grid gap-3 text-sm lg:grid-cols-3'>
      <div class='flex flex-col gap-1 border-border-base border-b pb-3 lg:border-r lg:border-b-0 lg:pr-3 lg:pb-0'>
        <span class='text-text-muted'>Records folder</span>
        <span class='wrap-break-word font-mono text-xs'>{storage.outputDirectory}</span>
      </div>
      <div class='flex flex-col gap-1 border-border-base border-b pb-3 lg:border-r lg:border-b-0 lg:pr-3 lg:pb-0'>
        <span class='text-text-muted'>Database</span>
        <span class='wrap-break-word font-mono text-xs'>{storage.databasePath}</span>
      </div>
      <div class='flex flex-col gap-1'>
        <span class='text-text-muted'>Model folder</span>
        <span class='wrap-break-word font-mono text-xs'>{storage.modelStorageDirectory}</span>
      </div>
    </div>
  </Panel>
);
