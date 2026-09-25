extends SceneTree

# #67 regression guard: prototype-based terrain/entity override merge must leave
# unspecified framework fields intact and apply the specified patch values.
#
# Validates the GDScript merge path that boot uses (`on_world_loaded` ->
# `apply_terrain_overrides` / `apply_world_entity_defs`) plus the archive codec
# that carries the override delta to and from disk.
#
# Inputs are built in-test from the TRACKED framework prototypes in
# `res://default_package/`. This guard deliberately does not read any saved world
# archive: `editor/world.zip` is gitignored and untracked, so depending on it made
# this test assert machine state instead of code behaviour -- it could not pass on
# a fresh clone, and it went red for reasons unrelated to the merge. See issue #79
# and docs/PLAN-2026-09-25-override-merge-guard-hermetic.md.

var _failures := 0

# Each guard appends its name on its final line. A guard that dies partway (a
# runtime script error aborts the coroutine but lets _run continue to the next
# guard) would otherwise skip its remaining assertions and still let the suite
# report failures=0. These checks turn a silently-truncated guard into a failure.
var _completed: Array[String] = []

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

func _framework_sub_tile_mask(terrains: Array[Dictionary], key: String) -> int:
    for t in terrains:
        if t.get("key", "") == key:
            return int(t.get("sub_tile_mask", -1))
    return -1

func _framework_field(entities: Array[Dictionary], key: String, field: String, fallback: Variant) -> Variant:
    for e in entities:
        if e.get("key", "") == key:
            return e.get(field, fallback)
    return fallback

# --- Guard A: terrain merge semantics -----------------------------------------
# The patch INVERTS the framework (air 0->15, dirt 15->0) so every assertion can
# fail. Asserting the framework's own values here would be vacuous.
func _guard_terrain_merge(fw_terrains: Array[Dictionary]) -> void:
    print("--- Guard A: terrain override merge ---")
    var patch := {"air": {"sub_tile_mask": 15}, "dirt": {"sub_tile_mask": 0}}

    # Vacuity tripwire: if a framework change ever makes the patch a no-op, fail
    # loudly instead of silently asserting nothing.
    var differs := 0
    for k in patch.keys():
        var key: String = k
        var want: int = int((patch[key] as Dictionary).get("sub_tile_mask", -1))
        if _framework_sub_tile_mask(fw_terrains, key) != want:
            differs += 1
        else:
            print("  tripwire: patch for '%s' (%d) equals framework value -- assertion would be vacuous"
                % [key, want])
    check(differs == patch.size(), "terrain patch differs from framework for all %d keys (%d differ)"
        % [patch.size(), differs])

    var med := (load("res://scenes/terrain_editor.tscn") as PackedScene).instantiate()
    root.add_child(med)
    await process_frame
    med.set_framework_terrains(fw_terrains)
    med.apply_terrain_overrides(patch)

    var got: Dictionary = {}
    for t in med.get_terrain_types():
        got[t.get("key", "")] = t

    check(int((got.get("air", {}) as Dictionary).get("sub_tile_mask", -1)) == 15,
        "air sub_tile_mask overridden to 15 (got %d)"
            % int((got.get("air", {}) as Dictionary).get("sub_tile_mask", -1)))
    check(int((got.get("dirt", {}) as Dictionary).get("sub_tile_mask", -1)) == 0,
        "dirt sub_tile_mask overridden to 0 (got %d)"
            % int((got.get("dirt", {}) as Dictionary).get("sub_tile_mask", -1)))
    # The other half of the stated purpose: an unpatched key keeps the framework value.
    var grass_fw: int = _framework_sub_tile_mask(fw_terrains, "grass")
    var grass_got: int = int((got.get("grass", {}) as Dictionary).get("sub_tile_mask", -1))
    check(grass_got == grass_fw,
        "unpatched grass sub_tile_mask intact (%d, framework %d)" % [grass_got, grass_fw])
    med.queue_free()
    await process_frame
    _completed.append("terrain_merge")

# --- Guard B: entity merge semantics ------------------------------------------
# Every patched field is chosen to DIFFER from the tracked framework, and each
# patch omits at least one framework field. Without that, the assertions pass
# whether or not the merge ran -- which is exactly the defect this guard had.
func _guard_entity_merge(fw_entities: Array[Dictionary]) -> void:
    print("--- Guard B: entity override merge ---")
    var patch: Array[Dictionary] = [
        {"key": "arrow_tower", "attack_power": 175, "max_hp": 650},
        {"key": "mine", "attack_power": 275, "max_hp": 75},
        {"key": "steam_tank", "class": "tower", "attack_power": 500, "max_hp": 900},
    ]

    var vacuous: int = 0
    for p in patch:
        for field in p.keys():
            if field == "key":
                continue
            if _framework_field(fw_entities, p.get("key", ""), field, null) == p[field]:
                vacuous += 1
                print("  tripwire: %s.%s=%s equals framework value -- assertion would be vacuous"
                    % [p.get("key", ""), field, p[field]])
    check(vacuous == 0, "no entity patch field equals its framework value (%d vacuous)" % vacuous)

    var eed := (load("res://scenes/entity_editor.tscn") as PackedScene).instantiate()
    root.add_child(eed)
    await process_frame
    eed.set_framework_entities(fw_entities)
    check(eed.get_entity_defs().size() == 8, "entity editor holds 8 framework defs")

    eed.apply_world_entity_defs(patch)
    var merged: Array[Dictionary] = eed.get_entity_defs()

    var by_key: Dictionary = {}
    for e in merged:
        by_key[e.get("key", "")] = e

    check(int((by_key.get("arrow_tower", {}) as Dictionary).get("attack_power", -1)) == 175,
        "arrow_tower attack_power patched to 175 (got %d)"
            % int((by_key.get("arrow_tower", {}) as Dictionary).get("attack_power", -1)))
    check(int((by_key.get("mine", {}) as Dictionary).get("attack_power", -1)) == 275,
        "mine attack_power patched to 275 (got %d)"
            % int((by_key.get("mine", {}) as Dictionary).get("attack_power", -1)))
    # arrow_tower's patch omits "class"; the framework's "tower" must survive.
    check((by_key.get("arrow_tower", {}) as Dictionary).get("class", "") == "tower",
        "arrow_tower unpatched class survives as tower (got %s)"
            % str((by_key.get("arrow_tower", {}) as Dictionary).get("class", "<missing>")))
    # Fully unpatched key untouched.
    var ballista_fw: Variant = _framework_field(fw_entities, "ballista", "attack_power", null)
    check(int((by_key.get("ballista", {}) as Dictionary).get("attack_power", -1)) == int(ballista_fw),
        "unpatched ballista attack_power intact (framework %s)" % str(ballista_fw))
    # Unknown key drives the append branch.
    check(by_key.has("steam_tank"), "unknown key steam_tank appended")
    check(merged.size() == 9, "merged entity defs 8 + 1 appended, no duplicates (%d)" % merged.size())
    eed.queue_free()
    await process_frame
    _completed.append("entity_merge")

# --- Guard C: archive codec round-trip ----------------------------------------
# No test exercised the override delta with a non-empty payload, so a save/load
# asymmetry in this channel would go unnoticed. Also pins the empty-delta rule the
# #79 correction relies on: an empty delta must write NO overrides file.
func _guard_archive_codec(patch: Dictionary) -> void:
    print("--- Guard C: override archive codec round-trip ---")
    var manifest := {"0,0": {"x": 0, "y": 0, "id": 1}}

    var tmp_full := "user://test_override_roundtrip.zip"
    # entity_overrides stays empty; the patch lands in terrain_overrides.
    check(WorldArchive.save_world(tmp_full, manifest, {}, {}, {}, patch),
        "save_world writes zip carrying a non-empty override delta")
    var back: Dictionary = WorldArchive.load_world(tmp_full)
    check(back.get("ok", false), "load_world ok on the non-empty delta archive")

    # A round-trip alone cannot catch a TERRAIN_OVERRIDES_PATH rename, because
    # writer and reader share the constant and a rename is self-consistent. The
    # on-disk name is a backward-compat contract with already-saved worlds, so pin
    # the literal filename here.
    var zf := ZIPReader.new()
    if zf.open(tmp_full) == OK:
        check(zf.get_files().has("terrain_overrides.json"),
            "delta stored under the compat filename terrain_overrides.json")
        zf.close()

    var got: Dictionary = back.get("terrain_overrides", {})
    check(got.size() == patch.size(),
        "override delta survived the archive (%d keys, wrote %d)" % [got.size(), patch.size()])
    for k in patch.keys():
        var key: String = k
        var want: int = int((patch[key] as Dictionary).get("sub_tile_mask", -2))
        var have: int = int((got.get(key, {}) as Dictionary).get("sub_tile_mask", -2))
        check(have == want, "override '%s' round-trips (got %d, wrote %d)" % [key, have, want])
    DirAccess.remove_absolute(ProjectSettings.globalize_path(tmp_full))

    var tmp_empty := "user://test_override_empty.zip"
    check(WorldArchive.save_world(tmp_empty, manifest, {}, {}, {}, {}),
        "save_world writes zip carrying an empty override delta")
    var z := ZIPReader.new()
    var opened: bool = z.open(tmp_empty) == OK
    check(opened, "empty-delta archive opens")
    if opened:
        var files: PackedStringArray = z.get_files()
        check(not files.has("terrain_overrides.json"),
            "empty delta writes no terrain_overrides.json (delta semantics pinned; got %d files)"
                % files.size())
        z.close()
    DirAccess.remove_absolute(ProjectSettings.globalize_path(tmp_empty))
    _completed.append("archive_codec")

func _run() -> void:
    print("--- #67 terrain/entity override merge ---")

    var fw_terrains := _load_framework_terrains()
    var fw_entities := _load_framework_entities()
    check(fw_terrains.size() == 7, "framework terrain types loaded (%d)" % fw_terrains.size())
    check(fw_entities.size() == 8, "framework entity defs loaded (%d)" % fw_entities.size())

    await _guard_terrain_merge(fw_terrains)
    await _guard_entity_merge(fw_entities)
    # Same payload as Guard A, so the codec test and the merge test cannot drift.
    await _guard_archive_codec({"air": {"sub_tile_mask": 15}, "dirt": {"sub_tile_mask": 0}})

    # Truncation check. See the note on _completed: without these, a guard that
    # aborts on a runtime error is silently reported as a pass.
    for g in ["terrain_merge", "entity_merge", "archive_codec"]:
        check(_completed.has(g), "guard '%s' ran to completion" % g)

    print("=== override merge done, failures=%d === " % _failures)
    quit(1 if _failures > 0 else 0)
