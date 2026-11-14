use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::thread;

use tauri::{AppHandle, Emitter, State};

pub struct ChildHandle {
    child: Child,
    stdin: ChildStdin,
}

#[derive(Default)]
pub struct RpcState(Mutex<HashMap<String, ChildHandle>>);

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

    let mut child = Command::new("pi")
        .args(["--mode", "rpc", ref_flag.as_str(), ref_value.as_str()])
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn pi: {e}"))?;

    let mut stdin = child.stdin.take().ok_or("sem stdin")?;
    // Boot: histórico, estado, modelos e níveis de thinking.
    for cmd in [
        "{\"id\":\"boot-messages\",\"type\":\"get_messages\"}",
        "{\"id\":\"boot-state\",\"type\":\"get_state\"}",
        "{\"id\":\"boot-models\",\"type\":\"get_available_models\"}",
        "{\"id\":\"boot-levels\",\"type\":\"get_available_thinking_levels\"}",
        "{\"id\":\"boot-stats\",\"type\":\"get_session_stats\"}",
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
) -> Result<(), String> {
    let mut map = state.0.lock().map_err(|e| e.to_string())?;
    let handle = map
        .get_mut(&session_id)
        .ok_or_else(|| format!("sessao '{session_id}' nao iniciada"))?;

    let command = format!(
        "{{\"id\":\"{}\",\"type\":\"prompt\",\"message\":\"{}\"}}\n",
        escape_json(&id),
        escape_json(&message)
    );
    handle
        .stdin
        .write_all(command.as_bytes())
        .map_err(|e| e.to_string())?;
    handle.stdin.flush().map_err(|e| e.to_string())
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
