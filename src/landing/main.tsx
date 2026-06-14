import { render } from 'preact';
import { LandingPage } from './LandingPage.view';

const root = document.getElementById('root');

if (root === null) {
  throw new Error('Root element not found');
}

render(<LandingPage />, root);
