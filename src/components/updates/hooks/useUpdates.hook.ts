import { type Signal, useComputed, useSignal } from '@preact/signals';
import { listen } from '@tauri-apps/api/event';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { useEffect } from 'preact/hooks';
import { errorMessage, isTauriRuntime } from '../../app-shell/hooks/appRuntime.helpers';

interface UseUpdatesInput {
  loading: Signal<boolean>;
  setError: (message: string | null) => void;
  setupReady: Signal<boolean>;
}

type UpdateCheckStatus =
  | 'notChecked'
  | 'checking'
  | 'available'
  | 'current'
  | 'failed'
  | 'installing'
  | 'installed';

interface UpdateToast {
  message: string;
}

export const useUpdates = ({ loading, setError, setupReady }: UseUpdatesInput) => {
  const updateChecking = useSignal(false);
  const updateInstalling = useSignal(false);
  const updateAvailable = useSignal<Update | null>(null);
  const updateCheckStatus = useSignal<UpdateCheckStatus>('notChecked');
  const updateStatus = useSignal('Updates have not been checked in this session.');
  const updateToast = useSignal<UpdateToast | null>(null);
  const initialCheckRequested = useSignal(false);
  const updateNoticeVisible = useComputed(() => updateCheckStatus.value !== 'current');

  const runUpdateCheck = async (notify: boolean) => {
    if (!isTauriRuntime()) {
      updateCheckStatus.value = 'failed';
      updateStatus.value = 'Updater is available in the desktop app.';

      return;
    }

    updateChecking.value = true;
    updateCheckStatus.value = 'checking';
    setError(null);

    try {
      const update = await check();

      updateAvailable.value = update;
      updateCheckStatus.value = update ? 'available' : 'current';
      updateStatus.value = update
        ? `Version ${update.version} is available.`
        : 'ActaVoces is up to date.';

      if (notify) {
        updateToast.value = { message: updateStatus.value };
      }
    } catch (error) {
      const message = errorMessage(error, 'Unable to check for updates');

      setError(message);
      updateCheckStatus.value = 'failed';
      updateStatus.value = message;
    } finally {
      updateChecking.value = false;
    }
  };

  const checkForUpdates = async () => {
    await runUpdateCheck(true);
  };

  const dismissUpdateToast = () => {
    updateToast.value = null;
  };

  const installUpdate = async () => {
    if (!isTauriRuntime()) {
      updateCheckStatus.value = 'failed';
      updateStatus.value = 'Updater is available in the desktop app.';

      return;
    }

    updateInstalling.value = true;
    updateCheckStatus.value = 'installing';
    setError(null);

    try {
      const update = updateAvailable.value ?? (await check());

      if (!update) {
        updateAvailable.value = null;
        updateCheckStatus.value = 'current';
        updateStatus.value = 'ActaVoces is up to date.';

        return;
      }

      updateStatus.value = `Installing version ${update.version}.`;
      await update.downloadAndInstall();
      updateCheckStatus.value = 'installed';
      updateStatus.value = 'Update installed. Relaunching ActaVoces.';
      await relaunch();
    } catch (error) {
      const message = errorMessage(error, 'Unable to install update');

      setError(message);
      updateCheckStatus.value = 'failed';
      updateStatus.value = message;
    } finally {
      updateInstalling.value = false;
    }
  };

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    const checkForUpdatesListener = listen('check-for-updates-requested', () => {
      void checkForUpdates();
    });

    return () => {
      void checkForUpdatesListener.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime() || initialCheckRequested.value || !setupReady.value || loading.value) {
      return;
    }

    initialCheckRequested.value = true;
    window.requestAnimationFrame(() => {
      void runUpdateCheck(false);
    });
  }, [setupReady.value, loading.value]);

  return {
    updateChecking,
    updateInstalling,
    updateAvailable,
    updateCheckStatus,
    updateNoticeVisible,
    updateStatus,
    updateToast,
    actions: {
      checkForUpdates,
      dismissUpdateToast,
      installUpdate
    }
  };
};
