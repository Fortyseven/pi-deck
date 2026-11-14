# STATE — pi-deck

App desktop (Tauri 2 + Rust) para sessões do `pi`, com subagents **por sessão** e workflows/schedules por projeto.
Status: **M0 concluído e provado** · **M1 scaffold compila e abre janela**.

## Decisões travadas

- **Não seguir com o papel de launcher/daemon por ora (decidido 2026-11-14).** Motivos: `schedule.run-due`
  é tool action interna (não é contrato público de RPC), exigiria uma sessão `pi` runner por projeto vigiado,
  e uma extensão-ponte de manutenção contínua contra um pacote de 101k linhas. Reversível: o painel read-only
  de missions/schedules é a primeira peça caso a necessidade apareça.

- **Não reescrever o pi-subagents.** 348 arquivos, 101.476 linhas de JS, 61.958 só em `workflows`+`runs`,
  13 agentes builtin. Reescrever = refazer a parte com mais casos de borda e perder manutenção da comunidade.
- **Não embutir o pi-subagents.** É extensão TS que roda dentro do processo do `pi` (usa `ctx.ui`/hooks).
  Só um host Node (como o pi-web, que depende de `pi-coding-agent` + `pi-tui`) consegue hospedar; Rust/Tauri não.
- **O papel do pi-deck é ser o launcher/supervisor.** `docs/missions.md:160`: "`schedule.run-due` lets an
  external launcher start due project work without making `pi-subagents` a daemon". O app lê
  `.pi/subagents/schedules/<id>/` + `~/.pi/agent/missions/projects/<hash>/`, calcula o que venceu e chama
  `schedule.run-due`. Isso exige uma sessão `pi` viva (o comando é tool action) — ou seja, uma sessão
  "runner" por projeto mantida pelo app.
- Ponte determinística: extensão fina `pi-deck-bridge` expondo `/deck:schedule-*` e `/deck:mission-*`,
  chamada pelo app via `prompt` (comando de extensão executa sem passar pelo LLM).

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

1. **Estabilizar o que existe** (maior valor): persistir abas, preservar stream ao trocar de aba, CI que gera o `.deb` na tag.
2. `set_session_name` e `get_commands` (paleta de comandos) — ambos já existem no RPC.
3. Opcional/reversível: painel read-only de missions/schedules (só leitura de disco, sem runner).
4. Referência visual: pi-gpui 0.2.4 usa JetBrains Mono e `src/ui/theme.rs`; a UI já migrou para JetBrains Mono.
