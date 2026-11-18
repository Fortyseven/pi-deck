<script>
  import { S } from '../lib/state.svelte.js';
</script>

{#if S.settingsPopOpen}
  <div class="settings-pop" role="menu" onclick={(e) => e.stopPropagation()}>
    <label><input type="checkbox" checked={S.uiSettings.expandTools}
      onchange={(e) => S.settingChange('expandTools', e.target.checked)} /> Expanded tools</label>
    <label><input type="checkbox" checked={S.uiSettings.expandThink}
      onchange={(e) => S.settingChange('expandThink', e.target.checked)} /> Expanded thinking</label>
    <label><input type="checkbox" checked={S.uiSettings.showHidden}
      onchange={(e) => S.settingChange('showHidden', e.target.checked)} /> Show hidden projects</label>
    <label><input type="checkbox" checked={S.uiSettings.notifyDesktop}
      onchange={(e) => S.settingChange('notifyDesktop', e.target.checked)} /> Desktop notification on background finish</label>
    <label><input type="checkbox" checked={S.uiSettings.notifySound}
      onchange={(e) => S.settingChange('notifySound', e.target.checked)} /> Sound on background finish</label>
    <label><input type="checkbox" checked={S.hooksInstalled}
      onchange={(e) => S.setHooksEnabled(e.target.checked)} /> Instrument hooks</label>
    <div class="settings-sep"></div>
    <button class="ghost" onclick={S.openPkgs}>Packages and MCP</button>
    <div class="settings-actions">
      <button class="ghost" onclick={() => S.runUpdate('extensions')}>Update extensions</button>
      <button class="ghost" onclick={() => S.runUpdate('self')}>Update pi</button>
    </div>
    {#if S.updOut}<pre class="upd-out">{S.updOut}</pre>{/if}
  </div>
{/if}

<style lang="scss">
  .settings-pop {
    position: absolute;
    top: 34px;
    right: 10px;
    z-index: 60;
    width: 300px;
    max-height: calc(100vh - 50px);
    overflow-y: auto;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);

    label {
      display: flex;
      align-items: center;
      gap: 8px;
      font-size: 12px;
      color: var(--dim);
      cursor: pointer;
      input { accent-color: var(--accent); }
    }
    button.ghost { width: 100%; text-align: left; }
    .settings-sep { height: 1px; background: var(--line-soft); margin: 2px 0; }
    .settings-actions { display: flex; gap: 8px; }
    .settings-actions button { flex: 1; }
    .upd-out {
      max-height: 160px;
      overflow-y: auto;
      background: var(--code-bg);
      border: 1px solid var(--line-soft);
      border-radius: 6px;
      padding: 8px;
      font-size: 11px;
      white-space: pre-wrap;
      word-break: break-word;
      color: var(--dim);
    }
  }
</style>
