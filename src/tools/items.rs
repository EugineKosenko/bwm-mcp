use crate::client;
use bitwarden_vault::VaultClientExt;
use crate::tools::reply;

fn item_id(arguments: &serde_json::Value) -> Result<String, String> {
    arguments["id"].as_str().map(|id| id.to_string()).ok_or("Потрібен аргумент id.".to_string())
}
async fn get(bw: &client::Bw, arguments: &serde_json::Value) -> Result<String, String> {
    let id = item_id(arguments)?;
    let view = bw.vault().ciphers().get(&id).await.map_err(|error| error.to_string())?;

    Ok(serde_json::to_string(&view).unwrap())
}
async fn list(bw: &client::Bw) -> Result<String, String> {
    let result = bw.vault().ciphers().list().await.map_err(|error| error.to_string())?;

    Ok(serde_json::to_string(&result.successes).unwrap())
}
pub async fn run(bw: &client::Bw, arguments: &serde_json::Value) -> serde_json::Value {
    let result = match arguments["action"].as_str() {
        Some("list") => list(bw).await,
        Some("get") => get(bw, arguments).await,
        Some("create") | Some("edit") =>
            Err("items create/edit поки не реалізовано — див. main-tool-items.org.".to_string()),
        _ => Err("Потрібен аргумент action: list/get.".to_string()),
    };

    reply(result)
}
