# Changelog

All notable changes to pi-deck. Format follows [Keep a Changelog](https://keepachangelog.com/),
versioning follows [SemVer](https://semver.org/).

## [0.1.8] — 2026-11-15

### Fixed
- **Opening the app from the desktop launcher failed with `spawn pi: No such file or directory`**:
  a GUI-launched process inherits the graphical session `PATH`, which does not include
  `~/.pi/agent/bin` or `~/.local/bin`. The `pi` binary is now resolved by path
  (`PI_DECK_PI_BIN` → `~/.pi/agent/bin/pi` → `~/.local/bin/pi` → `PATH`) instead of relying on
  `PATH` alone, so the app works the same from the launcher and from a shell.
- `delete_session` and `read_session_messages` accepted any path handed over by the webview.
  Both now require the path to resolve inside `~/.pi/agent/sessions`, with `..` and symlinks
  resolved before the comparison.

### Changed
- `escape_json` was duplicated in `rpc.rs`; it now lives only in `sessions.rs` as
  `crate::sessions::escape_json`.

### Added
- 11 hermetic unit tests (tempdir based, no `$HOME`, parallel-safe) covering `pi_bin` resolution,
  `escape_json`, session message paging/byte limits and the new path containment. The previous
  test scanned the real `~/.pi/agent/sessions` and passed vacuously on an empty install.
- CI: a `test` job running `cargo test`, with `build` depending on it.

## [0.1.7] — 2026-11-15

### Added
- **`+ Pasta`**: choose any folder with the system picker (`zenity` on Linux, `osascript` on macOS,
  PowerShell on Windows) and a session is created there — the project then shows up in the tree.

### Changed
- Release workflow now builds on four runners and publishes **once**: the duplicate draft per tag
  (one per matrix job) is gone.

## [0.1.6] — 2026-11-15

### Added
- **Commands from the palette**: `/compact`, `/model`, `/thinking`, `/new`, `/name`, `/session`,
  `/copy`, `/export`, `/abort` and `/login`, routed by the app to the RPC (`compact`,
  `cycle_thinking_level`, `set_session_name`, `get_session_stats`, `get_last_assistant_text`,
  `export_html`, `abort`) or the CLI — never sent to the model as prompt text.
- **Provider login picker**: `/login` (or the auth chip) opens the 44 providers known to the CLI,
  marking which ones have models and which are ready; `logar` opens a terminal running
  `pi auth login <provider>` and the app polls `pi auth check` until it turns ready.
- **Auth chip** next to the pi version: `logar <provider>` when unauthenticated, `auth ok` when ready.
- **Update check**: on start (cached 12h) and via `⚙ → Verificar atualização`; a green bar shows the
  newer release with a `Baixar` button. No signing key involved.
- **Detailed context popover**: click the context chip (`47% · 474k/1M`) for window usage,
  auto-compaction state, cache read/write, input/output, cost and message count.

### Changed
- Opening a session connects immediately, so context, thinking, model and commands are populated;
  a tab dropped by the 3-process cap reconnects when you return to it.

### Fixed
- Commands that needed a live `pi` failed silently on read-only sessions (the process is now started first).
- Responses with `success: false` and commands without a specific handler are now reported in the transcript.
- The `carregar anteriores` bar no longer leaks into a new session.
- Auto-compaction state in the context popover read the wrong source.

## [0.1.5] — 2026-11-14

### Added
- **Slash command palette**: typing `/` opens a filtered list of every command the session
  exposes — `skill:*` (110 commands on a typical install), extension commands and prompt
  templates — with description and origin. `↑↓` to select, `Tab` to complete, `Enter` to run,
  `Esc` to close. The list is cached, so it works before a session is connected.
- Empty state on boot: logo, title, instruction and shortcut hints instead of a blank pane.

### Changed
- New app icon (v2): larger mark, legible at 16 px.
- Delete buttons use a muted border instead of a solid red block (red on hover).

## [0.1.4] — 2026-11-14

### Added
- Activity indicator in the transcript: bouncing dots with `enviado…` → `pensando…` →
  `respondendo…`, driven by `agent_start`, `thinking_delta`, `text_delta`, `agent_settled`.
- Model catalog and thinking levels load at boot from `pi --list-models` plus a cache of the
  last used model, so the picker is populated before any session is opened.
- Floating `↓` button to return to the end of the transcript, without dragging you down
  while you read history.
- Hook instrumentation: `⚙ → Instrumentar hooks` writes `~/.pi/agent/extensions/pi-deck-hooks.js`,
  which logs every hook invocation to `~/.pi/agent/pi-deck/hooks.log`; a **Hooks** tab shows it live.
- Extension activity feed in the transcript (`setStatus`, `notify`, `role: custom` messages).
- Per-project `+` to start a session bound to that folder.

### Changed
- Opening a session is read-only: no `pi` process is spawned until you send a message or
  change the model. Live processes are capped at 3 (oldest stopped) and killed on window close.
- History is paginated (250 messages / 1 MB, 64 KB per line) with a `carregar anteriores` bar.
- Sidebar is a collapsible tree; hidden projects and collapse state persist.
- Deleting a session moves it to `<project>/.pi-deck-trash/` instead of removing the file.
- Back button in the subagent detail is a pill; delete buttons have visible contrast.

### Fixed
- Infinite loop in the markdown renderer on a table-like line without a separator, which
  froze the webview (the "doesn't open" report).
- `hasMore` was always false because the message count stopped at the collection break.
- Composer fell out of the bottom when the `main` flex rule was accidentally replaced.
- `TypeError: {} is not iterable` when opening a subagent transcript (nested `messages`).
- Tab scrollbar blocked the close button with many tabs.

## [0.1.1] — 2026-11-14

First public build.

### Added
- Sessions grouped by project, transcript with markdown, tool cards, thinking blocks.
- Subagents of the current session (live widget + historical runs), with detail view.
- Model picker, thinking levels, context usage, auto-compaction, reload.
- Package catalog (npm `pi-package` keyword), install/remove, MCP servers add/remove.
- Multi-platform release pipeline: `.deb`, `.rpm`, `.AppImage`, `.msi`, `.exe`, `.dmg`
  (arm64 and Intel).

[0.1.7]: https://github.com/matheusbbdutra/pi-deck/releases/tag/v0.1.7
[0.1.6]: https://github.com/matheusbbdutra/pi-deck/releases/tag/v0.1.6
[0.1.5]: https://github.com/matheusbbdutra/pi-deck/releases/tag/v0.1.5
[0.1.4]: https://github.com/matheusbbdutra/pi-deck/releases/tag/v0.1.4
[0.1.1]: https://github.com/matheusbbdutra/pi-deck/releases/tag/v0.1.1
