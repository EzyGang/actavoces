import preact from '@preact/preset-vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  let currentVersionTag: string | null = null;

  if (mode === 'landing') {
    try {
      const response = await fetch(
        'https://api.github.com/repos/EzyGang/actavoces/tags',
        { headers: { Accept: 'application/vnd.github+json' }, signal: AbortSignal.timeout(5000) }
      );
      if (response.ok) {
        const tags: Array<{ name: string }> = await response.json();
        if (tags.length > 0) {
          currentVersionTag = tags[0].name;
        }
      }
    } catch {
      // fall back to null if API unreachable
    }
  }

  return {
    plugins: [tailwindcss(), preact()],

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
    define: {
      __ACTAVOCES_VERSION__: JSON.stringify(currentVersionTag)
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
