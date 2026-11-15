# Changelog

All notable changes to pi-deck. Format follows [Keep a Changelog](https://keepachangelog.com/),
versioning follows [SemVer](https://semver.org/).

## [0.1.12] — 2026-11-15

### Added
- **Aba "Arquivos" no painel** (ao lado de Subagents): lista os arquivos editados pela sessão **e** pelos
  subagents, com contagem por arquivo; clicar abre os diffs no painel de detalhe (linhas `+`/`-` e hunks
  coloridos). O patch vem do `details.patch` que a própria tool `edit` grava no `.jsonl`; `write` vira um
  diff só de adições (limitado a 400 linhas). Comando Rust novo: `session_edits`, com a mesma checagem
  de contenção em `~/.pi/agent/sessions` dos demais leitores.
- **Botão "copiar" nas respostas formatadas** (aparece no hover) e nos blocos de código; copia o markdown
  cru via `copy_text`, com feedback "copiado" no próprio botão.

### Fixed
- **Transcript/barra da sessão anterior aparecendo na sessão nova**: `loadHistoryFromDisk` e `loadEarlier`
  pintavam no `#log` e reabriam a barra de paginação mesmo quando a leitura só resolvia depois de trocar de
  sessão (abrir sessão grande e clicar `+` no meio da leitura). Agora toda troca incrementa `viewSeq` e as
  leituras assíncronas (histórico, "carregar anteriores", children, transcript de child) descartam o
  resultado quando a geração mudou; `start()` também não rouba a aba de volta se for superado.
- **Colar texto travava o campo por ~1 s**: cada `Ctrl+V` chamava `paste_image`, que rodava
  `wl-paste --type image/png`; com texto no clipboard esse comando segura **1006 ms** antes de falhar
  (medido). Agora pergunta os tipos primeiro (`wl-paste --list-types`, 2 ms) e só lê os bytes quando há
  `image/*` (2 ms + 1 ms com imagem).
- Sessão nova não recebia `get_state`, então `sessionFile` (usado por "carregar anteriores" e pela aba
  Arquivos) só aparecia depois de trocar de aba.
- **Painel de subagents não listava nada em sessão aberta com `+ Nova`**: essas abas nascem com
  `childrenDir = null` e nada derivava o diretório depois; o `get_state` agora também descobre
  `<sessionFile sem .jsonl>` e recarrega a lista (clicar em `Subagents` também atualiza).
- **Painel mostrava os runs da sessão anterior**: `loadChildren` comparava só a geração capturada na
  chamada, e `switchTab` a chama depois do await do histórico — a leitura já nascia com a geração nova.
  Agora valida também que `current.childrenDir` é o diretório pedido, e `editedFiles`/badge da aba
  Arquivos são zerados na troca.
- `start()` registra aba e fila do LRU antes da guarda de geração (processo recém-aberto não vira órfão
  fora da evicção) e o caminho de erro também respeita a sessão em foco; `ensureRpc` captura
  `current.file` junto da chave; orçamento de `session_edits` agora corta do mais antigo (o patch novo
  não é o primeiro a cair).

## [0.1.11] — 2026-11-15

### Fixed
- **Sending in one tab killed another tab's running turn**: `ensureRpc()` picked the LRU victim purely by
  FIFO, so a `pi` process with a turn in flight was stopped to make room; keys also accumulated duplicates
  in `liveOrder` (a killed session stayed queued), making eviction fire with fewer than 3 live processes.
  Victims are now only idle tabs, the current tab is never a victim, the queue is deduped, and when every
  tab is busy the cap is exceeded instead of interrupting work.
- **`falha ao enviar get_messages: sessao '<id>' nao iniciada`**: `start()` stopped the previous process
  without clearing its tab state, and `rpc-exit` did not reset `connected`/`liveOrder`, so `switchTab`
  talked to a session the Rust side no longer had. Process exit now clears the tab, the counter and the
  pending rows, `switchTab` always calls `ensureRpc()` first, and a dropped process is announced in the
  transcript ("encerrou no meio do turno") instead of disappearing silently.
- `get_session_stats` was sent twice on every tab switch.

### Changed
- **Tab label is the session title, not the timestamp**: the date/time from the session id made tabs
  indistinguishable. The tab shows the project plus `list_sessions`' title (session name or first user
  message), falling back to the old `prettyId`; `/name` updates it immediately.
- **Session list is paginated per project**: 8 rows plus `carregar mais (N)`, expanded state persisted in
  `localStorage` (`expanded-projects`).

## [0.1.10] — 2026-11-15

### Added
- **Message queue while the agent runs**: prompts are always sent with `streamingBehavior: "steer"`,
  so sending during a run queues the message instead of failing with "Agent is already processing";
  the transcript line is marked `· na fila`.
- **Per-tab state and project**: each tab shows a status dot (pulsing green = turn running, grey =
  connected, none = read-only), the project name from the session `cwd`, and a tooltip with project,
  full path, session stem and state.
- **Per-tab composer**: draft text and pasted images live on the tab (`stashDraft`/`restoreDraft`);
  a new tab starts empty and switching back restores only that tab's draft.
- `/abort` clears the queue first and returns the last queued message to the composer, because pi
  keeps queued messages running after an abort.

### Fixed
- Switching tabs carried the previous tab's "enviado…/pensando…" (and its busy dot) into the new
  tab, including an empty session opened with `+`/`+ Nova`. Turn state is stored per tab now and
  background tabs keep feeding it, so the indicator cannot freeze.
- Tabs opened with `+`, `+ Nova` and `+ Pasta` showed "somente leitura" even though `pi` was already
  running for them.
- The tab status dot rendered as a ~20×16 circle: the `run` modifier collided with the subagent card
  class `.run` (`padding:8px 10px`), which wins over a 6px box under `box-sizing:border-box`.

## [0.1.9] — 2026-11-15

### Added
- **Paste screenshots**: `Ctrl+V` in the composer reads the image straight from the system clipboard
  (`wl-paste`/`xclip`, `pngpaste`, PowerShell) and sends it as `ImageContent`, which the pi RPC
  already supported. WebKitGTK does not deliver `image/*` on the webview `paste` event, so the app
  reads the clipboard itself instead of depending on it.
- **File as context**: the `@` button inserts `@<path>` into the prompt, so pi reads the file with
  its own tools instead of duplicating the content into the conversation.
- Sent images now show up in the transcript and open in a full-screen viewer on click
  (`Esc` or a click closes it).

### Fixed
- A new session from `+`, `+ Nova` or `+ Pasta` opened behind the opaque empty-state overlay, so the
  panel looked blank while the tab was already connected.
- That overlay was never hidden when messages rendered, so it could survive the first reply on a
  fresh session.
- `pi` subprocesses inherited the graphical session `PATH`, which lacks the mise shims — MCP servers
  died with `spawn npx ENOENT` (`sequential-thinking` went from `failed` to `connected`).
- Raw ANSI escapes from extension `setStatus`/`notify` leaked into the transcript
  (`0[38;5;102m LSP Inactive`); they are stripped now.

### Changed
- `Enter` sends and `Ctrl+Enter` inserts a newline (it was the other way around); `Shift+Enter`
  keeps breaking the line natively.

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
