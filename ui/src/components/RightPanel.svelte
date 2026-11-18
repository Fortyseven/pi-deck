<script>
  import { S } from '../lib/state.svelte.js';
  import { splitPath, relTime, fmtElapsed } from '../lib/util.js';
</script>

<section class="side-panel">
  <div class="panel-tabs">
    <button class="panel-tab" class:active={S.panelPane === 'subs'}
            title="Subagents of this session" onclick={() => S.switchPanel('subs')}>Subagents</button>
    <button class="panel-tab" class:active={S.panelPane === 'files'}
            title="Files edited by the session and its subagents" onclick={() => S.switchPanel('files')}>
      Files<span class="cnt">{S.filesCount ? S.filesCount : ''}</span>
    </button>
    <button class="panel-tab" class:active={S.panelPane === 'missions'}
            title="pi-subagents missions (read only)" onclick={() => S.switchPanel('missions')}>
      Missions<span class="cnt">{S.missionsList.length ? S.missionsList.length : ''}</span>
    </button>
    <button class="panel-tab" class:active={S.panelPane === 'diag'}
            title="Circular log of RPC lines (command / latency / result)" onclick={() => S.switchPanel('diag')}>Diagnostics</button>
  </div>

  {#if S.panelPane === 'subs'}
    <div class="panel-pane">
      {#if S.subsError}
        <div class="empty err">{S.subsError}</div>
      {:else if !S.subsList.length}
        <div class="empty">{S.noSubsMessage()}</div>
        {#if S.subsWarn}
          <div class="empty warn">pi-subagents did not load in this session (possible tool conflict). See ⚙ → Packages &amp; MCP.</div>
        {/if}
      {:else}
        {#each S.subsList as it (it.kind + ':' + (it.kind === 'live' ? it.run.id : (it.kind === 'disk' ? it.c.id : 'head'))) }
          {#if it.kind === 'head'}
            <div class="history-head">history · {it.count} run(s)</div>
          {:else if it.kind === 'live'}
            {@const run = it.run}
            {@const st = String(run.state || '?')}
            <div class="run">
              <div class="run-head">
                <span class="run-label">{String(run.label || run.kind || 'run')}</span>
                <span class="state state-{st}">{st}</span>
                <span class="tool-right">{fmtElapsed(run.startedAt, run.updatedAt || it.now)}</span>
              </div>
              {#each run.children || [] as ch (ch.id || ch.label)}
                <div class="child">
                  <b>{String(ch.label || ch.agent || ch.id)}</b> · {String(ch.state || '?')}{ch.model ? ' · ' + String(ch.model).split('/').pop() : ''}{ch.thinking ? ' · ' + ch.thinking : ''}
                </div>
              {/each}
            </div>
          {:else}
            {@const c = it.c}
            {@const meta = S.childStateMeta(c.state)}
            <div class="run clickable" title={c.id} onclick={() => S.openChild(c)}>
              <div class="run-head">
                <span class="run-label">{c.label || c.id.slice(0, 8)}</span>
                <span class="state {meta[1]}">{meta[0]}</span>
                <span class="tool-right">{c.run} · {relTime(c.mtime)}</span>
              </div>
            </div>
          {/if}
        {/each}
      {/if}
    </div>
  {/if}

  {#if S.panelPane === 'files'}
    <div class="panel-pane">
      {#if S.editsError}
        <div class="empty err">{S.editsError}</div>
      {:else if !S.fileGroups.length}
        <div class="empty">no file edited in this session yet</div>
      {:else}
        <div class="history-head">{S.fileGroups.length} file(s) · edited</div>
        {#each S.fileGroups as g (g.file)}
          {@const parts = splitPath(g.file)}
          <div class="file-row" title={g.file} onclick={() => S.openDiffs(g)}>
            <span class="file-name">{parts[0]}</span>
            <span class="file-dir">{parts[1]}</span>
            <span class="file-count">{g.items.length}</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  {#if S.panelPane === 'missions'}
    <div class="panel-pane">
      {#if S.missionsError}
        <div class="empty err">{S.missionsError}</div>
      {:else if !S.missionsList.length}
        <div class="empty">no mission on disk</div>
      {:else}
        <div class="history-head">{S.missionsList.length} mission(s)</div>
        {#each S.missionsList as m (m.id)}
          <div class="file-row" title={String(m.cwd || m.project || '')} onclick={() => S.openMission(m)}>
            <span class="file-name">{String(m.title || m.id || 'untitled')}</span>
            <span class="file-dir">{String(m.status || '')}</span>
            <span class="file-count">{String(m.runs || 0)}</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  {#if S.panelPane === 'diag'}
    <div class="panel-pane diag-pane">
      <div class="history-head">
        <span>{S.diagHead}</span>
        <button class="ghost tiny" onclick={S.diagClear}>clear</button>
      </div>
      {#if !S.diagBuf.length}
        <div class="empty">no RPC lines yet</div>
      {:else}
        <div class="diag-list">
          {#each S.diagBuf.slice().reverse() as e (e._k)}
            <div class="file-row" class:err={String(e.res || '').startsWith('failed')}
                 title={(e.cmd || '') + (e.res ? '\n' + e.res : '')}>
              <span class="file-name">{e.cmd || '—'}</span>
              <span class="file-dir">{e.res || ''}</span>
              <span class="file-count">{e.lat || '—'}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</section>

<style lang="scss">
  .side-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--panel);
    border-left: 1px solid var(--line);
  }

  .panel-tabs {
    display: flex;
    gap: 2px;
    padding: 0 4px;
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
    flex: none;
    .panel-tab {
      background: none;
      border: none;
      border-bottom: 2px solid transparent;
      color: var(--mute);
      padding: 7px 8px;
      font-family: var(--font-ui);
      font-size: calc(11px * var(--s-ui));
      cursor: pointer;
      &:hover { color: var(--dim); }
      &.active { color: var(--fg); border-bottom-color: var(--accent); }
      .cnt { margin-left: 4px; color: var(--mute); font-size: calc(9.5px * var(--s-ui)); }
    }
  }

  .panel-pane { flex: 1; overflow-y: auto; min-height: 0; }

  .diag-pane { display: flex; flex-direction: column;
    .diag-list { overflow-y: auto; flex: 1; }
  }

  /* subagent runs */
  .run { padding: 8px 10px; border-bottom: 1px solid var(--line-soft); }
  .run.clickable { cursor: pointer; &:hover { background: #1a1e2b; } }
  .run-head { display: flex; align-items: baseline; gap: 7px; font-family: var(--font-ui); font-size: calc(11.5px * var(--s-ui)); }
  .run-label { color: #c4cbdb; overflow: hidden; text-overflow: ellipsis; flex: 1; }
  .state { font-size: calc(9.5px * var(--s-ui)); padding: 1px 5px; border-radius: 3px; flex: none; }
  .state-running { background: #16301c; color: var(--ok); }
  .state-queued { background: #302a16; color: var(--warn); }
  .state-completed, .state-done { background: #1e2130; color: var(--dim); }
  .state-failed { background: #301619; color: var(--err); }
  .chip-done { background: #16301c; color: var(--ok); }
  .chip-unfinished { background: #302a16; color: var(--warn); }
  .chip-unknown { background: #1e2130; color: var(--dim); }
  .child {
    margin-top: 4px;
    padding-left: 12px;
    border-left: 1px solid var(--line);
    font-family: var(--font-ui);
    font-size: calc(11px * var(--s-ui));
    color: var(--dim);
    b { color: #a9b2c6; font-weight: 500; }
  }

  .history-head {
    padding: 8px 10px 6px;
    font-family: var(--font-ui);
    font-size: calc(10.5px * var(--s-ui));
    letter-spacing: 0.05em;
    color: var(--mute);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    flex: none;
  }

  .tool-right { color: var(--mute); font-size: calc(10px * var(--s-ui)); flex: none; }

  .file-row {
    display: flex;
    align-items: baseline;
    gap: 7px;
    padding: 6px 10px;
    cursor: pointer;
    border-bottom: 1px solid var(--line-soft);
    font-family: var(--font-ui);
    font-size: calc(11.5px * var(--s-ui));
    &:hover { background: #1a1e2b; }
    &.err .file-name { color: var(--err); }
  }
  .file-name { color: #c4cbdb; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .file-dir {
    color: #4d5568;
    font-size: calc(9.5px * var(--s-ui));
    flex: none;
    max-width: 42%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: right;
  }
  .file-count { flex: none; font-size: calc(9.5px * var(--s-ui)); padding: 1px 5px; border-radius: 3px; background: #1e2130; color: var(--dim); }

  .empty.err { color: var(--err); }
  .empty.warn { color: var(--warn); }
</style>
