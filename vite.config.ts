import { execSync } from 'node:child_process';
import preact from '@preact/preset-vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig, type Plugin } from 'vite';

const host = process.env.TAURI_DEV_HOST;

async function fetchLatestVersion(): Promise<string> {
  try {
    const response = await fetch(
      'https://api.github.com/repos/EzyGang/actavoces/releases/latest'
    );
    if (response.ok) {
      const data = (await response.json()) as { tag_name: string };
      return data.tag_name ?? '0.0.0';
    }
  } catch {
    // fall through to git tag
  }

  try {
    return execSync('git describe --tags --abbrev=0', {
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'ignore']
    }).trim();
  } catch {
    return '0.0.0';
  }
}

function landingVersionPlugin(): Plugin {
  let version = '0.0.0';
  return {
    name: 'landing-version',
    async buildStart() {
      version = await fetchLatestVersion();
    },
    resolveId(id) {
      if (id === 'virtual:landing-version') return '\0virtual:landing-version';
    },
    load(id) {
      if (id === '\0virtual:landing-version') {
        return `export const latestVersion = ${JSON.stringify(version)};`;
      }
    }
  };
}

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [
    tailwindcss(),
    preact(),
    mode === 'landing' && landingVersionPlugin()
  ],

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
