class_name TilePalette
extends ItemList
# Unified visual tile palette for the Map Editor side panel (issue #49).
# Replaces the old text-only PaletteList + TileSetPalette with one thumbnail
# grid that shows terrain swatches, world-shared tile-bank images, and stamp
# groupings. Clicking an entry reports a pick via `tile_picked(kind, key)`.

signal tile_picked(kind: String, key: String)

const THUMB := 32

var _entries: Array[Dictionary] = []
var _selected_kind := ""
var _selected_key := ""


func _ready() -> void:
    fixed_icon_size = Vector2i(THUMB, THUMB)
    icon_mode = ItemList.ICON_MODE_TOP
    allow_reselect = true
    item_selected.connect(_on_item_selected)


# Builds the grid from the editor's current data. `tiles` is the world-shared
# tile bank (key -> Image); `preview_of` renders a TileSetGrouping as a
# composite thumbnail (Callable(ts) -> Texture2D).
func populate(
    terrain: Array,
    tiles: Dictionary,
    groups: Array,
    preview_of: Callable,
) -> void:
    var prev_kind := _selected_kind
    var prev_key := _selected_key
    _entries.clear()
    clear()
    var seen: Dictionary = {}
    for t in terrain:
        var key := str(t.get("key", ""))
        if key.is_empty():
            continue
        seen[key] = true
        var icon := _swatch(Color(t.get("color_hex", "#808080")))
        var bank_img: Image = tiles.get(key)
        if bank_img != null:
            icon = ImageTexture.create_from_image(bank_img)
        _add_entry("terrain", key, t.get("display_name", key), icon)
    for k in tiles:
        if seen.has(str(k)):
            continue
        var img: Image = tiles[k]
        if img == null:
            continue
        _add_entry("tile", str(k), str(k), ImageTexture.create_from_image(img))
    for ts in groups:
        if "terrain_default" in ts.tags:
            continue
        _add_entry("group", ts.key, ts.display_name, preview_of.call(ts))
    for i in _entries.size():
        if _entries[i]["kind"] == prev_kind and _entries[i]["key"] == prev_key:
            select(i)


# Selects whichever entry carries `key` (used by main.gd navigate_to_tileset).
func select_key(key: String) -> void:
    for i in _entries.size():
        if _entries[i]["key"] == key:
            select(i)
            return


func _add_entry(kind: String, key: String, label: String, icon: Texture2D) -> void:
    var idx := add_item(label, icon, true)
    set_item_tooltip(idx, label)
    set_item_metadata(idx, {"kind": kind, "key": key})
    _entries.append({"kind": kind, "key": key, "label": label})


func _swatch(color: Color) -> ImageTexture:
    var img := Image.create(THUMB, THUMB, false, Image.FORMAT_RGBA8)
    img.fill(color)
    return ImageTexture.create_from_image(img)


func _on_item_selected(index: int) -> void:
    if index < 0 or index >= _entries.size():
        return
    var e: Dictionary = _entries[index]
    _selected_kind = e["kind"]
    _selected_key = e["key"]
    tile_picked.emit(e["kind"], e["key"])
