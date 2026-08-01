class_name ScreenMinimap extends Control

signal position_picked(x: int, y: int)

var store: ScreenStore
var mode: String = "pick"
var source: Vector2i = Vector2i(-1, -1)
var hover: Vector2i = Vector2i(-1, -1)

const CELL_SIZE := 26
const GAP := 2
const PAD := 8

func _ready() -> void:
	custom_minimum_size = Vector2(320, 180)
	mouse_filter = MOUSE_FILTER_STOP
	queue_redraw()

func setup(s: ScreenStore, m: String, src: Vector2i = Vector2i(-1, -1)) -> void:
	store = s
	mode = m
	source = src
	queue_redraw()

func _get_rect(x: int, y: int, bounds: Rect2i) -> Rect2:
	var px := PAD + (x - bounds.position.x) * (CELL_SIZE + GAP)
	var py := PAD + (y - bounds.position.y) * (CELL_SIZE + GAP)
	return Rect2(px, py, CELL_SIZE, CELL_SIZE)

func _draw() -> void:
	if not store:
		return
	var bounds := store.bounds()
	var width := bounds.size.x * (CELL_SIZE + GAP) - GAP + PAD * 2
	var height := bounds.size.y * (CELL_SIZE + GAP) - GAP + PAD * 2
	custom_minimum_size = Vector2(max(width, 320), max(height, 180))
	draw_rect(Rect2(Vector2.ZERO, custom_minimum_size), Color(0.13, 0.13, 0.15, 0.95))

	for x in bounds.size.x:
		for y in bounds.size.y:
			var gx := bounds.position.x + x
			var gy := bounds.position.y + y
			var cell := _get_rect(gx, gy, bounds)
			if store.is_occupied(gx, gy):
				draw_rect(cell, Color(0.35, 0.42, 0.55, 0.9))
				draw_rect(cell, Color(0.55, 0.62, 0.75), false, 1)
				var id_text := str(store.screen_id_at(gx, gy))
				var f := ThemeDB.fallback_font
				var tw := f.get_string_size(id_text, HORIZONTAL_ALIGNMENT_CENTER, -1, 12).x
				var pos := cell.position + Vector2((cell.size.x - tw) / 2, cell.size.y / 2)
				draw_string(f, pos, id_text, HORIZONTAL_ALIGNMENT_CENTER, -1, 12, Color.WHITE)
			else:
				draw_rect(cell, Color(0.2, 0.2, 0.22, 0.8))
				draw_rect(cell, Color(0.35, 0.35, 0.38), false, 1)
			if gx == source.x and gy == source.y:
				draw_rect(cell, Color(1, 0.85, 0.2, 0.5), true)
				draw_rect(cell, Color.YELLOW, false, 2)

	if hover.x >= 0 and hover.y >= 0:
		var bounds2 := store.bounds()
		if hover.x >= bounds2.position.x and hover.x < bounds2.end.x \
				and hover.y >= bounds2.position.y and hover.y < bounds2.end.y:
			var hc := _get_rect(hover.x, hover.y, bounds2)
			if not store.is_occupied(hover.x, hover.y):
				draw_rect(hc, Color(1, 1, 1, 0.35), true)
				draw_rect(hc, Color.WHITE, false, 2)
			else:
				draw_rect(hc, Color(1, 0.3, 0.3, 0.4), true)
				draw_rect(hc, Color(1, 0.4, 0.4), false, 2)

func _gui_input(event: InputEvent) -> void:
	if not store:
		return
	var bounds := store.bounds()
	var local := get_local_mouse_position()
	var gx := bounds.position.x + int((local.x - PAD) / (CELL_SIZE + GAP))
	var gy := bounds.position.y + int((local.y - PAD) / (CELL_SIZE + GAP))
	if event is InputEventMouseMotion:
		var inside := gx >= bounds.position.x and gx < bounds.end.x \
				and gy >= bounds.position.y and gy < bounds.end.y
		var nh := Vector2i(gx, gy) if inside else Vector2i(-1, -1)
		if nh != hover:
			hover = nh
			queue_redraw()
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		if gx < bounds.position.x or gx >= bounds.end.x or gy < bounds.position.y or gy >= bounds.end.y:
			return
		if store.is_occupied(gx, gy):
			return
		position_picked.emit(gx, gy)
