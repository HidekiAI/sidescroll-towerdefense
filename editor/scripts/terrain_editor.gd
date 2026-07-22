extends Control

var _bridge: Node
var _terrain_types: Array[Dictionary] = []
var _selected_index: int = -1

@onready var list: ItemList = $ListPanel/ItemList
@onready var add_btn: Button = $ListPanel/VBox/AddBtn
@onready var delete_btn: Button = $ListPanel/VBox/DeleteBtn
@onready var key_edit: LineEdit = $PropPanel/VBox/Grid/KeyEdit
@onready var name_edit: LineEdit = $PropPanel/VBox/Grid/NameEdit
@onready var walkable_check: CheckBox = $PropPanel/VBox/Grid/WalkableCheck
@onready var buildable_check: CheckBox = $PropPanel/VBox/Grid/BuildableCheck
@onready var surface_option: OptionButton = $PropPanel/VBox/Grid/SurfaceOption
@onready var hazard_option: OptionButton = $PropPanel/VBox/Grid/HazardOption
@onready var elev_spin: SpinBox = $PropPanel/VBox/Grid/ElevSpin
@onready var color_picker: ColorPickerButton = $PropPanel/VBox/Grid/ColorPicker
@onready var preview_rect: ColorRect = $PropPanel/VBox/PreviewRect
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
    key_edit.text = t["key"]
    name_edit.text = t["display_name"]
    walkable_check.button_pressed = t["is_walkable"]
    buildable_check.button_pressed = t["is_buildable"]
    surface_option.select(_surface_index(t["surface"]))
    hazard_option.select(_hazard_index(t["hazard"]))
    elev_spin.value = t["elevation_tiles"]
    color_picker.color = Color(t["color_hex"])
    _update_preview()

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

func _on_save() -> void:
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
