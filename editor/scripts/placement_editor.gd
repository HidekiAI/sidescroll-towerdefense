extends Control

var _grid_w: int = 60
var _grid_h: int = 33
var _tile_w: int = 32
var _tile_h: int = 32

var _tiles: Dictionary = {}
var _terrain_types: Array[Dictionary] = []
var _entity_defs: Array[Dictionary] = []
var _placements: Array[Dictionary] = []
var _selected_entity_key: String = ""
var _screen_id: int = 1
var _bridge: Node

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
        {"key": "catapult",    "class": "tower",  "width_tiles": 2.0, "height_tiles": 2.0},
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
    for y in _grid_h:
        for x in _grid_w:
            _tiles["%d,%d" % [x, y]] = "air"
    for x in _grid_w:
        _tiles["%d,%d" % [x, _grid_h - 1]] = "grass"
        _tiles["%d,%d" % [x, _grid_h - 2]] = "dirt"
    placement_grid.queue_redraw()

func get_tile(x: int, y: int) -> String:
    return _tiles.get("%d,%d" % [x, y], "air")

func terrain_color(key: String) -> Color:
    for t in _terrain_types:
        if t["key"] == key:
            return Color(t["color_hex"])
    return Color("#87ceeb")

func get_placements() -> Array[Dictionary]:
    var result: Array[Dictionary] = []
    for p in _placements:
        var entry := p.duplicate()
        for e in _entity_defs:
            if e["key"] == p["entity_key"]:
                entry["width"] = e["width_tiles"] * placement_grid.tile_size
                entry["height"] = e["height_tiles"] * placement_grid.tile_size
                entry["color"] = _entity_class_color(e["class"])
                entry["key"] = e["key"]
                break
        result.append(entry)
    return result

func _on_select_entity(index: int) -> void:
    if index >= 0 and index < _entity_defs.size():
        _selected_entity_key = _entity_defs[index]["key"]
        info_label.text = "Selected: %s" % _selected_entity_key

func _on_screen_changed(value: float) -> void:
    _screen_id = int(value)
    _placements.clear()
    info_label.text = "Screen %d" % _screen_id

func place_at(tile_x: int, tile_y: int) -> void:
    if _selected_entity_key.is_empty():
        return
    if tile_x < 0 or tile_x >= _grid_w or tile_y < 0 or tile_y >= _grid_h:
        return
    for p in _placements:
        if p["tile_x"] == tile_x and int(p["tile_y"]) == tile_y:
            return  # already occupied
    var terrain := get_tile(tile_x, tile_y)
    if terrain == "wall" or terrain == "lava":
        info_label.text = "Cannot place on %s" % terrain
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
    for i in range(_placements.size() - 1, -1, -1):
        if _placements[i]["tile_x"] == tile_x and int(_placements[i]["tile_y"]) == tile_y:
            _placements.remove_at(i)
            removed = true
            break
    if removed:
        placement_grid.queue_redraw()
        info_label.text = "Removed at %d,%d" % [tile_x, tile_y]

func _on_clear() -> void:
    _placements.clear()
    placement_grid.queue_redraw()
    info_label.text = "Placements cleared"

func _serialize() -> Dictionary:
    var tiles_out: Array[Dictionary] = []
    for y in _grid_h:
        for x in _grid_w:
            var key := get_tile(x, y)
            if key != "air":
                tiles_out.append({"x": x, "y": y, "terrain": key})
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
    if _bridge and _bridge.has_method("validate_screen"):
        var result: Variant = _bridge.validate_screen(json_str)
        var parsed = JSON.parse_string(result)
        if parsed and parsed.get("valid", false) == false:
            push_error("Validation: ", parsed.get("error", "unknown"))
            return
    print("Screen %d saved: %d tiles, %d entities" % [_screen_id, data["tiles"].size(), data["placed_entities"].size()])

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
    for y in _grid_h:
        for x in _grid_w:
            _tiles["%d,%d" % [x, y]] = "air"
    if parsed.has("tiles"):
        for t in parsed["tiles"]:
            if t.has("x") and t.has("y") and t.has("terrain"):
                _tiles["%d,%d" % [t["x"], t["y"]]] = t["terrain"]
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
