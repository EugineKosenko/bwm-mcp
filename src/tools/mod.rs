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
            "description": "Робота з елементами сейфа, без прямого доступу до секретів (password, totp, sshKey.privateKey не повертаються): list (елементи; необовʼязкові фільтри name — підрядок, folderId; для sshKey login у відповіді не показується — CipherListView структурно не має fields, лише get), sync (повторна синхронізація з сервером — елементи, створені деінде після старту, інакше не видно), get (обʼєкт за id; для sshKey — окреме поле login), create (login, secureNote чи sshKey, потрібні name і type; пароль лише через login.generatePassword, ключ ssh для type=5 генерується самим сервером — готове значення передати не можна; для sshKey додатково обовʼязковий sshKey.login), edit (змінити folderId/name/notes; для sshKey — sshKey.login опціонально оновлює збережений логін, не чіпає сам ключ), rotate_secret (єдина дія, що встановлює нове значення секрету для login чи sshKey, ротація на місці без другої картки: secret.password — готове значення, caller сам щойно передав його, не приховується; secret.generatePassword — генерація на сервері з опціями length/lowercase/uppercase/number/special, значення не повертається; secret.totp — лише готове значення, генератора totp в SDK нема; для sshKey — secret.algorithm перегенеровує ключ), delete (м'яко — у кошик, не назавжди), restore (повернути з кошика за id).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "get", "create", "edit", "rotate_secret", "delete", "restore", "sync"] },
                    "id": { "type": "string", "description": "ID елемента (для get/edit/rotate_secret/delete/restore)" },
                    "type": { "type": "integer", "enum": [1, 2, 5], "description": "1 — login, 2 — secureNote, 5 — sshKey: ключ генерується самим сервером, приватна частина не повертається (для create)" },
                    "name": { "type": "string", "description": "Назва (для create/edit); для list — підрядок назви без урахування регістру" },
                    "notes": { "type": "string", "description": "Нотатки (для create/edit)" },
                    "folderId": { "type": "string", "description": "Тека (для create/edit); для list — фільтр за текою" },
                    "login": {
                        "type": "object",
                        "description": "Дані для type=1 (login) при create — без секретів",
                        "properties": {
                            "username": { "type": "string" },
                            "generatePassword": { "type": "boolean", "description": "Згенерувати пароль автоматично; значення не повертається у відповіді" },
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
                    },
                    "sshKey": {
                        "type": "object",
                        "description": "Для type=5 (sshKey)",
                        "properties": {
                            "algorithm": { "type": "string", "enum": ["ed25519", "rsa3072", "rsa4096", "ecdsap256", "ecdsap384", "ecdsap521"], "description": "Типово ed25519 (лише create)" },
                            "login": { "type": "string", "description": "Формат <користувач>@<IPv4> (лише IPv4, без FQDN/IPv6/кількох адрес); обовʼязковий при create, опціональний при edit (оновлює збережене значення)" }
                        }
                    },
                    "secret": {
                        "type": "object",
                        "description": "Лише для rotate_secret: для login — password (готове значення) або generatePassword (генерація на сервері, значення не повертається, взаємовиключне з password) і/чи totp; для sshKey — algorithm (ключ генерується наново, значення не повертається)",
                        "properties": {
                            "password": { "type": "string", "description": "Готове значення пароля — caller сам щойно передав його, у відповіді не приховується" },
                            "generatePassword": { "type": "boolean", "description": "Згенерувати пароль на сервері; значення не повертається у відповіді; взаємовиключне з password" },
                            "length": { "type": "integer", "minimum": 5, "description": "Довжина пароля (лише з generatePassword)" },
                            "lowercase": { "type": "boolean", "description": "Лише з generatePassword" },
                            "uppercase": { "type": "boolean", "description": "Лише з generatePassword" },
                            "number": { "type": "boolean", "description": "Лише з generatePassword" },
                            "special": { "type": "boolean", "description": "Лише з generatePassword" },
                            "totp": { "type": "string" },
                            "algorithm": { "type": "string", "enum": ["ed25519", "rsa3072", "rsa4096", "ecdsap256", "ecdsap384", "ecdsap521"] }
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
