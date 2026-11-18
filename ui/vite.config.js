import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Front-end do pi-deck. O app roda dentro do webview do Tauri (production) ou no
// navegador puro (dev sem IPC). Porta fixa: o tauri.conf.json aponta para ela.
export default defineConfig({
  plugins: [svelte()],
  server: { port: 1420, strictPort: true },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
  },
});
