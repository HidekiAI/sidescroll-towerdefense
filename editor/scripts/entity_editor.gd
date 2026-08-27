extends Control

var _entity_defs: Array[Dictionary] = []
var _framework_entities: Array[Dictionary] = []
var _selected_index: int = -1
var _bridge: Node
var _main: Node
var _sprite_px: int = 32

@onready var entity_list: ItemList = $ListPanel/ItemList
@onready var add_btn: Button = $ListPanel/VBox/AddBtn
@onready var delete_btn: Button = $ListPanel/VBox/DeleteBtn
@onready var key_edit: LineEdit = $PropPanel/PixelSplit/Scroll/VBox/Grid/KeyEdit
@onready var class_option: OptionButton = $PropPanel/PixelSplit/Scroll/VBox/Grid/ClassOption
@onready var width_spin: SpinBox = $PropPanel/PixelSplit/Scroll/VBox/Grid/WidthSpin
@onready var height_spin: SpinBox = $PropPanel/PixelSplit/Scroll/VBox/Grid/HeightSpin
@onready var hp_spin: SpinBox = $PropPanel/PixelSplit/Scroll/VBox/Grid/HpSpin
@onready var speed_spin: SpinBox = $PropPanel/PixelSplit/Scroll/VBox/Grid/SpeedSpin
@onready var range_spin: SpinBox = $PropPanel/PixelSplit/Scroll/VBox/Grid/RangeSpin
@onready var damage_spin: SpinBox = $PropPanel/PixelSplit/Scroll/VBox/Grid/DamageSpin
@onready var element_option: OptionButton = $PropPanel/PixelSplit/Scroll/VBox/Grid/ElementOption
@onready var cooldown_spin: SpinBox = $PropPanel/PixelSplit/Scroll/VBox/Grid/CooldownSpin
@onready var projectile_edit: LineEdit = $PropPanel/PixelSplit/Scroll/VBox/Grid/ProjectileEdit
@onready var ground_check: CheckBox = $PropPanel/PixelSplit/Scroll/VBox/Grid/GroundCheck
@onready var ceiling_check: CheckBox = $PropPanel/PixelSplit/Scroll/VBox/Grid/CeilingCheck
@onready var save_btn: Button = $PropPanel/HSave/SaveBtn
@onready var import_btn: Button = $PropPanel/HSave/ImportBtn
@onready var export_btn: Button = $PropPanel/HSave/ExportBtn
@onready var footprint_rect: ColorRect = $PropPanel/PixelSplit/Scroll/VBox/FootprintRect
@onready var pixel_canvas: Control = $PropPanel/PixelSplit/ArtPanel/PixelCanvas
@onready var preview_3x3: Control = $PropPanel/PixelSplit/ArtPanel/Preview3x3
@onready var import_png_btn: Button = $PropPanel/PixelSplit/ArtPanel/ArtToolbar/ImportPngBtn
@onready var export_png_btn: Button = $PropPanel/PixelSplit/ArtPanel/ArtToolbar/ExportPngBtn

const CLASS_KEYS := ["tower", "trap", "structure", "vehicle", "beast", "projectile", "hero", "adventurer", "soldier", "enemy", "convoy", "wave"]
const ELEMENT_KEYS := ["physical", "fire", "ice", "lightning", "holy", "dark"]

func _ready() -> void:
    _populate_option_buttons()
    add_btn.pressed.connect(_on_add)
    delete_btn.pressed.connect(_on_delete)
    entity_list.item_selected.connect(_on_select)
    save_btn.pressed.connect(_on_save)
    import_btn.pressed.connect(_on_import)
    export_btn.pressed.connect(_on_export)

    for prop in [key_edit, width_spin, height_spin, hp_spin, speed_spin, range_spin, damage_spin, cooldown_spin, projectile_edit, ground_check, ceiling_check]:
        if prop is LineEdit:
            prop.text_changed.connect(_on_prop_changed)
        elif prop is CheckBox:
            prop.toggled.connect(_on_prop_changed)
        elif prop is SpinBox:
            prop.value_changed.connect(_on_prop_changed)
    class_option.item_selected.connect(_on_prop_changed)
    element_option.item_selected.connect(_on_prop_changed)

    import_png_btn.pressed.connect(_on_import_png)
    export_png_btn.pressed.connect(_on_export_png)
    pixel_canvas.pixel_changed.connect(_on_canvas_pixel_changed)

func _populate_option_buttons() -> void:
    for item in CLASS_KEYS:
        class_option.add_item(item)
    for item in ELEMENT_KEYS:
        element_option.add_item(item)

func _refresh_list() -> void:
    entity_list.clear()
    for e in _entity_defs:
        entity_list.add_item(e["key"])

func _on_add() -> void:
    var base := "new_entity"
    var idx := 1
    while _entity_defs.any(func(e): return e["key"] == base + str(idx)):
        idx += 1
    _entity_defs.append({
        "key": base + str(idx),
        "class": "tower",
        "width_tiles": 1.0,
        "height_tiles": 1.0,
        "max_hp": 500,
        "speed_pps": 0,
        "attack_range_tiles": 3,
        "attack_power": 50,
        "element": "physical",
        "action_cooldown_ticks": 60,
        "projectile_type": "arrow",
        "requires_ground": true,
        "requires_ceiling": false,
    })
    _refresh_list()
    entity_list.select(_entity_defs.size() - 1)
    _on_select(_entity_defs.size() - 1)

func _on_delete() -> void:
    if _selected_index < 0 or _selected_index >= _entity_defs.size():
        return
    _entity_defs.remove_at(_selected_index)
    _selected_index = -1
    _refresh_list()
    _clear_props()

func _on_select(index: int) -> void:
    _selected_index = index
    var e: Dictionary = _entity_defs[index]
    key_edit.text = e["key"]
    class_option.select(max(0, CLASS_KEYS.find(e["class"])))
    width_spin.value = e["width_tiles"]
    height_spin.value = e["height_tiles"]
    hp_spin.value = e["max_hp"]
    speed_spin.value = e["speed_pps"]
    range_spin.value = e["attack_range_tiles"]
    damage_spin.value = e["attack_power"]
    element_option.select(max(0, ELEMENT_KEYS.find(e["element"])))
    cooldown_spin.value = e["action_cooldown_ticks"]
    projectile_edit.text = e["projectile_type"]
    ground_check.button_pressed = e["requires_ground"]
    ceiling_check.button_pressed = e["requires_ceiling"]
    _update_preview()
    _load_sprite_png(e["key"])

func _sprite_png_path(key: String) -> String:
    return "res://assets/entities/%s_%dx%d.png" % [key, _sprite_px, _sprite_px]

func _load_sprite_png(key: String) -> void:
    var path := _sprite_png_path(key)
    var abs := ProjectSettings.globalize_path(path)
    if FileAccess.file_exists(abs):
        if not pixel_canvas.load_png(abs):
            push_warning("Failed to load PNG: ", abs)
            _make_default_sprite_image()
    else:
        _make_default_sprite_image()
    _sync_tiled_preview()

func _make_default_sprite_image() -> void:
    var img: Image = Image.create(_sprite_px, _sprite_px, false, Image.FORMAT_RGBA8)
    var c := Color(0.5, 0.5, 0.5, 1)
    for y in _sprite_px:
        for x in _sprite_px:
            img.set_pixel(x, y, c)
    pixel_canvas.set_image(img)

func _sync_tiled_preview() -> void:
    var img: Image = pixel_canvas.get_image()
    if img:
        preview_3x3.set_image(img)

func _clear_props() -> void:
    key_edit.text = ""
    class_option.select(0)
    width_spin.value = 1.0
    height_spin.value = 1.0
    hp_spin.value = 500
    speed_spin.value = 0
    range_spin.value = 3
    damage_spin.value = 50
    element_option.select(0)
    cooldown_spin.value = 60
    projectile_edit.text = ""
    ground_check.button_pressed = false
    ceiling_check.button_pressed = false

func _on_prop_changed(_val = null) -> void:
    if _selected_index < 0 or _selected_index >= _entity_defs.size():
        return
    var e: Dictionary = _entity_defs[_selected_index]
    e["key"] = key_edit.text
    e["class"] = CLASS_KEYS[class_option.selected] if class_option.selected >= 0 else "tower"
    e["width_tiles"] = width_spin.value
    e["height_tiles"] = height_spin.value
    e["max_hp"] = int(hp_spin.value)
    e["speed_pps"] = int(speed_spin.value)
    e["attack_range_tiles"] = int(range_spin.value)
    e["attack_power"] = int(damage_spin.value)
    e["element"] = ELEMENT_KEYS[element_option.selected] if element_option.selected >= 0 else "physical"
    e["action_cooldown_ticks"] = int(cooldown_spin.value)
    e["projectile_type"] = projectile_edit.text
    e["requires_ground"] = ground_check.button_pressed
    e["requires_ceiling"] = ceiling_check.button_pressed
    _update_preview()
    _refresh_list()

func _on_canvas_pixel_changed(_x: int, _y: int, _color: Color) -> void:
    _sync_tiled_preview()

func _on_import_png() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
    dialog.add_filter("*.png", "PNG images")
    dialog.title = "Import Sprite Image"
    add_child(dialog)
    dialog.file_selected.connect(func(path: String):
        var img: Image = Image.new()
        if img.load(path) != OK:
            push_error("Failed to load: ", path)
            return
        if img.get_width() != _sprite_px or img.get_height() != _sprite_px:
            img.resize(_sprite_px, _sprite_px, Image.INTERPOLATE_NEAREST)
        pixel_canvas.set_image(img)
        _sync_tiled_preview()
    )
    dialog.popup_centered(Vector2i(600, 400))

func _on_export_png() -> void:
    if _selected_index < 0:
        return
    var e := _entity_defs[_selected_index]
    pixel_canvas.save_png(ProjectSettings.globalize_path(_sprite_png_path(e["key"])))
    _log("PNG saved: " + _sprite_png_path(e["key"]))

func _on_save() -> void:
    _on_export_png()
    var data: Dictionary = {"version": "0.1.0", "entities": _entity_defs}
    var json_str := JSON.stringify(data, "\t")
    if _bridge and _bridge.has_method("import_entity_defs"):
        var result: Variant = _bridge.import_entity_defs(json_str)
        var parsed = JSON.parse_string(result)
        if parsed and parsed.has("error"):
            push_error("Bridge validation: ", parsed["error"])
            return
            _log("Entity defs saved: %d entities" % _entity_defs.size())

func _on_import() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
    dialog.add_filter("*.json", "JSON files")
    dialog.title = "Import entity_defs.json"
    add_child(dialog)
    dialog.file_selected.connect(func(path: String):
        var f := FileAccess.open(path, FileAccess.READ)
        if not f:
            push_error("Cannot open: ", path)
            return
        var json_str := f.get_as_text()
        f.close()
        var parsed = JSON.parse_string(json_str)
        if not parsed or not parsed.has("entities"):
            push_error("Invalid entity_defs.json")
            return
        _entity_defs.clear()
        for e in parsed["entities"]:
            _entity_defs.append(e)
        _selected_index = -1
        _refresh_list()
        _clear_props()
    )
    dialog.popup_centered(Vector2i(600, 400))

func _on_export() -> void:
    var dialog := FileDialog.new()
    dialog.file_mode = FileDialog.FILE_MODE_SAVE_FILE
    dialog.add_filter("*.json", "JSON files")
    dialog.title = "Export entity_defs.json"
    dialog.current_file = "entity_defs.json"
    add_child(dialog)
    dialog.file_selected.connect(func(path: String):
        var f := FileAccess.open(path, FileAccess.WRITE)
        if f:
            f.store_string(JSON.stringify({"version": "0.1.0", "entities": _entity_defs}, "\t"))
            f.close()
    )
    dialog.popup_centered(Vector2i(600, 400))

func set_bridge(b: Node) -> void:
    _bridge = b

func set_main_reference(m: Node) -> void:
    _main = m

func _log(msg: String) -> void:
    if _main:
        _main.log(msg)
    else:
        print(msg)

func get_entity_defs() -> Array[Dictionary]:
    return _entity_defs

func set_framework_entities(entities: Array[Dictionary]) -> void:
    _framework_entities = entities.duplicate(true)
    _entity_defs.clear()
    for e in _framework_entities:
        _entity_defs.append(e.duplicate(true))
    _selected_index = -1
    _refresh_list()
    _clear_props()

func apply_world_entity_defs(world_defs: Array) -> void:
    for wd in world_defs:
        var key: String = wd.get("key", "")
        var found := false
        for i in _entity_defs.size():
            if _entity_defs[i].get("key", "") == key:
                for prop in wd:
                    _entity_defs[i][prop] = wd[prop]
                found = true
                break
        if not found:
            _entity_defs.append(wd.duplicate(true))
    _refresh_list()
    if _selected_index >= 0 and _selected_index < _entity_defs.size():
        _on_select(_selected_index)

func collect_world_entity_defs() -> Array:
    return _entity_defs.duplicate(true)

func _update_preview() -> void:
    if _selected_index < 0 or _selected_index >= _entity_defs.size():
        footprint_rect.custom_minimum_size = Vector2(32, 32)
        footprint_rect.color = Color(1, 1, 1, 0.15)
        return
    var e: Dictionary = _entity_defs[_selected_index]
    var px_per_tile: int = 32
    var w: int = max(px_per_tile, int(ceil(e["width_tiles"] * px_per_tile)))
    var h: int = max(px_per_tile, int(ceil(e["height_tiles"] * px_per_tile)))
    footprint_rect.custom_minimum_size = Vector2(w, h)
    footprint_rect.color = Color(0.4, 0.7, 1.0, 0.3)
