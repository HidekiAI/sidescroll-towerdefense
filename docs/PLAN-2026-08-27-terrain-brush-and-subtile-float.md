# PLAN + Discovery — Terrain Direct-Paint Brush & sub_tile_mask Float Bug

> Scope: workspace-local. Two related editor fixes discovered while validating
> direct terrain painting. Tracker issues: see #62/#67 lineage (this is separate
> follow-up editor UX + data-integrity work).

## 1. DISCOVERY (must reuse in future): Godot JSON writes float u8 fields

Godot's `JSON.stringify` serializes an `int`-stored Variant as a bare integer
(e.g. `15`), but a Variant that is a `float` is written with a decimal point
(e.g. `15.0`, `0.0`, `2.0`). Any `sub_tile_mask` value that lives in a GDScript
Dictionary as a `float` Variant therefore round-trips into the persisted JSON as
a float.

The Rust core contract declares these integer fields as `u8`/`i32`:

- `crates/sstd-core/src/terrain.rs`:
  - `TerrainTypeDef.sub_tile_mask: u8` (line 169)
  - `TileSetEntry.sub_tile_mask: u8` (line 307)
  - `TerrainTile.sub_tile_mask: u8` (line 433)
  - `TerrainTypeOverride.sub_tile_mask: Option<u8>` (line 572) — this struct
    parses the world's `terrain_overrides.json` and is the DIRECT source of the
    observed error.

`serde`'s `u8` deserializer rejects `15.0`:

```
ERROR: Bridge validation: Parse error: invalid type: floating point `15.0`, expected u8 at line 15 column 24
```

Observed data (produced before the fix) in `editor/world.zip`:

- `terrain_overrides.json`: `"air": { "sub_tile_mask": 15.0 }`, `"dirt": { "sub_tile_mask": 0.0 }`
- `screens/1.json`: interleaved `0.0`/`2.0`/`15.0` alongside plain ints under a
  "209"/"841"-count of int `0` etc. — i.e. SOME tile entries carry float masks.

The framework fallback `editor/default_package/terrain_types.json` is clean
(plain ints: `0`x3, `15`x4) — so the float pollution entered at runtime via the
collision-paint editor path or a float-typed assignment, then got persisted.

### Fix strategy (contract-aligned, producer-side + defensive reader)

1. **Producer: GDScript coerces the integer fields to `int` at the JSON write
   boundary.**
   - `editor/scripts/map_editor.gd:_serialize()` — normalize each
     `tile_data.sub_tile_mask` to `int` when building `entry["tile_data"]`.
   - `editor/scripts/terrain_editor.gd:collect_terrain_overrides()` — write
     `sub_tile_mask` as `int` in the diff dict.
   This keeps the persisted artifact in the exact `u8`/`i32` shape the Rust
   contract declares.
2. **Defensive reader: serde tolerance for legacy float-typed data.** Add a
   `deserialize_with` on the four `sub_tile_mask` fields that accepts an
   integral JSON float and casts it to `u8` (fractional part must be 0),
   instead of hard-failing. Keeps already-polluted `world.zip` files loadable.
   Any genuinely fractional mask (>0xF after cast) still surfaces via the
   existing `TERRAIN_SUBTILE_MASK_INVALID` validate warning.
3. Re-save the world from the editor after the fix so `world.zip` is regenerated
   with integer masks (clean artifact).

## 2. FEATURE: Direct terrain paint = wheel-sized NxN uniform stamp (powers of 2)

User spec: "when I paint directly, I just want a single tile; by using wheel I
can increase it to 2x2, 4x4, etc but all tiles will be same, just stamp."

- Default brush = 1x1 (single tile). Mouse wheel grows 1 -> 2 -> 4 -> 8 and
  shrinks back. All cells in the brush are the SAME selected terrain
  (uniform stamp). NO per-cell auto-tiling / sub-tile variance.
- Progression confirmed by user: 1, 2, 4, 8 (powers of 2).

### Existing structures it builds on (grounding)

- `editor/scripts/map_editor.gd`:
  - `_paint_tile(x, y)` (line 307) — direct paint entry, called from
    `tile_grid_display.gd:_gui_input` (lines 269-277) on click + drag.
    Non-tileset branch sets `_tiles[_key(x, y)] = _selected_terrain` (line 313).
  - `_gui_input` (line ~2239) — existing wheel handler for `_collision_paint_mode
    == "smart"` (lines 2254-2266) is the pattern to mirror for the terrain brush.
  - `_selected_terrain` — the terrain being painted.
- `editor/scripts/tile_grid_display.gd`:
  - `_draw()` cursor branch (lines 134-138) — currently draws a 1-tile hover
    highlight; extend to draw the NxN brush rect.

### Design

- Add `var _brush_size: int = 1` to `map_editor.gd` (values {1,2,4,8}).
- Wheel handling in `_gui_input`, in the NON-collision-paint path: WHEEL_UP ->
  `_brush_size` to next power of two; WHEEL_DOWN -> previous. Update
  `info_label` (show `Brush: NxN`), `tile_grid.queue_redraw()`, and
  `get_viewport().set_input_as_handled()`.
- Add `func stamp_coords(ox: int, oy: int, size: int) -> Array[Vector2i]` — pure
  helper returning the NxN cell set anchored with the hovered tile as TOP-LEFT
  origin (consistent with `_place_tile_set(base_x, base_y)`), boundary-clamped to
  the grid. O(N^2), N <= 8.
- Paint path: when `_brush_size > 1`, `_paint_tile` stamps `_selected_terrain`
  over `stamp_coords` (uniform, bypassing `_place_tile_set`'s multi-cell tileset
  resolve). When `_brush_size == 1`, keep existing behavior (single tile).
- Brush is session-local editor state (not persisted in world).

### Complexity

O(N^2) per paint call, N <= 8 -> trivial. No new persistence.

### Tests

GDScript regression test (`editor/tests/test_terrain_brush.gd`, mirroring
`test_override_merge.gd`):
- `stamp_coords` for 1x1 -> 1 cell; 2x2 -> 4; 4x4 -> 16; boundary clamping.
- wheel cycle: {1,2,4,8} both directions.
- (`sub_tile_mask` float fix is covered by Rust unit test + a serialization check.)

## Verification

- `cargo test --workspace` (Rust: serde float->u8 tolerance + existing validation).
- Re-save world; confirm `terrain_overrides.json` + `screens/*.json` have integer
  `sub_tile_mask` (`0`, `2`, `15`) and NO `*X.0*` variants.
- Load editor on DISPLAY=:0; wheel brush grows 1x1->2x2->4x4->8x8 painting a
  uniform stamp; no collision-overlay "4 quadrants" during normal paint.
