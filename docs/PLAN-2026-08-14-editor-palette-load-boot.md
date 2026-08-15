# PLAN — 2026-08-14 Editor: tile-palette grid, Save/Load symmetry, boot-map race

state: in_progress
last-updated: 2026-08-14

Resumable work checklist. Tick a box + commit as each step completes so a
crash can resume from the first unchecked item. Root causes are captured
below so a resume needs no source re-digging.

---

## Root causes (from diagnosis)

1. **Palette is 1×H list with filenames**: `TilePalette extends ItemList`;
   Godot `ItemList.max_columns` defaults to `1`, so it can never wrap to a
   grid regardless of panel width. `_add_entry()` passes the filename as the
   item label, drawn under each icon (`ICON_MODE_TOP`). Palette entries =
   every on-disk tile PNG in `editor/assets/tiles/` (3486 PNGs) merged with
   the world.zip 2174-tile bank (`_load_stamp_catalog` + `_restore_tile_bank`).
2. **"Save" has no matching "Load"**: load handlers exist but are labeled
   `Import...` (Map Editor, map_editor.gd `_on_import`) and `Import Map...`
   (Placement Editor, placement_editor.gd `_on_import_map`). No button is
   literally "Load".
3. **Boot shows old/blank map (async `_ready` race)**:
   - `map_editor._ready()` yields across frames in `await _load_stamp_catalog()`
     (one `await get_tree().process_frame` per 128 files, ~3012 stamp PNGs).
   - While suspended, `main._ready()` → `_auto_load_last_map()` →
     `_on_import_file(world.zip)` → `_load_world_package()` → `_restore_screen()`
     fills `_tiles` with world screen 1 (journal: "4 screens, 2174 shared tiles").
   - When `_ready` resumes it calls `_populate_grid()` (map_editor.gd:83) which
     CLEARS `_tiles` → default air/grass/dirt. Loaded world is wiped.
   - Same race explains GitHub issues #47, #48.
4. **Negative-position sentinel collision** (found during verify; the real reason
   the stamped map did not render): `ScreenStore.position_of_id()` returns
   `Vector2i(-1,-1)` both for "not found" AND for a legitimately placed screen
   at (-1,-1). `_is_current_placed()` gated on `_screen_pos.x >= 0`, so a real
   screen at (-1,-1) was treated as unplaced and `_load_world_package` fell into
   `_populate_grid()` (default grid). world.zip manifest places screen 1 at -1,-1.
   Fix: added `ScreenStore.has_id(id)` and made `_is_current_placed()` + all
   `_screen_pos` sign checks (save/import/minimap HUD) use `has_id` instead.
5. **Near-duplicate near-black tiles** (found while pruning): stamp_1..86 and
   stamp_845..859 are all near-black opaque PNGs (mean ~20,14,27, alpha=255;
   265/2174 zip-bank tiles near-black, mean.r<32). The 8-bit canonical coarse
   fingerprint used for the stamp index never groups them (probe: 3025 entries ->
   3025 unique fingerprints) because any 4x4 block-mean change flips the hash.
   A 2-bit quantized coarse signature (each channel >> 6) DOES group them
   (ImageMagick: stamp_1..120 -> 33 unique groups, rep stamp_1 +86).

## Journal line the verify step must show

```
[editor/world] loaded package <...world.zip>: 4 screens, 2174 shared tiles, current screen 1
```

## Checklist

- [x] 0. Create this plan doc                                                      docs/PLAN-2026-08-14-editor-palette-load-boot.md
- [x] 1. TilePalette grid: `max_columns = 0` in `_ready()`                            editor/scripts/tile_palette.gd
- [x] 2. TilePalette no filename: empty label, tooltip kept                          editor/scripts/tile_palette.gd
- [x] 3. Map Editor button `Import...` -> `Load World...`                             editor/scenes/map_editor.tscn
- [x] 4. Placement Editor `Import Map...` -> `Load World...`                          editor/scenes/placement_editor.tscn
- [x] 5. Dialog titles -> `Load world package` (both editors)                         editor/scripts/map_editor.gd, editor/scripts/placement_editor.gd
- [x] 6. Guard `_populate_grid()` behind store-empty check in `_ready`                editor/scripts/map_editor.gd
- [x] 6b. Fix negative-position sentinel: `ScreenStore.has_id`; `_is_current_placed`   editor/scripts/screen_store.gd, map_editor.gd, placement_editor.gd
          and all `_screen_pos` sign checks use `has_id`; regression tests            editor/tests/test_screen_store.gd
- [x] 7. File GH issue: editors expose Save but no Load (ref #47/#48)                 gh issue create -> #50
- [x] 8. Run editor headless; journal shows `restored screen 1 (1140 tiles)`,           manual + journal
          no post-load `populated default grid`; test suite failures=0
- [ ] 9. `.backup` isolation experiment + journal review                              manual
- [ ] 10. Update wiki TODO.md session table + commit                                  sidescroll-towerdefense.wiki/TODO.md
- [x] 11. GH issue #51 for prune/merge duplicates button                                gh issue create -> #51
- [x] 12. Prune Duplicates button + merge logic + regression test                       editor/scenes/map_editor.tscn, editor/scripts/map_editor.gd, editor/tests/test_screen_store.gd
- [ ] 13. Headless verify prune on real catalog (expect journal `merged N duplicates`,  manual + journal
          `removed M disk PNGs`); test suite failures=0

## .backup isolation experiment (step 9)

Move the dynamically-loaded files out of the way, keep only the world zip, and
read the boot journal to see what the editor tries to load outside the zip:

- move `editor/world.json`, `editor/my_default_map_001.json`,
  `editor/assets/tiles/*.png`, `editor/assets/tilesets/` -> `editor/.backup/`
- keep `editor/world.zip`, `editor/config/` in place
- run `./scripts/run.sh`, capture stdout, review `[editor/*]` journal lines
- restore afterwards (git checkout for tracked, move back untracked)
