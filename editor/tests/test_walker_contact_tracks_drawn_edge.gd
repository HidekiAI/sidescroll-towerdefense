extends SceneTree
# Headless guard: the collision surface the walker samples must sit on the edge
# actually drawn by each generated slope PNG. The oracle decodes the PNG FROM THE FILE with
# Image.new().load(path) -- never an imported texture, whose RGB survives under alpha and shifts
# the edge by six rows.
# Pure per-file reads on one thread; no race or repeat state applies.

const TERRAIN_PATH: String = "res://default_package/terrain_types.json"
const TILE_PATH_PATTERN: String = "res://assets/tiles/%s_32x32.png"
const GENERATED_KEY_PREFIX: String = "slope_grass_30deg_4x2_"
const GENERATED_KEY_COUNT: int = 16
const TILE_WIDTH_PX: int = 32
const TILE_HEIGHT_PX: int = 32
const GROUND_GREY: int = 155
const AIR_GREY: int = 0
const MAX_CONTACT_ERROR_PX: int = 1

var _failures: int = 0


func _initialize() -> void:
    _run.call_deferred()


func _check(condition: bool, description: String) -> void:
    if not condition:
        _failures += 1
        printerr("FAIL: " + description)


func _mask_bit_is_set(mask_bytes: Array, col: int, row: int) -> bool:
    var bit_index: int = row * TILE_WIDTH_PX + col
    var byte_index: int = bit_index / 8
    if byte_index >= mask_bytes.size():
        return false
    return ((int(mask_bytes[byte_index]) >> (bit_index % 8)) & 1) == 1


func _topmost_row(image: Image, mask_bytes: Array, use_mask: bool) -> int:
    for row in range(TILE_HEIGHT_PX):
        for col in range(TILE_WIDTH_PX):
            var is_solid: bool = false
            if use_mask:
                is_solid = _mask_bit_is_set(mask_bytes, col, row)
            else:
                var grey: int = int(round(image.get_pixel(col, row).r * 255.0))
                is_solid = grey == GROUND_GREY
            if is_solid:
                return row
    return -1


func _check_key(key: String, entry: Dictionary) -> void:
    var image: Image = Image.new()
    var decode_error: Error = image.load(TILE_PATH_PATTERN % key)
    _check(decode_error == OK, "PNG for %s decodes from file" % key)
    if decode_error != OK:
        return
    _check(image.get_width() == TILE_WIDTH_PX and image.get_height() == TILE_HEIGHT_PX,
        "%s PNG is %dx%d" % [key, TILE_WIDTH_PX, TILE_HEIGHT_PX])

    var mask_bytes: Array = entry.get("solid_mask", [])
    _check(mask_bytes.size() == (TILE_WIDTH_PX * TILE_HEIGHT_PX + 7) / 8,
        "%s mask is a full W x H bitfield" % key)

    var drawn_row: int = _topmost_row(image, mask_bytes, false)
    var contact_row: int = _topmost_row(image, mask_bytes, true)
    _check(drawn_row >= 0, "%s has a drawn ground edge" % key)
    _check(contact_row >= 0, "%s has a collision contact edge" % key)
    if drawn_row < 0 or contact_row < 0:
        return
    var contact_error_px: int = absi(contact_row - drawn_row)
    _check(contact_error_px <= MAX_CONTACT_ERROR_PX,
        "%s contact row %d tracks drawn row %d within %d px (got %d)" % [
            key, contact_row, drawn_row, MAX_CONTACT_ERROR_PX, contact_error_px])

    # The same file oracle also pins the alpha overlay: air is transparent, ground is opaque.
    for row in range(TILE_HEIGHT_PX):
        for col in range(TILE_WIDTH_PX):
            var grey: int = int(round(image.get_pixel(col, row).r * 255.0))
            var expected_alpha: bool = grey == GROUND_GREY
            _check(grey == GROUND_GREY or grey == AIR_GREY,
                "%s pixel (%d,%d) is air or ground only" % [key, col, row])
            _check((image.get_pixel(col, row).a > 0.5) == expected_alpha,
                "%s pixel (%d,%d) alpha follows the file grey" % [key, col, row])


func _run() -> void:
    var decoded: Variant = JSON.parse_string(FileAccess.get_file_as_string(TERRAIN_PATH))
    _check(decoded is Dictionary, "framework terrain JSON parses")
    if not decoded is Dictionary:
        _finish()
        return
    var entries: Array = (decoded as Dictionary).get("tiles", [])
    var checked_count: int = 0
    for entry_variant in entries:
        var entry: Dictionary = entry_variant
        var key: String = String(entry.get("key", ""))
        if not key.begins_with(GENERATED_KEY_PREFIX):
            continue
        checked_count += 1
        _check_key(key, entry)
    _check(checked_count == GENERATED_KEY_COUNT,
        "all %d generated slopes were checked, got %d" % [GENERATED_KEY_COUNT, checked_count])
    _finish()


func _finish() -> void:
    print("walker_contact failures=%d" % _failures)
    quit(0 if _failures == 0 else 1)
