import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'

// Port fixe attendu par src-tauri/tauri.conf.json (devUrl).
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: 'es2022' },
  test: { environment: 'node', include: ['src/**/*.test.ts'] },
})
