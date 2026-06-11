import { getCurrentWindow } from '@tauri-apps/api/window';
import { render } from 'preact';
import App from './App';

const root = document.getElementById('root');
const currentWindow = getCurrentWindow();

if (root === null) {
  throw new Error('Root element not found');
}

if (currentWindow.label === 'main') {
  await currentWindow.show();
}

render(<App />, root);
