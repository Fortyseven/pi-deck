<script>
  import { S } from '../lib/state.svelte.js';
</script>

{#if S.fatalVisible}
  <div class="fatal" title="JavaScript error — click to dismiss">{S.fatalMsg}
    <button class="tiny" onclick={S.fatalDismiss}>dismiss</button>
  </div>
{/if}

{#if S.imageViewerOn}
  <div class="img-view" onclick={S.closeImage}>
    <img src={S.imageViewerSrc} alt="attached image"
         onclick={(e) => { e.stopPropagation(); S.closeImage(); }} />
  </div>
{/if}

<style lang="scss">
  .fatal {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 65;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 14px;
    background: #3a1518;
    border-top: 1px solid #6e2a30;
    color: #ffb3b8;
    font-family: var(--mono);
    font-size: 11px;
    button { margin-left: auto; flex: none; }
  }

  .img-view {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: rgba(6, 8, 12, 0.88);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: zoom-out;
    img {
      max-width: 92vw;
      max-height: 92vh;
      border-radius: 6px;
      border: 1px solid var(--line);
    }
  }
</style>
