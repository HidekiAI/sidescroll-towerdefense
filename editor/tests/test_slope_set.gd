extends SceneTree
# Guard for issue #122's 4:7 slope set: what `inject_simulator_hills` emits, that a second call
# does not clobber it, and the walkable surface `world_view` derives from it.
#
# NOT covered: the ramp polygon's pixels. `world_view_ground.gd` builds them inside `_draw()` with
# `draw_polygon`, which cannot be intercepted from GDScript; covering that would mean reshaping
# production code around the test. The surface asserted here is the same table the polygon's top
# edge is drawn from, so a wrong slope still shows up as a wrong surface.
#
# No race is possible here: every input is single-threaded scene-tree data. Back-to-back safety IS
# covered, by asserting the second injection is a no-op rather than a partial overwrite.

const SCREEN_COLS: int = 60
const SCREEN_ROWS: int = 33
const TILE_PX: int = 32
const GROUND_ROW: int = SCREEN_ROWS - 2
const SLOPE_RUN_COLUMNS: int = 7
# Row offset below ground for local column 0..6 of the set: int(4 * col / 7).
const SLOPE_ROW_OFFSETS: Array[int] = [0, 0, 1, 1, 2, 2, 3]
# Screen 2 sits at position (1,0), so its local column 0 is world column 60.
const SCREEN_2_WORLD_COL_OFFSET: int = SCREEN_COLS

var _failures: int = 0

func _initialize() -> void:
    _run.call_deferred()

func _check(condition: bool, description: String) -> void:
    if not condition:
        _failures += 1
        printerr("FAIL: " + description)

func _make_grid_config() -> Dictionary:
    return {
        "max_tiles_per_screen_x": SCREEN_COLS,
        "max_tiles_per_screen_y": SCREEN_ROWS,
        "tile_width_in_pixels": TILE_PX,
        "tile_height_in_pixels": TILE_PX,
    }

# main.gd never enters the tree here, so `_ready` does not run: the injection only needs
# `_grid_config` and the store it is handed.
func _make_main_instance() -> Control:
    var main_script: GDScript = load("res://scripts/main.gd")
    var main_instance: Control = main_script.new()
    main_instance.set("_grid_config", _make_grid_config())
    return main_instance

func _make_injected_store() -> ScreenStore:
    var store: ScreenStore = ScreenStore.new()
    _make_main_instance().call("inject_simulator_hills", store)
    return store

func _tile_at_local_col(tiles: Array, local_col: int) -> Dictionary:
    for tile in tiles:
        if int(tile.get("x", -1)) == local_col:
            return tile
    return {}

func _tiles_of(store: ScreenStore, key: String) -> Array:
    var cached: Dictionary = store.cache.get(key, {})
    return cached.get("tiles", [])

# The set starts at `first_local_col` and runs 7 columns; the column after it is flat ground.
func _check_slope_run(store: ScreenStore, key: String, first_local_col: int) -> void:
    var tiles: Array = _tiles_of(store, key)
    _check(tiles.size() == SCREEN_COLS,
        "%s: expected %d tiles, got %d" % [key, SCREEN_COLS, tiles.size()])
    for column_offset in range(SLOPE_RUN_COLUMNS):
        var local_col: int = first_local_col + column_offset
        var tile: Dictionary = _tile_at_local_col(tiles, local_col)
        _check(String(tile.get("terrain", "")) == "slope_up_" + str(column_offset),
            "%s col %d: terrain %s, expected slope_up_%d" % [
                key, local_col, tile.get("terrain", "<missing>"), column_offset])
        var expected_row: int = GROUND_ROW - SLOPE_ROW_OFFSETS[column_offset]
        _check(int(tile.get("y", -1)) == expected_row,
            "%s col %d: row %d, expected %d" % [
                key, local_col, int(tile.get("y", -1)), expected_row])

func _check_flat_columns(store: ScreenStore, key: String, local_cols: Array[int]) -> void:
    var tiles: Array = _tiles_of(store, key)
    for local_col in local_cols:
        var tile: Dictionary = _tile_at_local_col(tiles, local_col)
        _check(String(tile.get("terrain", "")) == "dirt",
            "%s col %d: terrain %s, expected dirt" % [key, local_col, tile.get("terrain", "<missing>")])
        _check(int(tile.get("y", -1)) == GROUND_ROW,
            "%s col %d: row %d, expected ground row %d" % [
                key, local_col, int(tile.get("y", -1)), GROUND_ROW])

func _check_injection() -> void:
    var store: ScreenStore = _make_injected_store()
    _check_slope_run(store, "1,0", 0)
    _check_slope_run(store, "2,0", 14)
    # Both sides of each run, plus one column inside the run's own screen but outside the set.
    _check_flat_columns(store, "1,0", [SLOPE_RUN_COLUMNS])
    _check_flat_columns(store, "2,0", [13, 21])

# Second call must leave an already-populated world untouched, not clobber or half-apply it.
func _check_repeat_is_noop() -> void:
    var store: ScreenStore = _make_injected_store()
    var tiles: Array = _tiles_of(store, "1,0")
    var sentinel_tile: Dictionary = _tile_at_local_col(tiles, 0)
    sentinel_tile["terrain"] = "dirt"
    sentinel_tile["y"] = GROUND_ROW
    _make_main_instance().call("inject_simulator_hills", store)
    var after_repeat: Dictionary = _tile_at_local_col(_tiles_of(store, "1,0"), 0)
    _check(String(after_repeat.get("terrain", "")) == "dirt",
        "repeat injection must not restore slope_up_0 over an occupied screen")
    _check(int(after_repeat.get("y", -1)) == GROUND_ROW,
        "repeat injection must not move the sentinel row back to the slope")

func _check_ground_profile() -> void:
    var store: ScreenStore = _make_injected_store()
    var world_view: Control = load("res://scenes/world_view.tscn").instantiate()
    root.add_child(world_view)
    world_view.set_grid_config(_make_grid_config())
    world_view.set_screen_store(store)
    var ground_y_by_col: Dictionary = world_view.get("_ground_y_by_col")
    for column_offset in range(SLOPE_RUN_COLUMNS):
        var world_col: int = SCREEN_2_WORLD_COL_OFFSET + column_offset
        var expected_y_px: float = float(GROUND_ROW - SLOPE_ROW_OFFSETS[column_offset]) * TILE_PX
        var actual_y_px: float = float(ground_y_by_col.get(world_col, -1.0))
        _check(actual_y_px == expected_y_px,
            "world col %d: surface %.1f px, expected %.1f px" % [
                world_col, actual_y_px, expected_y_px])
    # The column after the set is flat, which is the "flush to ground at the right edge" claim.
    var flat_y_px: float = float(ground_y_by_col.get(
        SCREEN_2_WORLD_COL_OFFSET + SLOPE_RUN_COLUMNS, -1.0))
    _check(flat_y_px == float(GROUND_ROW) * TILE_PX,
        "world col %d after the set: surface %.1f px, expected ground %.1f px" % [
            SCREEN_2_WORLD_COL_OFFSET + SLOPE_RUN_COLUMNS, flat_y_px, float(GROUND_ROW) * TILE_PX])
    world_view.queue_free()
    await process_frame

func _run() -> void:
    _check_injection()
    _check_repeat_is_noop()
    await _check_ground_profile()
    print("slope_set failures=%d" % _failures)
    quit(0 if _failures == 0 else 1)
