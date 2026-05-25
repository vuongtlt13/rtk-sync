#!/usr/bin/env bash
set -euo pipefail

REPO="${RTK_SYNC_REPO:-vuongtlt13/rtk-sync}"
BIN_NAME="rtk-sync"
SERVICE_LABEL="${RTK_SYNC_SERVICE_LABEL:-com.vuong.rtk-sync}"
PLIST_PATH="${RTK_SYNC_PLIST_PATH:-$HOME/Library/LaunchAgents/$SERVICE_LABEL.plist}"
RESTART_SERVICE="${RTK_SYNC_RESTART_SERVICE:-1}"

case "$(uname -s)" in
  Darwin)
    install_dir_default="/opt/homebrew/bin"
    case "$(uname -m)" in
      arm64) target="aarch64-apple-darwin" ;;
      x86_64) target="x86_64-apple-darwin" ;;
      *) echo "Unsupported macOS architecture: $(uname -m)" >&2; exit 1 ;;
    esac
    archive="rtk-sync-$target.tar.gz"
    ;;
  Linux)
    install_dir_default="$HOME/.local/bin"
    case "$(uname -m)" in
      x86_64) target="x86_64-unknown-linux-gnu" ;;
      *) echo "Unsupported Linux architecture: $(uname -m)" >&2; exit 1 ;;
    esac
    archive="rtk-sync-$target.tar.gz"
    ;;
  *)
    echo "Unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

INSTALL_DIR="${RTK_SYNC_INSTALL_DIR:-$install_dir_default}"

url="https://github.com/$REPO/releases/latest/download/$archive"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

service_was_running=0
if command -v launchctl >/dev/null 2>&1 && [ -f "$PLIST_PATH" ]; then
  if launchctl print "gui/$(id -u)/$SERVICE_LABEL" >/dev/null 2>&1; then
    service_was_running=1
    echo "Stopping $SERVICE_LABEL"
    launchctl bootout "gui/$(id -u)" "$PLIST_PATH" >/dev/null 2>&1 || true
  fi
fi

echo "Downloading $url"
curl -fsSL "$url" -o "$tmp_dir/$archive"
tar -xzf "$tmp_dir/$archive" -C "$tmp_dir"
chmod 0755 "$tmp_dir/$BIN_NAME"

mkdir -p "$INSTALL_DIR"
echo "Installing $BIN_NAME to $INSTALL_DIR/$BIN_NAME"
install -m 0755 "$tmp_dir/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME.new"
mv "$INSTALL_DIR/$BIN_NAME.new" "$INSTALL_DIR/$BIN_NAME"

if command -v xattr >/dev/null 2>&1; then
  xattr -d com.apple.provenance "$INSTALL_DIR/$BIN_NAME" 2>/dev/null || true
  xattr -d com.apple.quarantine "$INSTALL_DIR/$BIN_NAME" 2>/dev/null || true
fi

"$INSTALL_DIR/$BIN_NAME" --version

if [ "$service_was_running" -eq 1 ] && [ "$RESTART_SERVICE" != "0" ]; then
  echo "Starting $SERVICE_LABEL"
  launchctl bootstrap "gui/$(id -u)" "$PLIST_PATH"
fi
