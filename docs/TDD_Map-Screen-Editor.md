# TDD: Map Screen Editor — Stamp Engine & Performance

> **Status**: Implemented
> **Related**: `map_editor.gd`, `tile_grid_display.gd`, `editor/tests/test_screen_store.gd`
> **Root fix**: [Issue #42](https://github.com/anomalyco/SSTD/issues/42)

## 1. Objective

The map screen editor lets the user stamp a large reference image (e.g. 1088px wide)
onto the tile grid as a brush of deduped 32×32 tiles. This doc captures how stamping is
implemented, why a 1088px stamp originally froze the editor, and the performance/animation
guarantees that now keep the main loop responsive.

## 2. Stamp Pipeline

```
             ┌──────────────────────────────────────────────────────────────┐
 user        │ _on_stamp_import_file   _stamp_grid_input        _commit_stamp │
 selects ────► load PNG ────────────► define origin ────► slice+dedup+write    │
 image       └──────────────────────────────────────────────────────────────┘
                              │
                              ├─► _ensure_tile(cell) → catalog + PNG on disk
                              ├─► _tiles[gx,gy] = tile key
                              ├─► _register_stamp_brush → TileSetGrouping
                              └─► _record_stamp_map → stamp metadata (undo)
```

- Cell size: `STAMP_CELL = 32`. A stamp's dimensions are sliced into
  `ceil(w/32) × ceil(h/32)` cells; partial edge cells are `blit_rect`'d and only kept
  when `_has_ink` (non-empty).
- Each cell is deduped against the on-disk catalog (`assets/tiles/stamp_*_32x32.png`).
  A matched cell records its `flip_h`/`flip_v` variant instead of writing a new PNG.

## 3. Dedup by Orientation-Invariant Fingerprint

### 3.1 Original bug (O(N²) → hang)

`_find_match` used to scan the **entire catalog** per cell, comparing 4 flip variants
(`_cell_bytes`, 32×32×4 bytes each) with a tolerance `STAMP_TOLERANCE = 4.0`.
A 1088px image = 34×34 = 1156 cells; on a fresh catalog each cell inserts a new entry,
so total byte-comparison work was **O(N²)** over the main thread: dozens of seconds /
a hard freeze.

### 3.2 Fix (fingerprint index + buckets)

`map_editor.gd` now indexes each catalog entry once, in `_load_stamp_catalog()`, by a
**coarse, flip-invariant fingerprint**:

| Helper | Purpose |
|---|---|
| `_coarse_bytes(img)` | Downsample 32×32 RGBA → 8×8 blocks, average each 4×4 block → 8×8×4 bytes |
| `_flip_coarse(c, h, v)` | Mirror an 8×8 coarse image horizontally/vertically |
| `_canonical_coarse(img)` | Minimum of the 4 flip variants under `_bytes_less` (lexicographic) |
| `_fp_hash(bytes)` | FNV-1a 32-bit hash → **bucket key** |
| `_index_stamp_entry(i)` | Push catalog index `i` into `_stamp_fp_index[hash]` |
| `_find_match(cell)` | Hash canonical-coarse(cell), scan only its **bucket**, keep 4-variant tolerance compare |

Result: dedup is **O(bucket size)** per cell — effectively O(1) for mostly-unique stamps.
Because the hash is taken over *canonical coarse* (flip-invariant minimum), mirrored/rotated
source content still lands in the same bucket, preserving the original flip-dedup behaviour.

`_ensure_tile` appends a new catalog entry via `_index_stamp_entry(catalog_index)`, so the
index stays consistent with the catalog without a full rebuild.

## 4. Responsiveness Guarantees

Every CPU-heavy path now yields to the main loop and shows a busy cursor:

| Path | Cooldown | Indicator |
|---|---|---|
| `_load_stamp_catalog()` (dir scan + PNG decode) | `await process_frame` every `_BUSY_YIELD_EVERY` (128) files | `info_label.text = "Loading stamp catalog… N files"` |
| `_rebuild_stamp_index()` | every 128 indexed entries | `"Indexing stamps… N/M"` |
| `_commit_stamp()` | every 64 cells **only when** `iter_count > 256` | `"Stamping… N/M cells"` |
| All of the above | — | `DisplayServer.CURSOR_BUSY` ↔ `CURSOR_ARROW` |

- `_set_busy(bool)` is **headless-safe**: it no-ops when
  `DisplayServer.get_name() == "headless"`, so the regression suite runs without a cursor.
- `_ready()` is now a coroutine: it awaits `_load_stamp_catalog()` before
  `_populate_grid()` so signal wiring order is unchanged.

## 5. Verification

- Syntax: `godot4 --headless --check-only --script editor/scripts/map_editor.gd`
- Regression suite: `godot4 --headless --path editor --script res://tests/test_screen_store.gd`
  expects `=== done, failures=0 ===`.
- `_test_stamp_fingerprint` in `editor/tests/test_screen_store.gd` asserts:
  canonical hash invariance over heavy/flip-v/flip-hv variants, distinct buckets for
  distinct images, empty-catalog miss, exact match, and flip-variant match via
  `_find_match` (key + `flip_h`/`flip_v` reported).

## 6. Notes

- `editor/assets/tiles/stamp_*_32x32.png` are **runtime artifacts**, regenerated on every
  stamp; do not commit them.
- The fingerprint uses 8×8 average blocks, so two catalog tiles that differ only in fine
  detail (< 4×4 pixel grain) may share a bucket; `_find_match` still resolves them exactly
  with the tolerance compare inside the bucket.