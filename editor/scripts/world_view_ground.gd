@tool
extends Node2D

var tiles: Array = []
var grid: Dictionary = {}

var _tiles_textures: Dictionary = {}

func _get_tile_texture(key: String) -> Texture2D:
    if key.is_empty():
        return null
    if not _tiles_textures.has(key):
        var path := "res://assets/tiles/%s_32x32.png" % key
        _tiles_textures[key] = load(path)
    return _tiles_textures[key]

func set_data(t: Array, g: Dictionary) -> void:
    tiles = t
    grid = g
    queue_redraw()

func _draw() -> void:
    var tw: float = float(grid.get("tile_w", 64))
    var th: float = float(grid.get("tile_h", 64))
    for td in tiles:
        var isw = bool(td.get("is_walkable", false))
        if not isw:
            continue
        var wx = int(td.get("world_tile_x", 0))
        var wy = int(td.get("world_tile_y", 0))
        var terrain: String = String(td.get("terrain", ""))
        var rect := Rect2(Vector2(wx*tw, wy*th), Vector2(tw,th))
        var tex := _get_tile_texture(terrain)
        if tex != null:
            draw_texture(tex, rect.position)
        else:
            var c = Color(0.4,0.7,0.3,0.6)
            draw_rect(rect, c)