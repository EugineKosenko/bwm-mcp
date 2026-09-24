# bwm-mcp

A local [MCP](https://modelcontextprotocol.io) server for [Bitwarden](https://bitwarden.com) /
[Vaultwarden](https://github.com/dani-garcia/vaultwarden) password vaults, written in Rust as a
literate [org-babel](https://orgmode.org/worg/org-contrib/babel/) program. It talks directly to the
Bitwarden identity/API servers over the official (if internal) Rust SDK, not through the `bw` CLI, and
exposes vault operations as MCP tools over the standard `stdio` transport.

This is an unofficial project. It is not affiliated with, endorsed by or sponsored by Bitwarden Inc.

No third-party MCP SDK crate is used: the `JSON-RPC 2.0` wire protocol is implemented directly on top
of `tokio` and `serde_json`, following the pattern of
[azure-mcp](https://github.com/EugineKosenko/az-mcp),
[figma-mcp](https://github.com/EugineKosenko/figma-mcp) and
[org-tmetric-mcp](https://github.com/EugineKosenko/org-tmetric-mcp).

## Why

The official [`@bitwarden/mcp-server`](https://github.com/bitwarden/mcp-server) is a thin wrapper
around the `bw` CLI: every tool call spawns a fresh `bw` process. The CLI's local state
(`~/.config/Bitwarden CLI/data.json`) is a single shared file rewritten whole on every mutating call, so
concurrent or closely-timed calls race and silently lose writes — reproducible in practice, not a
theoretical concern.

This server keeps one long-lived, authenticated connection for the whole process instead of one
short-lived CLI subprocess per call, which removes that class of race by construction. It talks to the
vault through `bitwarden-core`, `bitwarden-vault`, `bitwarden-auth`, `bitwarden-sync` and
`bitwarden-generators` — crates from
[`bitwarden/sdk-internal`](https://github.com/bitwarden/sdk-internal) that the real, Rust-rewritten `bw`
CLI (`crates/bw` in that same repository) is itself built on. They are marked `"Internal crate for the
bitwarden crate. Do not use."` and carry no semver guarantee, which is why the dependency below is
pinned to an exact git commit rather than a published version. The public `bitwarden` crate on
`crates.io` was not an option: it only covers Secrets Manager, with no vault access at all. The
higher-level `bitwarden-pm` facade crate was deliberately avoided too — it unconditionally pulls in
`bitwarden-importers` (`.kdbx` import support), which in turn depends on a version of the `keepass`
crate whose `aes`/`cipher` requirements conflict with the rest of the dependency graph.

The dependency actually points at
[`EugineKosenko/sdk-internal`](https://github.com/EugineKosenko/sdk-internal) (branch
`bwm-mcp-item-support`), a fork carrying three small, upstreamable fixes found while building this
server: `ciphers` (vault items) had no `SyncHandler` at all, so nothing ever populated the local
repository; `CipherCreateRequest` / `CipherEditRequest` were never re-exported past their private
submodule despite the crate's own docs calling them public input DTOs; and a password login never
called `init_user_id`, so anything reading it (`CiphersClient::create`) failed with
`NotAuthenticatedError` even after a fully successful login. None of these are specific to this
project — `folders` and `generate` worked against plain upstream.

## Tools

This is an early, partial implementation — see the table below for what exists today. More tools
(`move`, `edit_item_collections`, ...) are added opportunistically, one at a time.

- **folders** — `list` / `get` / `create` / `edit` on vault folders, one tool with an `action`
  argument. Folders carry no secret field, so there is no reason to gate reads and writes separately
  the way item tools eventually will.
- **items** — `list` / `get` / `create` (`login` or `secureNote`) / `edit` (change `folderId`, `name`
  or `notes` on an item by `id`, everything else — `login`, custom fields, attachments — preserved as
  is). `card` / `identity` are not supported yet.
- **generate** — a password or, with `passphrase: true`, a passphrase. Runs locally, no vault access or
  login required.

## Authentication

The server logs in with a master password on every start (email, password and server URL from
environment variables) and keeps the resulting tokens and vault key in process memory only — nothing is
written to disk, and there is no persisted session to reuse across restarts yet. The email is not just a
login identifier: Bitwarden derives the master key from `KDF(master password, salt = email)`, so the
correct email is required to decrypt the vault at all, independent of the login request itself.

## Source layout

Every `.rs` file here is generated (tangled) from an `.org` file of the same purpose — edit the `.org`
source and re-tangle, never the `.rs` files directly. The prose of the `.org` files is in Ukrainian.

| org file | tangles to | purpose |
|---|---|---|
| `config.org` | `Cargo.toml`, `.cargo/config.toml` | dependencies, build target directory |
| `env.org` | `.env.example` | environment variables |
| `gitignore.org` | `.gitignore` | ignored files |
| `main.org` | `src/main.rs` | the JSON-RPC/stdio protocol loop |
| `main-client.org` | `src/client.rs` | login, sync, shared client type |
| `main-tools.org` | `src/tools/mod.rs` | tool registry, dispatch, shared helpers |
| `main-tool-*.org` | `src/tools/*.rs` | one file per tool |

## Building

```sh
cargo build --release
```

The first build clones the whole `sdk-internal` repository and compiles a few hundred crates; expect
several minutes.

## Configuration

Copy `.env.example` to `.env` for local `cargo run` testing only:

```
BWM_SERVER=https://localhost:8443
BWM_EMAIL=
BWM_PASSWD=
```

`BWM_SERVER` is the base URL of the Bitwarden/Vaultwarden server (no trailing slash); `/api` and
`/identity` are appended automatically.

## Registering with Claude Code

```sh
claude mcp add --scope user bwm-mcp \
  -e BWM_SERVER=https://localhost:8443 \
  -e BWM_EMAIL=<your email> \
  -e BWM_PASSWD=<your master password> \
  -- /path/to/bwm-mcp/target/release/bwm-mcp
```

## License

[MIT](LICENSE)