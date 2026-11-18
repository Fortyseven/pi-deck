// Bridge to the Tauri backend.
// Inside the webview, Tauri injects `window.__TAURI__` (withGlobalTauri: true).
// In a plain browser (vite dev without Tauri) the fallbacks make the UI render
// while IPC calls fail fast with a readable error.
const tauriGlobal = window.__TAURI__ || {};

export const inv =
  (tauriGlobal.core && tauriGlobal.core.invoke) ||
  (async () => {
    throw new Error('window.__TAURI__.core.invoke unavailable');
  });

export const listen =
  (tauriGlobal.event && tauriGlobal.event.listen) ||
  (async () => () => {});
