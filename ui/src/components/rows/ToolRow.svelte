<script>
  export let row; // { kind:'tool', name, argsText, resultText, state, expanded, timeText }

  const icon = () => (row.state === 'ok' ? '✓' : row.state === 'err' ? '✕' : '•');
</script>

<div class="tool-card" class:ok={row.state === 'ok'} class:err={row.state === 'err'} class:collapsed={!row.expanded}>
  <div class="tool-head" title="click to {row.expanded ? 'collapse' : 'expand'}" onclick={() => (row.expanded = !row.expanded)}>
    <span class="tool-icon">{icon()}</span>
    <span class="tool-name">{row.name}</span>
    <span class="tool-args">{row.argsText}</span>
    <span class="tool-time">{row.timeText}</span>
  </div>
  <pre class="tool-body">{row.resultText}</pre>
</div>
