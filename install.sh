#!/usr/bin/env bash

set -euo pipefail

APP_NAME="rundeck"
REQUIRED_COMMANDS=("cargo" "git" "tmux" "fzf" "lazygit")

info() {
  printf "\033[1;34m[RunDeck]\033[0m %s\n" "$1"
}

success() {
  printf "\033[1;32m[RunDeck]\033[0m %s\n" "$1"
}

warn() {
  printf "\033[1;33m[RunDeck]\033[0m %s\n" "$1"
}

error() {
  printf "\033[1;31m[RunDeck]\033[0m %s\n" "$1"
}

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

ensure_project_root() {
  if [[ ! -f "Cargo.toml" ]]; then
    error "Cargo.toml not found."
    error "Run this script from the RunDeck project root."
    exit 1
  fi
}

check_requirements() {
  local missing=()

  for cmd in "${REQUIRED_COMMANDS[@]}"; do
    if ! command_exists "$cmd"; then
      missing+=("$cmd")
    fi
  done

  if ((${#missing[@]} > 0)); then
    warn "Missing required tools: ${missing[*]}"

    if command_exists brew; then
      warn "On macOS, install missing tools with:"
      echo
      echo "  brew install tmux lazygit fzf"
      echo
    fi

    if [[ " ${missing[*]} " == *" cargo "* ]]; then
      warn "Rust/Cargo is missing. Install Rust from:"
      echo
      echo "  https://rustup.rs"
      echo
    fi

    exit 1
  fi
}

ensure_cargo_path() {
  local cargo_path_line='export PATH="$HOME/.cargo/bin:$PATH"'
  local shell_name
  shell_name="$(basename "${SHELL:-}")"

  local rc_file=""

  case "$shell_name" in
  zsh)
    rc_file="$HOME/.zshrc"
    ;;
  bash)
    rc_file="$HOME/.bashrc"
    ;;
  fish)
    warn "Fish shell detected. Add this manually:"
    echo
    echo '  fish_add_path $HOME/.cargo/bin'
    echo
    return
    ;;
  *)
    warn "Unknown shell. Make sure this is in your shell config:"
    echo
    echo "  $cargo_path_line"
    echo
    return
    ;;
  esac

  mkdir -p "$HOME/.cargo/bin"

  if [[ ":$PATH:" != *":$HOME/.cargo/bin:"* ]]; then
    warn "$HOME/.cargo/bin is not currently in PATH."
  fi

  if [[ -f "$rc_file" ]] && grep -Fq "$cargo_path_line" "$rc_file"; then
    info "Cargo bin PATH already exists in $rc_file"
  else
    info "Adding Cargo bin PATH to $rc_file"
    echo "" >>"$rc_file"
    echo "# Cargo binaries" >>"$rc_file"
    echo "$cargo_path_line" >>"$rc_file"
  fi
}

install_rundeck() {
  info "Installing RunDeck with cargo..."
  cargo install --path . --force
}

verify_install() {
  local binary="$HOME/.cargo/bin/$APP_NAME"

  if [[ ! -x "$binary" ]]; then
    error "RunDeck binary was not found at $binary"
    exit 1
  fi

  success "Installed RunDeck:"
  echo
  echo "  $binary"
  echo

  if command_exists "$APP_NAME"; then
    success "RunDeck is available in PATH."
    echo
    echo "  $(command -v "$APP_NAME")"
    echo
  else
    warn "RunDeck installed, but this shell has not reloaded PATH yet."
    echo
    echo "Run:"
    echo
    echo "  source ~/.zshrc"
    echo "  rehash"
    echo
    echo "Or open a new terminal."
    echo
  fi
}

main() {
  ensure_project_root
  check_requirements
  ensure_cargo_path
  install_rundeck
  verify_install

  success "Done."
  echo
  echo "Try:"
  echo
  echo "  rundeck doctor"
  echo "  rundeck"
  echo
}

main "$@"
