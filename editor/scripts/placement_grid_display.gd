extends Control

var placement_editor: Control
var tile_size: int = 64
var grid_w: int = 30
var grid_h: int = 16

func set_grid_config(cfg: Dictionary) -> void:
    tile_size = cfg.get("tile_width_in_pixels", 64)
    grid_w = cfg.get("max_tiles_per_screen_x", 30)
    grid_h = cfg.get("max_tiles_per_screen_y", 16)
    queue_redraw()

func pixel_to_tile(pos: Vector2) -> Vector2i:
    return Vector2i(int(pos.x / tile_size), int(pos.y / tile_size))

func _draw() -> void:
    if not placement_editor:
        return

    for y in grid_h:
        for x in grid_w:
            var key: String = placement_editor.get_tile(x, y)
            var color: Color = placement_editor.terrain_color(key)
            var rect := Rect2(x * tile_size, y * tile_size, tile_size, tile_size)
            draw_rect(rect, color)
            draw_rect(rect, Color(0.2, 0.2, 0.2, 0.3), false, 1)

    for e in placement_editor.get_placements():
        var ex: float = e["tile_x"] * tile_size
        var ey: float = e["tile_y"] * tile_size
        var ew: float = e.get("width", 1.0) * tile_size
        var eh: float = e.get("height", 1.0) * tile_size
        var ecolor: Color = e.get("color", Color(1, 1, 1, 0.6))
        draw_rect(Rect2(ex, ey, ew, eh), ecolor)
        draw_rect(Rect2(ex, ey, ew, eh), Color.WHITE, false, 2)
        if e.has("key"):
            draw_string(ThemeDB.fallback_font, Vector2(ex + 2, ey + 12), e["key"], HORIZONTAL_ALIGNMENT_LEFT, -1, 12, Color.WHITE)

    var mouse := get_local_mouse_position()
    var tile := pixel_to_tile(mouse)
    if tile.x >= 0 and tile.x < grid_w and tile.y >= 0 and tile.y < grid_h:
        var highlight := Rect2(tile.x * tile_size, tile.y * tile_size, tile_size, tile_size)
        draw_rect(highlight, Color(1, 1, 1, 0.15), true)
        draw_rect(highlight, Color.YELLOW, false, 2)

func _gui_input(event: InputEvent) -> void:
    if event is InputEventMouseButton and event.pressed:
        var tile := pixel_to_tile(event.position)
        if event.button_index == MOUSE_BUTTON_LEFT:
            placement_editor.place_at(tile.x, tile.y)
        elif event.button_index == MOUSE_BUTTON_RIGHT:
            placement_editor.remove_at(tile.x, tile.y)
