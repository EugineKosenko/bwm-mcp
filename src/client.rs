use bitwarden_core::Client;
use bitwarden_core::client::persisted_state::BaseUrls;
use bitwarden_core::ClientSettings;
use std::env;
use bitwarden_auth::token_management::PasswordManagerTokenHandler;
use bitwarden_core::ClientBuilder;
use bitwarden_state::registry::StateRegistry;
use std::sync::Arc;
use crate::state;
use std::error::Error as StdError;
use bitwarden_core::FromClient;
use bitwarden_sync::{SyncClientExt, SyncRequest};
use bitwarden_vault::{CipherSyncHandler, FolderSyncHandler};
use crate::unlock;

pub type Bw = Client;
pub fn urls(server: &str) -> BaseUrls {
    BaseUrls { identity_url: format!("{}/identity", server), api_url: format!("{}/api", server) }
}

fn settings(server: &str) -> ClientSettings {
    let BaseUrls { identity_url, api_url } = urls(server);

    ClientSettings {
        identity_url,
        api_url,
        bitwarden_client_version: Some("2026.6.0".to_string()),
        ..Default::default()
    }
}
pub async fn build(server: &str) -> Result<Bw, String> {
    let registry: StateRegistry = state::registry(server).await?;

    Ok(ClientBuilder::new()
        .with_token_handler(Arc::new(PasswordManagerTokenHandler::default()))
        .with_settings(settings(server))
        .with_state(registry)
        .build())
}
pub fn describe(error: &(dyn StdError + 'static)) -> String {
    let mut message = error.to_string();
    let mut cause = error.source();

    while let Some(next) = cause {
        message.push_str(": ");
        message.push_str(&next.to_string());
        cause = next.source();
    }

    message
}
pub fn required(name: &str) -> Result<String, String> {
    env::var(name).map_err(|_| format!("Змінну середовища {} не задано (див. .env.example).", name))
}
pub async fn sync(bw: &Bw, force: bool) -> Result<(), String> {
    let sync_client = bw.sync();
    sync_client.register_sync_handler(Arc::new(FolderSyncHandler::from_client(bw)));
    sync_client.register_sync_handler(Arc::new(CipherSyncHandler::from_client(bw)));

    sync_client
        .sync(SyncRequest { force, exclude_subdomains: None })
        .await
        .map_err(|error| format!("Не вдалося синхронізувати сейф: {}", describe(&error)))?;

    Ok(())
}
pub async fn connect() -> Result<Bw, String> {
    let server = required("BW_SERVER")?;

    unlock::unlock(&server).await
}
