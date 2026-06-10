import type { Signal } from '@preact/signals';
import { appSnapshotSignal } from '../../../stores/app.store';
import {
  type AppRoute,
  activeRouteSignal,
  navigationItems,
  setActiveRoute
} from '../../../stores/route.store';
import type { AppSettings } from '../../../types/desktop';

interface UseAppNavigationInput {
  hasUnsavedSettings: Signal<boolean>;
  resetSettingsDraft: (settings: AppSettings) => void;
}

export const useAppNavigation = ({
  hasUnsavedSettings,
  resetSettingsDraft
}: UseAppNavigationInput) => {
  const selectRoute = (route: AppRoute) => {
    if (
      activeRouteSignal.value === 'settings' &&
      route !== 'settings' &&
      hasUnsavedSettings.value
    ) {
      const shouldLeave = window.confirm('Discard unsaved settings changes?');

      if (!shouldLeave) {
        return;
      }

      resetSettingsDraft(appSnapshotSignal.value.settings);
    }

    setActiveRoute(route);
  };

  return navigationItems.map((item) => ({
    ...item,
    isActive: item.route === activeRouteSignal.value,
    onSelect: () => selectRoute(item.route)
  }));
};
