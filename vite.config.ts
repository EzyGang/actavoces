import preact from '@preact/preset-vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig, type Plugin } from 'vite';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

const githubVersionPlugin = (): Plugin => {
  let version = '0.0.0';

  return {
    name: 'github-version',
    async config() {
      try {
        const response = await fetch(
          'https://api.github.com/repos/EzyGang/actavoces/releases/latest'
        );
        if (response.ok) {
          const data: { tag_name: string } = await response.json();
          version = data.tag_name.replace(/^v/, '');
        }
      } catch {
        // fallback to 0.0.0
      }

      return {
        define: {
          LANDING_APP_VERSION: JSON.stringify(version)
        }
      };
    }
  };
};

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [tailwindcss(), preact(), githubVersionPlugin()],

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
}));
