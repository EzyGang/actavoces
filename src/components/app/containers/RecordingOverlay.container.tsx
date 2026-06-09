import type { JSX } from 'preact';
import { useApp } from '../hooks/useApp.hook';
import { RecordingOverlayView } from '../ui/RecordingOverlay.view';

export const RecordingOverlayContainer = (): JSX.Element => <RecordingOverlayView app={useApp()} />;
