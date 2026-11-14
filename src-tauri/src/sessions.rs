use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Extensão gerada pelo app: registra hooks e escreve uma linha por invocação.
const HOOK_EXTENSION: &str = r#"// pi-deck hook logger — gerado pelo pi-deck. Desligue em Ajustes → Instrumentar hooks.
import { appendFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

const dir = join(homedir(), ".pi", "agent", "pi-deck");
const file = join(dir, "hooks.log");

function log(hook, extra) {
  try {
    mkdirSync(dir, { recursive: true });
    appendFileSync(file, new Date().toISOString() + " " + hook + (extra ? " " + extra : "") + "\n");
  } catch { /* silencioso */ }
}

const HOOKS = [
  "session_start",
  "agent_start",
  "before_agent_start",
  "tool_call",
  "message_end",
  "agent_end",
  "agent_settled",
  "model_select",
];

export default function (pi) {
  for (const name of HOOKS) {
    pi.on(name, async (event) => {
      let extra = "";
      try {
        if (name === "tool_call" && event && event.toolName) extra = String(event.toolName);
        if (name === "model_select" && event && event.model) extra = String(event.model.id || event.model);
      } catch { /* ignora */ }
      log(name, extra);
    });
  }
}
"#;

/// Versão deste app (compilada do Cargo.toml).
#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Abre uma URL no navegador padrão do sistema.
#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("somente http(s)".to_string());
    }

    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(&url);
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(&url);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", &url]);
        c
    };

    cmd.spawn().map_err(|e| format!("abrir navegador: {e}"))?;
    Ok(())
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Eco de diagnóstico do frontend (só com PWUI_DEBUG=1).
#[tauri::command]
pub fn debug_note(msg: String) -> String {
    if std::env::var("PWUI_DEBUG").is_ok() {
        eprintln!("[ui] {msg}");
    }
    msg
}

/// Pacotes instalados (`pi list`), texto cru.
#[tauri::command]
pub fn pi_list() -> Result<String, String> {
    let out = std::process::Command::new("pi")
        .arg("list")
        .output()
        .map_err(|e| format!("spawn pi: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&out.stderr).to_string();
    }
    Ok(text)
}

/// Instala um pacote pelo CLI oficial (`pi install <source>`).
#[tauri::command]
pub fn pi_install(spec: String, local: bool) -> Result<String, String> {
    let mut cmd = std::process::Command::new("pi");
    cmd.args(["install", spec.as_str(), "--no-approve"]);
    if local {
        cmd.arg("-l");
    }
    let out = cmd.output().map_err(|e| format!("spawn pi: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.trim().is_empty() {
        text.push('\n');
        text.push_str(&err);
    }
    Ok(format!("exit {}\n{}", out.status.code().unwrap_or(-1), text.trim()))
}

/// Adiciona/atualiza servidor MCP via `pi mcp add` (o CLI grava o mcp.json).
#[tauri::command]
pub fn pi_mcp_add(
    name: String,
    url: String,
    argv: Vec<String>,
    env: Vec<String>,
    headers: Vec<String>,
    local: bool,
) -> Result<String, String> {
    let mut cmd = std::process::Command::new("pi");
    cmd.args(["mcp", "add", name.as_str()]);
    if local {
        cmd.arg("-l");
    }
    for e in &env {
        if !e.trim().is_empty() {
            cmd.args(["--env", e]);
        }
    }
    for h in &headers {
        if !h.trim().is_empty() {
            cmd.args(["--header", h]);
        }
    }
    if !url.trim().is_empty() {
        cmd.args(["--url", url.trim()]);
    } else {
        if argv.is_empty() {
            return Err("informe uma URL ou um comando".to_string());
        }
        cmd.arg("--");
        cmd.args(&argv);
    }

    let out = cmd.output().map_err(|e| format!("spawn pi: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.trim().is_empty() {
        text.push('\n');
        text.push_str(&err);
    }
    Ok(format!("exit {}\n{}", out.status.code().unwrap_or(-1), text.trim()))
}

/// Remove um pacote instalado (`pi remove <source>`).
#[tauri::command]
pub fn pi_remove(spec: String, local: bool) -> Result<String, String> {
    let mut cmd = std::process::Command::new("pi");
    cmd.args(["remove", spec.as_str(), "--no-approve"]);
    if local {
        cmd.arg("-l");
    }
    let out = cmd.output().map_err(|e| format!("spawn pi: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.trim().is_empty() {
        text.push('\n');
        text.push_str(&err);
    }
    Ok(format!("exit {}\n{}", out.status.code().unwrap_or(-1), text.trim()))
}

/// Remove servidor MCP (`pi mcp remove <name>`).
#[tauri::command]
pub fn pi_mcp_remove(name: String, local: bool) -> Result<String, String> {
    let mut cmd = std::process::Command::new("pi");
    cmd.args(["mcp", "remove", name.as_str()]);
    if local {
        cmd.arg("-l");
    }
    let out = cmd.output().map_err(|e| format!("spawn pi: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.trim().is_empty() {
        text.push('\n');
        text.push_str(&err);
    }
    Ok(format!("exit {}\n{}", out.status.code().unwrap_or(-1), text.trim()))
}

/// Servidores MCP em JSON (`pi mcp list --json`) — evita ler mcp.json e expor `env`.
#[tauri::command]
pub fn pi_mcp_list() -> Result<String, String> {
    let out = std::process::Command::new("pi")
        .args(["mcp", "list", "--json"])
        .output()
        .map_err(|e| format!("spawn pi: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&out.stderr).to_string();
    }
    Ok(text)
}

/// Atualiza pi e/ou extensões pelo próprio CLI (`pi update`).
#[tauri::command]
pub fn pi_update(scope: String) -> Result<String, String> {
    let target = match scope.as_str() {
        "self" => "--self",
        "all" => "--all",
        "models" => "--models",
        _ => "--extensions",
    };

    let out = std::process::Command::new("pi")
        .args(["update", target, "--no-approve"])
        .output()
        .map_err(|e| format!("spawn pi: {e}"))?;

    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.trim().is_empty() {
        text.push('\n');
        text.push_str(&err);
    }
    let code = out.status.code().unwrap_or(-1);
    Ok(format!("exit {code}\n{}", text.trim()))
}

/// `pi` filhos marcados com PI_DECK_CHILD cujo pai morreu (ppid 1) — limpa órfãos de crash.
#[tauri::command]
pub fn kill_orphans() -> Result<usize, String> {
    let entries = fs::read_dir("/proc").map_err(|e| e.to_string())?;
    let mut killed = 0usize;

    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let base = format!("/proc/{pid}");

        let Ok(environ) = fs::read(format!("{base}/environ")) else {
            continue;
        };
        if !String::from_utf8_lossy(&environ).contains("PI_DECK_CHILD=1") {
            continue;
        }

        let Ok(stat) = fs::read_to_string(format!("{base}/stat")) else {
            continue;
        };
        let ppid = stat
            .split_whitespace()
            .nth(3)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        if ppid != 1 {
            continue;
        }

        let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
        killed += 1;
    }

    Ok(killed)
}

/// Converte "204.8K"/"1M" da tabela do CLI em bytes.
fn parse_context(raw: &str) -> u64 {
    let t = raw.trim().to_uppercase();
    let (num, mult) = if let Some(v) = t.strip_suffix('K') {
        (v, 1_000u64)
    } else if let Some(v) = t.strip_suffix('M') {
        (v, 1_000_000u64)
    } else {
        (t.as_str(), 1u64)
    };
    num.replace(',', ".")
        .parse::<f64>()
        .map(|f| (f * mult as f64) as u64)
        .unwrap_or(0)
}

/// Catálogo de modelos direto do CLI (`pi --list-models`) — instantâneo, sem subir sessão.
#[tauri::command]
pub fn pi_models() -> Result<String, String> {
    let out = std::process::Command::new("pi")
        .arg("--list-models")
        .output()
        .map_err(|e| format!("spawn pi: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);

    let mut items: Vec<String> = Vec::new();
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 3 || cols[0].starts_with("provider") {
            continue;
        }
        items.push(format!(
            "{{\"provider\":\"{}\",\"id\":\"{}\",\"name\":\"{}\",\"contextWindow\":{}}}",
            escape_json(cols[0]),
            escape_json(cols[1]),
            escape_json(cols[1]),
            parse_context(cols[2])
        ));
    }

    Ok(format!("[{}]", items.join(",")))
}

/// Último modelo registrado na sessão (record `model_change`), sem precisar de RPC.
#[tauri::command]
pub fn session_model(path: String) -> Result<String, String> {
    let raw = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;

    let mut provider = String::new();
    let mut model_id = String::new();

    for line in raw.lines() {
        if !line.contains("\"type\":\"model_change\"") {
            continue;
        }
        if let Some(p) = extract_string(line, "\"provider\":\"") {
            provider = p;
        }
        if let Some(m) = extract_string(line, "\"modelId\":\"") {
            model_id = m;
        }
    }

    if model_id.is_empty() {
        return Ok("null".to_string());
    }
    Ok(format!(
        "{{\"provider\":\"{}\",\"id\":\"{}\"}}",
        escape_json(&provider),
        escape_json(&model_id)
    ))
}

/// Caminho do log de hooks e se a extensão de instrumentação está instalada.
#[tauri::command]
pub fn hooks_status() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let ext = std::path::PathBuf::from(format!("{home}/.pi/agent/extensions/pi-deck-hooks.js"));
    let log = std::path::PathBuf::from(format!("{home}/.pi/agent/pi-deck/hooks.log"));
    Ok(format!(
        "{{\"installed\":{},\"extension\":\"{}\",\"log\":\"{}\"}}",
        ext.is_file(),
        ext.display(),
        log.display()
    ))
}

/// Instala a extensão que registra hooks em `~/.pi/agent/pi-deck/hooks.log`.
#[tauri::command]
pub fn hooks_install() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let dir = std::path::PathBuf::from(format!("{home}/.pi/agent/extensions"));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ext = dir.join("pi-deck-hooks.js");
    fs::write(&ext, HOOK_EXTENSION).map_err(|e| format!("escrever {}: {e}", ext.display()))?;
    Ok(format!("instalado em {}", ext.display()))
}

#[tauri::command]
pub fn hooks_remove() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let ext = std::path::PathBuf::from(format!("{home}/.pi/agent/extensions/pi-deck-hooks.js"));
    if ext.is_file() {
        fs::remove_file(&ext).map_err(|e| e.to_string())?;
        return Ok("removido".to_string());
    }
    Ok("não estava instalado".to_string())
}

/// Últimas linhas do log de hooks.
#[tauri::command]
pub fn hooks_tail(lines: Option<usize>) -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let path = format!("{home}/.pi/agent/pi-deck/hooks.log");
    let Ok(raw) = fs::read_to_string(&path) else {
        return Ok(String::new());
    };
    let max = lines.unwrap_or(200).clamp(1, 2000);
    let all: Vec<&str> = raw.lines().collect();
    let start = all.len().saturating_sub(max);
    Ok(all[start..].join("\n"))
}

/// Versão do `pi` instalado.
#[tauri::command]
pub fn pi_version() -> String {
    std::process::Command::new("pi")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "?".to_string())
}

/// Flags de debug para testes automatizados (env vars).
#[tauri::command]
pub fn debug_flags() -> String {
    let mut flags: Vec<&str> = Vec::new();
    if std::env::var("PWUI_AUTONEW").is_ok() {
        flags.push("autonew");
    }
    if std::env::var("PWUI_DEBUG").is_ok() {
        flags.push("debug");
    }
    flags.join(",")
}

/// Sessão "sem pasta": cwd = HOME (grupo `--home-<user>--`).
#[tauri::command]
pub fn home_dir() -> Result<String, String> {
    std::env::var("HOME").map_err(|e| e.to_string())
}

/// Lê o valor string de uma chave, respeitando escapes (`\"`, `\n`, `\\`).
fn extract_string(line: &str, key: &str) -> Option<String> {
    let start = line.find(key)? + key.len();
    let bytes: Vec<char> = line[start..].chars().collect();

    let mut raw = String::new();
    let mut backslashes = 0usize;
    for ch in bytes {
        if ch == '"' && backslashes % 2 == 0 {
            break;
        }
        if ch == '\\' {
            backslashes += 1;
        } else {
            backslashes = 0;
        }
        raw.push(ch);
    }
    if raw.is_empty() {
        return None;
    }

    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push(' '),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    Some(out)
}

/// Título = `session_info.name` (setado por `/name`, `--name`) ou a 1ª mensagem do usuário.
fn read_title(session_file: &Path) -> Option<String> {
    let file = fs::File::open(session_file).ok()?;
    let mut name: Option<String> = None;
    let mut first_user: Option<String> = None;

    for (idx, line) in BufReader::new(file).lines().enumerate() {
        if idx > 200 {
            break;
        }
        let Ok(line) = line else { break };

        if line.contains("\"type\":\"session_info\"") {
            if let Some(n) = extract_string(&line, "\"name\":\"") {
                name = Some(n); // o último vence
            }
        }
        if first_user.is_none() && line.contains("\"role\":\"user\"") {
            // User message: content e' array [{"type":"text","text":"..."}].
            first_user = extract_string(&line, "\"text\":\"")
                .or_else(|| extract_string(&line, "\"content\":\""));
        }
        if name.is_some() && first_user.is_some() {
            break;
        }
    }

    let picked = name.or(first_user)?;
    // tira cercas de código e colapsa espacos
    let flat = picked
        .replace('`', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if flat.is_empty() {
        return None;
    }
    Some(flat.chars().take(64).collect())
}

fn read_cwd(session_file: &Path) -> Option<String> {
    let file = fs::File::open(session_file).ok()?;
    let mut first = String::new();
    BufReader::new(file).read_line(&mut first).ok()?;
    extract_string(&first, "\"cwd\":\"")
}

fn read_mtime(session_file: &Path) -> u64 {
    fs::metadata(session_file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Estado do child a partir do arquivo de sessão dele.
/// `stopReason: stop` = terminou; `toolUse` = parou no meio (interrompido).
/// Sem `stopReason` = desconhecido.
fn child_state(session_file: &Path) -> &'static str {
    use std::io::{Read, Seek, SeekFrom};

    let Ok(mut f) = fs::File::open(session_file) else {
        return "unknown";
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(64 * 1024);
    if f.seek(SeekFrom::Start(start)).is_err() {
        return "unknown";
    }
    let mut buf = Vec::new();
    if f.read_to_end(&mut buf).is_err() {
        return "unknown";
    }
    let tail = String::from_utf8_lossy(&buf);

    let Some(idx) = tail.rfind("\"role\":\"assistant\"") else {
        return "unknown";
    };

    match extract_string(&tail[idx..], "\"stopReason\":\"").as_deref() {
        Some("stop") => "done",
        Some("toolUse") => "unfinished",
        Some(_) => "unknown",
        None => "unknown",
    }
}

/// Últimas `limit` mensagens de um arquivo de sessão (JSONL), precedidas por um record
/// `__deck_meta` com quantas ficaram de fora. Evita mandar 20 MB+ para o webview.
#[tauri::command]
pub fn read_session_messages(
    path: String,
    limit: Option<usize>,
    skip: Option<usize>,
) -> Result<String, String> {
    let max = limit.unwrap_or(250).clamp(1, 2000);
    let skip = skip.unwrap_or(0);
    let raw = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;

    // Orçamento em bytes: 250 mensagens podem somar dezenas de MB (toolResults grandes).
    const MAX_LINE_BYTES: usize = 64 * 1024;
    const MAX_TOTAL_BYTES: usize = 1024 * 1024;

    // Passo 1: contar TODAS as mensagens. Sem isto, o `total` parava no `break`
    // da coleta e `hasMore` saía sempre falso (bug do botão "carregar anteriores").
    let total = raw.lines().filter(|l| l.contains("\"type\":\"message\"")).count();

    let mut kept: Vec<&str> = Vec::new();
    let mut skipped = 0usize;
    let mut oversized = 0usize;
    let mut budget = MAX_TOTAL_BYTES;

    // Passo 2: pegar de trás para frente, pulando o que já está na tela.
    for line in raw.lines().rev() {
        if !line.contains("\"type\":\"message\"") {
            continue;
        }

        if skipped < skip {
            skipped += 1;
            continue;
        }
        if line.len() > MAX_LINE_BYTES {
            oversized += 1;
            continue;
        }
        if line.len() > budget {
            break;
        }
        budget -= line.len();
        kept.push(line);
        if kept.len() >= max {
            break;
        }
    }
    kept.reverse();

    let older = total.saturating_sub(skipped + kept.len() + oversized);
    let consumed = skipped + kept.len() + oversized;
    let mut out = format!(
        "{{\"type\":\"__deck_meta\",\"omitted\":{older},\"oversized\":{oversized},\"total\":{total},\"consumed\":{consumed},\"hasMore\":{},\"bytes\":{}}}\n",
        older > 0,
        MAX_TOTAL_BYTES - budget
    );
    for line in kept {
        out.push_str(line);
        out.push('\n');
    }
    Ok(out)
}

/// "Exclui" uma sessão movendo o `.jsonl` e o diretório de children para
/// `<projeto>/.pi-deck-trash/<stem>/`. Nunca apaga de verdade.
#[tauri::command]
pub fn delete_session(file: String) -> Result<String, String> {
    let src = std::path::PathBuf::from(&file);
    if !src.is_file() {
        return Err(format!("sessão não encontrada: {file}"));
    }

    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or("arquivo sem nome")?;
    let dir = src.parent().ok_or("sem diretório")?.to_path_buf();
    let trash = dir.join(".pi-deck-trash").join(&stem);
    fs::create_dir_all(&trash).map_err(|e| format!("criar lixeira: {e}"))?;

    let file_name = src.file_name().ok_or("arquivo sem nome")?.to_owned();
    fs::rename(&src, trash.join(&file_name)).map_err(|e| format!("mover sessão: {e}"))?;

    let children = dir.join(&stem);
    if children.is_dir() {
        fs::rename(&children, trash.join(&stem)).map_err(|e| format!("mover children: {e}"))?;
    }

    Ok(format!("movido para {}", trash.display()))
}

/// Children da sessão lidos do disco: `<sessão>/<childId>/run-N/session.jsonl`.
/// Usado quando não há widget ao vivo (sessão aberta do histórico).
#[tauri::command]
pub fn list_children(dir: String) -> Result<String, String> {
    let root = std::path::PathBuf::from(&dir);
    if !root.is_dir() {
        return Ok("[]".to_string());
    }

    let mut items: Vec<(u64, String)> = Vec::new();

    let Ok(children) = fs::read_dir(&root) else {
        return Ok("[]".to_string());
    };

    for child in children.flatten() {
        let child_dir = child.path();
        if !child_dir.is_dir() {
            continue;
        }
        let child_id = child
            .file_name()
            .to_string_lossy()
            .to_string();

        let Ok(runs) = fs::read_dir(&child_dir) else {
            continue;
        };

        for run in runs.flatten() {
            let run_dir = run.path();
            let run_name = run.file_name().to_string_lossy().to_string();
            if !run_dir.is_dir() || !run_name.starts_with("run-") {
                continue;
            }
            let file = run_dir.join("session.jsonl");
            if !file.is_file() {
                continue;
            }
            let label = read_title(&file).unwrap_or_default();
            let label = label.trim_start_matches("Task:").trim().to_string();
            let short: String = label.chars().take(60).collect();

            let mtime = read_mtime(&file);
            let mut state = child_state(&file);
            // Heurística: arquivo tocado nos últimos 2 min e sem `stop` -> provavelmente rodando.
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            if state != "done" && now.saturating_sub(mtime) < 120_000 {
                state = "running";
            }
            let json = format!(
                "{{\"id\":\"{}\",\"run\":\"{}\",\"label\":\"{}\",\"state\":\"{}\",\"file\":\"{}\",\"mtime\":{}}}",
                escape_json(&child_id),
                escape_json(&run_name),
                escape_json(&short),
                state,
                escape_json(&file.to_string_lossy()),
                mtime
            );
            items.push((mtime, json));
        }
    }

    items.sort_by(|a, b| b.0.cmp(&a.0));
    let body = items
        .into_iter()
        .map(|(_, json)| json)
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!("[{body}]"))
}

/// Sidebar: sessões agrupadas por pasta.
/// `file` = caminho absoluto do `.jsonl` (é o que vai em `pi --session <path>`).
#[tauri::command]
pub fn list_sessions() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let root = std::path::PathBuf::from(format!("{home}/.pi/agent/sessions"));
    let entries = fs::read_dir(&root).map_err(|e| format!("{}: {e}", root.display()))?;

    let mut groups: Vec<(String, Vec<String>)> = Vec::new();

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Ok(inner) = fs::read_dir(&dir) else {
            continue;
        };

        let mut sessions: Vec<(u64, String, String)> = Vec::new(); // (mtime, json, cwd)

        for file in inner.flatten() {
            let path = file.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
                continue;
            };

            let cwd = read_cwd(&path).unwrap_or_default();
            let title = read_title(&path).unwrap_or_default();
            let child_dir = path.with_extension("");
            let children = if child_dir.is_dir() {
                fs::read_dir(&child_dir)
                    .map(|d| d.flatten().count())
                    .unwrap_or(0)
            } else {
                0
            };

            let json = format!(
                "{{\"id\":\"{}\",\"file\":\"{}\",\"cwd\":\"{}\",\"title\":\"{}\",\"children\":{}}}",
                escape_json(&stem),
                escape_json(&path.to_string_lossy()),
                escape_json(&cwd),
                escape_json(&title),
                children
            );
            sessions.push((read_mtime(&path), json, cwd));
        }

        // Grupo sem sessão não aparece (era o que virava "--home-user--" na sidebar).
        if sessions.is_empty() {
            continue;
        }

        sessions.sort_by(|a, b| b.0.cmp(&a.0));

        let group_path = sessions
            .iter()
            .map(|s| s.2.clone())
            .find(|c| !c.is_empty())
            .unwrap_or_else(|| {
                dir.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            });

        groups.push((group_path, sessions.into_iter().map(|s| s.1).collect()));
    }

    groups.sort_by(|a, b| a.0.cmp(&b.0));

    let json = groups
        .iter()
        .map(|(path, items)| {
            let name = path.rsplit('/').next().unwrap_or(path);
            let name = if name.is_empty() { path.as_str() } else { name };
            format!(
                "{{\"path\":\"{}\",\"name\":\"{}\",\"sessions\":[{}]}}",
                escape_json(path),
                escape_json(name),
                items.join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    Ok(format!("[{json}]"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prova que uma sessão grande não vai inteira para o frontend.
    #[test]
    fn caps_messages_from_biggest_session() {
        let home = std::env::var("HOME").unwrap();
        let dir = format!("{home}/.pi/agent/sessions");
        let mut best: Option<(u64, std::path::PathBuf)> = None;

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let d = entry.path();
                if !d.is_dir() {
                    continue;
                }
                let Ok(inner) = fs::read_dir(&d) else { continue };
                for f in inner.flatten() {
                    let p = f.path();
                    if p.extension().and_then(|x| x.to_str()) != Some("jsonl") {
                        continue;
                    }
                    let size = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                    if best.as_ref().map(|(s, _)| size > *s).unwrap_or(true) {
                        best = Some((size, p));
                    }
                }
            }
        }

        let Some((size, path)) = best else {
            println!("sem sessões para testar");
            return;
        };

        let out = read_session_messages(path.to_string_lossy().to_string(), Some(50), None).unwrap();
        let lines = out.lines().count();
        let first = out.lines().next().unwrap_or("");

        println!("arquivo {:.1} MB -> {} linhas", size as f64 / 1_048_576.0, lines);
        println!("meta: {first}");

        assert!(lines <= 51, "esperado no máximo 51 linhas, veio {lines}");
        assert!(first.contains("__deck_meta"), "1a linha deveria ser o meta: {first}");
        assert!(
            out.len() < 1_100_000,
            "payload acima do orçamento de 1 MB: {} bytes",
            out.len()
        );
        assert!(
            out.lines().all(|l| l.len() <= 64 * 1024),
            "alguma linha passou do limite de 64 KB"
        );
        assert!(first.contains("\"hasMore\":true"), "hasMore deveria ser true numa sessão grande: {first}");

        // Segunda página não pode repetir o que já foi consumido.
        let consumed = first
            .split("\"consumed\":")
            .nth(1)
            .and_then(|s| s.split([',', '}']).next())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        assert!(consumed > 0, "consumed inválido em {first}");

        let page2 = read_session_messages(path.to_string_lossy().to_string(), Some(50), Some(consumed)).unwrap();
        assert_eq!(page2.lines().count(), out.lines().count(), "página 2 deveria ter o mesmo tamanho");
        let first2 = page2.lines().next().unwrap_or("");
        println!("pagina 2 meta: {first2}");
    }
}

