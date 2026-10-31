use crate::client;
use crate::state;
use bitwarden_unlock::{UnlockClientExt, UnlockMethod};

pub async fn unlock(server: &str) -> Result<client::Bw, String> {
    let bw = client::build(server).await?;
    let session_key = state::load_session_key(server)?;

    bw.unlock()
        .unlock(UnlockMethod::SessionKey(session_key))
        .await
        .map_err(|error| format!("Не вдалося розблокувати локальний стан: {error}"))?;

    client::sync(&bw, false).await?;

    Ok(bw)
}
