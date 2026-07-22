#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
EDITOR_DIR="${PROJECT_DIR}/editor"
GODOT="${GODOT:-godot4}"

# Build bridge in debug
echo "==> Building sstd-editor-bridge (debug)..."
cargo build -p sstd-editor-bridge 2>&1

BRIDGE_SRC="${PROJECT_DIR}/target/debug/libsstd_editor_bridge.so"
BRIDGE_DST="${EDITOR_DIR}/rust/libsstd_editor_bridge.so"
mkdir -p "${EDITOR_DIR}/rust"
cp "$BRIDGE_SRC" "$BRIDGE_DST"

# Ensure GDExtension list exists (required for extension loading in game mode)
GODOT_DIR="${EDITOR_DIR}/.godot/editor"
mkdir -p "$GODOT_DIR"
GODOT_EXT_LIST="${GODOT_DIR}/extension_list.cfg"
if [ ! -f "$GODOT_EXT_LIST" ]; then
    echo "res://rust/editor_bridge.gdextension" > "$GODOT_EXT_LIST"
fi

echo "==> Launching Godot editor..."
exec "$GODOT" --path "$EDITOR_DIR"
