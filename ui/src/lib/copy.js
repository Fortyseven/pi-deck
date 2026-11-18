// Svelte action: turns a <button> into a "copy to clipboard" control.
// The copied label flashes for 1.2 s (original behavior, English copy).
import { inv } from './tauri.js';

/**
 * Flash a copy button with the outcome of a copy ("copied" / "failed") and
 * restore the original label after 1.2 s. Returns a promise that resolves to
 * the copy outcome so callers can branch on it.
 */
export async function flashCopy(node, text) {
  const label = node.textContent;
  let ok = false;
  try {
    await inv('copy_text', { text });
    ok = true;
  } catch { /* not on the clipboard */ }
  node.textContent = ok ? 'copied' : 'failed';
  node.classList.toggle('ok', ok);
  setTimeout(() => {
    node.textContent = label;
    node.classList.remove('ok');
  }, 1200);
  return ok;
}

export function copyBtn(node, [text]) {
  const onClick = async (e) => {
    e.stopPropagation();
    if (!text) return;
    await flashCopy(node, text);
  };
  node.addEventListener('click', onClick);
  return {
    destroy() {
      node.removeEventListener('click', onClick);
    },
  };
}
