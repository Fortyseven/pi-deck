<script>
  import { S } from '../lib/state.svelte.js';
</script>

{#if S.confirmBar}
  <div class="confirm-bar">
    <code class="confirm-cmd">{S.confirmCmd}</code>
    <span class="spacer"></span>
    <button class="primary" onclick={S.confirmOk}>Run</button>
    <button class="ghost" onclick={S.closeConfirm}>Cancel</button>
  </div>
{/if}

<style lang="scss">
  .confirm-bar {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 60;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 16px;
    border-top: 1px solid #2a3244;
    background: #11151e;
    .spacer { flex: 1; }
  }
  .confirm-cmd {
    font-family: var(--font-ui);
    font-size: calc(11.5px * var(--s-ui));
    color: var(--accent);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
