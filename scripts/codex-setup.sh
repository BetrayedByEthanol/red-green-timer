#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

log() {
  printf '\n==> %s\n' "$1"
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

install_tauri_linux_dependencies() {
  if [[ "${OSTYPE:-}" != linux* ]] || ! command -v apt-get >/dev/null 2>&1; then
    return
  fi

  # Current Tauri 2 prerequisites for Debian/Ubuntu, plus packaging tools.
  local packages=(
    build-essential
    curl
    file
    libayatana-appindicator3-dev
    librsvg2-dev
    libssl-dev
    libwebkit2gtk-4.1-dev
    libxdo-dev
    patchelf
    pkg-config
    wget
  )
  local missing=()
  local package

  for package in "${packages[@]}"; do
    if ! dpkg-query -W -f='${Status}' "$package" 2>/dev/null | grep -q 'install ok installed'; then
      missing+=("$package")
    fi
  done

  if (( ${#missing[@]} == 0 )); then
    return
  fi

  local -a elevate=()
  if (( EUID != 0 )); then
    require_command sudo
    elevate=(sudo)
  fi

  log "Installing Tauri Linux system dependencies"
  "${elevate[@]}" apt-get update
  "${elevate[@]}" apt-get install -y --no-install-recommends "${missing[@]}"
}

log "Checking required toolchains"
require_command cargo
require_command node
require_command npm
require_command rustup

install_tauri_linux_dependencies

log "Installing Rust components"
rustup component add rustfmt clippy

log "Installing Node dependencies"
npm ci

log "Fetching Rust dependencies"
cargo fetch --locked

log "Checking formatting"
cargo fmt --all --check

log "Running Rust lints"
cargo clippy --workspace --all-targets -- -D warnings

log "Running Rust tests"
cargo test --workspace --locked

log "Building the Svelte frontend"
npm run build

log "Codex environment setup complete"
