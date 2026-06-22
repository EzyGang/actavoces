import type { JSX } from 'preact';
import { useLandingVersion } from '../hooks/useLandingVersion.hook';
import { LandingPage } from '../LandingPage.view';

export const LandingPageContainer = (): JSX.Element => (
  <LandingPage landing={useLandingVersion()} />
);
