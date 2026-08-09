class_name ScreenStore extends RefCounted

const WORLD_VERSION := "0.2.0"

var screens: Dictionary = {}
var cache: Dictionary = {}
var _dir_path: String = ""
var world_package_path: String = ""  # active world .zip, or "" in legacy manifest mode

func _key(x: int, y: int) -> String:
    return "%d,%d" % [x, y]

func key_of(x: int, y: int) -> String:
    return _key(x, y)

func set_dir(path: String) -> void:
    _dir_path = path

func get_dir() -> String:
    return _dir_path

func screen_path(x: int, y: int) -> String:
    var entry: Dictionary = screens.get(_key(x, y), {})
    if entry.is_empty():
        return ""
    if entry.has("file"):
        return _dir_path.path_join(entry["file"])
    return ""

func is_occupied(x: int, y: int) -> bool:
    return screens.has(_key(x, y))

func screen_at(x: int, y: int) -> Dictionary:
    return screens.get(_key(x, y), {})

func screen_id_at(x: int, y: int) -> int:
    return int(screens.get(_key(x, y), {}).get("id", -1))

func position_of_id(id: int) -> Vector2i:
    for k in screens:
        if int(screens[k].get("id", -1)) == id:
            var parts: PackedStringArray = (k as String).split(",")
            return Vector2i(int(parts[0]), int(parts[1]))
    return Vector2i(-1, -1)

func next_id() -> int:
    var max_id := 0
    for k in screens:
        max_id = max(max_id, int(screens[k].get("id", 0)))
    return max_id + 1

func next_free_position(start: Vector2i = Vector2i.ZERO) -> Vector2i:
    var pos := start
    for _i in 1024:
        if not is_occupied(pos.x, pos.y):
            return pos
        pos.x += 1
    return pos

func bounds() -> Rect2i:
    var min_x := 0
    var max_x := 0
    var min_y := 0
    var max_y := 0
    for k in screens:
        var parts: PackedStringArray = (k as String).split(",")
        var x := int(parts[0])
        var y := int(parts[1])
        min_x = mini(min_x, x)
        max_x = maxi(max_x, x)
        min_y = mini(min_y, y)
        max_y = maxi(max_y, y)
    return Rect2i(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)

func register(x: int, y: int, id: int, file_name: String, data: Dictionary = {}) -> void:
    screens[_key(x, y)] = {"x": x, "y": y, "id": id, "file": file_name}
    if not data.is_empty():
        cache[_key(x, y)] = data

func remove(x: int, y: int) -> void:
    screens.erase(_key(x, y))
    cache.erase(_key(x, y))

func move(from_pos: Vector2i, to_pos: Vector2i) -> bool:
    var k := _key(from_pos.x, from_pos.y)
    if not screens.has(k):
        return false
    if is_occupied(to_pos.x, to_pos.y):
        return false
    var entry: Dictionary = screens[k]
    entry["x"] = to_pos.x
    entry["y"] = to_pos.y
    screens.erase(k)
    screens[_key(to_pos.x, to_pos.y)] = entry
    if cache.has(k):
        cache[_key(to_pos.x, to_pos.y)] = cache[k]
        cache.erase(k)
    return true

func set_cache(x: int, y: int, data: Dictionary) -> void:
    cache[_key(x, y)] = data

func get_cache(x: int, y: int) -> Dictionary:
    return cache.get(_key(x, y), {})

func occupied_positions() -> Array[Vector2i]:
    var out: Array[Vector2i] = []
    for k in screens:
        var parts: PackedStringArray = (k as String).split(",")
        out.append(Vector2i(int(parts[0]), int(parts[1])))
    return out

func load_world(path: String) -> bool:
    if not FileAccess.file_exists(path):
        return false
    var f := FileAccess.open(path, FileAccess.READ)
    if not f:
        return false
    var parsed = JSON.parse_string(f.get_as_text())
    f.close()
    if typeof(parsed) != TYPE_DICTIONARY:
        return false
    # World-package pointer: world.json now records the active .zip.
    if parsed.has("world") and parsed["world"] is String:
        var pkg: String = parsed["world"]
        if not pkg.begins_with("/") and not pkg.begins_with("res://"):
            pkg = path.get_base_dir().path_join(pkg)
        return load_world_package(pkg)
    # Legacy path: world.json itself held the screens manifest.
    return _load_screens_manifest(parsed, path.get_base_dir())

# Loads a whole world .zip (map = all its screens + world-shared tiles) into memory.
func load_world_package(path: String) -> bool:
    var data := WorldArchive.load_world(path)
    if not data.get("ok", false):
        return false
    world_package_path = path
    _dir_path = path.get_base_dir()
    return apply_world_data(data)

# Populates screens + cache from a WorldArchive.load_world result.
func apply_world_data(data: Dictionary) -> bool:
    if not data.get("ok", false):
        return false
    screens.clear()
    cache.clear()
    for k in data["manifest"]:
        var e: Dictionary = data["manifest"][k]
        if not e.has("x") or not e.has("y") or not e.has("id"):
            continue
        var pos: Vector2i = Vector2i(int(e["x"]), int(e["y"]))
        var id: int = int(e["id"])
        screens[_key(pos.x, pos.y)] = {"x": pos.x, "y": pos.y, "id": id, "file": ""}
        var screen_data: Dictionary = data["screens"].get(id, {})
        if not screen_data.is_empty():
            cache[_key(pos.x, pos.y)] = screen_data
    return true

func _load_screens_manifest(parsed: Dictionary, base_dir: String) -> bool:
    if not parsed.has("screens"):
        return false
    screens.clear()
    cache.clear()
    _dir_path = base_dir
    for k in parsed["screens"]:
        var e: Dictionary = parsed["screens"][k]
        if not e.has("x") or not e.has("y") or not e.has("id"):
            continue
        screens[k] = {
            "x": int(e["x"]),
            "y": int(e["y"]),
            "id": int(e["id"]),
            "file": e.get("file", ""),
        }
    return true

func save_world(path: String) -> bool:
    var out: Dictionary = {
        "version": WORLD_VERSION,
    }
    if not world_package_path.is_empty():
        out["world"] = world_package_path
    else:
        out["screens"] = screens
    var f := FileAccess.open(path, FileAccess.WRITE)
    if not f:
        return false
    f.store_string(JSON.stringify(out, "\t"))
    f.close()
    return true
