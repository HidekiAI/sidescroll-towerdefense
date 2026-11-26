extends SceneTree
# Slope conversion seam guards. Written RED before the orchestrator exists:
# every guard checks the seam by name first so a missing method records a failure and the suite
# still reaches quit(1). No race applies: all inputs are single-threaded Dictionaries.
# Back-to-back safety is pinned by merging the same generated set twice.

const MODEL_GATEWAY_PATH: String = "res://scripts/model_gateway.gd"
const SLOPE_TOOL_PATH: String = "res://scripts/slope_conversion_tool.gd"
const GENERATED_KEY_PREFIX: String = "slope_grass_30deg_4x2_"
const EXISTING_ENTRY_COUNT: int = 14
const GENERATED_CELL_COUNT: int = 8
const EXPECTED_ENTRY_COUNT: int = 30

var _failures: int = 0
var _completed: Array[String] = []


func _initialize() -> void:
    _run.call_deferred()


func _check(condition: bool, description: String) -> void:
    if not condition:
        _failures += 1
        printerr("FAIL: " + description)


func _load_object(path: String) -> Object:
    var script: GDScript = load(path) as GDScript
    return script.new() if script != null else null


func _generated_keys() -> Array[String]:
    var keys: Array[String] = []
    for cell_index in range(GENERATED_CELL_COUNT):
        keys.append(GENERATED_KEY_PREFIX + "r" + str(cell_index))
        keys.append(GENERATED_KEY_PREFIX + "l" + str(cell_index))
    return keys


func _run() -> void:
    _guard_conversion_request_coerces_integer_fields()
    _guard_terrain_merge_upserts_existing_keys_without_duplication()
    _guard_terrain_json_resaves_integral_float_as_integer()
    print("slope_conversion failures=%d completed=%d" % [_failures, _completed.size()])
    quit(0 if _failures == 0 else 1)


# The wire boundary must turn Godot's float Variants into integers before stringify; serde
# rejects an integral float for i32/u8 and silently breaks the request.
func _guard_conversion_request_coerces_integer_fields() -> void:
    var gateway: Object = _load_object(MODEL_GATEWAY_PATH)
    if gateway == null or not gateway.has_method("build_slope_conversion_request"):
        _check(false, "ModelGateway.build_slope_conversion_request exists")
        return

    var request: Dictionary = {
        "version": "slope-conversion-1",
        "source_image": "res://assets/samples/tileset_2-2.png",
        "margins_px": {"x": 104.0, "y": 65.0},
        "cell_size_px": {"x": 126.0, "y": 126.0},
        "separation_px": {"x": 1.0, "y": 1.0},
        "width_px": 32.0,
        "height_px": 32.0,
        "cells": [{
            "local_x": 3.0,
            "local_y": 1.0,
            "source_col": 3.0,
            "source_row": 1.0,
            "flip_h": false,
            "generated_key": GENERATED_KEY_PREFIX + "r7",
        }],
    }
    var request_text: String = String(gateway.call("build_slope_conversion_request", request))
    var decoded: Variant = JSON.parse_string(request_text)
    _check(decoded is Dictionary, "conversion request stringifies to a JSON object")
    if not decoded is Dictionary:
        return

    var parsed: Dictionary = decoded
    _check(typeof(parsed.get("width_px")) == TYPE_INT, "width_px is an integer on the wire")
    _check(typeof(parsed.get("height_px")) == TYPE_INT, "height_px is an integer on the wire")
    var margins: Dictionary = parsed.get("margins_px", {})
    _check(typeof(margins.get("x")) == TYPE_INT, "margin x is an integer on the wire")
    _check(typeof(margins.get("y")) == TYPE_INT, "margin y is an integer on the wire")
    var cells: Array = parsed.get("cells", [])
    _check(cells.size() == 1, "one cell survives coercion")
    if not cells.is_empty():
        var cell: Dictionary = cells[0]
        for field_name in ["local_x", "local_y", "source_col", "source_row"]:
            _check(typeof(cell.get(field_name)) == TYPE_INT,
                "%s is an integer on the wire" % field_name)
    _completed.append("conversion_request_coerces_integer_fields")


# A second run must replace generated keys, never append them; the 14 framework entries stay.
func _guard_terrain_merge_upserts_existing_keys_without_duplication() -> void:
    var tool: Object = _load_object(SLOPE_TOOL_PATH)
    if tool == null or not tool.has_method("merge_terrain_entries"):
        _check(false, "SlopeConversionTool.merge_terrain_entries exists")
        return

    var existing: Array = []
    for base_index in range(EXISTING_ENTRY_COUNT):
        existing.append({"key": "base_%d" % base_index, "solid_mask": [], "is_walkable": true})
    var generated: Array = []
    for generated_key in _generated_keys():
        generated.append({"key": generated_key, "solid_mask": [255], "is_walkable": true})

    var merged: Array = tool.call("merge_terrain_entries", existing, generated)
    _check(merged.size() == EXPECTED_ENTRY_COUNT,
        "first merge has 14 existing + 16 generated entries, got %d" % merged.size())
    var merged_again: Array = tool.call("merge_terrain_entries", merged, generated)
    _check(merged_again.size() == EXPECTED_ENTRY_COUNT,
        "second merge upserts instead of growing to 46, got %d" % merged_again.size())

    var seen_keys: Dictionary = {}
    for entry in merged_again:
        var entry_key: String = String((entry as Dictionary).get("key", ""))
        _check(not seen_keys.has(entry_key), "no duplicate key after two merges: %s" % entry_key)
        seen_keys[entry_key] = true
    for base_index in range(EXISTING_ENTRY_COUNT):
        _check(seen_keys.has("base_%d" % base_index), "framework entry base_%d survives" % base_index)
    for generated_key in _generated_keys():
        _check(seen_keys.has(generated_key), "generated key %s is registered" % generated_key)
    _completed.append("terrain_merge_upserts_existing_keys_without_duplication")


# The read side is already pinned in sstd-core; this guard pins the WRITE side of the bridge.
func _guard_terrain_json_resaves_integral_float_as_integer() -> void:
    var tool: Object = _load_object(SLOPE_TOOL_PATH)
    if tool == null or not tool.has_method("canonical_terrain_json"):
        _check(false, "SlopeConversionTool.canonical_terrain_json exists")
        return

    var entries: Array = [{
        "key": "slope_grass_30deg_4x2_r0",
        "sub_tile_mask": 1.0,
        "elevation_tiles": 0.0,
        "solid_mask": [255.0, 0.0],
    }]
    var json_text: String = String(tool.call("canonical_terrain_json", entries))
    var decoded: Variant = JSON.parse_string(json_text)
    _check(decoded is Array, "canonical terrain JSON parses as an array")
    if not decoded is Array:
        return

    var parsed_entries: Array = decoded
    _check(parsed_entries.size() == 1, "one entry survives canonicalization")
    if parsed_entries.is_empty():
        return
    var entry: Dictionary = parsed_entries[0]
    _check(typeof(entry.get("sub_tile_mask")) == TYPE_INT, "sub_tile_mask is an integer, not a float")
    _check(typeof(entry.get("elevation_tiles")) == TYPE_INT, "elevation_tiles is an integer, not a float")
    var mask_bytes: Array = entry.get("solid_mask", [])
    for byte_variant in mask_bytes:
        _check(typeof(byte_variant) == TYPE_INT, "solid_mask bytes are integers, not floats")
    _completed.append("terrain_json_resaves_integral_float_as_integer")
