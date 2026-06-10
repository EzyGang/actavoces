import type { JSX } from 'preact';
import { useRecordingOverlay } from '../hooks/useRecordingOverlay.hook';
import { RecordingOverlayView } from '../ui/RecordingOverlay.view';

export const RecordingOverlayContainer = (): JSX.Element => (
  <RecordingOverlayView overlay={useRecordingOverlay()} />
);
