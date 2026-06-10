import type { JSX } from 'preact';
import { useApp } from '../hooks/useApp.hook';
import { AppView } from '../ui/App.view';

export const AppContainer = (): JSX.Element => <AppView app={useApp()} />;
