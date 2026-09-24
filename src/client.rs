use bitwarden_core::Client;
use bitwarden_core::ClientSettings;
use std::env;
use bitwarden_auth::token_management::PasswordManagerTokenHandler;
use bitwarden_core::ClientBuilder;
use std::sync::Arc;
use bitwarden_core::auth::login::PasswordLoginRequest;
use bitwarden_core::FromClient;
use bitwarden_sync::{SyncClientExt, SyncRequest};
use bitwarden_vault::{CipherSyncHandler, FolderSyncHandler};

pub type Bw = Client;
fn settings(server: &str) -> ClientSettings {
    ClientSettings {
        identity_url: format!("{}/identity", server),
        api_url: format!("{}/api", server),
        ..Default::default()
    }
}
fn build(server: &str) -> Bw {
    ClientBuilder::new()
        .with_token_handler(Arc::new(PasswordManagerTokenHandler::default()))
        .with_settings(settings(server))
        .build()
}
fn required(name: &str) -> Result<String, String> {
    env::var(name).map_err(|_| format!("Змінну середовища {} не задано (див. .env.example).", name))
}

async fn login(bw: &Bw, email: String, password: String) -> Result<(), String> {
    let result = bw
        .auth()
        .login_password(&PasswordLoginRequest { email, password, two_factor: None })
        .await
        .map_err(|error| format!("Не вдалося увійти в Bitwarden: {}", error))?;

    if result.two_factor.is_some() {
        return Err("Акаунт вимагає двофакторну автентифікацію — bwm-mcp поки її не підтримує.".to_string());
    }

    Ok(())
}
async fn sync(bw: &Bw) -> Result<(), String> {
    let sync_client = bw.sync();
    sync_client.register_sync_handler(Arc::new(FolderSyncHandler::from_client(bw)));
    sync_client.register_sync_handler(Arc::new(CipherSyncHandler::from_client(bw)));

    sync_client
        .sync(SyncRequest { force: false, exclude_subdomains: None })
        .await
        .map_err(|error| format!("Не вдалося синхронізувати сейф: {}", error))?;

    Ok(())
}
pub async fn connect() -> Result<Bw, String> {
    let server = required("BWM_SERVER")?;
    let email = required("BWM_EMAIL")?;
    let password = required("BWM_PASSWD")?;

    let bw = build(&server);

    login(&bw, email, password).await?;
    sync(&bw).await?;

    Ok(bw)
}
