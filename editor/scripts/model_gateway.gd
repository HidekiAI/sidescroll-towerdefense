class_name ModelGateway
extends RefCounted
# Rust-owned physics and terrain models: the GDScript DAO transform seam.
# Godot Variants become bridge JSON here; integer fields are coerced before stringify because
# serde rejects an integral float for i32/u8. A null snapshot sample stays null until ground_y(),
# where it becomes NAN so every View caller keeps its existing is_nan() guard.

var _bridge: Object = null

# Keep the exact failed request in the engine log so an incident can be replayed from it.
func _log_bridge_failure(method_name: String, request_text: String, reason: String) -> void:
    push_warning("ModelGateway %s failed: %s; request=%s" % [method_name, reason, request_text])

# Keep one injected bridge reference; fall back to Main when no caller injected one.
func set_bridge(bridge: Object) -> void:
    _bridge = bridge

# Resolve lazily from Main, following ParallaxLoader's lookup order rather than instantiating a
# second bridge.
func _resolve_bridge() -> Object:
    if _bridge != null and is_instance_valid(_bridge) and _bridge.has_method("build_ground_profile"):
        return _bridge
    var main_loop: MainLoop = Engine.get_main_loop()
    # Walk the Main lookup one guard at a time; each fallback is the initializer's null.
    var scene_root: Node = null
    match main_loop != null:
        true:
            scene_root = main_loop.root
    var main_node: Node = null
    match scene_root != null:
        true:
            main_node = scene_root.get_node_or_null("Main")
    var candidate: Object = null
    match main_node != null:
        true:
            candidate = main_node.get("_bridge")
    var resolved_bridge: Object = null
    match candidate != null and is_instance_valid(candidate) and candidate.has_method("build_ground_profile"):
        true:
            resolved_bridge = candidate
    _bridge = resolved_bridge
    # Headless --script runs have no Main node; instantiate the registered extension so a test
    # never silently receives an empty snapshot instead of the Rust model.
    if _bridge == null and ClassDB.class_exists("SstdBridge"):
        var extension_bridge: Object = ClassDB.instantiate("SstdBridge")
        var valid_extension: Object = null
        match extension_bridge != null and extension_bridge.has_method("build_ground_profile"):
            true:
                valid_extension = extension_bridge
        _bridge = valid_extension
    return _bridge

# Coerce tile integers at the wire boundary; the grid stays float because the bridge declares f64.
func _coerce_tiles(tiles: Array) -> Array:
    var fn_coerce_tile: Callable = func(tile: Dictionary) -> Dictionary:
        var mask_bytes: Array = tile.get("solid_mask", [])
        return {
            "world_tile_x": int(tile.get("world_tile_x", 0)),
            "world_tile_y": int(tile.get("world_tile_y", 0)),
            "is_walkable": bool(tile.get("is_walkable", false)),
            "solid_mask": mask_bytes.map(func(byte_variant: Variant) -> int: return int(byte_variant)),
        }
    return tiles.map(fn_coerce_tile)

# Build the snapshot request, call the bridge, and decode without changing air columns.
func build_profile(tiles: Array, grid: Dictionary) -> Dictionary:
    var bridge: Object = _resolve_bridge()
    if bridge == null:
        _log_bridge_failure("build_ground_profile", "", "bridge unavailable")
        return {}
    var request_text: String = JSON.stringify({"grid": grid, "tiles": _coerce_tiles(tiles)})
    var decoded: Variant = JSON.parse_string(bridge.build_ground_profile(request_text))
    if not decoded is Dictionary:
        _log_bridge_failure("build_ground_profile", request_text, "response is not a JSON object")
    var profile: Dictionary = {}
    match decoded is Dictionary:
        true:
            profile = decoded
    return profile

# Decode the bridge's null sample to NAN at this boundary; solid columns keep their surface Y.
func ground_y(profile: Dictionary, pos_x: float) -> float:
    var bridge: Object = _resolve_bridge()
    if bridge == null:
        _log_bridge_failure("ground_y_at", "", "bridge unavailable")
        return NAN
    var request_text: String = JSON.stringify({"profile": profile, "pos_x": pos_x})
    var decoded: Variant = JSON.parse_string(bridge.ground_y_at(request_text))
    if not decoded is Dictionary:
        _log_bridge_failure("ground_y_at", request_text, "response is not a JSON object")
    var response: Dictionary = {}
    match decoded is Dictionary:
        true:
            response = decoded
    var sample_y: Variant = response.get("y", null)
    var surface_y: float = NAN
    match sample_y == null:
        false:
            surface_y = float(sample_y)
    return surface_y

# Route the walker through walk_step and unwrap the single walker field from the response.
func step_walker(state: Dictionary, input: Dictionary, tiles: Array, grid: Dictionary, dt: float) -> Dictionary:
    var bridge: Object = _resolve_bridge()
    if bridge == null:
        _log_bridge_failure("walk_step", "", "bridge unavailable")
        return {}
    var walker_input: Dictionary = {
        "move_x": float(input.get("move_x", 0.0)),
        "move_y": float(input.get("move_y", 0.0)),
        "move_to_world": input.get("move_to_world", null),
        "scroll_vec": input.get("scroll_vec", null),
        "place": bool(input.get("place", false)),
        "alt_place": bool(input.get("alt_place", false)),
        "interact": bool(input.get("interact", false)),
        "zoom_in": bool(input.get("zoom_in", false)),
        "zoom_out": bool(input.get("zoom_out", false)),
    }
    var walker_state: Dictionary = {
        "pos_x": float(state.get("pos_x", 0.0)),
        "pos_y": float(state.get("pos_y", 0.0)),
        "vel_x": float(state.get("vel_x", 0.0)),
        "vel_y": float(state.get("vel_y", 0.0)),
        "grounded": bool(state.get("grounded", false)),
        "hp": float(state.get("hp", 0.0)),
        "fall_start_y": float(state.get("fall_start_y", state.get("pos_y", 0.0))),
    }
    var request_text: String = JSON.stringify({
        "grid": grid,
        "input": walker_input,
        "walker": walker_state,
        "tiles": _coerce_tiles(tiles),
        "dt": dt,
    })
    var decoded: Variant = JSON.parse_string(bridge.walk_step(request_text))
    if not decoded is Dictionary:
        _log_bridge_failure("walk_step", request_text, "response is not a JSON object")
    var response: Dictionary = {}
    match decoded is Dictionary:
        true:
            response = decoded
    var walker: Variant = response.get("walker", null)
    var walker_result: Dictionary = {}
    match walker is Dictionary:
        true:
            walker_result = walker
    return walker_result

# Coerce terrain integers before stringify; the bridge's TerrainTypeDef rejects float i32/u8.
func import_terrain_types(terrain_file: Dictionary) -> Dictionary:
    var fn_coerce_file: Callable = func(file: Dictionary) -> Dictionary:
        var fn_coerce_terrain: Callable = func(terrain: Dictionary) -> Dictionary:
            var coerced: Dictionary = terrain.duplicate()
            for field_name in ["elevation_tiles", "hazard_damage_per_tick", "destructible_hp", "sub_tile_mask"]:
                coerced[field_name] = int(coerced.get(field_name, 0))
            var mask_bytes: Array = coerced.get("solid_mask", [])
            coerced["solid_mask"] = mask_bytes.map(func(byte_variant: Variant) -> int: return int(byte_variant))
            return coerced
        var coerced: Dictionary = file.duplicate()
        var tiles: Array = coerced.get("tiles", [])
        coerced["tiles"] = tiles.map(fn_coerce_terrain)
        return coerced
    var bridge: Object = _resolve_bridge()
    if bridge == null:
        _log_bridge_failure("import_terrain_types", "", "bridge unavailable")
        return {}
    var request_text: String = JSON.stringify(fn_coerce_file.call(terrain_file))
    var decoded: Variant = JSON.parse_string(bridge.import_terrain_types(request_text))
    if not decoded is Dictionary:
        _log_bridge_failure("import_terrain_types", request_text, "response is not a JSON object")
    var imported: Dictionary = {}
    match decoded is Dictionary:
        true:
            imported = decoded
    return imported

# Coerce every integer field of a slope conversion request before stringify; serde rejects an
# integral float for i32/u32, and one uncoerced cell index refuses the whole conversion.
func build_slope_conversion_request(request: Dictionary) -> String:
    # Point geometry comes from the grouping's pixel-space fields, not from a coordinate name.
    var fn_coerce_point: Callable = func(point: Dictionary) -> Dictionary:
        return {
            "x": int(point.get("x", 0)),
            "y": int(point.get("y", 0)),
        }
    # Cells keep their authored order; only typed indices and the flip flag are rewritten.
    var fn_coerce_cell: Callable = func(cell: Dictionary) -> Dictionary:
        var coerced: Dictionary = cell.duplicate()
        for field_name in ["local_x", "local_y", "source_col", "source_row"]:
            coerced[field_name] = int(coerced.get(field_name, 0))
        coerced["flip_h"] = bool(coerced.get("flip_h", false))
        coerced["generated_key"] = String(coerced.get("generated_key", ""))
        return coerced
    var bridged_request: Dictionary = request.duplicate()
    for field_name in ["width_px", "height_px"]:
        bridged_request[field_name] = int(bridged_request.get(field_name, 0))
    for point_field in ["margins_px", "cell_size_px", "separation_px"]:
        bridged_request[point_field] = fn_coerce_point.call(bridged_request.get(point_field, {}))
    var cells: Array = bridged_request.get("cells", [])
    bridged_request["cells"] = cells.map(fn_coerce_cell)
    return JSON.stringify(bridged_request)
