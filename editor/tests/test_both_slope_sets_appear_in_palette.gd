extends SceneTree
# Headless guard: both regenerated slope groupings must be discoverable as
# palette inputs after conversion. map_editor.gd::_load_tile_sets accepts a .tres only when it
# loads, has validate(), and validate() passes; this guard applies that same filter, then checks
# that every generated key has both a terrain entry and an on-disk tile PNG.
# Pure file reads on one thread; no race applies.

const RIGHT_RESOURCE_PATH: String = "res://assets/tilesets/slope_grass_30deg_4x2.tres"
const LEFT_RESOURCE_PATH: String = "res://assets/tilesets/slope_grass_30deg_4x2_left.tres"
const TERRAIN_PATH: String = "res://default_package/terrain_types.json"
const TILE_PATH_PATTERN: String = "res://assets/tiles/%s_32x32.png"
const GENERATED_KEY_PREFIX: String = "slope_grass_30deg_4x2_"
const CELLS_PER_GROUPING: int = 8
const GENERATED_KEY_COUNT: int = 16
const EXPECTED_TERRAIN_ENTRY_COUNT: int = 30

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


func _grouping_keys(path: String, label: String) -> Array[String]:
    var grouping: Resource = ResourceLoader.load(path)
    if grouping == null or not grouping.has_method("validate") or not bool(grouping.call("validate")):
        _check(false, "%s grouping loads and validates at %s" % [label, path])
        return []
    var tiles: Array = grouping.get("tiles")
    _check(tiles.size() == CELLS_PER_GROUPING,
        "%s grouping has %d tiles, got %d" % [label, CELLS_PER_GROUPING, tiles.size()])
    var keys: Array[String] = []
    for tile_variant in tiles:
        keys.append(String((tile_variant as Dictionary).get("terrain_key", "")))
    return keys


func _run() -> void:
    var right_keys: Array[String] = _grouping_keys(RIGHT_RESOURCE_PATH, "right")
    var left_keys: Array[String] = _grouping_keys(LEFT_RESOURCE_PATH, "left")
    var all_keys: Array[String] = right_keys + left_keys
    var expected_keys: Array[String] = _generated_keys()
    _check(all_keys.size() == GENERATED_KEY_COUNT,
        "both groupings contribute 16 tiles, got %d" % all_keys.size())
    for expected_key in expected_keys:
        _check(all_keys.has(expected_key), "palette receives generated key %s" % expected_key)
        _check(FileAccess.file_exists(TILE_PATH_PATTERN % expected_key),
            "tile PNG exists for %s" % expected_key)

    var decoded: Variant = JSON.parse_string(FileAccess.get_file_as_string(TERRAIN_PATH))
    _check(decoded is Dictionary, "framework terrain JSON parses")
    if decoded is Dictionary:
        var entries: Array = (decoded as Dictionary).get("tiles", [])
        var entry_keys: Array[String] = []
        for entry_variant in entries:
            entry_keys.append(String((entry_variant as Dictionary).get("key", "")))
        _check(entry_keys.size() == EXPECTED_TERRAIN_ENTRY_COUNT,
            "framework terrain has %d entries, got %d" % [EXPECTED_TERRAIN_ENTRY_COUNT, entry_keys.size()])
        for expected_key in expected_keys:
            _check(entry_keys.has(expected_key), "terrain entry exists for %s" % expected_key)

    print("both_slope_sets failures=%d" % _failures)
    quit(0 if _failures == 0 else 1)
