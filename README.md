# pi-deck

Desktop client for the [pi coding agent](https://github.com/earendil-works/pi).

It talks to your installed `pi` over its JSONL/RPC protocol (`pi --mode rpc`) — one
process per session — and renders the session in a native window (Tauri 2 + WebKitGTK).

## What it does

- **Sessions grouped by project**, read from `~/.pi/agent/sessions`. A session started
  without choosing a project lives under `sem projeto` (cwd = `$HOME`).
- **Live transcript**: streaming text, markdown (headings, lists, tables, code fences),
  and one compact card per tool call with icon, elapsed time, and collapsible output.
- **Thinking blocks**, collapsed by default (`⚙` → "Thinking expandido").
- **Subagents of the current session**, never global: live runs decoded from the
  `PI_SUBAGENT_ASYNC_JSON:` widget, and historical runs read from
  `<session>/<childId>/run-N/session.jsonl`. Click one to open its transcript.
- **Model, thinking and context** in the composer: model picker with search and
  context size, thinking as inline level bars, context usage percent, auto-compaction
  toggle, and a reload that restarts the session's `pi` process.
- **Tabs** per open session, **update pi / extensions** from the settings popover.

## Requirements

- `pi` 1.4+ on `PATH` (the app never installs or updates pi on its own)
- Linux: `webkit2gtk-4.1`, `libsoup-3.0`, `gtk+-3.0`, `librsvg2`
- Node.js 22+ (pi's own runtime)

## Run

```bash
npm install
npm run dev            # development
npm run build          # release bundle (.deb) in src-tauri/target/release/bundle/
```

Or run the release binary directly:

```bash
./src-tauri/target/release/pi-deck
```

## Status

Early (0.1.0). Verified working: session list, transcript from disk and live stream,
tool cards, thinking, model/thinking switching, context percent, auto-compaction toggle,
subagent panel (live + history), tabs, pi version, pi/extension update.

Known gaps: session tabs are not persisted across restarts; a tab switch reloads the
transcript from disk, so in-flight text from an inactive tab is not replayed.

## License

MIT
