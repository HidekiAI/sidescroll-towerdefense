class_name GodotTileSetGrouping extends Resource

@export var key: String = ""
@export var godot_tileset: TileSet
@export var display_name: String = ""
@export var width_tiles: int = 4
@export var height_tiles: int = 2
@export var tags: Array[String] = []

@export var tile_coords: Array[Dictionary] = []


func _read_custom_data(tile_data: TileData, field: String, default):
    if tile_data and godot_tileset.has_custom_data_layer(field):
        return tile_data.get_custom_data(field)
    return default

func resolve(base_x: int, base_y: int) -> Array[Dictionary]:
    var result: Array[Dictionary] = []
    if not godot_tileset or godot_tileset.get_source_count() < 1:
        return result
    var source: TileSetAtlasSource = godot_tileset.get_source(0)
    var tex: Texture2D = source.texture
    var tex_path: String = tex.resource_path if tex else ""
    for tc in tile_coords:
        var atlas_coords := Vector2i(tc.get("col", 0), tc.get("row", 0))
        var region: Rect2i = source.get_tile_region(atlas_coords)
        var tile_data: TileData = source.get_tile_data(atlas_coords, 0)

        var tkey: String = _read_custom_data(tile_data, "terrain_key", tc.get("terrain_key", "grass"))
        var elevation := int(_read_custom_data(tile_data, "elevation_tiles", tc.get("elevation_tiles", 0)))
        var z := int(_read_custom_data(tile_data, "z_depth", tc.get("z_depth", 0)))
        var mask := int(_read_custom_data(tile_data, "sub_tile_mask", tc.get("sub_tile_mask", 15)))
        var flip_h := bool(_read_custom_data(tile_data, "flip_h", tc.get("flip_h", false)))
        var flip_v := bool(_read_custom_data(tile_data, "flip_v", tc.get("flip_v", false)))

        if tkey.is_empty():
            tkey = source.resource_name

        result.append({
            "x": base_x + tc.get("local_x", 0),
            "y": base_y + tc.get("local_y", 0),
            "terrain_key": tkey,
            "elevation_tiles": elevation,
            "z_depth": z,
            "sub_tile_mask": mask,
            "flip_h": flip_h,
            "flip_v": flip_v,
            "source_image": tex_path,
            "source_rect": {"x": region.position.x, "y": region.position.y, "w": region.size.x, "h": region.size.y},
        })
    return result


func validate() -> bool:
    if not godot_tileset:
        return false
    if display_name.is_empty():
        return false
    if width_tiles < 1 or height_tiles < 1:
        return false
    if tile_coords.size() != width_tiles * height_tiles:
        return false
    var seen: Array[Vector2i] = []
    for tc in tile_coords:
        var pos := Vector2i(tc.get("local_x", 0), tc.get("local_y", 0))
        if pos in seen:
            return false
        seen.append(pos)
    return true
