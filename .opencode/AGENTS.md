# Project: SSTD (Sidescroll Tower Defense)

## gRPC Integration

Three gRPC service contexts, each on a separate port (localhost-only by default, no auth):

### 1. Editor gRPC (port *TBD*, e.g. 50051)

Bridges external editors (custom tile editors, Aseprite→SSTD pipelines, CI scripts) to the editor backend. All operations go through the shared `EditorState` — same validation, same CCMV (Config-Control-Model-View) constraints as the Godot editor UI.

- Import/validate terrain types, entity defs, screen files
- Query grid config, dimension compatibility
- List / export current editor state
- Follows the same bridge API shape as `SstdBridge` (terrain → import_terrain_types, etc.)

### 2. Simulator gRPC (port *TBD*, e.g. 50052)

Controls the deterministic simulator for batch evaluation, AI training, headless testing.

- Step simulation N ticks
- Inject entity at tile position
- Query entity state, world state, combat log
- No display/rendering — pure data in/out

### 3. Game gRPC (port *TBD*, e.g. 50053)

Runtime game interaction — multiplayer coordination, external AI opponents, spectator clients.

- Query game state
- Submit action (place entity, trigger ability, etc.)
- Stream game events

### Shared architecture

- All three run on background tokio threads in the same process (or in `sstd-headless`)
- Each has its own proto service definition, combined in a shared `sstd-grpc` crate
- Editor gRPC shares `Arc<RwLock<EditorState>>` with the Godot bridge
- Simulator/game gRPC have their own state (world state, sim state)
- No auth/API keys — localhost-only by default; expose to network only via explicit config
- `sstd-headless` binary starts all three servers without Godot UI

### CCMV Constraint

All gRPC services must honor the **Config-Control-Model-View** (CCMV) architecture: every operation is validated against `sstd_config.sqlite3` config constants first, then processed through the Rust data model, and finally returned as structured data (never as raw display state). External tools bypassing CCMV (e.g., writing config values directly without going through the population system) must be rejected at the gRPC layer.

## LUCK Stat (Phase 2 Consideration)

- If LUCK is added as a stat, use a **bounded modifier** (`±LUCK%` to damage, no roll) to preserve deterministic feel
- Decided against dice (2D20 or flat random) — SSTD is strategic/no-RNG; even a bell-curve roll undermines placement strategy
- LUCK acts as a predictable fudge factor: `final_damage × (1 ± LUCK%)`, capped at reasonable bounds (e.g., ±25%)

## Pathfinding

- **No A*** — SSTD is a side-scroller; corridors are linear with occasional 2-3 way forks
- At fork joints, AI picks by priority rule (nearest enemy, weakest enemy, waypoint-route priority)
- Linear corridor segments: entities just walk forward, no pathfinding needed
- Waypoint-route system: entities follow waypoint chains; at junctions, fork-priority AI decides which branch to take

## Config Population Versioning

- Schema version bumps happen **only** when the editor has a "save configs" feature that can persist changes.
- Until then, default values can be changed in-place in `populate_001` (or a subsequent script) without bumping the version. Fresh databases get the latest defaults; existing databases retain whatever was seeded.
- Once the editor can save config overrides, any new population script must bump the version and old scripts must not be modified retroactively.

## TileSet (Multi-Tile Grouping) System

- `TileSetGrouping` (`scripts/resources/tile_set_grouping.gd`) is the primary authoring format — a `.tres` Resource with `key`, `display_name`, `width_tiles`, `height_tiles`, `tiles: Array`, `tags`, `source_image`.
- Each entry in `tiles` is a Dictionary with `local_x`, `local_y`, `terrain_key`, `elevation_tiles`, `z_depth`, `sub_tile_mask`, `source_col`, `source_row`.
- `terrain_key` doubles as the individual tile PNG filename: `{terrain_key}_32x32.png` in `editor/assets/tiles/`.
- `source_image` is the original spritesheet path; `source_col`/`source_row` are the grid position within it. Currently unused at render time (PNG extraction is done offline), but stored for provenance.
- `.tres` files in `editor/assets/tilesets/` are auto-discovered on Placement tab switch.
- The `validate()` method checks for duplicate positions, empty keys, and bounds.
- `resolve(base_x, base_y)` expands the group into flat terrain tiles with metadata.

### tileset_2-2.png (30° Grass Slope)
- Located at `editor/assets/samples/tileset_2-2.png`.
- Parameters (from Godot TileSet auto-detect): margins=(104,65), texture_region_size=(126,126), separation=(1,1). Pitch = 127px.
- Grid: 4 columns × 2 rows = 8 tiles, all with content.
- Each tile is 126×126 px in source, extracted and scaled to 32×32.
- Extraction tool was at `tools/extract-tiles/` (deleted after use). The Rust code used `image::imageops::resize(..., 32, 32, Nearest)`.
- Naming convention: `slope_30deg_{col}_{row}_32x32.png`.

## Tile Art Workflow

- Tile/entity pixel art is stored as **PNG files** on disk (`assets/tiles/{key}_{W}x{H}.png`, `assets/entities/{key}_{W}x{H}.png`), not in JSON/Dict.
- The native editor includes a built-in **PixelCanvas** for quick paint/touch-up and a **3×3 tiled preview** for seam checking.
- **Hybrid workflow**: PNGs can be created in external tools (Godot IDE, Aseprite, Photoshop) and imported via the "Import PNG" button. The two-way PNG roundtrip means no lock-in.
- PNG resolution matches the config tile size (default 32×32). External images are resampled with nearest-neighbor on import.

## Editor Tooling & Validation Commands

- Godot binary: `/home/hidekiai/bin/godot4` (4.4.1.stable).
- **Editor GDScript regression tests** (must pass after any editor change):
  ```bash
  /home/hidekiai/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd
  ```
  Expect `=== done, failures=0 ===`. Clean up any `editor/world.json` the tests write (gitignored).
- **Syntax check** a single script:
  ```bash
  /home/hidekiai/bin/godot4 --headless --path editor --check-only --script res://scripts/<file>.gd
  ```
- **Headless app boot** (surfaces node/scene errors): `/home/hidekiai/bin/godot4 --headless --path editor --quit-after 120`.
- After adding a new `class_name` global (e.g. `ScreenStore`, `ScreenMinimap`), run `godot4 --headless --path editor --import` once so other scripts resolve the type.
- `main.tscn` embeds the map/placement editors **inline** — the standalone `map_editor.tscn`/`placement_editor.tscn` are not what runs at runtime; edit `main.tscn` too.

## Screen Registry (ScreenStore)

- `editor/scripts/screen_store.gd` is the on-disk registry (`world.json`): maps `screen_id → (x, y) → file`, assigns `next_id`, and provides a per-screen cache.
- An **unregistered screen id is represented by position `(-1, -1)`** (sentinel) — never fall back to `Vector2i.ZERO`, which collides with the legitimate screen at `(0,0)` and caused test regressions.
- Guard screen ops with `_is_current_placed()` (`_screen_pos.x >= 0 and _store.is_occupied(...)`); cache/restore only for placed screens.
- On save/import of an unplaced screen, assign `_store.next_free_position()` before registering.
- Map editor writes `placed_entities: []`; `_merge_placed_entities()` re-attaches the cached entity array on save/clone so entity work isn't wiped.
- World coordinates: `world_x = screen_pos.x * grid_w + local_x` (wiki [TDD_Map-World § World Coordinates](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TechnicalDesign/TDD_Map-World)).
