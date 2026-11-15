use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::thread;

use tauri::{AppHandle, Emitter, State};

use crate::sessions::{escape_json, pi_bin};

pub struct ChildHandle {
    child: Child,
    stdin: ChildStdin,
}

#[derive(Default)]
pub struct RpcState(Mutex<HashMap<String, ChildHandle>>);

impl RpcState {
    /// Encerra todos os processos `pi` filhos (usado ao fechar a janela).
    pub fn kill_all(&self) {
        if let Ok(mut map) = self.0.lock() {
            for (_, mut handle) in map.drain() {
                let _ = handle.child.kill();
                let _ = handle.child.wait();
            }
        }
    }

    /// Quantos processos estão vivos agora.
    pub fn running(&self) -> Vec<String> {
        self.0
            .lock()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// PATH do processo filho do `pi`.
/// Aberto pelo lançador gráfico, o app nasce com o PATH do `systemd --user`, que não tem
/// `~/.local/bin` nem os shims do mise — daí `spawn npx ENOENT` nos servidores MCP.
fn child_path() -> Option<std::ffi::OsString> {
    let home = std::path::PathBuf::from(std::env::var_os("HOME")?);
    let mut dirs: Vec<std::ffi::OsString> = [
        ".local/bin",
        ".local/share/mise/shims",
        ".local/share/pi-node/current/bin",
        ".pi/agent/bin",
    ]
    .iter()
    .map(|rel| home.join(rel))
    .filter(|d| d.is_dir())
    .map(|d| d.into_os_string())
    .collect();
    if dirs.is_empty() {
        return None;
    }
    let current = std::env::var_os("PATH").unwrap_or_default();
    dirs.extend(std::env::split_paths(&current).map(|p| p.into_os_string()));
    std::env::join_paths(dirs).ok()
}

/// Uma sessao = um processo `pi --mode rpc` (padrao do pi-desktop).
#[tauri::command]
pub fn start_session(
    app: AppHandle,
    state: State<'_, RpcState>,
    session_id: String,
    cwd: String,
    ref_flag: String,
    ref_value: String,
) -> Result<String, String> {
    {
        let map = state.0.lock().map_err(|e| e.to_string())?;
        if map.contains_key(&session_id) {
            return Ok(session_id);
        }
    }

    let mut cmd = Command::new(pi_bin());
    cmd.args(["--mode", "rpc", ref_flag.as_str(), ref_value.as_str()])
        .env("PI_DECK_CHILD", "1")
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Servidores MCP e o bash tool do pi herdam este PATH. Sem os shims do mise
    // (fora do PATH do systemd --user) o `pi mcp` morre com "spawn npx ENOENT".
    if let Some(path) = child_path() {
        cmd.env("PATH", path);
    }
    let mut child = cmd.spawn().map_err(|e| format!("spawn pi: {e}"))?;

    let mut stdin = child.stdin.take().ok_or("sem stdin")?;
    // Boot: histórico, estado, modelos e níveis de thinking.
    for cmd in [
        "{\"id\":\"boot-messages\",\"type\":\"get_messages\"}",
        "{\"id\":\"boot-state\",\"type\":\"get_state\"}",
        "{\"id\":\"boot-models\",\"type\":\"get_available_models\"}",
        "{\"id\":\"boot-levels\",\"type\":\"get_available_thinking_levels\"}",
        "{\"id\":\"boot-stats\",\"type\":\"get_session_stats\"}",
        "{\"id\":\"boot-commands\",\"type\":\"get_commands\"}",
    ] {
        let _ = stdin.write_all(format!("{cmd}\n").as_bytes());
    }
    let _ = stdin.flush();
    let stdout = child.stdout.take().ok_or("sem stdout")?;
    let stderr = child.stderr.take().ok_or("sem stderr")?;

    let sid_out = session_id.clone();
    let app_out = app.clone();
    let debug = std::env::var("PWUI_DEBUG").is_ok();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if debug {
                let head: String = line.chars().take(150).collect();
                eprintln!("[rpc] {head}");
            }
            let _ = app_out.emit("rpc-line", (sid_out.clone(), line));
        }
        let _ = app_out.emit("rpc-exit", sid_out);
    });

    let sid_err = session_id.clone();
    let app_err = app.clone();
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if std::env::var("PWUI_DEBUG").is_ok() {
                eprintln!("[rpc stderr] {line}");
            }
            let _ = app_err.emit("rpc-stderr", (sid_err.clone(), line));
        }
    });

    state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(session_id.clone(), ChildHandle { child, stdin });

    Ok(session_id)
}

#[tauri::command]
pub fn send_prompt(
    state: State<'_, RpcState>,
    session_id: String,
    message: String,
    id: String,
    // Pares (base64, mimeType) — formato ImageContent do RPC do pi.
    images: Option<Vec<(String, String)>>,
) -> Result<(), String> {
    let mut map = state.0.lock().map_err(|e| e.to_string())?;
    let handle = map
        .get_mut(&session_id)
        .ok_or_else(|| format!("sessao '{session_id}' nao iniciada"))?;

    let images_json = match images.as_deref() {
        Some(list) if !list.is_empty() => {
            let items: Vec<String> = list
                .iter()
                .map(|(data, mime)| {
                    format!(
                        "{{\"type\":\"image\",\"data\":\"{}\",\"mimeType\":\"{}\"}}",
                        escape_json(data),
                        escape_json(mime)
                    )
                })
                .collect();
            format!(",\"images\":[{}]", items.join(","))
        }
        _ => String::new(),
    };

    let command = format!(
        "{{\"id\":\"{}\",\"type\":\"prompt\",\"message\":\"{}\"{images_json}}}\n",
        escape_json(&id),
        escape_json(&message)
    );
    handle
        .stdin
        .write_all(command.as_bytes())
        .map_err(|e| e.to_string())?;
    handle.stdin.flush().map_err(|e| e.to_string())
}

/// Quantos processos `pi` filhos estão vivos (visibilidade do custo).
#[tauri::command]
pub fn running_sessions(state: State<'_, RpcState>) -> Vec<String> {
    state.running()
}

/// Comando RPC genérico: o frontend manda o JSON pronto (set_model, set_thinking_level, etc.).
#[tauri::command]
pub fn rpc_command(
    state: State<'_, RpcState>,
    session_id: String,
    payload: String,
) -> Result<(), String> {
    let mut map = state.0.lock().map_err(|e| e.to_string())?;
    let handle = map
        .get_mut(&session_id)
        .ok_or_else(|| format!("sessao '{session_id}' nao iniciada"))?;
    handle
        .stdin
        .write_all(format!("{payload}\n").as_bytes())
        .map_err(|e| e.to_string())?;
    handle.stdin.flush().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn stop_session(state: State<'_, RpcState>, session_id: String) -> Result<(), String> {
    let mut map = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(mut handle) = map.remove(&session_id) {
        let _ = handle.child.kill();
        let _ = handle.child.wait();
    }
    Ok(())
}
