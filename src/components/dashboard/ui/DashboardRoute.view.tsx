import type { JSX } from 'preact';
import type { DashboardViewModel } from '../hooks/useDashboard.hook';
import { DashboardDiarizationWarning } from './DashboardDiarizationWarning.view';
import { DashboardGlossaryPanel } from './DashboardGlossaryPanel.view';
import { DashboardMetricGrid } from './DashboardMetricGrid.view';
import { DashboardPipelinePanel } from './DashboardPipelinePanel.view';
import { DashboardRuntimePanel } from './DashboardRuntimePanel.view';
import { DashboardStoragePanel } from './DashboardStoragePanel.view';
import { DashboardUpdatesPanel } from './DashboardUpdatesPanel.view';

interface DashboardRouteProps {
  dashboard: DashboardViewModel;
}

export const DashboardRoute = ({ dashboard }: DashboardRouteProps): JSX.Element => (
  <div class='flex flex-col gap-5'>
    <DashboardMetricGrid metrics={dashboard.data.metrics} />

    {dashboard.data.updateNotice.visible ? (
      <DashboardUpdatesPanel
        actions={dashboard.actions}
        notice={dashboard.data.updateNotice}
        status={dashboard.status}
      />
    ) : null}

    {dashboard.data.showDiarizationWarning ? <DashboardDiarizationWarning /> : null}

    <section class='grid gap-4'>
      <DashboardPipelinePanel
        loading={dashboard.status.loading}
        pipeline={dashboard.data.pipeline}
      />

      <div class='grid gap-4 lg:grid-cols-2'>
        <DashboardRuntimePanel
          loading={dashboard.status.loading}
          onCheckWorker={dashboard.actions.checkWorker}
          runtime={dashboard.data.runtime}
        />
        <DashboardGlossaryPanel
          field={dashboard.data.glossaryField}
          saving={dashboard.status.savingSettings}
        />
      </div>

      <DashboardStoragePanel storage={dashboard.data.storage} />
    </section>
  </div>
);
