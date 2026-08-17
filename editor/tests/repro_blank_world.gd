extends SceneTree

var failures := 0

func check(cond: bool, msg: String) -> void:
    if cond:
        print("  ok  " + msg)
    else:
        failures += 1
        print("  FAIL " + msg)

func _initialize() -> void:
    await _repro_placement_blank()
    await _repro_reopen_boot()
    print("=== repro done, failures=%d ===" % failures)
    quit(failures)

# #47: Placement Editor shows blank after a world was loaded in the Map Editor.
func _repro_placement_blank() -> void:
    print("--- #47: placement editor blank after world load ---")
    var tmp_zip := "/tmp/user/1000/opencode/sstd_repro47.zip"
    var tile_key := "stamp_repro47"
    DirAccess.remove_absolute(tmp_zip)

    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    await process_frame
    var store := ScreenStore.new()
    store.set_dir("/tmp/user/1000/opencode")
    ed.set_screen_store(store)

    var custom := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    custom.fill(Color(0.9, 0.6, 0.2, 1.0))
    ed._tile_images[tile_key] = custom
    ed._on_add_screen_at(0, 0)
    ed._tiles[ed._key(3, 3)] = tile_key
    ed._cache_current()

    var world: Dictionary = ed._collect_world_manifest()
    var bank: Dictionary = ed._collect_world_tiles(world["screens"])
    check(WorldArchive.save_world(tmp_zip, world["manifest"], world["screens"], bank), "save authored world")
    ed.queue_free()
    await process_frame

    # Fresh boot: map editor loads the world; placement editor only gets the store.
    var ed2 = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed2)
    await process_frame
    var store2 := ScreenStore.new()
    store2.set_dir("/tmp/user/1000/opencode")
    ed2.set_screen_store(store2)
    ed2._load_world_package(tmp_zip)
    check(ed2.get_tile_image(tile_key) != null, "map editor tile bank seeded from package")

    var pe = (load("res://scenes/placement_editor.tscn") as PackedScene).instantiate()
    root.add_child(pe)
    await process_frame
    pe.set_screen_store(store2)
    # Simulate the self-contained case: NO disk PNG for the private tile, exactly
    # like _test_world_reopen_not_blank does for the map editor.
    DirAccess.remove_absolute(ProjectSettings.globalize_path("res://assets/tiles/%s_32x32.png" % tile_key))
    # Mimic main.gd _on_tab_changed(3): seed tile bank, sync screen id + restore.
    pe.set_tile_bank(ed2.get_tile_bank())
    pe._screen_id = ed2._screen_id
    pe.screen_spin.set_value_no_signal(pe._screen_id)
    pe._sync_position_from_id()
    if pe._store and pe._store.is_occupied(pe._screen_pos.x, pe._screen_pos.y):
        pe._restore_screen()
    else:
        var grid: Dictionary = ed2.get_tile_grid()
        pe.set_tile_grid(grid["tiles"], grid["tile_data"])
    var tex: Texture2D = pe.get_tile_texture(tile_key)
    check(tex != null, "placement editor has texture for world-shared tile (bug #47)")
    if tex == null:
        print("  NOTE: placement editor _tile_images.size=%d" % pe._tile_images.size())
    else:
        print("  DBG: pe._tile_images.has=%s size=%d" % [pe._tile_images.has(tile_key), pe._tile_images.size()])
        print("  DBG: pe._tiles at (3,3)=%s" % pe.get_tile(3, 3))

    ed2.queue_free()
    pe.queue_free()
    await process_frame
    DirAccess.remove_absolute(tmp_zip)

# #48: quit + reopen must restore the map via the last_map_path config pointer.
func _repro_reopen_boot() -> void:
    print("--- #48: reopen restores world via last_map_path ---")
    var tmp_zip := "/tmp/user/1000/opencode/sstd_repro48.zip"
    var tile_key := "stamp_repro48"
    DirAccess.remove_absolute(tmp_zip)

    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    await process_frame
    var store := ScreenStore.new()
    store.set_dir("/tmp/user/1000/opencode")
    ed.set_screen_store(store)

    var custom := Image.create(32, 32, false, Image.FORMAT_RGBA8)
    custom.fill(Color(0.2, 0.6, 0.9, 1.0))
    ed._tile_images[tile_key] = custom
    ed._on_add_screen_at(0, 0)
    ed._tiles[ed._key(5, 5)] = tile_key
    ed._cache_current()
    var world: Dictionary = ed._collect_world_manifest()
    var bank: Dictionary = ed._collect_world_tiles(world["screens"])
    check(WorldArchive.save_world(tmp_zip, world["manifest"], world["screens"], bank), "save authored world")

    # Record last_map_path exactly as the editor does on save/load, via the bridge.
    if ClassDB.class_exists("SstdBridge"):
        var b: Node = ClassDB.instantiate("SstdBridge")
        root.add_child(b)
        var db_path := ProjectSettings.globalize_path("res://config/sstd_config.sqlite3")
        b.init_config_db(db_path)
        b.set_config_value("last_map_path", tmp_zip)
        b.queue_free()
    else:
        check(false, "SstdBridge available for last_map_path persistence test")
    ed.queue_free()
    await process_frame

    # Fresh boot path: read config + _on_import_file, like main.gd _auto_load_last_map.
    var ed2 = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed2)
    await process_frame
    var store2 := ScreenStore.new()
    store2.set_dir("/tmp/user/1000/opencode")
    ed2.set_screen_store(store2)
    var last := ""
    if ClassDB.class_exists("SstdBridge"):
        var b: Node = ClassDB.instantiate("SstdBridge")
        root.add_child(b)
        var db_path := ProjectSettings.globalize_path("res://config/sstd_config.sqlite3")
        b.init_config_db(db_path)
        last = b.get_config_value("last_map_path") as String
        b.queue_free()
    print("  last_map_path=%s" % last)
    if not last.is_empty() and FileAccess.file_exists(last):
        ed2._on_import_file(last)
        check(ed2.get_tile_image(tile_key) != null, "reopen restores tile bank")
        check(ed2.get_tile(5, 5) == tile_key, "reopen restores screen tiles (bug #48)")
    else:
        check(false, "last_map_path persisted and resolvable (bug #48)")
    ed2.queue_free()
    await process_frame
    DirAccess.remove_absolute(tmp_zip)