# Session Checkpoint — Prune Optimization (#51)

> Purpose: resume cold after a session switch or an abandoned session (e.g.
> model change -> brand-new session with zero prior context). Every section must
> be self-contained for that reader: committed history, exact in-flight step,
> next move in runnable order, all commands/numbers. Updated continuously, not
> only at the end. See `.opencode/AGENTS.md` "Session Progress Checkpoint
> (permanent)".

Last updated: 2026-08-17 (batch complete: native A3 + finalize committed and
documented, trunk `164d25c`/`5b6f25e`, wiki `ccffbf6`).

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

### Active step
None — this batch fully committed and documented.
- Trunk: `164d25c` (native A3 + finalize + truncation fix), `5b6f25e`
  (self-contained checkpoint rule).
- Wiki: `ccffbf6` — TDD_Tile-Deduplication.md v6 entry (native A3 + finalize,
  int-truncation quirk, 6792 ms real catalog, release-build requirement),
  status blockquote, section 4 cost-dialog wording, section 8 test list.
- Verified: cargo 101 pass (87 core + 14 bridge), GDScript suite failures=0,
  real-catalog breakdown profile (scan 3912 / a3 1672 / finalize 445 / full
  plan 6792 ms, dup_keys=982).

### Next move (proposed order)
1. Optional speedup / parity items are now tracked as feature requests:
   - #52: align native `flip_of_variants` with GDScript int-truncation semantics
     (boundary-diff parity; latent only, verified consistent on real data).
   - #53: move the prune gate (phase C) and variant-building (phase B) into the
     bridge for further speedup (current full plan is already 6.8s, under target).
2. Resume from either issue when the user picks one up; re-measure with
   `editor/tests/profile_real.gd` and confirm match streams / dup_keys=982 stay
   identical.

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
- Cargo: `cargo test -p sstd-editor-bridge` (14 pass; workspace total 92).
- Bridge rebuild (RELEASE — debug is 3-10x slower):
  `cargo build --release -p sstd-editor-bridge` then
  `cp target/release/libsstd_editor_bridge.so editor/rust/`.
- Real-catalog breakdown:
  `~/bin/godot4 --headless --path editor --script res://tests/profile_real.gd`
- Repos: trunk `sidescroll-towerdefense`, wiki `sidescroll-towerdefense.wiki`.
  Commits reference #51. `main.tscn` embeds MapEditor inline — edit both scenes.
