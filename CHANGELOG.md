# Changelog

All notable changes to pi-deck. Format follows [Keep a Changelog](https://keepachangelog.com/),
versioning follows [SemVer](https://semver.org/).

## [0.1.6] — 2026-11-14

### Added
- **Update check**: on start (cached for 12h) and via `⚙ → Verificar atualização`, the app asks
  GitHub for the latest release and shows a green bar with the version and a `Baixar` button that
  opens the release page. No signing key involved; `.deb`/`.rpm` still update by reinstalling.

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

[0.1.6]: https://github.com/matheusbbdutra/pi-deck/releases/tag/v0.1.6
[0.1.5]: https://github.com/matheusbbdutra/pi-deck/releases/tag/v0.1.5
[0.1.4]: https://github.com/matheusbbdutra/pi-deck/releases/tag/v0.1.4
[0.1.1]: https://github.com/matheusbbdutra/pi-deck/releases/tag/v0.1.1
