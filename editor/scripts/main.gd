extends Control

const USER_DATA_DIR := "user://data"
const DEFAULT_WORLD_PATH := "user://data/world.json"
const DEFAULT_WORLD_FILE := "world.zip"
const KEY_WORLD_FILE_NAME := "defaults.world_file_name"
const KEY_WORLD_JSON_PATH := "defaults.world_json_path"
const KEY_FILE_DIALOG_DIR := "defaults.file_dialog_dir"

var _world_json_path := DEFAULT_WORLD_PATH

var current_tab := 0
var is_dirty := false
var project_path := ""
var _bridge: Node
var _grid_config: Dictionary = {}
var screen_store: ScreenStore

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
	# Quit is handled manually (dirty-save prompt on WM_CLOSE_REQUEST); the
	# engine's default auto-accept would quit even while the dialog awaits (#60).
	get_tree().auto_accept_quit = false
	tabs = $TabContainer
	terrain_editor = tabs.get_child(0)
	entity_editor = tabs.get_child(1)
	map_editor = tabs.get_child(2)
	placement_editor = tabs.get_child(3)
	simulator = tabs.get_child(4)
	_load_bridge()
	_init_config_db()
	_seed_config_defaults()
	_world_json_path = get_world_json_path()
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
	var config_dir := user_data_dir().path_join("config")
	DirAccess.make_dir_recursive_absolute(config_dir)
	var db_path := config_dir.path_join("sstd_config.sqlite3")
	var result: Variant = _bridge.init_config_db(db_path)
	var parsed = JSON.parse_string(result)
	assert(parsed != null and parsed.get("ok", false), "Failed to init config DB: " + str(parsed))

# Per-user writable base for all runtime writes (`res://` stays read-only so a
# packaged build works). First call creates the directory.
func user_data_dir() -> String:
	var abs := ProjectSettings.globalize_path(USER_DATA_DIR)
	DirAccess.make_dir_recursive_absolute(abs)
	return abs

func _get_config_or(key: String, fallback: String) -> String:
	if _bridge and _bridge.has_method("get_config_value"):
		var value: Variant = _bridge.get_config_value(key)
		if value != null and value != "":
			return str(value)
	return fallback

func _seed_config_defaults() -> void:
	for key in [KEY_WORLD_FILE_NAME, KEY_WORLD_JSON_PATH, KEY_FILE_DIALOG_DIR]:
		if _get_config_or(key, "") == "":
			var value: String = DEFAULT_WORLD_FILE if key == KEY_WORLD_FILE_NAME \
					else DEFAULT_WORLD_PATH if key == KEY_WORLD_JSON_PATH else ""
			if _bridge and _bridge.has_method("set_config_value"):
				_bridge.set_config_value(key, value)
				print("[main] seeded config %s = %s" % [key, value])

func get_default_world_file() -> String:
	return _get_config_or(KEY_WORLD_FILE_NAME, DEFAULT_WORLD_FILE)

func get_world_json_path() -> String:
	return _get_config_or(KEY_WORLD_JSON_PATH, DEFAULT_WORLD_PATH)

func get_file_dialog_dir() -> String:
	return _get_config_or(KEY_FILE_DIALOG_DIR, "")

func _load_grid_config() -> void:
	assert(_bridge != null and _bridge.has_method("get_grid_config"), "Bridge must be loaded before grid config")
	var json: Variant = _bridge.get_grid_config()
	var parsed = JSON.parse_string(json)
	assert(parsed != null and typeof(parsed) == TYPE_DICTIONARY, "Failed to parse grid config from bridge")
	_grid_config = parsed

func _wire_editors() -> void:
	screen_store = ScreenStore.new()
	screen_store.load_world(_world_json_path)
	for ed in [terrain_editor, entity_editor, map_editor, placement_editor, simulator]:
		if ed.has_method("set_bridge"):
			ed.set_bridge(_bridge)
		if ed.has_method("set_grid_config"):
			ed.set_grid_config(_grid_config)
		if ed.has_method("set_main_reference"):
			ed.set_main_reference(self)
		if ed.has_method("set_screen_store"):
			ed.set_screen_store(screen_store)

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
		print("[main] No last map recorded — editor starts fresh")
		return
	if path.begins_with("res://"):
		path = ProjectSettings.globalize_path(path)
	if not FileAccess.file_exists(path):
		print("[main] Last map no longer exists, skipping: " + path)
		return
	map_editor._on_import_file(path)
	print("[main] Auto-loaded last map: " + path)
	print("[main] world_package_path=%s screens=%d" % [screen_store.world_package_path if screen_store else "", screen_store.screens.size() if screen_store else 0])

func _on_tab_changed(tab: int) -> void:
	current_tab = tab
	print("[%s] Switched to tab" % get_current_tab_name())
	if tab == 0:
		terrain_editor.set_tile_set_groupings(map_editor._tile_set_groupings)
		_sync_tile_set_categories_to_db()
	if tab == 2:
		placement_editor.cache_current_screen()
		map_editor.set_terrain_types(terrain_editor.get_terrain_types())
		map_editor.set_tile_bank(placement_editor.get_tile_bank())
	if tab == 3:
		map_editor.cache_current_screen()
		placement_editor.set_terrain_types(terrain_editor.get_terrain_types())
		placement_editor.set_entity_defs(entity_editor.get_entity_defs())
		placement_editor.set_tile_bank(map_editor.get_tile_bank())
		placement_editor._screen_id = map_editor._screen_id
		placement_editor.screen_spin.set_value_no_signal(placement_editor._screen_id)
		placement_editor._sync_position_from_id()
		if placement_editor._store and placement_editor._store.is_occupied(
				placement_editor._screen_pos.x, placement_editor._screen_pos.y):
			placement_editor._restore_screen()
		else:
			var grid: Dictionary = map_editor.get_tile_grid()
			placement_editor.set_tile_grid(grid["tiles"], grid["tile_data"])
		placement_editor._update_hud()

func new_project() -> void:
	if await _confirm_save_dirty() == 0:
		return
	pass

func open_project(path: String) -> void:
	if await _confirm_save_dirty() == 0:
		return
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

func mark_dirty() -> void:
	is_dirty = true

func clear_dirty() -> void:
	is_dirty = false

# Save/Discard/Cancel prompt for dirty state (#60). Returns 2=save-then-proceed,
# 1=discard-and-proceed, 0=cancel. Non-dirty callers skip the prompt and get 1.
# Modal (awaited): safe to call from _notification quit / new / open / import.
func _confirm_save_dirty() -> int:
	if not is_dirty:
		return 1
	var dlg := _build_unsaved_dialog()
	var done := false
	var choice := 0
	var on_resolve := func(c: int) -> void:
		choice = c
		done = true
		dlg.queue_free()
	dlg.get_ok_button().pressed.connect(func() -> void: on_resolve.call(2))
	dlg.get_cancel_button().pressed.connect(func() -> void: on_resolve.call(1))
	dlg.close_requested.connect(func() -> void: on_resolve.call(0))
	dlg.popup_centered()
	while not done:
		await get_tree().process_frame
	return choice

# NOTIFICATION_WM_CLOSE_REQUEST cannot be handled with an await: the engine calls
# the handler, discards the coroutine, and the continuation never resumes — so the
# quit prompt would return without quitting. Instead we drive it via callbacks:
# the dialog must resolve to an explicit get_tree().quit() or Cancel keeps the
# app alive. (#60)
func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST:
		if not is_dirty:
			get_tree().quit()
			return
		_close_open_file_dialogs()
		var dlg := _build_unsaved_dialog()
		var on_resolve := func(c: int) -> void:
			dlg.queue_free()
			if c == 2 and map_editor and map_editor.has_method("_save_world"):
				map_editor._save_world()
			if c != 0:
				get_tree().quit()
		dlg.get_ok_button().pressed.connect(func() -> void: on_resolve.call(2))
		dlg.get_cancel_button().pressed.connect(func() -> void: on_resolve.call(1))
		dlg.close_requested.connect(func() -> void: on_resolve.call(0))
		dlg.popup_centered()

# A still-open FileDialog is an exclusive child window; a second exclusive dialog
# (the quit prompt) would fail to open. Hide them before prompting. (#60)
func _close_open_file_dialogs() -> void:
	for child in get_tree().root.get_children():
		_hide_dialogs_in(child)

func _hide_dialogs_in(node: Node) -> void:
	if node is FileDialog and node.visible:
		node.hide()
	for child in node.get_children():
		_hide_dialogs_in(child)

func _build_unsaved_dialog() -> ConfirmationDialog:
	var dlg := ConfirmationDialog.new()
	dlg.title = "Unsaved changes"
	dlg.dialog_text = "The current map has unsaved changes.\nSave before continuing?"
	dlg.ok_button_text = "Save"
	dlg.cancel_button_text = "Discard"
	var abort := Button.new()
	abort.text = "Cancel"
	abort.pressed.connect(func() -> void: dlg.close_requested.emit())
	dlg.add_child(abort)
	dlg.get_cancel_button().icon = null
	dlg.min_size = Vector2(420, 0)
	add_child(dlg)
	return dlg

# Guard for interactive load/new/open: returns 0=cancel, else proceeds (after
# saving if the user chose Save). Callers await this before replacing the world.
func _confirm_dirty_or_save() -> int:
	var choice: int = await _confirm_save_dirty()
	if choice == 2:
		if map_editor and map_editor.has_method("_save_world"):
			map_editor._save_world()
	return choice
