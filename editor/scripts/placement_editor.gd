extends Control

const _ScreenMinimapDialog := preload("res://scenes/screen_minimap_dialog.tscn")
const WORLD_PATH := "res://world.json"
const STAMP_CELL := 32

var _grid_w: int = 60
var _grid_h: int = 33
var _tile_w: int = 32
var _tile_h: int = 32

var _tiles: Dictionary = {}
var _tile_data: Dictionary = {}  # "x,y" -> {sub_tile_mask, elevation_tiles, z_depth}
var _terrain_types: Array[Dictionary] = []
var _tile_images: Dictionary = {}  # tile key -> Image (resident in memory; reused across the world)
var _tile_texture_cache: Dictionary = {}  # tile key -> ImageTexture
var _entity_defs: Array[Dictionary] = []
var _placements: Array[Dictionary] = []
var _selected_entity_key: String = ""
var _screen_id: int = 1
var _bridge: Node
var _main: Node

var _store: ScreenStore
var _screen_pos: Vector2i = Vector2i.ZERO
var _clipboard: Dictionary = {}

@onready var entity_palette: ItemList = $LeftPanel/EntityPalette
@onready var placement_grid: Control = $RightPanel/Scroll/PlacementGrid
@onready var screen_spin: SpinBox = $LeftPanel/TopBar/ScreenSpin
@onready var save_btn: Button = $LeftPanel/BottomBar/SaveBtn
@onready var import_map_btn: Button = $LeftPanel/BottomBar/ImportMapBtn
@onready var clear_btn: Button = $LeftPanel/BottomBar/ClearBtn
@onready var info_label: Label = $LeftPanel/BottomBar/InfoLabel
@onready var hud_label: Label = $HUD
@onready var add_btn: Button = $LeftPanel/BottomBar/AddBtn
@onready var delete_btn: Button = $LeftPanel/BottomBar/DeleteBtn
@onready var move_btn: Button = $LeftPanel/BottomBar/MoveBtn
@onready var clone_btn: Button = $LeftPanel/BottomBar/CloneBtn

func _ready() -> void:
    hud_label.visible = false
    _load_defaults()
    _refresh_palette()
    _populate_terrain()

    entity_palette.item_selected.connect(_on_select_entity)
    screen_spin.value_changed.connect(_on_screen_changed)
    save_btn.pressed.connect(_on_save)
    import_map_btn.pressed.connect(_on_import_map)
    clear_btn.pressed.connect(_on_clear)
    add_btn.pressed.connect(_on_add_screen)
    delete_btn.pressed.connect(_on_delete_screen)
    move_btn.pressed.connect(_on_move_screen)
    clone_btn.pressed.connect(_on_clone_screen)
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
        {"key": "arrow_tower", "class": "tower",  "width_tiles": 1.0, "height_tiles": 2.5},
        {"key": "ballista",    "class": "tower",  "width_tiles": 1.0, "height_tiles": 2.0},
        {"key": "catapult",    "class": "tower",  "width_tiles": 1.0, "height_tiles": 2.0},
        {"key": "stone_golem", "class": "tower",  "width_tiles": 1.0, "height_tiles": 2.5},
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

func get_tile_image(key: String) -> Image:
    if key.is_empty():
        return null
    if _tile_images.has(key):
        return _tile_images[key]
    var abs := ProjectSettings.globalize_path("res://assets/tiles/%s_%dx%d.png" % [key, STAMP_CELL, STAMP_CELL])
    if FileAccess.file_exists(abs):
        var img := Image.new()
        if img.load(abs) == OK:
            _tile_images[key] = WorldArchive.normalize_rgba8(img)
            return _tile_images[key]
    return null

func get_tile_texture(key: String) -> Texture2D:
    if _tile_texture_cache.has(key):
        return _tile_texture_cache[key]
    var img := get_tile_image(key)
    if img:
        var tex: ImageTexture = ImageTexture.create_from_image(img)
        _tile_texture_cache[key] = tex
        return tex
    _tile_texture_cache[key] = null
    return null

func _restore_embedded_tiles(embedded: Dictionary) -> void:
    if embedded.is_empty():
        return
    for key in embedded:
        var img := Image.new()
        if img.load_png_from_buffer(Marshalls.base64_to_raw(embedded[key])) != OK:
            continue
        _tile_images[str(key)] = img
        _tile_texture_cache.erase(str(key))

func _restore_tile_bank(bank: Dictionary) -> void:
    if bank.is_empty():
        return
    for key in bank:
        var img: Image = bank[key]
        if img:
            _tile_images[str(key)] = img
            _tile_texture_cache.erase(str(key))

# Tile-bank sync between editors: the placement editor renders grid textures
# from its OWN _tile_images, which is only seeded when THIS editor loads a world
# package. When the world was loaded in the map editor, main.gd seeds this bank
# from map_editor.get_tile_bank() on tab switch. See #47.
func get_tile_bank() -> Dictionary:
    return _tile_images

func set_tile_bank(bank: Dictionary) -> void:
    if bank.is_empty():
        return
    for key in bank:
        var img: Image = bank[key]
        if img:
            _tile_images[str(key)] = img
            _tile_texture_cache.erase(str(key))

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
    _cache_current()
    _screen_id = int(value)
    _restore_screen()
    _update_hud()

func set_screen_store(s: ScreenStore) -> void:
    _store = s
    _sync_position_from_id()
    _restore_screen()
    _update_hud()

func cache_current_screen() -> void:
    _cache_current()

func _screen_pos_of_current() -> Vector2i:
    if _store:
        return _store.position_of_id(_screen_id)
    return Vector2i(-1, -1)

func _sync_position_from_id() -> void:
    _screen_pos = _screen_pos_of_current()

func _is_current_placed() -> bool:
    return _store != null and _store.has_id(_screen_id)

func _cache_current() -> void:
    if _is_current_placed():
        _store.set_cache(_screen_pos.x, _screen_pos.y, _serialize())

func _load_screen_file(path: String) -> Dictionary:
    if path.is_empty() or not FileAccess.file_exists(path):
        return {}
    if WorldArchive.is_world_path(path):
        var data := WorldArchive.load_world(path)
        return data if data.get("ok", false) else {}
    return _load_plain_json(path)

func _load_plain_json(path: String) -> Dictionary:
    var f := FileAccess.open(path, FileAccess.READ)
    if not f:
        return {}
    var parsed = JSON.parse_string(f.get_as_text())
    f.close()
    if typeof(parsed) != TYPE_DICTIONARY:
        return {}
    return parsed

func _apply_screen(parsed: Dictionary) -> void:
    _restore_embedded_tiles(parsed.get("embedded_tiles", {}))
    _tiles.clear()
    _tile_data.clear()
    if parsed.has("width_tiles"):
        _grid_w = parsed["width_tiles"]
    if parsed.has("height_tiles"):
        _grid_h = parsed["height_tiles"]
    for y in _grid_h:
        for x in _grid_w:
            _tiles["%d,%d" % [x, y]] = "air"
    for t in parsed.get("tiles", []):
        if t.has("x") and t.has("y") and t.has("terrain"):
            _tiles["%d,%d" % [t["x"], t["y"]]] = t["terrain"]
            var td: Dictionary = {}
            if t.has("tile_data"):
                td = t["tile_data"]
            else:
                if t.has("sub_tile_mask"):
                    td["sub_tile_mask"] = t["sub_tile_mask"]
                if t.has("elevation_tiles"):
                    td["elevation_tiles"] = t["elevation_tiles"]
                if t.has("z_depth"):
                    td["z_depth"] = t["z_depth"]
            if not td.is_empty():
                _tile_data["%d,%d" % [t["x"], t["y"]]] = td
    _placements.clear()
    for e in parsed.get("placed_entities", []):
        _placements.append({
            "entity_key": e["entity_key"],
            "tile_x": e["world_tile_x"],
            "tile_y": float(e["world_tile_y"]),
            "rotation": e.get("rotation", 0.0),
        })
    placement_grid.queue_redraw()

func _restore_screen() -> void:
    _sync_position_from_id()
    if _is_current_placed():
        var cached: Dictionary = _store.get_cache(_screen_pos.x, _screen_pos.y)
        var parsed: Dictionary = cached if not cached.is_empty() \
                else _load_screen_file(_store.screen_path(_screen_pos.x, _screen_pos.y))
        if not parsed.is_empty() and parsed.has("tiles"):
            _apply_screen(parsed)
            return
    _populate_terrain()

func _world_json_path() -> String:
    if _main and _main.has_method("get_world_json_path"):
        return _main.get_world_json_path()
    return WORLD_PATH

# Preselect the load/save dialog filter to match the current world format (#62):
func _save_world() -> void:
    if _store:
        _store.save_world(_world_json_path())
    if _main and _main.has_method("clear_dirty"):
        _main.clear_dirty()

func _open_minimap(mode: String, on_pick: Callable) -> void:
    if not _store:
        push_error("ScreenStore not set")
        return
    var dialog: Window = _ScreenMinimapDialog.instantiate()
    add_child(dialog)
    var grid: ScreenMinimap = dialog.get_node("Panel/VBox/Grid")
    grid.setup(_store, mode, _screen_pos)
    var title: Label = dialog.get_node("Panel/VBox/Title")
    var hint: Label = dialog.get_node("Panel/VBox/Hint")
    match mode:
        "add":
            title.text = "Add screen — click where to place it"
            hint.text = "Empty cells create a new screen at that position"
        "move":
            title.text = "Move screen %d (%d, %d) — click target" % [_screen_id, _screen_pos.x, _screen_pos.y]
            hint.text = "Yellow cell is the source; empty cells are valid targets"
        "clone":
            title.text = "Clone screen %d — click target" % _screen_id
            hint.text = "Creates a copy at the clicked empty cell"
    grid.position_picked.connect(func(x: int, y: int):
        dialog.queue_free()
        on_pick.call(x, y)
    )
    var cancel: Callable = func():
        dialog.queue_free()
    dialog.get_node("Panel/VBox/Buttons/CancelBtn").pressed.connect(cancel)
    dialog.close_requested.connect(cancel)
    dialog.popup_centered()

func _on_add_screen() -> void:
    _open_minimap("add", _on_add_screen_at)

func _on_add_screen_at(x: int, y: int) -> void:
    _cache_current()
    _screen_id = _store.next_id()
    _store.register(x, y, _screen_id, "")
    _screen_pos = Vector2i(x, y)
    screen_spin.set_value_no_signal(_screen_id)
    _populate_terrain()
    _save_world()
    info_label.text = "Added empty screen %d at (%d, %d)" % [_screen_id, x, y]
    _update_hud()

func _on_delete_screen() -> void:
    if not _store or not _store.is_occupied(_screen_pos.x, _screen_pos.y):
        info_label.text = "Screen %d is not placed — nothing to delete" % _screen_id
        return
    var confirm := ConfirmationDialog.new()
    confirm.dialog_text = "Delete screen %d at (%d, %d)? Its JSON file will be removed too." \
            % [_screen_id, _screen_pos.x, _screen_pos.y]
    confirm.ok_button_text = "Delete"
    confirm.confirmed.connect(func():
        var path := _store.screen_path(_screen_pos.x, _screen_pos.y)
        if not path.is_empty() and FileAccess.file_exists(path):
            DirAccess.remove_absolute(path)
        _store.remove(_screen_pos.x, _screen_pos.y)
        _save_world()
        _populate_terrain()
        _update_hud()
    )
    add_child(confirm)
    confirm.popup_centered()

func _on_move_screen() -> void:
    if not _store or not _store.is_occupied(_screen_pos.x, _screen_pos.y):
        info_label.text = "Screen %d is not placed — add it first" % _screen_id
        return
    _open_minimap("move", _on_move_screen_at)

func _on_move_screen_at(x: int, y: int) -> void:
    var from := _screen_pos
    if _store.move(from, Vector2i(x, y)):
        _screen_pos = Vector2i(x, y)
        _save_world()
        info_label.text = "Moved screen %d to (%d, %d)" % [_screen_id, x, y]
    else:
        push_error("Move failed")
    _update_hud()

func _on_clone_screen() -> void:
    _clipboard = _serialize()
    _open_minimap("clone", _on_clone_screen_at)

func _on_clone_screen_at(x: int, y: int) -> void:
    var new_id := _store.next_id()
    var data: Dictionary = _clipboard.duplicate(true)
    data["screen_id"] = new_id
    var file_name := "screen_%d.json" % new_id
    var path := _store.get_dir().path_join(file_name) if not _store.get_dir().is_empty() else file_name
    var f := FileAccess.open(path, FileAccess.WRITE)
    if f:
        f.store_string(JSON.stringify(data, "\t"))
        f.close()
    _store.register(x, y, new_id, file_name, data)
    _save_world()
    info_label.text = "Cloned screen %d -> %d at (%d, %d)" % [_screen_id, new_id, x, y]
    _update_hud()

func _update_hud() -> void:
    var pos := placement_grid.get_local_mouse_position()
    var tx := clampi(int(pos.x / placement_grid.tile_size), 0, _grid_w - 1)
    var ty := clampi(int(pos.y / placement_grid.tile_size), 0, _grid_h - 1)
    var base := _screen_pos if _is_current_placed() else Vector2i.ZERO
    var wx := base.x * _grid_w + tx
    var wy := base.y * _grid_h + ty
    var brush := _selected_entity_key if not _selected_entity_key.is_empty() else "—"
    var under := get_tile(tx, ty)
    var under_desc := under
    var td := get_tile_data(tx, ty)
    if not td.is_empty():
        under_desc += "  mask:%s elev:%s" % [td.get("sub_tile_mask", "-"), td.get("elevation_tiles", "-")]
    var placed := "placed" if _is_current_placed() else "unplaced"
    hud_label.text = "Screen %d @ (%d, %d) [%s]\nTile (%d, %d)  World (%d, %d)\nBrush: %s\nUnder: %s\n[I/M/H] toggle" % [
        _screen_id, base.x, base.y, placed, tx, ty, wx, wy, brush, under_desc,
    ]

func _input(event: InputEvent) -> void:
    if event is InputEventMouseMotion:
        _update_hud()
    if event is InputEventKey and event.pressed and not event.echo:
        if event.keycode == KEY_H or event.keycode == KEY_I or event.keycode == KEY_M:
            hud_label.visible = not hud_label.visible
            get_viewport().set_input_as_handled()

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
    _mark_dirty()
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
        _mark_dirty()
        placement_grid.queue_redraw()
        info_label.text = "Removed at %d,%d" % [tile_x, tile_y]

func _mark_dirty() -> void:
    if _main and _main.has_method("mark_dirty"):
        _main.mark_dirty()

func _on_clear() -> void:
    _placements.clear()
    _mark_dirty()
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
        "version": "0.3.0",
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
    _cache_current()
    var world := _collect_world_manifest()
    var tiles := _collect_world_tiles(world["screens"])
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_SAVE_FILE
    var zip_mode := _store and not _store.world_package_path.is_empty()
    if zip_mode:
        dialog.add_filter("*.zip", "SSTD World Package")
        dialog.add_filter("*.json", "Screen JSON (legacy)")
    else:
        dialog.add_filter("*.json", "Screen JSON (legacy)")
        dialog.add_filter("*.zip", "SSTD World Package")
    dialog.title = "Save world (package)"
    if _main and _main.has_method("get_default_world_file"):
        dialog.current_file = _main.get_default_world_file()
    if _main and _main.has_method("get_file_dialog_dir"):
        var dialog_dir: String = _main.get_file_dialog_dir()
        if dialog_dir != "":
            dialog.current_dir = dialog_dir
    add_child(dialog)
    dialog.file_selected.connect(func(path: String):
        var wrote := false
        if WorldArchive.is_world_path(path):
            var terrain_ov := {}
            var world_ents: Array = []
            if _main:
                if _main.has_method("get_terrain_overrides_for_save"):
                    terrain_ov = _main.get_terrain_overrides_for_save()
                if _main.has_method("get_world_entity_defs_for_save"):
                    world_ents = _main.get_world_entity_defs_for_save()
            wrote = WorldArchive.save_world(path, world["manifest"], world["screens"], tiles, _collect_entity_overrides(), terrain_ov, world_ents)
        else:
            wrote = _write_plain_json(path, _serialize())
        if not wrote:
            push_error("Cannot write: ", path)
            _log_import("world", "FAIL save: " + path)
            return
        if _store:
            if not WorldArchive.is_world_path(path) and not _is_current_placed():
                _screen_pos = _store.next_free_position()
            if WorldArchive.is_world_path(path):
                _store.world_package_path = path
            _store.register(_screen_pos.x, _screen_pos.y, _screen_id, path.get_file(), _serialize())
            _save_world()
        if _main and _main.has_method("_save_last_map_path"):
            _main._save_last_map_path(path)
        info_label.text = "Saved: %s (%d screens, %d world-shared tiles)" % [path.get_file(), world["screens"].size(), tiles.size()]
        _update_hud()
        _log_import("world", "saved %s (%d screens, %d shared tiles)" % [path, world["screens"].size(), tiles.size()])
    )
    dialog.popup_centered(Vector2i(600, 400))

func _collect_entity_overrides() -> Dictionary:
    return {}

func _collect_world_manifest() -> Dictionary:
    var manifest: Dictionary = {}
    var screens_data: Dictionary = {}
    if _store:
        for pos_key in _store.screens:
            var entry: Dictionary = _store.screens[pos_key]
            var pos: Vector2i = Vector2i(int(entry["x"]), int(entry["y"]))
            var id: int = int(entry["id"])
            manifest[pos_key] = {"x": pos.x, "y": pos.y, "id": id}
            var cached: Dictionary = _store.get_cache(pos.x, pos.y)
            if not cached.is_empty():
                screens_data[id] = cached
    if _is_current_placed():
        screens_data[_screen_id] = _serialize()
    elif _store and _store.screens.is_empty():
        manifest[_store.key_of(_screen_pos.x, _screen_pos.y)] = {"x": _screen_pos.x, "y": _screen_pos.y, "id": _screen_id}
        screens_data[_screen_id] = _serialize()
    return {"manifest": manifest, "screens": screens_data}

func _collect_world_tiles(screens_data: Dictionary) -> Dictionary:
    var tiles: Dictionary = {}
    for id in screens_data:
        var screen_data: Dictionary = screens_data[id]
        for t in screen_data.get("tiles", []):
            var key: String = t.get("terrain", "")
            if key == "" or key == "air" or tiles.has(key):
                continue
            if not WorldArchive.is_allowed_terrain_key(key):
                continue
            var img := get_tile_image(key)
            if img:
                tiles[key] = img
    return tiles

func _write_plain_json(path: String, data: Dictionary) -> bool:
    var f := FileAccess.open(path, FileAccess.WRITE)
    if not f:
        return false
    f.store_string(JSON.stringify(data, "\t"))
    f.close()
    return true

func _on_import_map() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
    var zip_mode := _store and not _store.world_package_path.is_empty()
    if zip_mode:
        dialog.add_filter("*.zip", "SSTD World Package")
        dialog.add_filter("*.json", "Screen JSON (legacy)")
    else:
        dialog.add_filter("*.json", "Screen JSON (legacy)")
        dialog.add_filter("*.zip", "SSTD World Package")
    dialog.title = "Load world package"
    if _main and _main.has_method("get_file_dialog_dir"):
        var dialog_dir: String = _main.get_file_dialog_dir()
        if dialog_dir != "":
            dialog.current_dir = dialog_dir
    add_child(dialog)
    dialog.file_selected.connect(_on_import_file_guarded)
    dialog.popup_centered(Vector2i(600, 400))

# Guard interactive load against dirty state (#60): Save/Discard/Cancel before
# replacing the current world. Await-safe; tests call `_on_import_file` directly.
func _on_import_file_guarded(path: String) -> void:
    if _main and _main.has_method("_confirm_dirty_or_save"):
        if await _main._confirm_dirty_or_save() == 0:
            _log_import("placement", "import cancelled (dirty save prompt): " + path)
            return
    _on_import_file(path)

func _on_import_file(path: String) -> void:
    _log_import("placement", "import start: " + path)
    var pkg := WorldArchive.resolve_world_pointer(path)
    if not pkg.is_empty():
        _log_import("placement", "resolved world pointer %s -> %s" % [path, pkg])
        _load_world_package(pkg)
        return
    var parsed := _load_screen_file(path)
    if parsed.is_empty() or not parsed.has("tiles"):
        push_error("Invalid screen file")
        _log_import("placement", "FAIL invalid screen file: " + path + " (no world pointer, no tiles key)")
        return
    _apply_screen(parsed)
    if parsed.has("screen_id"):
        _screen_id = int(parsed["screen_id"])
        screen_spin.set_value_no_signal(_screen_id)
    _sync_position_from_id()
    if _store:
        if not _is_current_placed():
            _screen_pos = _store.next_free_position()
        _store.register(_screen_pos.x, _screen_pos.y, _screen_id, path.get_file(), parsed)
        _save_world()
    info_label.text = "Imported %d tiles, %d entities" % [parsed.get("tiles", []).size(), _placements.size()]
    _update_hud()
    _log_import("placement", "loaded legacy screen %s (%d tiles)" % [path, parsed["tiles"].size()])

func _log_import(kind: String, msg: String) -> void:
    print("[editor/%s] %s" % [kind, msg])

func _load_world_package(path: String) -> void:
    var data := WorldArchive.load_world(path)
    if not data.get("ok", false):
        push_error("Invalid world package: ", path)
        _log_import("world", "FAIL load world package: " + path)
        return
    if _store:
        _store.apply_world_data(data)
        _store.world_package_path = path
        _save_world()
    if _main and _main.has_method("on_world_loaded"):
        _main.on_world_loaded(data)
    _restore_tile_bank(data["tile_images"])
    var current_id: int = _screen_id
    if not data["screens"].has(current_id):
        var first_id: Array = data["screens"].keys()
        current_id = int(first_id[0]) if not first_id.is_empty() else _screen_id
    _screen_id = current_id
    screen_spin.set_value_no_signal(current_id)
    _sync_position_from_id()
    if _is_current_placed():
        _restore_screen()
    else:
        _populate_terrain()
    if _main and _main.has_method("_save_last_map_path"):
        _main._save_last_map_path(path)
    info_label.text = "Loaded world: %s (%d screens, %d world-shared tiles)" % [path.get_file(), data["screens"].size(), data["tile_images"].size()]
    _update_hud()
    _log_import("world", "loaded package %s: %d screens, %d shared tiles, current screen %d" % [path, data["screens"].size(), data["tile_images"].size(), current_id])

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
