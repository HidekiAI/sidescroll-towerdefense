# TDD: Tile Reuse & Reference-only Stamp Groups

> **Status**: Design — the image-import path already stores reference-only groupings; the *arbitrary region-select → group* tool is NOT yet implemented. The **duplicate-brush bug (#46)** on re-stamping the same image IS fixed.
> **Related**: `map_editor.gd` (`_commit_stamp`, `_ensure_tile`, `_find_match`, `_register_stamp_brush`, `_record_stamp_map`, `_place_tile_set`), `resources/tile_set_grouping.gd`, `resources/tile_set_entry_res.gd`, `schemas/tile_sets.schema.json`
> **Prompt**: "after I paste/stamp the hand-drawn image to convert it to tiles, how do I use/reuse individual tiles as a tile? how can I group some of the tiles to make new stamps of already stored tiles (so a 2×2 stamp is just 4 tileIDs instead of a 64×64 image)?"
> **Issue**: [#46](https://github.com/HidekiAI/sidescroll-towerdefense/issues/46)

## 1. Why the question is already half-answered

The 2×2-stamp-is-4-tile-IDs idea is **exactly how the editor already stores stamps**.
Nothing is ever stored as a big 64×64 image for a brush. A grouping is a list of
`{local_x, local_y, terrain_key, flip_h, flip_v, sub_tile_mask, …}` entries — the
`terrain_key` is a **reference** to a world-shared tile, not pixel data. See
`TileSetGrouping.tiles` (`resources/tile_set_grouping.gd`) and `TileSetEntryRes`
(`resources/tile_set_entry_res.gd`).

What is **missing** is the ability to build such a grouping from a *rectangular
selection of tiles already placed on the grid*, without re-importing a source image.

## 2. Current flow: image → tiles → reference brush

```
hand-drawn PNG
   │  _on_stamp_import_file → _stamp_image
   ▼
_stamp_grid_input (click+hold origin, release)
   ▼
_commit_stamp(origin)
   ├─ split into 32×32 cells (STAMP_CELL)
   ├─ per cell: _ensure_tile(cell)  ── fingerprint-dedupe, else mint ──► stamp_N
   │     ├─ _find_match: coarse 8×8 avg + flip variants, diff ≤ STAMP_TOLERANCE(4.0)
   │     └─ else _stamp_seq+=1, key="stamp_N", save PNG to assets/tiles/, add to _terrain_types
   ├─ _tiles[x,y] = stamp_N   (grid now references the tile)
   ├─ _tile_data[x,y] = {flip_h, flip_v, sub_tile_mask, elevation, z_depth}
   ├─ _register_stamp_brush(cells, cols, rows)  ─► TileSetGrouping ("stamp_<basename>")
   │     tiles[] = reference-only entries {local_x, local_y, terrain_key, flips, src_col, src_row}
   └─ _record_stamp_map(cells, cols, rows)      ─► _stamp_maps[] (embedded in screen JSON)
           cells[] = {tile_id, local_x, local_y, src_col, src_row, flip_h, flip_v}
```

Result: the brush is a reference-only `TileSetGrouping`; the placement history is a
reference-only `_stamp_maps` array. A 2×2 stamp = **4 `stamp_N` tile IDs**, never a
64×64 image. This is already correct.

## 3. How to reuse an individual tile (as-is, no changes)

After stamping, every unique inked cell is a `stamp_N` entry in `_terrain_types`
(added by `_ensure_tile`), so it appears in the **terrain palette**
(`_refresh_palette`). Select it and paint single cells with the brush tool.

Reuse is also **automatic**: `_ensure_tile` → `_find_match` returns the *existing*
`stamp_N` when a new cell's fingerprint (coarse 8×8 average + 4 flip variants) is
within `STAMP_TOLERANCE`. So re-stamping a region that visually matches an earlier
tile reuses that tile ID instead of minting a duplicate. This is the "reuse individual
tiles as a tile" behavior — already implemented.

Tile pixel data lives in `_tile_images` (in-memory source of truth) and is mirrored to
`assets/tiles/stamp_N_32x32.png` on disk; renderers query `get_tile_texture`/`get_tile_image`.

## 4. The gap: region-select → new reference-only stamp

There is **no tool** to select an arbitrary rectangle of *already-placed* grid tiles
and register them as a new reusable stamp. The only way to author a grouping today is
by importing an image. The design below closes that gap.

### 4.1 Proposed UI

- Add a **"Group selection"** mode (toolbar button) alongside paint/erase/stamp.
- `tile_grid_display` draws a marquee rectangle while the user click-drags.
- On release, the editor collects every tile **inside** the rectangle that has ink
  (terrain ≠ `air`), and builds a reference-only `TileSetGrouping` + `_stamp_maps` entry.

### 4.2 Grouping algorithm (reference-only)

```
_selected_rect = {x0, y0, x1, y1}  (inclusive, snapped to tile grid)
cells = []
for y in y0..y1:
  for x in x0..x1:
    key = _tiles[x,y]
    if key == "air" or empty: continue
    td = _tile_data[x,y]
    cells += { local_x: x-x0, local_y: y-y0,
               terrain_key: key,
               flip_h: td.flip_h, flip_v: td.flip_v,
               sub_tile_mask: td.sub_tile_mask,
               elevation_tiles: td.elevation_tiles, z_depth: td.z_depth }
ts = TileSetGrouping.new()
ts.key = "group_<basename or seq>"      # reference ID
ts.display_name = "Group <N>"
ts.width_tiles  = x1-x0+1
ts.height_tiles = y1-y0+1
ts.tiles = cells                          # reference-only; NO pixel copies
ts.tags = ["stamp", "terrain"]
_tile_set_groupings.append(ts)
_record_stamp_map(cells, width, height)   # reuse existing _stamp_maps writer
```

**Storage invariant** (unchanged): the grouping stores only tile IDs + local coords +
flips. The tile pixels remain world-shared in `_tile_images`/`assets/tiles/`. A 2×2
group is exactly 4 tile-ID references.

### 4.3 Placement

Reuse of the new group is identical to any existing stamp: selecting it in the
tile-set palette sets `_selected_tile_set_key`, and `_place_tile_set` resolves the
group back to grid tiles via `TileSetGrouping.resolve(base_x, base_y)` — stamps the
referenced `terrain_key` onto the grid with the recorded flips. Flipping is applied at
paint time (`_flip_h_active`/`_flip_v_active`), not stored into the group.

## 5. Persistence & round-trip

- `TileSetGrouping` entries persist via the existing `tile_sets.json` export
  (`terrain_editor.gd`) and the `tile_sets.schema.json` (each entry is `{local_x,
  local_y, terrain_key, flips, …}` — reference-only).
- `_stamp_maps` entries are embedded in the screen serialization (`_serialize`,
  and therefore in `world.zip` `screens/{id}.json`), so a group's placement history
  survives save/load exactly like the current image stamps.
- On `load_world_package`, `_restore_tile_bank` repopulates `_tile_images` from the
  world's `tiles/` bank; groups are reconstructed from the screen's `stamp_maps`.

## 6. Acceptance criteria (for the new region-select tool)

1. Select a 2×2 rectangle of placed tiles → a new stamp appears in the tile-set
   palette whose `tiles` array has exactly **4 entries** (4 tile IDs), and whose
   serialized JSON contains **no base64/pixel data**.
2. Painting that stamp onto a fresh area reproduces the 4 tiles with correct flips.
3. The stamp survives world save/load (reference-only, no pixel bloat).
4. Empty (`air`) cells inside the selection are omitted from the group (they are not
   stored and not stamped).
5. An existing image-imported stamp still round-trips unchanged (regression).

## 7. Open questions

1. **Group identity**: auto-name `group_N` (sequence) vs. let the user name the group
   at creation? Default: auto-name, editable later.
2. **Empty-cell behavior**: omit `air` cells (lean) vs. keep the full rectangle so the
   brush preserves holes? Default: omit (lean, matches `_record_stamp_map`).
3. **Slope/terrain_default interop**: should a group be taggable as a slope set so it
   gets the yellow palette color, or always `stamp`? Default: always `stamp`.
4. Should grouping be undoable (an undo stack) or one-shot (current editor has no
   undo)? Default: one-shot to match current tooling.

## 8. No-action guard

This doc is **design**. The only production code that already realizes the
reference-only principle is the image-import path (`_register_stamp_brush`,
`_record_stamp_map`). The region-select → group tool in §4 is **not implemented** and
will be tracked as a new issue before any code is written.

## 9. Implemented changes (issue #46)

**#46 — duplicate brush on re-stamp, fixed.** `_register_stamp_brush` now reuses an
existing `TileSetGrouping` by key instead of appending a duplicate: it calls
`_find_tile_set(key)` first and only mints/`append`s when the key is absent.
`_refresh_tile_set_palette()` refresh + `select_tile_set(ts.key)` follow either way.

While fixing, hardened the typed-array assignments that produced a runtime
`SCRIPT ERROR` in Godot 4 (`Invalid assignment of property 'tags' … on Array[String]`):
all `ts.tags = [...]` / `wrapper.tags = [...]` sites now use `.assign([...])`.

**Verify**: `_test_stamp_brush_dedupe` regression added to `editor/tests/test_screen_store.gd` — stamps the same image twice (same basename) and asserts the `stamp_<basename>` grouping count stays `1`; a different basename yields a distinct `2`nd.

## 10. Implemented: unified visual tile palette (issue #49)

**#49 — visual tile palette for pick-and-choose.** The Map Editor's two
text-only lists (`PaletteList`, `TileSetPalette`) were replaced by one
**thumbnail grid** (`editor/scripts/tile_palette.gd`, an `ItemList` in
icon mode) that shows every paintable thing as a 32px icon:

- canonical terrain types → color swatches (fallback to their tile image
  when the world bank carries one),
- world-shared `_tile_images` entries → their tile thumbnails,
- `TileSetGrouping`/`GodotTileSetGrouping` brushes → composite previews
  built by `_group_preview_image` (stamps each referenced cell into a
  `w×h` thumbnail; `STAMP_CELL` cells, reference-only, never pixel bloat).

Selection maps to the existing brush path: `tile_picked(kind, key)` →
`_on_tile_picked` sets `_selected_tile_set_key` (`terrain_*` for terrain,
raw key for bank tiles, group key for groupings); single-tile picks paint
via the same `_place_tile_set`/`_paint_tile` machinery used by image
stamps. `select_tile_set(key)` (used by `main.gd` tileset navigation) now
highlights the entry via `tile_palette.select_key(key)`.

The palette is refreshed after the async stamp-catalog load, after world
package import (`_restore_tile_bank`), and on terrain-type changes, so a
loaded `world.zip` immediately shows all its shared tiles as pickable
icons — the prerequisite picker for the #44 region-select grouping tool.

**Verify**: `_test_tile_palette` in `editor/tests/test_screen_store.gd`
- asserts the palette node exists and is scripted,
- asserts the grid is populated (≥ terrain count, ≥ tile-bank count),
- emits `tile_picked` with kind `terrain`/`tile` and that the editor's
  selected brush key follows the pick.

**Regression note (GDScript gotcha)**: capturing an outer array in a
lambda and reassigning it (`picked = [...]`) does **not** update the
caller's variable — use `picked.assign([...])` instead (§10 test uses
`assign`). Also, `ItemList.icon_mode` is an enum
(`ItemList.ICON_MODE_TOP`), not a bool.

## 11. Implemented: prune/merge near-duplicate tiles (issue #51)

**#51 — opt-in "Prune Duplicates" button.** The Map Editor carries many
visually-identical near-black tiles (stamp_1..86, stamp_845..859; 265/2174
zip-bank tiles near-black, mean.r<32). The exact 8-bit coarse fingerprint
used by the stamp index never groups them — any 4x4 block-mean change flips
the hash, so a probe over 3025 tiles produced 3025 unique fingerprints. An
ImageMagick 2-bit quantized 8x8 gray signature collapsed stamp_1..120 into
33 unique groups (rep stamp_1 +86).

The merge is **explicitly user-triggered** (button in the Map Editor bottom
bar), never part of boot/load — dedup at load time would block on a ~3k-tile
pass and feel hung. Flow (`editor/scripts/map_editor.gd`):

1. `_similarity_sig(img)`: 2-bit quantized coarse bytes (`_coarse_bytes`
   result `>> 6`), canonicalized over the 4 flips via `_flip_coarse` +
   `_bytes_less` so flipped duplicates also group.
2. Group by signature; within a group pick the representative with the
   lowest numeric stamp id (`_catalog_rank`).
3. Verify each candidate against the representative with the exact
   XOR-style diff `_flip_of` (`_cell_bytes` + `_diff <= STAMP_TOLERANCE`,
   trying all 4 flips) — coarse grouping only buckets; the exact diff
   confirms visual identity.
4. `_rewrite_references(merge_map)`: rewrite `_tiles`, `_tile_data` flip
   flags (XOR of old + merge flips), `_store.cache` screen tiles, and
   `_tile_set_groupings` brush entries onto the surviving id.
5. `_drop_tiles(dup_keys)`: erase from `_tile_images`/`_tile_texture_cache`,
   delete the on-disk `assets/tiles/<key>_32x32.png` (journal records the
   count), rebuild `_stamp_catalog`, refresh palette + grid.
6. Journal line: `[editor/prune] merged N duplicates onto lowest-id tiles
   (first <k> -> <rep>), removed M disk PNGs, bank=B`.

`map_editor._ready` now sets `_ready_done = true` at its end so tests can
await the async stamp-catalog load deterministically before mutating state.

**Verify**: `_test_prune_duplicates` in `editor/tests/test_screen_store.gd`
seeds a synthetic 3-tile catalog (two identical + one distinct, using
nonexistent `stamp_5000..` ids so the disk is never touched), awaits
`_ready_done`, runs `_on_prune_duplicates`, and asserts the duplicate is
dropped, the lowest-id representative survives, the distinct tile survives,
and a grid reference pointing at the duplicate is rewritten to the rep.

**Test isolation lesson (discovered the hard way)**: the first version of
this test instantiated the editor, awaited one frame, and added seeds — but
`_ready`'s `await _load_stamp_catalog()` was still mid-load, so the prune
ran on the real partial catalog and deleted 20 real disk PNGs
(`editor/assets/tiles/stamp_12_32x32.png` among them; 19 were recoverable
from the world.zip tile bank, 1 was a disk-only visual duplicate and was
gone — no map data lost). Tests must always wait for `_ready_done` and clear
the catalog before seeding, and synthetic ids must not collide with real
disk keys.