extends Control

var _grid_w: int = 60
var _grid_h: int = 33
var _tile_w: int = 32
var _tile_h: int = 32

var _tiles: Dictionary = {}
var _tile_data: Dictionary = {}  # "x,y" -> {sub_tile_mask, elevation_tiles, z_depth}
var _terrain_types: Array[Dictionary] = []
var _entity_defs: Array[Dictionary] = []
var _placements: Array[Dictionary] = []
var _selected_entity_key: String = ""
var _screen_id: int = 1
var _bridge: Node
var _main: Node

@onready var entity_palette: ItemList = $LeftPanel/EntityPalette
@onready var placement_grid: Control = $RightPanel/Scroll/PlacementGrid
@onready var screen_spin: SpinBox = $LeftPanel/TopBar/ScreenSpin
@onready var save_btn: Button = $LeftPanel/BottomBar/SaveBtn
@onready var import_map_btn: Button = $LeftPanel/BottomBar/ImportMapBtn
@onready var clear_btn: Button = $LeftPanel/BottomBar/ClearBtn
@onready var info_label: Label = $LeftPanel/BottomBar/InfoLabel

func _ready() -> void:
    _load_defaults()
    _refresh_palette()
    _populate_terrain()

    entity_palette.item_selected.connect(_on_select_entity)
    screen_spin.value_changed.connect(_on_screen_changed)
    save_btn.pressed.connect(_on_save)
    import_map_btn.pressed.connect(_on_import_map)
    clear_btn.pressed.connect(_on_clear)
    placement_grid.placement_editor = self

func _load_defaults() -> void:
    _terrain_types = [
        {"key": "air",   "color_hex": "#87ceeb"},
        {"key": "grass", "color_hex": "#4a7c3f"},
        {"key": "dirt",  "color_hex": "#8b5e3c"},
        {"key": "stone", "color_hex": "#7a7a7a"},
        {"key": "wall",  "color_hex": "#555555"},
    ]
    _entity_defs = [
        {"key": "arrow_tower", "class": "tower",  "width_tiles": 1.0, "height_tiles": 1.5},
        {"key": "ballista",    "class": "tower",  "width_tiles": 1.0, "height_tiles": 1.5},
        {"key": "catapult",    "class": "tower",  "width_tiles": 1.0, "height_tiles": 2.0},
        {"key": "wall",        "class": "structure", "width_tiles": 1.0, "height_tiles": 1.0},
        {"key": "bridge",      "class": "structure", "width_tiles": 2.0, "height_tiles": 0.5},
        {"key": "tarpit",      "class": "trap",   "width_tiles": 1.0, "height_tiles": 0.5},
        {"key": "mine",        "class": "trap",   "width_tiles": 1.0, "height_tiles": 0.5},
    ]

func _refresh_palette() -> void:
    entity_palette.clear()
    for e in _entity_defs:
        var idx := entity_palette.add_item(e["key"] + " (%s)" % e["class"])
        entity_palette.set_item_custom_fg_color(idx, _entity_class_color(e["class"]))



func _entity_class_color(c: String) -> Color:
    match c:
        "tower": return Color(1, 0.8, 0.2)
        "structure": return Color(0.6, 0.6, 0.6)
        "trap": return Color(1, 0.3, 0.3)
        _: return Color.WHITE

func _populate_terrain() -> void:
    _tiles.clear()
    _tile_data.clear()
    for y in _grid_h:
        for x in _grid_w:
            _tiles["%d,%d" % [x, y]] = "air"
    for x in _grid_w:
        _tiles["%d,%d" % [x, _grid_h - 1]] = "grass"
        _tiles["%d,%d" % [x, _grid_h - 2]] = "dirt"
    placement_grid.queue_redraw()

func get_tile(x: int, y: int) -> String:
    return _tiles.get("%d,%d" % [x, y], "air")

func get_tile_data(x: int, y: int) -> Dictionary:
    return _tile_data.get("%d,%d" % [x, y], {})

func terrain_color(key: String) -> Color:
    for t in _terrain_types:
        if t["key"] == key:
            return Color(t["color_hex"])
    if key.begins_with("slope_"):
        return Color("#4a7c3f")
    return Color("#87ceeb")

func _entity_def(key: String) -> Dictionary:
    for e in _entity_defs:
        if e["key"] == key:
            return e
    return {"width_tiles": 1.0, "height_tiles": 1.0}

func _occupied_tiles(tile_x: int, tile_y: int, w: float, h: float) -> Array[Vector2i]:
    var tiles: Array[Vector2i] = []
    var cols := int(ceil(w))
    var rows := int(ceil(h))
    for dy in rows:
        for dx in range(cols):
            tiles.append(Vector2i(tile_x + dx, tile_y - dy))
    return tiles

func get_placements() -> Array[Dictionary]:
    var result: Array[Dictionary] = []
    for p in _placements:
        var entry := p.duplicate()
        var def := _entity_def(p["entity_key"])
        entry["width"] = def["width_tiles"] * placement_grid.tile_size
        entry["height"] = def["height_tiles"] * placement_grid.tile_size
        entry["color"] = _entity_class_color(def["class"])
        entry["key"] = def["key"]
        result.append(entry)
    return result

func get_selected_footprint() -> Dictionary:
    if _selected_entity_key.is_empty():
        return {"w_tiles": 0, "h_tiles": 0}
    var def := _entity_def(_selected_entity_key)
    return {"w_tiles": def["width_tiles"], "h_tiles": def["height_tiles"]}

func _on_select_entity(index: int) -> void:
    if index >= 0 and index < _entity_defs.size():
        _selected_entity_key = _entity_defs[index]["key"]
        info_label.text = "Selected: %s" % _selected_entity_key

func _on_screen_changed(value: float) -> void:
    _screen_id = int(value)
    _placements.clear()
    info_label.text = "Screen %d" % _screen_id

func place_at(tile_x: int, tile_y: int) -> void:
    if tile_x < 0 or tile_x >= _grid_w or tile_y < 0 or tile_y >= _grid_h:
        return

    if _selected_entity_key.is_empty():
        return

    var def := _entity_def(_selected_entity_key)
    var new_tiles := _occupied_tiles(tile_x, tile_y, def["width_tiles"], def["height_tiles"])
    for p in _placements:
        var pdef := _entity_def(p["entity_key"])
        for pt in _occupied_tiles(p["tile_x"], int(p["tile_y"]), pdef["width_tiles"], pdef["height_tiles"]):
            if pt in new_tiles:
                return
    for t in new_tiles:
        if t.x >= 0 and t.x < _grid_w and t.y >= 0 and t.y < _grid_h:
            var terrain := get_tile(t.x, t.y)
            if terrain == "wall" or terrain == "lava":
                info_label.text = "Cannot place on %s at %d,%d" % [terrain, t.x, t.y]
                return
    _placements.append({
        "entity_key": _selected_entity_key,
        "tile_x": tile_x,
        "tile_y": float(tile_y),
        "rotation": 0.0,
    })
    placement_grid.queue_redraw()
    info_label.text = "Placed %s at %d,%d" % [_selected_entity_key, tile_x, tile_y]

func remove_at(tile_x: int, tile_y: int) -> void:
    var removed := false
    var click := Vector2i(tile_x, tile_y)
    for i in range(_placements.size() - 1, -1, -1):
        var p := _placements[i]
        var def := _entity_def(p["entity_key"])
        var tiles := _occupied_tiles(p["tile_x"], int(p["tile_y"]), def["width_tiles"], def["height_tiles"])
        if click in tiles:
            _placements.remove_at(i)
            removed = true
            break
    if removed:
        placement_grid.queue_redraw()
        info_label.text = "Removed at %d,%d" % [tile_x, tile_y]

func _on_clear() -> void:
    _placements.clear()
    _populate_terrain()
    info_label.text = "Reset to default"

func _serialize() -> Dictionary:
    var tiles_out: Array[Dictionary] = []
    for y in _grid_h:
        for x in _grid_w:
            var key := get_tile(x, y)
            if key != "air":
                var td: Dictionary = _tile_data.get("%d,%d" % [x, y], {})
                var entry := {"x": x, "y": y, "terrain": key}
                if td.has("sub_tile_mask"):
                    entry["sub_tile_mask"] = td["sub_tile_mask"]
                if td.has("elevation_tiles"):
                    entry["elevation_tiles"] = td["elevation_tiles"]
                if td.has("z_depth"):
                    entry["z_depth"] = td["z_depth"]
                tiles_out.append(entry)
    var ents_out: Array[Dictionary] = []
    for p in _placements:
        ents_out.append({
            "entity_key": p["entity_key"],
            "world_tile_x": p["tile_x"],
            "world_tile_y": int(p["tile_y"]),
        })
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
        "placed_entities": ents_out,
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
        if _bridge and _bridge.has_method("validate_screen"):
            var result: Variant = _bridge.validate_screen(json_str)
            var parsed = JSON.parse_string(result)
            if parsed and parsed.get("valid", false) == false:
                push_error("Validation: ", parsed.get("error", "unknown"))
                return
        var f := FileAccess.open(path, FileAccess.WRITE)
        if not f:
            push_error("Cannot write: ", path)
            return
        f.store_string(json_str)
        f.close()
        info_label.text = "Saved: %s (%d tiles, %d entities)" % [path.get_file(), data["tiles"].size(), data["placed_entities"].size()]
    )
    dialog.popup_centered(Vector2i(600, 400))

func _on_import_map() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
    dialog.add_filter("*.json", "Screen JSON")
    dialog.title = "Import screen map"
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
    if not parsed:
        push_error("Invalid JSON")
        return
    _tiles.clear()
    _tile_data.clear()
    for y in _grid_h:
        for x in _grid_w:
            _tiles["%d,%d" % [x, y]] = "air"
    if parsed.has("tiles"):
        for t in parsed["tiles"]:
            if t.has("x") and t.has("y") and t.has("terrain"):
                _tiles["%d,%d" % [t["x"], t["y"]]] = t["terrain"]
                var td: Dictionary = {}
                if t.has("sub_tile_mask"):
                    td["sub_tile_mask"] = t["sub_tile_mask"]
                if t.has("elevation_tiles"):
                    td["elevation_tiles"] = t["elevation_tiles"]
                if t.has("z_depth"):
                    td["z_depth"] = t["z_depth"]
                if not td.is_empty():
                    _tile_data["%d,%d" % [t["x"], t["y"]]] = td
    _placements.clear()
    if parsed.has("placed_entities"):
        for e in parsed["placed_entities"]:
            _placements.append({
                "entity_key": e["entity_key"],
                "tile_x": e["world_tile_x"],
                "tile_y": float(e["world_tile_y"]),
                "rotation": e.get("rotation", 0.0),
            })
    placement_grid.queue_redraw()
    info_label.text = "Imported %d tiles, %d entities" % [parsed.get("tiles", []).size(), _placements.size()]

func set_bridge(b: Node) -> void:
    _bridge = b

func set_main_reference(m: Node) -> void:
    _main = m

func _log(msg: String) -> void:
    if _main:
        _main.log(msg)
    else:
        print(msg)

func set_terrain_types(types: Array[Dictionary]) -> void:
    _terrain_types = types
    placement_grid.queue_redraw()

func set_tile_grid(tiles: Dictionary, tile_data: Dictionary) -> void:
    _tiles = tiles.duplicate()
    _tile_data = tile_data.duplicate()
    placement_grid.queue_redraw()

func set_grid_config(cfg: Dictionary) -> void:
    _grid_w = cfg.get("max_tiles_per_screen_x", 60)
    _grid_h = cfg.get("max_tiles_per_screen_y", 33)
    _tile_w = cfg.get("tile_width_in_pixels", 32)
    _tile_h = cfg.get("tile_height_in_pixels", 32)
    if placement_grid and placement_grid.has_method("set_grid_config"):
        placement_grid.set_grid_config(cfg)
    _populate_terrain()

func set_entity_defs(defs: Array[Dictionary]) -> void:
    if defs.size() > 0:
        _entity_defs = defs
        _refresh_palette()
