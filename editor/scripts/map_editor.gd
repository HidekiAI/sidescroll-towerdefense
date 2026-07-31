extends Control

const _GodotTileSetGrouping := preload("res://scripts/resources/godot_tileset_grouping.gd")

var _grid_w: int = 60
var _grid_h: int = 33
var _tile_w: int = 32
var _tile_h: int = 32

var _tiles: Dictionary = {}
var _tile_data: Dictionary = {}
var _terrain_types: Array[Dictionary] = []
var _selected_terrain: String = "air"
var _paint_mode: bool = true
var _screen_id: int = 1
var _bridge: Node

var _main: Node
var _tile_set_groupings: Array = []
var _selected_tile_set_key: String = ""
var _flip_h_active: bool = false
var _flip_v_active: bool = false

@onready var palette_list: ItemList = $LeftPanel/PaletteList
@onready var tile_set_palette: ItemList = $LeftPanel/TileSetPalette
@onready var tile_grid: Control = $RightPanel/Scroll/TileGrid
@onready var screen_spin: SpinBox = $LeftPanel/TopBar/ScreenSpin
@onready var paint_btn: CheckButton = $LeftPanel/TopBar/PaintBtn
@onready var erase_btn: CheckButton = $LeftPanel/TopBar/EraseBtn
@onready var save_btn: Button = $LeftPanel/BottomBar/SaveBtn
@onready var import_btn: Button = $LeftPanel/BottomBar/ImportBtn
@onready var export_btn: Button = $LeftPanel/BottomBar/ExportBtn
@onready var collision_btn: CheckButton = $LeftPanel/TopBar/CollisionBtn
@onready var cursor_label: Label = $LeftPanel/BottomBar/CursorLabel
@onready var info_label: Label = $LeftPanel/BottomBar/InfoLabel

func _ready() -> void:
    _load_defaults()
    _refresh_palette()
    _load_tile_sets()
    _rebuild_terrain_tilesets()
    _refresh_tile_set_palette()
    _populate_grid()

    paint_btn.toggled.connect(_on_paint_mode)
    erase_btn.toggled.connect(_on_erase_mode)
    save_btn.pressed.connect(_on_save)
    import_btn.pressed.connect(_on_import)
    export_btn.pressed.connect(_on_export)
    screen_spin.value_changed.connect(_on_screen_changed)
    palette_list.item_selected.connect(_on_palette_select)
    tile_set_palette.item_selected.connect(_on_select_tile_set)
    collision_btn.toggled.connect(_on_toggle_collision)
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

func _rebuild_terrain_tilesets() -> void:
    _tile_set_groupings = _tile_set_groupings.filter(func(ts):
        return not ts.tags.has("terrain_default"))
    for t in _terrain_types:
        var key: String = "terrain_" + str(t.get("key", ""))
        if _tile_set_groupings.any(func(ts): return ts.key == key):
            continue
        var ts := TileSetGrouping.new()
        ts.key = key
        ts.display_name = t.get("display_name", t["key"])
        ts.width_tiles = 1
        ts.height_tiles = 1
        ts.tiles = [{
            "local_x": 0, "local_y": 0,
            "terrain_key": t["key"],
            "elevation_tiles": 0, "z_depth": 0,
            "sub_tile_mask": 15,
            "flip_h": false, "flip_v": false,
        }]
        ts.tags = ["terrain_default", "category:terrain"]
        _tile_set_groupings.append(ts)

func _populate_grid() -> void:
    _tiles.clear()
    _tile_data.clear()
    for y in _grid_h:
        for x in _grid_w:
            _tiles[_key(x, y)] = "air"
    for x in _grid_w:
        _tiles[_key(x, _grid_h - 1)] = "grass"
        _tiles[_key(x, _grid_h - 2)] = "dirt"
    tile_grid.queue_redraw()

func _key(x: int, y: int) -> String:
    return "%d,%d" % [x, y]

func get_tile(x: int, y: int) -> String:
    return _tiles.get(_key(x, y), "air")

func get_tile_data(x: int, y: int) -> Dictionary:
    return _tile_data.get(_key(x, y), {})

func get_tile_grid() -> Dictionary:
    return {
        "tiles": _tiles.duplicate(),
        "tile_data": _tile_data.duplicate(),
    }

func terrain_color(key: String) -> Color:
    for t in _terrain_types:
        if t["key"] == key:
            return Color(t["color_hex"])
    if key.begins_with("slope_"):
        return Color("#4a7c3f")
    return Color("#87ceeb")

func _paint_tile(x: int, y: int) -> void:
    if x < 0 or x >= _grid_w or y < 0 or y >= _grid_h:
        return
    if _paint_mode and not _selected_tile_set_key.is_empty():
        _place_tile_set(x, y)
    elif _paint_mode:
        _tiles[_key(x, y)] = _selected_terrain
    else:
        _tiles[_key(x, y)] = "air"
        _tile_data.erase(_key(x, y))
    tile_grid.queue_redraw()

func _load_tile_sets() -> void:
    _tile_set_groupings.clear()
    var dir := DirAccess.open("res://assets/tilesets")
    if not dir:
        return
    dir.list_dir_begin()
    var fname := dir.get_next()
    while fname != "":
        if fname.ends_with(".tres") or fname.ends_with(".res"):
            var path := "res://assets/tilesets/" + fname
            var res = load(path)
            if res and res.has_method("validate") and res.validate():
                _tile_set_groupings.append(res)
            elif res is TileSet:
                _wrap_godot_tileset(res)
        fname = dir.get_next()
    dir.list_dir_end()
    _refresh_tile_set_palette()

func _wrap_godot_tileset(ts: TileSet) -> void:
    if ts.get_source_count() < 1:
        return
    var source: TileSetAtlasSource = ts.get_source(0)
    var tex: Texture2D = source.texture
    if not tex:
        return
    var tex_w := tex.get_width()
    var tex_h := tex.get_height()
    var valid: Array[Vector2i] = []
    for id in source.get_tile_ids():
        var reg: Rect2i = source.get_tile_region(id)
        if reg.position.x < tex_w and reg.position.y < tex_h:
            valid.append(id)
    if valid.size() < 1:
        return
    valid.sort_custom(func(a, b): return a.y == b.y if a.y != b.y else a.x < b.x)
    var max_x := 0
    var max_y := 0
    for id in valid:
        if id.x > max_x:
            max_x = id.x
        if id.y > max_y:
            max_y = id.y
    var w := max_x + 1
    var h := max_y + 1
    if w > 8 or h > 4:
        w = min(4, valid.size())
        h = max(1, ceili(float(valid.size()) / w))
    var is_terrain := w * h <= 4
    var wrapper = _GodotTileSetGrouping.new()
    wrapper.key = source.resource_name if not source.resource_name.is_empty() else ("godot_ts_" + str(ts.get_instance_id()))
    wrapper.godot_tileset = ts
    wrapper.display_name = source.resource_name if not source.resource_name.is_empty() else "TileSet"
    wrapper.width_tiles = w
    wrapper.height_tiles = h
    wrapper.tags = ["terrain"] if is_terrain else ["slope"]
    for i in range(min(valid.size(), w * h)):
        var id := valid[i]
        wrapper.tile_coords.append({"col": id.x, "row": id.y, "local_x": i % w, "local_y": i / w})
    _tile_set_groupings.append(wrapper)

func _refresh_tile_set_palette() -> void:
    tile_set_palette.clear()
    for ts in _tile_set_groupings:
        if ts.tags.has("terrain_default"):
            continue
        var label := "%s  %d×%d" % [ts.display_name, ts.width_tiles, ts.height_tiles]
        var idx := tile_set_palette.add_item(label)
        tile_set_palette.set_item_metadata(idx, ts.key)
        if "terrain" in ts.tags:
            tile_set_palette.set_item_custom_fg_color(idx, Color(0.3, 0.8, 0.3))
        elif "slope" in ts.tags:
            tile_set_palette.set_item_custom_fg_color(idx, Color(0.8, 0.7, 0.3))

func _on_palette_select(index: int) -> void:
    if index >= 0 and index < _terrain_types.size():
        _selected_terrain = _terrain_types[index]["key"]
        _selected_tile_set_key = "terrain_" + _selected_terrain
        tile_set_palette.deselect_all()
        _update_info()

func _on_select_tile_set(index: int) -> void:
    if index < 0 or index >= tile_set_palette.get_item_count():
        return
    var key = tile_set_palette.get_item_metadata(index)
    if typeof(key) != TYPE_STRING or (key as String).is_empty():
        return
    _selected_tile_set_key = key
    palette_list.deselect_all()
    _update_info()

func _on_toggle_collision(visible: bool) -> void:
    tile_grid.show_collision = visible
    tile_grid.queue_redraw()

func select_tile_set(key: String) -> void:
    for i in tile_set_palette.item_count:
        if tile_set_palette.get_item_metadata(i) as String == key:
            tile_set_palette.select(i)
            _on_select_tile_set(i)
            return
    _selected_tile_set_key = key
    tile_set_palette.deselect_all()
    _update_info()

func _find_tile_set(key: String):
    for ts in _tile_set_groupings:
        if ts.key == key:
            return ts
    return null

func _place_tile_set(base_x: int, base_y: int) -> void:
    var ts = _find_tile_set(_selected_tile_set_key)
    if not ts:
        return
    var resolved: Array = ts.resolve(base_x, base_y)
    for entry in resolved:
        var ex: int = entry["x"]
        var ey: int = entry["y"]
        if ex < 0 or ex >= _grid_w or ey < 0 or ey >= _grid_h:
            continue
        var tkey: String = entry["terrain_key"]
        var flip_h := bool(entry.get("flip_h", false))
        var flip_v := bool(entry.get("flip_v", false))
        if _flip_h_active:
            flip_h = not flip_h
        if _flip_v_active:
            flip_v = not flip_v
        _tiles[_key(ex, ey)] = tkey
        var td: Dictionary = {
            "sub_tile_mask": entry["sub_tile_mask"],
            "elevation_tiles": entry["elevation_tiles"],
            "z_depth": entry["z_depth"],
            "flip_h": flip_h,
            "flip_v": flip_v,
        }
        if entry.has("source_image") and not entry["source_image"].is_empty():
            td["source_image"] = entry["source_image"]
            if entry.has("source_rect"):
                td["source_rect"] = entry["source_rect"]
        _tile_data[_key(ex, ey)] = td
    tile_grid.queue_redraw()
    var placed_msg := "Placed %s tiles from %s" % [resolved.size(), ts.display_name]
    if _flip_h_active:
        placed_msg += "  [X-flip]"
    if _flip_v_active:
        placed_msg += "  [Y-flip]"
    info_label.text = placed_msg

func _update_info() -> void:
    var msg := ""
    if not _selected_tile_set_key.is_empty():
        var ts = _find_tile_set(_selected_tile_set_key)
        if ts:
            msg = "%s  %d×%d" % [ts.display_name, ts.width_tiles, ts.height_tiles]
    if _flip_h_active:
        msg += "  [X-flip]"
    if _flip_v_active:
        msg += "  [Y-flip]"
    info_label.text = msg

func _on_paint_mode(toggled: bool) -> void:
    _paint_mode = paint_btn.button_pressed
    if _paint_mode:
        erase_btn.set_pressed_no_signal(false)

func _on_erase_mode(toggled: bool) -> void:
    _paint_mode = not toggled
    if toggled:
        paint_btn.set_pressed_no_signal(false)

func _on_screen_changed(value: float) -> void:
    _screen_id = int(value)
    _populate_grid()

func _serialize() -> Dictionary:
    var tiles_out: Array[Dictionary] = []
    for y in _grid_h:
        for x in _grid_w:
            var key := get_tile(x, y)
            if key != "air":
                var td := get_tile_data(x, y)
                var entry: Dictionary = {"x": x, "y": y, "terrain": key}
                if not td.is_empty():
                    entry["tile_data"] = td
                tiles_out.append(entry)
    return {
        "version": "0.1.0",
        "screen_id": _screen_id,
        "width_tiles": _grid_w,
        "height_tiles": _grid_h,
        "tile_width_px": _tile_w,
        "tile_height_px": _tile_h,
        "elevation_floor_tiles": 0,
        "elevation_ceiling_tiles": 4,
        "tiles": tiles_out,
        "placed_entities": [],
    }

func _on_save() -> void:
    var data := _serialize()
    var json_str := JSON.stringify(data, "\t")
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_SAVE_FILE
    dialog.add_filter("*.json", "Screen JSON")
    dialog.title = "Save screen_%d.json" % _screen_id
    dialog.current_file = "screen_%d.json" % _screen_id
    add_child(dialog)
    dialog.file_selected.connect(func(path: String):
        if _bridge and _bridge.has_method("import_screen"):
            var result: Variant = _bridge.import_screen(json_str)
            var parsed = JSON.parse_string(result)
            if parsed and parsed.has("error"):
                push_error("Bridge validation: ", parsed["error"])
                return
        var f := FileAccess.open(path, FileAccess.WRITE)
        if not f:
            push_error("Cannot write: ", path)
            return
        f.store_string(json_str)
        f.close()
        if _main and _main.has_method("_save_last_map_path"):
            _main._save_last_map_path(path)
        info_label.text = "Saved: %s (%d tiles)" % [path.get_file(), data["tiles"].size()]
    )
    dialog.popup_centered(Vector2i(600, 400))

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
    if parsed.has("width_tiles"):
        _grid_w = parsed["width_tiles"]
    if parsed.has("height_tiles"):
        _grid_h = parsed["height_tiles"]
    _tiles.clear()
    _tile_data.clear()
    for y in _grid_h:
        for x in _grid_w:
            _tiles[_key(x, y)] = "air"
    for t in parsed["tiles"]:
        if t.has("x") and t.has("y") and t.has("terrain"):
            _tiles[_key(t["x"], t["y"])] = t["terrain"]
            if t.has("tile_data"):
                _tile_data[_key(t["x"], t["y"])] = t["tile_data"]
    tile_grid.queue_redraw()
    if _main and _main.has_method("_save_last_map_path"):
        _main._save_last_map_path(path)
    info_label.text = "Loaded: %s (%d tiles)" % [path.get_file(), parsed["tiles"].size()]

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
        var tile := Vector2i(int(pos.x / tile_grid.tile_size), int(pos.y / tile_grid.tile_size))
        cursor_label.text = "Tile: %d, %d" % [tile.x, tile.y]
    if event is InputEventKey and event.pressed and not event.echo:
        if event.keycode == KEY_X:
            _flip_h_active = not _flip_h_active
            _update_info()
            get_viewport().set_input_as_handled()
        if event.keycode == KEY_Y:
            _flip_v_active = not _flip_v_active
            _update_info()
            get_viewport().set_input_as_handled()

func set_main_reference(m: Node) -> void:
    _main = m

func set_bridge(b: Node) -> void:
    _bridge = b

func set_terrain_types(types: Array[Dictionary]) -> void:
    _terrain_types = types
    _rebuild_terrain_tilesets()
    _refresh_palette()
    _refresh_tile_set_palette()
    tile_grid.queue_redraw()

func set_grid_config(cfg: Dictionary) -> void:
    _grid_w = cfg.get("max_tiles_per_screen_x", 60)
    _grid_h = cfg.get("max_tiles_per_screen_y", 33)
    _tile_w = cfg.get("tile_width_in_pixels", 32)
    _tile_h = cfg.get("tile_height_in_pixels", 32)
    if tile_grid and tile_grid.has_method("set_grid_config"):
        tile_grid.set_grid_config(cfg)
    _populate_grid()
