#!/usr/bin/env bash

set -euo pipefail

REPO="IntScription/rundeck"
BIN="rundeck"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

info() {
  printf "\033[1;34m[RunDeck]\033[0m %s\n" "$1"
}

error() {
  printf "\033[1;31m[RunDeck]\033[0m %s\n" "$1"
}

detect_target() {
  local os arch

  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
  Darwin)
    case "$arch" in
    arm64) echo "aarch64-apple-darwin" ;;
    x86_64) echo "x86_64-apple-darwin" ;;
    *)
      error "Unsupported macOS architecture: $arch"
      exit 1
      ;;
    esac
    ;;
  Linux)
    case "$arch" in
    x86_64) echo "x86_64-unknown-linux-gnu" ;;
    *)
      error "Unsupported Linux architecture: $arch"
      exit 1
      ;;
    esac
    ;;
  *)
    error "Unsupported OS: $os"
    exit 1
    ;;
  esac
}

ensure_path_hint() {
  case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    info "Add this to your shell config if rundeck is not found:"
    echo
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    echo
    ;;
  esac
}

main() {
  local target archive url tmp

  target="$(detect_target)"
  archive="rundeck-${target}.tar.gz"
  url="https://github.com/${REPO}/releases/latest/download/${archive}"
  tmp="$(mktemp -d)"

  info "Detected target: ${target}"
  info "Downloading ${url}"

  mkdir -p "$INSTALL_DIR"

  curl -fsSL "$url" -o "$tmp/$archive"
  tar -xzf "$tmp/$archive" -C "$tmp"

  install -m 755 "$tmp/$BIN" "$INSTALL_DIR/$BIN"

  rm -rf "$tmp"

  info "Installed RunDeck to $INSTALL_DIR/$BIN"
  ensure_path_hint

  if command -v "$BIN" >/dev/null 2>&1; then
    "$BIN" doctor || true
  else
    info "Run: $INSTALL_DIR/$BIN doctor"
  fi
}

main "$@"
