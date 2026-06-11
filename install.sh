#!/bin/sh
# mindpalace installer
#   curl -fsSL https://raw.githubusercontent.com/rootnix/mindpalace/main/install.sh | sh
set -eu

REPO="${MP_REPO:-https://github.com/rootnix/mindpalace.git}"
INSTALL_DIR="${MP_INSTALL_DIR:-$HOME/.local/share/mindpalace}"

command -v git >/dev/null 2>&1 || { echo "mindpalace: git is required"; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo "mindpalace: python3 is required"; exit 1; }

if [ -d "$INSTALL_DIR/.git" ]; then
  echo "updating existing install at $INSTALL_DIR"
  git -C "$INSTALL_DIR" pull --ff-only -q
else
  echo "installing to $INSTALL_DIR"
  mkdir -p "$(dirname "$INSTALL_DIR")"
  git clone --depth 1 -q "$REPO" "$INSTALL_DIR"
fi
chmod +x "$INSTALL_DIR/bin/mp" "$INSTALL_DIR"/integrations/claude/mindpalace/hooks/*.sh

# put `mp` on PATH: prefer an existing writable bin dir
BIN_DIR=""
for d in /opt/homebrew/bin /usr/local/bin "$HOME/.local/bin"; do
  if [ -d "$d" ] && [ -w "$d" ]; then BIN_DIR="$d"; break; fi
done
[ -n "$BIN_DIR" ] || { BIN_DIR="$HOME/.local/bin"; mkdir -p "$BIN_DIR"; }
ln -sf "$INSTALL_DIR/bin/mp" "$BIN_DIR/mp"
echo "linked mp -> $BIN_DIR/mp"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "NOTE: add $BIN_DIR to your PATH" ;;
esac

echo
echo "mindpalace installed. next:"
echo "  mp init -g        # create your wiki + auto-integrate agent tools"
