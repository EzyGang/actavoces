import { useSignal } from '@preact/signals';
import { stopRecording } from '../../../services/desktop/app.service';
import { appErrorSignal } from '../../../stores/app.store';

export const useRecordingOverlay = () => {
  const stopping = useSignal(false);

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

  return {
    status: {
      stopping
    },
    actions: {
      stopRecording: handleStopRecording
    }
  };
};
