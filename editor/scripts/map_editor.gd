extends Control

const GRID_W := 30
const GRID_H := 16

var _tiles: Dictionary = {}
var _terrain_types: Array[Dictionary] = []
var _selected_terrain: String = "air"
var _paint_mode: bool = true
var _screen_id: int = 1
var _bridge: Node

@onready var palette_list: ItemList = $HSplit/LeftPanel/PaletteList
@onready var tile_grid: Control = $HSplit/RightPanel/Scroll/TileGrid
@onready var screen_spin: SpinBox = $HSplit/LeftPanel/TopBar/ScreenSpin
@onready var paint_btn: CheckButton = $HSplit/LeftPanel/TopBar/PaintBtn
@onready var erase_btn: CheckButton = $HSplit/LeftPanel/TopBar/EraseBtn
@onready var save_btn: Button = $HSplit/LeftPanel/BottomBar/SaveBtn
@onready var import_btn: Button = $HSplit/LeftPanel/BottomBar/ImportBtn
@onready var export_btn: Button = $HSplit/LeftPanel/BottomBar/ExportBtn
@onready var cursor_label: Label = $HSplit/LeftPanel/BottomBar/CursorLabel

func _ready() -> void:
    _load_defaults()
    _refresh_palette()
    _populate_grid()

    paint_btn.toggled.connect(_on_paint_mode)
    erase_btn.toggled.connect(_on_erase_mode)
    save_btn.pressed.connect(_on_save)
    import_btn.pressed.connect(_on_import)
    export_btn.pressed.connect(_on_export)
    screen_spin.value_changed.connect(_on_screen_changed)
    palette_list.item_selected.connect(_on_palette_select)
    tile_grid.map_editor = self

    paint_btn.button_pressed = true

func _load_defaults() -> void:
    _terrain_types = [
        {"key": "air",    "display_name": "Air",    "color_hex": "#87ceeb"},
        {"key": "grass",  "display_name": "Grass",  "color_hex": "#4a7c3f"},
        {"key": "dirt",   "display_name": "Dirt",   "color_hex": "#8b5e3c"},
        {"key": "stone",  "display_name": "Stone",  "color_hex": "#7a7a7a"},
        {"key": "wall",   "display_name": "Wall",   "color_hex": "#555555"},
        {"key": "water",  "display_name": "Water",  "color_hex": "#3a7bd5"},
        {"key": "lava",   "display_name": "Lava",   "color_hex": "#ff4500"},
    ]

func _refresh_palette() -> void:
    palette_list.clear()
    for t in _terrain_types:
        var idx := palette_list.add_item(t["display_name"])
        palette_list.set_item_custom_fg_color(idx, Color(t["color_hex"]))

func _populate_grid() -> void:
    _tiles.clear()
    for y in GRID_H:
        for x in GRID_W:
            _tiles[_key(x, y)] = "air"
    for x in GRID_W:
        _tiles[_key(x, GRID_H - 1)] = "grass"
        _tiles[_key(x, GRID_H - 2)] = "dirt"
    tile_grid.queue_redraw()

func _key(x: int, y: int) -> String:
    return "%d,%d" % [x, y]

func get_tile(x: int, y: int) -> String:
    return _tiles.get(_key(x, y), "air")

func terrain_color(key: String) -> Color:
    for t in _terrain_types:
        if t["key"] == key:
            return Color(t["color_hex"])
    return Color("#87ceeb")

func _paint_tile(x: int, y: int) -> void:
    if x < 0 or x >= GRID_W or y < 0 or y >= GRID_H:
        return
    _tiles[_key(x, y)] = _selected_terrain if _paint_mode else "air"
    tile_grid.queue_redraw()

func _on_paint_mode(_toggled: bool) -> void:
    _paint_mode = paint_btn.button_pressed
    if _paint_mode:
        erase_btn.set_pressed_no_signal(false)

func _on_erase_mode(toggled: bool) -> void:
    _paint_mode = not toggled
    if toggled:
        paint_btn.set_pressed_no_signal(false)

func _on_palette_select(index: int) -> void:
    if index >= 0 and index < _terrain_types.size():
        _selected_terrain = _terrain_types[index]["key"]

func _on_screen_changed(value: float) -> void:
    _screen_id = int(value)
    _populate_grid()

func _serialize() -> Dictionary:
    var tiles_out: Array[Dictionary] = []
    for y in GRID_H:
        for x in GRID_W:
            var key := get_tile(x, y)
            if key != "air":
                tiles_out.append({"x": x, "y": y, "terrain": key})
    return {
        "version": "0.1.0",
        "screen_id": _screen_id,
        "width_tiles": GRID_W,
        "height_tiles": GRID_H,
        "elevation_floor_tiles": 0,
        "elevation_ceiling_tiles": 4,
        "tiles": tiles_out,
        "placed_entities": [],
    }

func _on_save() -> void:
    var data := _serialize()
    var json_str := JSON.stringify(data, "\t")
    if _bridge and _bridge.has_method("import_screen"):
        var result := _bridge.import_screen(json_str)
        var parsed = JSON.parse_string(result)
        if parsed and parsed.has("error"):
            push_error("Bridge validation: ", parsed["error"])
            return
    print("Screen %d saved: %d tiles" % [_screen_id, data["tiles"].size()])

func _on_import() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
    dialog.add_filter("*.json", "Screen JSON")
    dialog.title = "Import screen_%d.json" % _screen_id
    add_child(dialog)
    dialog.file_selected.connect(_on_import_file)
    dialog.popup_centered(Vector2i(600, 400))

func _on_import_file(path: String) -> void:
    var f := FileAccess.open(path, FileAccess.READ)
    if not f:
        push_error("Cannot open: ", path)
        return
    var json_str := f.get_as_text()
    f.close()
    var parsed = JSON.parse_string(json_str)
    if not parsed or not parsed.has("tiles"):
        push_error("Invalid screen file")
        return
    _tiles.clear()
    for y in GRID_H:
        for x in GRID_W:
            _tiles[_key(x, y)] = "air"
    for t in parsed["tiles"]:
        if t.has("x") and t.has("y") and t.has("terrain"):
            _tiles[_key(t["x"], t["y"])] = t["terrain"]
    tile_grid.queue_redraw()

func _on_export() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_SAVE_FILE
    dialog.add_filter("*.json", "Screen JSON")
    dialog.title = "Export screen_%d.json" % _screen_id
    dialog.current_file = "screen_%d.json" % _screen_id
    add_child(dialog)
    dialog.file_selected.connect(func(path: String):
        var f := FileAccess.open(path, FileAccess.WRITE)
        if f:
            f.store_string(JSON.stringify(_serialize(), "\t"))
            f.close()
    )
    dialog.popup_centered(Vector2i(600, 400))

func _input(event: InputEvent) -> void:
    if event is InputEventMouseMotion:
        var pos := tile_grid.get_local_mouse_position()
        var tile := Vector2i(int(pos.x / 48), int(pos.y / 48))
        cursor_label.text = "Tile: %d, %d" % [tile.x, tile.y]

func set_bridge(b: Node) -> void:
    _bridge = b
