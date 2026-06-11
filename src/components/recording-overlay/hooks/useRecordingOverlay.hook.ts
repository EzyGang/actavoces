import { useSignal } from '@preact/signals';
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'preact/hooks';
import { getAppSnapshot, stopRecording } from '../../../services/desktop/app.service';
import { appErrorSignal } from '../../../stores/app.store';
import type { AppSettings, AppSnapshot } from '../../../types/desktop';

const initialDisplayMode = (): AppSettings['overlayDisplayMode'] => {
  if (typeof window !== 'undefined' && window.innerWidth <= 80) {
    return 'minimal';
  }

  return 'full';
};

export const useRecordingOverlay = () => {
  const stopping = useSignal(false);
  const displayMode = useSignal<AppSettings['overlayDisplayMode']>(initialDisplayMode());

  const handleStopRecording = async () => {
    stopping.value = true;
    appErrorSignal.value = null;

    try {
      await stopRecording();
    } catch (error) {
      appErrorSignal.value = error instanceof Error ? error.message : 'Unable to stop recording';
      stopping.value = false;
    }
  };

  useEffect(() => {
    if (!('__TAURI_INTERNALS__' in window)) {
      return;
    }

    void getAppSnapshot()
      .then((snapshot) => {
        displayMode.value = snapshot.settings.overlayDisplayMode;
      })
      .catch(() => undefined);

    const snapshotListener = listen<AppSnapshot>('app-snapshot-updated', (event) => {
      displayMode.value = event.payload.settings.overlayDisplayMode;
    });

    return () => {
      void snapshotListener.then((unlisten) => unlisten());
    };
  }, []);

  return {
    status: {
      displayMode,
      stopping
    },
    actions: {
      stopRecording: handleStopRecording
    }
  };
};
