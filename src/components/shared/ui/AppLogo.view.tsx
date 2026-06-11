import { clsx } from 'clsx';
import type { JSX } from 'preact';
import logoUrl from '../../../../src-tauri/icons/logo.svg';

interface AppLogoProps {
  class?: string;
}

export const AppLogo = ({ class: classProp }: AppLogoProps): JSX.Element => (
  <img
    alt='ActaVoces'
    class={clsx('block border border-border-base object-cover', classProp)}
    src={logoUrl}
  />
);
