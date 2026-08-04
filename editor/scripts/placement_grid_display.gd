extends Control

var placement_editor: Control
var tile_size: int = 32
var grid_w: int = 60
var grid_h: int = 33
var _texture_cache: Dictionary = {}

func set_grid_config(cfg: Dictionary) -> void:
    tile_size = cfg.get("tile_width_in_pixels", 32)
    grid_w = cfg.get("max_tiles_per_screen_x", 60)
    grid_h = cfg.get("max_tiles_per_screen_y", 33)
    _texture_cache.clear()
    queue_redraw()

func reload_textures() -> void:
    _texture_cache.clear()
    queue_redraw()

func _sprite_sheet_texture(path: String) -> Texture2D:
    var cache_key := "sheet_" + path
    if _texture_cache.has(cache_key):
        return _texture_cache[cache_key]
    var abs := ProjectSettings.globalize_path(path)
    if FileAccess.file_exists(abs):
        var img: Image = Image.new()
        if img.load(abs) == OK:
            var tex: ImageTexture = ImageTexture.create_from_image(img)
            _texture_cache[cache_key] = tex
            return tex
    _texture_cache[cache_key] = null
    return null

func _entity_texture(key: String) -> Texture2D:
    if _texture_cache.has("ent_" + key):
        return _texture_cache["ent_" + key]
    var path := "res://assets/entities/%s_32x32.png" % key
    var abs := ProjectSettings.globalize_path(path)
    if FileAccess.file_exists(abs):
        var img: Image = Image.new()
        if img.load(abs) == OK:
            var tex: ImageTexture = ImageTexture.create_from_image(img)
            _texture_cache["ent_" + key] = tex
            return tex
    _texture_cache["ent_" + key] = null
    return null

func _tile_texture(key: String) -> Texture2D:
    if _texture_cache.has(key):
        return _texture_cache[key]
    var path := "res://assets/tiles/%s_%dx%d.png" % [key, tile_size, tile_size]
    var abs := ProjectSettings.globalize_path(path)
    if FileAccess.file_exists(abs):
        var img: Image = Image.new()
        if img.load(abs) == OK:
            var tex: ImageTexture = ImageTexture.create_from_image(img)
            _texture_cache[key] = tex
            return tex
    _texture_cache[key] = null
    return null

func pixel_to_tile(pos: Vector2) -> Vector2i:
    return Vector2i(int(pos.x / tile_size), int(pos.y / tile_size))

func _draw() -> void:
    if not placement_editor:
        return

    for y in grid_h:
        for x in grid_w:
            var key: String = placement_editor.get_tile(x, y)
            var rect := Rect2(x * tile_size, y * tile_size, tile_size, tile_size)
            if not key.is_empty():
                var td: Dictionary = placement_editor.get_tile_data(x, y)
                var flip_h := bool(td.get("flip_h", false))
                var drawn := false
                if td.has("source_image") and td.has("source_rect"):
                    var sr: Dictionary = td["source_rect"]
                    var sheet_tex: Texture2D = _sprite_sheet_texture(td["source_image"])
                    if sheet_tex:
                        var src := Rect2(sr["x"], sr["y"], sr["w"], sr["h"])
                        if flip_h:
                            draw_set_transform(Vector2(rect.position.x + tile_size, rect.position.y), 0.0, Vector2(-1, 1))
                            draw_texture_rect_region(sheet_tex, Rect2(0, 0, tile_size, tile_size), src)
                            draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
                        else:
                            draw_texture_rect_region(sheet_tex, rect, src)
                        drawn = true
                if not drawn:
                    var tex: Texture2D = _tile_texture(key)
                    if tex:
                        if flip_h:
                            draw_set_transform(Vector2(rect.position.x + tile_size, rect.position.y), 0.0, Vector2(-1, 1))
                            draw_texture_rect(tex, Rect2(0, 0, tile_size, tile_size), false)
                            draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
                        else:
                            draw_texture_rect(tex, rect, false)
                    else:
                        draw_rect(rect, placement_editor.terrain_color(key))
            draw_rect(rect, Color(0.2, 0.2, 0.2, 0.3), false, 1)

    for e in placement_editor.get_placements():
        var ex: float = e["tile_x"] * tile_size
        var eh: float = e.get("height", tile_size)
        var ey: float = (e["tile_y"] + 1) * tile_size - eh
        var ew: float = e.get("width", tile_size)
        var ecolor: Color = e.get("color", Color(1, 1, 1, 0.6))
        var entity_tex: Texture2D = _entity_texture(e.get("key", "")) if e.has("key") else null
        if entity_tex:
            draw_texture_rect(entity_tex, Rect2(ex, ey, ew, eh), false)
        else:
            var overlay := Color(ecolor.r, ecolor.g, ecolor.b, 0.3)
            draw_rect(Rect2(ex, ey, ew, eh), overlay)
        draw_rect(Rect2(ex, ey, ew, eh), Color.WHITE, false, 2)
        if e.has("key"):
            draw_string(ThemeDB.fallback_font, Vector2(ex + 2, ey + 12), e["key"], HORIZONTAL_ALIGNMENT_LEFT, -1, 12, Color.WHITE)

    var mouse := get_local_mouse_position()
    var tile := pixel_to_tile(mouse)
    if tile.x >= 0 and tile.x < grid_w and tile.y >= 0 and tile.y < grid_h:
        var fp: Dictionary = placement_editor.get_selected_footprint()
        if fp["w_tiles"] > 0 and fp["h_tiles"] > 0:
            var hw: float = ceil(fp["w_tiles"]) * tile_size
            var hh: float = ceil(fp["h_tiles"]) * tile_size
            var hx: float = tile.x * tile_size
            var hy: float = tile.y * tile_size
            if not fp.get("anchor_top", false):
                hy = (tile.y - ceil(fp["h_tiles"]) + 1) * tile_size
            draw_rect(Rect2(hx, hy, hw, hh), Color(1, 1, 0, 0.15), true)
            draw_rect(Rect2(hx, hy, hw, hh), Color.YELLOW, false, 2)
        else:
            draw_rect(Rect2(tile.x * tile_size, tile.y * tile_size, tile_size, tile_size), Color(1, 1, 0, 0.15), true)
            draw_rect(Rect2(tile.x * tile_size, tile.y * tile_size, tile_size, tile_size), Color.YELLOW, false, 2)

func _gui_input(event: InputEvent) -> void:
    if event is InputEventMouseButton and event.pressed:
        var tile := pixel_to_tile(event.position)
        if event.button_index == MOUSE_BUTTON_LEFT:
            placement_editor.place_at(tile.x, tile.y)
        elif event.button_index == MOUSE_BUTTON_RIGHT:
            placement_editor.remove_at(tile.x, tile.y)
