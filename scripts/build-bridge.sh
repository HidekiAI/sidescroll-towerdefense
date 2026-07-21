#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "==> Building sstd-editor-bridge..."
cargo build -p sstd-editor-bridge "$@"

PROFILE="debug"
for arg in "$@"; do
    if [ "$arg" = "--release" ] || [ "$arg" = "-r" ]; then
        PROFILE="release"
    fi
done

cp "${PROJECT_DIR}/target/${PROFILE}/libsstd_editor_bridge.so" \
   "${PROJECT_DIR}/editor/rust/libsstd_editor_bridge.so"

echo "==> Copied to editor/rust/libsstd_editor_bridge.so"
