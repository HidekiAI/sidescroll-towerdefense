extends SceneTree
# Issue #161 (Rust-owned physics and terrain models): the GDScript DAO transform seam.
#
# The DAO is the trust boundary where Godot Variants become the JSON the Rust bridge deserializes.
# Godot writes a float Variant as `15.0`, while the bridge's wire types are integers, so every
# integer field must be coerced before JSON.stringify. The other half of the boundary is null: the
# snapshot keeps air columns as JSON null, and the sampling boundary turns a null result into NAN.
#
# A recording stand-in is used instead of the real GDExtension. These guards pin what the DAO
# SENDS, which no Rust test can observe; the real-bridge E2E belongs to the later View delegation.
#
# Each guard appends its name on its final line; the runner fails on a missing name and gates on
# the exit code, never the printed counter.

const GRID: Dictionary = {"tile_w": 32.0, "tile_h": 32.0}

# Load by path, not by global class name: a headless --script run does not rescan the class cache.
const MODEL_GATEWAY_SCRIPT: GDScript = preload("res://scripts/model_gateway.gd")

const GUARDS: Array[String] = [
    "profile_request_coerces_integer_fields",
    "profile_response_keeps_air_columns_null",
    "ground_sample_maps_null_to_nan",
    "walk_step_delegates_and_unwraps_walker",
    "terrain_import_coerces_integer_fields",
    "repeated_requests_are_identical",
]

var _failures: int = 0
var _completed: Array[String] = []


# Record the exact JSON handed to the bridge, so a decimal on an integer field is visible.
class RecordingBridge extends RefCounted:
    var calls: Array[Dictionary] = []
    var responses: Dictionary = {}

    func _record(method_name: String, request_json: String) -> void:
        calls.append({"method": method_name, "request": request_json})

    func build_ground_profile(request_json: String) -> String:
        _record("build_ground_profile", request_json)
        return String(responses.get("build_ground_profile", "{}"))

    func ground_y_at(request_json: String) -> String:
        _record("ground_y_at", request_json)
        return String(responses.get("ground_y_at", "{}"))

    func walk_step(request_json: String) -> String:
        _record("walk_step", request_json)
        return String(responses.get("walk_step", "{}"))

    func import_terrain_types(request_json: String) -> String:
        _record("import_terrain_types", request_json)
        return String(responses.get("import_terrain_types", "{}"))


func check(condition: bool, message: String) -> void:
    if condition:
        print("ok: " + message)
    else:
        _failures += 1
        printerr("FAIL: " + message)


func _initialize() -> void:
    _run.call_deferred()


func _run() -> void:
    _test_profile_request_coercion()
    _test_profile_null_columns()
    _test_ground_sample_mapping()
    _test_walk_step_delegation()
    _test_terrain_import_coercion()
    _test_repeated_requests()
    _assert_all_guards_completed()
    print("=== test_model_gateway: %d failures ===" % _failures)
    quit(0 if _failures == 0 else 1)


# A guard that dies partway would otherwise report a clean counter.
func _assert_all_guards_completed() -> void:
    for guard_name in GUARDS:
        if not _completed.has(guard_name):
            _failures += 1
            printerr("FAIL: guard '%s' did not complete" % guard_name)


# Tiles carry float Variants on integer fields, exactly as the editor authors them.
func _float_typed_tiles() -> Array:
    return [{
        "world_tile_x": 15.0,
        "world_tile_y": 3.0,
        "is_walkable": true,
        "solid_mask": [0.0, 255.0],
    }]


# Coerce the tile integers before stringify so serde sees integers, not integral floats.
func _test_profile_request_coercion() -> void:
    var bridge: RecordingBridge = RecordingBridge.new()
    var gateway: Object = MODEL_GATEWAY_SCRIPT.new()
    gateway.set_bridge(bridge)
    gateway.build_profile(_float_typed_tiles(), GRID)
    var request_text: String = String(bridge.calls[0]["request"])
    check(String(bridge.calls[0]["method"]) == "build_ground_profile",
        "coercion: the DAO calls the profile bridge method")
    # Godot parses every JSON number as TYPE_FLOAT, so the wire text is the only valid oracle.
    check(request_text.contains("\"world_tile_x\":15")
        and not request_text.contains("\"world_tile_x\":15.0"),
        "coercion: world_tile_x is emitted as an integer")
    check(request_text.contains("\"world_tile_y\":3")
        and not request_text.contains("\"world_tile_y\":3.0"),
        "coercion: world_tile_y is emitted as an integer")
    check(request_text.contains("\"solid_mask\":[0,255]"),
        "coercion: solid_mask bytes are emitted as integers")
    _completed.append("profile_request_coerces_integer_fields")


# Keep the snapshot's air columns null; only the sampling boundary converts null to NAN.
func _test_profile_null_columns() -> void:
    var bridge: RecordingBridge = RecordingBridge.new()
    var gateway: Object = MODEL_GATEWAY_SCRIPT.new()
    gateway.set_bridge(bridge)
    bridge.responses["build_ground_profile"] = JSON.stringify({
        "world_left_px": 0.0,
        "world_right_px": 96.0,
        "surface_y_by_column": [10.0, null, 30.0],
    })
    var profile: Dictionary = gateway.build_profile([], GRID)
    var columns: Array = profile["surface_y_by_column"]
    check(columns.size() == 3, "null: all three columns survive the decode")
    check(typeof(columns[1]) == TYPE_NIL, "null: an air column stays null, not NAN")
    check(is_equal_approx(float(columns[0]), 10.0) and is_equal_approx(float(columns[2]), 30.0),
        "null: solid columns keep their surface Y")
    _completed.append("profile_response_keeps_air_columns_null")


# Decode a null sample to NAN so every View caller keeps its existing is_nan() guard.
func _test_ground_sample_mapping() -> void:
    var bridge: RecordingBridge = RecordingBridge.new()
    var gateway: Object = MODEL_GATEWAY_SCRIPT.new()
    gateway.set_bridge(bridge)
    var profile: Dictionary = {
        "world_left_px": 0.0,
        "world_right_px": 32.0,
        "surface_y_by_column": [10.0, null],
    }
    bridge.responses["ground_y_at"] = "{\"y\": null}"
    check(is_nan(gateway.ground_y(profile, 16.0)), "sample: a null sample becomes NAN")
    bridge.responses["ground_y_at"] = "{\"y\": 15.0}"
    check(is_equal_approx(gateway.ground_y(profile, 16.0), 15.0),
        "sample: a numeric sample stays a float")
    var request_text: String = String(bridge.calls[1]["request"])
    check(request_text.contains("\"pos_x\":16.0"),
        "sample: the requested X is forwarded to the bridge")
    _completed.append("ground_sample_maps_null_to_nan")


# Route the walker through walk_step and unwrap its result into the walker Dictionary.
func _test_walk_step_delegation() -> void:
    var bridge: RecordingBridge = RecordingBridge.new()
    var gateway: Object = MODEL_GATEWAY_SCRIPT.new()
    gateway.set_bridge(bridge)
    bridge.responses["walk_step"] = JSON.stringify({"walker": {"pos_x": 20.0, "grounded": true}})
    var state: Dictionary = {
        "pos_x": 16.0, "pos_y": 992.0, "vel_x": 0.0, "vel_y": 0.0, "grounded": true,
    }
    var walker: Dictionary = gateway.step_walker(state, {"move_x": 1.0}, [], GRID, 1.0 / 60.0)
    check(String(bridge.calls[0]["method"]) == "walk_step", "step: the DAO delegates to walk_step")
    check(is_equal_approx(float(walker.get("pos_x", -1.0)), 20.0),
        "step: the walker result is unwrapped from the response")
    _completed.append("walk_step_delegates_and_unwraps_walker")


# Coerce every integer field named by the design before the terrain import crosses the wire.
func _test_terrain_import_coercion() -> void:
    var bridge: RecordingBridge = RecordingBridge.new()
    var gateway: Object = MODEL_GATEWAY_SCRIPT.new()
    gateway.set_bridge(bridge)
    var terrain_file: Dictionary = {
        "version": "1.0.0",
        "tiles": [{
            "key": "dirt",
            "display_name": "Dirt",
            "is_walkable": true,
            "is_buildable": true,
            "surface": "normal",
            "hazard": "none",
            "color_hex": "#4a7c3f",
            "elevation_tiles": 1.0,
            "hazard_damage_per_tick": 2.0,
            "destructible_hp": 3.0,
            "sub_tile_mask": 4.0,
            "solid_mask": [0.0, 255.0],
        }],
    }
    gateway.import_terrain_types(terrain_file)
    var request_text: String = String(bridge.calls[0]["request"])
    check(String(bridge.calls[0]["method"]) == "import_terrain_types",
        "import: the DAO calls the terrain import bridge method")
    check(not request_text.contains("\"elevation_tiles\":1.0")
        and not request_text.contains("\"hazard_damage_per_tick\":2.0")
        and not request_text.contains("\"destructible_hp\":3.0")
        and not request_text.contains("\"sub_tile_mask\":4.0"),
        "import: every listed integer field loses its decimal")
    _completed.append("terrain_import_coerces_integer_fields")


# Pin the deterministic back-to-back case: the same input must serialize and decode identically.
func _test_repeated_requests() -> void:
    var bridge: RecordingBridge = RecordingBridge.new()
    var gateway: Object = MODEL_GATEWAY_SCRIPT.new()
    gateway.set_bridge(bridge)
    var tiles: Array = _float_typed_tiles()
    var first: Dictionary = gateway.build_profile(tiles, GRID)
    var second: Dictionary = gateway.build_profile(tiles, GRID)
    check(bridge.calls.size() == 2, "repeat: two calls reach the bridge")
    check(String(bridge.calls[0]["request"]) == String(bridge.calls[1]["request"]),
        "repeat: the same input serializes identically")
    check(first == second, "repeat: the same input yields the same snapshot")
    _completed.append("repeated_requests_are_identical")
