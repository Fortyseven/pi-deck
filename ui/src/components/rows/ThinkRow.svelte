<script>
  let { row } = $props(); // { kind:'think', text, collapsed }

  // Per-row collapsed state (mirrors the original DOM class toggle).
  let collapsed = $state(!!row.collapsed);
</script>

<div class="think-block" class:collapsed>
  <button class="think-head" type="button" onclick={() => (collapsed = !collapsed)}>
    thinking {collapsed ? '▸' : '▾'}
  </button>
  <pre class="think-body">{row.text}</pre>
</div>
