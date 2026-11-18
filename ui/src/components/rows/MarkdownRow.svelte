<script>
  import { flashCopy } from '../../lib/copy.js';
  import { S } from '../../lib/state.svelte.js';

  // row: { kind:'md', raw, html, forkText, forkMsg, forkIndex }
  export let row;

  // The markdown is pre-rendered to escaped HTML (lib/markdown.js). Two kinds
  // of interactive elements live inside it and are handled by delegation:
  //  - a[data-ext]: external links → open_external, the webview never navigates
  //  - .code-copy:  copies the block's pre text read from the DOM at click time
  function onMdClick(e) {
    const link = e.target.closest('a[data-ext]');
    if (link) {
      e.preventDefault();
      S.openExternal(link.dataset.ext);
      return;
    }
    const copy = e.target.closest('.code-copy');
    if (copy) {
      e.preventDefault();
      const box = copy.closest('.md-code-box');
      const pre = box && box.querySelector('pre');
      flashCopy(copy, pre ? pre.textContent : '');
    }
  }
</script>

<div class="row md" onclick={onMdClick}>
  <div class="md-body">
    {@html row.html}
  </div>
  <button class="copybtn" type="button" title="Copy this message"
          onclick={(e) => flashCopy(e.currentTarget, row.raw)}>copy</button>
  {#if row.forkText}
    <button class="tiny forkrow" title={row.forkText} onclick={(e) => { e.stopPropagation(); S.forkRowClick(row); }}>fork</button>
  {/if}
</div>
