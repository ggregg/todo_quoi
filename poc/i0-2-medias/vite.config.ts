import { defineConfig } from 'vite'

// Port fixe attendu par `src-tauri/tauri.conf.json` (devUrl).
export default defineConfig({
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: 'es2022' },
})
