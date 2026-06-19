import { execSync } from 'child_process';
import preact from '@preact/preset-vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

/** Fetch the latest GitHub tag from the local git repository. */
function getLatestTag(): string {
  try {
    const tag = execSync('git describe --tags --abbrev=0', {
      encoding: 'utf-8',
      timeout: 5000
    }).trim();
    return tag || 'latest';
  } catch {
    return 'latest';
  }
}

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
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
  define: mode === 'landing'
    ? { __LATEST_VERSION__: JSON.stringify(getLatestTag()) }
    : undefined,
  build: mode === 'landing'
    ? {
        outDir: 'dist-landing',
        rollupOptions: {
          input: 'landing.html'
        }
      }
    : undefined
}));
