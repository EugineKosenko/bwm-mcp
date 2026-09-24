use crate::client;
mod generate;
mod folders;
mod items;

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
        },
        {
            "name": "items",
            "description": "Робота з елементами сейфа: list (усі елементи), get (повний обʼєкт за id), create (login чи secureNote, потрібні name і type), edit (змінити folderId/name/notes елемента за id, зберігши решту полів — login, fields тощо — без змін).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "get", "create", "edit"] },
                    "id": { "type": "string", "description": "ID елемента (для get/edit)" },
                    "type": { "type": "integer", "enum": [1, 2], "description": "1 — login, 2 — secureNote (для create)" },
                    "name": { "type": "string", "description": "Назва (для create/edit)" },
                    "notes": { "type": "string", "description": "Нотатки (для create/edit)" },
                    "folderId": { "type": "string", "description": "Тека (для create/edit)" },
                    "login": {
                        "type": "object",
                        "description": "Дані для type=1 (login)",
                        "properties": {
                            "username": { "type": "string" },
                            "password": { "type": "string" },
                            "totp": { "type": "string" },
                            "uris": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "uri": { "type": "string" },
                                        "match": { "type": "integer", "enum": [0, 1, 2, 3, 4, 5], "description": "0 Domain, 1 Host, 2 StartsWith, 3 Exact, 4 RegularExpression, 5 Never" }
                                    }
                                }
                            }
                        }
                    }
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
        "items" => items::run(bw, arguments).await,
        _ => reply(Err(format!("Невідомий інструмент: {}", name)))
    }
}

pub fn reply(result: Result<String, String>) -> serde_json::Value {
    match result {
        Ok(text) => serde_json::json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        Err(text) => serde_json::json!({ "content": [{ "type": "text", "text": text }], "isError": true }),
    }
}
