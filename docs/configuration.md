# Configuration

`rtk-sync` auto-creates a default config file on first run.

Default config location:

```text
macOS:   ~/Library/Application Support/rtk-sync/config.toml
Linux:   ~/.config/rtk-sync/config.toml
Windows: %APPDATA%\rtk-sync\config.toml
```

Configuration precedence:

1. CLI flags
2. environment variables
3. `config.toml`
4. defaults

A sample config is available in [../config.example.toml](../config.example.toml).

## Configure endpoint and token

Recommended setup:

```bash
rtk-sync config \
  --endpoint https://your-server.example.com/api/rtk/events \
  --token your-token
```

During development:

```bash
cargo run -- config \
  --endpoint https://your-server.example.com/api/rtk/events \
  --token your-token
```

This updates the OS-specific `config.toml` file. To use a local development config instead, pass `--config`:

```bash
cargo run -- config --config ./config.local.toml \
  --endpoint http://localhost:3000/api/rtk/events \
  --token your-token \
  --allow-insecure-http true

cargo run -- once --config ./config.local.toml --dry-run
```

## Keep token outside config.toml

If you do not want to store the token in `config.toml`, store only the env var name:

```bash
rtk-sync config \
  --endpoint https://your-server.example.com/api/rtk/events \
  --token-env RTK_SYNC_TOKEN

export RTK_SYNC_TOKEN="your-token"
```

## Background sync interval

The service sync interval lives in `config.toml`:

```toml
interval = 60
```

Update it with:

```bash
rtk-sync config --interval 60
```

`install-service` reads this value through the internal service runner, so there is no public `daemon --interval` command to configure.

## RTK database detection

By default, `rtk-sync` uses the same OS-specific RTK tracking database location as RTK:

```text
dirs::data_local_dir()/rtk/history.db
```

Common examples:

```text
macOS:   ~/Library/Application Support/rtk/history.db
Linux:   ~/.local/share/rtk/history.db
Windows: %LOCALAPPDATA%\rtk\history.db
```

DB path precedence:

1. CLI flag: `--db <path>`
2. environment variable: `RTK_SYNC_DB`
3. environment variable: `RTK_DB_PATH`
4. `db` in `config.toml`
5. default OS-specific RTK path

## Environment variables

`rtk-sync` reads real process environment variables, but does not auto-load `.env` files. For local development, pass `--config <path>` to use a repository-local config file instead of the installed default.

Example:

```bash
cargo run -- once --config ./config.local.toml --dry-run
rtk-sync once --config ./config.local.toml --dry-run
```

See [../.env.example](../.env.example) for shell env examples.

Supported variables:

```bash
RTK_SYNC_CONFIG               # config.toml path override
RTK_SYNC_DB                   # RTK SQLite DB path override
RTK_DB_PATH                   # RTK-compatible DB path override
RTK_SYNC_ENDPOINT             # upload endpoint override
RTK_SYNC_TOKEN                # default bearer token env var
RTK_SYNC_MACHINE_ID           # machine ID override
RTK_SYNC_BATCH_SIZE           # batch size override
RTK_SYNC_ALLOW_INSECURE_HTTP  # allow http:// endpoints for local development
RTK_SYNC_STATE                # state file path override
```

## State file

`rtk-sync` stores sync state separately from RTK's database.

Default state location:

```text
dirs::data_local_dir()/rtk-sync/state.json
```

Example:

```json
{
  "machine_id": "macbook-vuong-a1b2c3d4",
  "last_synced_id": 12345,
  "last_synced_at": "2026-05-23T10:30:00Z"
}
```

The checkpoint is updated only after a successful upload response.
