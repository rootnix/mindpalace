#!/bin/sh
# mindpalace installer (macOS / Linux)
#   curl -fsSL https://raw.githubusercontent.com/rootnix/mindpalace/main/install.sh | sh
# Windows: irm https://raw.githubusercontent.com/rootnix/mindpalace/main/install.ps1 | iex
set -eu

REPO="${MP_REPO:-https://github.com/rootnix/mindpalace.git}"
RELEASE_BASE="${MP_RELEASE_BASE:-https://github.com/rootnix/mindpalace/releases/latest/download}"
INSTALL_DIR="${MP_INSTALL_DIR:-$HOME/.local/share/mindpalace}"

command -v git >/dev/null 2>&1 || { echo "mindpalace: git is required"; exit 1; }

# 1. repo checkout — integrations (Claude plugin, skill) + templates live here
if [ -d "$INSTALL_DIR/.git" ]; then
  echo "updating existing install at $INSTALL_DIR"
  git -C "$INSTALL_DIR" pull --ff-only -q
else
  echo "installing to $INSTALL_DIR"
  mkdir -p "$(dirname "$INSTALL_DIR")"
  git clone --depth 1 -q "$REPO" "$INSTALL_DIR"
fi
mkdir -p "$INSTALL_DIR/bin"

# 2. mp binary: MP_BIN override > prebuilt release download > cargo build
get_binary() {
  if [ -n "${MP_BIN:-}" ] && [ -f "$MP_BIN" ]; then
    cp "$MP_BIN" "$INSTALL_DIR/bin/mp"
    echo "installed binary from MP_BIN=$MP_BIN"
    return 0
  fi
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64)              ASSET="mp-macos-arm64" ;;
    Darwin-x86_64)             ASSET="mp-macos-x64" ;;
    Linux-x86_64)              ASSET="mp-linux-x64" ;;
    Linux-aarch64|Linux-arm64) ASSET="mp-linux-arm64" ;;
    *)                         ASSET="" ;;
  esac
  if [ -n "$ASSET" ] && command -v curl >/dev/null 2>&1; then
    if curl -fsSL --retry 2 -o "$INSTALL_DIR/bin/mp.tmp" "$RELEASE_BASE/$ASSET"; then
      mv "$INSTALL_DIR/bin/mp.tmp" "$INSTALL_DIR/bin/mp"
      echo "installed prebuilt binary ($ASSET)"
      return 0
    fi
    rm -f "$INSTALL_DIR/bin/mp.tmp"
    echo "prebuilt binary unavailable — falling back to a source build"
  fi
  if command -v cargo >/dev/null 2>&1; then
    echo "building from source (cargo)..."
    (cd "$INSTALL_DIR" && cargo build --release -q)
    cp "$INSTALL_DIR/target/release/mp" "$INSTALL_DIR/bin/mp"
    return 0
  fi
  echo "mindpalace: no prebuilt binary for this platform and cargo is not installed"
  echo "  install Rust (https://rustup.rs) and re-run, or set MP_BIN=/path/to/mp"
  exit 1
}
get_binary
chmod +x "$INSTALL_DIR/bin/mp"

# 3. put `mp` on PATH: prefer an existing writable bin dir (override: MP_BIN_DIR)
BIN_DIR="${MP_BIN_DIR:-}"
if [ -z "$BIN_DIR" ]; then
  for d in /opt/homebrew/bin /usr/local/bin "$HOME/.local/bin"; do
    if [ -d "$d" ] && [ -w "$d" ]; then BIN_DIR="$d"; break; fi
  done
fi
[ -n "$BIN_DIR" ] || BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
ln -sf "$INSTALL_DIR/bin/mp" "$BIN_DIR/mp"
echo "linked mp -> $BIN_DIR/mp"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "NOTE: add $BIN_DIR to your PATH" ;;
esac

echo
echo "mindpalace installed. next:"
echo "  mp init -g        # create your wiki + auto-integrate agent tools"
