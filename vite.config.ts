import preact from '@preact/preset-vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

async function fetchLatestTag(): Promise<string> {
  try {
    const response = await fetch(
      'https://api.github.com/repos/EzyGang/actavoces/releases/latest'
    );
    if (response.ok) {
      const data: { tag_name: string } = await response.json();
      return data.tag_name;
    }
    const tagsResponse = await fetch(
      'https://api.github.com/repos/EzyGang/actavoces/tags?per_page=1'
    );
    if (tagsResponse.ok) {
      const tags: Array<{ name: string }> = await tagsResponse.json();
      if (tags.length > 0) return tags[0].name;
    }
  } catch {
    // Network or parse failure — fall through to default
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
      ? { __APP_VERSION__: JSON.stringify(await fetchLatestTag()) }
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
