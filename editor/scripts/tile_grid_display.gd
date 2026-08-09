extends Control

var map_editor: Control
var tile_size: int = 32
var grid_w: int = 60
var grid_h: int = 33
var show_collision: bool = false
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

func _tile_texture(key: String) -> Texture2D:
    if _texture_cache.has(key):
        return _texture_cache[key]
    if map_editor and map_editor.has_method("get_tile_texture"):
        var editor_tex: Texture2D = map_editor.get_tile_texture(key)
        if editor_tex:
            _texture_cache[key] = editor_tex
            return editor_tex
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
            var rect := Rect2(x * tile_size, y * tile_size, tile_size, tile_size)
            if not key.is_empty():
                var td: Dictionary = map_editor.get_tile_data(x, y)
                var flip_h := bool(td.get("flip_h", false))
                var flip_v := bool(td.get("flip_v", false))
                var use_flip := flip_h or flip_v
                var drawn := false
                var tex: Texture2D = _tile_texture(key)
                if tex:
                    if use_flip:
                        var sx := -1.0 if flip_h else 1.0
                        var sy := -1.0 if flip_v else 1.0
                        var ox := rect.position.x + (tile_size if flip_h else 0)
                        var oy := rect.position.y + (tile_size if flip_v else 0)
                        draw_set_transform(Vector2(ox, oy), 0.0, Vector2(sx, sy))
                        draw_texture_rect(tex, Rect2(0, 0, tile_size, tile_size), false)
                        draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
                    else:
                        draw_texture_rect(tex, rect, false)
                    drawn = true
                if not drawn and td.has("source_image") and td.has("source_rect"):
                    var sr: Dictionary = td["source_rect"]
                    var sw := int(sr.get("w", 0))
                    var sh := int(sr.get("h", 0))
                    if sw > 0 and sh > 0:
                        var sheet_tex: Texture2D = _sprite_sheet_texture(td["source_image"])
                        if sheet_tex:
                            var src := Rect2(sr["x"], sr["y"], sw, sh)
                            if use_flip:
                                var sx := -1.0 if flip_h else 1.0
                                var sy := -1.0 if flip_v else 1.0
                                var ox := rect.position.x + (tile_size if flip_h else 0)
                                var oy := rect.position.y + (tile_size if flip_v else 0)
                                draw_set_transform(Vector2(ox, oy), 0.0, Vector2(sx, sy))
                                draw_texture_rect_region(sheet_tex, Rect2(0, 0, tile_size, tile_size), src)
                                draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
                            else:
                                draw_texture_rect_region(sheet_tex, rect, src)
                            drawn = true
                if not drawn:
                    draw_rect(rect, map_editor.terrain_color(key))
            draw_rect(rect, Color(0.2, 0.2, 0.2, 0.3), false, 1)

    if show_collision:
        _draw_collision_overlay()

    var mouse := get_local_mouse_position()
    var tile := pixel_to_tile(mouse)
    if tile.x >= 0 and tile.x < grid_w and tile.y >= 0 and tile.y < grid_h:
        var highlight := Rect2(tile.x * tile_size, tile.y * tile_size, tile_size, tile_size)
        draw_rect(highlight, Color(1, 1, 1, 0.25), true)
        draw_rect(highlight, Color.WHITE, false, 2)

    if map_editor and map_editor.get("_stamp_active"):
        _draw_stamp_preview(tile)

func _draw_stamp_preview(current_tile: Vector2i) -> void:
    var tex: ImageTexture = map_editor.get("_stamp_texture")
    if not tex:
        return
    var origin: Vector2i = map_editor.get("_stamp_origin")
    var tile_pos: Vector2i = origin if origin.x >= 0 else current_tile
    var tex_size := tex.get_size()
    var w_cells := ceili(float(tex_size.x) / map_editor.STAMP_CELL)
    var h_cells := ceili(float(tex_size.y) / map_editor.STAMP_CELL)
    var rect := Rect2(tile_pos.x * tile_size, tile_pos.y * tile_size, w_cells * tile_size, h_cells * tile_size)
    draw_texture_rect(tex, rect, false, Color(1, 1, 1, 0.55))
    draw_rect(rect, Color(1, 1, 0, 1), false, 1)

func _draw_collision_overlay() -> void:
    var hw := tile_size / 2
    var hh := tile_size / 2
    for y in grid_h:
        for x in grid_w:
            var td: Dictionary = map_editor.get_tile_data(x, y)
            if not td.has("sub_tile_mask"):
                continue
            var mask: int = int(td["sub_tile_mask"])
            for qy in 2:
                for qx in 2:
                    var bit := qy * 2 + qx
                    var solid := (mask >> bit) & 1 == 1
                    var qrect := Rect2(
                        x * tile_size + qx * hw,
                        y * tile_size + qy * hh,
                        hw, hh
                    )
                    draw_rect(qrect, Color(0, 1, 0, 0.5) if solid else Color(1, 0, 0, 0.5), true, 0)

func _gui_input(event: InputEvent) -> void:
    if map_editor and map_editor.get("_stamp_active"):
        map_editor._stamp_grid_input(event)
        return
    if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
        var tile := pixel_to_tile(event.position)
        if map_editor:
            map_editor._paint_tile(tile.x, tile.y)
    elif event is InputEventMouseMotion and event.button_mask & MOUSE_BUTTON_MASK_LEFT:
        var tile := pixel_to_tile(event.position)
        if map_editor:
            map_editor._paint_tile(tile.x, tile.y)
