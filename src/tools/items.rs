use crate::client;
use bitwarden_vault::VaultClientExt;
use bitwarden_generators::{GeneratorClientsExt, PasswordGeneratorRequest};
use bitwarden_vault::{
    CipherCreateRequest, CipherRepromptType, CipherViewType, LoginUriView, LoginView,
    SecureNoteType, SecureNoteView, UriMatchType,
};
use bitwarden_vault::CipherEditRequest;
use crate::tools::reply;

fn item_id(arguments: &serde_json::Value) -> Result<String, String> {
    arguments["id"].as_str().map(|id| id.to_string()).ok_or("Потрібен аргумент id.".to_string())
}
fn redact(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(fields) => {
            fields.remove("password");
            fields.remove("totp");

            for field in fields.values_mut() {
                redact(field);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact(item);
            }
        }
        _ => {}
    }
}
async fn get(bw: &client::Bw, arguments: &serde_json::Value) -> Result<String, String> {
    let id = item_id(arguments)?;
    let view = bw.vault().ciphers().get(&id).await.map_err(|error| error.to_string())?;
    let mut value = serde_json::to_value(&view).unwrap();
    redact(&mut value);

    Ok(value.to_string())
}
async fn list(bw: &client::Bw) -> Result<String, String> {
    let result = bw.vault().ciphers().list().await.map_err(|error| error.to_string())?;
    let mut value = serde_json::to_value(&result.successes).unwrap();
    redact(&mut value);

    Ok(value.to_string())
}
fn uri_match(value: &serde_json::Value) -> Option<UriMatchType> {
    match value.as_i64() {
        Some(0) => Some(UriMatchType::Domain),
        Some(1) => Some(UriMatchType::Host),
        Some(2) => Some(UriMatchType::StartsWith),
        Some(3) => Some(UriMatchType::Exact),
        Some(4) => Some(UriMatchType::RegularExpression),
        Some(5) => Some(UriMatchType::Never),
        _ => None,
    }
}

fn uris(arguments: &serde_json::Value) -> Option<Vec<LoginUriView>> {
    let array = arguments["login"]["uris"].as_array()?;

    Some(
        array
            .iter()
            .map(|item| LoginUriView {
                uri: item["uri"].as_str().map(|uri| uri.to_string()),
                r#match: uri_match(&item["match"]),
                uri_checksum: None,
            })
            .collect(),
    )
}

fn login_view(arguments: &serde_json::Value) -> LoginView {
    LoginView {
        username: arguments["login"]["username"].as_str().map(|username| username.to_string()),
        password: None,
        password_revision_date: None,
        uris: uris(arguments),
        totp: None,
        autofill_on_page_load: None,
        fido2_credentials: None,
    }
}

fn cipher_type(arguments: &serde_json::Value) -> Result<CipherViewType, String> {
    match arguments["type"].as_i64() {
        Some(1) => Ok(CipherViewType::Login(login_view(arguments))),
        Some(2) => Ok(CipherViewType::SecureNote(SecureNoteView { r#type: SecureNoteType::Generic })),
        _ => Err("Потрібен аргумент type: 1 (login) або 2 (secureNote).".to_string()),
    }
}

fn generate_password(
    bw: &client::Bw,
    arguments: &serde_json::Value,
    cipher_type: &mut CipherViewType,
) -> Result<(), String> {
    if !arguments["login"]["generatePassword"].as_bool().unwrap_or(false) {
        return Ok(());
    }

    let CipherViewType::Login(login) = cipher_type else {
        return Err("login.generatePassword можливий лише для type=1 (login).".to_string());
    };

    let request = PasswordGeneratorRequest {
        lowercase: true,
        uppercase: true,
        numbers: true,
        special: false,
        length: 20,
        ..Default::default()
    };

    login.password = Some(bw.generator().password(request).map_err(|error| error.to_string())?);

    Ok(())
}

async fn create(bw: &client::Bw, arguments: &serde_json::Value) -> Result<String, String> {
    let name = arguments["name"].as_str().ok_or("Потрібен аргумент name.".to_string())?.to_string();
    let mut r#type = cipher_type(arguments)?;
    generate_password(bw, arguments, &mut r#type)?;
    let folder_id = match arguments["folderId"].as_str() {
        Some(id) => Some(id.parse().map_err(|_| "Некоректний folderId.".to_string())?),
        None => None,
    };

    let request = CipherCreateRequest {
        organization_id: None,
        collection_ids: vec![],
        folder_id,
        name,
        notes: arguments["notes"].as_str().map(|notes| notes.to_string()),
        favorite: false,
        reprompt: CipherRepromptType::default(),
        r#type,
        fields: vec![],
        archived_date: None,
    };

    let created = bw.vault().ciphers().create(request).await.map_err(|error| error.to_string())?;
    let mut value = serde_json::to_value(&created).unwrap();
    redact(&mut value);

    Ok(value.to_string())
}
async fn edit(bw: &client::Bw, arguments: &serde_json::Value) -> Result<String, String> {
    let id = item_id(arguments)?;
    let view = bw.vault().ciphers().get(&id).await.map_err(|error| error.to_string())?;
    let mut request = CipherEditRequest::try_from(view).map_err(|error| error.to_string())?;

    if let Some(folder_id) = arguments["folderId"].as_str() {
        request.folder_id = Some(folder_id.parse().map_err(|_| "Некоректний folderId.".to_string())?);
    }
    if let Some(name) = arguments["name"].as_str() {
        request.name = name.to_string();
    }
    if let Some(notes) = arguments["notes"].as_str() {
        request.notes = Some(notes.to_string());
    }

    let updated = bw.vault().ciphers().edit(request).await.map_err(|error| error.to_string())?;
    let mut value = serde_json::to_value(&updated).unwrap();
    redact(&mut value);

    Ok(value.to_string())
}
async fn rotate_secret(bw: &client::Bw, arguments: &serde_json::Value) -> Result<String, String> {
    let id = item_id(arguments)?;
    let view = bw.vault().ciphers().get(&id).await.map_err(|error| error.to_string())?;
    let mut request = CipherEditRequest::try_from(view).map_err(|error| error.to_string())?;

    let CipherViewType::Login(login) = &mut request.r#type else {
        return Err("rotate_secret можливий лише для елементів login.".to_string());
    };

    let password = arguments["secret"]["password"].as_str();
    let totp = arguments["secret"]["totp"].as_str();

    if password.is_none() && totp.is_none() {
        return Err("Потрібен хоча б secret.password чи secret.totp.".to_string());
    }
    if let Some(password) = password {
        login.password = Some(password.to_string());
    }
    if let Some(totp) = totp {
        login.totp = Some(totp.to_string());
    }

    let updated = bw.vault().ciphers().edit(request).await.map_err(|error| error.to_string())?;

    Ok(serde_json::to_string(&updated).unwrap())
}
pub async fn run(bw: &client::Bw, arguments: &serde_json::Value) -> serde_json::Value {
    let result = match arguments["action"].as_str() {
        Some("list") => list(bw).await,
        Some("get") => get(bw, arguments).await,
        Some("create") => create(bw, arguments).await,
        Some("edit") => edit(bw, arguments).await,
        Some("rotate_secret") => rotate_secret(bw, arguments).await,
        _ => Err("Потрібен аргумент action: list/get/create/edit/rotate_secret.".to_string()),
    };

    reply(result)
}
