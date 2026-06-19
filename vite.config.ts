import { execSync } from 'node:child_process';
import preact from '@preact/preset-vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

function getLatestTag(): string {
  try {
    return execSync('git describe --tags --abbrev=0').toString().trim();
  } catch {
    return '0.0.0';
  }
}

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [tailwindcss(), preact()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  define: {
    __LANDING_LATEST_TAG__: JSON.stringify(getLatestTag())
  },
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
}));
