# Command Reference

`rtk-sync` supports these commands:

```bash
rtk-sync inspect
rtk-sync machine-id
rtk-sync config
rtk-sync reset
rtk-sync once
rtk-sync install-service
rtk-sync uninstall-service
```

## `inspect`

Open the local RTK SQLite database read-only and print diagnostic information.

```bash
rtk-sync inspect
```

With explicit DB path:

```bash
rtk-sync inspect --db "$HOME/Library/Application Support/rtk/history.db"
```

This prints tables, command count, latest row, and schema information.

## `machine-id`

Print or initialize the local machine ID.

```bash
rtk-sync machine-id
```

Machine ID resolution order:

1. `--machine-id`
2. `RTK_SYNC_MACHINE_ID`
3. `machine_id` in config
4. existing state file
5. deterministic ID derived from local machine info

Generated IDs use this format:

```text
<sanitized-hostname>-<8-char-hash>
```

## `config`

Update the current `config.toml` from CLI values, similar to `git config`.

```bash
rtk-sync config \
  --endpoint https://your-server.example.com/api/rtk/events \
  --token your-token
```

Supported options:

```bash
--config <path>
--endpoint <url>
--token <token>
--token-env <env-var>
--machine-id <id>
--batch-size <n>
--interval <seconds>
--allow-insecure-http <true|false>
--db <path>
--state <path>
```

## `reset`

Delete the local sync state file.

```bash
rtk-sync reset
```

This only removes the `rtk-sync` state/checkpoint file. It does not modify `config.toml` and never changes the RTK SQLite database.

With explicit state path:

```bash
rtk-sync reset --state /path/to/state.json
```

## `once`

Sync one batch and exit.

```bash
rtk-sync once
```

To test local DB reading without a server, token, upload, or checkpoint update, and print each event that would be uploaded as one JSON line:

```bash
rtk-sync once --dry-run
```

With explicit overrides:

```bash
rtk-sync once \
  --db /path/to/rtk/history.db \
  --endpoint https://your-server.example.com/api/rtk/events
```

For local HTTP testing only:

```bash
rtk-sync once \
  --endpoint http://127.0.0.1:8080/api/rtk/events \
  --allow-insecure-http
```

`once` prints progress logs such as:

```text
rtk-sync: sync started at 2026-05-23T10:30:00Z
rtk-sync: machine_id=macbook-a1b2c3d4 checkpoint=123
rtk-sync: fetching up to 100 events after local_id 123
rtk-sync: uploading 25 events to https://your-server.example.com/api/rtk/events (local_id 124..=148)
rtk-sync: upload accepted=25 duplicates=0 server_max_local_id=148
rtk-sync: checkpoint updated to 148
rtk-sync: sync completed
```

## `install-service`

Install a user-level auto-start service for background sync.

```bash
rtk-sync install-service
```

The service runs an internal hidden command and reads the sync interval from `config.toml` (`interval = 60` by default).

Supported platforms:

- macOS: creates a LaunchAgent at `~/Library/LaunchAgents/com.vuong.rtk-sync.plist`
- Linux: creates a systemd user service at `~/.config/systemd/user/rtk-sync.service`

Options:

```bash
--binary <absolute-path-to-rtk-sync>
--config <path>
```

By default, `--binary` uses the currently running executable path.

## `uninstall-service`

Remove the user-level auto-start service.

```bash
rtk-sync uninstall-service
```

On macOS this unloads and removes the LaunchAgent. On Linux this disables and removes the systemd user service.
