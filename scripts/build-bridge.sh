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
   "${PROJECT_DIR}/editor/rust/libsstd_editor_bridge.so.tmp"
# Atomic replace: the running editor already has the .so mapped; cp over the
# same inode leaves the mapped file truncated-and-rewritten, which segfaults
# the process that holds it at teardown (observed: exit 139 in the headless
# guard). mv swaps the directory entry; the old inode stays intact for mapped
# consumers and new processes open the fresh one.
mv "${PROJECT_DIR}/editor/rust/libsstd_editor_bridge.so.tmp" \
   "${PROJECT_DIR}/editor/rust/libsstd_editor_bridge.so"

echo "==> Copied to editor/rust/libsstd_editor_bridge.so"
