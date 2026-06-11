import type { AppSettingsUpdate } from '../types/desktop';

export const validateSettingsDraft = (
  settings: AppSettingsUpdate,
  _providerApiKeyConfigured = false,
  cudaAvailable = false
): string[] => {
  const errors: string[] = [];

  if (settings.outputDirectory.trim().length === 0) {
    errors.push('Output directory is required.');
  }

  if (settings.hotkey.trim().length === 0) {
    errors.push('Global hotkey is required.');
  }

  if (settings.sampleRate <= 0) {
    errors.push('Sample rate must be greater than zero.');
  }

  if (settings.computeType === 'cuda' && !cudaAvailable) {
    errors.push(
      'CUDA runtime is not ready. Install CUDA drivers, cuBLAS for CUDA 12, and cuDNN 9 for CUDA 12.'
    );
  }

  if (settings.summaryEnabled) {
    if (settings.providerBaseUrl.trim().length === 0) {
      errors.push('Provider base URL is required when summaries are enabled.');
    }

    if (settings.providerModel.trim().length === 0) {
      errors.push('Provider model is required when summaries are enabled.');
    }
  }

  if (settings.speakerCountMode === 'exact' && (settings.exactSpeakers ?? 0) <= 0) {
    errors.push('Exact speaker count must be greater than zero.');
  }

  if (settings.speakerCountMode === 'range') {
    const minSpeakers = settings.minSpeakers ?? 0;
    const maxSpeakers = settings.maxSpeakers ?? 0;

    if (minSpeakers <= 0 || maxSpeakers < minSpeakers) {
      errors.push('Speaker range must include a valid minimum and maximum.');
    }
  }

  return errors;
};
