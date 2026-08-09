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
    _test_screen_store()
    await _test_map_editor()
    await _test_placement_editor()
    _test_stamp_fingerprint()
    print("=== done, failures=%d ===" % _failures)
    quit(0 if _failures == 0 else 1)

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
