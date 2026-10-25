@tool
extends Node2D

var tiles: Array = []
var grid: Dictionary = {}

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
        var c = Color(0.4,0.7,0.3,0.6) if isw else Color(0.2,0.2,0.2,0.4)
        draw_rect(Rect2(Vector2(wx*tw, wy*th), Vector2(tw,th)), c)
