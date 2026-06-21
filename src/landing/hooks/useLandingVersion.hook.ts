import { useSignal } from '@preact/signals';
import { useEffect } from 'preact/hooks';

export const useLandingVersion = () => {
  const version = useSignal<string | null>(null);
  const loading = useSignal<boolean>(true);
  const error = useSignal<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();

    const fetchVersion = async () => {
      try {
        const response = await fetch(
          'https://api.github.com/repos/EzyGang/actavoces/releases/latest',
          { signal: controller.signal }
        );

        if (!response.ok) {
          throw new Error(`GitHub API error: ${response.status}`);
        }

        const data = await response.json();
        version.value = data.tag_name ?? null;
      } catch (e) {
        if (e instanceof DOMException && e.name === 'AbortError') {
          return;
        }

        error.value = e instanceof Error ? e.message : 'Unknown error';
      } finally {
        loading.value = false;
      }
    };

    fetchVersion();

    return () => {
      controller.abort();
    };
  }, []);

  return {
    get version() {
      return version.value;
    },
    get loading() {
      return loading.value;
    },
    get error() {
      return error.value;
    }
  };
};
