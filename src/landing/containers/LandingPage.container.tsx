import type { JSX } from 'preact';
import { useLandingVersion } from '../hooks/useLandingVersion.hook';
import { LandingPage } from '../LandingPage.view';

export const LandingPageContainer = (): JSX.Element => {
  const { version, loading, error } = useLandingVersion();

  return (
    <LandingPage
      landing={{
        data: { version },
        status: { loading, error },
        actions: {}
      }}
    />
  );
};
