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

const effectiveDisplayMode = (snapshot: AppSnapshot): AppSettings['overlayDisplayMode'] =>
  snapshot.desktop.overlayVisible ? snapshot.settings.overlayDisplayMode : 'none';

export const useRecordingOverlay = () => {
  const stopping = useSignal(false);
  const displayMode = useSignal<AppSettings['overlayDisplayMode']>('full');
  const overlaySyncReceived = useSignal(false);

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
        if (!overlaySyncReceived.value) {
          displayMode.value = effectiveDisplayMode(snapshot);
        }
      })
      .catch(() => undefined);

    const snapshotListener = listen<AppSnapshot>('app-snapshot-updated', (event) => {
      displayMode.value = effectiveDisplayMode(event.payload);
    });
    const overlaySyncListener = listen<RecordingOverlaySyncPayload>(
      'recording-overlay-sync',
      (event) => {
        overlaySyncReceived.value = true;
        displayMode.value = event.payload.visible ? event.payload.displayMode : 'none';

        if (!event.payload.visible) {
          stopping.value = false;
        }
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
