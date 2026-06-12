import { useSignal } from '@preact/signals';
import { listen } from '@tauri-apps/api/event';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { useEffect } from 'preact/hooks';
import { errorMessage, isTauriRuntime } from '../../app-shell/hooks/appRuntime.helpers';

interface UseUpdatesInput {
  setError: (message: string | null) => void;
}

export const useUpdates = ({ setError }: UseUpdatesInput) => {
  const updateChecking = useSignal(false);
  const updateInstalling = useSignal(false);
  const updateAvailable = useSignal<Update | null>(null);
  const updateStatus = useSignal('Updates have not been checked in this session.');

  const checkForUpdates = async () => {
    if (!isTauriRuntime()) {
      updateStatus.value = 'Updater is available in the desktop app.';

      return;
    }

    updateChecking.value = true;
    setError(null);

    try {
      const update = await check();

      updateAvailable.value = update;
      updateStatus.value = update
        ? `Version ${update.version} is available.`
        : 'ActaVoces is up to date.';
    } catch (error) {
      const message = errorMessage(error, 'Unable to check for updates');

      setError(message);
      updateStatus.value = message;
    } finally {
      updateChecking.value = false;
    }
  };

  const installUpdate = async () => {
    if (!isTauriRuntime()) {
      updateStatus.value = 'Updater is available in the desktop app.';

      return;
    }

    updateInstalling.value = true;
    setError(null);

    try {
      const update = updateAvailable.value ?? (await check());

      if (!update) {
        updateAvailable.value = null;
        updateStatus.value = 'ActaVoces is up to date.';

        return;
      }

      updateStatus.value = `Installing version ${update.version}.`;
      await update.downloadAndInstall();
      updateStatus.value = 'Update installed. Relaunching ActaVoces.';
      await relaunch();
    } catch (error) {
      const message = errorMessage(error, 'Unable to install update');

      setError(message);
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

  return {
    updateChecking,
    updateInstalling,
    updateAvailable,
    updateStatus,
    actions: {
      checkForUpdates,
      installUpdate
    }
  };
};
