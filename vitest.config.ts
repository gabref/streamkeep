import { fileURLToPath, URL } from 'node:url';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vitest/config';

const SRC_WEB_PATH = fileURLToPath(new URL('./src-web', import.meta.url));

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': SRC_WEB_PATH,
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['src-web/**/*.test.ts', 'tests/**/*.ts'],
  },
});

