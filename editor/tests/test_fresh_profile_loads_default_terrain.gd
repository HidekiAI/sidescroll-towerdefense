extends SceneTree
# Headless guard: an empty user profile must see the committed default terrain
# without running an import. The orchestrator seam reads default_package when the profile has no
# terrain file; this guard supplies a fresh profile dir under user:// and never touches world.zip.
# Single-threaded file reads; no race applies.

const TOOL_PATH: String = "res://scripts/slope_conversion_tool.gd"
const PROFILE_DIR: String = "user://data/tools/sstd_fresh_profile_guard"
const GENERATED_KEY_PREFIX: String = "slope_grass_30deg_4x2_"
const CELLS_PER_GROUPING: int = 8
const TILE_PATH_PATTERN: String = "res://assets/tiles/%s_32x32.png"
const TILE_WIDTH_PX: int = 32
const TILE_HEIGHT_PX: int = 32
const EXPECTED_ENTRY_COUNT: int = 30
const EXPECTED_MASK_BYTES: int = (TILE_WIDTH_PX * TILE_HEIGHT_PX + 7) / 8

var _failures: int = 0


func _initialize() -> void:
    _run.call_deferred()


func _check(condition: bool, description: String) -> void:
    if not condition:
        _failures += 1
        printerr("FAIL: " + description)


func _generated_keys() -> Array[String]:
    var keys: Array[String] = []
    for cell_index in range(CELLS_PER_GROUPING):
        keys.append(GENERATED_KEY_PREFIX + "r" + str(cell_index))
        keys.append(GENERATED_KEY_PREFIX + "l" + str(cell_index))
    return keys


func _run() -> void:
    var script: GDScript = load(TOOL_PATH) as GDScript
    if script == null:
        _check(false, "SlopeConversionTool script exists at %s" % TOOL_PATH)
        _finish()
        return
    var tool: Object = script.new()
    if not tool.has_method("load_default_terrain"):
        _check(false, "SlopeConversionTool.load_default_terrain exists")
        _finish()
        return

    var loaded: Variant = tool.call("load_default_terrain", PROFILE_DIR)
    _check(loaded is Dictionary, "fresh profile load returns a result object")
    if not loaded is Dictionary:
        _finish()
        return
    var result: Dictionary = loaded
    _check(result.has("imported"), "load result states whether an import ran")
    _check(not bool(result.get("imported", true)), "no import runs for a fresh profile")

    var entries: Array = result.get("tiles", [])
    _check(entries.size() == EXPECTED_ENTRY_COUNT,
        "fresh profile sees %d default entries, got %d" % [EXPECTED_ENTRY_COUNT, entries.size()])
    var entry_by_key: Dictionary = {}
    for entry_variant in entries:
        var entry: Dictionary = entry_variant
        entry_by_key[String(entry.get("key", ""))] = entry
    for generated_key in _generated_keys():
        _check(entry_by_key.has(generated_key), "default terrain carries %s" % generated_key)
        _check(FileAccess.file_exists(TILE_PATH_PATTERN % generated_key),
            "default tile PNG resolves for %s" % generated_key)
        if entry_by_key.has(generated_key):
            var mask_bytes: Array = (entry_by_key[generated_key] as Dictionary).get("solid_mask", [])
            _check(mask_bytes.size() == EXPECTED_MASK_BYTES,
                "%s has a %d-byte solid mask, got %d" % [
                    generated_key, EXPECTED_MASK_BYTES, mask_bytes.size()])

    _finish()


func _finish() -> void:
    print("fresh_profile failures=%d" % _failures)
    quit(0 if _failures == 0 else 1)
