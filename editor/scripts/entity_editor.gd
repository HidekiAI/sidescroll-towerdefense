extends Control

var _entity_defs: Array[Dictionary] = []
var _selected_index: int = -1
var _bridge: Node

@onready var entity_list: ItemList = $ListPanel/ItemList
@onready var add_btn: Button = $ListPanel/VBox/AddBtn
@onready var delete_btn: Button = $ListPanel/VBox/DeleteBtn
@onready var key_edit: LineEdit = $PropPanel/Scroll/Grid/KeyEdit
@onready var class_option: OptionButton = $PropPanel/Scroll/Grid/ClassOption
@onready var width_spin: SpinBox = $PropPanel/Scroll/Grid/WidthSpin
@onready var height_spin: SpinBox = $PropPanel/Scroll/Grid/HeightSpin
@onready var hp_spin: SpinBox = $PropPanel/Scroll/Grid/HpSpin
@onready var speed_spin: SpinBox = $PropPanel/Scroll/Grid/SpeedSpin
@onready var range_spin: SpinBox = $PropPanel/Scroll/Grid/RangeSpin
@onready var damage_spin: SpinBox = $PropPanel/Scroll/Grid/DamageSpin
@onready var element_option: OptionButton = $PropPanel/Scroll/Grid/ElementOption
@onready var cooldown_spin: SpinBox = $PropPanel/Scroll/Grid/CooldownSpin
@onready var projectile_edit: LineEdit = $PropPanel/Scroll/Grid/ProjectileEdit
@onready var ground_check: CheckBox = $PropPanel/Scroll/Grid/GroundCheck
@onready var ceiling_check: CheckBox = $PropPanel/Scroll/Grid/CeilingCheck
@onready var save_btn: Button = $PropPanel/HSave/SaveBtn
@onready var import_btn: Button = $PropPanel/HSave/ImportBtn
@onready var export_btn: Button = $PropPanel/HSave/ExportBtn

const CLASS_KEYS := ["tower", "trap", "structure", "vehicle", "beast", "projectile", "hero", "adventurer", "soldier", "enemy", "convoy", "wave"]
const ELEMENT_KEYS := ["physical", "fire", "ice", "lightning", "holy", "dark"]

func _ready() -> void:
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

    _add_defaults()

func _add_defaults() -> void:
    var defaults: Array[Dictionary] = [
        {"key": "arrow_tower",  "class": "tower",  "width_tiles": 1.0, "height_tiles": 1.5, "max_hp": 500,  "speed_pps": 0, "attack_range_tiles": 5, "attack_power": 100,  "element": "physical",  "action_cooldown_ticks": 60,  "projectile_type": "arrow", "requires_ground": true, "requires_ceiling": false},
        {"key": "ballista",     "class": "tower",  "width_tiles": 1.0, "height_tiles": 1.5, "max_hp": 800,  "speed_pps": 0, "attack_range_tiles": 8, "attack_power": 250,  "element": "physical",  "action_cooldown_ticks": 120, "projectile_type": "bolt",  "requires_ground": true, "requires_ceiling": false},
        {"key": "catapult",     "class": "tower",  "width_tiles": 2.0, "height_tiles": 2.0, "max_hp": 1200, "speed_pps": 0, "attack_range_tiles": 10, "attack_power": 400,  "element": "physical",  "action_cooldown_ticks": 180, "projectile_type": "boulder", "requires_ground": true, "requires_ceiling": false},
        {"key": "wall",         "class": "structure", "width_tiles": 1.0, "height_tiles": 1.0, "max_hp": 2000, "speed_pps": 0, "attack_range_tiles": 0, "attack_power": 0,    "element": "physical",  "action_cooldown_ticks": 0,   "projectile_type": "",      "requires_ground": true, "requires_ceiling": false},
        {"key": "bridge",       "class": "structure", "width_tiles": 2.0, "height_tiles": 0.5, "max_hp": 1000, "speed_pps": 0, "attack_range_tiles": 0, "attack_power": 0,    "element": "physical",  "action_cooldown_ticks": 0,   "projectile_type": "",      "requires_ground": true, "requires_ceiling": false},
        {"key": "tarpit",       "class": "trap",   "width_tiles": 1.0, "height_tiles": 0.5, "max_hp": 200,  "speed_pps": 0, "attack_range_tiles": 0, "attack_power": 0,    "element": "physical",  "action_cooldown_ticks": 0,   "projectile_type": "",      "requires_ground": true, "requires_ceiling": false},
        {"key": "mine",         "class": "trap",   "width_tiles": 1.0, "height_tiles": 0.5, "max_hp": 50,   "speed_pps": 0, "attack_range_tiles": 2, "attack_power": 200,  "element": "physical",  "action_cooldown_ticks": 0,   "projectile_type": "",      "requires_ground": true, "requires_ceiling": false},
    ]
    for e in defaults:
        _entity_defs.append(e.duplicate(true))
    _refresh_list()

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
    _refresh_list()

func _on_save() -> void:
    var data: Dictionary = {"version": "0.1.0", "entities": _entity_defs}
    var json_str := JSON.stringify(data, "\t")
    if _bridge and _bridge.has_method("import_entity_defs"):
        var result: Variant = _bridge.import_entity_defs(json_str)
        var parsed = JSON.parse_string(result)
        if parsed and parsed.has("error"):
            push_error("Bridge validation: ", parsed["error"])
            return
    print("Entity defs saved: %d entities" % _entity_defs.size())

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

func get_entity_defs() -> Array[Dictionary]:
    return _entity_defs
