import './App.css';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { AppContainer } from './components/app/containers/App.container';
import { RecordingOverlayContainer } from './components/app/containers/RecordingOverlay.container';

const currentWindowLabel = () => {
  if (!('__TAURI_INTERNALS__' in window)) {
    return 'main';
  }

  return getCurrentWindow().label;
};

const App = () =>
  currentWindowLabel() === 'recording-overlay' ? <RecordingOverlayContainer /> : <AppContainer />;

export default App;
