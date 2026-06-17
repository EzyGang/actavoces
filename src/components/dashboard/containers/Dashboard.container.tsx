import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { useDashboard } from '../hooks/useDashboard.hook';
import { DashboardRoute } from '../ui/DashboardRoute.view';

interface DashboardContainerProps {
  app: ReturnType<typeof useApp>;
}

export const DashboardContainer = ({ app }: DashboardContainerProps): JSX.Element => (
  <DashboardRoute dashboard={useDashboard(app)} />
);
