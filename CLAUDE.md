# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository state

This repository contains a standalone Rust crate for `rtk-sync`, plus the original RTK source under [tmp/rtk/](tmp/rtk/) for schema/reference checks. Treat [plan.md](plan.md) as product background, but prefer the implemented source files for current behavior.

## Intended project

`rtk-sync` should read RTK's local SQLite tracking database in read-only mode and synchronize usage events to a central server via HTTPS batch upload.

Core design constraints:

- Rust implementation, optimized for fast startup and low memory/CPU usage.
- Blocking I/O only; do not add Tokio, async-std, futures, or a background worker runtime.
- Local-first and fail-safe: RTK command execution must not require network access.
- Never write sync metadata into RTK's SQLite database.
- Keep sync state in a separate state file, recommended at `~/.local/share/rtk-sync/state.json`.
- Upload metadata only; do not upload raw command output unless that requirement is explicitly added later.

## Development commands

Use standard Cargo commands unless the project later adds a Makefile or justfile:

```bash
cargo build
cargo test
cargo test <test_name>
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- inspect --db ~/.local/share/rtk/history.db
cargo run -- machine-id
cargo run -- config --endpoint <https-url> --token <token>
cargo run -- once --db ~/.local/share/rtk/history.db --endpoint <https-url>
cargo run -- daemon --interval 60
```

Configuration examples live in [config.example.toml](config.example.toml) and [.env.example](.env.example). The binary auto-creates an OS-specific default config at `dirs::config_dir()/rtk-sync/config.toml`; env vars override config values, and CLI flags override both. It auto-loads a local `.env` file without overriding variables already set in the process environment.

## CLI shape

Planned subcommands:

- `inspect`: open the RTK SQLite database read-only and print schema/diagnostic information.
- `machine-id`: print or initialize the local stable machine ID.
- `config`: update the OS-specific `config.toml` from CLI values, similar to `git config`.
- `once`: synchronize one batch and exit; this is the MVP sync mode.
- `daemon`: run sync repeatedly with a blocking sleep interval; defer until `once` is stable if needed.

Configuration should be resolved in this order:

1. CLI flags.
2. Environment variables.
3. `config.toml`.
4. Defaults.

Important environment variables from the plan:

```bash
RTK_SYNC_DB
RTK_SYNC_ENDPOINT
RTK_SYNC_TOKEN
RTK_SYNC_MACHINE_ID
RTK_SYNC_BATCH_SIZE
RTK_SYNC_STATE
```

## Planned Rust architecture

If implemented as a separate crate, use this module split:

```text
src/main.rs
src/cli.rs
src/config.rs
src/state.rs
src/machine.rs
src/rtkdb.rs
src/client.rs
src/syncer.rs
tests/sync_once.rs
```

Responsibilities:

- `cli.rs`: clap parser and subcommands.
- `config.rs`: resolve flags/env/defaults, validate required values, reject insecure HTTP unless explicitly allowed.
- `state.rs`: load/save JSON state, create parent directory as needed, save atomically.
- `machine.rs`: resolve or generate stable non-sensitive machine IDs.
- `rtkdb.rs`: open RTK SQLite read-only, inspect schema, fetch unsynced rows, map DB rows to upload events.
- `client.rs`: send blocking HTTP requests, add bearer auth, parse upload responses, avoid token leaks.
- `syncer.rs`: orchestrate `once`, including checkpoint updates only after successful upload.

Key dependencies include `anyhow`, `clap`, `rusqlite`, `serde`, `serde_json`, `toml`, `chrono`, `dirs`, `hostname`, `sha2`, and blocking HTTP via `ureq`. Do not add async runtimes.

## Data and sync rules

Local state should include:

```json
{
  "machine_id": "macbook-vuong-7f3a9c",
  "last_synced_id": 12345,
  "last_synced_at": "2026-05-23T10:30:00Z"
}
```

Machine ID resolution:

1. `--machine-id` flag.
2. `RTK_SYNC_MACHINE_ID`.
3. Existing state file.
4. Generate a deterministic non-sensitive ID from hostname, username, OS, and arch, then persist it in state. Format: `<sanitized-hostname>-<8-char-hash>`.

Event identity:

- Use stable idempotency keys in the form `<machine_id>:<local_row_id>`.
- Send this as `source_id`.
- Server storage must enforce uniqueness on `source_id`.

Checkpoint behavior:

- Update `last_synced_id` only after a successful 2xx upload response.
- Do not update the checkpoint on timeout, network error, 401/403, other 4xx, or 5xx.
- Treat duplicate events as successful only when the server confirms they were handled.

## SQLite requirements

Open the RTK database read-only using `rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY`.

The implemented mapper reads RTK's `commands` table:

```sql
SELECT id, timestamp, original_cmd, rtk_cmd, input_tokens, output_tokens, saved_tokens, savings_pct, COALESCE(exec_time_ms, 0), COALESCE(project_path, '')
FROM commands
WHERE id > ?
ORDER BY id ASC
LIMIT ?;
```

The default RTK DB path is `dirs::data_local_dir()/rtk/history.db`, with `RTK_SYNC_DB` or `RTK_DB_PATH` overrides.

## HTTP API expectations

The sync client should POST JSON to the configured endpoint with bearer auth:

```http
Authorization: Bearer <token>
Content-Type: application/json
```

Expected response shape:

```json
{
  "accepted": 100,
  "duplicates": 3,
  "max_local_id": 12445
}
```

Require HTTPS by default. Allow HTTP only with an explicit `--allow-insecure-http` flag.

## Testing focus

When implementation exists, prioritize tests for:

- config precedence;
- state load/save and atomic checkpoint behavior;
- machine ID generation and reuse;
- source ID generation;
- upload response parsing;
- insecure HTTP rejection;
- read-only SQLite fetch logic using a temporary database;
- `once` behavior against a local test HTTP server, including no checkpoint advance after failed upload.

Manual smoke tests should cover:

```bash
rtk-sync inspect --db /path/to/real/rtk.db
rtk-sync machine-id
rtk-sync once --endpoint <https-url>
rtk-sync once
```

The second `once` should report no new events after a successful first sync.
