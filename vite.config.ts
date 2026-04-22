import { fileURLToPath, URL } from 'node:url';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vite';

const SRC_WEB_PATH = fileURLToPath(new URL('./src-web', import.meta.url));

export default defineConfig({
  root: SRC_WEB_PATH,
  plugins: [vue()],
  resolve: {
    alias: {
      '@': SRC_WEB_PATH,
    },
  },
  server: {
    strictPort: true,
    host: process.env.TAURI_DEV_HOST || false,
    hmr: process.env.TAURI_DEV_HOST
      ? {
          protocol: 'ws',
          host: process.env.TAURI_DEV_HOST,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  clearScreen: false,
  envPrefix: [
    'VITE_',
    'TAURI_PLATFORM',
    'TAURI_ARCH',
    'TAURI_FAMILY',
    'TAURI_PLATFORM_VERSION',
    'TAURI_PLATFORM_TYPE',
    'TAURI_DEBUG',
  ],
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    minify: process.env.TAURI_DEBUG ? false : 'esbuild',
    sourcemap: Boolean(process.env.TAURI_DEBUG),
  },
});
