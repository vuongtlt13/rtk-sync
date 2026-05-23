# RTK Sync Tool Plan (Rust)

## Goal

Build a separate lightweight Rust tool, tentatively named `rtk-sync`, that reads RTK's local SQLite tracking database in read-only mode and synchronizes usage events to a central server.

The tool should be extremely light on startup time, memory usage, and CPU usage. It should follow RTK's design philosophy: local-first, fail-safe, no async runtime, low overhead, and predictable resource usage.

```text
RTK local SQLite
    ↓ read-only
rtk-sync
    ↓ HTTPS batch upload
Server API
    ↓
central DB
```

## Decision: Rust Instead of Go

Rust is preferred for this tool because the user wants the sync process to be very lightweight and fast.

Advantages of Rust for this use case:

- no garbage collector;
- lower idle memory footprint;
- predictable CPU usage;
- fast startup;
- easy static-ish binary deployment;
- same language ecosystem as RTK;
- can reuse RTK-compatible patterns such as `rusqlite`, `anyhow`, and blocking I/O;
- easier to integrate back into RTK later if desired.

Go would still be acceptable for a simple cron-style sync command, but Rust is the better fit for a low-footprint daemon or frequently invoked sync tool.

## Non-goals

- Do not make multiple machines write directly to the same SQLite file.
- Do not write sync metadata into RTK's own database.
- Do not require RTK to be online for normal command execution.
- Do not add an async runtime such as Tokio.
- Do not build a dashboard or analytics UI in the first version.
- Do not upload raw command output unless explicitly added later.

## High-level Architecture

```text
Machine A/B/C
  RTK command execution
      ↓
  RTK writes local SQLite tracking DB
      ↓
  rtk-sync reads new rows read-only
      ↓
  rtk-sync sends batch JSON to server
      ↓
  server inserts events idempotently
      ↓
  server stores central analytics DB
```

Each client machine has:

- one local RTK SQLite DB;
- one `rtk-sync` state file;
- one stable `machine_id`;
- one upload checkpoint.

The server handles deduplication using a stable `source_id`.

## Recommended Implementation Style

Use blocking Rust only.

Avoid:

```text
tokio
async-std
futures
background worker runtime
heavy telemetry/logging frameworks
```

Prefer:

```text
std::process
std::fs
std::time
rusqlite
ureq or reqwest blocking
anyhow
serde
serde_json
clap
```

For the HTTP client, prefer `ureq` first if its TLS setup is acceptable because it is simple and blocking. Use `reqwest` with `blocking` + `rustls-tls` only if more HTTP features are needed.

## Suggested Dependencies

```toml
[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive", "env"] }
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_with = "3"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
dirs = "5"
hostname = "0.4"
ureq = { version = "2", features = ["json", "tls"] }
```

If binary size is more important than bundled SQLite convenience, evaluate removing `rusqlite/bundled` and linking against system SQLite.

## CLI Design

### Commands

```bash
rtk-sync inspect
rtk-sync machine-id
rtk-sync once
rtk-sync daemon
```

### `inspect`

Checks the local RTK SQLite database and prints schema/diagnostic information.

Example:

```bash
rtk-sync inspect --db ~/.local/share/rtk/rtk.db
```

Expected output:

```text
DB: /Users/me/.local/share/rtk/rtk.db
Tables: command_history, ...
Detected rows: 12450
Latest row: 2026-05-23T10:30:00Z
Machine ID: macbook-vuong-7f3a9c
State file: ~/.local/share/rtk-sync/state.json
```

### `machine-id`

Prints or initializes the local machine ID.

Example:

```bash
rtk-sync machine-id
```

### `once`

Synchronizes one batch and exits.

Example:

```bash
rtk-sync once \
  --db ~/.local/share/rtk/rtk.db \
  --endpoint https://server.example.com/api/rtk/events
```

This should be the MVP sync mode.

### `daemon`

Runs sync repeatedly on an interval.

Example:

```bash
rtk-sync daemon --interval 60
```

The first version can defer this until `once` is stable.

## Configuration

Support flags and environment variables first. Add TOML config only if needed.

Priority order:

1. CLI flags;
2. environment variables;
3. defaults.

### Environment Variables

```bash
export RTK_SYNC_DB="$HOME/.local/share/rtk/rtk.db"
export RTK_SYNC_ENDPOINT="https://server.example.com/api/rtk/events"
export RTK_SYNC_TOKEN="..."
export RTK_SYNC_MACHINE_ID="macbook-vuong"
export RTK_SYNC_BATCH_SIZE="100"
export RTK_SYNC_STATE="$HOME/.local/share/rtk-sync/state.json"
```

### CLI Flags

Common flags:

```bash
--db <path>
--state <path>
--endpoint <url>
--token-env <env-var-name>
--machine-id <id>
--batch-size <n>
--allow-insecure-http
```

Default token env var:

```text
RTK_SYNC_TOKEN
```

## Local State

The sync tool must keep state outside RTK's database.

Recommended path:

```text
~/.local/share/rtk-sync/state.json
```

Example:

```json
{
  "machine_id": "macbook-vuong-7f3a9c",
  "last_synced_id": 12345,
  "last_synced_at": "2026-05-23T10:30:00Z"
}
```

Rules:

- if `--machine-id` is supplied, use it;
- else if `RTK_SYNC_MACHINE_ID` is set, use it;
- else if state has `machine_id`, reuse it;
- else generate a new machine ID and persist it;
- only update `last_synced_id` after successful upload;
- never update checkpoint on failed upload.

State writes should be atomic:

```text
write state.json.tmp
fsync if practical
rename to state.json
```

## Machine ID

Generate a stable, non-sensitive ID.

Recommended format:

```text
<hostname>-<short-uuid>
```

Example:

```text
macbook-vuong-7f3a9c
```

Do not use raw hardware identifiers.

## Event Identity

Each uploaded event must have a stable idempotency key.

Recommended format:

```text
<machine_id>:<local_row_id>
```

Example:

```text
macbook-vuong-7f3a9c:12345
```

This becomes `source_id` in the server database.

The server must enforce uniqueness on `source_id`.

## Reading RTK SQLite Safely

Use `rusqlite` read-only flags.

```rust
use rusqlite::{Connection, OpenFlags};

let conn = Connection::open_with_flags(
    db_path,
    OpenFlags::SQLITE_OPEN_READ_ONLY,
)?;
```

Do not write to the RTK DB.

Initial query pattern:

```sql
SELECT *
FROM command_history
WHERE id > ?
ORDER BY id ASC
LIMIT ?;
```

The exact table and column names must be confirmed with `rtk-sync inspect` against a real RTK DB.

## Event Model

```rust
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RtkEvent {
    pub source_id: String,
    pub machine_id: String,
    pub local_id: i64,
    pub command: String,
    pub raw_tokens: i64,
    pub filtered_tokens: i64,
    pub saved_tokens: i64,
    pub created_at: DateTime<Utc>,
}
```

The DB mapper should live in one module so schema changes are isolated.

## Upload API

Client request:

```http
POST /api/rtk/events
Authorization: Bearer <token>
Content-Type: application/json
```

Payload:

```json
{
  "machine_id": "macbook-vuong-7f3a9c",
  "events": [
    {
      "source_id": "macbook-vuong-7f3a9c:12345",
      "machine_id": "macbook-vuong-7f3a9c",
      "local_id": 12345,
      "command": "git status",
      "raw_tokens": 1200,
      "filtered_tokens": 180,
      "saved_tokens": 1020,
      "created_at": "2026-05-23T10:30:00Z"
    }
  ]
}
```

Server response:

```json
{
  "accepted": 100,
  "duplicates": 3,
  "max_local_id": 12445
}
```

Checkpoint rule:

- update `last_synced_id` to `max_local_id` only after successful 2xx response;
- do not update checkpoint on timeout, network error, 401/403, 4xx, or 5xx.

## Server-side Storage

### PostgreSQL Recommended Schema

```sql
CREATE TABLE rtk_events (
    source_id TEXT PRIMARY KEY,
    machine_id TEXT NOT NULL,
    local_id BIGINT NOT NULL,
    command TEXT NOT NULL,
    raw_tokens BIGINT,
    filtered_tokens BIGINT,
    saved_tokens BIGINT,
    created_at TIMESTAMPTZ NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_rtk_events_machine_created_at
ON rtk_events (machine_id, created_at);
```

Idempotent insert:

```sql
INSERT INTO rtk_events (
    source_id,
    machine_id,
    local_id,
    command,
    raw_tokens,
    filtered_tokens,
    saved_tokens,
    created_at
)
VALUES (...)
ON CONFLICT (source_id) DO NOTHING;
```

### Server-local SQLite Option

SQLite is acceptable on the server if only the server process writes to the SQLite file.

Do not allow many clients to write directly to one SQLite file over NFS/SMB/SSHFS.

## Rust Project Structure

If built as a separate repository:

```text
rtk-sync/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── state.rs
│   ├── machine.rs
│   ├── rtkdb.rs
│   ├── client.rs
│   └── syncer.rs
└── tests/
    └── sync_once.rs
```

If added inside the RTK repository later:

```text
src/bin/rtk-sync.rs
src/sync_tool/
├── mod.rs
├── cli.rs
├── config.rs
├── state.rs
├── machine.rs
├── rtkdb.rs
├── client.rs
└── syncer.rs
```

Recommendation: start as a separate repository or separate binary to avoid changing RTK behavior.

## Module Responsibilities

### `cli.rs`

- Define clap parser.
- Define subcommands.
- Keep command-specific options small and explicit.

### `config.rs`

- Resolve flags and environment variables.
- Validate required values for `once` and `daemon`.
- Reject insecure HTTP unless `--allow-insecure-http` is set.

### `state.rs`

- Load/save state JSON.
- Create parent directory if needed.
- Save atomically.
- Store `machine_id`, `last_synced_id`, and `last_synced_at`.

### `machine.rs`

- Resolve machine ID from CLI/env/state.
- Generate a new machine ID if missing.

### `rtkdb.rs`

- Open RTK SQLite read-only.
- Inspect schema.
- Fetch unsynced rows.
- Map rows to `RtkEvent`.

### `client.rs`

- Send blocking HTTP request.
- Add bearer auth.
- Parse upload response.
- Avoid logging token values.

### `syncer.rs`

- Orchestrate `once` flow.
- Fetch rows.
- Upload batch.
- Update checkpoint only on success.

## `once` Flow

```text
load config
load state
resolve machine_id
open RTK DB read-only
fetch rows where id > last_synced_id limit batch_size
if no rows: print short message and exit 0
upload batch
if upload success: update state.last_synced_id and state.last_synced_at
if upload fail: exit non-zero and do not update state
```

Pseudo-code:

```rust
pub fn run_once(config: Config) -> anyhow::Result<()> {
    let mut state = State::load_or_default(&config.state_path)?;
    let machine_id = resolve_machine_id(&config, &mut state)?;

    let conn = rtkdb::open_read_only(&config.db_path)?;
    let events = rtkdb::fetch_events(
        &conn,
        state.last_synced_id,
        config.batch_size,
        &machine_id,
    )?;

    if events.is_empty() {
        println!("No events to sync");
        return Ok(());
    }

    let result = client::upload_events(&config, &machine_id, &events)?;

    state.last_synced_id = result.max_local_id;
    state.last_synced_at = Some(chrono::Utc::now());
    state.save(&config.state_path)?;

    println!("Synced {} events", result.accepted);
    Ok(())
}
```

## `daemon` Flow

Keep daemon simple and blocking.

```rust
loop {
    if let Err(error) = run_once(config.clone()) {
        eprintln!("rtk-sync: sync failed: {error:#}");
    }

    std::thread::sleep(config.interval);
}
```

Add graceful SIGINT/SIGTERM handling only after MVP if needed.

## Performance Targets

Target footprint:

- startup: under 10 ms where practical;
- idle daemon memory: low single-digit MB if possible;
- no async runtime;
- no continuous polling faster than needed;
- default daemon interval: 60 seconds or more;
- default batch size: 100.

Recommended sync mode for minimum overhead:

```bash
rtk-sync once
```

Run it from cron/launchd every few minutes instead of keeping a daemon alive.

## Failure Handling Rules

- Upload fail: do not update checkpoint.
- Server timeout: do not update checkpoint.
- Server 401/403: fail loudly; do not retry in tight loop.
- Server 5xx: retry on next run.
- Duplicate events: treat as success if server confirms they were handled.
- SQLite locked: fail current sync; retry next run.
- Unknown DB schema: tell user to run `rtk-sync inspect`.

## Security Rules

- Do not log bearer tokens.
- Prefer token from `RTK_SYNC_TOKEN`.
- Require HTTPS by default.
- Allow HTTP only with explicit `--allow-insecure-http`.
- Keep machine ID non-sensitive.
- Upload metadata only, not raw command output.
- Do not execute commands from DB content.

## Testing Plan

### Unit Tests

- config precedence;
- state load/save;
- machine ID generation;
- source ID generation;
- upload response parsing;
- insecure HTTP rejection.

### Integration Tests

- create temp SQLite DB with a fake RTK tracking table;
- insert sample command rows;
- run fetch logic;
- run `once` against a local test HTTP server;
- verify checkpoint updates only after successful upload;
- verify failed upload does not advance checkpoint.

### Manual Tests

```bash
rtk-sync inspect --db /path/to/real/rtk.db
rtk-sync machine-id
rtk-sync once --endpoint https://server.example.com/api/rtk/events
rtk-sync once
```

Expected behavior:

- first `once` uploads rows;
- second `once` reports no new events;
- failed server does not advance checkpoint.

## Implementation Phases

### Phase 1: Inspect RTK DB

Deliver:

```bash
rtk-sync inspect --db <path>
```

Tasks:

- create Rust binary skeleton;
- add clap parser;
- open SQLite read-only;
- list tables;
- print schemas;
- print row counts for likely tracking tables.

Success criteria:

- runs against a real RTK DB without modifying it;
- confirms exact table/column names needed by mapper.

### Phase 2: Machine ID and State File

Deliver:

```bash
rtk-sync machine-id
```

Tasks:

- implement state JSON;
- generate stable machine ID;
- support CLI/env override;
- save state atomically.

Success criteria:

- repeated runs return the same machine ID;
- state lives outside RTK DB.

### Phase 3: Fetch Unsynced Events

Deliver internal function:

```rust
pub fn fetch_events(
    conn: &rusqlite::Connection,
    after_id: i64,
    limit: usize,
    machine_id: &str,
) -> anyhow::Result<Vec<RtkEvent>>
```

Tasks:

- map rows into `RtkEvent`;
- compute `source_id`;
- order by local ID;
- limit batch size.

Success criteria:

- can print JSON batch locally without uploading.

### Phase 4: HTTP Upload Client

Deliver internal function:

```rust
pub fn upload_events(
    config: &Config,
    machine_id: &str,
    events: &[RtkEvent],
) -> anyhow::Result<UploadResult>
```

Tasks:

- POST JSON;
- add bearer token;
- enforce timeout;
- parse response;
- avoid token leaks in errors.

Success criteria:

- uploads successfully to a local test server.

### Phase 5: `once`

Deliver:

```bash
rtk-sync once
```

Tasks:

- load config;
- load state;
- fetch unsynced events;
- upload batch;
- update checkpoint on success.

Success criteria:

- repeat runs do not duplicate checkpointed events;
- failed upload retries later.

### Phase 6: `daemon`

Deliver:

```bash
rtk-sync daemon --interval 60
```

Tasks:

- loop around `once`;
- sleep between runs;
- continue after temporary failures.

Success criteria:

- can run continuously with low overhead.

## MVP Scope

Include:

- `inspect`;
- `machine-id`;
- `once`;
- read-only SQLite access;
- state file checkpoint;
- batch HTTP upload;
- idempotent `source_id`;
- bearer token auth;
- HTTPS by default.

Defer:

- daemon if time is limited;
- config TOML;
- dashboard;
- compression;
- advanced retry/backoff;
- schema migration support.

## Recommended First Commit Scope

Keep the first implementation small:

```text
rtk-sync inspect
rtk-sync machine-id
state.json support
read-only SQLite open
```

After inspecting the real RTK database schema, implement event mapping and upload in the next step.
