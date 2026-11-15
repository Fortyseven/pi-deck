use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Caminho do binário `pi`.
/// Aberto pelo lançador gráfico o app não herda o PATH do shell (os error 2),
/// então procura nos diretórios de instalação antes de cair no PATH.
pub fn pi_bin() -> PathBuf {
    let env_val = std::env::var("PI_DECK_PI_BIN").ok();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    resolve_pi_bin(env_val.as_deref(), home.as_deref(), Path::is_file)
}

/// Decisão pura de `pi_bin()` — sem env e sem fs, para testar sem `set_var` (global).
fn resolve_pi_bin(
    env_val: Option<&str>,
    home: Option<&Path>,
    exists: impl Fn(&Path) -> bool,
) -> PathBuf {
    if let Some(p) = env_val.map(str::trim).filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    if let Some(home) = home {
        for rel in [".pi/agent/bin/pi", ".local/bin/pi"] {
            let cand = home.join(rel);
            if exists(&cand) {
                return cand;
            }
        }
    }
    PathBuf::from("pi")
}

/// Root onde o pi guarda as sessões (`~/.pi/agent/sessions`).
fn sessions_root() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME não definido")?;
    Ok(PathBuf::from(home).join(".pi/agent/sessions"))
}

/// Resolve `path` e exige que esteja sob `root`.
/// `canonicalize` resolve `..` e symlinks antes de comparar, então `../` não escapa.
fn contained_in(path: &Path, root: &Path) -> Result<PathBuf, String> {
    let canon_root = root
        .canonicalize()
        .map_err(|e| format!("root de sessões indisponível ({}): {e}", root.display()))?;
    let canon = path
        .canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if !canon.starts_with(&canon_root) {
        return Err(format!(
            "caminho fora do diretório de sessões: {}",
            path.display()
        ));
    }
    Ok(canon)
}

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

/// Lista de providers conhecidos pelo pi (com flag `configured`).
/// O CLI só expõe essa lista no erro de `pi auth login <invalido>`, então é de lá que vem.
#[tauri::command]
pub fn auth_providers() -> Result<String, String> {
    let mut providers: Vec<String> = Vec::new();

    if let Ok(out) = std::process::Command::new(pi_bin())
        .args(["auth", "login", "__deck_list__", "--no-browser"])
        .output()
    {
        let err = String::from_utf8_lossy(&out.stderr);
        for line in err.lines() {
            if let Some(rest) = line.strip_prefix("Providers: ") {
                providers = rest
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }

    // Fallback: os providers que já têm modelos configurados.
    if providers.is_empty() {
        if let Ok(out) = std::process::Command::new(pi_bin()).arg("--list-models").output() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines().skip(1) {
                if let Some(p) = line.split_whitespace().next() {
                    if !providers.iter().any(|x| x == p) {
                        providers.push(p.to_string());
                    }
                }
            }
        }
    }

    // providers com modelo no catálogo
    let mut configured: Vec<String> = Vec::new();
    if let Ok(out) = std::process::Command::new(pi_bin()).arg("--list-models").output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines().skip(1) {
            if let Some(p) = line.split_whitespace().next() {
                if !configured.iter().any(|x| x == p) {
                    configured.push(p.to_string());
                }
            }
        }
    }

    let items: Vec<String> = providers
        .iter()
        .map(|p| {
            let configured = configured.iter().any(|c| c == p);
            // Só consulta credencial dos que têm modelo no catálogo (evita 44 chamadas).
            let status = if configured {
                std::process::Command::new(pi_bin())
                    .args(["auth", "check", "--provider", p, "--json"])
                    .output()
                    .ok()
                    .map(|o| {
                        let out = String::from_utf8_lossy(&o.stdout);
                        let parsed = out.trim();
                        if parsed.contains("\"ready\"") {
                            "ready".to_string()
                        } else if parsed.contains("not_ready") {
                            "not_ready".to_string()
                        } else {
                            "unknown".to_string()
                        }
                    })
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                String::new()
            };

            format!(
                "{{\"name\":\"{}\",\"configured\":{},\"status\":\"{}\"}}",
                escape_json(p),
                configured,
                status
            )
        })
        .collect();

    Ok(format!("[{}]", items.join(",")))
}

/// Status de auth do provider: `pi auth check --provider X --json`.
#[tauri::command]
pub fn auth_status(provider: String) -> Result<String, String> {
    let p = provider.trim();
    if p.is_empty() {
        return Err("provider vazio".to_string());
    }
    let out = std::process::Command::new(pi_bin())
        .args(["auth", "check", "--provider", p, "--json"])
        .output()
        .map_err(|e| format!("spawn pi auth: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Abre um terminal já rodando `pi auth login <provider>` (login é interativo).
#[tauri::command]
pub fn open_terminal_login(provider: String) -> Result<String, String> {
    let p = provider.trim();
    if p.is_empty() {
        return Err("informe o provider".to_string());
    }
    let inner = format!("pi auth login {p}; echo; printf 'enter para fechar '; read -r _");

    #[cfg(target_os = "linux")]
    let candidates: Vec<(&str, Vec<String>)> = vec![
        ("xdg-terminal-exec", vec!["sh".into(), "-c".into(), inner.clone()]),
        ("kitty", vec!["sh".into(), "-c".into(), inner.clone()]),
        ("alacritty", vec!["-e".into(), "sh".into(), "-c".into(), inner.clone()]),
        ("foot", vec!["sh".into(), "-c".into(), inner.clone()]),
        ("wezterm", vec!["start".into(), "--".into(), "sh".into(), "-c".into(), inner.clone()]),
        ("xterm", vec!["-e".into(), "sh".into(), "-c".into(), inner.clone()]),
    ];
    #[cfg(target_os = "macos")]
    let candidates: Vec<(&str, Vec<String>)> = vec![(
        "open",
        vec!["-a".into(), "Terminal".into(), inner.clone()],
    )];
    #[cfg(target_os = "windows")]
    let candidates: Vec<(&str, Vec<String>)> = vec![(
        "cmd",
        vec!["/C".into(), "start".into(), inner.clone()],
    )];

    for (prog, args) in candidates {
        if let Ok(mut child) = std::process::Command::new(prog).args(&args).spawn() {
            // "spawn ok" não significa terminal aberto: se ele morrer logo, tenta o próximo.
            std::thread::sleep(std::time::Duration::from_millis(300));
            match child.try_wait() {
                Ok(Some(status)) if !status.success() => continue,
                _ => return Ok(format!("terminal aberto ({prog}) — conclua o login de {p}")),
            }
        }
    }

    Err(format!("nenhum terminal respondeu; copie e rode: pi auth login {p}"))
}

/// Copia texto para a área de transferência usando utilitário do sistema.
#[tauri::command]
pub fn copy_text(text: String) -> Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    #[cfg(target_os = "linux")]
    let candidates: Vec<(&str, Vec<&str>)> = vec![
        ("wl-copy", vec![]),
        ("xclip", vec!["-selection", "clipboard"]),
        ("xsel", vec!["--clipboard", "--input"]),
    ];
    #[cfg(target_os = "macos")]
    let candidates: Vec<(&str, Vec<&str>)> = vec![("pbcopy", vec![])];
    #[cfg(target_os = "windows")]
    let candidates: Vec<(&str, Vec<&str>)> = vec![("clip", vec![])];

    for (prog, args) in candidates {
        let Ok(mut child) = Command::new(prog)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        if child.wait().map(|s| s.success()).unwrap_or(false) {
            return Ok(());
        }
    }
    Err("nenhum utilitário de clipboard disponível".to_string())
}


/// Abre o seletor de pasta do sistema e devolve o caminho escolhido.
#[tauri::command]
pub fn pick_directory() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    let out = std::process::Command::new("zenity")
        .args([
            "--file-selection",
            "--directory",
            "--title=Escolher pasta do projeto",
        ])
        .output();

    #[cfg(target_os = "macos")]
    let out = std::process::Command::new("osascript")
        .args(["-e", "POSIX path of (choose folder)"])
        .output();

    #[cfg(target_os = "windows")]
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Add-Type -AssemblyName System.Windows.Forms; $f=New-Object System.Windows.Forms.FolderBrowserDialog; if($f.ShowDialog() -eq 'OK'){$f.SelectedPath}",
        ])
        .output();

    let out = out.map_err(|e| format!("seletor de pasta indisponível: {e}"))?;
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();

    if path.is_empty() {
        return Err("cancelado".to_string());
    }
    if !std::path::PathBuf::from(&path).is_dir() {
        return Err(format!("não é uma pasta: {path}"));
    }
    Ok(path)
}

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

pub(crate) fn escape_json(s: &str) -> String {
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
    let out = std::process::Command::new(pi_bin())
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
    let mut cmd = std::process::Command::new(pi_bin());
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
    let mut cmd = std::process::Command::new(pi_bin());
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
    let mut cmd = std::process::Command::new(pi_bin());
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
    let mut cmd = std::process::Command::new(pi_bin());
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
    let out = std::process::Command::new(pi_bin())
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

    let out = std::process::Command::new(pi_bin())
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
    let out = std::process::Command::new(pi_bin())
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
    std::process::Command::new(pi_bin())
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
    let root = sessions_root()?;
    read_session_messages_in(Path::new(&path), &root, limit, skip)
}

/// Mesma lógica, com o root explícito — testável sem tocar em `HOME`.
fn read_session_messages_in(
    path: &Path,
    root: &Path,
    limit: Option<usize>,
    skip: Option<usize>,
) -> Result<String, String> {
    // Sem isso, um path vindo do webview lê qualquer arquivo legível.
    let path = contained_in(path, root)?;
    let max = limit.unwrap_or(250).clamp(1, 2000);
    let skip = skip.unwrap_or(0);
    let raw = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;

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
    let root = sessions_root()?;
    delete_session_in(Path::new(&file), &root)
}

/// Mesma lógica, com o root explícito — testável sem tocar em `HOME`.
fn delete_session_in(file: &Path, root: &Path) -> Result<String, String> {
    if !file.is_file() {
        return Err(format!("sessão não encontrada: {}", file.display()));
    }
    // Sem isso, um path vindo do webview move qualquer arquivo para .pi-deck-trash.
    let src = contained_in(file, root)?;

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
    let root = sessions_root()?;
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SEQ: AtomicUsize = AtomicUsize::new(0);

    /// Diretório temporário do teste (só std; some no Drop).
    struct Tmp(PathBuf);

    impl Tmp {
        fn new(tag: &str) -> Self {
            let n = SEQ.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("pideck-{}-{tag}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("criar tmpdir");
            Tmp(dir)
        }

        fn at(&self, rel: &str) -> PathBuf {
            self.0.join(rel)
        }

        /// Cria `rel` com `content`, criando os diretórios pais.
        fn file(&self, rel: &str, content: &str) -> PathBuf {
            let p = self.at(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).expect("criar diretório");
            }
            fs::write(&p, content).expect("escrever arquivo");
            p
        }
    }

    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// JSONL com `n` mensagens, precedido de um record que não é mensagem.
    fn session_file(t: &Tmp, rel: &str, n: usize) -> PathBuf {
        let mut body = String::from("{\"type\":\"session_start\"}\n");
        for i in 0..n {
            body.push_str(&format!("{{\"type\":\"message\",\"id\":{i}}}\n"));
        }
        t.file(rel, &body)
    }

    // ---------- resolve_pi_bin: decisão pura, sem env e sem fs ----------

    #[test]
    fn pi_bin_env_wins_over_home() {
        let got = resolve_pi_bin(Some("/opt/x/pi"), Some(Path::new("/home/u")), |_| true);
        assert_eq!(got, PathBuf::from("/opt/x/pi"));
    }

    #[test]
    fn pi_bin_blank_env_falls_through() {
        let got = resolve_pi_bin(Some("   "), Some(Path::new("/home/u")), |p| {
            p.ends_with(".pi/agent/bin/pi")
        });
        assert_eq!(got, PathBuf::from("/home/u/.pi/agent/bin/pi"));
    }

    #[test]
    fn pi_bin_prefers_managed_install_then_local_bin_then_path() {
        let managed = resolve_pi_bin(None, Some(Path::new("/home/u")), |_| true);
        assert_eq!(managed, PathBuf::from("/home/u/.pi/agent/bin/pi"));

        let local = resolve_pi_bin(None, Some(Path::new("/home/u")), |p| {
            p.ends_with(".local/bin/pi")
        });
        assert_eq!(local, PathBuf::from("/home/u/.local/bin/pi"));

        assert_eq!(
            resolve_pi_bin(None, Some(Path::new("/home/u")), |_| false),
            PathBuf::from("pi")
        );
        assert_eq!(resolve_pi_bin(None, None, |_| true), PathBuf::from("pi"));
    }

    // ---------- escape_json ----------

    #[test]
    fn escape_json_covers_quotes_slashes_and_controls() {
        assert_eq!(escape_json("a\"b\\c\nd\re\tf"), "a\\\"b\\\\c\\nd\\re\\tf");
        assert_eq!(escape_json("\u{1}"), "\\u0001");
        assert_eq!(escape_json("\u{0}"), "\\u0000");
        // Não-ASCII passa cru: continua JSON válido e o parser do frontend aceita.
        assert_eq!(escape_json("é—🙂"), "é—🙂");
    }

    // ---------- read_session_messages ----------

    #[test]
    fn read_messages_caps_limit_and_counts_all() {
        let t = Tmp::new("caps");
        let root = t.at("sessions");
        let f = session_file(&t, "sessions/proj/s.jsonl", 300);

        let out = read_session_messages_in(&f, &root, Some(50), None).unwrap();
        assert_eq!(out.lines().count(), 51, "meta + 50 mensagens");

        let meta = out.lines().next().unwrap();
        assert!(meta.contains("\"total\":300"), "{meta}");
        assert!(meta.contains("\"omitted\":250"), "{meta}");
        assert!(meta.contains("\"hasMore\":true"), "{meta}");
    }

    #[test]
    fn read_messages_drops_oversized_line() {
        let t = Tmp::new("huge");
        let root = t.at("sessions");
        let huge = format!(
            "{{\"type\":\"message\",\"payload\":\"{}\"}}\n",
            "x".repeat(70 * 1024)
        );
        let f = t.file(
            "sessions/proj/s.jsonl",
            &format!("{{\"type\":\"message\",\"id\":1}}\n{huge}{{\"type\":\"message\",\"id\":3}}\n"),
        );

        let out = read_session_messages_in(&f, &root, Some(50), None).unwrap();
        let meta = out.lines().next().unwrap();
        assert!(meta.contains("\"oversized\":1"), "{meta}");
        assert!(meta.contains("\"total\":3"), "{meta}");
        assert_eq!(out.lines().count(), 3, "meta + 2 mensagens");
        assert!(!out.contains("xxxxx"), "linha gigante não pode ir ao webview");
    }

    #[test]
    fn read_messages_skip_walks_back_and_clears_has_more() {
        let t = Tmp::new("skip");
        let root = t.at("sessions");
        let f = session_file(&t, "sessions/proj/s.jsonl", 120);

        let out = read_session_messages_in(&f, &root, Some(50), Some(100)).unwrap();
        let meta = out.lines().next().unwrap();
        assert_eq!(out.lines().count() - 1, 20, "120 - 100 = 20 restantes");
        // `skip` anda de trás pra frente: as 100 mais novas já estão na tela,
        // então não sobra nada mais antigo que o retornado.
        assert!(meta.contains("\"omitted\":0"), "{meta}");
        assert!(meta.contains("\"consumed\":120"), "{meta}");
        assert!(meta.contains("\"hasMore\":false"), "{meta}");
        assert!(out.contains("\"id\":0}"), "deveria começar na mais antiga: {out}");
        assert!(
            !out.contains("\"id\":20}"),
            "não deveria alcançar as já carregadas: {out}"
        );
    }

    // ---------- containment: o webview não escolhe o que ler/apagar ----------

    #[test]
    fn read_messages_rejects_path_outside_root() {
        let t = Tmp::new("readout");
        let root = t.at("sessions");
        fs::create_dir_all(&root).unwrap();
        let secret = t.file("important.txt", "não deveria vazar");

        let err = read_session_messages_in(&secret, &root, None, None).unwrap_err();
        assert!(err.contains("fora do diretório de sessões"), "{err}");
    }

    #[test]
    fn read_messages_rejects_traversal() {
        let t = Tmp::new("trav");
        let root = t.at("sessions");
        t.file("sessions/proj/s.jsonl", "{\"type\":\"message\"}\n");
        t.file("important.txt", "não deveria vazar");

        let trav = t.at("sessions/proj/../../important.txt");
        let err = read_session_messages_in(&trav, &root, None, None).unwrap_err();
        assert!(err.contains("fora do diretório de sessões"), "{err}");
    }

    // ---------- delete_session ----------

    #[test]
    fn delete_session_moves_file_and_children_to_trash() {
        let t = Tmp::new("del");
        let root = t.at("sessions");
        let f = t.file("sessions/proj/s.jsonl", "{\"type\":\"message\"}\n");
        t.file("sessions/proj/s/child1/run-1/session.jsonl", "{}\n");

        let out = delete_session_in(&f, &root).unwrap();
        assert!(out.contains("movido para"), "{out}");
        assert!(!f.exists(), "jsonl deveria ter saído do lugar");
        assert!(
            !t.at("sessions/proj/s").exists(),
            "children deveriam ter saído"
        );
        assert!(
            t.at("sessions/proj/.pi-deck-trash/s/s.jsonl").exists(),
            "jsonl deveria estar na lixeira"
        );
        assert!(
            t.at("sessions/proj/.pi-deck-trash/s/s/child1/run-1/session.jsonl")
                .exists(),
            "children deveriam estar na lixeira"
        );
    }

    #[test]
    fn delete_session_rejects_file_outside_root() {
        let t = Tmp::new("delout");
        let root = t.at("sessions");
        fs::create_dir_all(&root).unwrap();
        let victim = t.file("important.txt", "conteúdo");

        let err = delete_session_in(&victim, &root).unwrap_err();
        assert!(err.contains("fora do diretório de sessões"), "{err}");
        assert!(victim.exists(), "arquivo fora do root não pode ser movido");
        assert!(
            !t.at(".pi-deck-trash").exists(),
            "nada pode ser criado fora do root"
        );
    }
}
