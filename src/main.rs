mod client;
mod state;
mod login;
mod unlock;
mod tools;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

fn require_url(command: &str) -> String {
    std::env::args().nth(2).unwrap_or_else(|| {
        eprintln!("Потрібен url сервера: bwm-mcp {command} <url>");
        std::process::exit(1);
    })
}

fn print_help() {
    println!("bwm-mcp — MCP-сервер для Bitwarden/Vaultwarden (json-rpc над stdio).");
    println!();
    println!("Використання:");
    println!("  bwm-mcp                запустити сервер (звичайний режим mcp-клієнта)");
    println!("  bwm-mcp login <url>    увійти на сервер url (пошта/майстер-пароль інтерактивно), зберегти стан");
    println!("  bwm-mcp logout <url>   видалити збережений стан (session-key, state.sqlite3) для сервера url");
    println!();
    println!("Звичайний старт сервера мережево НЕ логіниться — лише локально розблоковує вже збережений");
    println!("стан (unlock.org) для сервера з BW_SERVER. Немає збереженого стану для нього — спершу");
    println!("виконайте `bwm-mcp login <url>` вручну з тим самим url.");
    println!();
    println!("Стан окремий на кожен сервер (url) — можна входити на кілька різних Vaultwarden незалежно,");
    println!("кожен проєкт у власному .mcp.json вказує BW_SERVER того сервера, з яким має працювати.");
    println!();
    println!("Змінна середовища (обов'язкова для звичайного старту сервера):");
    println!("  BW_SERVER   базова адреса Vaultwarden без кінцевого слеша (напр. https://localhost:8443)");
    println!();
    println!("Довіри до самопідписаного сертифіката сервера (TLS) поки не реалізовано (див. env.org) —");
    println!("самопідписаний сертифікат призведе до помилки під час входу.");
    println!();
    println!("Локальний стан — ~/.local/share/bwm-mcp/<сервер>/ (state.sqlite3, session-key, права 600),");
    println!("окремо на кожен url сервера.");
    println!("Помилка старту додатково пишеться в ~/.local/state/bwm-mcp/last-error.log — mcp-клієнт");
    println!("(Claude Code) не показує stderr невдалого підключення, лише \"connection closed\".");
}
fn log_startup_error(message: &str) {
    let hint = if message.contains("429") {
        " Це, найімовірніше, rate-limit входу Vaultwarden \
          (LOGIN_RATELIMIT_MAX_BURST/LOGIN_RATELIMIT_SECONDS) — часті реконекти вичерпують ліміт."
    } else {
        ""
    };

    let Ok(home) = std::env::var("HOME") else { return };
    let dir = std::path::Path::new(&home).join(".local/state/bwm-mcp");

    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(dir.join("last-error.log"), format!("{message}{hint}\n"));
    }
}
async fn dispatch(bw: Arc<client::Bw>, method: String, params: serde_json::Value) -> Result<serde_json::Value, (i64, String)> {
    match method.as_str() {
        "initialize" => Ok(serde_json::json!({
            "protocolVersion": params["protocolVersion"].as_str().unwrap_or("2024-11-05"),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "bwm-mcp", "version": env!("CARGO_PKG_VERSION") },
        })),
        "notifications/initialized" | "ping" => Ok(serde_json::json!({})),
        "tools/list" => Ok(serde_json::json!({ "tools": tools::list() })),
        "tools/call" => {
            let name = params["name"].as_str().unwrap();
            let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
            Ok(tools::call(&bw, name, &arguments).await)
        }
        _ => Err((-32601, format!("Метод не підтримується: {}", method))),
    }
}
async fn write_response(stdout: &mut tokio::io::Stdout, response: &serde_json::Value) {
    let text = serde_json::to_string(response).unwrap();
    stdout.write_all(text.as_bytes()).await.unwrap();
    stdout.write_all(b"\n").await.unwrap();
    stdout.flush().await.unwrap();
}

#[tokio::main]
async fn main() {
    if std::env::args().skip(1).any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing_subscriber::filter::LevelFilter::ERROR)
        .init();
    dotenvy::dotenv().ok();
    client::init_platform();

    if std::env::args().nth(1).as_deref() == Some("login") {
        let server = require_url("login");

        match login::run(&server).await {
            Ok(message) => println!("{message}"),
            Err(message) => {
                eprintln!("{message}");
                std::process::exit(1);
            }
        }
        return;
    }

    if std::env::args().nth(1).as_deref() == Some("logout") {
        let server = require_url("logout");

        match state::logout(&server) {
            Ok(message) => println!("{message}"),
            Err(message) => {
                eprintln!("{message}");
                std::process::exit(1);
            }
        }
        return;
    }

    let bw = match client::connect().await {
        Ok(bw) => Arc::new(bw),
        Err(message) => {
            log_startup_error(&message);
            eprintln!("{}", message);
            std::process::exit(1);
        }
    };

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await.unwrap() {
        if line.trim().is_empty() {
            continue;
        }
        
        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": serde_json::Value::Null,
                    "error": { "code": -32700, "message": format!("Не вдалося розібрати JSON: {}", error) },
                });
                write_response(&mut stdout, &response).await;
                continue;
            }
        };
        
        let id = request.get("id").cloned();
        let method = request["method"].as_str().unwrap_or("").to_string();
        let params = request.get("params").cloned().unwrap_or(serde_json::json!({}));
        
        match id {
            None => {
                tokio::spawn(dispatch(bw.clone(), method, params));
            }
            Some(id) => {
                let outcome = tokio::spawn(dispatch(bw.clone(), method, params)).await;
        
                let response = match outcome {
                    Ok(Ok(result)) => serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                    Ok(Err((code, message))) =>
                        serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }),
                    Err(_panic) => serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32603, "message": "Внутрішня помилка сервера під час обробки запиту." },
                    }),
                };
        
                write_response(&mut stdout, &response).await;
            }
        }
    }
}
