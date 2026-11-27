extends SceneTree
# Slope conversion seam guards. Written RED before the orchestrator exists:
# every guard checks the seam by name first so a missing method records a failure and the suite
# still reaches quit(1). No race applies: all inputs are single-threaded Dictionaries.
# Back-to-back safety is pinned by merging the same generated set twice.
# Measured on 4.4.1 and 4.7.2: JSON.parse_string returns every JSON number as a float, so an
# "integer on the wire" check must inspect the serialized literal, not the parsed Variant.

const MODEL_GATEWAY_PATH: String = "res://scripts/model_gateway.gd"
const SLOPE_TOOL_PATH: String = "res://scripts/slope_conversion_tool.gd"
const GENERATED_KEY_PREFIX: String = "slope_grass_30deg_4x2_"
const EXISTING_ENTRY_COUNT: int = 14
const GENERATED_CELL_COUNT: int = 8
const EXPECTED_ENTRY_COUNT: int = 30
# The Rust convert-slope reader pins this format token; a drift here looks like a clean run until
# the converter rejects the whole request with a version error.
const EXPECTED_REQUEST_VERSION: String = "1.0.0"
const EXPECTED_TILE_WIDTH_PX: int = 32
const EXPECTED_TILE_HEIGHT_PX: int = 32
const CAPTURE_RUN_DIR: String = "user://data/tools/sstd_stdout_capture_guard"

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


# Match the literal after a JSON field name; a fractional literal means the coercion seam missed it.
func _all_wire_integers(text: String, field_name: String) -> bool:
    var regex: RegEx = RegEx.new()
    regex.compile("\"" + field_name + "\"\\s*:\\s*(-?[0-9]+(?:\\.[0-9]+)?)")
    var matches: Array[RegExMatch] = regex.search_all(text)
    var fn_has_no_fraction: Callable = func(match_result: RegExMatch) -> bool:
        return not match_result.get_string(1).contains(".")
    return not matches.is_empty() and matches.all(fn_has_no_fraction)


func _has_fractional_number(text: String) -> bool:
    var regex: RegEx = RegEx.new()
    regex.compile("-?[0-9]+\\.[0-9]+")
    return regex.search(text) != null


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
    _guard_request_version_matches_rust_reader()
    _guard_headless_grid_size_comes_from_config_db()
    _guard_convert_all_captures_converter_stdout()
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
    var cells: Array = parsed.get("cells", [])
    _check(cells.size() == 1, "one cell survives coercion")
    for field_name in ["width_px", "height_px", "x", "y",
            "local_x", "local_y", "source_col", "source_row"]:
        _check(_all_wire_integers(request_text, field_name),
            "%s is an integer literal on the wire" % field_name)
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
    # A keyed upsert must leave the framework entries where they were; sorting them rewrites the
    # committed catalog even though no framework value changed.
    var framework_order_kept: bool = true
    for base_index in range(EXISTING_ENTRY_COUNT):
        var entry: Dictionary = merged_again[base_index]
        framework_order_kept = framework_order_kept and String(entry.get("key", "")) == "base_%d" % base_index
    _check(framework_order_kept, "framework entries keep their original order after keyed upsert")
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
    for field_name in ["sub_tile_mask", "elevation_tiles"]:
        _check(_all_wire_integers(json_text, field_name),
            "%s is an integer literal in canonical JSON" % field_name)
    _check(not _has_fractional_number(json_text),
        "solid_mask bytes serialize without a decimal point")
    # Canonicalization must not invent an optional field on pre-existing framework entries.
    var without_mask: Array = [{"key": "no_mask", "elevation_tiles": 0.0}]
    var without_mask_text: String = String(tool.call("canonical_terrain_json", without_mask))
    var without_mask_decoded: Variant = JSON.parse_string(without_mask_text)
    var preserved: bool = without_mask_decoded is Array and not (without_mask_decoded as Array)[0].has("solid_mask")
    _check(preserved, "canonical JSON preserves an entry with no solid_mask")
    _completed.append("terrain_json_resaves_integral_float_as_integer")


# A version drift only shows up when the Rust reader rejects the whole request, so pin the
# orchestrator's wire token directly.
func _guard_request_version_matches_rust_reader() -> void:
    var tool_script: GDScript = load(SLOPE_TOOL_PATH) as GDScript
    if tool_script == null:
        _check(false, "SlopeConversionTool script exists for the version guard")
        return
    var constants: Dictionary = tool_script.get_script_constant_map()
    _check(String(constants.get("REQUEST_VERSION", "")) == EXPECTED_REQUEST_VERSION,
        "request version matches the Rust reader's format token")
    _completed.append("request_version_matches_rust_reader")


# A freshly instantiated bridge answers with its code default (64 px), not the runtime config
# DB. Pin the headless path to the configured size so a whole conversion is never scaled wrongly.
func _guard_headless_grid_size_comes_from_config_db() -> void:
    var tool: Object = _load_object(SLOPE_TOOL_PATH)
    if tool == null or not tool.has_method("_grid_tile_size"):
        _check(false, "SlopeConversionTool._grid_tile_size exists")
        return
    var tile_size: Array = tool.call("_grid_tile_size")
    _check(tile_size.size() == 2, "headless grid size returns a width and a height")
    if tile_size.size() == 2:
        _check(int(tile_size[0]) == EXPECTED_TILE_WIDTH_PX,
            "headless grid width comes from the config DB")
        _check(int(tile_size[1]) == EXPECTED_TILE_HEIGHT_PX,
            "headless grid height comes from the config DB")
    _completed.append("headless_grid_size_comes_from_config_db")


# OS.execute silently drops captured lines when its output parameter is a PackedStringArray;
# without those lines the decoded result is empty and a successful conversion looks like a refusal.
func _guard_convert_all_captures_converter_stdout() -> void:
    var tool: Object = _load_object(SLOPE_TOOL_PATH)
    if tool == null or not tool.has_method("convert_all"):
        _check(false, "SlopeConversionTool.convert_all exists")
        return
    var conversion: Dictionary = tool.call("convert_all", CAPTURE_RUN_DIR)
    _check(bool(conversion.get("ok", false)),
        "convert_all captures the converter stdout: %s" % String(conversion.get("error", "")))
    var result_path: String = ProjectSettings.globalize_path(CAPTURE_RUN_DIR).path_join("result.json")
    _check(FileAccess.file_exists(result_path), "convert_all writes the decoded result artifact")
    _completed.append("convert_all_captures_converter_stdout")
