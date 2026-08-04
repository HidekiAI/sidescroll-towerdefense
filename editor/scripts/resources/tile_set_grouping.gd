class_name TileSetGrouping extends Resource

@export var key: String = ""
@export var display_name: String = ""
@export_range(1, 32) var width_tiles: int = 1
@export_range(1, 32) var height_tiles: int = 1
@export var tiles: Array = []
@export var tags: Array[String] = []
@export var source_image: String = ""
@export var src_margins: Vector2i = Vector2i(0, 0)
@export var src_cell_size: Vector2i = Vector2i(32, 32)
@export var src_separation: Vector2i = Vector2i(0, 0)


func _source_rect(col: int, row: int) -> Dictionary:
    var pitch_x := src_cell_size.x + src_separation.x
    var pitch_y := src_cell_size.y + src_separation.y
    return {
        "x": src_margins.x + col * pitch_x,
        "y": src_margins.y + row * pitch_y,
        "w": src_cell_size.x,
        "h": src_cell_size.y,
    }


func resolve(base_x: int, base_y: int) -> Array[Dictionary]:
    var result: Array[Dictionary] = []
    for entry in tiles:
        var r: Dictionary = {
            "x": base_x + entry.get("local_x", 0),
            "y": base_y + entry.get("local_y", 0),
            "terrain_key": entry.get("terrain_key", ""),
            "elevation_tiles": entry.get("elevation_tiles", 0),
            "z_depth": entry.get("z_depth", 0),
            "sub_tile_mask": entry.get("sub_tile_mask", 15),
            "flip_h": entry.get("flip_h", false),
            "flip_v": entry.get("flip_v", false),
        }
        if not source_image.is_empty():
            r["source_image"] = source_image
            r["source_rect"] = _source_rect(
                entry.get("source_col", 0),
                entry.get("source_row", 0),
            )
        result.append(r)
    return result


func resolve_to_json(base_x: int, base_y: int) -> Dictionary:
    return {
        "key": key,
        "display_name": display_name,
        "width_tiles": width_tiles,
        "height_tiles": height_tiles,
        "tiles": resolve(base_x, base_y),
        "tags": tags,
    }


func validate() -> bool:
    if key.is_empty() or display_name.is_empty():
        return false
    if width_tiles < 1 or height_tiles < 1:
        return false
    var seen: Array[Vector2i] = []
    for entry in tiles:
        var lx: int = entry.get("local_x", -1)
        var ly: int = entry.get("local_y", -1)
        var pos := Vector2i(lx, ly)
        if pos in seen:
            return false
        seen.append(pos)
        if entry.get("terrain_key", "").is_empty():
            return false
        if lx < 0 or lx >= width_tiles:
            return false
        if ly < 0 or ly >= height_tiles:
            return false
    return true
