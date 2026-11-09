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
	# Ground baseline: bottom of the lowest tile + one row, so ramp fills read solid.
	var max_wy: int = 0
	for td in tiles:
		var wy = int(td.get("world_tile_y", 0))
		if wy > max_wy:
			max_wy = wy
	var baseline_y: float = float(max_wy + 1) * th
	var ramp_color := Color(0.4, 0.7, 0.3, 0.8)
	var ramp_dark := Color(0.32, 0.55, 0.22, 0.8)
	var dirt_color := Color(0.55, 0.35, 0.12, 0.8)

	# Flat ground tiles: full rects.
	for td in tiles:
		if not bool(td.get("is_walkable", false)):
			continue
		var terrain: String = String(td.get("terrain", ""))
		if terrain.begins_with("slope_up_"):
			continue
		var wx = int(td.get("world_tile_x", 0))
		var wy = int(td.get("world_tile_y", 0))
		draw_rect(Rect2(Vector2(wx * tw, wy * th), Vector2(tw, th)), dirt_color)

	# Slope runs: one solid polygon per contiguous run, top edge on the 4:7 diagonal.
	var slope_tiles: Array = []
	for td in tiles:
		var terrain: String = String(td.get("terrain", ""))
		if terrain.begins_with("slope_up_") and bool(td.get("is_walkable", false)):
			slope_tiles.append(td)
	if not slope_tiles:
		return
	slope_tiles.sort_custom(func(a, b):
		return int(a.get("world_tile_x", 0)) < int(b.get("world_tile_x", 0))
	)
	var runs: Array = []
	var current: Array = [slope_tiles[0]]
	runs.append(current)
	for i in range(1, slope_tiles.size()):
		var prev_x = int(slope_tiles[i - 1].get("world_tile_x", 0))
		var cur_x = int(slope_tiles[i].get("world_tile_x", 0))
		if cur_x == prev_x + 1:
			current.append(slope_tiles[i])
		else:
			current = [slope_tiles[i]]
			runs.append(current)
	for run in runs:
		var first = run[0]
		var last = run[run.size() - 1]
		var first_x = int(first.get("world_tile_x", 0))
		var last_x = int(last.get("world_tile_x", 0))
		var first_wy = int(first.get("world_tile_y", 0))
		var last_col = int(String(last.get("terrain", "")).substr("slope_up_".length()))
		# Surface at the left edge of the run, and at the right edge of the last
		# column (one step further along the 4:7 rise).
		var surface_left := float(first_wy) * th
		var surface_right := float(max_wy - int(4 * (last_col + 1) / 7)) * th
		var poly := PackedVector2Array([
			Vector2(float(first_x) * tw, surface_left),
			Vector2(float(last_x + 1) * tw, surface_right),
			Vector2(float(last_x + 1) * tw, baseline_y),
			Vector2(float(first_x) * tw, baseline_y),
		])
		var colors := PackedColorArray([ramp_color, ramp_color, ramp_dark, ramp_dark])
		draw_polygon(poly, colors)