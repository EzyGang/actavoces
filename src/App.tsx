import './App.css';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { AppContainer } from './components/app-shell/containers/App.container';
import { RecordingOverlayContainer } from './components/recording-overlay/containers/RecordingOverlay.container';

const currentWindowLabel = () => {
  if (!('__TAURI_INTERNALS__' in window)) {
    return 'main';
  }

  return getCurrentWindow().label;
};

const App = () =>
  currentWindowLabel() === 'recording-overlay' ? <RecordingOverlayContainer /> : <AppContainer />;

export default App;
