use crate::tools::reply;
use crate::client;
use bitwarden_generators::{GeneratorClientsExt, PasswordGeneratorRequest};
use bitwarden_generators::PassphraseGeneratorRequest;

fn flag(arguments: &serde_json::Value, name: &str) -> bool {
    arguments[name].as_bool().unwrap_or(false)
}
fn password(bw: &client::Bw, arguments: &serde_json::Value) -> Result<String, String> {
    let request = PasswordGeneratorRequest {
        lowercase: flag(arguments, "lowercase"),
        uppercase: flag(arguments, "uppercase"),
        numbers: flag(arguments, "number"),
        special: flag(arguments, "special"),
        length: arguments["length"].as_u64().unwrap_or(16) as u8,
        ..Default::default()
    };

    bw.generator().password(request).map_err(|error| error.to_string())
}
fn passphrase(bw: &client::Bw, arguments: &serde_json::Value) -> Result<String, String> {
    let request = PassphraseGeneratorRequest {
        num_words: arguments["words"].as_u64().unwrap_or(4) as u8,
        word_separator: arguments["separator"].as_str().unwrap_or("-").to_string(),
        capitalize: flag(arguments, "capitalize"),
        include_number: flag(arguments, "number"),
    };

    bw.generator().passphrase(request).map_err(|error| error.to_string())
}
pub fn run(bw: &client::Bw, arguments: &serde_json::Value) -> serde_json::Value {
    let result = if flag(arguments, "passphrase") {
        passphrase(bw, arguments)
    } else {
        password(bw, arguments)
    };

    reply(result)
}
