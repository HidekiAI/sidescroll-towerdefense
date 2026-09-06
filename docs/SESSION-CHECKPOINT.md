# Session Checkpoint — Editor Core & Persistence (post-#67)

## SHIPPED (2026-09-06) — #7 rarity FK seed + doc integrity (impl from PLAN below)

> Implemented per the PLAN block below and landed this pass.
> - `crates/sstd-core/src/config.rs`: population `005` creates + seeds
>   `gacha_rarities` (R=1/SR=2/SSR=3/UR=4, display names per GDD_Progression-Gacha)
>   and `guild_revive_cooldowns(rarity_id INTEGER PRIMARY KEY REFERENCES
>   gacha_rarities(id), cooldown_scenarios)` (R=5/SR=3/SSR=1/UR=0); new public
>   structs `GachaRarity` + `GuildReviveCooldown`, accessors `gacha_rarities()` /
>   `guild_revive_cooldowns()`; schema_version now `0.0.5`. Re-exported in lib.rs.
> - Tests added: `test_gacha_rarities_seeded`, `test_guild_revive_cooldowns_seeded`,
>   `test_rarity_fk_integrity` (PRAGMA foreign_key_check -> 0 violations),
>   `test_gacha_seed_idempotent` (re-run keeps 4/4). Updated
>   `test_schema_version_after_population`/`test_population_applied`. cargo: 110 core,
>   full workspace 136 pass.
> - Wiki: TDD_Training-Center Revive Mechanics now FK lookup via
>   `guild_revive_cooldowns`; the 4 loose `training_guildReviveCooldown{R,SR,SSR,UR}`
>   config keys deleted from the GM-config table; TDD_GM-Config Gacha Rarity Enum gains
>   consumer note; TDD_Enum-Tables gains `gacha_rarities` section + cross-ref matrix row.
> - Bridge: no change (no GDScript consumer of gacha tables yet — deferred).

## PLAN (pre-implementation, 2026-09-06) — #7 rarity FK seed + doc integrity

> Grounding (read before planning): `crates/sstd-core/src/config.rs` (Population
> mechanism `Population{id,description,func:`}, `POPULATIONS` array, `pop_id_to_version`
> id->version, `run_populations` gate, `INSERT OR IGNORE` seeding, `all_config`/loaders/
> tests at lines 555-730), `crates/sstd-editor-bridge/src/lib.rs` init_config_db,
> wiki `TDD_GM-Config.md` ("Gacha Rarity Enum" `gacha_rarities` table, line 75),
> `TDD_Training-Center.md` (Revive Mechanics + GM config rows 98-111),
> `TDD_Enum-Tables.md` (enum inventory + cross-reference matrix, lines 351-364).
>
> **Defect (issue #7):** rarity is referenced as loose TEXT config keys
> (`training_guildReviveCooldown{R,SR,SSR,UR}`) and an inline comment, NOT FK-encoded;
> the `gacha_rarities` enum table is absent from TDD_Enum-Tables inventory, and has no
> SQLite presence. Design decisions D6/D7 require INTEGER FK references for every enum.
>
> **Design (module ownership: `crates/sstd-core/src/config.rs` owns schema + accessors;**
> **bridge exposure deferred — no GDScript consumer exists yet):**
> 1. Add `Population { id: "005", description: "Gacha rarity enum + guild revive cooldowns", func: populate_005 }`.
>    `pop_id_to_version("005")=5` -> `schema_version` becomes `0.0.5`. Idempotent on
>    existing DBs (CREATE TABLE IF NOT EXISTS + INSERT OR IGNORE), matching populations 001-004.
> 2. `populate_005` (execute_batch):
>    - `gacha_rarities(id INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE, display_name TEXT NOT NULL, sort_order INTEGER NOT NULL)`
>      seeded R/SR/SSR/UR x Rare/Super Rare/Specially Super Rare/Ultra Rare, sort 0-3
>      (names per GDD_Progression-Gacha; schema per TDD_GM-Config Gacha Rarity Enum).
>    - `guild_revive_cooldowns(rarity_id INTEGER PRIMARY KEY REFERENCES gacha_rarities(id), cooldown_scenarios INTEGER NOT NULL)`
>      seeded R=5/SR=3/SSR=1/UR=0 (per TDD_Training-Center Revive Mechanics). Replaces the
>      4 loose config keys. Revive-fee formula key `training_guildReviveBaseFee` stays a config key.
> 3. New public structs `GachaRarity` + `GuildReviveCooldown` and accessors
>    `gacha_rarities() -> SstdResult<Vec<GachaRarity>>`,
>    `guild_revive_cooldowns() -> SstdResult<Vec<GuildReviveCooldown>>` following the
>    existing loader pattern (grid_config/revive_config); re-export in lib.rs.
> 4. Tests (config.rs `mod tests`): rarity rows=4; cooldown rows=4 with exact mapping;
>    `PRAGMA foreign_key_check` empty (referential integrity, works even without the FK
>    pragma enabled via connection); idempotent re-run (no dupes); update
>    `test_schema_version_after_population` -> "0.0.5" and `test_population_applied` + "005".
> 5. Wiki, same pass: TDD_GM-Config Gacha Rarity Enum gains consumer note
>    (`guild_revive_cooldowns.rarity_id`); TDD_Enum-Tables gains "Gacha Rarities" section
>    (canonical pointer to GM-Config) + cross-reference matrix row; TDD_Training-Center
>    Revive Mechanics rewritten to FK lookup + GM-config table rows replaced.
> 6. No Rust enum change for `RarityTier`/`DEFAULT_RARITIES` (luckbot.rs) — that is a
>    separate loot-weighting vocabulary (common/uncommon/rare/epic/legendary), untouched.

## SHIPPED (2026-09-06) — A19/A20 stale-code reconciliations (issues #24/#25, doc-only)

> Chosen resolutions (user): (a) #24 biome — keep code `BiomeType` (gameplay terrain
> enum == TDD_Enum-Tables `biome_types`, no Rust change) and the GDD_Art-Direction
> palette as a SEPARATE visual taxonomy with an explicit palette->biome mapping table;
> (b) #25 surface — `SurfaceType` (terrain movement modifiers) and transport road
> tiers (hauler speed/fuel bonuses) are separate concerns; road tiers will be a
> transport-system enum when implemented, NOT `SurfaceType` variants.
> Edits (wiki, commit ref on issues): GDD_Art-Direction Biome Palette gained the
> mapping column + reconciliation note; TDD_Enum-Tables biome_types + surface_types
> gained separation notes; TDD_Transport-Infrastructure Road Tiers gained the
> forward-design note. Also fixed a pre-existing unclosed code fence at the end of
> TDD_Enum-Tables (biome_types INSERT block; odd fence count). TODO.md A19/A20 -> done.
> No Rust/GDScript changes. Confirmed code `BiomeType` is `Crystal` (not Frozen) and
> matches `from_str` keys exactly.

> IN-FLIGHT (2026-08-27, after #64 CLOSED): terrain brush + sub_tile_mask float

> COMMITTED (this session): rewrote every broken wiki cross-reference to GitHub's
> flat-slug form. Root cause (empirically verified on the live wiki before touching
> anything): GitHub wiki **flattens** subdirectory pages — a page stored at
> `TechnicalDesign/TDD_Map-World.md` renders at `/wiki/TDD_Map-World` (confirmed
> 200), NOT at `/wiki/TechnicalDesign/TDD_Map-World` (confirmed 404), and any link
> written with a directory prefix (`TechnicalDesign/...`, `GameDesign/...`,
> `../TechnicalDesign/...`, `./Foo.md`) or a `.md` extension is left verbatim by
> GitHub's renderer and 404s / redirects to Home. The ONLY working link form is a
> bare flat slug like `](TDD_Map-World)`. Image/puml refs to nested repo paths only
> render via absolute `https://raw.githubusercontent.com/wiki/HidekiAI/...` URLs.
>
> **What changed (222 rewrites across 35 wiki pages):**
> - Stripped `TechnicalDesign/`/`GameDesign/`/`../`/`./` prefixes and `.md`
>   extensions from all page links -> flat slug. Post-fix audit: ZERO remaining
>   prefixed/.md link forms, and every flat link target resolves to an existing
>   wiki page.
> - Nested image/puml refs (`TechnicalDesign/images/*.png|.puml`,
>   `images/tdd-*`, `GameDesign/assets/*`) -> absolute `raw.githubusercontent.com/wiki`
>   URLs (files stay where they are; raw URLs verified working for nested paths).
> - Cross-repo links fixed: `../../editor/scripts/{terrain_editor,tile_grid_display}.gd`
>   -> `https://github.com/HidekiAI/sidescroll-towerdefense/blob/trunk/...`;
>   `../docs/TDD_Tile-Art-Pipeline.md` -> `TDD_Tile-Art-Pipeline` (the real wiki page);
>   dead `Tower Defense Side-Scroller - Design Doc.md` -> `GDD_Executive-Summary`.
> - Main repo also contained the broken pattern: `docs/PLAN-2026-08-27-grpc-editor-control.md`
>   (wiki URL with `/TechnicalDesign/` prefix), `editor/README.md`,
>   `editor/tests/README.md` (2 wiki URLs with `/TechnicalDesign/` prefix) — fixed.
>
> NEXT (cold resume): issue #8 CLOSING comment cites the wiki-path audit on
> `sidescroll-towerdefense.wiki` and the fix commit; #8 closed this pass. After
> that, #7 (element rarity FK references — verify `rarity_id` integrity vs enum
> tables) and #24/#25 (stale-code enum mismatches, see issues) are the remaining
> labeled bug/fix issues, then the parallax BG feature request recorded in
> `.opencode/AGENTS.md` (layered/depth-based parallax, not scanline).

## Bug-Squash Session (2026-09-05) — dialog lifecycle + HUD anchor + issue alignment

> COMMITTED (this session): dialog `queue_free()` lifecycle fix for #45 plus #33
> HUD anchoring restore in `main.tscn`. See "Bug-fix record" below. All editor
> regression tests green (`failures=0`), headless boot clean, all 4 edited
> scripts pass `--check-only`. Issues closed in the SAME pass: #45, #46, #33,
> #66, #43 (see per-issue closing comments for commit refs).
>
> **Bug-fix record**
> - #45 (dashboard: "screen dialogs stay open and aren't dismissed"): root cause
>   was missing `queue_free()` on EVERY code-created dialog. 17 dialog sites
>   across `map_editor.gd`, `placement_editor.gd`, `terrain_editor.gd`,
>   `entity_editor.gd` created `ConfirmationDialog`/`AcceptDialog`/`FileDialog`
>   via `.new()` + `add_child()` + `popup_centered()` and never freed them on
>   ANY exit path (confirmed/canceled/close_requested/file_selected) — the
>   Window nodes accumulated in the scene tree and lingered. Fixed by wiring
>   `queue_free()` into each `file_selected`/`confirmed` handler and connecting
>   `canceled` + `close_requested`. The minimap picker dialog
>   (`screen_minimap_dialog.tscn`, exclusive=true) already freed itself via
>   `position_picked`/Cancel — untouched.
> - Regression test `_test_dialog_lifecycle` (issue #45) added to
>   `editor/tests/test_screen_store.gd`: asserts no orphan Windows at editor
>   start, delete confirm opens exactly one, and confirm/cancel/close_requested
>   each free it. Green.
> - #33 (HUD overlap half): the `@onready` node-path half and ScrollContainer
>   single-child half were already fixed in `904467f`; the REMAINING defect was
>   `main.tscn` HUD Labels for MapEditor + PlacementEditor carrying only
>   `top_level=true` + `layout_mode=2` with NO bottom-right anchors/offsets (the
>   standalone `map_editor.tscn`/`placement_editor.tscn` HUDs have
>   `anchors_preset=3`, anchors 1.0, offsets (-430,-120,-12,-12),
>   `mouse_filter=2`). When toggled visible, the inline main.tscn HUDs rendered
>   at top-left overlapping the toolbars. Restored the full anchor/offset/mouse-
>   filter set into both main.tscn HUDs (commit 904467f claimed this but the diff
>   replaced anchors with bare top_level).
> - #46 was already fixed in `33338ba` (dedupe stamp brushes by key +
>   `_find_tile_set`) with regression test `_test_stamp_brush_dedupe` — issue had
>   simply not been closed. Closed with commit ref.
> - #66 (terrain paint palette) was already shipped + committed `98d8d8c` — issue
>   had not been closed. Closed with commit ref.
> - #43 (saved screens self-contained) was already shipped via World Archive
>   commits `d03023e`/`f3e93b8` (documented on wiki TDD_Saved-World) — issue had
>   not been closed. Closed with commit ref.
>
> NEXT (cold resume): none pending in this pass — all 5 issues closed.

> IN-FLIGHT (2026-08-27, after #64 CLOSED): terrain brush + sub_tile_mask float
> bug. Plan: `docs/PLAN-2026-08-27-terrain-brush-and-subtile-float.md`.
>
> COMMITTED: `b020b86` = brush feature + sub_tile_mask producer/serde fix (cargo
> workspace 132 pass; editor/tests/test_terrain_brush.gd 14 assert PASS).
> `e63fbf6` = PALETTE INVISIBLE FIX + visibility logging.
> `adb65aa` = checkpoint.
>
> PALETTE BUG (root cause, journal-proven): tile_palette populated 732 entries
> (7 terrain/722 tile/3 group), visible=true, but rendered size=(0,0) — the
> ItemList had no custom_minimum_size and collapsed to zero in the LeftPanel VBox.
> Fix: map_editor.tscn TilePalette custom_minimum_size=(0,160). Verified via
> direct scene probe: custom_min=(0,160), size=(565,160), visible=true.
> tile_palette.gd now logs populate counts + a deferred layout probe.
>
> NOTE on the "4 tile flicker / quadrant toggle": that is the COLLISION-PAINT
> mode ("Collision Map" button -> Paint Direct), a SEPARATE feature from terrain
> painting that XOR-toggles a tile's sub_tile_mask bitfield (0x0..0xF quadrants).
> Exit with Esc. Not part of the terrain brush.
>
> CONFIRMED SAFE: deleting editor/world.zip is fine — editor boots fresh with a
> default 1980-tile grid, logs "Last map no longer exists, skipping" (main.gd:262),
> no crash.
>
> RESUME VIA: `.opencode/sessions/terrain-brush-subtile-palette-fix.md` (the current
> session handoff with full task detail).

T-1 [DOING] Regenerate world.zip with integer sub_tile_mask — float masks persist
    because _serialize() coercion is not on the load->save path (details below).
T-2 [TODO] Live GUI verify wheel brush + palette (needs awake DISPLAY=:0).
T-3 [TODO] Reconcile PLAN doc to shipped state.
T-4 [TODO] Optionally file issue for the float mask load->save data-integrity gap.

Detailed task notes follow after the git-history section below.


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