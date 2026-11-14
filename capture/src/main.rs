//! M0: prova de protocolo.
//! Spawna `pi --mode rpc`, dispara um subagent ASYNC e captura:
//!   - todo `extension_ui_request` (dialogs + setStatus/setWidget)
//!   - o payload `PI_SUBAGENT_ASYNC_JSON:` cru
//! Sem dependencias externas: so std.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const LOG_PATH: &str = "/tmp/m0-rpc.jsonl";
const WIDGET_PREFIX: &str = "PI_SUBAGENT_ASYNC_JSON:";
const MAX_SECONDS: u64 = 240;

fn json_escape(s: &str) -> String {
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

/// Extrai o valor de "type" de um record JSONL sem parser completo.
fn record_type(line: &str) -> &str {
    let key = "\"type\":\"";
    let Some(start) = line.find(key) else {
        return "unknown";
    };
    let rest = &line[start + key.len()..];
    match rest.find('"') {
        Some(end) => &rest[..end],
        None => "unknown",
    }
}

/// Extrai o valor de um campo string simples.
fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("\"{}\":\"", name);
    let start = line.find(&key)?;
    let rest = &line[start + key.len()..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn main() {
    let cwd = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".".to_string());

    println!("[m0] cwd do child : {cwd}");
    println!("[m0] log completo: {LOG_PATH}");

    let mut child = Command::new("pi")
        .args(["--mode", "rpc", "--session-id", "m0-capture"])
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("falha ao spawnar `pi` (esta no PATH?)");

    let (tx, rx) = mpsc::channel::<String>();

    let stdout = child.stdout.take().expect("stdout");
    let tx_out = tx.clone();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(l) => {
                    if tx_out.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let stderr = child.stderr.take().expect("stderr");
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            eprintln!("[child stderr] {line}");
        }
    });

    let prompt = "Chame o tool `subagent` com estes argumentos exatos: \
agent=\"scout\", task=\"Responda apenas a palavra OK e nada mais.\", async=true. \
Nao use nenhum outro tool. Depois de disparar, responda somente: disparado.";

    let command = format!(
        "{{\"id\":\"m0-prompt\",\"type\":\"prompt\",\"message\":\"{}\"}}\n",
        json_escape(prompt)
    );

    let mut stdin = child.stdin.take().expect("stdin");
    if let Err(e) = stdin.write_all(command.as_bytes()) {
        eprintln!("[m0] falha ao enviar prompt: {e}");
    }
    let _ = stdin.flush();

    let mut raw = File::create(LOG_PATH).expect("criar log");
    let started = Instant::now();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut widgets = 0usize;
    let mut statuses = 0usize;
    let mut dialogs = 0usize;
    let mut settled = 0usize;

    while started.elapsed() < Duration::from_secs(MAX_SECONDS) {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(line) => {
                let _ = writeln!(raw, "{line}");

                let ty = record_type(&line).to_string();
                *counts.entry(ty.clone()).or_insert(0) += 1;

                if line.contains(WIDGET_PREFIX) {
                    widgets += 1;
                    println!("\n[WIDGET #{widgets}] ---------------------------");
                    match line.find(WIDGET_PREFIX) {
                        Some(idx) => println!("{}", &line[idx..]),
                        None => println!("{line}"),
                    }
                    println!("-------------------------------------------\n");
                } else if ty == "extension_ui_request" {
                    let method = field(&line, "method").unwrap_or("?");
                    match method {
                        "setWidget" => widgets += 1,
                        "setStatus" => statuses += 1,
                        _ => dialogs += 1,
                    }
                    let short = if line.len() > 400 { &line[..400] } else { &line };
                    println!("[ui] method={method} :: {short}");
                } else if ty == "tool_execution_start" {
                    let tool = field(&line, "toolName").unwrap_or("?");
                    println!("[tool] start {tool}");
                } else if ty == "agent_settled" {
                    settled += 1;
                }

                if settled > 0 && widgets > 0 && started.elapsed() > Duration::from_secs(20) {
                    println!("[m0] widget capturado e run estabilizado, encerrando.");
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if settled > 0 && widgets > 0 {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = child.kill();
    let _ = child.wait();

    println!("\n=== RESUMO M0 ===");
    println!("duracao        : {:?}", started.elapsed());
    println!("widgets        : {widgets}");
    println!("setStatus      : {statuses}");
    println!("outros ui      : {dialogs}");
    println!("log completo   : {LOG_PATH}");
    println!("records por tipo:");
    for (ty, n) in counts {
        println!("  {n:>5}  {ty}");
    }
}
