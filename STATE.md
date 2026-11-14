# STATE — pi-deck

App desktop (Tauri 2 + Rust) para sessões do `pi`, com subagents **por sessão** e workflows/schedules por projeto.
Status: **M0 concluído e provado** · **M1 scaffold compila e abre janela**.

## Decisões travadas

- Stack: **Tauri 2** (Rust). Não usa Electron nem embrulha o `pi-web` (Next.js) — sem servidor embutido.
- **1 processo `pi --mode rpc` por sessão** (mesmo padrão do `pi-desktop`).
- Subagents são **por sessão**, nunca globais (pedido do usuário).
- Painel ao vivo vem do **widget**; disco (`<stem>/<childId>/run-N/session.jsonl`) só para histórico de sessão fechada.
- Sessão "sem pasta" = `cwd = $HOME` → grupo `--home-<user>--`. Nada de conceito novo.

## Contrato verificado (não re-verificar)

- `pi --mode rpc` (`--session <path>` p/ abrir, `--session-id <id>` p/ criar). Prompt:
  `{"id":"x","type":"prompt","message":"..."}`.
- Subagent ao vivo chega como `extension_ui_request` `method=setWidget`, payload
  `"PI_SUBAGENT_ASYNC_JSON:"+JSON` → `{version:1, runs:[{id,label,state,startedAt,children:[{agent,model,thinking}]}], caps, omitted}`.
  Também chega `setStatus`. `custom()` é **stub** em RPC (tela cheia do `/subagents-fleet` não existe aqui).
- Sidebar: `~/.pi/agent/sessions/--<proj>--/<stem>.jsonl` + dir irmão `<stem>/` com os children da sessão.
  Caminho real do projeto = campo `cwd` da 1ª linha do `.jsonl` (nome do dir troca `/` por `-`, é ambíguo).
- Workflows/schedules já existem no pi-subagents: `.pi/workflows/*.js`,
  `subagent({action:"schedule.create", options:{at|every|catchUp}})`, missions em `~/.pi/agent/missions/projects/<hash>/`.

## Como rodar

```bash
export PATH="$HOME/.cargo/bin:$PATH"   # cargo/rustc 1.99.1 fora do PATH
cd ~/Projects/pi-deck/src-tauri && cargo run
```

## Estado atual

- M0 provado · M1 funcionando · sessão abre, transcript carrega, modelos/thinking chegam.
- **RPC do pi 1.5.2 = 33 comandos** (`docs/rpc-commands.md`): tem `set_model`, `get_available_models`,
  `set_thinking_level`, `get_available_thinking_levels`, `get_session_stats`, `set_auto_compaction`,
  `compact`, `set_session_name`, `get_commands`. **Não tem** `reload`, nem MCP, nem config → recarregar = reiniciar processo.
- Boot escreve 4 comandos: `get_messages`, `get_state`, `get_available_models`, `get_available_thinking_levels`.
- Formas reais: `get_state.model = {id, provider, contextWindow, ...}`; `get_available_models.data.models[]`
  (mesmo objeto, ~36 modelos); `get_available_thinking_levels.data.levels[]`; `get_session_stats.data.tokens{...}`;
  `set_auto_compaction {enabled}`.
- Debug: `PWUI_DEBUG=1` loga cada linha RPC no stderr; `PWUI_AUTONEW=1` abre sessão nova sozinha;
  comando `debug_note` devolve eco do frontend (`[ui] models=36 selected=...`).
- Prova de UI sem Tauri: harness Playwright em `/tmp/harness` (`build.cjs` + `test.cjs`), 0 erros, 19 tool cards.

## Gotchas já pagos

- `generate_context!` exige `src-tauri/icons/icon.png` **RGBA** (PNG RGB falha).
- Comando `#[tauri::command]` no `main.rs` colide com `generate_handler!` (E0255) → manter em módulos.
- `window.__TAURI__.event.listen` precisa de guarda: sem ela o script morria antes de `loadSessions()`.
- `agent_settled` não existe no arquivo de sessão do child; estado vem de `stopReason` da última mensagem do assistant.
- Crate `capture/` é a sonda de protocolo do M0; log cru em `/tmp/m0-rpc.jsonl`.

## Próximo

1. Substituir `<select>` nativo (branco, destoa) por picker próprio: busca + grupos por provider + `contextWindow`.
2. Thinking como segmentos inline (`low high max`), não dropdown.
3. Indicador de contexto + toggle de auto-compactação (`get_session_stats` / `set_auto_compaction`).
4. Referência visual: pi-gpui 0.2.4 usa JetBrains Mono e `src/ui/theme.rs`; UI já migrou para JetBrains Mono.
5. Depois: `set_session_name`, `get_commands` (paleta), M4 schedules.
