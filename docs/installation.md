# Installation

## Build from source

```bash
git clone <repo-url>
cd rtk-sync
cargo build --release
```

The compiled binary is available at:

```bash
./target/release/rtk-sync
```

You can copy it into a directory on your `PATH`:

```bash
cp ./target/release/rtk-sync /usr/local/bin/rtk-sync
```

## Install background service

After configuring endpoint/token, install the user-level background service:

```bash
rtk-sync install-service
```

Supported platforms:

- macOS: LaunchAgent at `~/Library/LaunchAgents/com.vuong.rtk-sync.plist`
- Linux: systemd user service at `~/.config/systemd/user/rtk-sync.service`

The service starts automatically on login/restart and uses `interval` from `config.toml`.

Service logs:

```text
macOS stdout: /tmp/rtk-sync.out.log
macOS stderr: /tmp/rtk-sync.err.log
Linux: journalctl --user -u rtk-sync -f
```

Remove it with:

```bash
rtk-sync uninstall-service
```

## Development usage

During development, run commands through Cargo:

```bash
cargo run -- <command>
```

Examples:

```bash
cargo run -- inspect
cargo run -- config --endpoint https://example.com/api/rtk/events --token test
cargo run -- once
```
