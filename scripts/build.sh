#!/usr/bin/env bash
#===============================================================================
# Build script for SSTD World Editor
#
# Usage:
#   ./scripts/build.sh                    # debug build, no AppImage
#   ./scripts/build.sh --release          # release build, no AppImage
#   ./scripts/build.sh --release --appimage  # release build + AppImage
#   ./scripts/build.sh --appimage             # debug build + AppImage
#   ./scripts/build.sh --install-templates    # install Godot export templates
#
# Prerequisites:
#   - Godot 4.4+ installed (in PATH or ~/.local/bin/godot4)
#   - Rust toolchain (rustc, cargo)
#   - appimagetool (auto-downloaded if missing and --appimage is used)
#===============================================================================

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
EDITOR_DIR="${PROJECT_DIR}/editor"
BUILD_DIR="${PROJECT_DIR}/build"

GODOT="${GODOT:-godot4}"
APPIMAGETOOL_URL="https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"

# --- Parse flags ---
RELEASE=""
APPIMAGE=false
INSTALL_TEMPLATES=false

for arg in "$@"; do
    case "$arg" in
        --release|-r) RELEASE="--release" ;;
        --appimage)   APPIMAGE=true ;;
        --install-templates) INSTALL_TEMPLATES=true ;;
        *) echo "Unknown flag: $arg"; exit 1 ;;
    esac
done

CARGO_PROFILE="debug"
PRESET="Linux/X11"
if [ -n "$RELEASE" ]; then
    CARGO_PROFILE="release"
    PRESET="Linux/X11"
fi

# ------------------------------------------------------------------
# Step 0: Install Godot export templates (if requested)
# ------------------------------------------------------------------
if [ "$INSTALL_TEMPLATES" = true ]; then
    echo "==> Installing Godot export templates..."
    "$GODOT" --headless --export-templates
    echo "==> Export templates installed."
    exit 0
fi

# ------------------------------------------------------------------
# Step 1: Build Rust sstd-editor-bridge
# ------------------------------------------------------------------
echo "==> Building sstd-editor-bridge ($CARGO_PROFILE)..."
cargo build -p sstd-editor-bridge $RELEASE

BRIDGE_SRC="${PROJECT_DIR}/target/${CARGO_PROFILE}/libsstd_editor_bridge.so"
BRIDGE_DST="${EDITOR_DIR}/rust/libsstd_editor_bridge.so"
mkdir -p "${EDITOR_DIR}/rust"
cp "$BRIDGE_SRC" "$BRIDGE_DST"
echo "==> Copied bridge .so to editor/rust/"

# Ensure GDExtension list exists (required for extension loading in game mode)
GODOT_DIR="${EDITOR_DIR}/.godot/editor"
mkdir -p "$GODOT_DIR"
GODOT_EXT_LIST="${GODOT_DIR}/extension_list.cfg"
if [ ! -f "$GODOT_EXT_LIST" ]; then
    echo "res://rust/editor_bridge.gdextension" > "$GODOT_EXT_LIST"
fi

# ------------------------------------------------------------------
# Step 2: Verify export templates exist
# ------------------------------------------------------------------
if ! "$GODOT" --headless --export --quiet 2>/dev/null; then
    TEMPLATES_DIR="$HOME/.local/share/godot/export_templates"
    if [ ! -d "$TEMPLATES_DIR" ]; then
        echo "!! Godot export templates not found."
        echo "   Run: $0 --install-templates"
        echo "   Or manually: $GODOT --headless --export-templates"
        exit 1
    fi
fi

# ------------------------------------------------------------------
# Step 3: Export Godot project
# ------------------------------------------------------------------
echo "==> Exporting Godot project ($PRESET, $CARGO_PROFILE)..."
mkdir -p "$BUILD_DIR"

# Export to a temporary staging directory
STAGING_DIR="${BUILD_DIR}/sstd-editor-linux-x86_64.${CARGO_PROFILE}"
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR"

EXPORT_PATH="${STAGING_DIR}/sstd-editor.x86_64"
"$GODOT" --headless --export-release "$PRESET" "$EXPORT_PATH" \
    --path "$EDITOR_DIR"

echo "==> Godot export complete: $EXPORT_PATH"

# ------------------------------------------------------------------
# Step 4: Copy runtime data (bridge .so, default configs)
# ------------------------------------------------------------------
cp "${EDITOR_DIR}/rust/libsstd_editor_bridge.so" "${STAGING_DIR}/"
echo "==> Runtime data staged at: $STAGING_DIR"

# ------------------------------------------------------------------
# Step 5: Create AppImage (if requested)
# ------------------------------------------------------------------
if [ "$APPIMAGE" = true ]; then
    echo "==> Creating AppImage..."

    APPIMAGETOOL="${BUILD_DIR}/appimagetool"
    if [ ! -f "$APPIMAGETOOL" ]; then
        echo "   Downloading appimagetool..."
        curl -Lo "$APPIMAGETOOL" "$APPIMAGETOOL_URL"
        chmod +x "$APPIMAGETOOL"
    fi

    # Build AppDir structure
    APPDIR="${BUILD_DIR}/SSTD.Editor.AppDir"
    rm -rf "$APPDIR"
    mkdir -p "${APPDIR}/usr/bin"
    mkdir -p "${APPDIR}/usr/lib"

    cp "$EXPORT_PATH" "${APPDIR}/usr/bin/sstd-editor"
    cp "${BRIDGE_DST}" "${APPDIR}/usr/lib/"
    cp "${PROJECT_DIR}/LICENSE" "${APPDIR}/"

    # Desktop entry
    cat > "${APPDIR}/sstd-editor.desktop" <<EOF
[Desktop Entry]
Name=SSTD World Editor
Comment=Side-Scrolling Tower Defense — World Editor
Exec=usr/bin/sstd-editor
Icon=sstd-editor
Type=Application
Categories=Game;Development;
Terminal=false
EOF

    # Icon (create a placeholder 1x1 PNG if no icon exists)
    ICON_SRC="${EDITOR_DIR}/icon.png"
    if [ -f "$ICON_SRC" ]; then
        cp "$ICON_SRC" "${APPDIR}/sstd-editor.png"
    fi

    # AppRun entry point
    cat > "${APPDIR}/AppRun" <<'APPRUN'
#!/bin/bash
HERE="$(dirname "$(readlink -f "$0")")"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH:-}"
exec "${HERE}/usr/bin/sstd-editor" "$@"
APPRUN
    chmod +x "${APPDIR}/AppRun"

    APPIMAGE_OUT="${BUILD_DIR}/SSTD-Editor-${CARGO_PROFILE}-x86_64.AppImage"
    ARCH=x86_64 "$APPIMAGETOOL" "$APPDIR" "$APPIMAGE_OUT"

    echo "==> AppImage created: $APPIMAGE_OUT"
fi

# ------------------------------------------------------------------
# Summary
# ------------------------------------------------------------------
echo ""
echo "=========================================="
echo " Build complete: $CARGO_PROFILE"
echo "   Editor binary:   $EXPORT_PATH"
if [ "$APPIMAGE" = true ]; then
    echo "   AppImage:       $APPIMAGE_OUT"
fi
echo "=========================================="
