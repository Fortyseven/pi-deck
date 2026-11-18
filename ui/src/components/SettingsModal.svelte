<script>
  import { S } from '../lib/state.svelte.js';

  function close() { S.settingsOpen = false; }

  function onKey(e) {
    if (e.key === 'Escape' && S.settingsOpen) close();
  }

  // Close only when the click lands on the backdrop itself, not the dialog.
  function onBackdropClick(e) {
    if (e.target === e.currentTarget) close();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if S.settingsOpen}
  <!-- Click-away is a visual affordance; Esc and × are the keyboard paths. -->
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div class="backdrop" onclick={onBackdropClick}>
    <div class="modal" role="dialog" aria-label="Settings" tabindex="-1">
      <div class="modal-head">
        <span class="modal-title">settings</span>
        <span class="spacer"></span>
        <button class="icon" title="Close (Esc)" onclick={close}>×</button>
      </div>
      <div class="modal-body">
        <h4>view</h4>
        <label><input type="checkbox" checked={S.uiSettings.expandTools}
          onchange={(e) => S.settingChange('expandTools', e.target.checked)} /> Expanded tools</label>
        <label><input type="checkbox" checked={S.uiSettings.expandThink}
          onchange={(e) => S.settingChange('expandThink', e.target.checked)} /> Expanded thinking</label>
        <label><input type="checkbox" checked={S.uiSettings.showHidden}
          onchange={(e) => S.settingChange('showHidden', e.target.checked)} /> Show hidden projects</label>

        <h4>notifications</h4>
        <label><input type="checkbox" checked={S.uiSettings.notifyDesktop}
          onchange={(e) => S.settingChange('notifyDesktop', e.target.checked)} /> Desktop notification on background finish</label>
        <label><input type="checkbox" checked={S.uiSettings.notifySound}
          onchange={(e) => S.settingChange('notifySound', e.target.checked)} /> Sound on background finish</label>

        <h4>pi</h4>
        <label><input type="checkbox" checked={S.hooksInstalled}
          onchange={(e) => S.setHooksEnabled(e.target.checked)} /> Instrument hooks</label>
        <button class="ghost" onclick={S.openPkgs}>Packages and MCP</button>

        <h4>updates</h4>
        <div class="modal-actions">
          <button class="ghost" onclick={() => S.runUpdate('extensions')}>Update extensions</button>
          <button class="ghost" onclick={() => S.runUpdate('self')}>Update pi</button>
        </div>
        {#if S.updOut}<pre class="upd-out">{S.updOut}</pre>{/if}
      </div>
    </div>
  </div>
{/if}

<style lang="scss">
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: rgba(6, 8, 12, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    width: min(440px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
    outline: none;

    .modal-head {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 10px 14px;
      border-bottom: 1px solid var(--line);
      flex: none;
      .modal-title { font-size: 12px; color: var(--fg); font-weight: 600; }
      .spacer { flex: 1; }
    }

    .modal-body {
      padding: 14px;
      overflow-y: auto;
      display: flex;
      flex-direction: column;
      gap: 8px;
      h4 {
        margin: 10px 0 2px;
        font-size: 10px;
        color: var(--mute);
        font-weight: 600;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        &:first-child { margin-top: 0; }
      }
      label {
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 12px;
        color: var(--dim);
        cursor: pointer;
        input { accent-color: var(--accent); }
      }
      button.ghost { text-align: left; }
    }

    .modal-actions { display: flex; gap: 8px;
      button { flex: 1; }
    }
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
