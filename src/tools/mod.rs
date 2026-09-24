use crate::client;
mod generate;
mod folders;

pub fn list() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "generate",
            "description": "Згенерувати надійний пароль чи passphrase. Без passphrase=true генерує пароль (потрібна хоча б одна з lowercase/uppercase/number/special); з passphrase=true — фразу зі слів.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "passphrase": { "type": "boolean", "description": "Генерувати passphrase (фразу зі слів) замість пароля" },
                    "length": { "type": "integer", "minimum": 5, "description": "Довжина пароля (лише для пароля, не passphrase)" },
                    "lowercase": { "type": "boolean" },
                    "uppercase": { "type": "boolean" },
                    "number": { "type": "boolean" },
                    "special": { "type": "boolean" },
                    "words": { "type": "integer", "description": "Кількість слів passphrase (3–20)" },
                    "separator": { "type": "string", "description": "Роздільник слів passphrase" },
                    "capitalize": { "type": "boolean", "description": "Велика перша літера кожного слова passphrase" }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "folders",
            "description": "Робота з теками сейфа: list (усі теки), get (одна тека за id), create (нова тека, потрібне name), edit (перейменувати теку, потрібні id і name).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "get", "create", "edit"] },
                    "id": { "type": "string", "description": "ID теки (для get/edit)" },
                    "name": { "type": "string", "description": "Назва теки (для create/edit)" }
                },
                "required": ["action"],
                "additionalProperties": false
            }
        }
    ])
}

pub async fn call(bw: &client::Bw, name: &str, arguments: &serde_json::Value) -> serde_json::Value {
    match name {
        "generate" => generate::run(bw, arguments),
        "folders" => folders::run(bw, arguments).await,
        _ => reply(Err(format!("Невідомий інструмент: {}", name)))
    }
}

pub fn reply(result: Result<String, String>) -> serde_json::Value {
    match result {
        Ok(text) => serde_json::json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        Err(text) => serde_json::json!({ "content": [{ "type": "text", "text": text }], "isError": true }),
    }
}
