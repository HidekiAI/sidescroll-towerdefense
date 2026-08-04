class_name TileSetEntryRes extends Resource

@export var local_x: int = 0
@export var local_y: int = 0
@export var terrain_key: String = ""
@export var elevation_tiles: int = 0
@export_range(-1, 1) var z_depth: int = 0
@export_range(0, 15) var sub_tile_mask: int = 15
@export var flip_h: bool = false
@export var flip_v: bool = false
@export var source_image: String = ""
@export var source_col: int = 0
@export var source_row: int = 0
