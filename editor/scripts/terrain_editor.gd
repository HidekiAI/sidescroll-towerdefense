extends Control

var _bridge: Node
var _terrain_types: Array[Dictionary] = []
var _selected_index: int = -1
var _suppress_prop_change: bool = false
var _tile_px: int = 32

@onready var list: ItemList = $ListPanel/ItemList
@onready var add_btn: Button = $ListPanel/VBox/AddBtn
@onready var delete_btn: Button = $ListPanel/VBox/DeleteBtn
@onready var key_edit: LineEdit = $PropPanel/PixelSplit/VBox/Grid/KeyEdit
@onready var name_edit: LineEdit = $PropPanel/PixelSplit/VBox/Grid/NameEdit
@onready var walkable_check: CheckBox = $PropPanel/PixelSplit/VBox/Grid/WalkableCheck
@onready var buildable_check: CheckBox = $PropPanel/PixelSplit/VBox/Grid/BuildableCheck
@onready var surface_option: OptionButton = $PropPanel/PixelSplit/VBox/Grid/SurfaceOption
@onready var hazard_option: OptionButton = $PropPanel/PixelSplit/VBox/Grid/HazardOption
@onready var elev_spin: SpinBox = $PropPanel/PixelSplit/VBox/Grid/ElevSpin
@onready var color_picker: ColorPickerButton = $PropPanel/PixelSplit/VBox/Grid/ColorPicker
@onready var preview_rect: ColorRect = $PropPanel/PixelSplit/VBox/PreviewRect
@onready var pixel_canvas: Control = $PropPanel/PixelSplit/ArtPanel/PixelCanvas
@onready var preview_3x3: Control = $PropPanel/PixelSplit/ArtPanel/Preview3x3
@onready var import_png_btn: Button = $PropPanel/PixelSplit/ArtPanel/ArtToolbar/ImportPngBtn
@onready var export_png_btn: Button = $PropPanel/PixelSplit/ArtPanel/ArtToolbar/ExportPngBtn
@onready var fill_color_btn: Button = $PropPanel/PixelSplit/ArtPanel/ArtToolbar/FillColorBtn
@onready var save_btn: Button = $PropPanel/HSave/SaveBtn
@onready var import_btn: Button = $PropPanel/HSave/ImportBtn
@onready var export_btn: Button = $PropPanel/HSave/ExportBtn

func _ready() -> void:
    _populate_option_buttons()
    add_btn.pressed.connect(_on_add)
    delete_btn.pressed.connect(_on_delete)
    list.item_selected.connect(_on_select)
    save_btn.pressed.connect(_on_save)
    import_btn.pressed.connect(_on_import)
    export_btn.pressed.connect(_on_export)

    for prop in [key_edit, name_edit, walkable_check, buildable_check, surface_option, hazard_option, elev_spin, color_picker]:
        if prop is LineEdit:
            prop.text_changed.connect(_on_prop_changed)
        elif prop is CheckBox:
            prop.toggled.connect(_on_prop_changed)
        elif prop is OptionButton:
            prop.item_selected.connect(_on_prop_changed)
        elif prop is SpinBox:
            prop.value_changed.connect(_on_prop_changed)
        elif prop is ColorPickerButton:
            prop.color_changed.connect(_on_prop_changed)

    import_png_btn.pressed.connect(_on_import_png)
    export_png_btn.pressed.connect(_on_export_png)
    fill_color_btn.pressed.connect(_on_fill_color)
    pixel_canvas.pixel_changed.connect(_on_canvas_pixel_changed)

    _add_default_terrains()

func _populate_option_buttons() -> void:
    for item in ["normal", "ice", "mud"]:
        surface_option.add_item(item)
    for item in ["none", "lava"]:
        hazard_option.add_item(item)

func _add_default_terrains() -> void:
    var defaults: Array[Dictionary] = [
        {"key": "grass", "display_name": "Grass", "is_walkable": true, "is_buildable": true, "surface": "normal", "hazard": "none", "elevation_tiles": 0, "color_hex": "#4a7c3f"},
        {"key": "dirt", "display_name": "Dirt", "is_walkable": true, "is_buildable": true, "surface": "normal", "hazard": "none", "elevation_tiles": 0, "color_hex": "#8b5e3c"},
        {"key": "stone", "display_name": "Stone", "is_walkable": true, "is_buildable": true, "surface": "normal", "hazard": "none", "elevation_tiles": 0, "color_hex": "#7a7a7a"},
        {"key": "wall", "display_name": "Wall", "is_walkable": false, "is_buildable": false, "surface": "normal", "hazard": "none", "elevation_tiles": 0, "color_hex": "#555555"},
        {"key": "water", "display_name": "Water", "is_walkable": true, "is_buildable": false, "surface": "normal", "hazard": "none", "elevation_tiles": 0, "color_hex": "#3a7bd5"},
        {"key": "lava", "display_name": "Lava", "is_walkable": false, "is_buildable": false, "surface": "normal", "hazard": "lava", "elevation_tiles": 0, "color_hex": "#ff4500"},
        {"key": "air", "display_name": "Air", "is_walkable": false, "is_buildable": false, "surface": "normal", "hazard": "none", "elevation_tiles": 0, "color_hex": "#87ceeb"},
    ]
    for t in defaults:
        _terrain_types.append(t.duplicate())
    _refresh_list()

func _refresh_list() -> void:
    list.clear()
    for t in _terrain_types:
        list.add_item(t["display_name"])

func _on_add() -> void:
    var name_base: String = "new_terrain"
    var idx: int = 1
    while _terrain_types.any(func(t): return t["key"] == name_base + str(idx)):
        idx += 1
    var key: String = name_base + str(idx)
    _terrain_types.append({
        "key": key,
        "display_name": "New Terrain",
        "is_walkable": true,
        "is_buildable": true,
        "surface": "normal",
        "hazard": "none",
        "elevation_tiles": 0,
        "color_hex": "#aaaaaa",
    })
    _refresh_list()
    list.select(_terrain_types.size() - 1)
    _on_select(_terrain_types.size() - 1)

func _on_delete() -> void:
    if _selected_index < 0 or _selected_index >= _terrain_types.size():
        return
    _terrain_types.remove_at(_selected_index)
    _selected_index = -1
    _refresh_list()
    _clear_props()

func _on_select(index: int) -> void:
    _selected_index = index
    var t: Dictionary = _terrain_types[index]

    _suppress_prop_change = true
    key_edit.text = t["key"]
    name_edit.text = t["display_name"]
    walkable_check.button_pressed = t["is_walkable"]
    buildable_check.button_pressed = t["is_buildable"]
    surface_option.select(_surface_index(t["surface"]))
    hazard_option.select(_hazard_index(t["hazard"]))
    elev_spin.value = t["elevation_tiles"]
    color_picker.color = Color(t["color_hex"])
    _suppress_prop_change = false

    _update_preview()
    _load_tile_png(t)

func _tile_png_path(key: String) -> String:
    return "res://assets/tiles/%s_%dx%d.png" % [key, _tile_px, _tile_px]

func _load_tile_png(t: Dictionary) -> void:
    var path := _tile_png_path(t["key"])
    var abs := ProjectSettings.globalize_path(path)
    if FileAccess.file_exists(abs):
        if not pixel_canvas.load_png(abs):
            push_warning("Failed to load PNG: ", abs)
            _make_default_tile_image(t["color_hex"])
    else:
        _make_default_tile_image(t["color_hex"])
    _sync_tiled_preview()

func _make_default_tile_image(color_hex: String) -> void:
    var img: Image = Image.create(_tile_px, _tile_px, false, Image.FORMAT_RGBA8)
    var c := Color(color_hex)
    for y in _tile_px:
        for x in _tile_px:
            img.set_pixel(x, y, c)
    pixel_canvas.set_image(img)

func _sync_tiled_preview() -> void:
    var img: Image = pixel_canvas.get_image()
    if img:
        preview_3x3.set_image(img)

func _clear_props() -> void:
    key_edit.text = ""
    name_edit.text = ""
    walkable_check.button_pressed = false
    buildable_check.button_pressed = false
    surface_option.select(0)
    hazard_option.select(0)
    elev_spin.value = 0
    color_picker.color = Color.WHITE

func _on_prop_changed(_val = null) -> void:
    if _suppress_prop_change:
        return
    if _selected_index < 0 or _selected_index >= _terrain_types.size():
        return
    var t: Dictionary = _terrain_types[_selected_index]
    t["key"] = key_edit.text
    t["display_name"] = name_edit.text
    t["is_walkable"] = walkable_check.button_pressed
    t["is_buildable"] = buildable_check.button_pressed
    t["surface"] = _surface_key(surface_option.selected)
    t["hazard"] = _hazard_key(hazard_option.selected)
    t["elevation_tiles"] = int(elev_spin.value)
    t["color_hex"] = "#" + color_picker.color.to_html(false)
    _update_preview()
    _refresh_list()

func _update_preview() -> void:
    if _selected_index >= 0 and _selected_index < _terrain_types.size():
        preview_rect.color = Color(_terrain_types[_selected_index]["color_hex"])

func _on_canvas_pixel_changed(_x: int, _y: int, _color: Color) -> void:
    _sync_tiled_preview()

func _on_import_png() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
    dialog.add_filter("*.png", "PNG images")
    dialog.title = "Import Tile Image"
    add_child(dialog)
    dialog.file_selected.connect(func(path: String):
        var img: Image = Image.new()
        if img.load(path) != OK:
            push_error("Failed to load: ", path)
            return
        if img.get_width() != _tile_px or img.get_height() != _tile_px:
            img.resize(_tile_px, _tile_px, Image.INTERPOLATE_NEAREST)
        pixel_canvas.set_image(img)
        _sync_tiled_preview()
    )
    dialog.popup_centered(Vector2i(600, 400))

func _on_export_png() -> void:
    if _selected_index < 0:
        return
    var t := _terrain_types[_selected_index]
    pixel_canvas.save_png(ProjectSettings.globalize_path(_tile_png_path(t["key"])))
    print("PNG saved: ", _tile_png_path(t["key"]))

func _on_fill_color() -> void:
    if _selected_index < 0:
        return
    var t := _terrain_types[_selected_index]
    _make_default_tile_image(t["color_hex"])
    _sync_tiled_preview()

func _on_save() -> void:
    _on_export_png()
    var data: Dictionary = {
        "version": "0.1.0",
        "tiles": _terrain_types,
    }
    var json_str: String = JSON.stringify(data, "\t")
    if _bridge and _bridge.has_method("import_terrain_types"):
        var result: String = _bridge.import_terrain_types(json_str)
        var parsed: Dictionary = JSON.parse_string(result)
        if parsed and parsed.has("error"):
            push_error("Bridge validation: ", parsed["error"])
            return
    print("Terrain types saved: ", _terrain_types.size(), " types")

func _on_import() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
    dialog.add_filter("*.json", "JSON files")
    dialog.title = "Import terrain_types.json"
    add_child(dialog)
    dialog.file_selected.connect(func(path: String):
        var f := FileAccess.open(path, FileAccess.READ)
        if not f:
            push_error("Cannot open file: ", path)
            return
        var json_str: String = f.get_as_text()
        f.close()
        var parsed: Dictionary = JSON.parse_string(json_str)
        if not parsed or not parsed.has("tiles"):
            push_error("Invalid terrain_types.json")
            return
        _terrain_types.clear()
        for t in parsed["tiles"]:
            _terrain_types.append(t)
        _selected_index = -1
        _refresh_list()
        _clear_props()
    )
    dialog.popup_centered(Vector2i(600, 400))

func _on_export() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_SAVE_FILE
    dialog.add_filter("*.json", "JSON files")
    dialog.title = "Export terrain_types.json"
    dialog.current_file = "terrain_types.json"
    add_child(dialog)
    dialog.file_selected.connect(func(path: String):
        var data: Dictionary = {
            "version": "0.1.0",
            "tiles": _terrain_types,
        }
        var f := FileAccess.open(path, FileAccess.WRITE)
        if not f:
            push_error("Cannot write file: ", path)
            return
        f.store_string(JSON.stringify(data, "\t"))
        f.close()
    )
    dialog.popup_centered(Vector2i(600, 400))

static func _surface_index(s: String) -> int:
    match s:
        "normal": return 0
        "ice": return 1
        "mud": return 2
    return 0

static func _surface_key(i: int) -> String:
    match i:
        0: return "normal"
        1: return "ice"
        2: return "mud"
    return "normal"

static func _hazard_index(s: String) -> int:
    match s:
        "none": return 0
        "lava": return 1
    return 0

static func _hazard_key(i: int) -> String:
    match i:
        0: return "none"
        1: return "lava"
    return "none"

func get_terrain_types() -> Array[Dictionary]:
    return _terrain_types
