/// <reference types="vitest/config" />

import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

const devProxyTarget =
  process.env.VITE_DEV_PROXY_TARGET?.trim() || 'http://127.0.0.1:8080';

export default defineConfig({
  plugins: [vue()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
  server: {
    host: '0.0.0.0',
    port: 5173,
    proxy: {
      '/api': devProxyTarget,
      '/health': devProxyTarget,
      '/images': devProxyTarget,
      '/thumbnails': devProxyTarget,
      '/favicon.ico': devProxyTarget,
    },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.spec.ts'],
  },
});
