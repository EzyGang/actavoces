import type { JSX } from 'preact';
import { Panel } from '../../shared/ui/Panel.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import type { DashboardViewModel } from '../hooks/useDashboard.hook';

interface DashboardMetricGridProps {
  metrics: DashboardViewModel['data']['metrics'];
}

export const DashboardMetricGrid = ({ metrics }: DashboardMetricGridProps): JSX.Element => (
  <section class='grid gap-4 lg:grid-cols-4'>
    {metrics.map((metric) => (
      <Panel class='min-h-32 justify-between' gap='none' key={metric.label}>
        <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
          {metric.label}
        </span>
        <div class='flex items-end justify-between gap-3'>
          <p class='font-semibold text-3xl'>{metric.value}</p>
          {metric.badge ? (
            <StatusBadge label={metric.badge.label} status={metric.badge.status} />
          ) : null}
        </div>
      </Panel>
    ))}
  </section>
);
