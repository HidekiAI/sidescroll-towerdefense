#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

GODOT_VERSION="4.4.1"
GODOT_URL="https://github.com/godotengine/godot/releases/download/${GODOT_VERSION}-stable/Godot_v${GODOT_VERSION}-stable_linux.x86_64.zip"
GODOT_ZIP="/tmp/godot4.zip"
GODOT_EXTRACT_DIR="/tmp/godot4"
GODOT_BIN="${HOME}/.local/bin/godot4"

echo "==> Installing Godot ${GODOT_VERSION}..."

# Download
if [ ! -f "$GODOT_ZIP" ]; then
    echo "    Downloading ${GODOT_URL}..."
    wget -q --show-progress "$GODOT_URL" -O "$GODOT_ZIP"
fi

# Extract
echo "    Extracting..."
mkdir -p "$GODOT_EXTRACT_DIR"
unzip -q -o "$GODOT_ZIP" -d "$GODOT_EXTRACT_DIR"

# Install to ~/.local/bin
mkdir -p "${HOME}/.local/bin"
cp "$GODOT_EXTRACT_DIR/Godot_v${GODOT_VERSION}-stable_linux.x86_64" "$GODOT_BIN"
chmod +x "$GODOT_BIN"

# Verify
echo "    Verifying..."
"$GODOT_BIN" --version

# Add to PATH if not already
if [[ ":$PATH:" != *":${HOME}/.local/bin:"* ]]; then
    echo "    NOTE: Add ~/.local/bin to your PATH if not already:"
    echo "          export PATH=\"\${HOME}/.local/bin:\${PATH}\""
fi

# Cleanup
rm -rf "$GODOT_EXTRACT_DIR" "$GODOT_ZIP"

echo "==> Godot ${GODOT_VERSION} installed at ${GODOT_BIN}"

# ---- Project skeleton ----
echo "==> Creating Godot editor project skeleton..."

EDITOR_DIR="${PROJECT_DIR}/editor"
mkdir -p "${EDITOR_DIR}/scenes"
mkdir -p "${EDITOR_DIR}/scripts"
mkdir -p "${EDITOR_DIR}/rust"

# project.godot
cat > "${EDITOR_DIR}/project.godot" << 'GDS'
; Engine configuration file.
; Godot 4.4+ project file.

name="SSTD World Editor"
description="Side-Scrolling Tower Defense — World Editor"
config_version=5

[application]
run/main_scene="res://scenes/main.tscn"
config/name="SSTD World Editor"
config/description="World Editor for SSTD"
config/features=PackedStringArray("4.4")
config/icon="res://icon.png"

[editor_plugins]
enabled=PackedStringArray()

[rendering]
renderer/rendering_method="gl_compatibility"
window/viewport_width=1920
window/viewport_height=1080
GDS

# Default environment
cat > "${EDITOR_DIR}/default_env.tres" << 'GDS'
[gd_resource type="Environment" load_steps=2 format=3]

[sub_resource type="ProceduralSky" id=1]

[resource]
background_mode = 1
background_sky = SubResource(1)
GDS

# Placeholder icon (1x1 pixel PNG)
echo -n "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==" | base64 -d > "${EDITOR_DIR}/icon.png"

# Main scene — bootstrap GDScript
cat > "${EDITOR_DIR}/scenes/main.tscn" << 'GDS'
[gd_scene load_steps=2 format=3 uid="uid://dtn6qocve6qai"]

[ext_resource type="Script" path="res://scripts/main.gd" id="1"]

[node name="Main" type="Control"]
layout_mode = 3
anchors_preset = 0
script = ExtResource("1")
GDS

# Script stubs with selector pattern
cat > "${EDITOR_DIR}/scripts/main.gd" << 'GDS'
extends Control

var current_tab := 0
var is_dirty := false
var project_path := ""

func _ready() -> void:
    pass

func new_project() -> void:
    pass

func open_project(path: String) -> void:
    pass

func save_project() -> void:
    pass

func save_project_as(path: String) -> void:
    pass

func export_json(path: String) -> void:
    pass

func import_json(path: String) -> void:
    pass
GDS

# Scene stubs for each editor tab
for scene in terrain_editor entity_editor map_editor placement_editor simulator; do
    cat > "${EDITOR_DIR}/scenes/${scene}.tscn" << GDS
[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://scripts/${scene}.gd" id="1"]

[node name="$(echo ${scene} | sed 's/_/ /g' | sed 's/\b\(.\)/\u\1/g' | sed 's/ //g')Editor" type="Control"]
layout_mode = 3
anchors_preset = 0
script = ExtResource("1")
GDS

    cat > "${EDITOR_DIR}/scripts/${scene}.gd" << GDS
extends Control

func _ready() -> void:
    pass
GDS
done

# GDExtension bridge stub
cat > "${EDITOR_DIR}/rust/editor_bridge.gdextension" << 'GDS'
[configuration]
entry_symbol = "gdextension_init"

[libraries]
linux.x86_64 = "res://rust/libsstd_core_editor.so"
windows.x86_64 = "res://rust/sstd_core_editor.dll"
GDS

echo "==> Editor skeleton created at ${EDITOR_DIR}"
echo "==> Setup complete. Run: godot4 --path ${EDITOR_DIR}"
