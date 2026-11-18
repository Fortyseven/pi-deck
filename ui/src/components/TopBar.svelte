<script>
  import { S } from '../lib/state.svelte.js';
</script>

<header>
  <span class="brand">pi-deck</span>
  <span class="status">
    <span class="dot" class:ready={S.stateCls === 'ready'} class:busy={S.stateCls === 'busy'}
      class:off={S.stateCls === 'off'}></span>
    <span>{S.statusText}</span>
  </span>
  <span class="status" title="live pi processes">{S.procCount}</span>
  <span class="spacer"></span>
  <button class="ghost" title="Show/hide the panel (Ctrl+B)" onclick={S.toggleSubs}>Panel</button>
  <button class="ghost" title="Settings" class:on={S.settingsOpen} onclick={() => S.settingsOpen = true}>⚙</button>
</header>

<style lang="scss">
  header {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }
  header .brand { font-size: 12px; color: var(--accent); font-weight: 600; }
  header .status {
    font-size: 11px;
    color: var(--mute);
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: #4a5568; flex: none; }
  .dot.ready { background: var(--ok); }
  .dot.busy { background: var(--warn); }
  .dot.off { background: #4a5568; }
  header .spacer { flex: 1; }
</style>
