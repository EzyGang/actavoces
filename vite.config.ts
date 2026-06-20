import preact from '@preact/preset-vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

async function fetchLatestVersion(): Promise<string> {
  try {
    const res = await fetch(
      'https://api.github.com/repos/EzyGang/actavoces/releases/latest'
    );
    if (res.ok) {
      const data = (await res.json()) as { tag_name: string };
      return data.tag_name;
    }
  } catch {
    // network or parse failure — fall through to unknown
  }
  return 'unknown';
}

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  const isLanding = mode === 'landing';

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
    define: isLanding
      ? { __APP_VERSION__: JSON.stringify(await fetchLatestVersion()) }
      : undefined,
    build: isLanding
      ? {
          outDir: 'dist-landing',
          rollupOptions: {
            input: 'landing.html'
          }
        }
      : undefined
  };
});
