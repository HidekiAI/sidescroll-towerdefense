extends Control

var current_tab := 0
var is_dirty := false
var project_path := ""
var _bridge: Node

@onready var tabs: TabContainer = $TabContainer
@onready var terrain_editor = $TabContainer/TerrainEditor
@onready var entity_editor = $TabContainer/EntityEditor
@onready var map_editor = $TabContainer/MapEditor
@onready var placement_editor = $TabContainer/PlacementEditor
@onready var simulator = $TabContainer/Simulator

func _ready() -> void:
    _load_bridge()
    tabs.tab_changed.connect(_on_tab_changed)
    _wire_editors()

func _load_bridge() -> void:
    var gdext = load("res://rust/editor_bridge.gdextension")
    if gdext:
        _bridge = gdext.new()
    else:
        push_warning("GDExtension not found")

func _wire_editors() -> void:
    for ed in [terrain_editor, entity_editor, map_editor, placement_editor, simulator]:
        if ed.has_method("set_bridge"):
            ed.set_bridge(_bridge)

func _on_tab_changed(tab: int) -> void:
    current_tab = tab
    if tab == 2 and entity_editor.has_method("get_entity_defs"):
        map_editor.set_entity_defs(entity_editor.get_entity_defs())
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
