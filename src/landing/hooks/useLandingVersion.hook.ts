import { useSignal } from '@preact/signals';
import { useEffect } from 'preact/hooks';

const GITHUB_API_URL = 'https://api.github.com/repos/EzyGang/actavoces/releases/latest';

export const useLandingVersion = () => {
  const version = useSignal<string | null>(null);
  const loading = useSignal(true);
  const error = useSignal<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const fetchVersion = async () => {
      loading.value = true;
      error.value = null;

      try {
        const response = await fetch(GITHUB_API_URL);

        if (!response.ok) {
          throw new Error(`GitHub API returned ${response.status}`);
        }

        const data = (await response.json()) as { tag_name: string };

        if (!cancelled) {
          version.value = data.tag_name;
        }
      } catch (err) {
        if (!cancelled) {
          error.value = err instanceof Error ? err.message : 'Failed to fetch version';
        }
      } finally {
        if (!cancelled) {
          loading.value = false;
        }
      }
    };

    void fetchVersion();

    return () => {
      cancelled = true;
    };
  }, []);

  return {
    status: {
      version,
      loading,
      error
    }
  };
};
