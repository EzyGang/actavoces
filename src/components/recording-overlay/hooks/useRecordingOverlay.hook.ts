import { useSignal } from '@preact/signals';
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'preact/hooks';
import { getAppSnapshot, stopRecording } from '../../../services/desktop/app.service';
import { appErrorSignal } from '../../../stores/app.store';
import type { AppSettings, AppSnapshot } from '../../../types/desktop';

interface RecordingOverlaySyncPayload {
  visible: boolean;
  displayMode: AppSettings['overlayDisplayMode'];
}

export const useRecordingOverlay = () => {
  const stopping = useSignal(false);
  const displayMode = useSignal<AppSettings['overlayDisplayMode']>('full');

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
    const overlaySyncListener = listen<RecordingOverlaySyncPayload>(
      'recording-overlay-sync',
      (event) => {
        displayMode.value = event.payload.visible ? event.payload.displayMode : 'none';
      }
    );

    return () => {
      void snapshotListener.then((unlisten) => unlisten());
      void overlaySyncListener.then((unlisten) => unlisten());
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
