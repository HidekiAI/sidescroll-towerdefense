extends SceneTree
# Headless guard: the collision surface the walker samples must sit on the edge
# actually drawn by each generated slope PNG. The oracle decodes the PNG FROM THE FILE with
# Image.new().load(path) -- never an imported texture. Its ALPHA is the drawn edge: the grass
# slope art is textured (air ~210 grey, ground ~45-134, and 133/135 greys on both sides of a
# row), so greyscale clusters cannot separate air from ground here. The greyscale-vs-mask
# contract is pinned separately by the Rust two-grey synthetic-fixture unit test.
# Contact is checked per column, not as one whole-image top row. All-air columns are exempt from
# the contact assertion because a walker cannot stand where no ground is drawn; they are still
# asserted to carry no collision.
# Pure per-file reads on one thread; no race or repeat state applies.

const TERRAIN_PATH: String = "res://default_package/terrain_types.json"
const TILE_PATH_PATTERN: String = "res://assets/tiles/%s_32x32.png"
const GENERATED_KEY_PREFIX: String = "slope_grass_30deg_4x2_"
const GENERATED_KEY_COUNT: int = 16
const TILE_WIDTH_PX: int = 32
const TILE_HEIGHT_PX: int = 32
const MAX_CONTACT_ERROR_PX: int = 1
const NO_SOLID_ROW: int = -1

var _failures: int = 0


func _initialize() -> void:
    _run.call_deferred()


# Records one assertion failure in the suite counter, prints it, and returns the verdict so a
# caller can also build its own result without re-reading the counter.
func _record_check(condition: bool, description: String) -> bool:
    if not condition:
        _failures += 1
        printerr("FAIL: " + description)
    return condition


func _mask_bit_is_set(mask_bytes: Array, col: int, row: int) -> bool:
    var bit_index: int = row * TILE_WIDTH_PX + col
    var byte_index: int = bit_index / 8
    if byte_index >= mask_bytes.size():
        return false
    return ((int(mask_bytes[byte_index]) >> (bit_index % 8)) & 1) == 1


# The rendered edge is alpha, baked by the Rust converter from the same mask the terrain catalog
# carries. Reading it from the file keeps this guard independent of the catalog and of any
# imported texture.
func _fn_png_alpha_is_ground(image: Image, col: int, row: int) -> bool:
    return image.get_pixel(col, row).a > 0.5


# Returns the first solid row in `col`, or NO_SOLID_ROW (-1) when the column has none. Callers
# MUST compare against NO_SOLID_ROW before using the value as a row index: -1 is a sentinel, not
# a row, and a bare `>= 0` test hides that contract from anyone reading only this function.
func _topmost_solid_row_in_column(image: Image, mask_bytes: Array, use_mask: bool, col: int) -> int:
    for row in range(TILE_HEIGHT_PX):
        var is_solid: bool = _mask_bit_is_set(mask_bytes, col, row) if use_mask else \
            _fn_png_alpha_is_ground(image, col, row)
        if is_solid:
            return row
    return NO_SOLID_ROW


# Returns true only when this key's PNG decodes and its alpha edge agrees with the catalog mask.
# Each failed assertion is still reported through `_record_check`; the returned verdict lets a
# caller decide which keys passed without reading the suite's counter.
func _check_key(key: String, entry: Dictionary) -> bool:
    var key_ok: bool = true
    var image: Image = Image.new()
    var decode_error: Error = image.load(TILE_PATH_PATTERN % key)
    key_ok = _record_check(decode_error == OK, "PNG for %s decodes from file" % key)
    if decode_error != OK:
        return false
    key_ok = _record_check(image.get_width() == TILE_WIDTH_PX and image.get_height() == TILE_HEIGHT_PX,
        "%s PNG is %dx%d" % [key, TILE_WIDTH_PX, TILE_HEIGHT_PX]) and key_ok

    var mask_bytes: Array = entry.get("solid_mask", [])
    key_ok = _record_check(mask_bytes.size() == (TILE_WIDTH_PX * TILE_HEIGHT_PX + 7) / 8,
        "%s mask is a full W x H bitfield" % key) and key_ok

    # Per-column contact is the acceptance criterion; a single whole-image top row would miss
    # a shifted edge on any slope column after the first. An authored all-air column has no
    # surface to stand on, so it only has to stay collision-free.
    for col in range(TILE_WIDTH_PX):
        var drawn_row: int = _topmost_solid_row_in_column(image, mask_bytes, false, col)
        var contact_row: int = _topmost_solid_row_in_column(image, mask_bytes, true, col)
        if drawn_row == NO_SOLID_ROW:
            key_ok = _record_check(contact_row == NO_SOLID_ROW,
                "%s column %d has no collision where no ground is drawn" % [key, col]) and key_ok
            continue
        key_ok = _record_check(contact_row != NO_SOLID_ROW,
            "%s column %d has a collision contact edge" % [key, col]) and key_ok
        if contact_row == NO_SOLID_ROW:
            continue
        var contact_error_px: int = absi(contact_row - drawn_row)
        key_ok = _record_check(contact_error_px <= MAX_CONTACT_ERROR_PX,
            "%s column %d contact row %d tracks drawn row %d within %d px (got %d)" % [
                key, col, contact_row, drawn_row, MAX_CONTACT_ERROR_PX, contact_error_px]) and key_ok

    # Every pixel of the committed catalog must agree with the committed art's rendered edge.
    for row in range(TILE_HEIGHT_PX):
        for col in range(TILE_WIDTH_PX):
            var drawn_ground: bool = _fn_png_alpha_is_ground(image, col, row)
            var mask_ground: bool = _mask_bit_is_set(mask_bytes, col, row)
            key_ok = _record_check(mask_ground == drawn_ground,
                "%s pixel (%d,%d) mask follows the PNG alpha edge" % [key, col, row]) and key_ok
    return key_ok


func _run() -> void:
    var decoded: Variant = JSON.parse_string(FileAccess.get_file_as_string(TERRAIN_PATH))
    _record_check(decoded is Dictionary, "framework terrain JSON parses")
    if not decoded is Dictionary:
        _finish()
        return
    var entries: Array = (decoded as Dictionary).get("tiles", [])
    var checked_count: int = 0
    var passing_count: int = 0
    for entry_variant in entries:
        var entry: Dictionary = entry_variant
        var key: String = String(entry.get("key", ""))
        if not key.begins_with(GENERATED_KEY_PREFIX):
            continue
        checked_count += 1
        if _check_key(key, entry):
            passing_count += 1
    _record_check(checked_count == GENERATED_KEY_COUNT,
        "all %d generated slopes were checked, got %d" % [GENERATED_KEY_COUNT, checked_count])
    print("walker_contact passing_keys=%d/%d" % [passing_count, GENERATED_KEY_COUNT])
    _finish()


func _finish() -> void:
    print("walker_contact failures=%d" % _failures)
    quit(0 if _failures == 0 else 1)
