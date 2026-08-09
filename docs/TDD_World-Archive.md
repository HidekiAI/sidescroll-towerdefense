# TDD: World Archive — self-contained `.zip` package

> **Status**: Implemented — regression suite green (58 checks, 0 failures / 0 script errors)
> **Related**: `world_archive.gd`, `map_editor.gd`, `placement_editor.gd`, `screen_store.gd`, `world.json`
> **Root issue**: [Issue #43](https://github.com/anomalyco/SSTD/issues/43) — saved screens are not self-contained

## 1. Glossary (world / map / screen / tile)

The editor reasons about four nesting levels. This vocabulary is normative for this repo.

| Term | Definition |
|---|---|
| **Tile** | A 32×32 RGBA8 cell of pixel art. Tiles carry *canonical gameplay type* (e.g. `lava`) or are *decorative stamp art* (`stamp_*`). Tile pixel data is **world-shared**: it is stored once per world and reused by every screen that references it. |
| **Screen** | One fixed-size (60×33 tiles) slice of the world. A screen stores a tile grid (references to world-shared tiles by tile key) plus placed entities. Screens are **shareable** — multiple screens reference the same world-shared tiles. |
| **Map** | A **collection of shareable screens**. There is exactly one map per world. |
| **World** | The top-level playable unit — exactly one self-contained **`.zip` package** that embeds the map (all screens) and the full world-shared tile bank. Also called a *package* or *scenario*. |

Hierarchy: `world (1 .zip) → map (1) → screens (N, shareable) → tiles (world-shared, referenced by key)`.

The editor loads the **whole world**, never a single screen in isolation: "we don't load per-screen, we load the entire map (which consists of multiple screens); hence tiles are world-shared."

```
┌──────────────────────────── world.zip (one self-contained package) ────────────────────────────┐
│                                                                                                  │
│  manifest.json   screens table: {"0,0": {x, y, id}, "1,0": {x, y, id}, …}  (map = screen set)   │
│                                                                                                  │
│  screens/        1.json, 2.json …           each: grid of tile-key refs + placed entities        │
│                                                                                                  │
│  tiles/          grass.png, lava.png, stamp_001.png …   world-shared RGBA8 pixels (raw PNG)       │
│                  embedded for EVERY tile referenced by any screen in the world                    │
│                                                                                                  │
│  entity_overrides/   catapult.json … optional visual/animation overrides of DEFAULT entity defs   │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## 2. Design decisions (confirmed)

1. **Container**: the whole world is **one `.zip`** (Godot `ZIPPacker`/`ZIPReader`, standard deflate — verified readable by system `unzip`).
2. **No invented formats/extensions**: files inside are plain `.json` (per-screen data, manifest) and raw `.png` tile bytes. Original filename convention: `<world-name>.zip`.
3. **Tiles are world-shared, not per-screen**: identical tile keys reused across screens are stored *once* in `tiles/`. All binary tile data lives in memory (`_tile_images`) because it is reused.
4. **Tile identity**: a tile's ID/key is **constant per world** once defined (e.g. `stamp_001`). `_ensure_tile` reuses an existing tile when the fingerprint matches instead of minting a new ID.
5. **Embed all in-use tiles**: `tiles/` holds a PNG for **every** tile referenced by any screen — including plain defaults (grass/dirt/…) snapshotted into the world — so the world is self-contained even if the editor's `assets/tiles/` cache is deleted. Reopening is never blank.
6. **Terrain override semantics (no new gameplay types)**:
   - A world may override the **visuals** of a canonical terrain type (`CanonicalTerrain` = `air, grass, dirt, stone, wall, water, lava`) via `tiles/{key}.png`.
   - Gameplay identity always comes from the **canonical type**: hot-spring-looking lava **is** lava (an entity falling in dies).
   - `stamp_*` keys are the only non-canonical terrain keys allowed (user-authored decorative art).
   - Loader **rejects** any other terrain key.
7. **Entity override semantics**: a world may include `entity_overrides/{key}.json` (custom visuals/animation) but only for keys that exist in the **default entity package**. It may never introduce a net-new entity type.
8. **`world.json` becomes a pointer**: it records which active `.zip` (world package) is loaded, instead of being the screens table itself. Multi-screen world data lives in the `.zip`.

## 3. World archive layout

```
{name}.zip                WorldArchive.WORLD_EXT = ".zip"
├── manifest.json         {"version": "0.3.0", "screens": {"x,y": {x, y, id}}}
├── screens/
│   └── {id}.json          per-screen serialized dict (ScreenFile shape, see §5)
├── tiles/
│   └── {tile_key}.png     raw RGBA8 PNG, one per distinct tile key used in the world
└── entity_overrides/     (optional)
    └── {entity_key}.json  override def keyed by a DEFAULT entity key
```

## 4. Codec API (`world_archive.gd`, `class_name WorldArchive`)

| Member | Purpose |
|---|---|
| `WORLD_VERSION` | `"0.3.0"` |
| `WORLD_EXT` | `".zip"` |
| `is_world_path(path)` | true when `path.ends_with(".zip")` |
| `is_allowed_terrain_key(key)` | canonical terrain OR `stamp_*`; else reject |
| `save_world(path, manifest, screens, tiles, entity_overrides={})` | write the whole world zip |
| `load_world(path)` | → `{"ok", "manifest", "screens", "tile_images", "entity_overrides"}` |
| `png_bytes(img)` / `png_to_image(bytes)` | PNG byte serialization (normalizes to RGBA8 on load) |

Validation: `load_world` rejects a world whose screens reference a terrain key failing
`is_allowed_terrain_key`.

## 5. Screen inside a world

Each `screens/{id}.json` keeps the existing ScreenFile shape (matches
`schemas/screen.schema.json`): `version, screen_id, width_tiles, height_tiles,
tile_width_px, tile_height_px, elevation_floor_tiles, elevation_ceiling_tiles,
tiles[] (x, y, terrain, tile_data?), placed_entities[]`.

**Change**: the bloated `embedded_tiles` (base64 PNGs) dict is **removed** — tile pixels
are no longer per-screen; they live once in the world's `tiles/` dir and are restored into
`_tile_images` from the shared bank.

## 6. Editor flows

- **Save** (`map_editor._on_save`, `placement_editor._on_save`): collect the map (every
  screen's serialized data from `ScreenStore`), collect the **world-shared tile bank**
  (every distinct tile key referenced, via `get_tile_image`), then
  `WorldArchive.save_world(path, manifest, screens, tiles, entity_overrides)`.
- **Import** (`_on_import`/`_on_import_map`): `WorldArchive.load_world(path)` →
  seed `_tile_images` from the bank → restore **all screens** into `ScreenStore` (cache) →
  restore the current screen into the editor grid.
- **Renderers** (`tile_grid_display.gd`, `placement_grid_display.gd`): query
  `get_tile_texture(map_editor…)` (in-memory) first; disk `assets/tiles/` is only a
  regenerable cache/mirror, **never** required for correctness.

## 7. Back-compat

- Loading a legacy `.json` (per-screen, 0.1.0/0.2.0, possibly with `embedded_tiles`)
  remains supported via `_load_screen_file` → `_load_plain_json`.

## 8. Acceptance criteria

1. Save a world (≥2 screens sharing ≥1 tile) to `{name}.zip`; delete/rename
   `assets/tiles/`; reopen the zip → **every screen renders tile pixels, nothing blank**.
2. The shared tile is stored **exactly once** in the zip (world-shared), and
   `_tile_images` is the source of truth in memory.
3. Round-trip preserves screen grid, tile placement, `placed_entities`, and tile identity
   (a stamp keeps its `stamp_001` key across save/load — no re-mint).
4. A world referencing an illegal terrain key (e.g. `hotspring`) is **rejected**; a
   `lava` override with hot-spring *pixels* is accepted and behaves as lava.
5. `.zip` is readable by system `unzip` (standard deflate).
6. Regression suite runs with **0 script errors** (not a vacuous `failures=0`).