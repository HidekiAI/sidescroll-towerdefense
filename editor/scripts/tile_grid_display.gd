extends Control

var map_editor: Control
var tile_size: int = 64
var grid_w: int = 30
var grid_h: int = 16

func set_grid_config(cfg: Dictionary) -> void:
    tile_size = cfg.get("tile_width_in_pixels", 64)
    grid_w = cfg.get("max_tiles_per_screen_x", 30)
    grid_h = cfg.get("max_tiles_per_screen_y", 16)
    queue_redraw()

func pixel_to_tile(pos: Vector2) -> Vector2i:
    return Vector2i(
        int(pos.x / tile_size),
        int(pos.y / tile_size)
    )

func _draw() -> void:
    if not map_editor:
        return

    for y in grid_h:
        for x in grid_w:
            var key: String = map_editor.get_tile(x, y)
            var color: Color = map_editor.terrain_color(key)
            var rect := Rect2(x * tile_size, y * tile_size, tile_size, tile_size)
            draw_rect(rect, color)
            draw_rect(rect, Color(0.2, 0.2, 0.2, 0.3), false, 1)

    var mouse := get_local_mouse_position()
    var tile := pixel_to_tile(mouse)
    if tile.x >= 0 and tile.x < grid_w and tile.y >= 0 and tile.y < grid_h:
        var highlight := Rect2(tile.x * tile_size, tile.y * tile_size, tile_size, tile_size)
        draw_rect(highlight, Color(1, 1, 1, 0.25), true)
        draw_rect(highlight, Color.WHITE, false, 2)

func _gui_input(event: InputEvent) -> void:
    if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
        var tile := pixel_to_tile(event.position)
        if map_editor:
            map_editor._paint_tile(tile.x, tile.y)
    elif event is InputEventMouseMotion and event.button_mask & MOUSE_BUTTON_MASK_LEFT:
        var tile := pixel_to_tile(event.position)
        if map_editor:
            map_editor._paint_tile(tile.x, tile.y)
