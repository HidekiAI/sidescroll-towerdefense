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

const TAB_NAMES: Array[String] = [
    "tile", "entity", "map", "placement", "simulator",
]
const TAB_LABELS: Array[String] = [
    "Tile Types", "Entity Types", "Map Editor", "Placement Editor", "Simulator",
]

func get_current_tab_name() -> String:
    return TAB_NAMES[current_tab]

func log(msg: String) -> void:
    print("[%s] %s" % [get_current_tab_name(), msg])

func log_error(msg: String) -> void:
    push_error("[%s] %s" % [get_current_tab_name(), msg])

func log_warning(msg: String) -> void:
    push_warning("[%s] %s" % [get_current_tab_name(), msg])

func _ready() -> void:
    tabs = $TabContainer
    terrain_editor = tabs.get_child(0)
    entity_editor = tabs.get_child(1)
    map_editor = tabs.get_child(2)
    placement_editor = tabs.get_child(3)
    simulator = tabs.get_child(4)
    _load_bridge()
    _init_config_db()
    _load_grid_config()
    tabs.tab_changed.connect(_on_tab_changed)
    _wire_editors()
    _auto_load_last_map()

func _instantiate_tabs() -> void:
    var scenes := {
        terrain_editor = preload("res://scenes/terrain_editor.tscn"),
        entity_editor = preload("res://scenes/entity_editor.tscn"),
        map_editor = preload("res://scenes/map_editor.tscn"),
        placement_editor = preload("res://scenes/placement_editor.tscn"),
        simulator = preload("res://scenes/simulator.tscn"),
    }
    var keys: Array[String] = ["terrain_editor", "entity_editor", "map_editor", "placement_editor", "simulator"]
    for i in keys.size():
        var instance = scenes[keys[i]].instantiate()
        tabs.add_child(instance)
        tabs.set_tab_title(i, TAB_LABELS[i])
        set(keys[i], instance)

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
    var db_path := config_dir.path_join("sstd_config.sqlite3")
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
        if ed.has_method("set_main_reference"):
            ed.set_main_reference(self)

func _sync_tile_set_categories_to_db() -> void:
    if not _bridge or not _bridge.has_method("set_tile_set_category"):
        return
    for ts in map_editor._tile_set_groupings:
        for tag in ts.tags:
            if tag.begins_with("category:"):
                var terrain_key: String = tag.trim_prefix("category:")
                _bridge.set_tile_set_category(ts.key, terrain_key)

func _save_last_map_path(path: String) -> void:
    if _bridge and _bridge.has_method("set_config_value"):
        _bridge.set_config_value("last_map_path", path)

func _get_last_map_path() -> String:
    if _bridge and _bridge.has_method("get_config_value"):
        return _bridge.get_config_value("last_map_path") as String
    return ""

func _auto_load_last_map() -> void:
    var path := _get_last_map_path()
    if path.is_empty():
        return
    if path.begins_with("res://"):
        path = ProjectSettings.globalize_path(path)
    if FileAccess.file_exists(path):
        map_editor._on_import_file(path)
        print("[load] Auto-loaded last map: " + path)

func _on_tab_changed(tab: int) -> void:
    current_tab = tab
    print("[%s] Switched to tab" % get_current_tab_name())
    if tab == 0:
        terrain_editor.set_tile_set_groupings(map_editor._tile_set_groupings)
        _sync_tile_set_categories_to_db()
    if tab == 2:
        map_editor.set_terrain_types(terrain_editor.get_terrain_types())
    if tab == 3:
        placement_editor.set_terrain_types(terrain_editor.get_terrain_types())
        placement_editor.set_entity_defs(entity_editor.get_entity_defs())
        var grid: Dictionary = map_editor.get_tile_grid()
        placement_editor.set_tile_grid(grid["tiles"], grid["tile_data"])

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

func navigate_to_tileset(tileset_key: String) -> void:
    tabs.current_tab = 2
    map_editor.select_tile_set(tileset_key)

func export_json(path: String) -> void:
    pass

func import_json(path: String) -> void:
    pass

func _confirm_discard() -> void:
    pass
