mod client;
mod tools;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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
    dotenvy::dotenv().ok();
    let bw = match client::connect().await {
        Ok(bw) => Arc::new(bw),
        Err(message) => {
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
