extends Control

const _GodotTileSetGrouping := preload("res://scripts/resources/godot_tileset_grouping.gd")
const _ScreenMinimapDialog := preload("res://scenes/screen_minimap_dialog.tscn")
const _TilePalette := preload("res://scripts/tile_palette.gd")
const WORLD_PATH := "res://world.json"

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

var _store: ScreenStore
var _screen_pos: Vector2i = Vector2i.ZERO
var _clipboard: Dictionary = {}

var _hud_dragging: bool = false
var _hud_drag_grab: Vector2 = Vector2.ZERO
var _hud_start_pos: Vector2 = Vector2.ZERO

const STAMP_CELL := 32
const STAMP_TOLERANCE := 4.0
const _BUSY_YIELD_EVERY := 128  # yield + repaint once per N heavy-loop iterations
var _stamp_active: bool = false
var _stamp_image: Image
var _stamp_texture: ImageTexture
var _stamp_origin: Vector2i = Vector2i(-1, -1)
var _stamp_seq: int = 0
var _stamp_catalog: Array[Dictionary] = []
var _stamp_fp_index: Dictionary = {}  # FNV(int) -> Array[int] of catalog indices with that canonical-coarse fingerprint
var _stamp_basename: String = ""
var _stamp_maps: Array[Dictionary] = []
var _tile_images: Dictionary = {}  # tile key -> Image (resident in memory; reused across the world)
var _tile_texture_cache: Dictionary = {}  # tile key -> ImageTexture
var _ready_done := false  # set when _ready finishes (async catalog load) — used by tests

@onready var tile_palette: _TilePalette = $LeftPanel/TilePalette
@onready var tile_grid: Control = $RightPanel/Scroll/TileGrid
@onready var screen_spin: SpinBox = $LeftPanel/TopBar/ScreenSpin
@onready var paint_btn: CheckButton = $LeftPanel/TopBar/PaintBtn
@onready var erase_btn: CheckButton = $LeftPanel/TopBar/EraseBtn
@onready var save_btn: Button = $LeftPanel/BottomBar/SaveBtn
@onready var import_btn: Button = $LeftPanel/BottomBar/ImportBtn
@onready var export_btn: Button = $LeftPanel/BottomBar/ExportBtn
@onready var prune_btn: Button = $LeftPanel/BottomBar/PruneBtn
@onready var stamp_btn: Button = $LeftPanel/BottomBar/StampBtn
@onready var collision_btn: CheckButton = $LeftPanel/TopBar/CollisionBtn
@onready var cursor_label: Label = $LeftPanel/BottomBar/CursorLabel
@onready var info_label: Label = $LeftPanel/BottomBar/InfoLabel
@onready var hud_label: Label = $HUD
@onready var add_btn: Button = $LeftPanel/BottomBar/AddBtn
@onready var delete_btn: Button = $LeftPanel/BottomBar/DeleteBtn
@onready var move_btn: Button = $LeftPanel/BottomBar/MoveBtn
@onready var clone_btn: Button = $LeftPanel/BottomBar/CloneBtn

func _set_busy(busy: bool) -> void:
    if DisplayServer.get_name() == "headless":
        return
    DisplayServer.cursor_set_shape(DisplayServer.CURSOR_BUSY if busy else DisplayServer.CURSOR_ARROW)

func _ready() -> void:
    hud_label.visible = false
    _load_defaults()
    _refresh_tile_palette()
    _load_tile_sets()
    _rebuild_terrain_tilesets()
    _set_busy(true)
    await _load_stamp_catalog()
    _set_busy(false)
    _refresh_tile_palette()
    # main._ready may auto-load the last world while this _ready was suspended
    # on the stamp-catalog await; do not wipe that grid with the default one.
    if _tiles.is_empty():
        _populate_grid()

    paint_btn.toggled.connect(_on_paint_mode)
    erase_btn.toggled.connect(_on_erase_mode)
    save_btn.pressed.connect(_on_save)
    import_btn.pressed.connect(_on_import)
    export_btn.pressed.connect(_on_export)
    prune_btn.pressed.connect(_on_prune_duplicates)
    stamp_btn.pressed.connect(_on_stamp_import)
    screen_spin.value_changed.connect(_on_screen_changed)
    tile_palette.tile_picked.connect(_on_tile_picked)
    collision_btn.toggled.connect(_on_toggle_collision)
    add_btn.pressed.connect(_on_add_screen)
    delete_btn.pressed.connect(_on_delete_screen)
    move_btn.pressed.connect(_on_move_screen)
    clone_btn.pressed.connect(_on_clone_screen)
    tile_grid.map_editor = self

    paint_btn.button_pressed = true

    _ready_done = true

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
    # Legacy alias: the terrain list and tileset list are now one visual palette.
    _refresh_tile_palette()

# Unified thumbnail grid (tile_palette): populations terrain swatches, the
# world-shared tile bank, and stamp/tileset groupings.
func _refresh_tile_palette() -> void:
    if tile_palette == null:
        return
    tile_palette.populate(_terrain_types, _tile_images, _tile_set_groupings, _group_preview_image)

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
        ts.tags.assign(["terrain_default", "category:terrain"])
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
    print("[editor/map] populated default grid (%d tiles)" % _tiles.size())
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

func get_tile_image(key: String) -> Image:
    if key.is_empty():
        return null
    if _tile_images.has(key):
        return _tile_images[key]
    for entry in _stamp_catalog:
        if entry["key"] == key:
            var img: Image = entry["cell"]
            _tile_images[key] = img
            return img
    var abs := ProjectSettings.globalize_path("res://assets/tiles/%s_%dx%d.png" % [key, STAMP_CELL, STAMP_CELL])
    if FileAccess.file_exists(abs):
        var img := Image.new()
        if img.load(abs) == OK:
            _tile_images[key] = _as_rgba8(img)
            return _tile_images[key]
    return null

static func _as_rgba8(img: Image) -> Image:
    if img == null or img.get_format() == Image.FORMAT_RGBA8:
        return img
    img.convert(Image.FORMAT_RGBA8)
    return img

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
        var b64: String = embedded[key]
        var img := Image.new()
        if img.load_png_from_buffer(Marshalls.base64_to_raw(b64)) != OK:
            continue
        _register_tile_image(str(key), _as_rgba8(img))

func _restore_tile_bank(bank: Dictionary) -> void:
    if bank.is_empty():
        return
    for key in bank:
        var img: Image = bank[key]
        if img:
            _register_tile_image(str(key), _as_rgba8(img))

func _register_tile_image(key: String, img: Image) -> void:
    _tile_images[key] = img
    _tile_texture_cache.erase(key)
    var abs_dir := ProjectSettings.globalize_path("res://assets/tiles")
    DirAccess.make_dir_recursive_absolute(abs_dir)
    if not FileAccess.file_exists(abs_dir + "/%s_%dx%d.png" % [key, STAMP_CELL, STAMP_CELL]):
        img.save_png(abs_dir + "/%s_%dx%d.png" % [key, STAMP_CELL, STAMP_CELL])
    var idx := -1
    for i in _stamp_catalog.size():
        if _stamp_catalog[i]["key"] == key:
            idx = i
            break
    if idx < 0:
        idx = _stamp_catalog.size()
        _stamp_catalog.append({"key": key, "cell": img})
        _index_stamp_entry(idx)
    else:
        _stamp_catalog[idx]["cell"] = img
        _rebuild_stamp_index()

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
    wrapper.tags.assign(["terrain"] if is_terrain else ["slope"])
    for i in range(min(valid.size(), w * h)):
        var id := valid[i]
        wrapper.tile_coords.append({"col": id.x, "row": id.y, "local_x": i % w, "local_y": i / w})
    _tile_set_groupings.append(wrapper)

func _refresh_tile_set_palette() -> void:
    # Legacy alias: the tile-set list is now part of the visual tile palette.
    _refresh_tile_palette()

# Builds a composite thumbnail for a TileSetGrouping (stamp_* or wrapped
# Godot tile-set): stamps each referenced cell into a w×h preview image.
func _group_preview_image(ts) -> ImageTexture:
    if ts == null:
        return _empty_texture()
    var image_size := maxi(ts.width_tiles, 1) * STAMP_CELL
    var preview := Image.create(
        image_size, image_size, false, Image.FORMAT_RGBA8)
    preview.fill(Color(0, 0, 0, 0))

    var blit_cell := func(cell_key: String, lx: int, ly: int) -> void:
        var cell: Image = get_tile_image(cell_key)
        if cell == null:
            return
        var src := cell.duplicate()
        preview.blit_rect(
            src, Rect2i(0, 0, src.get_width(), src.get_height()),
            Vector2i(lx * STAMP_CELL, ly * STAMP_CELL))

    if "tile_coords" in ts and not (ts.tile_coords as Array).is_empty():
        for tc in ts.tile_coords:
            blit_cell.call(tc.get("terrain_key", ""), tc.get("local_x", 0), tc.get("local_y", 0))
    else:
        for cell in ts.tiles:
            blit_cell.call(cell.get("terrain_key", ""), cell.get("local_x", 0), cell.get("local_y", 0))
    return ImageTexture.create_from_image(preview)

func _empty_texture() -> ImageTexture:
    var img := Image.create(STAMP_CELL, STAMP_CELL, false, Image.FORMAT_RGBA8)
    img.fill(Color(0.2, 0.2, 0.2, 1))
    return ImageTexture.create_from_image(img)

func _on_tile_picked(kind: String, key: String) -> void:
    match kind:
        "terrain":
            _selected_terrain = key
            _selected_tile_set_key = "terrain_" + key
        "tile":
            _selected_terrain = key
            _selected_tile_set_key = key
        "group":
            _selected_terrain = ""
            _selected_tile_set_key = key
    _update_info()

func _on_toggle_collision(visible: bool) -> void:
    tile_grid.show_collision = visible
    tile_grid.queue_redraw()

func select_tile_set(key: String) -> void:
    tile_palette.select_key(key)
    _selected_tile_set_key = key
    _selected_terrain = ""
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

func _merge_placed_entities(data: Dictionary) -> Dictionary:
    if _is_current_placed():
        var prev: Dictionary = _store.get_cache(_screen_pos.x, _screen_pos.y)
        if prev.has("placed_entities") and not (prev["placed_entities"] as Array).is_empty():
            data["placed_entities"] = prev["placed_entities"]
    return data

func _cache_current() -> void:
    if _is_current_placed():
        _store.set_cache(_screen_pos.x, _screen_pos.y, _merge_placed_entities(_serialize()))

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

func _restore_screen() -> void:
    _sync_position_from_id()
    if _is_current_placed():
        var cached: Dictionary = _store.get_cache(_screen_pos.x, _screen_pos.y)
        var parsed: Dictionary = cached if not cached.is_empty() \
                else _load_screen_file(_store.screen_path(_screen_pos.x, _screen_pos.y))
        if not parsed.is_empty() and parsed.has("tiles"):
            _apply_screen(parsed)
            print("[editor/map] restored screen %d (%d tiles)" % [_screen_id, parsed["tiles"].size()])
            return
    _populate_grid()

func _save_world() -> void:
    if _store:
        _store.save_world(WORLD_PATH)

# Journal-style log line: every persist/load failure or step is recorded so an
# incident can be re-traced from the engine log alone (see AGENTS: log requirement).
func _log_import(kind: String, msg: String) -> void:
    print("[editor/%s] %s" % [kind, msg])

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
    _populate_grid()
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
        _populate_grid()
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
    _clipboard = _merge_placed_entities(_serialize())
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
    var pos := tile_grid.get_local_mouse_position()
    var tx := clampi(int(pos.x / tile_grid.tile_size), 0, _grid_w - 1)
    var ty := clampi(int(pos.y / tile_grid.tile_size), 0, _grid_h - 1)
    var base := _screen_pos if _is_current_placed() else Vector2i.ZERO
    var wx := base.x * _grid_w + tx
    var wy := base.y * _grid_h + ty
    var brush := "—"
    if not _selected_tile_set_key.is_empty():
        var ts = _find_tile_set(_selected_tile_set_key)
        if ts:
            brush = ts.display_name
    var under := get_tile(tx, ty)
    var under_desc := under
    var td := get_tile_data(tx, ty)
    if not td.is_empty():
        under_desc += "  mask:%s elev:%s" % [td.get("sub_tile_mask", "-"), td.get("elevation_tiles", "-")]
    var placed := "placed" if _is_current_placed() else "unplaced"
    hud_label.text = "Screen %d @ (%d, %d) [%s]\nTile (%d, %d)  World (%d, %d)\nBrush: %s\nUnder: %s\n[I/M/H] toggle · drag to move" % [
        _screen_id, base.x, base.y, placed, tx, ty, wx, wy, brush, under_desc,
    ]

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
        "version": "0.3.0",
        "screen_id": _screen_id,
        "width_tiles": _grid_w,
        "height_tiles": _grid_h,
        "tile_width_px": _tile_w,
        "tile_height_px": _tile_h,
        "elevation_floor_tiles": 0,
        "elevation_ceiling_tiles": 4,
        "tiles": tiles_out,
        "placed_entities": [],
        "stamp_maps": _stamp_maps,
    }

# Collects the whole world for packaging: manifest + per-screen data (world-shared
# tiles are resolved separately via the shared tile bank below).
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
    # ensure current screen is flushed
    if _is_current_placed():
        screens_data[_screen_id] = _merge_placed_entities(_serialize())
    elif _store and _store.screens.is_empty():
        manifest[_store.key_of(_screen_pos.x, _screen_pos.y)] = {"x": _screen_pos.x, "y": _screen_pos.y, "id": _screen_id}
        screens_data[_screen_id] = _serialize()
    return {"manifest": manifest, "screens": screens_data}

# Collects the world-shared tile bank: EVERY distinct tile image referenced by the world.
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

func _on_save() -> void:
    _cache_current()
    var world := _collect_world_manifest()
    var tiles := _collect_world_tiles(world["screens"])
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_SAVE_FILE
    dialog.add_filter("*.zip", "SSTD World Package")
    dialog.add_filter("*.json", "Screen JSON (legacy)")
    dialog.title = "Save world (package)"
    dialog.current_file = "world.zip"
    add_child(dialog)
    dialog.file_selected.connect(func(path: String):
        var wrote := false
        if WorldArchive.is_world_path(path):
            wrote = WorldArchive.save_world(path, world["manifest"], world["screens"], tiles)
        else:
            var f := FileAccess.open(path, FileAccess.WRITE)
            if f:
                f.store_string(JSON.stringify(_merge_placed_entities(_serialize()), "\t"))
                f.close()
                wrote = true
        if not wrote:
            push_error("Cannot write: ", path)
            _log_import("world", "FAIL save: " + path)
            return
        if _store:
            if not WorldArchive.is_world_path(path) and not _is_current_placed():
                _screen_pos = _store.next_free_position()
            if WorldArchive.is_world_path(path):
                _store.world_package_path = path
            _store.register(_screen_pos.x, _screen_pos.y, _screen_id, path.get_file(), _merge_placed_entities(_serialize()))
            _save_world()
        if _main and _main.has_method("_save_last_map_path"):
            _main._save_last_map_path(path)
        info_label.text = "Saved: %s (%d screens, %d world-shared tiles)" % [path.get_file(), world["screens"].size(), tiles.size()]
        _update_hud()
        _log_import("world", "saved %s (%d screens, %d shared tiles)" % [path, world["screens"].size(), tiles.size()])
    )
    dialog.popup_centered(Vector2i(600, 400))

func _write_plain_json(path: String, data: Dictionary) -> bool:
    var f := FileAccess.open(path, FileAccess.WRITE)
    if not f:
        return false
    f.store_string(JSON.stringify(data, "\t"))
    f.close()
    return true

func _on_import() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
    dialog.add_filter("*.zip", "SSTD World Package")
    dialog.add_filter("*.json", "Screen JSON (legacy)")
    dialog.title = "Load world package"
    add_child(dialog)
    dialog.file_selected.connect(_on_import_file)
    dialog.popup_centered(Vector2i(600, 400))

func _on_import_file(path: String) -> void:
    _log_import("map", "import start: " + path)
    # A path may be a .zip world directly, or a world.json *pointer* to one.
    var pkg := WorldArchive.resolve_world_pointer(path)
    if not pkg.is_empty():
        _log_import("map", "resolved world pointer %s -> %s" % [path, pkg])
        _load_world_package(pkg)
        return
    var parsed := _load_screen_file(path)
    if parsed.is_empty() or not parsed.has("tiles"):
        push_error("Invalid screen file")
        _log_import("map", "FAIL invalid screen file: " + path + " (no world pointer, no tiles key)")
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
    if _main and _main.has_method("_save_last_map_path"):
        _main._save_last_map_path(path)
    info_label.text = "Loaded: %s (%d tiles)" % [path.get_file(), parsed["tiles"].size()]
    _update_hud()
    _log_import("map", "loaded legacy screen %s (%d tiles)" % [path, parsed["tiles"].size()])

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
    _restore_tile_bank(data["tile_images"])
    _refresh_tile_palette()
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
        _populate_grid()
    if _main and _main.has_method("_save_last_map_path"):
        _main._save_last_map_path(path)
    info_label.text = "Loaded world: %s (%d screens, %d world-shared tiles)" % [path.get_file(), data["screens"].size(), data["tile_images"].size()]
    _update_hud()
    _log_import("world", "loaded package %s: %d screens, %d shared tiles, current screen %d" % [path, data["screens"].size(), data["tile_images"].size(), current_id])

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

func _load_stamp_catalog() -> void:
    _stamp_catalog.clear()
    var abs_dir := ProjectSettings.globalize_path("res://assets/tiles")
    if not DirAccess.dir_exists_absolute(abs_dir):
        return
    var dir := DirAccess.open(abs_dir)
    if not dir:
        return
    dir.list_dir_begin()
    var fname := dir.get_next()
    var processed := 0
    while fname != "":
        processed += 1
        if processed % _BUSY_YIELD_EVERY == 0:
            info_label.text = "Loading stamp catalog… %d files" % processed
            await get_tree().process_frame
        if fname.begins_with("stamp_") and fname.ends_with("_%dx%d.png" % [STAMP_CELL, STAMP_CELL]):
            var key := fname.get_basename().rsplit("_%dx%d" % [STAMP_CELL, STAMP_CELL], false)[0]
            var img := Image.new()
            if img.load(abs_dir + "/" + fname) == OK:
                _stamp_catalog.append({"key": key, "cell": _as_rgba8(img)})
                _tile_images[key] = _stamp_catalog.back()["cell"]
                var idx := key.trim_prefix("stamp_").to_int()
                if idx > _stamp_seq:
                    _stamp_seq = idx
        fname = dir.get_next()
    dir.list_dir_end()
    await _rebuild_stamp_index()

func _rebuild_stamp_index() -> void:
    _stamp_fp_index.clear()
    for i in _stamp_catalog.size():
        if (i + 1) % _BUSY_YIELD_EVERY == 0:
            info_label.text = "Indexing stamps… %d/%d" % [i + 1, _stamp_catalog.size()]
            await get_tree().process_frame
        _index_stamp_entry(i)

# 2-bit quantized coarse signature (flip-canonicalized). The exact 8-bit coarse
# fingerprint used for the stamp index changes with any 4x4 block's mean, so
# near-identical tiles (e.g. many near-black cells) never share a bucket there.
# Quantizing each channel to its top 2 bits collapses visually-identical tiles
# into one signature for the prune/merge pass.
func _similarity_sig(img: Image) -> PackedByteArray:
    var c := _coarse_bytes(img)
    var q := PackedByteArray()
    q.resize(c.size())
    for i in c.size():
        q[i] = c[i] >> 6
    var best := q
    for h in [false, true]:
        for v in [false, true]:
            if h or v:
                var f := _flip_coarse(q, h, v)
                if _bytes_less(f, best):
                    best = f
    return best

# Lowest numeric stamp id wins so a group keeps its smallest tile id.
func _catalog_rank(idx: int) -> int:
    var key: String = _stamp_catalog[idx]["key"]
    if key.begins_with("stamp_"):
        var n := key.trim_prefix("stamp_").to_int()
        if n > 0:
            return n
    return 0x7fffffff

# Is `cell` visually the same as some flip of `base_img` (XOR-style exact diff)?
# Returns {"flip_h", "flip_v"} on match within STAMP_TOLERANCE, else {}.
func _flip_of(base_img: Image, cell: Image) -> Dictionary:
    var cand := _cell_bytes(cell, false, false)
    var best_diff := 0x7fffffff
    var best := {}
    for f in [[false, false], [true, false], [false, true], [true, true]]:
        var d := _diff(cand, _cell_bytes(base_img, f[0], f[1]))
        if d < best_diff:
            best_diff = d
            best = {"flip_h": f[0], "flip_v": f[1]}
    if best_diff <= STAMP_TOLERANCE:
        return best
    return {}

# Opt-in cleanup (never run at boot — that would feel like a hang). Groups tiles
# by _similarity_sig, verifies each group with the exact diff metric, then merges
# every duplicate onto the lowest stamp id: rewrites all references, drops the
# redundant bank entries + disk PNGs, and journals the operation.
func _on_prune_duplicates() -> void:
    if _stamp_catalog.is_empty():
        info_label.text = "Nothing to prune — tile catalog is empty"
        return
    _set_busy(true)
    var groups: Dictionary = {}
    var palette_before: int = tile_palette._entries.size()
    for i in _stamp_catalog.size():
        var sig := _similarity_sig(_as_rgba8(_stamp_catalog[i]["cell"]))
        var h := _fp_hash(sig)
        if not groups.has(h):
            groups[h] = {"members": []}
        groups[h]["members"].append(i)
    var merge_map: Dictionary = {}  # dup key -> {"key", "flip_h", "flip_v"}
    var dup_keys: Array = []
    for h in groups:
        var members: Array = groups[h]["members"]
        if members.size() < 2:
            continue
        members.sort_custom(func(a, b): return _catalog_rank(a) < _catalog_rank(b))
        var rep_idx: int = members[0]
        var rep_key: String = _stamp_catalog[rep_idx]["key"]
        var rep_img: Image = _as_rgba8(_stamp_catalog[rep_idx]["cell"])
        for i in members.slice(1):
            var cand_key: String = _stamp_catalog[i]["key"]
            if cand_key == rep_key:
                continue
            var flip := _flip_of(rep_img, _as_rgba8(_stamp_catalog[i]["cell"]))
            if flip.is_empty():
                continue
            merge_map[cand_key] = {"key": rep_key, "flip_h": flip["flip_h"], "flip_v": flip["flip_v"]}
            dup_keys.append(cand_key)
    if dup_keys.is_empty():
        _set_busy(false)
        info_label.text = "Prune: no duplicates (%d tiles scanned)" % _stamp_catalog.size()
        _log_import("prune", "no duplicates among %d tiles" % _stamp_catalog.size())
        var dlg := AcceptDialog.new()
        dlg.title = "Prune Duplicates"
        dlg.dialog_text = "No duplicates found.\n%d tiles scanned, palette unchanged." % _stamp_catalog.size()
        add_child(dlg)
        dlg.popup_centered()
        return
    _rewrite_references(merge_map)
    var removed_pngs := _drop_tiles(dup_keys)
    await _rebuild_stamp_index()
    _refresh_tile_palette()
    tile_grid.reload_textures()
    tile_grid.queue_redraw()
    _set_busy(false)
    var palette_after: int = tile_palette._entries.size()
    var first := " -> ".join([dup_keys[0], merge_map[dup_keys[0]]["key"]])
    info_label.text = "Pruned %d duplicate tiles (%s); bank now %d tiles" % [dup_keys.size(), first, _tile_images.size()]
    _log_import("prune", "merged %d duplicates onto lowest-id tiles (first %s), removed %d disk PNGs, bank=%d" % [dup_keys.size(), first, removed_pngs, _tile_images.size()])
    var summary := "Merged %d duplicate tiles onto lowest-id tiles.\n\n" % dup_keys.size()
    summary += "Example: %s\n\n" % first
    summary += "Removed %d redundant disk PNGs.\n" % removed_pngs
    summary += "Tile bank: %d tiles (was %d).\n" % [_tile_images.size(), _tile_images.size() + dup_keys.size()]
    summary += "Palette entries: %d (was %d).\n" % [palette_after, palette_before]
    summary += "All map/branch references now point at the surviving tiles."
    var dlg_ok := AcceptDialog.new()
    dlg_ok.title = "Prune Duplicates"
    dlg_ok.dialog_text = summary
    add_child(dlg_ok)
    dlg_ok.popup_centered()

# Rewrites every terrain reference that points at a merged duplicate so the map,
# cached screens, and stamp brushes all use the canonical lowest-id tile. Flip
# flags are XOR-combined (the duplicate may itself have been a flipped variant).
func _rewrite_references(merge_map: Dictionary) -> void:
    for cell_key in _tiles:
        var k: String = _tiles[cell_key]
        if merge_map.has(k):
            var m: Dictionary = merge_map[k]
            _tiles[cell_key] = m["key"]
            var td: Dictionary = _tile_data.get(cell_key, {})
            if not td.is_empty():
                td["flip_h"] = bool(td.get("flip_h", false)) != bool(m["flip_h"])
                td["flip_v"] = bool(td.get("flip_v", false)) != bool(m["flip_v"])
                _tile_data[cell_key] = td
    if _store:
        for pos_key in _store.cache:
            var screen_data: Dictionary = _store.cache[pos_key]
            var tiles_arr: Array = screen_data.get("tiles", [])
            for t in tiles_arr:
                if t.has("terrain") and merge_map.has(t["terrain"]):
                    var m: Dictionary = merge_map[t["terrain"]]
                    t["terrain"] = m["key"]
                    if t.has("flip_h"):
                        t["flip_h"] = bool(t["flip_h"]) != bool(m["flip_h"])
                    if t.has("flip_v"):
                        t["flip_v"] = bool(t["flip_v"]) != bool(m["flip_v"])
            screen_data["tiles"] = tiles_arr
    for ts in _tile_set_groupings:
        for tile_entry in ts.tiles:
            if tile_entry.has("terrain_key") and merge_map.has(tile_entry["terrain_key"]):
                var m: Dictionary = merge_map[tile_entry["terrain_key"]]
                tile_entry["terrain_key"] = m["key"]
                tile_entry["flip_h"] = bool(tile_entry.get("flip_h", false)) != bool(m["flip_h"])
                tile_entry["flip_v"] = bool(tile_entry.get("flip_v", false)) != bool(m["flip_v"])

# Removes merged duplicate keys from the in-memory bank/catalog and deletes the
# redundant on-disk PNGs. Returns how many disk files were removed.
func _drop_tiles(dup_keys: Array) -> int:
    var drop: Dictionary = {}
    for k in dup_keys:
        drop[k] = true
    var removed_pngs := 0
    for key in dup_keys:
        _tile_images.erase(key)
        _tile_texture_cache.erase(key)
        var abs := ProjectSettings.globalize_path("res://assets/tiles/%s_%dx%d.png" % [key, STAMP_CELL, STAMP_CELL])
        if FileAccess.file_exists(abs):
            if DirAccess.remove_absolute(abs) == OK:
                removed_pngs += 1
    var kept: Array = []
    for entry in _stamp_catalog:
        if not drop.has(entry["key"]):
            kept.append(entry)
    _stamp_catalog.assign(kept)
    return removed_pngs

func _on_stamp_import() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
    dialog.add_filter("*.png", "PNG (stamp source)")
    dialog.title = "Import stamp image"
    add_child(dialog)
    dialog.file_selected.connect(_on_stamp_import_file)
    dialog.popup_centered(Vector2i(600, 400))

func _on_stamp_import_file(path: String) -> void:
    var img := Image.new()
    if img.load(path) != OK:
        push_error("Failed to load stamp image: ", path)
        return
    _stamp_image = img
    _stamp_texture = ImageTexture.create_from_image(img)
    _stamp_active = true
    _stamp_origin = Vector2i(-1, -1)
    _stamp_basename = path.get_file().get_basename().strip_edges()
    info_label.text = "Stamp loaded: %s (%dx%d). Click+hold on grid to define stamp origin, release to commit." \
            % [path.get_file(), img.get_width(), img.get_height()]
    tile_grid.queue_redraw()

func _stamp_grid_input(event: InputEvent) -> void:
    if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
        if event.pressed:
            _stamp_origin = tile_grid.pixel_to_tile(event.position)
            tile_grid.queue_redraw()
            get_viewport().set_input_as_handled()
        elif _stamp_origin.x >= 0:
            _commit_stamp(_stamp_origin)
            _stamp_origin = Vector2i(-1, -1)
            get_viewport().set_input_as_handled()

func _commit_stamp(origin: Vector2i) -> void:
    if not _stamp_image:
        return
    _set_busy(true)
    var w: int = _stamp_image.get_width()
    var h: int = _stamp_image.get_height()
    var col_count: int = ceili(float(w) / STAMP_CELL)
    var row_count: int = ceili(float(h) / STAMP_CELL)
    var total := 0
    var cells: Array[Dictionary] = []
    var iter_count := col_count * row_count
    var processed := 0
    for ry in row_count:
        for cx in col_count:
            processed += 1
            if iter_count > 256 and processed % 64 == 0:
                info_label.text = "Stamping… %d/%d cells" % [processed, iter_count]
                await get_tree().process_frame
            var gx := origin.x + cx
            var gy := origin.y + ry
            if gx < 0 or gx >= _grid_w or gy < 0 or gy >= _grid_h:
                continue
            total += 1
            var cell := Image.create(STAMP_CELL, STAMP_CELL, false, Image.FORMAT_RGBA8)
            var sx := cx * STAMP_CELL
            var sy := ry * STAMP_CELL
            var uw: int = min(STAMP_CELL, w - sx)
            var uh: int = min(STAMP_CELL, h - sy)
            if uw > 0 and uh > 0:
                cell.blit_rect(_stamp_image, Rect2i(sx, sy, uw, uh), Vector2i.ZERO)
            if _has_ink(cell):
                var entry := _ensure_tile(cell)
                _tiles[_key(gx, gy)] = entry["key"]
                _tile_data[_key(gx, gy)] = {
                    "flip_h": entry["flip_h"],
                    "flip_v": entry["flip_v"],
                    "sub_tile_mask": 15,
                    "elevation_tiles": 0,
                    "z_depth": 0,
                }
                cells.append({
                    "local_x": cx, "local_y": ry,
                    "gx": gx, "gy": gy,
                    "terrain_key": entry["key"],
                    "flip_h": entry["flip_h"],
                    "flip_v": entry["flip_v"],
                    "src_col": cx, "src_row": ry,
                })
    _refresh_palette()
    tile_grid.reload_textures()
    tile_grid.queue_redraw()
    _set_busy(false)
    if not cells.is_empty():
        _register_stamp_brush(cells, col_count, row_count)
        _record_stamp_map(cells, col_count, row_count)
    info_label.text = "Stamped %d cells, %d unique tiles (from %dx%d px image)" \
            % [total, _stamp_catalog.size(), w, h]

func _register_stamp_brush(cells: Array[Dictionary], cols: int, rows: int) -> void:
    if _stamp_basename.is_empty():
        return
    # Reuse an existing grouping with the same key instead of appending a
    # duplicate. Re-stamping the same image at a different spot must yield one
    # brush, not two (issue #46).
    var key := "stamp_%s" % _stamp_basename
    var ts = _find_tile_set(key)
    if ts == null:
        ts = TileSetGrouping.new()
        ts.key = key
        _tile_set_groupings.append(ts)
    ts.display_name = "Stamp %s" % _stamp_basename
    ts.width_tiles = cols
    ts.height_tiles = rows
    ts.tiles = []
    for cell in cells:
        ts.tiles.append({
            "local_x": cell["local_x"],
            "local_y": cell["local_y"],
            "terrain_key": cell["terrain_key"],
            "sub_tile_mask": 15,
            "elevation_tiles": 0,
            "z_depth": 0,
            "flip_h": cell["flip_h"],
            "flip_v": cell["flip_v"],
            "source_col": cell["src_col"],
            "source_row": cell["src_row"],
        })
    ts.tags.assign(["stamp", "terrain"])
    _refresh_tile_set_palette()
    select_tile_set(ts.key)

func _record_stamp_map(cells: Array[Dictionary], cols: int, rows: int) -> void:
    if _stamp_basename.is_empty():
        return
    var meta: Dictionary = {
        "stamp_key": "stamp_%s" % _stamp_basename,
        "grid_cols": cols,
        "grid_rows": rows,
        "cell_size_px": STAMP_CELL,
        "cells": [],
    }
    for cell in cells:
        meta["cells"].append({
            "tile_id": cell["terrain_key"],
            "local_x": cell["local_x"],
            "local_y": cell["local_y"],
            "src_col": cell["src_col"],
            "src_row": cell["src_row"],
            "flip_h": bool(cell["flip_h"]),
            "flip_v": bool(cell["flip_v"]),
        })
    _stamp_maps.append(meta)

func _has_ink(cell: Image) -> bool:
    _as_rgba8(cell)
    var data := cell.get_data()
    for i in range(0, data.size(), 4):
        if data[i + 3] > 0:
            return true
    return false

func _ensure_tile(cell: Image) -> Dictionary:
    var match := _find_match(cell)
    if match:
        return match
    _stamp_seq += 1
    var key := "stamp_%d" % _stamp_seq
    var img: Image = cell.duplicate()
    var abs_dir := ProjectSettings.globalize_path("res://assets/tiles")
    DirAccess.make_dir_recursive_absolute(abs_dir)
    img.save_png(abs_dir + "/%s_%dx%d.png" % [key, STAMP_CELL, STAMP_CELL])
    _terrain_types.append({
        "key": key,
        "display_name": key,
        "color_hex": _average_hex(cell),
    })
    var catalog_index := _stamp_catalog.size()
    _stamp_catalog.append({"key": key, "cell": cell, "flip_h": false, "flip_v": false})
    _index_stamp_entry(catalog_index)
    return {"key": key, "flip_h": false, "flip_v": false}

func _index_stamp_entry(idx: int) -> void:
    var cell: Image = _stamp_catalog[idx]["cell"]
    var h := _fp_hash(_canonical_coarse(cell))
    if not _stamp_fp_index.has(h):
        _stamp_fp_index[h] = []
    _stamp_fp_index[h].append(idx)

func _average_hex(cell: Image) -> String:
    _as_rgba8(cell)
    var r := 0.0
    var g := 0.0
    var b := 0.0
    var n := 0
    var data := cell.get_data()
    for i in range(0, data.size(), 4):
        r += data[i]
        g += data[i + 1]
        b += data[i + 2]
        n += 1
    if n == 0:
        return "#808080"
    r /= n
    g /= n
    b /= n
    return "#%02x%02x%02x" % [int(r), int(g), int(b)]

func _cell_bytes(cell: Image, flip_h: bool, flip_v: bool) -> PackedByteArray:
    _as_rgba8(cell)
    var res := PackedByteArray()
    res.resize(STAMP_CELL * STAMP_CELL * 4)
    var data := cell.get_data()
    for y in STAMP_CELL:
        for x in STAMP_CELL:
            var src_x := (STAMP_CELL - 1 - x) if flip_h else x
            var src_y := (STAMP_CELL - 1 - y) if flip_v else y
            var si := (src_y * STAMP_CELL + src_x) * 4
            var di := (y * STAMP_CELL + x) * 4
            res[di] = data[si]
            res[di + 1] = data[si + 1]
            res[di + 2] = data[si + 2]
            res[di + 3] = data[si + 3]
    return res

func _diff(a: PackedByteArray, b: PackedByteArray) -> float:
    var sum := 0
    for i in a.size():
        sum += abs(a[i] - b[i])
    return float(sum) / max(1, a.size())

func _find_match(cell: Image) -> Dictionary:
    var base := _cell_bytes(cell, false, false)
    var bucket: Array = _stamp_fp_index.get(_fp_hash(_canonical_coarse(cell)), [])
    var best_diff := 0x7fffffff
    var best := {}
    for idx in bucket:
        var entry: Dictionary = _stamp_catalog[idx]
        var canonical: Image = entry["cell"]
        var variants: Array[Dictionary] = [
            {"h": false, "v": false},
            {"h": true, "v": false},
            {"h": false, "v": true},
            {"h": true, "v": true},
        ]
        for variant in variants:
            var vb: PackedByteArray = _cell_bytes(canonical, variant["h"], variant["v"])
            var d := _diff(base, vb)
            if d < best_diff:
                best_diff = d
                best = {
                    "key": entry["key"],
                    "flip_h": variant["h"],
                    "flip_v": variant["v"],
                }
    if best_diff <= STAMP_TOLERANCE:
        return best
    return {}

func _coarse_bytes(img: Image) -> PackedByteArray:
    _as_rgba8(img)
    const CB := 8
    var res := PackedByteArray()
    res.resize(CB * CB * 4)
    var data := img.get_data()
    for by in CB:
        for bx in CB:
            var r := 0
            var g := 0
            var b := 0
            var a := 0
            for y in 4:
                for x in 4:
                    var si := ((by * 4 + y) * STAMP_CELL + (bx * 4 + x)) * 4
                    r += data[si]
                    g += data[si + 1]
                    b += data[si + 2]
                    a += data[si + 3]
            var o := (by * CB + bx) * 4
            res[o] = r / 16
            res[o + 1] = g / 16
            res[o + 2] = b / 16
            res[o + 3] = a / 16
    return res

func _flip_coarse(c: PackedByteArray, flip_h: bool, flip_v: bool) -> PackedByteArray:
    const CB := 8
    var out := PackedByteArray()
    out.resize(CB * CB * 4)
    for by in CB:
        for bx in CB:
            var sby := (CB - 1 - by) if flip_v else by
            var sbx := (CB - 1 - bx) if flip_h else bx
            var si := (sby * CB + sbx) * 4
            var di := (by * CB + bx) * 4
            for k in 4:
                out[di + k] = c[si + k]
    return out

func _canonical_coarse(img: Image) -> PackedByteArray:
    var c := _coarse_bytes(img)
    var best: PackedByteArray = c
    for flip_h in [false, true]:
        for flip_v in [false, true]:
            var f := _flip_coarse(c, flip_h, flip_v)
            if _bytes_less(f, best):
                best = f
    return best

func _bytes_less(a: PackedByteArray, b: PackedByteArray) -> bool:
    var n: int = mini(a.size(), b.size())
    for i in n:
        if a[i] != b[i]:
            return a[i] < b[i]
    return a.size() < b.size()

func _fp_hash(bytes: PackedByteArray) -> int:
    var h := 2166136261
    for b in bytes:
        h = (h ^ b) * 16777619
    return h

func _input(event: InputEvent) -> void:
    if _hud_dragging:
        if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and not event.pressed:
            _hud_dragging = false
            get_viewport().set_input_as_handled()
            return
        if event is InputEventMouseMotion and event.button_mask & MOUSE_BUTTON_MASK_LEFT:
            var delta: Vector2 = event.position - _hud_drag_grab
            hud_label.set_anchors_preset(Control.PRESET_TOP_LEFT)
            hud_label.position = _hud_start_pos + delta
            get_viewport().set_input_as_handled()
            return
    if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
        if hud_label.visible and hud_label.get_global_rect().has_point(event.position):
            _hud_dragging = true
            _hud_drag_grab = event.position
            _hud_start_pos = hud_label.position
            get_viewport().set_input_as_handled()
            return
    if event is InputEventMouseMotion:
        var pos := tile_grid.get_local_mouse_position()
        var tile := Vector2i(int(pos.x / tile_grid.tile_size), int(pos.y / tile_grid.tile_size))
        cursor_label.text = "Tile: %d, %d" % [tile.x, tile.y]
        _update_hud()
    if event is InputEventKey and event.pressed and not event.echo:
        if event.keycode == KEY_X:
            _flip_h_active = not _flip_h_active
            _update_info()
            get_viewport().set_input_as_handled()
        if event.keycode == KEY_Y:
            _flip_v_active = not _flip_v_active
            _update_info()
            get_viewport().set_input_as_handled()
        if event.keycode == KEY_H or event.keycode == KEY_I or event.keycode == KEY_M:
            hud_label.visible = not hud_label.visible
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
