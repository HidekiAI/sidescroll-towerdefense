# Session Checkpoint — Editor Packaging & Persistence (#54)

> Purpose: resume cold after a session switch or an abandoned session (e.g.
> model change -> brand-new session with zero prior context). Every section must
> be self-contained for that reader: committed history, exact in-flight step,
> next move in runnable order, all commands/numbers. Updated continuously, not
> only at the end. See `.opencode/AGENTS.md` "Session Progress Checkpoint
> (permanent)".

Last updated: 2026-08-18. #54 M1 (relocate `res://` writes to `user://data`) is
IMPLEMENTED and suite-green, pending commit. See "Active step" for the runnable
resume point.

## Objective

Package the editor as an installer (issue #54). First milestone M1: make the
runtime read-only-friendly by relocating every `res://` write to
`user://data`, so a packaged build with read-only `res://` (export/pck) works.
M2 (export/pck/.so/AppImage/deb/CI) is deferred.

## Where we are

### Done and committed (git history)
- Save/load bug batch #47/#48/#50: closed, committed `dfa9ca8`. Regression
  fixtures live in `editor/tests/` (`_test_placement_editor_tile_bank_sync`,
  `_test_placement_records_last_map_path`, `repro_blank_world.gd`).
- #55 config-backed defaults (main.gd seed/getters `defaults.*`, map/placement
  dialogs, fake_bridge.gd, `_test_config_defaults_seed`): committed `9f33983`,
  left open for review.
- #52 native `flip_of_variants` int-truncation parity: committed `28c3384`
  (bridge) + wiki TDD v7 `303c1b7`. Discovery: the [tol, tol+1) acceptance
  window is UNREACHABLE for `flip_of_variants` because `_diff_capped`
  early-exits to `tol+1.0`. Bridge suite 16/16, workspace cargo 103/103.
- #53 v8 native scan front-end: committed (code: `scan_signatures`/
  `scan_projections` native f64, `_prune_bytes`, `_exact_matches_bridge_stream`,
  a3/finalize reuse, `_canonical_coarse_bytes`, parity tests; wiki TDD v8
  `c312f7c`). Real catalog full `_build_prune_plan` 6507 -> **4551 ms**,
  `dup_keys=982` identical, cargo 105/105.
- Wiki `TDD_gRPC-Architecture.md`: added "EditorRemote — future consideration
  (NOT implemented)" (embedded EditorService gRPC for GUI-feature E2E), commit
  `cdc2396`. Tracked as deferred feature issue #58.
- Editor E2E harness issue #56 created (Playwright, browserless-by-construction,
  blocked on #40 documented contract). Not started.

### In-flight — #54 M1 (IMPLEMENTED, suite green, NOT yet committed)
- `editor/scripts/main.gd`: `USER_DATA_DIR := "user://data"`,
  `DEFAULT_WORLD_PATH := "user://data/world.json"`, `user_data_dir()` helper
  (globalized + mkdir), `_init_config_db` writes config DB under
  `user://data/config/sstd_config.sqlite3`. Config keys unchanged:
  `defaults.world_json_path`, `defaults.world_file_name`, `defaults.file_dialog_dir`.
- `editor/scripts/map_editor.gd`: `_tiles_write_dir()` (user://data/tiles,
  creates dir), `get_tile_image` reads user dir first then built-in res://
  fallback, `_register_tile_image` writes PNG to user dir, `_load_stamp_catalog`
  precedence [user dir, res://], prune delete/est sites use `_tiles_write_dir()`.
- `editor/tests/test_screen_store.gd`: added `_test_user_data_relocation`
  (config DB under user://data/config, stamp PNG under user://data/tiles, no
  res:// write); added `_wipe_tile_artifact(key)` helper wiping BOTH
  user://data/tiles and res://assets/tiles copies; updated stale `res://world.json`
  expectations in `_test_config_defaults_seed` to `user://data/world.json`;
  cleaned up `_test_world_reopen_not_blank`/`_test_placement_editor_tile_bank_sync`
  (world restore persists the user:// PNG, so cleanups must wipe both).
- **Flake fixed (root cause)**: world `_load_world_package` ->
  `_register_tile_image` writes a user://data/tiles PNG that older cleanups never
  removed; a later suite run's fresh editor picked it up via `_load_stamp_catalog`
  and "private tile absent from disk/catalog before load" failed. `_wipe_tile_artifact`
  now clears both locations.
- **Persistence matrix (documented, wiki TDD_World-Editor.md, UNCOMMITTED)**:
  | Path | Value | SQLite-configurable? |
  |---|---|---|
  | ConfigStore DB | `user://data/config/sstd_config.sqlite3` | No |
  | Boot world pointer | `defaults.world_json_path` = `user://data/world.json` | Yes |
  | Save-dialog file name | `defaults.world_file_name` = `world.zip` | Yes |
  | Save/load dialog start dir | `defaults.file_dialog_dir` = `""` | Yes |
  | Tile/stamp cache | `user://data/tiles` | No |
  Data base dir override: tracked as #57 (CLI-arg-only, future/deferred).
- Verification: full GDScript suite `=== done, failures=0 ===`; syntax check
  passes on `tests/test_screen_store.gd`; no Rust changes this round (`.so` not
  rebuilt).

### Design decision (recorded 2026-08-17): Simulator tab is pure gRPC
- Simulator state/stepping live entirely in a Rust process (`sstd-headless`,
  per TDD_gRPC-Architecture.md). Port 50052, chunky round-trips. Not the
  godot-rust FFI hot-loop tier.
- **Caveat (must decide when implementing)**: Godot `HTTPClient` is HTTP/1.1 and
  cannot speak gRPC (HTTP/2). The Godot tab needs a hop: (a) `sstd-editor-bridge`
  hosts a tonic CLIENT, or (b) `sstd-headless` exposes a second
  JSON-over-unix-socket/WebSocket face. Consistent with #40's "transport-agnostic,
  gRPC is one face".
- Status: design decision only. Current crates: `sstd-core` + `sstd-editor-bridge`.
  NO simulator crate, NO `sstd-headless` binary yet. Simulator tab exists as
  `simulator.tscn` + `editor/scripts/simulator.gd` (stub).

### Design consideration (recorded 2026-08-18): EditorRemote (deferred, #58)
- If remote E2E of the Godot GUI editor (map/tile/terrain/editor) were wanted:
  an EditorService gRPC server EMBEDDED in the editor process (godot-rust bridge
  hosting tonic on the editor port) exposing editor/data operations, NOT UI
  widget automation. Off-thread tonic workers must defer RPCs onto the Godot
  main thread, apply against live state, reply via channel (1 op/frame; Result
  replies). Not via OS GUI automation nor browser/WASM (native `.so` can't load
  in WASM — see #56). Simulator stays separate (port 50052). Documented in
  wiki TDD_gRPC-Architecture.md "EditorRemote"; tracked as #58. NOT implemented.

## Active step
1. Commit #54 M1 batch: `editor/scripts/main.gd`, `editor/scripts/map_editor.gd`,
   `editor/tests/test_screen_store.gd`, `docs/SESSION-CHECKPOINT.md` —
   referencing #54. Post issue comment on #54.
2. Commit wiki `TDD_World-Editor.md` (persistence matrix section) under the wiki
   repo, referencing #54.
3. Refresh `.opencode/AGENTS.md` "Session Progress" table.

## Next move (proposed order)
1. #54 M1 commit + comments (above). Then M2 (packaging/CI) is deferred; mark
   that on #54.
2. Outstanding tracked work (no active branch):
   - Simulator-gRPC: decide transport face (a) tonic client in
     `sstd-editor-bridge` vs (b) JSON-over-unix-socket face on `sstd-headless`,
     then create simulator crate + headless binary + protobuf contract; fill
     TDD_Simulator-Service-Contract.md TBDs.
   - #56 Playwright E2E: harness scaffolding can land anytime; full coverage
     blocked on #40 (documented service contract) + simulator contract.
   - #57 CLI data-dir override (deferred); #58 EditorRemote (deferred).
   - Optional: #53 follow-up — native bulk `canonical_coarse` to cut a3_discover
     from ~1.6 s toward ~0.8 s (not needed for acceptance).

## Key numbers / constants
- `STAMP_CELL=32`, `TILE_BYTES=4096`, `STAMP_TOLERANCE=4.0`.
- Rust bridge constants: `COARSE_BYTES = 8*8*4` (256), `STAMP_CELL=32`.
- `fp_hash` = FNV-1a 32-bit. Golden values: empty=2166136261,
  `[0x00,0x01,0x02,0x03]`=3282719153, `[0xff;8]`=1823345245.
  a3 neighbor hashes over all-zero/all-ff canon: 257 distinct = COARSE_BYTES+1.
- Gate: `_grayscale_sig_near` over 256-byte sigs, diff <= 96.
- `BRIDGE_MIN_PAIRS=512`, `BRIDGE_MAX_WORKERS=32` (host has 32 cores).
- A3: index keyed on 8-bit `_canonical_coarse` (256 bytes), neighbour delta +/-1,
  `A3_K=8`, `A3_LRU_CAP=64`, <=512 neighbour hashes.
- Deep tier: `DEEP_CONFIRM_THRESHOLD=2500`, `DEEP_YIELD_EVERY=500`.
- GDScript floats are 64-bit doubles (probe-verified:
  `0.299*16+0.587*16+0.114*16` = `15.99999999999999822` -> int 15). Bridge
  ports MUST use f64; an f32 port swung boundary sums by +1. Recorded in wiki
  TDD v8.

## Commands
- Editor regression tests:
  `~/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd`
  -> expect `=== done, failures=0 ===`.
- Script syntax check:
  `~/bin/godot4 --headless --path editor --check-only --script res://scripts/map_editor.gd`
  (also for `res://tests/test_screen_store.gd`).
- Cargo: `cargo test` (workspace; 105 pass at #53).
- Bridge rebuild (RELEASE — debug is 3-10x slower):
  `cargo build --release -p sstd-editor-bridge` then
  `cp target/release/libsstd_editor_bridge.so editor/rust/`.
- Real-catalog breakdown:
  `~/bin/godot4 --headless --path editor --script res://tests/profile_real.gd`
- Repos: trunk `sidescroll-towerdefense`, wiki `sidescroll-towerdefense.wiki`.
  `main.tscn` embeds MapEditor inline — edit both scenes.