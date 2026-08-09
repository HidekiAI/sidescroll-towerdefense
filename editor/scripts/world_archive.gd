class_name WorldArchive
extends RefCounted

const WORLD_VERSION := "0.3.0"
const WORLD_EXT := ".zip"

# Canonical gameplay terrain types a world is allowed to reference.
# A world may override the *visuals* of these (tiles/{key}.png) but never
# introduce a new gameplay type. User-authored decorative tile art is
# allowed under the "stamp_" key prefix.
const CANONICAL_TERRAIN: PackedStringArray = [
    "air", "grass", "dirt", "stone", "wall", "water", "lava",
]

const MANIFEST_PATH := "manifest.json"
const SCREEN_DIR := "screens"
const TILE_DIR := "tiles"
const ENTITY_OVERRIDE_DIR := "entity_overrides"

static func is_world_path(path: String) -> bool:
    return path.ends_with(WORLD_EXT)

static func is_allowed_terrain_key(key: String) -> bool:
    if key.is_empty():
        return false
    if key.begins_with("stamp_"):
        return true
    return CANONICAL_TERRAIN.has(key)

static func png_bytes(img: Image) -> PackedByteArray:
    if img == null:
        return PackedByteArray()
    return img.save_png_to_buffer()

static func normalize_rgba8(img: Image) -> Image:
    if img == null or img.get_format() == Image.FORMAT_RGBA8:
        return img
    img.convert(Image.FORMAT_RGBA8)
    return img

static func png_to_image(data: PackedByteArray) -> Image:
    if data.is_empty():
        return null
    var img := Image.new()
    if img.load_png_from_buffer(data) != OK:
        return null
    return normalize_rgba8(img)

# Saves a whole world (map = collection of screens) into a self-contained zip.
#   manifest: ScreenStore-compatible screens table: {"0,0": {x,y,id}, ...}
#   screens:  per-screen serialized dicts keyed by screen id: {1: {...}, 2: {...}}
#   tiles:    shared tile bank keyed by tile id: {"grass": Image, "stamp_5": Image, ...}
#   entity_overrides: optional overrides of default entity defs keyed by entity key
static func save_world(
    path: String,
    manifest: Dictionary,
    screens: Dictionary,
    tiles: Dictionary,
    entity_overrides: Dictionary = {},
) -> bool:
    if FileAccess.file_exists(path):
        DirAccess.remove_absolute(path)
    var p := ZIPPacker.new()
    if p.open(path) != OK:
        return false
    p.start_file(MANIFEST_PATH)
    p.write_file(JSON.stringify({
        "version": WORLD_VERSION,
        "screens": manifest,
    }, "\t").to_utf8_buffer())
    p.close_file()
    for key in screens:
        p.start_file("%s/%d.json" % [SCREEN_DIR, int(key)])
        p.write_file(JSON.stringify(screens[key], "\t").to_utf8_buffer())
        p.close_file()
    for key in tiles:
        var img: Image = tiles[key]
        if img == null:
            continue
        p.start_file("%s/%s.png" % [TILE_DIR, key])
        p.write_file(png_bytes(img))
        p.close_file()
    for key in entity_overrides:
        p.start_file("%s/%s.json" % [ENTITY_OVERRIDE_DIR, key])
        p.write_file(JSON.stringify(entity_overrides[key], "\t").to_utf8_buffer())
        p.close_file()
    p.close()
    return true

# Loads a whole world zip into memory. Returns:
#   {"ok": true, "manifest": {...}, "screens": {id: {...}}, "tile_images": {key: Image}, "entity_overrides": {...}}
#   {"ok": false} on any failure.
static func load_world(path: String) -> Dictionary:
    if not FileAccess.file_exists(path):
        return {"ok": false}
    var r := ZIPReader.new()
    if r.open(path) != OK:
        return {"ok": false}
    var manifest_data := r.read_file(MANIFEST_PATH)
    if manifest_data.is_empty():
        r.close()
        return {"ok": false}
    var parsed = JSON.parse_string(manifest_data.get_string_from_utf8())
    if typeof(parsed) != TYPE_DICTIONARY or not parsed.has("screens"):
        r.close()
        return {"ok": false}
    var manifest: Dictionary = parsed["screens"]
    var screens: Dictionary = {}
    var tile_images: Dictionary = {}
    var entity_overrides: Dictionary = {}
    for f in r.get_files():
        if f.begins_with(SCREEN_DIR + "/") and f.ends_with(".json"):
            var id_str: String = f.trim_prefix(SCREEN_DIR + "/").trim_suffix(".json")
            var file_json = JSON.parse_string(r.read_file(f).get_string_from_utf8())
            if typeof(file_json) == TYPE_DICTIONARY:
                screens[int(id_str)] = file_json
        elif f.begins_with(TILE_DIR + "/") and f.ends_with(".png"):
            var key: String = f.trim_prefix(TILE_DIR + "/").trim_suffix(".png")
            var img := png_to_image(r.read_file(f))
            if img != null:
                tile_images[key] = img
        elif f.begins_with(ENTITY_OVERRIDE_DIR + "/") and f.ends_with(".json"):
            var key: String = f.trim_prefix(ENTITY_OVERRIDE_DIR + "/").trim_suffix(".json")
            var file_json = JSON.parse_string(r.read_file(f).get_string_from_utf8())
            if typeof(file_json) == TYPE_DICTIONARY:
                entity_overrides[key] = file_json
    r.close()
    # Validate per-doc rule: a world may only reference canonical terrain types or
    # stamp_* decorative art. Introducing a new gameplay type (e.g. "hotspring")
    # is rejected even if a tile PNG exists for it.
    for id in screens:
        for t in screens[id].get("tiles", []):
            var key: String = t.get("terrain", "")
            if key == "" or key == "air":
                continue
            if not is_allowed_terrain_key(key):
                push_error("World rejected: illegal terrain key '%s' in screen %d" % [key, id])
                return {"ok": false}
    return {
        "ok": true,
        "manifest": manifest,
        "screens": screens,
        "tile_images": tile_images,
        "entity_overrides": entity_overrides,
    }