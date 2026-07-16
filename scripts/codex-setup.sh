#!/usr/bin/env bash
set -euo pipefail

log() {
  printf '\n==> %s\n' "$1"
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

find_repository_root() {
  local root

  # Codex environment scripts are copied to a temporary location, so
  # BASH_SOURCE may not point inside the checked-out repository.
  if root="$(git -C "$PWD" rev-parse --show-toplevel 2>/dev/null)"; then
    printf '%s\n' "$root"
    return
  fi

  local script_dir
  script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
  if root="$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null)"; then
    printf '%s\n' "$root"
    return
  fi

  printf 'Could not locate the checked-out Git repository.\n' >&2
  exit 1
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
require_command git
require_command node
require_command npm
require_command rustup

ROOT_DIR="$(find_repository_root)"
cd "$ROOT_DIR"
printf 'Repository root: %s\n' "$ROOT_DIR"

if [[ ! -f package-lock.json ]]; then
  printf 'package-lock.json not found in repository root: %s\n' "$ROOT_DIR" >&2
  exit 1
fi

if [[ ! -f Cargo.lock ]]; then
  printf 'Cargo.lock not found in repository root: %s\n' "$ROOT_DIR" >&2
  exit 1
fi

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
