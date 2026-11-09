import { readFileSync } from 'node:fs';
import { defineConfig } from 'vite';

export default defineConfig({
  clearScreen: false,
  server: {
    // Jay-C 5173, Jay-A 5175, Jay-H 5176, Jay-E 5177: all of them can run in dev at once.
    port: 5178,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  // The app's version (Settings › About), bumped with each release.
  define: { __APP_VERSION__: JSON.stringify(JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8')).version) },
  build: {
    // WebView2 (Chromium) on Windows.
    target: ['es2022', 'chrome110'],
    minify: process.env.TAURI_ENV_DEBUG ? false : undefined,
    sourcemap: Boolean(process.env.TAURI_ENV_DEBUG),
  },
});
