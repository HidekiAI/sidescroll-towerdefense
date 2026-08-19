# Project: SSTD (Sidescroll Tower Defense)

## Session Progress Checkpoint (permanent)

The user may switch sessions mid-task at any time, and may also **abandon a
session (e.g. change of model) and start a brand-new session with zero memory
of this conversation**. For multi-step work (`#51` prune optimization and any
long task), **keep `docs/SESSION-CHECKPOINT.md` continuously updated as work
proceeds** so a fresh session can resume cold:

- Record what shipped/committed, the exact current active step, and the next
  move after each meaningful milestone — not only at the end.
- Record any measured numbers, decisions, and commands so the successor does
  not re-derive them.
- Commit the checkpoint update together with (or immediately after) the code
  change it describes. (Rule recorded 2026-08-16.)
- **Write the checkpoint as if the next reader is a different model on a
  brand-new session that never saw this conversation**: it must be fully
  self-contained — state the objective, the committed history, the exact
  in-flight step, the next move in runnable order, and every command/finding
  needed to resume, without relying on prior chat context. A checkpoint that
  assumes the reader "knows the project" from memory is not sufficient; if a
  crash/abandonment means the in-flight step is lost, the checkpoint must be
  enough to restart that step from scratch. (Rule recorded 2026-08-17.)

## Wiki Maintenance Rule (permanent)

Whenever a **new wiki page** is added (GDD or TDD), **update the wiki Home page** (`sidescroll-towerdefense.wiki/Home.md`) in the same commit — add a row in the relevant section (Game Design / Technical Design) linking the new page. Do the same in `TODO.md` (tracked-work session table). A newly shipped wiki page with no Home.md/TODO.md entry is an oversight. (Rule recorded 2026-08-09, see #40.)

## Ticket-First Rule (permanent)

Do NOT wire up / implement anything (code changes, wiring, edits) until the
corresponding GitHub issue is created AND documented/planned in the issue body
(design, approach, scope). Feature requests and bugs alike: create the ticket,
document the plan in it, then implement — and reference the issue number in the
commit that implements it. (User directive 2026-08-17, enforced twice on the
#55 config-defaults work.)

## Issue-State Hygiene (permanent, user directive 2026-08-19)

GitHub issue states are part of the persisted status progression and MUST be
kept in sync with commits, alongside the checkpoint/wiki/AGENTS updates. This
applies to ALL issue types — features, bugs, and chores alike.

- **Close** an issue the moment its work is addressed and committed (a "DONE
  (committed ...)" comment alone is NOT enough — the issue stays open).
  - For FEATURES: the closing comment must also confirm the feature is
    implemented as stated on the wiki, citing the wiki page(s) that document
    it (e.g. `TDD_World-Editor`, `TDD_Tile-Deduplication`).
  - For BUGS: reference the commit that fixes it AND the wiki page(s)
    describing the intended/correct behavior.
  - For CHORES/DOCS: reference the commit (and wiki page if applicable).
  - Traceability invariant: every path — commit, closing comment, wiki TODO
    table — must lead (directly or indirectly) to a wiki page. Commit ->
    issue -> wiki; the closing comment is the issue->wiki link, so cite the
    wiki page in it.
- **Reopen** it if a follow-up proves the fix incomplete (with the new
  repro/evidence in the body).
- **Keep open** when the issue is deliberately deferred in-body (e.g. #54 M2)
  or is a filed-but-unfixed bug (#61).
- Do this in the SAME pass as the code commit and doc updates, not as a
  separate "later" step. (Rule recorded 2026-08-19, after #55/#52/#53/#59/#60/
  #62 shipped but were left open.)

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

## LUCK Stat

SSTD is strategic / no-RNG, so LUCK is a **deterministic, bounded modifier** — never a dice roll.

- **Deterministic, not random** (Decision, 2026-08-07): LuckBot companions apply a predictable,
  capped boost; no 2D20, no flat random, no bell-curve rolls for luck. Rolls stay out of the
  placement-strategy loop.
- **Behavior** (implemented @ `crates/sstd-core/src/luckbot.rs`):
  - `final_damage × (1 + %LUCK)` — additive-on-base, capped at `luck.bonus_cap` (default +25%)
  - Crit is a fixed cadence (`crit_interval`: every N-th hit), max luck → crit every hit
  - Drops use a guaranteed `rarity_floor` (luck raises the minimum tier), not weighted rolls
  - Neutral luck `0` → no effect (`×1.0`), so investing in a LuckBot is always a deliberate choice
  - All constants config-driven (`luck.*` keys via `populate_003`)
- Supersedes the earlier "Phase 2 Consideration" ±LUCK% to damage note; the deterministic stance is kept, now implemented as a bounded modifier with explicit crit cadence + rarity floor.

- AURA / radius and patrol/follow movement are core geometry (`within_aura`); actual
  ally-selection and target-following remain the engine's job.

### Replay & seeded rolls

- **Persist the SEED, not rolls.** One `u64` seeds `sstd-core`'s dependency-free splitmix64
  (`seeded_roll(seed, counter)`); everything else (tiers, crits, damage) is derived data.
- `RollLog` (luckbot.rs) folds every `roll()` into a rotating `checksum()`; **same seed + same
  roll count ⇒ identical checksum** is the replay/CI invariant. Any call-order change, missing
  roll, PRNG swap, or config change breaks it.
- Rule: never persist a rolled tier directly as a replay source of truth; recompute it from the
  seed on replay.

## Pathfinding

- **No A*** — SSTD is a side-scroller; corridors are linear with occasional 2-3 way forks
- At fork joints, AI picks by priority rule (nearest enemy, weakest enemy, waypoint-route priority)
- Linear corridor segments: entities just walk forward, no pathfinding needed
- Waypoint-route system: entities follow waypoint chains; at junctions, fork-priority AI decides which branch to take

## Column Naming Convention

All SQL column names follow `snake_case` (code-enforced). Fixed `descriptionID` → `description_id` across wiki 2026-08-01 (TDD_Localization-System, TDD_Skill-Dependency-Graph, TDD_Skill-Designer-Templating).

## Status Badges

As of 2026-08-01 audit, most TDD/GDD pages describe planned systems with no code implementation (~80% design target). Recommended badge format for page headers: `**Status:** ✅ Implemented` / `⬜ Design Target` / `🔧 In Progress`. Not yet applied.

## Element Enums

Code has 6 (`Physical`/`Fire`/`Ice`/`Lightning`/`Holy`/`Dark`). Wiki documents 14 elements total (6 implemented ✅ + 8 planned ⬜) with 2 extra combo-only elements (Wind, Oil). Wiki header clarified 2026-08-01: dropped "12-element" claim, added explicit "8 planned" count.

## Config Key Naming Convention

All config keys follow `domain.snake_case` format (code-enforced). The wiki's `TDD_GM-Config.md` had bare `camelCase` keys in presets/cheatsheet (e.g. `baseDamage`, `mineSpawnRate`). Fixed 2026-08-01:
- Preset tables: `difficultyMultiplier` → `difficulty.multiplier`, `enemyHpMultiplier` → `difficulty.enemy_hp_multiplier`, etc.
- SQL examples: per-domain tables (`inventory_config`) → single `config` table per code
- Cheatsheet: all keys prefixed with domain + snake_cased
- Attribute keys in `TDD_Entity-Instance-System` (e.g. `attackRange`, `elementAffinity`) remain camelCase — those are entity attribute keys, not config keys.

## Rule of Thumb: Config-Based Constants

All tunable gameplay constants must live in `sstd_config.sqlite3` via the population system (`config.rs` POPULATIONS / `populate_NNN`). Code structs expose a `Default` ONLY as a fallback for an unseeded store — never as the runtime source of truth. The game always loads from `ConfigStore`. New systems: write the population script + keys + a `ConfigStore::<x>_config()` loader + loader test alongside the logic.

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

## Editor Persistence Paths (#54 M1, 2026-08-18)

All runtime writes go under `user://data` (never `res://`, which is read-only in a packaged build):

| Path | Value | SQLite-configurable? |
|---|---|---|
| ConfigStore DB | `user://data/config/sstd_config.sqlite3` | No |
| Boot world pointer | `defaults.world_json_path` = `user://data/world.json` | Yes |
| Save-dialog file name | `defaults.world_file_name` = `world.zip` | Yes |
| Save/load dialog start dir | `defaults.file_dialog_dir` = `""` | Yes |
| Tile/stamp cache | `user://data/tiles` | No |

- `main.gd`: `USER_DATA_DIR := "user://data"`, `DEFAULT_WORLD_PATH := "user://data/world.json"`, `user_data_dir()` (globalized + mkdir). `_init_config_db` writes the config DB under `user://data/config/`.
- `map_editor.gd`: `_tiles_write_dir()` = `user://data/tiles`; `get_tile_image`/`_load_stamp_catalog` read user dir first then built-in `res://` fallback; `_register_tile_image` persists PNGs to the user dir.
- Data base dir override: deferred, issue #57 (CLI-arg-only, future).
- Test hygiene: world-restore tests persist the user:// PNG, so cleanups MUST wipe both `user://data/tiles` and `res://assets/tiles` copies (helper `_wipe_tile_artifact(key)` in `tests/test_screen_store.gd`).

## Dirty-State Save Prompt (#60, 2026-08-18)

- `main.gd` owns `is_dirty`. `mark_dirty()`/`clear_dirty()`; `_confirm_save_dirty()`
  returns 2=Save / 1=Discard / 0=Cancel (awaited modal; non-dirty skips -> 1);
  `_confirm_dirty_or_save()` is the guard for interactive loads.
- **Godot gotcha**: an `await`ed handler in `_notification(NOTIFICATION_WM_CLOSE_REQUEST)`
  does NOT block the engine — `SceneTree.auto_accept_quit` defaults to `true`, so the
  app quits right after the notification fires, even while the dialog awaits. Must set
  `get_tree().auto_accept_quit = false` in `_ready()` and call `get_tree().quit()`
  explicitly from the handler (dirty: after the prompt; clean: immediately).
- **Godot gotcha #2**: you CANNOT `await` inside a `_notification()` handler at all —
  Godot calls the handler and discards the returned coroutine, so code after the
  `await` never runs (the quit prompt "returned" but quit never fired). Use
  callback-driven dialogs there (connect button signals), not `await`.
- **World formats**: `.zip` = whole world package (primary, since #43). `.json` in the
  load/save dialogs = a single legacy screen. `user://data/world.json` = boot pointer
  file recording the active `.zip` path — never picked in a dialog. Dialog filter
  default follows the current world format via `_apply_world_default_filter` (#62):
  `.zip` when `world_package_path` is set, `.json` in legacy manifest mode.
- Dirty is set by map paint (`_paint_tile`), placement entity place/remove/clear,
  and cleared by `_save_world()` in either editor. Guards: quit
  (NOTIFICATION_WM_CLOSE_REQUEST), import dialogs (via `_on_import_file_guarded`),
  `new_project`/`open_project`.
- Test seam: tests call load/import methods directly (bypass the guarded dialog
  path) and never reach the modal; `_test_dirty_prompt` covers the flag mechanics.

## Prune Scan Hot Loops (#51/#53/#59)

Native bridge (`sstd-editor-bridge`) owns the scan hot loops; GDScript is the
oracle. Current front-end: `scan_signatures`+`scan_projections` (f64),
`scan_gate_pairs`, `build_variants`, `scan_exact_matches`, `a3_neighbor_hashes`,
`flip_of_bytes`, and `scan_canonical_coarse` (N*256, #59). `_a3_discover` slices
per-rep canon from `_canonical_flat` when the bridge produced it. Real catalog
(3346 tiles): full `_build_prune_plan` 5030 ms, dup_keys=1328.

## Screen Registry (ScreenStore)

- `editor/scripts/screen_store.gd` is the on-disk registry (`world.json`): maps `screen_id → (x, y) → file`, assigns `next_id`, and provides a per-screen cache.
- An **unregistered screen id is represented by position `(-1, -1)`** (sentinel) — never fall back to `Vector2i.ZERO`, which collides with the legitimate screen at `(0,0)` and caused test regressions.
- Guard screen ops with `_is_current_placed()` (`_screen_pos.x >= 0 and _store.is_occupied(...)`); cache/restore only for placed screens.
- On save/import of an unplaced screen, assign `_store.next_free_position()` before registering.
- Map editor writes `placed_entities: []`; `_merge_placed_entities()` re-attaches the cached entity array on save/clone so entity work isn't wiped.
- World coordinates: `world_x = screen_pos.x * grid_w + local_x` (wiki [TDD_Map-World § World Coordinates](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TechnicalDesign/TDD_Map-World)).
