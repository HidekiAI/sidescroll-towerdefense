extends SceneTree

# #67 regression guard: prototype-based terrain/entity override merge must leave
# unspecified framework fields intact and apply the specified patch values.
#
# Validates the GDScript merge path that boot uses (`on_world_loaded` ->
# `apply_terrain_overrides` / `apply_world_entity_defs`) against the real
# `editor/world.zip` and `editor/default_package/` framework prototypes.

var _failures := 0

func check(cond: bool, msg: String) -> void:
    if cond:
        print("ok: " + msg)
    else:
        _failures += 1
        print("FAIL: " + msg)

func _initialize() -> void:
    _run.call_deferred()

func _load_framework_terrains() -> Array[Dictionary]:
    var tf := FileAccess.open("res://default_package/terrain_types.json", FileAccess.READ)
    if tf == null:
        return []
    var parsed = JSON.parse_string(tf.get_as_text())
    tf.close()
    var out: Array[Dictionary] = []
    if typeof(parsed) == TYPE_DICTIONARY and parsed.has("tiles"):
        for item in parsed["tiles"]:
            out.append(item as Dictionary)
    return out

func _load_framework_entities() -> Array[Dictionary]:
    var ef := FileAccess.open("res://default_package/entity_defs.json", FileAccess.READ)
    if ef == null:
        return []
    var parsed = JSON.parse_string(ef.get_as_text())
    ef.close()
    var out: Array[Dictionary] = []
    if typeof(parsed) == TYPE_DICTIONARY and parsed.has("entities"):
        for item in parsed["entities"]:
            out.append(item as Dictionary)
    return out

func _load_world_overrides() -> Dictionary:
    # terrain overrides
    var terrain_overrides: Dictionary = {}
    var tf := FileAccess.open("res://world.zip", FileAccess.READ)
    if tf != null:
        var z := ZIPReader.new()
        z.open("res://world.zip")
        var raw := z.read_file("terrain_overrides.json")
        z.close()
        if raw.size() > 0:
            var parsed = JSON.parse_string(raw.get_string_from_utf8())
            if typeof(parsed) == TYPE_DICTIONARY and parsed.has("overrides"):
                terrain_overrides = parsed["overrides"]
    return terrain_overrides

func _run() -> void:
    print("--- #67 terrain/entity override merge ---")

    var fw_terrains := _load_framework_terrains()
    var fw_entities := _load_framework_entities()
    check(fw_terrains.size() == 7, "framework terrain types loaded (%d)" % fw_terrains.size())
    check(fw_entities.size() == 8, "framework entity defs loaded (%d)" % fw_entities.size())

    var terrain_overrides := _load_world_overrides()
    check(terrain_overrides.has("air") and terrain_overrides.has("dirt"),
        "world terrain overrides present for air+dirt (%d keys)" % terrain_overrides.size())

    # Terrain merge (partial-patch semantics)
    var med := (load("res://scenes/terrain_editor.tscn") as PackedScene).instantiate()
    root.add_child(med)
    await process_frame
    med.set_framework_terrains(fw_terrains)
    var air_base: int = -1
    for t in fw_terrains:
        if t.get("key", "") == "air":
            air_base = int(t.get("sub_tile_mask", -1))
            check(air_base != 15, "framework air sub_tile_mask != 15 (base=%d)" % air_base)
    med.apply_terrain_overrides(terrain_overrides)

    var got_air: int = -1
    var got_dirt: int = -1
    for t in med.get_terrain_types():
        if t.get("key", "") == "air":
            got_air = int(t.get("sub_tile_mask", -1))
        if t.get("key", "") == "dirt":
            got_dirt = int(t.get("sub_tile_mask", -1))
    check(got_air == 15, "air sub_tile_mask overridden to 15 (got %d)" % got_air)
    check(got_dirt == 0, "dirt sub_tile_mask overridden to 0 (got %d)" % got_dirt)
    med.queue_free()
    await process_frame

    # Entity merge by key (apply_world_entity_defs)
    var eed := (load("res://scenes/entity_editor.tscn") as PackedScene).instantiate()
    root.add_child(eed)
    await process_frame
    eed.set_framework_entities(fw_entities)
    check(eed.get_entity_defs().size() == 8, "entity editor holds 8 framework defs")
    # Read the 8 full defs from world.zip and apply
    var z := ZIPReader.new()
    z.open("res://world.zip")
    var e_raw := z.read_file("entity_defs.json")
    z.close()
    var world_entities: Array = []
    if e_raw.size() > 0:
        var parsed = JSON.parse_string(e_raw.get_string_from_utf8())
        if typeof(parsed) == TYPE_DICTIONARY and parsed.has("entities"):
            world_entities = parsed["entities"]
    check(world_entities.size() == 8, "world entity defs count 8 (%d)" % world_entities.size())
    eed.apply_world_entity_defs(world_entities)
    var merged: Array[Dictionary] = eed.get_entity_defs()
    check(merged.size() == 8, "merged entity defs still 8 (no duplicates)")
    for e in merged:
        var key: String = e.get("key", "")
        if key == "arrow_tower":
            check(int(e.get("attack_power", 0)) == 100 and int(e.get("max_hp", 0)) == 500,
                "arrow_tower merged values applied")
        if key == "mine":
            check(e.get("class", "") == "trap", "mine class preserved as trap")
    eed.queue_free()
    await process_frame

    print("=== override merge done, failures=%d === " % _failures)
    quit(1 if _failures > 0 else 0)
