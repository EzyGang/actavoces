import preact from '@preact/preset-vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

const fetchLatestTag = async (): Promise<string> => {
  try {
    const response = await fetch(
      'https://api.github.com/repos/EzyGang/actavoces/releases/latest'
    );
    if (response.ok) {
      const data = await response.json();
      return data.tag_name;
    }
  } catch {
    // network or parse failure
  }
  return 'v0.0.0';
};

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  const latestTag = mode === 'landing' ? await fetchLatestTag() : 'v0.0.0';

  return {
    plugins: [tailwindcss(), preact()],

    define: {
      __APP_VERSION__: JSON.stringify(latestTag)
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      hmr: host
        ? {
            protocol: 'ws',
            host,
            port: 1421
          }
        : undefined,
      watch: {
        // 3. tell Vite to ignore watching `src-tauri`
        ignored: ['**/src-tauri/**']
      }
    },
    resolve: {
      alias: {
        react: 'preact/compat',
        'react-dom': 'preact/compat'
      }
    },
    build: mode === 'landing'
      ? {
          outDir: 'dist-landing',
          rollupOptions: {
            input: 'landing.html'
          }
        }
      : undefined
  };
});
