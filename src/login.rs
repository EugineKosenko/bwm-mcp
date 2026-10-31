use crate::client;
use crate::state;
use bitwarden_core::auth::JwtToken;
use bitwarden_core::auth::login::PasswordLoginRequest;
use bitwarden_core::client::persisted_state::AUTHENTICATION_TOKENS;
use bitwarden_core::UserId;
use bitwarden_unlock::UnlockClientExt;
use std::io::Write;

fn prompt_email() -> Result<String, String> {
    print!("Пошта: ");
    std::io::stdout().flush().map_err(|e| format!("не вдалося вивести підказку: {e}"))?;

    let mut line = String::new();
    std::io::stdin().read_line(&mut line).map_err(|e| format!("не вдалося прочитати email: {e}"))?;

    Ok(line.trim().to_string())
}

pub async fn run(server: &str) -> Result<String, String> {
    let email = prompt_email()?;
    let password = rpassword::prompt_password("Майстер-пароль: ")
        .map_err(|e| format!("не вдалося прочитати пароль: {e}"))?;

    let bw = client::build(server).await?;

    let result = bw
        .auth()
        .login_password(&PasswordLoginRequest { email: email.clone(), password, two_factor: None })
        .await
        .map_err(|error| format!("Не вдалося увійти в Bitwarden: {}", client::describe(&error)))?;

    if result.two_factor.is_some() {
        return Err("Акаунт вимагає двофакторну автентифікацію — bwm-mcp поки її не підтримує.".to_string());
    }

    client::sync(&bw, false).await?;

    let user_id = user_id(&bw).await?;
    state::save(&bw, user_id, email, server).await?;

    let session_key = bw
        .unlock()
        .generate_session_key()
        .await
        .map_err(|error| format!("Не вдалося створити сесійний ключ: {error}"))?;

    state::save_session_key(server, &session_key)?;

    Ok("Вхід виконано.".to_string())
}
async fn user_id(bw: &client::Bw) -> Result<UserId, String> {
    let tokens = bw
        .platform()
        .state()
        .setting(AUTHENTICATION_TOKENS)
        .map_err(|e| format!("не вдалося прочитати токени входу: {e}"))?
        .get()
        .await
        .map_err(|e| format!("не вдалося прочитати токени входу: {e}"))?
        .ok_or("вхід не зберіг токени автентифікації".to_string())?;

    let claims: JwtToken = tokens
        .access_token
        .parse()
        .map_err(|e| format!("не вдалося розібрати токен доступу: {e}"))?;

    claims.sub.parse().map_err(|_| "не вдалося розібрати ідентифікатор користувача".to_string())
}
