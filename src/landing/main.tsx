import { render } from 'preact';
import { LandingPageContainer } from './containers/LandingPage.container';

const root = document.getElementById('root');

if (root === null) {
  throw new Error('Root element not found');
}

render(<LandingPageContainer />, root);
