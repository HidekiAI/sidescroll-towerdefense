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

## Journal line the verify step must show

```
[editor/world] loaded package <...world.zip>: 4 screens, 2174 shared tiles, current screen 1
```

## Checklist

- [ ] 0. Create this plan doc                                                      docs/PLAN-2026-08-14-editor-palette-load-boot.md
- [ ] 1. TilePalette grid: `max_columns = 0` in `_ready()`                            editor/scripts/tile_palette.gd
- [ ] 2. TilePalette no filename: empty label, tooltip kept                          editor/scripts/tile_palette.gd
- [ ] 3. Map Editor button `Import...` -> `Load World...`                             editor/scenes/map_editor.tscn
- [ ] 4. Placement Editor `Import Map...` -> `Load World...`                          editor/scenes/placement_editor.tscn
- [ ] 5. Dialog titles -> `Load world package` (both editors)                         editor/scripts/map_editor.gd, editor/scripts/placement_editor.gd
- [ ] 6. Guard `_populate_grid()` behind store-empty check in `_ready`                editor/scripts/map_editor.gd
- [ ] 7. File GH issue: editors expose Save but no Load (ref #47/#48)                 gh issue create
- [ ] 8. Run `./scripts/run.sh`; verify palette grid + screen 1 of world.zip          manual + journal
- [ ] 9. `.backup` isolation experiment + journal review                              manual
- [ ] 10. Update wiki TODO.md session table + commit                                  sidescroll-towerdefense.wiki/TODO.md

## .backup isolation experiment (step 9)

Move the dynamically-loaded files out of the way, keep only the world zip, and
read the boot journal to see what the editor tries to load outside the zip:

- move `editor/world.json`, `editor/my_default_map_001.json`,
  `editor/assets/tiles/*.png`, `editor/assets/tilesets/` -> `editor/.backup/`
- keep `editor/world.zip`, `editor/config/` in place
- run `./scripts/run.sh`, capture stdout, review `[editor/*]` journal lines
- restore afterwards (git checkout for tracked, move back untracked)
