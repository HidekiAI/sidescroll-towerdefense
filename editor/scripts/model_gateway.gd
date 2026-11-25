class_name ModelGateway
extends RefCounted
# Issue #161 (Rust-owned physics and terrain models): the GDScript DAO transform seam.
# Godot Variants become bridge JSON here; integer fields are coerced before stringify because
# serde rejects an integral float for i32/u8. A null snapshot sample stays null until ground_y(),
# where it becomes NAN so every View caller keeps its existing is_nan() guard.

var _bridge: Object = null

# Keep one injected bridge reference; fall back to Main when no caller injected one.
func set_bridge(bridge: Object) -> void:
    _bridge = bridge

# Resolve lazily from Main, following ParallaxLoader's lookup order rather than instantiating a
# second bridge.
func _resolve_bridge() -> Object:
    if _bridge != null and is_instance_valid(_bridge) and _bridge.has_method("build_ground_profile"):
        return _bridge
    var main_loop: MainLoop = Engine.get_main_loop()
    var scene_root: Node = main_loop.root if main_loop != null else null
    var main_node: Node = scene_root.get_node_or_null("Main") if scene_root != null else null
    var candidate: Object = main_node.get("_bridge") if main_node != null else null
    _bridge = candidate if candidate != null and is_instance_valid(candidate) and candidate.has_method("build_ground_profile") else null
    # Headless --script runs have no Main node; instantiate the registered extension so a test
    # never silently receives an empty snapshot instead of the Rust model.
    if _bridge == null and ClassDB.class_exists("SstdBridge"):
        var extension_bridge: Object = ClassDB.instantiate("SstdBridge")
        _bridge = extension_bridge if extension_bridge != null and extension_bridge.has_method("build_ground_profile") else null
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
        return {}
    var request: Dictionary = {"grid": grid, "tiles": _coerce_tiles(tiles)}
    var decoded: Variant = JSON.parse_string(bridge.build_ground_profile(JSON.stringify(request)))
    return decoded if decoded is Dictionary else {}

# Decode the bridge's null sample to NAN at this boundary; solid columns keep their surface Y.
func ground_y(profile: Dictionary, pos_x: float) -> float:
    var bridge: Object = _resolve_bridge()
    if bridge == null:
        return NAN
    var request: Dictionary = {"profile": profile, "pos_x": pos_x}
    var decoded: Variant = JSON.parse_string(bridge.ground_y_at(JSON.stringify(request)))
    var response: Dictionary = decoded if decoded is Dictionary else {}
    var sample_y: Variant = response.get("y", null)
    return NAN if sample_y == null else float(sample_y)

# Route the walker through walk_step and unwrap the single walker field from the response.
func step_walker(state: Dictionary, input: Dictionary, tiles: Array, grid: Dictionary, dt: float) -> Dictionary:
    var bridge: Object = _resolve_bridge()
    if bridge == null:
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
    var request: Dictionary = {
        "grid": grid,
        "input": walker_input,
        "walker": walker_state,
        "tiles": _coerce_tiles(tiles),
        "dt": dt,
    }
    var decoded: Variant = JSON.parse_string(bridge.walk_step(JSON.stringify(request)))
    var response: Dictionary = decoded if decoded is Dictionary else {}
    var walker: Variant = response.get("walker", null)
    return walker if walker is Dictionary else {}

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
        return {}
    var decoded: Variant = JSON.parse_string(bridge.import_terrain_types(JSON.stringify(fn_coerce_file.call(terrain_file))))
    return decoded if decoded is Dictionary else {}
