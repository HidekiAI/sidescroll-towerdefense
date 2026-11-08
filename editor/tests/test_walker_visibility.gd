extends SceneTree
# In-memory flat world; no local world.zip fixture. Main-thread layout only, no races.
# Repeated entry at two viewport sizes checks layout-dependent framing and safe retries.

const SCREEN_COLS: int = 60
const SCREEN_ROWS: int = 33
const TILE_PX: int = 32
const GROUND_ROW: int = 31
const LAYOUT_FRAMES: int = 4
const TRAVEL_FRAMES: int = 200
const TEST_VIEW_SIZES: Array[Vector2i] = [Vector2i(1152, 617), Vector2i(800, 450)]
var _failures: int = 0

func _initialize() -> void:
    _run.call_deferred()

func _check(condition: bool, description: String) -> void:
    if not condition:
        _failures += 1
        printerr("FAIL: " + description)

func _run() -> void:
    var world_view: Control = load("res://scenes/world_view.tscn").instantiate()
    root.add_child(world_view)
    var store: ScreenStore = ScreenStore.new()
    for screen_col in range(3):
        store.register(screen_col, 0, screen_col + 1, "")
        var tiles: Array[Dictionary] = []
        for local_col in range(SCREEN_COLS):
            tiles.append({"x": local_col, "y": GROUND_ROW, "terrain": "dirt"})
        # Hill at left edge of screen 2 (world_col = 120): 30° up-slope over 4 columns (rise 2 tiles per col)
        if screen_col == 2:
            for i in range(4):
                var local_col = i
                var world_col = 120 + i
                # Hill up: each column rises by 2 rows (30° slope)
                tiles.append({"x": local_col, "y": GROUND_ROW - 2 * (i + 1), "terrain": "hill_up"})
        # Jump-hill on screen 2 (world_col ~135): up then down over 6 columns
        if screen_col == 2:
            for i in range(6):
                var local_col = 15 + i
                var world_col = 135 + i
                if i < 3:
                    # Up slope: 15→17→18 (rows 16, 15, 14)
                    tiles.append({"x": local_col, "y": GROUND_ROW - 16 + 2 * i, "terrain": "hill_up"})
                else:
                    # Down slope: steep descent to below ground
                    tiles.append({"x": local_col, "y": GROUND_ROW + 2 * (i - 3), "terrain": "hill_down"})
        store.cache["%d,0" % screen_col] = {"tiles": tiles}
    world_view.set_grid_config({"max_tiles_per_screen_x": SCREEN_COLS,
        "max_tiles_per_screen_y": SCREEN_ROWS, "tile_width_in_pixels": TILE_PX,
        "tile_height_in_pixels": TILE_PX})
    world_view.set_camera_follow(true)
    world_view.set_screen_store(store)
    var avatar: Node2D = world_view.get("_avatar")
    var viewport: SubViewport = world_view.get("_viewport")
    var front_draw: Node2D = world_view.get_node("SubViewportContainer/Viewport/FrontPlane/ParallaxDraw")
    for view_size in TEST_VIEW_SIZES:
        world_view.size = Vector2(view_size)
        for entry_index in range(2):
            world_view.center_on_screen(1)
            world_view.reset_walker_on_screen(1)
            for frame_index in range(LAYOUT_FRAMES):
                await process_frame
            var canvas_feet: Vector2 = avatar.get_global_transform_with_canvas().origin
            _check(Rect2(Vector2.ZERO, Vector2(viewport.size)).has_point(canvas_feet),
                "entry feet must be inside viewport: %s in %s" % [canvas_feet, viewport.size])
            _check(canvas_feet.y < float(viewport.size.y) and canvas_feet.y > 0.0,
                "ground must be visible after entry")
            _check(not front_draw.visible, "opaque foreground must not cover walker prototype")

    # Movement test: walker starts on screen 2 (left edge hill) and walks right
    world_view.reset_walker_on_screen(3)  # screen_col=2 has screen_id=3

    var start_world_x: float = avatar.position.x
    var start_world_y: float = avatar.position.y
    var min_world_y: float = start_world_y
    var direction_key: InputEventKey = InputEventKey.new()
    direction_key.keycode = KEY_D
    direction_key.pressed = true
    Input.parse_input_event(direction_key)
    for frame_index in range(TRAVEL_FRAMES):
        await process_frame
        min_world_y = mini(min_world_y, avatar.position.y)
    direction_key.pressed = false
    Input.parse_input_event(direction_key)
    _check(avatar.position.x > start_world_x, "D must move the avatar")
    _check(Rect2(Vector2.ZERO, Vector2(viewport.size)).has_point(
        avatar.get_global_transform_with_canvas().origin), "walker must remain visible while moving")
    _check(min_world_y < start_world_y,
        "hill climb: walker reaches a higher surface (min_y=%.1f, start_y=%.1f)" % [
            min_world_y, start_world_y])
    _check(avatar.position.y > start_world_y,
        "fall off hill: walker drops below the starting ground line (y=%.1f, start_y=%.1f)" % [
            avatar.position.y, start_world_y])
    print("walker_visibility failures=%d" % _failures)
    world_view.queue_free()
    await process_frame
    quit(0 if _failures == 0 else 1)
