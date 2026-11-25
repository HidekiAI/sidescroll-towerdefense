extends SceneTree
# The View delegation seam: physics and terrain models live on the Rust side.
#
# The three static functions in world_view.gd keep their signatures and become thin proxies over
# ModelGateway. These guards pin both sides: the exact calls the proxies make, and a real-bridge
# round trip so a recording stand-in cannot hide a wire-shape mismatch.
#
# Guards append their name on completion; the runner fails on a missing name and gates on the
# process exit code, never on the printed counter.

const GATEWAY_SCRIPT: GDScript = preload("res://scripts/model_gateway.gd")
const TILE_W_PX: float = 32.0
const TILE_H_PX: float = 32.0
const STEP_SECONDS: float = 1.0 / 60.0

const GUARDS: Array[String] = [
    "view_profile_delegates_to_gateway",
    "view_surface_sample_delegates_to_gateway",
    "view_walker_delegates_to_gateway",
    "real_bridge_profile_round_trip",
]

var _failures: int = 0
var _completed: Array[String] = []
var _bridge: RecordingBridge = null


# Record the wire text for each bridge call so a decimal on an integer field is visible.
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


func check(condition: bool, message: String) -> void:
    if condition:
        print("ok: " + message)
    else:
        _failures += 1
        printerr("FAIL: " + message)


func _initialize() -> void:
    _run.call_deferred()


func _run() -> void:
    _test_view_profile_delegation()
    _test_view_surface_sample_delegation()
    _test_view_walker_delegation()
    _test_real_bridge_round_trip()
    _assert_all_guards_completed()
    print("=== test_view_model_delegation: %d failures ===" % _failures)
    quit(0 if _failures == 0 else 1)


# A guard that dies partway would otherwise report a clean counter.
func _assert_all_guards_completed() -> void:
    for guard_name in GUARDS:
        if not _completed.has(guard_name):
            _failures += 1
            printerr("FAIL: guard '%s' did not complete" % guard_name)


# Install a gateway backed by the recording stand-in; caller keeps _bridge for inspection.
func _install_recording_gateway(responses: Dictionary) -> void:
    _bridge = RecordingBridge.new()
    _bridge.responses = responses
    var gateway: Object = GATEWAY_SCRIPT.new()
    gateway.set_bridge(_bridge)
    WorldView.set_model_gateway(gateway)


# Tiles carry float Variants on integer fields, exactly as the editor authors them.
func _float_typed_tiles() -> Array:
    return [{
        "world_tile_x": 0.0,
        "world_tile_y": 1.0,
        "is_walkable": true,
        "solid_mask": [0.0, 128.0],
    }]


# One bridge snapshot: a solid column followed by an air column, across one 32 px tile.
func _snapshot_response() -> String:
    return JSON.stringify({
        "world_left_px": 0.0,
        "world_right_px": 32.0,
        "surface_y_by_column": [32.0, null],
    })


# The View must return the old [profile, left, right] shape, with null decoded to NAN.
func _test_view_profile_delegation() -> void:
    _install_recording_gateway({"build_ground_profile": _snapshot_response()})
    var built: Array = WorldView.build_ground_profile(_float_typed_tiles(), TILE_W_PX, TILE_H_PX)
    check(_bridge.calls.size() == 1,
        "profile: the View makes exactly one gateway call")
    var request_text: String = String(_bridge.calls[0]["request"])
    check(String(_bridge.calls[0]["method"]) == "build_ground_profile"
        and request_text.contains("\"world_tile_x\":0")
        and not request_text.contains("\"world_tile_x\":0.0"),
        "profile: the View sends coerced tiles and grid through the gateway")
    check(built.size() == 3 and absf(float(built[0][0]) - 32.0) < 0.001
        and is_nan(float(built[0][1]))
        and absf(float(built[1])) < 0.001 and absf(float(built[2]) - 32.0) < 0.001,
        "profile: the View unpacks bounds and keeps air columns as NAN")
    _completed.append("view_profile_delegates_to_gateway")


# The View hands its profile array back through the gateway and keeps the NAN contract.
func _test_view_surface_sample_delegation() -> void:
    _install_recording_gateway({"ground_y_at": "{\"y\": 39.0}"})
    var profile: Array = [32.0, 39.0]
    var sampled_y: float = WorldView.ground_y_at(profile, 0.0, 5.0)
    check(_bridge.calls.size() == 1 and String(_bridge.calls[0]["method"]) == "ground_y_at",
        "sample: the View delegates to the gateway")
    var request_text: String = String(_bridge.calls[0]["request"])
    check(request_text.contains("\"pos_x\":5.0")
        and request_text.contains("\"world_left_px\":0.0")
        and request_text.contains("\"surface_y_by_column\":[32.0,39.0]"),
        "sample: the View wraps the profile array as a snapshot")
    check(absf(sampled_y - 39.0) < 0.001,
        "sample: the View returns the gateway's sampled Y")
    _bridge.responses["ground_y_at"] = "{\"y\": null}"
    check(is_nan(WorldView.ground_y_at(profile, 0.0, 6.0)),
        "sample: a null gateway sample stays NAN at the View")
    _completed.append("view_surface_sample_delegates_to_gateway")


# The walker proxy must supply tiles and grid from the profile build, since its signature has neither.
func _test_view_walker_delegation() -> void:
    _install_recording_gateway({
        "build_ground_profile": _snapshot_response(),
        "walk_step": JSON.stringify({"walker": {
            "pos_x": 17.0, "pos_y": 32.0, "vel_x": 200.0, "vel_y": 0.0,
            "grounded": true, "hp": 100.0, "fall_start_y": 32.0,
        }}),
    })
    var built: Array = WorldView.build_ground_profile(
        _float_typed_tiles(), TILE_W_PX, TILE_H_PX)
    var state: Dictionary = {
        "pos_x": 16.0, "pos_y": 32.0, "vel_x": 0.0, "vel_y": 0.0,
        "grounded": true, "hp": 100.0, "fall_start_y": 32.0,
    }
    var next_state: Dictionary = WorldView.step_walker(
        state, 1.0, STEP_SECONDS, built[0], float(built[1]), float(built[2]))
    check(_bridge.calls.size() == 2 and String(_bridge.calls[1]["method"]) == "walk_step",
        "walker: the View delegates to the gateway's walk step")
    var request_text: String = String(_bridge.calls[1]["request"])
    check(request_text.contains("\"move_x\":1.0")
        and request_text.contains("\"pos_x\":16.0")
        and request_text.contains("\"world_tile_x\":0"),
        "walker: the View sends state, intent and cached tiles")
    check(absf(float(next_state.get("pos_x", -1.0)) - 17.0) < 0.001,
        "walker: the View unwraps the gateway's walker result")
    _completed.append("view_walker_delegates_to_gateway")


# A mask bit at local row 7 in columns 5 and 6, with column 7 left as air.
func _bridge_mask_tiles() -> Array:
    var mask: PackedByteArray = PackedByteArray()
    mask.resize(128)
    for local_col in [5, 6]:
        var bit_index: int = 7 * int(TILE_W_PX) + local_col
        mask[bit_index / 8] = mask[bit_index / 8] | (1 << (bit_index % 8))
    return [{
        "world_tile_x": 0,
        "world_tile_y": 1,
        "is_walkable": true,
        "solid_mask": mask,
    }]


# The recording stand-in cannot prove serde accepts the payload; use the real extension here.
func _test_real_bridge_round_trip() -> void:
    if not ClassDB.class_exists("SstdBridge"):
        check(false, "real bridge: SstdBridge is not registered; run scripts/build-bridge.sh first")
        _completed.append("real_bridge_profile_round_trip")
        return
    var bridge: Object = ClassDB.instantiate("SstdBridge")
    var gateway: Object = GATEWAY_SCRIPT.new()
    gateway.set_bridge(bridge)
    WorldView.set_model_gateway(gateway)
    var built: Array = WorldView.build_ground_profile(
        _bridge_mask_tiles(), TILE_W_PX, TILE_H_PX)
    var profile: Array = built[0]
    var world_left_px: float = float(built[1])
    check(profile.size() >= 8,
        "real bridge: the snapshot covers at least the first eight pixel columns")
    check(absf(WorldView.ground_y_at(profile, world_left_px, 5.0) - 39.0) < 0.001,
        "real bridge: the masked surface at column 5 is 39 px")
    check(absf(WorldView.ground_y_at(profile, world_left_px, 6.0) - 39.0) < 0.001,
        "real bridge: the masked surface at column 6 is 39 px")
    check(is_nan(WorldView.ground_y_at(profile, world_left_px, 7.0)),
        "real bridge: an air column stays NAN")
    _completed.append("real_bridge_profile_round_trip")
