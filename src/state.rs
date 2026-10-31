use std::path::PathBuf;
use bitwarden_core::client::persisted_state::OrganizationSharedKey;
use bitwarden_send::Send;
use bitwarden_state::registry::StateRegistry;
use bitwarden_state::repository::{RepositoryItem, RepositoryMigrationStep, RepositoryMigrations};
use bitwarden_state::{DatabaseConfiguration, SettingItem};
use bitwarden_vault::{Cipher, Folder};
use bitwarden_core::client::SaveStateData;
use bitwarden_core::key_management::account_cryptographic_state::WrappedAccountCryptographicState;
use bitwarden_core::{Client, UserId};
use crate::client::Bw;
use std::os::unix::fs::PermissionsExt;
use std::str::FromStr;
use bitwarden_unlock::SessionKey;

fn server_dir_name(server: &str) -> String {
    server.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect()
}

fn server_path(server: &str) -> Result<PathBuf, String> {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .map_err(|_| "не вдалося визначити домашній каталог (XDG_DATA_HOME/HOME)".to_string())?;

    Ok(base.join("bwm-mcp").join(server_dir_name(server)))
}

fn dir(server: &str) -> Result<PathBuf, String> {
    let dir = server_path(server)?;

    std::fs::create_dir_all(&dir).map_err(|e| format!("не вдалося створити {}: {e}", dir.display()))?;

    Ok(dir)
}
fn migrations() -> RepositoryMigrations {
    use RepositoryMigrationStep::Add;

    RepositoryMigrations::new(vec![
        Add(Cipher::data()),
        Add(Folder::data()),
        Add(SettingItem::data()),
        Add(OrganizationSharedKey::data()),
        Add(Send::data()),
    ])
}

pub async fn registry(server: &str) -> Result<StateRegistry, String> {
    let configuration = DatabaseConfiguration::Sqlite {
        db_name: "state".to_string(),
        folder_path: dir(server)?,
    };

    StateRegistry::new_with_db(configuration, migrations())
        .await
        .map_err(|e| format!("не вдалося відкрити локальний стан: {e}"))
}
pub async fn save(
    bw: &Bw,
    user_id: UserId,
    email: String,
    server: &str,
) -> Result<(), String> {
    let crypto_state = WrappedAccountCryptographicState::get_from_key_store(&bw.internal.get_key_store().context())
        .map_err(|e| format!("не вдалося прочитати криптостан акаунта: {e}"))?;

    let data = SaveStateData { user_id, email, urls: crate::client::urls(server), crypto_state };

    Client::save_to_state(data, &registry(server).await?)
        .await
        .map_err(|e| format!("не вдалося зберегти стан входу: {e}"))
}
fn session_key_path(server: &str) -> Result<PathBuf, String> {
    Ok(dir(server)?.join("session-key"))
}

pub fn save_session_key(server: &str, key: &SessionKey) -> Result<(), String> {
    let path = session_key_path(server)?;

    std::fs::write(&path, key.to_string()).map_err(|e| format!("не вдалося записати {}: {e}", path.display()))?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("не вдалося встановити права 600 на {}: {e}", path.display()))
}

pub fn load_session_key(server: &str) -> Result<SessionKey, String> {
    let path = session_key_path(server)?;
    let text = std::fs::read_to_string(&path)
        .map_err(|_| format!("локальний стан для {server} відсутній — спершу виконайте `bwm-mcp login {server}`"))?;

    SessionKey::from_str(text.trim())
        .map_err(|e| format!("пошкоджений файл сесійного ключа {}: {e}", path.display()))
}
pub fn logout(server: &str) -> Result<String, String> {
    let path = server_path(server)?;

    if !path.exists() {
        return Ok(format!("Немає збереженого стану для {server}."));
    }

    std::fs::remove_dir_all(&path).map_err(|e| format!("не вдалося видалити {}: {e}", path.display()))?;

    Ok(format!("Вихід виконано для {server}."))
}
