extends SceneTree

const STAMP_CELL := 32

var _failures := 0

func check(cond: bool, msg: String) -> void:
    if cond:
        print("ok: " + msg)
    else:
        _failures += 1
        print("FAIL: " + msg)

func _initialize() -> void:
    _run()

func _run() -> void:
    _test_scripts_compile()
    _test_screen_store()
    await _test_map_editor()
    await _test_placement_editor()
    _test_stamp_fingerprint()
    await _test_stamp_brush_dedupe()
    await _test_prune_duplicates()
    await _test_prune_boundary_straddler()
    await _test_grayscale_projection()
    await _test_grayscale_candidates()
    await _test_prune_chain()
    await _test_a3_neighbor_scan()
    await _test_deep_prune()
    await _test_parallel_exact_scan()
    _test_diff_capped()
    _test_world_archive_roundtrip()
    await _test_world_reopen_not_blank()
    await _test_world_pointer_bootstrap()
    await _test_tile_palette()
    print("=== done, failures=%d ===" % _failures)
    quit(0 if _failures == 0 else 1)

# Guards against the vacuous-pass trap: a scene whose script failed to parse
# instantiates as its base node (e.g. HSplitContainer) and the suite silently
# "passes". We assert every project script compiles and editors attach their script.
func _test_scripts_compile() -> void:
    print("--- scripts compile ---")
    var scripts: Array = [
        "res://scripts/world_archive.gd",
        "res://scripts/screen_store.gd",
        "res://scripts/map_editor.gd",
        "res://scripts/placement_editor.gd",
        "res://scripts/screen_minimap.gd",
        "res://scripts/tile_palette.gd",
        "res://scripts/main.gd",
    ]
    var bad := 0
    for path in scripts:
        var s = load(path)
        if s == null or not (s as GDScript).can_instantiate():
            bad += 1
            print("FAIL: script did not compile: " + path)
    check(bad == 0, "no project script fails to compile")

func _test_screen_store() -> void:
    print("--- ScreenStore ---")
    var store := ScreenStore.new()
    store.register(0, 0, 1, "screen_1.json")
    store.register(2, 0, 2, "screen_2.json")
    check(store.is_occupied(0, 0), "occupied (0,0)")
    check(not store.is_occupied(1, 0), "empty (1,0)")
    check(store.screen_id_at(2, 0) == 2, "id lookup")
    check(store.position_of_id(1) == Vector2i(0, 0), "id->pos")
    check(store.next_id() == 3, "next_id")
    check(store.bounds() == Rect2i(0, 0, 3, 1), "bounds 0,0..2,0")
    check(store.move(Vector2i(0, 0), Vector2i(0, 1)), "move ok")
    check(not store.is_occupied(0, 0) and store.is_occupied(0, 1), "move applied")
    check(not store.move(Vector2i(0, 1), Vector2i(2, 0)), "move to occupied rejected")
    check(store.next_free_position() == Vector2i(0, 0), "next free position")
    store.remove(0, 1)
    check(not store.is_occupied(0, 1), "remove")

    # Negative positions are valid placements and must not collide with the
    # "not found" sentinel Vector2i(-1,-1) (regression: screen 1 at -1,-1 was
    # treated as unplaced and the stamped map rendered as the default grid).
    store.register(-1, -1, 7, "screen_7.json")
    check(store.is_occupied(-1, -1), "negative position is occupied")
    check(store.has_id(7), "has_id true for screen at -1,-1")
    check(store.position_of_id(7) == Vector2i(-1, -1), "position_of_id returns the real negative position")
    check(not store.has_id(999), "has_id false for unregistered id")

    var tmp := "/tmp/user/1000/opencode/sstd_world_test.json"
    store.set_dir("/tmp/user/1000/opencode")
    check(store.save_world(tmp), "save_world")
    var store2 := ScreenStore.new()
    check(store2.load_world(tmp), "load_world")
    check(store2.screen_id_at(2, 0) == 2, "roundtrip id")
    check(store2.get_dir() == "/tmp/user/1000/opencode", "dir from load")

func _test_map_editor() -> void:
    print("--- MapEditor screen ops ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    await process_frame
    var store := ScreenStore.new()
    store.set_dir("/tmp/user/1000/opencode")
    ed.set_screen_store(store)

    check(ed._screen_id == 1, "default id 1")
    ed._on_add_screen_at(0, 0)
    check(store.is_occupied(0, 0) and store.screen_id_at(0, 0) == 1, "add screen id 1 at (0,0)")
    ed._on_add_screen_at(1, 0)
    check(store.screen_id_at(1, 0) == 2, "add screen id 2 at (1,0)")

    ed._on_screen_changed(1)
    ed._tiles[ed._key(5, 5)] = "grass"
    ed._cache_current()
    ed._on_screen_changed(2)
    check(ed.get_tile(5, 5) == "air", "switch to empty screen 2")
    ed._on_screen_changed(1)
    check(ed.get_tile(5, 5) == "grass", "tile preserved across screen switch")

    ed._on_screen_changed(99)
    check(ed._screen_pos.x < 0, "unregistered id is unplaced")
    check(ed.get_tile(5, 5) == "air", "unregistered id shows empty canvas")

    # A screen placed at negative coordinates is still "placed": restore must
    # show its grid instead of the default air/grass/dirt (regression).
    var neg_store := ScreenStore.new()
    neg_store.register(-1, -1, 1, "screen_1.json")
    var neg_data: Dictionary = {
        "version": "0.3.0", "screen_id": 1,
        "tiles": [{"x": 4, "y": 4, "terrain": "stamp_neg", "sub_tile_mask": 15}],
    }
    neg_store.set_cache(-1, -1, neg_data)
    var ed2 = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed2)
    await process_frame
    ed2.set_screen_store(neg_store)
    check(ed2._is_current_placed(), "screen at -1,-1 is considered placed")
    ed2._on_screen_changed(1)
    check(ed2.get_tile(4, 4) == "stamp_neg", "negative-position screen restores its grid, not default")
    ed2.queue_free()
    await process_frame

    ed._clipboard = ed._serialize()
    ed._on_clone_screen_at(3, 0)
    var clone_id: int = store.screen_id_at(3, 0)
    check(clone_id == 3, "clone registered as id 3 at (3,0)")
    var clone_path := store.screen_path(3, 0)
    check(FileAccess.file_exists(clone_path), "clone file written")

    store.move(Vector2i(1, 0), Vector2i(1, 1))
    check(store.is_occupied(1, 1) and not store.is_occupied(1, 0), "move via store")

    var dlg = (load("res://scenes/screen_minimap_dialog.tscn") as PackedScene).instantiate()
    ed.add_child(dlg)
    await process_frame
    var grid: ScreenMinimap = dlg.get_node("Panel/VBox/Grid")
    grid.setup(store, "move", Vector2i(0, 0))
    check(grid.mode == "move", "minimap dialog setup")
    dlg.queue_free()
    await process_frame

func _test_placement_editor() -> void:
    print("--- PlacementEditor screen ops ---")
    var ed = (load("res://scenes/placement_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    await process_frame
    var store := ScreenStore.new()
    store.set_dir("/tmp/user/1000/opencode")
    ed.set_screen_store(store)

    ed._on_add_screen_at(0, 0)
    ed._on_add_screen_at(1, 0)
    ed._on_screen_changed(1)
    ed._selected_entity_key = "arrow_tower"
    ed.place_at(5, 5)
    check(ed._placements.size() == 1, "entity placed")
    ed._cache_current()
    ed._on_screen_changed(2)
    check(ed._placements.is_empty(), "switch clears placements view")
    ed._on_screen_changed(1)
    check(ed._placements.size() == 1, "placements preserved across switch")

    var parsed: Dictionary = {
        "version": "0.1.0", "screen_id": 9,
        "width_tiles": 60, "height_tiles": 33,
        "tile_width_px": 32, "tile_height_px": 32,
        "elevation_floor_tiles": 0, "elevation_ceiling_tiles": 4,
        "tiles": [{"x": 3, "y": 4, "terrain": "grass", "sub_tile_mask": 15}],
        "placed_entities": [{"entity_key": "bridge", "world_tile_x": 7, "world_tile_y": 2}],
    }
    ed._apply_screen(parsed)
    check(ed.get_tile(3, 4) == "grass", "apply_screen tile")
    check(int(ed._tile_data["3,4"]["sub_tile_mask"]) == 15, "apply_screen flattened tile_data")
    check(ed._placements.size() == 1 and ed._placements[0]["entity_key"] == "bridge", "apply_screen placements")

func _make_gradient_cell() -> Image:
    var img := Image.create(STAMP_CELL, STAMP_CELL, false, Image.FORMAT_RGBA8)
    for y in STAMP_CELL:
        for x in STAMP_CELL:
            img.set_pixel(x, y, Color(float(x) / 31.0, float(y) / 31.0, 0.5, 1.0))
    return img

func _flip_image(src: Image, flip_h: bool, flip_v: bool) -> Image:
    var out := Image.create(src.get_width(), src.get_height(), false, Image.FORMAT_RGBA8)
    for y in src.get_height():
        for x in src.get_width():
            var sx := (src.get_width() - 1 - x) if flip_h else x
            var sy := (src.get_height() - 1 - y) if flip_v else y
            out.set_pixel(x, y, src.get_pixel(sx, sy))
    return out

func _test_stamp_fingerprint() -> void:
    print("--- MapEditor stamp fingerprint dedup ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    check(ed._stamp_catalog.is_empty(), "fresh catalog empty")
    check(ed._stamp_fp_index.is_empty(), "fresh fingerprint index empty")

    var grad := _make_gradient_cell()
    var grad_fh := _flip_image(grad, true, false)
    var grad_fv := _flip_image(grad, false, true)
    var grad_fhv := _flip_image(grad, true, true)

    var canon: PackedByteArray = ed._canonical_coarse(grad)
    check(ed._fp_hash(canon) == ed._fp_hash(ed._canonical_coarse(grad_fh)), "canonical hash flip-h invariant")
    check(ed._fp_hash(canon) == ed._fp_hash(ed._canonical_coarse(grad_fv)), "canonical hash flip-v invariant")
    check(ed._fp_hash(canon) == ed._fp_hash(ed._canonical_coarse(grad_fhv)), "canonical hash flip-hv invariant")

    var base_hash: int = ed._fp_hash(canon)
    var other := _make_gradient_cell()
    for y in 8:
        for x in 8:
            if (x + y) % 3 == 0:
                other.set_pixel(x, y, Color(1.0, 0.0, 0.0, 1.0))
    check(ed._fp_hash(ed._canonical_coarse(other)) != base_hash, "different image -> different bucket")

    check(ed._find_match(grad) == {}, "no match in empty catalog")

    ed._stamp_catalog.append({"key": "stamp_test", "cell": grad})
    ed._index_stamp_entry(0)
    check(ed._stamp_fp_index.get(base_hash, []).size() == 1, "indexed entry lands in its bucket")

    var m0: Dictionary = ed._find_match(grad)
    check(m0.get("key") == "stamp_test" and not m0.get("flip_h") and not m0.get("flip_v"), "exact match, no flip")
    var mh: Dictionary = ed._find_match(grad_fh)
    check(mh.get("key") == "stamp_test" and mh.get("flip_h") and not mh.get("flip_v"), "flip-h matched as flip_h variant")
    var mhv: Dictionary = ed._find_match(grad_fhv)
    check(mhv.get("key") == "stamp_test" and mhv.get("flip_h") and mhv.get("flip_v"), "flip-hv matched as flip_h+flip_v variant")
    check(ed._find_match(other) == {}, "different image in own bucket -> no false match")

func _stamp_group_count(ed) -> int:
    var n := 0
    for ts in ed._tile_set_groupings:
        if (ts.key as String).begins_with("stamp_"):
            n += 1
    return n

# Regression for #46: re-stamping the same source image at a different map spot must
# update the existing TileSetGrouping (dedupe by key), not append a duplicate brush.
func _test_stamp_brush_dedupe() -> void:
    print("--- MapEditor stamp brush dedupe (#46) ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    await process_frame
    check(_stamp_group_count(ed) == 0, "fresh editor has no stamp brushes")

    var cells_a: Array[Dictionary] = [
        {"local_x": 0, "local_y": 0, "terrain_key": "stamp_1", "flip_h": false, "flip_v": false, "src_col": 0, "src_row": 0},
        {"local_x": 1, "local_y": 0, "terrain_key": "stamp_2", "flip_h": false, "flip_v": false, "src_col": 1, "src_row": 0},
    ]
    ed._stamp_basename = "hero"
    ed._register_stamp_brush(cells_a, 2, 1)
    check(_stamp_group_count(ed) == 1, "first stamp registers one grouping")
    check(ed._find_tile_set("stamp_hero").tiles.size() == 2, "grouping holds the cell entries")

    var cells_b: Array[Dictionary] = [
        {"local_x": 0, "local_y": 0, "terrain_key": "stamp_1", "flip_h": false, "flip_v": false, "src_col": 0, "src_row": 0},
        {"local_x": 1, "local_y": 0, "terrain_key": "stamp_2", "flip_h": false, "flip_v": false, "src_col": 1, "src_row": 0},
    ]
    ed._register_stamp_brush(cells_b, 2, 1)
    check(_stamp_group_count(ed) == 1, "re-stamp same image reuses the grouping, no duplicate (#46)")
    check(ed._find_tile_set("stamp_hero").tiles.size() == 2, "existing grouping still has entries")
    check(ed._selected_tile_set_key == "stamp_hero", "re-stamped brush stays selected")

    var cells_c: Array[Dictionary] = [
        {"local_x": 0, "local_y": 0, "terrain_key": "stamp_7", "flip_h": false, "flip_v": false, "src_col": 0, "src_row": 0},
    ]
    ed._stamp_basename = "villain"
    ed._register_stamp_brush(cells_c, 1, 1)
    check(_stamp_group_count(ed) == 2, "different basename -> distinct grouping")

    ed.queue_free()

    ed.queue_free()

# Acceptance for the opt-in "Prune Duplicates" button (#51): visually-identical
# tiles (identical 2-bit coarse signature AND exact diff <= tolerance) collapse
# onto the lowest stamp id; every reference is rewritten; distinct tiles stay.
func _test_prune_duplicates() -> void:
    print("--- prune duplicates (#51) ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    ed._stamp_catalog.clear()
    ed._tile_images.clear()
    ed._rebuild_stamp_index()
    var cell_a := _make_gradient_cell()
    var cell_b := cell_a.duplicate()
    var cell_c := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    cell_c.fill(Color(0.9, 0.1, 0.1, 1.0))

    ed._stamp_catalog.append({"key": "stamp_5000", "cell": cell_a, "flip_h": false, "flip_v": false})
    ed._stamp_catalog.append({"key": "stamp_5001", "cell": cell_b, "flip_h": false, "flip_v": false})
    ed._stamp_catalog.append({"key": "stamp_5002", "cell": cell_c, "flip_h": false, "flip_v": false})
    ed._tile_images["stamp_5000"] = cell_a
    ed._tile_images["stamp_5001"] = cell_b
    ed._tile_images["stamp_5002"] = cell_c
    ed._rebuild_stamp_index()
    ed._tiles[ed._key(2, 2)] = "stamp_5001"
    ed._refresh_tile_palette()
    var entries_before: int = ed.tile_palette._entries.size()

    var plan: Dictionary = ed._build_prune_plan()
    check(plan["dup_keys"].size() == 1 and plan["dup_keys"][0] == "stamp_5001", "plan finds the exact duplicate")
    check(plan["removed_pngs_est"] == 0, "plan estimates disk-PNG removal without touching disk")
    check(ed._tile_images.has("stamp_5001"), "plan is read-only: no bank mutation yet")
    check(ed.get_tile(2, 2) == "stamp_5001", "plan is read-only: no grid mutation yet")

    await ed._execute_prune(plan["dup_keys"], plan["merge_map"])
    check(not ed._tile_images.has("stamp_5001"), "duplicate tile dropped from bank")
    check(ed._tile_images.has("stamp_5000"), "lowest-id representative kept")
    check(ed._tile_images.has("stamp_5002"), "distinct tile kept")
    check(ed.get_tile(2, 2) == "stamp_5000", "grid reference rewritten to lowest-id tile")
    print("    palette entries before=%d after=%d" % [entries_before, ed.tile_palette._entries.size()])
    var pal_after: Dictionary = {}
    for e in ed.tile_palette._entries:
        pal_after[e["key"]] = true
    check(not pal_after.has("stamp_5001"), "dup key absent from palette after prune")
    check(pal_after.has("stamp_5000"), "rep key present in palette after prune")
    check(pal_after.has("stamp_5002"), "distinct key present in palette after prune")
    var dups := 0
    for e in ed._stamp_catalog:
        if e["key"] == "stamp_5001":
            dups += 1
    check(dups == 0, "duplicate removed from catalog")

    ed.queue_free()
    await process_frame

# Regression (#51 follow-up): a pair of near-identical tiles whose coarse block
# means straddle a `>>6` quantization boundary (e.g. 63 vs 64) must still be
# grouped, because depth-2 shift grids union their buckets.
func _test_prune_boundary_straddler() -> void:
    print("--- prune boundary-straddler ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    ed._stamp_catalog.clear()
    ed._tile_images.clear()
    ed._rebuild_stamp_index()
    var near_63 := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    near_63.fill(Color(63.0 / 255.0, 63.0 / 255.0, 63.0 / 255.0, 1.0))
    var below_64 := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    below_64.fill(Color(64.0 / 255.0, 64.0 / 255.0, 64.0 / 255.0, 1.0))
    var far := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    far.fill(Color(0.9, 0.1, 0.1, 1.0))

    ed._stamp_catalog.append({"key": "stamp_5010", "cell": near_63, "flip_h": false, "flip_v": false})
    ed._stamp_catalog.append({"key": "stamp_5011", "cell": below_64, "flip_h": false, "flip_v": false})
    ed._stamp_catalog.append({"key": "stamp_5012", "cell": far, "flip_h": false, "flip_v": false})
    ed._tile_images["stamp_5010"] = near_63
    ed._tile_images["stamp_5011"] = below_64
    ed._tile_images["stamp_5012"] = far
    ed._rebuild_stamp_index()
    ed._tiles[ed._key(3, 3)] = "stamp_5011"

    var plan: Dictionary = ed._build_prune_plan()
    check(plan["dup_keys"].size() == 1 and plan["dup_keys"][0] == "stamp_5011", "boundary-straddler grouped with 63 (not 64) tile")
    check(plan["merge_map"]["stamp_5011"]["key"] == "stamp_5010", "straddler merges onto lowest-id rep")
    check(not plan["dup_keys"].has("stamp_5012"), "distinct tile is not a dup")
    await ed._execute_prune(plan["dup_keys"], plan["merge_map"])
    check(not ed._tile_images.has("stamp_5011"), "straddler dropped from bank after prune")
    check(ed._tile_images.has("stamp_5010"), "63 rep kept")
    check(ed._tile_images.has("stamp_5012"), "distinct tile kept")
    check(ed.get_tile(3, 3) == "stamp_5010", "straddler grid reference rewritten")

    ed.queue_free()
    await process_frame

func _test_grayscale_projection() -> void:
    print("--- grayscale 4-bit projection ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    var dark := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    dark.fill(Color(0.1, 0.1, 0.1, 1.0))
    var bright := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    bright.fill(Color(0.8, 0.8, 0.8, 1.0))
    var sig: PackedByteArray = ed._grayscale_sig(dark)
    check(sig.size() == 32, "grayscale signature packs 64 nibbles into 32 bytes")
    check(ed._grayscale_projection(dark) < ed._grayscale_projection(bright), "darker tile sorts before brighter tile")
    check(ed._grayscale_projection(dark) == ed._grayscale_projection(dark.duplicate()), "identical tiles share projection")
    ed.queue_free()
    await process_frame

func _test_grayscale_candidates() -> void:
    print("--- grayscale candidate range ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    ed._stamp_catalog.clear()
    var base := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    base.fill(Color(0.2, 0.2, 0.2, 1.0))
    var near := base.duplicate()
    near.fill(Color(0.21, 0.21, 0.21, 1.0))
    var distinct := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    distinct.fill(Color(0.8, 0.1, 0.1, 1.0))
    ed._stamp_catalog.append({"key": "stamp_5020", "cell": base})
    ed._stamp_catalog.append({"key": "stamp_5021", "cell": near})
    ed._stamp_catalog.append({"key": "stamp_5022", "cell": distinct})
    var result: Dictionary = ed._grayscale_projection_candidates(256)
    check(result["candidates"].size() > 0, "projection range produces candidates")
    check(result["exact_matches"].size() == 1, "exact diff rejects distinct projection neighbor")
    check(result["exact_matches"][0]["candidate"] == 1, "exact projection match identifies near tile")
    ed.queue_free()
    await process_frame

func _test_prune_chain() -> void:
    print("--- prune chain resolution ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    ed._stamp_catalog.clear()
    ed._tile_images.clear()
    ed._rebuild_stamp_index()
    var a := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    a.fill(Color(0.20, 0.20, 0.20, 1.0))
    var b := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    b.fill(Color(0.21, 0.21, 0.21, 1.0))
    var c := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    c.fill(Color(0.22, 0.22, 0.22, 1.0))
    var far := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    far.fill(Color(0.9, 0.1, 0.1, 1.0))

    ed._stamp_catalog.append({"key": "stamp_5030", "cell": a})
    ed._stamp_catalog.append({"key": "stamp_5031", "cell": b})
    ed._stamp_catalog.append({"key": "stamp_5032", "cell": c})
    ed._stamp_catalog.append({"key": "stamp_5033", "cell": far})
    ed._tile_images["stamp_5030"] = a
    ed._tile_images["stamp_5031"] = b
    ed._tile_images["stamp_5032"] = c
    ed._tile_images["stamp_5033"] = far
    ed._rebuild_stamp_index()
    var plan: Dictionary = ed._build_prune_plan()
    check(not plan["dup_keys"].has("stamp_5033"), "distinct tile stays in chain test")
    for key in plan["merge_map"]:
        if key == "stamp_5033":
            check(false, "chain plan must not merge distinct tile")
    check(plan["merge_map"].size() <= 2, "chain resolves without exceeding one merge per dup")
    ed.queue_free()
    await process_frame

func _test_a3_neighbor_scan() -> void:
    print("--- A3 neighbour scan ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    ed._stamp_catalog.clear()
    ed._tile_images.clear()
    ed._rebuild_stamp_index()
    var a := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    a.fill(Color(0.25, 0.25, 0.25, 1.0))
    var b := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    b.fill(Color(0.25, 0.25, 0.25, 1.0))
    b.set_pixel(0, 0, Color(0.26, 0.25, 0.25, 1.0))
    var far := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    far.fill(Color(0.9, 0.1, 0.1, 1.0))

    ed._stamp_catalog.append({"key": "stamp_5040", "cell": a, "flip_h": false, "flip_v": false})
    ed._stamp_catalog.append({"key": "stamp_5041", "cell": b, "flip_h": false, "flip_v": false})
    ed._stamp_catalog.append({"key": "stamp_5042", "cell": far, "flip_h": false, "flip_v": false})
    ed._tile_images["stamp_5040"] = a
    ed._tile_images["stamp_5041"] = b
    ed._tile_images["stamp_5042"] = far
    ed._rebuild_stamp_index()

    var canon: PackedByteArray = ed._canonical_coarse(ed._as_rgba8(a))
    var neighbor_hashes: Array = ed._a3_neighbor_hashes(canon)
    check(neighbor_hashes.size() > 0 and neighbor_hashes.size() <= 512, "A3 enumerates bounded one-unit neighbours")
    var plan: Dictionary = ed._build_prune_plan()
    check(plan["dup_keys"].has("stamp_5041"), "A3 path merges a one-block-deviation duplicate")
    check(not plan["dup_keys"].has("stamp_5042"), "A3 path leaves the distinct tile alone")
    ed.queue_free()
    await process_frame

func _test_deep_prune() -> void:
    print("--- deep prune (approach B) ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    ed._stamp_catalog.clear()
    ed._tile_images.clear()
    ed._rebuild_stamp_index()
    var a := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    a.fill(Color(0.2, 0.2, 0.2, 1.0))
    var b := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    b.fill(Color(0.21, 0.21, 0.21, 1.0))
    var far := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    far.fill(Color(0.8, 0.1, 0.1, 1.0))

    ed._stamp_catalog.append({"key": "stamp_5050", "cell": a, "flip_h": false, "flip_v": false})
    ed._stamp_catalog.append({"key": "stamp_5051", "cell": b, "flip_h": false, "flip_v": false})
    ed._stamp_catalog.append({"key": "stamp_5052", "cell": far, "flip_h": false, "flip_v": false})
    ed._tile_images["stamp_5050"] = a
    ed._tile_images["stamp_5051"] = b
    ed._tile_images["stamp_5052"] = far
    ed._rebuild_stamp_index()
    ed._tiles[ed._key(4, 4)] = "stamp_5051"

    var plan: Dictionary = await ed._build_deep_prune_plan()
    check(plan["dup_keys"].size() == 1 and plan["dup_keys"][0] == "stamp_5051", "deep pass finds the exact duplicate")
    check(plan["merge_map"]["stamp_5051"]["key"] == "stamp_5050", "deep pass merges onto lowest-id rep")
    check(not plan["dup_keys"].has("stamp_5052"), "deep pass rejects the distinct tile")
    check(ed.get_tile(4, 4) == "stamp_5051", "deep plan is read-only: no grid mutation yet")
    await ed._execute_prune(plan["dup_keys"], plan["merge_map"])
    check(not ed._tile_images.has("stamp_5051"), "deep merge drops the duplicate from the bank")
    check(ed.get_tile(4, 4) == "stamp_5050", "deep merge rewrites the grid reference")
    ed.queue_free()
    await process_frame

func _test_parallel_exact_scan() -> void:
    print("--- parallel exact scan ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    ed._stamp_catalog.clear()
    ed._tile_images.clear()
    ed._rebuild_stamp_index()
    # 48 identical near-black tiles -> 48*47/2 = 1128 candidate pairs, well
    # above PARALLEL_MIN_PAIRS so the threaded path runs in the test.
    for i in 48:
        var img := Image.create(32, 32, false, Image.FORMAT_RGBA8)
        img.fill(Color(0.05, 0.05, 0.05, 1.0))
        var key := "stamp_6000%d" % i
        ed._stamp_catalog.append({"key": key, "cell": img, "flip_h": false, "flip_v": false})
        ed._tile_images[key] = img
    ed._rebuild_stamp_index()
    var plan: Dictionary = ed._build_prune_plan()
    check(plan["dup_keys"].size() == 47, "parallel scan merges all 47 duplicates")
    check(plan["merge_map"].values().all(func(m): return m["key"] == "stamp_60000"), "parallel scan merges onto lowest id")
    check(plan["dup_keys"].all(func(k): return k != "stamp_60000"), "rep itself is never a dup")
    ed.queue_free()
    await process_frame

func _test_diff_capped() -> void:
    print("--- diff capped ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    var a := PackedByteArray()
    var b := PackedByteArray()
    var n := 64
    for i in n:
        a.append(i)
        b.append(i)
    check(is_equal_approx(ed._diff_capped(a, b, 4.0), 0.0), "cap matches for identical input")
    b[0] = 200
    check(ed._diff_capped(a, b, 4.0) > 4.0, "cap reports over-limit when deviation is heavy")
    b[0] = 1
    check(ed._diff_capped(a, b, 4.0) <= 4.0, "cap returns true value under limit")
    ed.queue_free()
    await process_frame

# Acceptance: save/load whole world as a .zip; a tile referenced by many screens
# is stored ONCE; a world that invents a new gameplay terrain type is rejected.
func _test_world_archive_roundtrip() -> void:
    print("--- WorldArchive round-trip ---")
    var tmp_zip := "/tmp/user/1000/opencode/sstd_world_roundtrip.zip"
    var shared_img := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    shared_img.fill(Color(0.2, 0.5, 0.8, 1.0))
    var brick_img := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    brick_img.fill(Color(0.6, 0.3, 0.1, 1.0))
    var manifest := {
        "0,0": {"x": 0, "y": 0, "id": 1},
        "1,0": {"x": 1, "y": 0, "id": 2},
    }
    var screens := {
        1: {
            "version": "0.3.0", "screen_id": 1,
            "tiles": [
                {"x": 5, "y": 5, "terrain": "grass", "sub_tile_mask": 15},
                {"x": 6, "y": 5, "terrain": "stamp_tower_1", "sub_tile_mask": 0},
            ],
        },
        2: {
            "version": "0.3.0", "screen_id": 2,
            "tiles": [
                {"x": 3, "y": 2, "terrain": "grass", "sub_tile_mask": 15},
                {"x": 4, "y": 2, "terrain": "stamp_tower_1", "sub_tile_mask": 0},
            ],
        },
    }
    var tiles := {"grass": shared_img, "stamp_tower_1": brick_img}
    check(WorldArchive.save_world(tmp_zip, manifest, screens, tiles), "save_world writes zip")
    var data := WorldArchive.load_world(tmp_zip)
    check(data.get("ok", false), "load_world ok")

    var z := ZIPReader.new()
    z.open(tmp_zip)
    var files: PackedStringArray = z.get_files()
    var tile_count := 0
    for f in files:
        if f.begins_with("tiles/"):
            tile_count += 1
    z.close()
    check(files.has("manifest.json"), "manifest.json present")
    check(files.has("screens/1.json") and files.has("screens/2.json"), "per-screen files present")
    check(tile_count == 2, "world-shared tile stored exactly once")
    check(data["screens"].size() == 2, "two screens round-trip")
    check(data["manifest"].has("0,0") and data["manifest"].has("1,0"), "manifest round-trip")
    check(data["tile_images"].has("grass") and data["tile_images"].has("stamp_tower_1"), "tile bank round-trip")
    var ref_img: Image = data["tile_images"]["grass"]
    check(ref_img.get_width() == 32 and ref_img.get_format() == Image.FORMAT_RGBA8, "banked tile decoded to RGBA8 32x32")
    var px: Color = ref_img.get_pixel(0, 0)
    check(px.r < 0.3 and px.g > 0.4 and px.b > 0.7, "banked tile color preserved")

    var bad := screens.duplicate(true)
    bad[2] = bad[2].duplicate(true)
    bad[2]["tiles"].append({"x": 9, "y": 9, "terrain": "hotspring", "sub_tile_mask": 0})
    var tmp_bad := "/tmp/user/1000/opencode/sstd_world_bad.zip"
    check(WorldArchive.save_world(tmp_bad, manifest, bad, tiles), "save_world writes bad zip")
    check(not WorldArchive.load_world(tmp_bad).get("ok", false), "world with invented terrain key rejected")
    DirAccess.remove_absolute(tmp_bad)
    DirAccess.remove_absolute(tmp_zip)

# Acceptance: a saved world is fully self-contained. Reopening it (fresh editor,
# empty catalog, no assets/tiles/ PNG for the tile) still restores the tile bank
# from the package, so no screen renders blank.
func _test_world_reopen_not_blank() -> void:
    print("--- world reopen not blank (no disk assets) ---")
    var tmp_zip := "/tmp/user/1000/opencode/sstd_world_reopen.zip"
    var tile_key := "stamp_orange"
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    await process_frame
    var store := ScreenStore.new()
    store.set_dir("/tmp/user/1000/opencode")
    ed.set_screen_store(store)

    # Author a private tile that does NOT exist in assets/tiles, used by 2 screens.
    var custom := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    custom.fill(Color(0.9, 0.6, 0.2, 1.0))
    ed._tile_images[tile_key] = custom
    ed._on_add_screen_at(0, 0)
    ed._tiles[ed._key(3, 3)] = tile_key
    ed._cache_current()
    ed._on_add_screen_at(1, 0)
    ed._tiles[ed._key(7, 7)] = tile_key
    ed._cache_current()

    var world: Dictionary = ed._collect_world_manifest()
    var bank: Dictionary = ed._collect_world_tiles(world["screens"])
    check(bank.has(tile_key), "collect gathers private tile")
    check(WorldArchive.save_world(tmp_zip, world["manifest"], world["screens"], bank), "save authored world")
    var z2 := ZIPReader.new()
    z2.open(tmp_zip)
    var once := 0
    for f in z2.get_files():
        if f == "tiles/%s.png" % tile_key:
            once += 1
    z2.close()
    check(once == 1, "shared private tile stored once across whole world")
    ed.queue_free()
    await process_frame
    DirAccess.remove_absolute(ProjectSettings.globalize_path("res://assets/tiles/%s_32x32.png" % tile_key))

    var ed2 = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed2)
    await process_frame
    var store2 := ScreenStore.new()
    store2.set_dir("/tmp/user/1000/opencode")
    ed2.set_screen_store(store2)
    ed2._on_screen_changed(1)
    check(not ed2._tile_images.has(tile_key), "fresh editor has no private tile cached")
    check(ed2.get_tile_image(tile_key) == null, "private tile absent from disk/catalog before load")

    ed2._load_world_package(tmp_zip)
    var restored: Image = ed2.get_tile_image(tile_key)
    check(restored != null, "world restores tile bank from package without disk assets")
    if restored:
        check(restored.get_format() == Image.FORMAT_RGBA8, "restored tile is RGBA8")
        var rp: Color = restored.get_pixel(0, 0)
        check(rp.r > 0.8 and rp.g > 0.5, "restored tile keeps authored color")
    check(store2.screen_id_at(1, 0) == 2, "reopen: second screen registered")
    ed2._on_screen_changed(2)
    check(ed2.get_tile(7, 7) == tile_key, "reopen: shared tile placed in screen 2")
    check(ed2.get_tile(3, 3) != "", "reopen: screen not blank")
    ed2.queue_free()
    await process_frame
    DirAccess.remove_absolute(tmp_zip)
    DirAccess.remove_absolute(ProjectSettings.globalize_path("res://assets/tiles/%s_32x32.png" % tile_key))

# Acceptance: the editor's import/bootstrap must follow a world.json *pointer*
# to the referenced .zip. Regression for #43 hotfix: auto-load passed world.json
# to _on_import_file which rejected it with "Invalid screen file" -> blank editor.
func _test_world_pointer_bootstrap() -> void:
    print("--- world pointer bootstrap (world.json -> .zip) ---")
    var tile_key := "stamp_pointer"
    var tmp_zip := "/tmp/user/1000/opencode/sstd_world_pointer.zip"
    var tmp_pointer := "/tmp/user/1000/opencode/world_pointer.json"
    var png := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    png.fill(Color(0.1, 0.9, 0.3, 1.0))
    var manifest := {"0,0": {"x": 0, "y": 0, "id": 1}}
    var screens := {1: {"version": "0.3.0", "screen_id": 1, "tiles": [{"x": 2, "y": 3, "terrain": tile_key}]}}
    var tiles := {tile_key: png}
    check(WorldArchive.save_world(tmp_zip, manifest, screens, tiles), "save world for pointer bootstrap")
    var pf := FileAccess.open(tmp_pointer, FileAccess.WRITE)
    pf.store_string(JSON.stringify({"version": "0.2.0", "world": tmp_zip.get_file()}, "\t"))
    pf.close()

    # resolve_world_pointer: relative path resolves against pointer's dir.
    var resolved := WorldArchive.resolve_world_pointer(tmp_pointer)
    check(resolved == tmp_zip, "resolve_world_pointer follows relative reference")
    check(WorldArchive.resolve_world_pointer(tmp_zip) == tmp_zip, "resolve_world_pointer passes through .zip")
    check(WorldArchive.resolve_world_pointer("/nonexistent/x.json") == "", "resolve_world_pointer empty when missing")

    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    await process_frame
    var store := ScreenStore.new()
    store.set_dir("/tmp/user/1000/opencode")
    ed.set_screen_store(store)
    # Bootstrap a pointer file like main._auto_load_last_map -> map_editor._on_import_file
    ed._on_import_file(tmp_pointer)
    check(store.is_occupied(0, 0), "bootstrap registered screen at (0,0)")
    check(ed.get_tile_image(tile_key) != null, "bootstrap restored tile bank from package")
    ed._on_screen_changed(1)
    check(ed.get_tile(2, 3) == tile_key, "bootstrap: grid not blank, tile placed")
    check(store.world_package_path == tmp_zip, "store tracks world_package_path after pointer load")

    ed.queue_free()
    await process_frame
    DirAccess.remove_absolute(tmp_zip)
    DirAccess.remove_absolute(tmp_pointer)
    DirAccess.remove_absolute(ProjectSettings.globalize_path("res://assets/tiles/%s_32x32.png" % tile_key))

func _test_tile_palette() -> void:
    print("--- TilePalette (issue #49) ---")
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    await process_frame
    await ed._load_stamp_catalog()
    var palette = ed.get_node("LeftPanel/TilePalette")
    check(palette != null and palette.get("script") != null, "map_editor has TilePalette node")
    check(palette.has_method("populate"), "TilePalette has populate")
    check(palette.has_method("select_key"), "TilePalette has select_key")
    check(ed._tile_images.size() > 0, "world-shared tile bank populates from stamp catalog")
    check(palette.get_item_count() >= ed._terrain_types.size(), "palette shows all terrain entries")
    check(palette.get_item_count() >= ed._tile_images.size(), "palette shows every world-shared tile")

    # Selecting a terrain swatch maps to the terrain_* grouping brush.
    var picked: Array = []
    palette.tile_picked.connect(func(kind: String, key: String): picked.assign([kind, key]))
    palette.item_selected.emit(0)
    await process_frame
    check(picked.size() == 2 and picked[0] == "terrain", "tile_picked emitted with kind 'terrain'")
    check(not ed._selected_tile_set_key.is_empty(), "terrain pick sets a brush key")

    # Selecting a world-shared tile (kind "tile") selects it as a paint brush.
    var tile_idx: int = ed._terrain_types.size()
    if tile_idx < palette.get_item_count():
        palette.item_selected.emit(tile_idx)
        await process_frame
        check(picked.size() == 2 and picked[0] == "tile", "tile_picked emitted with kind 'tile'")
        check(ed._selected_tile_set_key == picked[1], "tile pick selects the single tile brush")

    ed.queue_free()
    await process_frame
