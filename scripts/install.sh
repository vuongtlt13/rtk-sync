#!/usr/bin/env bash
set -euo pipefail

REPO="${RTK_SYNC_REPO:-vuongtlt13/rtk-sync}"
BIN_NAME="rtk-sync"
SERVICE_LABEL="${RTK_SYNC_SERVICE_LABEL:-com.vuong.rtk-sync}"
PLIST_PATH="${RTK_SYNC_PLIST_PATH:-$HOME/Library/LaunchAgents/$SERVICE_LABEL.plist}"
RESTART_SERVICE="${RTK_SYNC_RESTART_SERVICE:-1}"

error() {
  echo "Error: $*" >&2
  exit 1
}

detect_os() {
  case "$(uname -s)" in
    Darwin)
      OS="darwin"
      INSTALL_DIR_DEFAULT="/opt/homebrew/bin"
      ;;
    Linux)
      OS="linux"
      INSTALL_DIR_DEFAULT="$HOME/.local/bin"
      ;;
    *)
      error "unsupported OS: $(uname -s)"
      ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) error "unsupported architecture: $(uname -m)" ;;
  esac
}

get_target() {
  case "$OS" in
    darwin)
      case "$ARCH" in
        aarch64) TARGET="aarch64-apple-darwin" ;;
        x86_64) TARGET="x86_64-apple-darwin" ;;
        *) error "unsupported macOS architecture: $ARCH" ;;
      esac
      ;;
    linux)
      case "$ARCH" in
        x86_64) TARGET="x86_64-unknown-linux-musl" ;;
        *) error "unsupported Linux architecture: $ARCH" ;;
      esac
      ;;
  esac
}

cleanup() {
  rm -rf "$TMP_DIR"
}

stop_service_if_running() {
  SERVICE_WAS_RUNNING=0
  if command -v launchctl >/dev/null 2>&1 && [ -f "$PLIST_PATH" ]; then
    if launchctl print "gui/$(id -u)/$SERVICE_LABEL" >/dev/null 2>&1; then
      SERVICE_WAS_RUNNING=1
      echo "Stopping $SERVICE_LABEL"
      launchctl bootout "gui/$(id -u)" "$PLIST_PATH" >/dev/null 2>&1 || true
    fi
  fi
}

verify_archive() {
  if tar -tzf "$ARCHIVE_PATH" | grep -qE '^/|(^|/)\.\.(/|$)'; then
    error "archive contains unsafe paths"
  fi
}

install_binary() {
  INSTALL_DIR="${RTK_SYNC_INSTALL_DIR:-$INSTALL_DIR_DEFAULT}"
  ARCHIVE="rtk-sync-$TARGET.tar.gz"
  URL="https://github.com/$REPO/releases/latest/download/$ARCHIVE"
  TMP_DIR="$(mktemp -d)"
  ARCHIVE_PATH="$TMP_DIR/$ARCHIVE"
  trap cleanup EXIT

  echo "Downloading $URL"
  curl -fsSL "$URL" -o "$ARCHIVE_PATH"
  verify_archive
  tar -xzf "$ARCHIVE_PATH" -C "$TMP_DIR"
  chmod 0755 "$TMP_DIR/$BIN_NAME"

  mkdir -p "$INSTALL_DIR"
  echo "Installing $BIN_NAME to $INSTALL_DIR/$BIN_NAME"
  install -m 0755 "$TMP_DIR/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME.new"
  mv "$INSTALL_DIR/$BIN_NAME.new" "$INSTALL_DIR/$BIN_NAME"
}

remove_macos_xattrs() {
  if command -v xattr >/dev/null 2>&1; then
    xattr -d com.apple.provenance "$INSTALL_DIR/$BIN_NAME" 2>/dev/null || true
    xattr -d com.apple.quarantine "$INSTALL_DIR/$BIN_NAME" 2>/dev/null || true
  fi
}

restart_service_if_needed() {
  if [ "$SERVICE_WAS_RUNNING" -eq 1 ] && [ "$RESTART_SERVICE" != "0" ]; then
    echo "Starting $SERVICE_LABEL"
    launchctl bootstrap "gui/$(id -u)" "$PLIST_PATH"
  fi
}

main() {
  detect_os
  detect_arch
  get_target
  stop_service_if_running
  install_binary
  remove_macos_xattrs
  "$INSTALL_DIR/$BIN_NAME" --version
  restart_service_if_needed
}

main "$@"
