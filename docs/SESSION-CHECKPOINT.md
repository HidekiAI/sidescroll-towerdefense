# Session Checkpoint — Prune Optimization (#51)

> Purpose: resume cold after a session switch or an abandoned session (e.g.
> model change -> brand-new session with zero prior context). Every section must
> be self-contained for that reader: committed history, exact in-flight step,
> next move in runnable order, all commands/numbers. Updated continuously, not
> only at the end. See `.opencode/AGENTS.md` "Session Progress Checkpoint
> (permanent)".

Last updated: 2026-08-17 (save/load bug batch #47/#48/#50 complete: committed
`dfa9ca8`, all three issues closed. #51 prune work fully shipped. Active work:
#55 config-backed defaults — in flight, main.gd partial changes uncommitted).
See "Active step" for the runnable resume point.

## Objective

Make "Prune Duplicates" fast enough that a ~3000-tile catalog scan is usable
(10s target), while keeping the fast tier exact-diff identical to the serial
reference implementation. Tracked in issue #51.

Real-catalog baseline (v3): 2718 tiles, `candidate_count=475445`,
`gate_survivors=403290`, `exact_comparison_count=403290`,
`scan_elapsed_ms=535776` (~8.9 min serial), ~197 ms/tile.

## Where we are

### Done and committed
- A3 neighbour scan (+/-1 on the 8-bit canonical coarse index) hardens the fast
  tier; deep tier (Approach B) added with its own button. `e31eb0f`.
- Cost warning dialog (`PRUNE_WARN_THRESHOLD=500`, `PRUNE_EST_MS_PER_TILE=197`)
  + projection-window binary search (`_projection_lower_bound`). `22e6d4e`.
- **Rust bridge parallel exact-diff** (`SstdBridge.scan_exact_matches`,
  `std::thread::scope` workers in `crates/sstd-editor-bridge/src/lib.rs`).
  GDScript marshals flat `variants_flat`/`bytes_flat`/`pair_stream` buffers in
  `_exact_matches_bridge`; falls back to serial `_exact_matches_slice` when no
  bridge or `< BRIDGE_MIN_PAIRS` (512). Measured **~131x** on 19900 pairs
  (89.3s serial -> 683ms, 32 cores), identical match stream. `775be1c`.
- Wiki TDD `TechnicalDesign/TDD_Tile-Deduplication.md` updated (v5 entry).
- Full suite green: GDScript `=== done, failures=0 ===`; cargo 92 tests.

### This batch (A3 + finalize native, ~7.3x total speedup on the full plan)
Real-catalog profile (2999 tiles, `editor/tests/profile_real.gd`), final
breakdown with the release .so:
- Phase scan (projection candidates): 3912 ms
- Phase A3 (`_a3_discover` -> native `a3_neighbor_hashes`): 1672 ms, 0 matches
- Phase finalize (native `flip_of_bytes`): 445 ms, dup_keys=982
- **Full `_build_prune_plan`: 6792 ms** (was ~29.5s debug / ~26.5s pre-native
  release; down from ~535s original baseline). dup_keys=982 matches the serial
  reference.
- A3 -> native: 12s (GDScript) -> 5.3s (debug .so) -> 1.6s (release .so).
- finalize -> native: 5.9s -> 445ms (bytes_cache + `flip_of_bytes`).
- **Release build is mandatory**: the debug .so is 3-10x slower
  (scan 8.6s->4.2s, a3 nh 4.3s->0.38s). Build command changed (see Commands).

### Critical finding: GDScript int-truncation in `_flip_of_bytes`
- GDScript `_flip_of_bytes` (map_editor.gd:1204) declares
  `var best_diff := 0x7fffffff` (int), so `best_diff = d` TRUNCATES float diffs.
  A variant whose diff is in `[tol, tol+1)` (e.g. 4.17 with tol 4.0) still
  matches. Native `flip_of_bytes_impl` must replicate exactly:
  `best_diff: i32`, `best_diff = d as i32`, final test `best_diff as f64 <= tol`.
- Without this, native reported dup_keys=490 vs GDScript 982 (the f64 port
  was stricter). After the fix: pair-for-pair parity on the real catalog
  (checked=1228, native=982, gdscript=982, 0 mismatches).
- Related latent issue: GDScript `_flip_of_variants` (line 1167, serial scan
  path) has the same int best_diff truncation, but native `flip_of_variants`
  (used by `scan_exact_matches`) still uses f64. Verified consistent on the real
  catalog (exact=133231) and the 600-tile probe, but a boundary diff could
  diverge on future catalogs. Follow-up candidate.
- `flip_of_bytes` return codes (wiring in `_finalize_prune_plan`):
  0=none, 1=identity, 2=h, 4=v, 8=hv; flip_h = code 2 or 8, flip_v = code 4 or 8.

## Design decision (recorded 2026-08-17): Simulator tab is pure gRPC

Context: the Rust<->Godot4 boundary question (from the #51 bridge work).
`SstdBridge` is godot-rust FFI (in-process) and that stays for the editor's
per-frame hot loops. But the **Simulator tab in the editor will NOT be a
godot-rust sim class** — the simulator is the natural pure-gRPC boundary:

- Sim state and stepping live entirely in a Rust process (`sstd-headless`,
  per TDD_gRPC-Architecture.md line 72). Deterministic, isolated from the Godot
  main loop; batch eval / AI training / headless CI use the SAME server.
- Simulator gRPC (port 50052, per `.opencode/AGENTS.md` "gRPC Integration"):
  chunky `step N ticks / inject / query world` round-trips, so loopback gRPC
  latency is negligible — the "must be in-process FFI" argument that justified
  `SstdBridge` for byte-diff hot loops does NOT apply here.
- No shared `Arc<RwLock<EditorState>>` needed on the sim tier: the sim owns its
  world state in Rust. The #40 EditorState gap stays scoped to the EDITOR gRPC
  (50051) tier.
- **Caveat (must decide when implementing)**: Godot's `HTTPClient` is HTTP/1.1
  and cannot speak gRPC (HTTP/2). The Godot tab therefore needs a hop:
  (a) `sstd-editor-bridge` hosts a tonic CLIENT (`SstdBridge.step_sim(...)`),
  or (b) `sstd-headless` exposes a second JSON-over-unix-socket/WebSocket
  transport face — consistent with #40's "transport-agnostic, gRPC is one face".
- Status: design decision, not yet implemented. Tracked via comment on #40.
  Current crates: only `sstd-core` + `sstd-editor-bridge`; NO simulator crate,
  NO `sstd-headless` binary yet. Simulator tab exists as `simulator.tscn` +
  `editor/scripts/simulator.gd` (stub).

### Active step
**#55 config-backed defaults — DONE, committed `9f33983`, comment on #55.** See
"Bug fixes logged" for the closed #47/#48/#50 batch.

**#52 native `flip_of_variants` truncation parity — implemented, verified,
NOT yet committed.**
- Ported `best_diff: i32` + `d as i32` into `flip_of_variants`
  (`crates/sstd-editor-bridge/src/lib.rs`), mirroring GDScript `_flip_of_variants`.
- **Discovery (documented in wiki TDD v7 + issue #52)**: the [tol, tol+1)
  acceptance window is UNREACHABLE for `flip_of_variants` because it scores via
  `_diff_capped`, which early-exits to `tol+1.0` whenever any running mean
  exceeds tol — returned diff is always `<= tol` or exactly `tol+1.0`. Int vs
  f64 accumulation are observationally identical. Verdict: the port is
  defense-in-depth (GDScript verbatim parity), not a behavior change.
- New cargo tests: `flip_of_variants_truncates_best_diff_like_gdscript` (capped
  path must REJECT an uncapped diff in (4.0, 5.0), as GDScript does) +
  `flip_of_variants_still_matches_below_tolerance`. Bridge suite 16/16,
  workspace cargo 103/103, GDScript suite `failures=0` (native vs serial
  identical), real-catalog profile: full plan 6365 ms, `dup_keys=982` unchanged.
- Release .so rebuilt and copied to `editor/rust/`.
- NOT yet: commit (bridge lib.rs + checkpoint); post issue comment on #52; the
  wiki TDD v7 entry is written but not committed (wiki repo).

### Bug fixes logged this continuation
- **#47** (placement editor blank after opening a world.zip) and **#48** (blank
  world after quit+reopen) root causes + fixes are recorded permanently in the
  issue comments (`gh issue view 47`, `gh issue view 48`), committed in
  `dfa9ca8`, and both issues closed. #50 was already closed.
- Save/load regression fixtures keep living in `editor/tests/`:
  `_test_placement_editor_tile_bank_sync`, `_test_placement_records_last_map_path`,
  `repro_blank_world.gd`.

### Next move (proposed order)
1. Commit #52 batch (bridge lib.rs, SESSION-CHECKPOINT under the code repo;
   `TDD_Tile-Deduplication.md` under the wiki repo) referencing #52; post a
   comment on #52.
2. Outstanding tracked work (no active branch):
   - #53: move prune gate (phase C) + variant building (phase B) into the bridge
     for further speedup (optional; full plan already 6.4s, under target).
   - #54: packaged installer — relocate `res://` writes to `user://` first.
   - Simulator-gRPC: decide transport face (a) tonic client in
     `sstd-editor-bridge` vs (b) JSON-over-unix-socket face on `sstd-headless`,
     then create simulator crate + headless binary + protobuf contract; fill
     TDD_Simulator-Service-Contract.md TBDs (see design decision below).

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
- GDScript threads measured useless for the diff loop (0.72x shared, 1.77x
  private) — the bridge is the honest 32-core path.

## Commands
- Editor regression tests:
  `~/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd`
  -> expect `=== done, failures=0 ===`.
- Script syntax check:
  `~/bin/godot4 --headless --path editor --check-only --script res://scripts/map_editor.gd`
- Cargo: `cargo test -p sstd-editor-bridge` (14 pass; workspace total 101).
- Bridge rebuild (RELEASE — debug is 3-10x slower):
  `cargo build --release -p sstd-editor-bridge` then
  `cp target/release/libsstd_editor_bridge.so editor/rust/`.
- Real-catalog breakdown:
  `~/bin/godot4 --headless --path editor --script res://tests/profile_real.gd`
- Repos: trunk `sidescroll-towerdefense`, wiki `sidescroll-towerdefense.wiki`.
  Commits reference #51. `main.tscn` embeds MapEditor inline — edit both scenes.
