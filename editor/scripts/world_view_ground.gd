@tool
extends Node2D

var tiles: Array = []
var grid: Dictionary = {}

# #126 step 8: cache each terrain key's loaded sprite once, not once per tile per redraw. A null
# entry means the key was tried and has no PNG, so a missing sprite is not re-read every frame.
# There is no fog/atmosphere tint anywhere in this layer: the sprite alpha is AC1's drawn surface.
var _sprite_cache: Dictionary = {}

func set_data(t: Array, g: Dictionary) -> void:
	tiles = t
	grid = g
	queue_redraw()

# #126 step 8: every walkable tile draws its own sprite at its cell. The 4:7 polygon and the flat
# rects are gone; the sprite alpha is the drawn surface AC1 compares feet against. A tile with an
# empty terrain key or a missing PNG is skipped, and no draw call lowers a sprite's alpha.
# Legacy hill_up/hill_down have no committed PNG, so on this sprite path they render blank instead
# of the retired generic rect; the framework config no longer defines them and injected hills use
# slope_up_* art. If a shipped world still needs those keys drawn, that is a separate follow-up.
func _draw() -> void:
	var tw: float = float(grid.get("tile_w", 64))
	var th: float = float(grid.get("tile_h", 64))
	for td in tiles:
		if not bool(td.get("is_walkable", false)):
			continue
		var terrain: String = String(td.get("terrain", ""))
		if terrain == "":
			continue
		var sprite: Texture2D = _sprite_for_key(terrain)
		if sprite == null:
			continue
		var wx: int = int(td.get("world_tile_x", 0))
		var wy: int = int(td.get("world_tile_y", 0))
		draw_texture_rect(sprite, Rect2(Vector2(wx * tw, wy * th), Vector2(tw, th)), false)

func _sprite_for_key(terrain_key: String) -> Texture2D:
	if _sprite_cache.has(terrain_key):
		return _sprite_cache[terrain_key]
	var sprite: Texture2D = null
	var path: String = "res://assets/tiles/%s_32x32.png" % terrain_key
	if ResourceLoader.exists(path):
		sprite = load(path) as Texture2D
	_sprite_cache[terrain_key] = sprite
	return sprite
