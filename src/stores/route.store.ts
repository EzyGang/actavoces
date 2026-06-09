import { signal } from '@preact/signals';

export type AppRoute = 'dashboard' | 'recordings' | 'jobs' | 'settings';

export interface NavigationItem {
  route: AppRoute;
  label: string;
}

export const navigationItems: NavigationItem[] = [
  { route: 'dashboard', label: 'Dashboard' },
  { route: 'recordings', label: 'Recordings' },
  { route: 'jobs', label: 'Jobs' },
  { route: 'settings', label: 'Settings' }
];

export const activeRouteSignal = signal<AppRoute>('dashboard');

export const setActiveRoute = (route: AppRoute) => {
  activeRouteSignal.value = route;
};

export const isActiveRoute = (route: AppRoute) => activeRouteSignal.value === route;
