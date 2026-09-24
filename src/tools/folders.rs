use bitwarden_vault::FolderId;
use crate::client;
use bitwarden_vault::VaultClientExt;
use bitwarden_vault::FolderAddEditRequest;
use crate::tools::reply;

fn folder_id(arguments: &serde_json::Value) -> Result<FolderId, String> {
    arguments["id"]
        .as_str()
        .ok_or("Потрібен аргумент id.".to_string())?
        .parse()
        .map_err(|_| "Некоректний id теки.".to_string())
}

fn folder_name(arguments: &serde_json::Value) -> Result<String, String> {
    arguments["name"].as_str().map(|name| name.to_string()).ok_or("Потрібен аргумент name.".to_string())
}
async fn list(bw: &client::Bw) -> Result<String, String> {
    let folders = bw.vault().folders().list().await.map_err(|error| error.to_string())?;

    Ok(serde_json::to_string(&folders).unwrap())
}
async fn get(bw: &client::Bw, arguments: &serde_json::Value) -> Result<String, String> {
    let id = folder_id(arguments)?;
    let folder = bw.vault().folders().get(id).await.map_err(|error| error.to_string())?;

    Ok(serde_json::to_string(&folder).unwrap())
}
async fn create(bw: &client::Bw, arguments: &serde_json::Value) -> Result<String, String> {
    let name = folder_name(arguments)?;
    let folder = bw.vault().folders().create(FolderAddEditRequest { name }).await.map_err(|error| error.to_string())?;

    Ok(serde_json::to_string(&folder).unwrap())
}
async fn edit(bw: &client::Bw, arguments: &serde_json::Value) -> Result<String, String> {
    let id = folder_id(arguments)?;
    let name = folder_name(arguments)?;
    let folder = bw.vault().folders().edit(id, FolderAddEditRequest { name }).await.map_err(|error| error.to_string())?;

    Ok(serde_json::to_string(&folder).unwrap())
}
pub async fn run(bw: &client::Bw, arguments: &serde_json::Value) -> serde_json::Value {
    let result = match arguments["action"].as_str() {
        Some("list") => list(bw).await,
        Some("get") => get(bw, arguments).await,
        Some("create") => create(bw, arguments).await,
        Some("edit") => edit(bw, arguments).await,
        _ => Err("Потрібен аргумент action: list/get/create/edit.".to_string()),
    };

    reply(result)
}
