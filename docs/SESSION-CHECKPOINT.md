# Session Checkpoint — Editor Core & Persistence (post-#67)

> IN-FLIGHT (2026-08-27, after #64 CLOSED): investigating direct terrain paint +
> discovered a sub_tile_mask float bug. Plan: `docs/PLAN-2026-08-27-terrain-brush-and-subtile-float.md`
> (created, NOT yet committed). Work is documented there; see its "Fix strategy"
> and "Feature" sections. Blocked-for-now on: (a) wheel brush = powers-of-2 NxN
> uniform stamp (user confirmed 1,2,4,8; top-left origin; ALL same terrain); (b)
> sub_tile_mask float producer fix + serde reader tolerance + world re-save.
> Do NOT re-derive: Godot JSON.stringify writes float Variants as `15.0`; the Rust
> u8 deserializer rejects that -> `invalid type: floating point 15.0, expected u8`.


> Purpose: resume cold after a session switch or an abandoned session (e.g.
> model change -> brand-new session with zero prior context). Every section must
> be self-contained for that reader: committed history, exact in-flight step,
> next move in runnable order, all commands/numbers. Updated continuously, not
> only at the end. See `.opencode/AGENTS.md` "Session Progress Checkpoint
> (permanent)".

Last updated: 2026-08-27. #64 gRPC editor control DONE + **CLOSED** (Phase 1
`4e40cae`, Phase 2 `b6cbdaa`, wiki `59f17ab`, live all-tab wire verification
passed on DISPLAY=:0). #67 entity/terrain def overrides shipped `9d78881` with
regression test `e1d18af` (real world.zip merge, 11 assertions green) — still
OPEN pending wiki TDD + close. #61 prune crash FIXED `6103ba4`. #62 dialog
filter re-fixed `6103ba4`. See "Active step" + "Next move" for resume state.

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
- #59 native bulk canonical-coarse: committed `b8c357e` (a3_discover
  1671->873 ms; dup_keys=1328 consistent; cargo 107 pass; parity 0 mismatches).
- #60 dirty-save prompt: committed `aa3bc6f` (`_confirm_save_dirty` Save/Discard/
  Cancel modal, quit + import + new/open gated, paint/entity-ops mark dirty).

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
1. **#61 prune crash — DONE, committed `6103ba4`, issue closed.**
   - Root cause: after first prune, `_drop_tiles()` shrinks `_stamp_catalog`
     while cached `_prune_bytes`/`_canonical_flat` kept pre-prune sizes, and
     `_stamp_fp_index` held stale indices one past the shrunken catalog. Second
     scan's `_a3_discover` fed those stale indices to `_uf_union`/
     `_finalize_prune_plan` -> `Invalid access of index 2017`.
   - Fix: `_execute_prune` clears `_prune_bytes`+`_canonical_flat` after
     `_drop_tiles`; `_a3_discover` skips `other >= _stamp_catalog.size()`;
     `_finalize_prune_plan` emits journal log of catalog/uf_parent/cache sizes.
2. **#62 dialog filter default — re-fixed `6103ba4`, reopened, needs re-close.**
   - Original `_apply_world_default_filter` wrote `FileDialog.current_filter`
     which DOES NOT EXIST in Godot 4.4 (both string and int throw "Invalid
     assignment of property"). Replaced with filter-ORDERING: `.zip` first when
     package mode, `.json` first in legacy mode. Helper removed; inlined into all
     4 dialogs (map_editor _on_save/_on_import, placement_editor _on_save/_on_import_map).
3. **#67 entity/terrain def overrides — DONE, committed `9d78881`.**
   - Rust `EntityDefOverride`/`TerrainTypeOverride` prototype-based partial
     patches (merge onto framework def, deny_unknown_fields, enum type checks),
     `TerrainOverridesFile.merge_all`/`EntityOverridesFile`, exported in lib.rs.
   - world_archive.gd writes `terrain_overrides.json`/`entity_defs.json`;
     entity_editor.gd: `set_framework_entities`/`apply_world_entity_defs`/
     `collect_world_entity_defs` (removed hardcoded `_add_defaults`).
   - editor/default_package/: framework prototypes terrain_types.json (7 types)
     + entity_defs.json (8 defs) + 7 tile PNGs.
   - cargo workspace 122 pass. Issue #67 OPEN — close on next pass with final
     GUI/world-roundtrip confirm.
4. **#64 Phase 1 headless all-tab capture — DONE, committed `4e40cae`.**
   - `editor/capture_all_tabs.gd` (SceneTree entrypoint) instantiates main.tscn,
     iterates `TAB_NAMES`, saves `res://captures/{tile,entity,map,placement,simulator}.png`.
   - Verified 5 valid 1152x648 PNGs against `DISPLAY=:0` (Dummy renderer headless
     cannot capture — null viewport texture).

5. **#64 Phase 2 gRPC editor control — IMPLEMENTED (not yet committed).**
   - NEW `crates/sstd-grpc`: `proto/editor.proto` (SwitchTab/CaptureScreenshot,
     EditorService), `build.rs` (tonic-build + prost-build with VENDORED protoc
     via `protobuf-src` because no system protoc on host), `lib.rs` exposing
     `EditorServer` (tonic), `EditorCommand` (mpsc + oneshot bridge),
     `spawn_server(addr) -> mpsc::Receiver`, `TAB_NAMES`, `resolve_tab_index`,
     `SwitchTabResult`/`ScreenshotResult`. `#[tokio::test]` unit + 2 full-wire
     e2e tests.
   - Bridge `crates/sstd-editor-bridge/src/lib.rs` (new fields `grpc_tx?`,
     `grpc_rx?`, `tab_container: Option<Gd<TabContainer>>`; 5 `#[func]`s):
     `set_tab_container`, `switch_tab`, `capture_screenshot`, `start_grpc_server`,
     `poll_grpc_commands`. Key APIs: `Gd::upcast::<Texture2D>()` +
     `Texture2D::get_image()` + `Image::save_png_to_buffer()` (returns
     `PackedByteArray`, not Option); capture resolves from `tab_container`'s
     viewport (NOT `self.base` — godot-rust 0.5 `Base` has no Deref/get_viewport).
     `poll_grpc_commands` takes each command OUT of the channel before `&mut self`
     calls to satisfy the borrow checker. Adds deps: `sstd-grpc`, `tokio`(sync),
     `base64`.
   - `editor/scripts/main.gd`: `GRPC_ENABLED=true`, `GRPC_PORT=50051`,
     `_start_grpc_server_if_enabled()` calls `set_tab_container` +
     `start_grpc_server`; new `_process(delta)` polls `poll_grpc_commands`.
   - Verified: cargo workspace 128 pass (102 core + 20 bridge + 4 grpc unit +
     2 grpc e2e). Headless smoke: bridge loads, `[sstd-bridge] grpc: tab_container
     registered`, `start_grpc_server(50051) -> {"ok": true, "port": 50051}`,
     exit 0.
   - COMMITTED `b6cbdaa` (amended with `crates/sstd-grpc/examples/grpc_smoke.rs`,
     a live smoke client: `cargo run -p sstd-grpc --example grpc_smoke [port] [tab]`).
   - LIVE VERIFICATION DONE + **#64 CLOSED** (2026-08-27). With DISPLAY=:0 woken,
     editor launched, server confirmed listening on 127.0.0.1:50051. Drove all 5
     tabs over the wire: each switch ok=true (idx 0-4), each capture a valid
     non-empty PNG at 1152x648 (tile 46KB, entity 51KB, map 575KB, placement
     762KB, simulator 11KB). Tabs visibly switched in the live editor. Verified
     per wiki TDD_GRPC-Editor-Control. Close comment posted, issue #64 CLOSED.
     NOTE: live verification needs a real display; from a locked/screensaver
     session Godot reports "X11 Display is not available" — wake the desktop
     first (Xvfb crashes on this host, headless/Dummy cannot capture).

## Next move (proposed order)
1. **#67 — author wiki TDD + close.** #64 is DONE and CLOSED. #67 code shipped
   `9d78881`, regression test `e1d18af` validated the real-world merge (terrain:
   air sub_tile_mask 0->15, dirt->0, partial-patch semantics; entity: 8 defs by
   key, no dups; 11 assertions green). Remaining: author/provide wiki TDD page
   (e.g. TDD_Entity-and-Terrain-Overrides) + Home/TODO entry, post closing comment
   citing the wiki, then close #67.
2. **#62 — GUI preselect re-verify then close.** Re-verify functionally that Load
   defaults to `.zip` when the world package is active (the `current_filter`
   property DOES NOT EXIST in Godot 4.4; the fix uses filter-ORDERING instead).
   Needs a real-display session to click through. Then close #62.
3. (Optional) Add a grpcurl CI loop for the TabNames x CaptureScreenshot flow
   referenced in the TDD; currently verified via the Rust smoke client instead.

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