extends RefCounted
# Art -> data orchestrator for the grass slope groupings. GDScript owns the human sign-off gate,
# the request/result JSON and artifact staging; Rust owns greyscale, masks and PNG baking.
# Headless-safe: every path is resolved from res:// or from an explicit user:// output directory.

const MODEL_GATEWAY_PATH: String = "res://scripts/model_gateway.gd"
const RIGHT_GROUPING_PATH: String = "res://assets/tilesets/slope_grass_30deg_4x2.tres"
const LEFT_GROUPING_PATH: String = "res://assets/tilesets/slope_grass_30deg_4x2_left.tres"
const DEFAULT_TERRAIN_PATH: String = "res://default_package/terrain_types.json"
const GENERATED_KEY_PREFIX: String = "slope_grass_30deg_4x2_"
const RIGHT_KEY_SUFFIX: String = "r"
const LEFT_KEY_SUFFIX: String = "l"
# Mirrors the Rust convert-slope reader's format token; a respell must touch both sides.
const REQUEST_VERSION: String = "1.0.0"
# The same runtime config DB `main.gd` boots; it seeds grid W/H from populate_001 on a fresh host.
const CONFIG_DB_PATH: String = "user://data/config/sstd_config.sqlite3"
const CONVERTER_RELATIVE_PATH: String = "../target/debug/import-tiles"
const REQUEST_FILE_NAME: String = "request.json"
const RESULT_FILE_NAME: String = "result.json"
const CLEANED_FILE_NAME: String = "slope_cleaned.png"
# Measured from grid.tile_width_in_pixels / grid.tile_height_in_pixels in the config DB; used only
# when a headless run has no booted Main to expose the bridge.
const FALLBACK_TILE_WIDTH_PX: int = 32
const FALLBACK_TILE_HEIGHT_PX: int = 32
const INTEGER_TERRAIN_FIELDS: Array[String] = [
    "elevation_tiles", "hazard_damage_per_tick", "destructible_hp", "sub_tile_mask"
]


# Resolve the gateway by script path so a headless --script run needs no Main node.
func _load_gateway() -> Object:
    var script: GDScript = load(MODEL_GATEWAY_PATH) as GDScript
    match script:
        null:
            return null
        _:
            return script.new()


# Read the configured tile size through the same bridge Main uses, falling back to the measured
# config defaults when no config DB has been booted in this process.
func _grid_tile_size() -> Array[int]:
    var tile_size: Array[int] = [FALLBACK_TILE_WIDTH_PX, FALLBACK_TILE_HEIGHT_PX]
    var bridge: Object = null
    if ClassDB.class_exists("SstdBridge"):
        bridge = ClassDB.instantiate("SstdBridge")
    if bridge == null or not bridge.has_method("get_grid_config"):
        return tile_size
    # A fresh bridge has no config store and answers with its 64 px code defaults, so point it at
    # the runtime config DB first; this keeps a headless run's tile size deterministic.
    if bridge.has_method("init_config_db"):
        var config_db_path: String = ProjectSettings.globalize_path(CONFIG_DB_PATH)
        var init_decoded: Variant = JSON.parse_string(
            String(bridge.call("init_config_db", config_db_path)))
        var init_ok: bool = init_decoded is Dictionary and bool((init_decoded as Dictionary).get("ok", false))
        if not init_ok:
            printerr("SLOPE-CONFIG init failed path=%s result=%s" % [config_db_path, str(init_decoded)])
    var decoded: Variant = JSON.parse_string(String(bridge.call("get_grid_config")))
    if not decoded is Dictionary:
        return tile_size
    var grid_config: Dictionary = decoded
    tile_size[0] = int(grid_config.get("tile_width_in_pixels", FALLBACK_TILE_WIDTH_PX))
    tile_size[1] = int(grid_config.get("tile_height_in_pixels", FALLBACK_TILE_HEIGHT_PX))
    return tile_size


# Load a grouping only when the same validate() filter the palette applies accepts it.
func _load_grouping(path: String) -> Resource:
    var grouping: Resource = load(path) as Resource
    var valid: bool = grouping != null and grouping.has_method("validate") and bool(grouping.call("validate"))
    match valid:
        true:
            return grouping
        _:
            return null


# Turn one grouping's authored tiles into conversion cells, keyed by their local grid position.
func _cells_from_grouping(grouping: Resource, key_suffix: String) -> Array:
    var width_tiles: int = int(grouping.get("width_tiles"))
    var fn_cell: Callable = func(tile_variant: Variant) -> Dictionary:
        var tile: Dictionary = tile_variant
        var local_x: int = int(tile.get("local_x", 0))
        var local_y: int = int(tile.get("local_y", 0))
        var cell_index: int = local_y * width_tiles + local_x
        return {
            "local_x": local_x,
            "local_y": local_y,
            "source_col": int(tile.get("source_col", 0)),
            "source_row": int(tile.get("source_row", 0)),
            "flip_h": bool(tile.get("flip_h", false)),
            "generated_key": GENERATED_KEY_PREFIX + key_suffix + str(cell_index),
        }
    var tiles_variant: Variant = grouping.get("tiles")
    var authored_tiles: Array = []
    match tiles_variant is Array:
        true:
            authored_tiles = tiles_variant
    return authored_tiles.map(fn_cell)


# Build one request covering both groupings, taking source geometry from the right resource.
func _build_request() -> Dictionary:
    var right_grouping: Resource = _load_grouping(RIGHT_GROUPING_PATH)
    var left_grouping: Resource = _load_grouping(LEFT_GROUPING_PATH)
    if right_grouping == null or left_grouping == null:
        return {}
    var margins: Vector2i = right_grouping.get("src_margins") as Vector2i
    var cell_size: Vector2i = right_grouping.get("src_cell_size") as Vector2i
    var separation: Vector2i = right_grouping.get("src_separation") as Vector2i
    var tile_size: Array[int] = _grid_tile_size()
    var source_path: String = ProjectSettings.globalize_path(String(right_grouping.get("source_image")))
    var request: Dictionary = {
        "version": REQUEST_VERSION,
        "source_image": source_path,
        "margins_px": {"x": margins.x, "y": margins.y},
        "cell_size_px": {"x": cell_size.x, "y": cell_size.y},
        "separation_px": {"x": separation.x, "y": separation.y},
        "width_px": tile_size[0],
        "height_px": tile_size[1],
        "cells": _cells_from_grouping(right_grouping, RIGHT_KEY_SUFFIX)
            + _cells_from_grouping(left_grouping, LEFT_KEY_SUFFIX),
    }
    print("SLOPE-REQUEST source=%s tile=%dx%d cells=%d" % [
        source_path, tile_size[0], tile_size[1], (request["cells"] as Array).size()])
    return request


# Resolve the Rust tool from the repo root; it is a build artifact, not a Godot resource.
func _converter_path() -> String:
    return ProjectSettings.globalize_path("res://").path_join(CONVERTER_RELATIVE_PATH)


# Write the request into the throwaway dir, then run both Rust phases and capture every log line.
func _run_converter(request_text: String, output_dir_path: String, signed_off: bool) -> Dictionary:
    var request_path: String = output_dir_path.path_join(REQUEST_FILE_NAME)
    var request_file: FileAccess = FileAccess.open(request_path, FileAccess.WRITE)
    if request_file == null:
        return {"ok": false, "error": "cannot write %s" % request_path}
    request_file.store_string(request_text)
    request_file.close()
    var args: PackedStringArray = PackedStringArray([
        "convert-slope", request_path, output_dir_path,
    ])
    if signed_off:
        args.append("--signed-off")
    # OS.execute only fills a plain Array; a PackedStringArray is accepted but captures nothing.
    var output_lines: Array = []
    var tool_path: String = _converter_path()
    if not FileAccess.file_exists(tool_path):
        return {"ok": false, "error": "converter binary missing at %s" % tool_path}
    var exit_code: int = OS.execute(tool_path, args, output_lines, true, false)
    print("SLOPE-RUN tool=%s out=%s signed_off=%s exit=%d lines=%d" % [
        tool_path, output_dir_path, str(signed_off), exit_code, output_lines.size()])
    # The failure text comes from the last journal line, or from the exit code when nothing was
    # captured; a success keeps an empty error so two runs serialize byte-identically.
    var error_text: String = ""
    match exit_code:
        0:
            error_text = ""
        _:
            match output_lines.is_empty():
                true:
                    error_text = "exit %d" % exit_code
                false:
                    error_text = String(output_lines[output_lines.size() - 1])
    return {
        "ok": exit_code == 0,
        "error": error_text,
        "lines": output_lines,
    }


# OS.execute hands back each stream as one chunk on this runtime, so flatten the journal before
# reading it; the JSON result sits after many CLEAN and CONVERT progress lines.
func _journal_lines(chunks: Array) -> Array[String]:
    var journal: Array[String] = []
    for chunk_variant in chunks:
        for text_line in String(chunk_variant).split("\n"):
            var trimmed_line: String = text_line.strip_edges()
            if not trimmed_line.is_empty():
                journal.append(trimmed_line)
    return journal


# Pick the JSON result line out of the journal; progress lines are plain text, not JSON objects.
func _decode_result(lines: Array) -> Dictionary:
    for journal_line in _journal_lines(lines):
        if not journal_line.begins_with("{"):
            continue
        var decoded: Variant = JSON.parse_string(journal_line)
        if decoded is Dictionary and (decoded as Dictionary).has("outputs"):
            return decoded
    return {}


# Store one artifact with an explicit failure path so an incomplete run is never published.
func _write_text(path: String, text: String) -> bool:
    var artifact_file: FileAccess = FileAccess.open(path, FileAccess.WRITE)
    if artifact_file == null:
        push_warning("SlopeConversionTool cannot write %s" % path)
        return false
    artifact_file.store_string(text)
    artifact_file.close()
    return true


# Keyed upsert: generated keys replace their own prior value and never append a second entry.
func merge_terrain_entries(existing: Array, generated: Array) -> Array:
    var by_key: Dictionary = {}
    for entry_variant in existing:
        var entry: Dictionary = entry_variant
        by_key[String(entry.get("key", ""))] = entry.duplicate(true)
    for entry_variant in generated:
        var entry: Dictionary = entry_variant
        by_key[String(entry.get("key", ""))] = entry.duplicate(true)
    var sorted_keys: Array[String] = []
    for key_variant in by_key.keys():
        sorted_keys.append(String(key_variant))
    sorted_keys.sort()
    var fn_entry: Callable = func(entry_key: String) -> Dictionary:
        return by_key[entry_key]
    return sorted_keys.map(fn_entry)


# Re-save the terrain catalog with integer numeric fields; serde refuses integral floats.
func canonical_terrain_json(entries: Array) -> String:
    var fn_coerce_entry: Callable = func(entry_variant: Variant) -> Dictionary:
        var entry: Dictionary = entry_variant
        var coerced: Dictionary = entry.duplicate(true)
        for field_name in INTEGER_TERRAIN_FIELDS:
            coerced[field_name] = int(coerced.get(field_name, 0))
        var mask_bytes: Array = coerced.get("solid_mask", [])
        coerced["solid_mask"] = mask_bytes.map(func(byte_variant: Variant) -> int: return int(byte_variant))
        return coerced
    return JSON.stringify(entries.map(fn_coerce_entry), "\t")


# Build a framework-shaped terrain entry for one converted cell; masks and flags stay integers.
func _terrain_entry(output: Dictionary) -> Dictionary:
    var mask_bytes: Array = output.get("solid_mask", [])
    return {
        "key": String(output.get("generated_key", "")),
        "display_name": "30° Grass Slope (generated)",
        "is_walkable": true,
        "is_buildable": true,
        "surface": "normal",
        "hazard": "none",
        "elevation_tiles": 0,
        "color_hex": "#4a7c3f",
        "sub_tile_mask": 15,
        "hazard_damage_per_tick": 0,
        "is_destructible": false,
        "destructible_hp": 0,
        "on_destroy_terrain_key": "",
        "solid_mask": mask_bytes.map(func(byte_variant: Variant) -> int: return int(byte_variant)),
    }


# Full conversion into an output dir. Committed artifacts are untouched here; publishing is a
# separate, explicitly staged step so a refusal cannot leave a half-updated catalog.
func convert_all(output_dir: String) -> Dictionary:
    var output_dir_path: String = ProjectSettings.globalize_path(output_dir)
    var make_error: Error = DirAccess.make_dir_recursive_absolute(output_dir_path)
    if make_error != OK:
        return {"ok": false, "entries": 0, "error": "cannot create %s" % output_dir_path}
    var gateway: Object = _load_gateway()
    if gateway == null or not gateway.has_method("build_slope_conversion_request"):
        return {"ok": false, "entries": 0, "error": "ModelGateway coercion seam unavailable"}
    var request: Dictionary = _build_request()
    if request.is_empty():
        return {"ok": false, "entries": 0, "error": "both slope groupings must load and validate"}
    var request_text: String = String(gateway.call("build_slope_conversion_request", request))
    var conversion: Dictionary = _run_converter(request_text, output_dir_path, true)
    if not bool(conversion.get("ok", false)):
        return {"ok": false, "entries": 0, "error": String(conversion.get("error", ""))}
    var result: Dictionary = _decode_result(conversion.get("lines", []))
    var outputs: Array = result.get("outputs", [])
    if outputs.is_empty():
        return {"ok": false, "entries": 0, "error": "convert-slope emitted no result JSON"}
    var fn_is_resolved: Callable = func(output_variant: Variant) -> bool:
        return bool((output_variant as Dictionary).get("resolved", false))
    if not outputs.all(fn_is_resolved):
        return {"ok": false, "entries": 0, "error": "convert-slope flagged unresolved cells"}
    if not _write_text(output_dir_path.path_join(RESULT_FILE_NAME), JSON.stringify(result, "\t")):
        return {"ok": false, "entries": 0, "error": "cannot write the result artifact"}
    var framework: Variant = JSON.parse_string(FileAccess.get_file_as_string(DEFAULT_TERRAIN_PATH))
    if not framework is Dictionary:
        return {"ok": false, "entries": 0, "error": "framework terrain JSON does not parse"}
    var fn_entry: Callable = func(output_variant: Variant) -> Dictionary:
        return _terrain_entry(output_variant)
    var merged: Array = merge_terrain_entries(
        (framework as Dictionary).get("tiles", []), outputs.map(fn_entry))
    print("SLOPE-CONVERT out=%s cells=%d entries=%d" % [output_dir_path, outputs.size(), merged.size()])
    return {"ok": true, "entries": merged.size(), "error": ""}


# Phase I only: run the same input through Rust without sign-off so Phase II refuses to publish.
func preprocess(source_path: String, output_dir: String) -> Dictionary:
    var request: Dictionary = _build_request()
    if request.is_empty():
        return {"ok": false, "cleaned": "", "flagged": [], "error": "slope groupings unavailable"}
    request["source_image"] = ProjectSettings.globalize_path(source_path)
    var gateway: Object = _load_gateway()
    if gateway == null or not gateway.has_method("build_slope_conversion_request"):
        return {"ok": false, "cleaned": "", "flagged": [], "error": "ModelGateway coercion seam unavailable"}
    var request_text: String = String(gateway.call("build_slope_conversion_request", request))
    var output_dir_path: String = ProjectSettings.globalize_path(output_dir)
    DirAccess.make_dir_recursive_absolute(output_dir_path)
    var conversion: Dictionary = _run_converter(request_text, output_dir_path, false)
    var flagged: Array[String] = []
    for journal_line in _journal_lines(conversion.get("lines", [])):
        if journal_line.begins_with("CLEAN") and journal_line.contains("resolved=false"):
            flagged.append(journal_line)
    var cleaned_path: String = output_dir_path.path_join(CLEANED_FILE_NAME)
    var cleaned: String = ""
    match FileAccess.file_exists(cleaned_path):
        true:
            cleaned = cleaned_path
    print("SLOPE-PREPROCESS source=%s cleaned=%s flagged=%d" % [source_path, cleaned, flagged.size()])
    return {
        "ok": bool(conversion.get("ok", false)),
        "cleaned": cleaned,
        "flagged": flagged,
        "error": String(conversion.get("error", "")),
    }


# Read the committed default when the profile has no terrain file; no import is run for a fresh boot.
func load_default_terrain(profile_dir: String) -> Dictionary:
    var profile_path: String = ProjectSettings.globalize_path(profile_dir).path_join("terrain_types.json")
    var imported: bool = FileAccess.file_exists(profile_path)
    var source_path: String = ""
    match imported:
        true:
            source_path = profile_path
        false:
            source_path = DEFAULT_TERRAIN_PATH
    var decoded: Variant = JSON.parse_string(FileAccess.get_file_as_string(source_path))
    if not decoded is Dictionary:
        push_warning("SlopeConversionTool cannot parse terrain JSON at %s" % source_path)
        return {"imported": imported, "tiles": [], "error": "terrain JSON does not parse at %s" % source_path}
    var entries: Array = (decoded as Dictionary).get("tiles", [])
    print("SLOPE-DEFAULT source=%s imported=%s entries=%d" % [source_path, str(imported), entries.size()])
    return {"imported": imported, "tiles": entries, "error": ""}
