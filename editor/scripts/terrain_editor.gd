extends Control

var _bridge: Node
var _main: Node
var _terrain_types: Array[Dictionary] = []
var _framework_terrains: Array[Dictionary] = []
var _selected_index: int = -1
var _suppress_prop_change: bool = false
var _tile_px: int = 32
var _tile_set_groupings: Array = []

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
@onready var preview_rect: ColorRect = $PropPanel/PixelSplit/VBox/Grid/PreviewRect
@onready var pixel_canvas: Control = $PropPanel/PixelSplit/ArtPanel/PixelCanvas
@onready var preview_3x3: Control = $PropPanel/PixelSplit/ArtPanel/Preview3x3
@onready var import_png_btn: Button = $PropPanel/PixelSplit/ArtPanel/ArtToolbar/ImportPngBtn
@onready var export_png_btn: Button = $PropPanel/PixelSplit/ArtPanel/ArtToolbar/ExportPngBtn
@onready var fill_color_btn: Button = $PropPanel/PixelSplit/ArtPanel/ArtToolbar/FillColorBtn
@onready var save_btn: Button = $PropPanel/HSave/SaveBtn
@onready var import_btn: Button = $PropPanel/HSave/ImportBtn
@onready var export_btn: Button = $PropPanel/HSave/ExportBtn
@onready var import_sprite_btn: Button = $PropPanel/HSave/ImportSpriteBtn
@onready var tile_set_preview_list: ItemList = $PropPanel/PixelSplit/VBox/Grid/TileSetPreviewList
@onready var tile_set_label: Label = $PropPanel/PixelSplit/VBox/Grid/TileSetLabel
@onready var mask_tl: CheckBox = $PropPanel/PixelSplit/VBox/Grid/CollisionGroup/MaskContainer/MaskGrid/TLToggle
@onready var mask_tr: CheckBox = $PropPanel/PixelSplit/VBox/Grid/CollisionGroup/MaskContainer/MaskGrid/TRToggle
@onready var mask_bl: CheckBox = $PropPanel/PixelSplit/VBox/Grid/CollisionGroup/MaskContainer/MaskGrid/BLToggle
@onready var mask_br: CheckBox = $PropPanel/PixelSplit/VBox/Grid/CollisionGroup/MaskContainer/MaskGrid/BRToggle
@onready var mask_preview: TextureRect = $PropPanel/PixelSplit/VBox/Grid/CollisionGroup/MaskContainer/MaskPreview
@onready var auto_detect_btn: Button = $PropPanel/PixelSplit/VBox/Grid/CollisionGroup/AutoDetectBtn
@onready var dmg_spin: SpinBox = $PropPanel/PixelSplit/VBox/Grid/HazardDmgSpin
@onready var destruct_check: CheckBox = $PropPanel/PixelSplit/VBox/Grid/DestructCheck
@onready var destruct_hp_spin: SpinBox = $PropPanel/PixelSplit/VBox/Grid/DestructHpSpin
@onready var on_destroy_option: OptionButton = $PropPanel/PixelSplit/VBox/Grid/OnDestroyOption

func set_main_reference(m: Node) -> void:
	_main = m

func set_tile_set_groupings(groupings: Array) -> void:
	_tile_set_groupings = groupings
	_refresh_tile_set_preview()

func set_bridge(b: Node) -> void:
	_bridge = b

func _log(msg: String) -> void:
	if _main:
		_main.log(msg)
	else:
		print(msg)

func _ready() -> void:
	_populate_option_buttons()
	add_btn.pressed.connect(_on_add)
	delete_btn.pressed.connect(_on_delete)
	list.item_selected.connect(_on_select)
	save_btn.pressed.connect(_on_save)
	import_btn.pressed.connect(_on_import)
	export_btn.pressed.connect(_on_export)
	import_sprite_btn.pressed.connect(_on_import_sprite)
	tile_set_preview_list.item_selected.connect(_on_tile_set_preview_selected)

	for prop in [key_edit, name_edit, walkable_check, buildable_check, surface_option, hazard_option, elev_spin, color_picker, dmg_spin, destruct_check, destruct_hp_spin, on_destroy_option]:
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

	for cb in [mask_tl, mask_tr, mask_bl, mask_br]:
		cb.toggled.connect(_on_mask_toggled)

	import_png_btn.pressed.connect(_on_import_png)
	export_png_btn.pressed.connect(_on_export_png)
	fill_color_btn.pressed.connect(_on_fill_color)
	pixel_canvas.pixel_changed.connect(_on_canvas_pixel_changed)
	auto_detect_btn.pressed.connect(_on_auto_detect_mask)

func _populate_option_buttons() -> void:
	for item in ["normal", "ice", "mud"]:
		surface_option.add_item(item)
	for item in ["none", "lava"]:
		hazard_option.add_item(item)

func _refresh_on_destroy_options() -> void:
	on_destroy_option.clear()
	on_destroy_option.add_item("(none)")
	for t in _terrain_types:
		on_destroy_option.add_item(t["key"])

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
		"sub_tile_mask": 0xF,
		"hazard_damage_per_tick": 0,
		"is_destructible": false,
		"destructible_hp": 0,
		"on_destroy_terrain_key": "",
	})
	_refresh_list()
	_refresh_on_destroy_options()
	list.select(_terrain_types.size() - 1)
	_on_select(_terrain_types.size() - 1)

func _on_delete() -> void:
	if _selected_index < 0 or _selected_index >= _terrain_types.size():
		return
	_terrain_types.remove_at(_selected_index)
	_selected_index = -1
	_refresh_list()
	_refresh_on_destroy_options()
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
	dmg_spin.value = t.get("hazard_damage_per_tick", 0)
	destruct_check.button_pressed = t.get("is_destructible", false)
	destruct_hp_spin.value = t.get("destructible_hp", 0)
	_set_on_destroy_option(t.get("on_destroy_terrain_key", ""))
	var submask := int(t.get("sub_tile_mask", 0xF))
	mask_tl.button_pressed = (submask >> 0) & 1 == 1
	mask_tr.button_pressed = (submask >> 1) & 1 == 1
	mask_bl.button_pressed = (submask >> 2) & 1 == 1
	mask_br.button_pressed = (submask >> 3) & 1 == 1
	_suppress_prop_change = false

	_update_preview()
	_draw_mask_preview()
	_load_tile_png(t)
	_refresh_tile_set_preview()

func _set_on_destroy_option(key: String) -> void:
	if key.is_empty():
		on_destroy_option.select(0)
		return
	for i in on_destroy_option.item_count:
		if on_destroy_option.get_item_text(i) == key:
			on_destroy_option.select(i)
			return
	on_destroy_option.select(0)

func _refresh_tile_set_preview() -> void:
	tile_set_preview_list.clear()
	if _selected_index < 0 or _selected_index >= _terrain_types.size():
		tile_set_label.text = "TileSets using this terrain"
		return
	var terrain_key: String = _terrain_types[_selected_index]["key"]
	var matched_keys: Array = []

	if _bridge and _bridge.has_method("get_tile_sets_for_terrain"):
		var json: String = _bridge.get_tile_sets_for_terrain(terrain_key)
		var parsed = JSON.parse_string(json)
		if parsed is Array:
			for k in parsed:
				matched_keys.append(k as String)

	if matched_keys.is_empty():
		matched_keys = _tile_set_groupings.filter(
			func(ts): return ts.tags.any(
				func(tag): return tag.begins_with("category:") and tag.trim_prefix("category:") == terrain_key
			)
		).map(func(ts): return ts.key)

	var matches: Array = _tile_set_groupings.filter(
		func(ts): return matched_keys.has(ts.key)
	)
	tile_set_label.text = "TileSets using %s (%d)" % [terrain_key, matches.size()]
	for ts in matches:
		var label := "%s  %d\u00d7%d" % [ts.display_name, ts.width_tiles, ts.height_tiles]
		var idx := tile_set_preview_list.add_item(label)
		tile_set_preview_list.set_item_metadata(idx, ts.key)

func _on_tile_set_preview_selected(index: int) -> void:
	if index < 0 or index >= tile_set_preview_list.get_item_count():
		return
	var key = tile_set_preview_list.get_item_metadata(index)
	if typeof(key) != TYPE_STRING:
		return
	_log("Navigating to TileSet: " + key)
	if _main and _main.has_method("navigate_to_tileset"):
		_main.navigate_to_tileset(key)

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
	dmg_spin.value = 0
	destruct_check.button_pressed = false
	destruct_hp_spin.value = 0
	on_destroy_option.select(0)
	mask_tl.button_pressed = true
	mask_tr.button_pressed = true
	mask_bl.button_pressed = true
	mask_br.button_pressed = true
	_draw_mask_preview()

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
	t["hazard_damage_per_tick"] = int(dmg_spin.value)
	t["is_destructible"] = destruct_check.button_pressed
	t["destructible_hp"] = int(destruct_hp_spin.value)
	if on_destroy_option.selected == 0:
		t["on_destroy_terrain_key"] = ""
	else:
		t["on_destroy_terrain_key"] = on_destroy_option.get_item_text(on_destroy_option.selected)
	_update_preview()
	_refresh_list()
	_refresh_on_destroy_options()

func _on_mask_toggled(_val = false) -> void:
	if _suppress_prop_change:
		return
	if _selected_index < 0 or _selected_index >= _terrain_types.size():
		return
	var mask: int = 0
	if mask_tl.button_pressed: mask |= 1 << 0
	if mask_tr.button_pressed: mask |= 1 << 1
	if mask_bl.button_pressed: mask |= 1 << 2
	if mask_br.button_pressed: mask |= 1 << 3
	_terrain_types[_selected_index]["sub_tile_mask"] = mask
	_draw_mask_preview()

func _update_preview() -> void:
	if _selected_index >= 0 and _selected_index < _terrain_types.size():
		preview_rect.color = Color(_terrain_types[_selected_index]["color_hex"])

func _draw_mask_preview() -> void:
	if _selected_index < 0:
		mask_preview.modulate = Color(0.2, 0.2, 0.2, 0.5)
		return
	var mask: int = int(_terrain_types[_selected_index].get("sub_tile_mask", 0xF))
	var tl: bool = (mask >> 0) & 1 == 1
	var tr: bool = (mask >> 1) & 1 == 1
	var bl: bool = (mask >> 2) & 1 == 1
	var br: bool = (mask >> 3) & 1 == 1

	var bg := Color(0.1, 0.1, 0.1, 0.9)
	var solid := Color(0.2, 0.9, 0.2, 0.8)
	var empty := Color(0.9, 0.2, 0.2, 0.5)

	var img := Image.create(32, 32, false, Image.FORMAT_RGBA8)
	for y in 32:
		for x in 32:
			var qx: int = 0 if x < 16 else 1
			var qy: int = 0 if y < 16 else 1
			var bit_index := qy * 2 + qx
			var is_solid: bool = (mask >> bit_index) & 1 == 1
			var c := solid if is_solid else empty
			var border := (x % 16 == 0 or y % 16 == 0)
			img.set_pixel(x, y, bg if border else c)
	# quadrant labels as single-pixel markers: TL=top-left, etc.
	# TL marker (green dot at 4,4)
	if tl: img.set_pixel(4, 4, Color.GREEN)
	else:   img.set_pixel(4, 4, Color.RED)
	if tr: img.set_pixel(28, 4, Color.GREEN)
	else:   img.set_pixel(28, 4, Color.RED)
	if bl: img.set_pixel(4, 28, Color.GREEN)
	else:   img.set_pixel(4, 28, Color.RED)
	if br: img.set_pixel(28, 28, Color.GREEN)
	else:   img.set_pixel(28, 28, Color.RED)

	var tex := ImageTexture.create_from_image(img)
	mask_preview.texture = tex

func _on_canvas_pixel_changed(_x: int, _y: int, _color: Color) -> void:
	_sync_tiled_preview()

func _on_auto_detect_mask() -> void:
	if _selected_index < 0:
		return
	var img: Image = pixel_canvas.get_image()
	if not img:
		return
	var w: int = img.get_width()
	var h: int = img.get_height()
	if w == 0 or h == 0:
		return

	var mask: int = 0
	var quadrants := [
		{"index": 0, "x0": 0, "y0": 0, "x1": w/2, "y1": h/2},       # TL
		{"index": 1, "x0": w/2, "y0": 0, "x1": w, "y1": h/2},       # TR
		{"index": 2, "x0": 0, "y0": h/2, "x1": w/2, "y1": h},       # BL
		{"index": 3, "x0": w/2, "y0": h/2, "x1": w, "y1": h},       # BR
	]
	for q in quadrants:
		var total: float = 0.0
		var count: int = 0
		for y in range(q["y0"], q["y1"]):
			for x in range(q["x0"], q["x1"]):
				var px := img.get_pixel(x, y)
				var lum: float = 0.299 * px.r + 0.587 * px.g + 0.114 * px.b
				total += lum
				count += 1
		if count == 0:
			continue
		var mean_lum: float = total / count
		# Ground threshold: > 0.5 brightness → solid; < 0.25 → empty
		if mean_lum > 0.5:
			mask |= 1 << q["index"]
		# else: leave it 0 (empty)

	_suppress_prop_change = true
	mask_tl.button_pressed = (mask >> 0) & 1 == 1
	mask_tr.button_pressed = (mask >> 1) & 1 == 1
	mask_bl.button_pressed = (mask >> 2) & 1 == 1
	mask_br.button_pressed = (mask >> 3) & 1 == 1
	_terrain_types[_selected_index]["sub_tile_mask"] = mask
	_suppress_prop_change = false
	_draw_mask_preview()
	_log("Auto-detected sub_tile_mask: 0x%X" % mask)

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
	_log("PNG saved: " + _tile_png_path(t["key"]))

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
	_log("Terrain types saved: %d types" % _terrain_types.size())

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
			_ensure_defaults(t)
			_terrain_types.append(t)
		_selected_index = -1
		_refresh_list()
		_refresh_on_destroy_options()
		_clear_props()
	)
	dialog.popup_centered(Vector2i(600, 400))

func _ensure_defaults(t: Dictionary) -> void:
	if not t.has("sub_tile_mask"): t["sub_tile_mask"] = 0xF
	if not t.has("hazard_damage_per_tick"): t["hazard_damage_per_tick"] = 0
	if not t.has("is_destructible"): t["is_destructible"] = false
	if not t.has("destructible_hp"): t["destructible_hp"] = 0
	if not t.has("on_destroy_terrain_key"): t["on_destroy_terrain_key"] = ""

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

func _on_import_sprite() -> void:
	var dialog := FileDialog.new()
	dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
	dialog.add_filter("*.png", "PNG images")
	dialog.title = "Select Spritesheet"
	dialog.exclusive = false
	add_child(dialog)
	dialog.file_selected.connect(func(path: String):
		dialog.queue_free()
		_on_sprite_selected(path)
	, CONNECT_ONE_SHOT)
	dialog.popup_centered(Vector2i(800, 500))

func _on_sprite_selected(path: String) -> void:
	_prompt_grid_config(path)

func _prompt_grid_config(sprite_path: String) -> void:
	var popup := AcceptDialog.new()
	popup.title = "Spritesheet Grid Configuration"
	popup.ok_button_text = "Import"
	popup.dialog_autowrap = true
	add_child(popup)

	var vbox := VBoxContainer.new()
	vbox.size_flags_horizontal = Control.SIZE_FILL
	popup.add_child(vbox)

	# Spritesheet preview
	var preview := TextureRect.new()
	preview.expand_mode = TextureRect.EXPAND_FIT_WIDTH_PROPORTIONAL
	preview.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	preview.custom_minimum_size = Vector2(0, 200)
	preview.size_flags_horizontal = Control.SIZE_FILL
	preview.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	preview.mouse_filter = Control.MOUSE_FILTER_STOP
	var img := Image.new()
	if img.load(ProjectSettings.globalize_path(sprite_path)) == OK:
		preview.texture = ImageTexture.create_from_image(img)
	vbox.add_child(preview)

	# Info label
	var info := Label.new()
	info.text = sprite_path.get_file() + "  (" + str(img.get_width()) + "x" + str(img.get_height()) + " px)"
	info.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(info)

	vbox.add_child(_make_label("Columns"))
	var cols_spin := _make_spin(1, 200, 21)
	vbox.add_child(cols_spin)

	vbox.add_child(_make_label("Rows"))
	var rows_spin := _make_spin(1, 200, 12)
	vbox.add_child(rows_spin)

	vbox.add_child(_make_label("Offset X (px)"))
	var ox_spin := _make_spin(0, 999, 3)
	vbox.add_child(ox_spin)

	vbox.add_child(_make_label("Offset Y (px)"))
	var oy_spin := _make_spin(0, 999, 3)
	vbox.add_child(oy_spin)

	vbox.add_child(_make_label("Margin (px)"))
	var margin_spin := _make_spin(0, 99, 0)
	vbox.add_child(margin_spin)

	preview.gui_input.connect(func(event: InputEvent):
		if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			var tex_size := preview.texture.get_size() if preview.texture else Vector2.ZERO
			if tex_size.x <= 0 or tex_size.y <= 0:
				return
			var rect_size := preview.get_rect().size
			var scale := rect_size.x / tex_size.x
			var scaled_h := tex_size.y * scale
			var y_off := (rect_size.y - scaled_h) / 2.0
			var ix := int(event.position.x / scale)
			var iy := int((event.position.y - y_off) / scale)
			ix = clampi(ix, 0, int(tex_size.x) - 1)
			iy = clampi(iy, 0, int(tex_size.y) - 1)
			ox_spin.value = ix
			oy_spin.value = iy
	)

	popup.confirmed.connect(func():
		_run_import_tool(
			sprite_path,
			int(cols_spin.value),
			int(rows_spin.value),
			int(ox_spin.value),
			int(oy_spin.value),
			int(margin_spin.value),
		)
	, CONNECT_ONE_SHOT)
	popup.popup_centered(Vector2i(520, 520))

func _make_label(text: String) -> Label:
	var label := Label.new()
	label.text = text
	return label

func _make_spin(min_val: float, max_val: float, default: float) -> SpinBox:
	var spin := SpinBox.new()
	spin.min_value = min_val
	spin.max_value = max_val
	spin.value = default
	spin.step = 1
	spin.size_flags_horizontal = Control.SIZE_FILL
	return spin

func _run_import_tool(sprite_path: String, cols: int, rows: int, ox: int, oy: int, margin: int) -> void:
	var binary := _find_import_tiles_binary()
	if binary.is_empty():
		push_error("import-tiles binary not found. Build with: cargo build -p import-tiles --release")
		return

	var timestamp := str(Time.get_ticks_usec())
	var out_dir := ProjectSettings.globalize_path("res://assets/imported/import_" + timestamp)
	DirAccess.make_dir_recursive_absolute(out_dir + "/tiles")

	var config := {
		input = ProjectSettings.globalize_path(sprite_path),
		output_dir = out_dir,
		tile_size_px = 32,
		grid_cols = cols,
		grid_rows = rows,
		offset_x = ox,
		offset_y = oy,
		margin = margin,
		auto_bounding = false,
		auto_detect_collision = true,
		luminance_threshold = 0.45,
		terrain_map = {},
		tile_sets = [],
	}

	var config_path := "/tmp/import_config_" + timestamp + ".json"
	var cf := FileAccess.open(config_path, FileAccess.WRITE)
	if not cf:
		push_error("Failed to write temp config at ", config_path)
		return
	cf.store_string(JSON.stringify(config, "\t"))
	cf.close()

	_log("Running import-tiles...")
	var args := PackedStringArray(["--config", config_path])
	var output := []
	var exit_code := OS.execute(binary, args, output, true)

	for line in output:
		print(line)

	if exit_code != 0:
		var err_msg := "import-tiles failed (exit " + str(exit_code) + ")"
		if output.size() > 0:
			err_msg += "\n" + output[0]
		push_error(err_msg)
		return

	var terrain_json := out_dir + "/terrain_types.json"
	var f := FileAccess.open(terrain_json, FileAccess.READ)
	if not f:
		push_error("import-tiles succeeded but terrain_types.json not found")
		return
	var json_str := f.get_as_text()
	f.close()

	var parsed = JSON.parse_string(json_str) as Dictionary
	if not parsed or not parsed.has("tiles"):
		push_error("Invalid terrain_types.json in output")
		return

	_terrain_types.clear()
	for t in parsed["tiles"]:
		_ensure_defaults(t)
		_terrain_types.append(t)

	var tiles_src := out_dir + "/tiles"
	var tiles_dst := ProjectSettings.globalize_path("res://assets/tiles")
	var dir := DirAccess.open(tiles_src)
	if dir:
		dir.list_dir_begin()
		var fn := dir.get_next()
		while not fn.is_empty():
			if not dir.current_is_dir() and fn.ends_with(".png"):
				DirAccess.copy_absolute(tiles_src + "/" + fn, tiles_dst + "/" + fn)
			fn = dir.get_next()
		dir.list_dir_end()

	for t in _terrain_types:
		var key: String = t["key"]
		var def_png := tiles_dst + "/" + key + "_32x32.png"
		var dir2 := DirAccess.open(tiles_src)
		if dir2:
			dir2.list_dir_begin()
			var fn2 := dir2.get_next()
			while not fn2.is_empty() and not FileAccess.file_exists(def_png):
				if not dir2.current_is_dir() and fn2.begins_with(key + "_") and fn2.ends_with("_32x32.png"):
					DirAccess.copy_absolute(tiles_src + "/" + fn2, def_png)
				fn2 = dir2.get_next()
			dir2.list_dir_end()

	var sets_src := out_dir + "/tile_sets.json"
	if FileAccess.file_exists(sets_src):
		var dest := ProjectSettings.globalize_path("res://assets/imported/tile_sets.json")
		DirAccess.copy_absolute(sets_src, dest)

	_selected_index = -1
	_refresh_list()
	_refresh_on_destroy_options()
	_clear_props()
	_log("Imported spritesheet: %d terrain types" % _terrain_types.size())

func _find_import_tiles_binary() -> String:
	var candidates := [
		ProjectSettings.globalize_path("res://../target/release/import-tiles"),
		ProjectSettings.globalize_path("res://../target/debug/import-tiles"),
	]
	for c in candidates:
		if FileAccess.file_exists(c):
			return c
	return ""

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

func get_mask_preview() -> int:
	if _selected_index < 0:
		return 0xF
	return int(_terrain_types[_selected_index].get("sub_tile_mask", 0xF))

func set_framework_terrains(terrains: Array[Dictionary]) -> void:
	_framework_terrains = terrains.duplicate(true)
	_terrain_types.clear()
	for t in _framework_terrains:
		_terrain_types.append(t.duplicate(true))
	_selected_index = -1
	_refresh_list()
	_refresh_on_destroy_options()
	_clear_props()

func apply_terrain_overrides(overrides: Dictionary) -> void:
	for t in _terrain_types:
		var key: String = t.get("key", "")
		if overrides.has(key):
			for prop in overrides[key]:
				t[prop] = overrides[key][prop]
	_refresh_list()
	if _selected_index >= 0 and _selected_index < _terrain_types.size():
		_on_select(_selected_index)
	_refresh_on_destroy_options()

func collect_terrain_overrides() -> Dictionary:
	var overrides: Dictionary = {}
	for t in _terrain_types:
		var key: String = t.get("key", "")
		if key.is_empty():
			continue
		var fw: Dictionary = {}
		for ft in _framework_terrains:
			if ft.get("key", "") == key:
				fw = ft
				break
		if fw.is_empty():
			continue
		var diffs: Dictionary = {}
		for prop in t:
			if prop == "key":
				continue
			var value: Variant = t[prop]
			if prop == "sub_tile_mask":
				# Godot JSON.stringify writes float Variants as 15.0; the Rust
				# contract declares u8. Coerce to int (see
				# docs/PLAN-2026-08-27-terrain-brush-and-subtile-float.md).
				value = int(value)
			if fw.has(prop) and t[prop] != fw[prop]:
				diffs[prop] = value
			elif not fw.has(prop):
				diffs[prop] = value
		if not diffs.is_empty():
			overrides[key] = diffs
	return overrides
