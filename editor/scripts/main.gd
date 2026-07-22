extends Control

var current_tab := 0
var is_dirty := false
var project_path := ""
var _bridge: Node
var _grid_config: Dictionary = {}

var tabs: TabContainer
var terrain_editor: Node
var entity_editor: Node
var map_editor: Node
var placement_editor: Node
var simulator: Node

func _ready() -> void:
    tabs = $TabContainer
    _instantiate_tabs()
    _load_bridge()
    _init_config_db()
    _load_grid_config()
    tabs.tab_changed.connect(_on_tab_changed)
    _wire_editors()

func _instantiate_tabs() -> void:
    var scenes := {
        terrain_editor = preload("res://scenes/terrain_editor.tscn"),
        entity_editor = preload("res://scenes/entity_editor.tscn"),
        map_editor = preload("res://scenes/map_editor.tscn"),
        placement_editor = preload("res://scenes/placement_editor.tscn"),
        simulator = preload("res://scenes/simulator.tscn"),
    }
    for key in scenes:
        var instance = scenes[key].instantiate()
        tabs.add_child(instance)
        set(key, instance)

func _load_bridge() -> void:
    if ClassDB.class_exists("SstdBridge"):
        _bridge = ClassDB.instantiate("SstdBridge")
        add_child(_bridge)
    else:
        push_error("GDExtension not found — SstdBridge class unavailable")
        assert(false, "GDExtension bridge is required")

func _init_config_db() -> void:
    var config_dir := ProjectSettings.globalize_path("res://config/")
    DirAccess.make_dir_recursive_absolute(config_dir)
    var db_path := config_dir.path_join("sstd_config.db")
    var result: Variant = _bridge.init_config_db(db_path)
    var parsed = JSON.parse_string(result)
    assert(parsed != null and parsed.get("ok", false), "Failed to init config DB: " + str(parsed))

func _load_grid_config() -> void:
    assert(_bridge != null and _bridge.has_method("get_grid_config"), "Bridge must be loaded before grid config")
    var json: Variant = _bridge.get_grid_config()
    var parsed = JSON.parse_string(json)
    assert(parsed != null and typeof(parsed) == TYPE_DICTIONARY, "Failed to parse grid config from bridge")
    _grid_config = parsed

func _wire_editors() -> void:
    for ed in [terrain_editor, entity_editor, map_editor, placement_editor, simulator]:
        if ed.has_method("set_bridge"):
            ed.set_bridge(_bridge)
        if ed.has_method("set_grid_config"):
            ed.set_grid_config(_grid_config)

func _on_tab_changed(tab: int) -> void:
    current_tab = tab
    if tab == 2:
        map_editor._populate_grid()
    if tab == 3:
        placement_editor.set_entity_defs(entity_editor.get_entity_defs())

func new_project() -> void:
    if is_dirty:
        _confirm_discard()
    pass

func open_project(path: String) -> void:
    pass

func save_project() -> void:
    pass

func save_project_as(path: String) -> void:
    pass

func export_json(path: String) -> void:
    pass

func import_json(path: String) -> void:
    pass

func _confirm_discard() -> void:
    pass
