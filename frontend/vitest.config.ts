import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    include: ['src/**/*.{test,spec}.{ts,mts,cts,js,jsx,tsx}'],
    environment: 'jsdom',
    root: './',
  },
})
