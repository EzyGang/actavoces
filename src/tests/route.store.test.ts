import { describe, expect, it } from 'vitest';
import {
  activeRouteSignal,
  isActiveRoute,
  navigationItems,
  setActiveRoute
} from '../stores/route.store';

describe('route store', () => {
  it('tracks active navigation state', () => {
    setActiveRoute('dashboard');

    expect(isActiveRoute('dashboard')).toBe(true);
    expect(isActiveRoute('settings')).toBe(false);

    setActiveRoute('settings');

    expect(activeRouteSignal.value).toBe('settings');
    expect(isActiveRoute('settings')).toBe(true);
  });

  it('exposes all primary app routes', () => {
    expect(navigationItems.map((item) => item.route)).toEqual([
      'dashboard',
      'recordings',
      'jobs',
      'settings'
    ]);
  });
});
