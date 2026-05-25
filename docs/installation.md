# Installation

`rtk-sync` is distributed as a standalone binary. You do not need Rust or Cargo unless you want to build from source.

## Install from GitHub Releases

Download the asset that matches your OS and CPU from the latest GitHub Release.

### macOS Apple Silicon

Use this on M1/M2/M3 Macs:

```bash
curl -L -o rtk-sync-aarch64-apple-darwin.tar.gz \
  https://github.com/<owner>/<repo>/releases/latest/download/rtk-sync-aarch64-apple-darwin.tar.gz
tar -xzf rtk-sync-aarch64-apple-darwin.tar.gz
chmod +x rtk-sync
sudo mv rtk-sync /usr/local/bin/rtk-sync
rtk-sync --version
```

### macOS Intel

Use this on Intel Macs:

```bash
curl -L -o rtk-sync-x86_64-apple-darwin.tar.gz \
  https://github.com/<owner>/<repo>/releases/latest/download/rtk-sync-x86_64-apple-darwin.tar.gz
tar -xzf rtk-sync-x86_64-apple-darwin.tar.gz
chmod +x rtk-sync
sudo mv rtk-sync /usr/local/bin/rtk-sync
rtk-sync --version
```

### Linux x86_64

```bash
curl -L -o rtk-sync-x86_64-unknown-linux-musl.tar.gz \
  https://github.com/<owner>/<repo>/releases/latest/download/rtk-sync-x86_64-unknown-linux-musl.tar.gz
tar -xzf rtk-sync-x86_64-unknown-linux-musl.tar.gz
chmod +x rtk-sync
sudo mv rtk-sync /usr/local/bin/rtk-sync
rtk-sync --version
```

### Windows x86_64

Download this asset from the latest GitHub Release:

```text
rtk-sync-x86_64-pc-windows-msvc.zip
```

In PowerShell:

```powershell
Expand-Archive .\rtk-sync-x86_64-pc-windows-msvc.zip -DestinationPath .\rtk-sync
.\rtk-sync\rtk-sync.exe --version
```

Move `rtk-sync.exe` into a directory on your `PATH`, or keep using it with the explicit path.

## Build from source

Use this path for local development or unsupported platforms.

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
sudo cp ./target/release/rtk-sync /usr/local/bin/rtk-sync
rtk-sync --version
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
cargo run -- once --dry-run
```
