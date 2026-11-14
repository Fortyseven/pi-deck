use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

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

/// Conteúdo cru (JSONL) de um arquivo de sessão — usado para abrir o transcript de um child.
#[tauri::command]
pub fn read_session_messages(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))
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
