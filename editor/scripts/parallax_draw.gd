@tool
class_name ParallaxDraw extends Node2D
# ParallaxDraw renders the placements `SstdBridge.parallax_placements` returns, using `_draw()`
# rather than child nodes. It owns no maths: it reads `screen_top_px`, `first_copy_left_px`,
# `copy_count`, `scale`, `period_px`, `art_h_px` and the two fills straight out of the placement
# entry, so `last_placements()` equal to the bridge's array is the whole contract (G1).
#
# One node draws one plane. `WorldView` makes a single bridge call and hands `BackPlane` the
# back-plane slice and `FrontPlane` the front-plane slice, so the two concatenated
# `last_placements()` arrays are exactly that one response's `placements`.
#
# Design of record: PLAN-2026-10-16-parallax-step4.md section 4.3;
# TDD_Parallax-Scrolling.Math.md section 7. The per-copy loop the plan keeps as a fallback for
# the unverified `tile = true` transform question (section 4.3.1) is NOT here yet — it lands only
# if the G5 spike shows the repeat is not computed in the item's pre-transform space.
#
# Rejected alternative, kept as a view rather than deleted: one `Sprite2D` per art copy, built as
# children. It was the first working shape and is described in plan section 2.2; it costs
# `copy_count` nodes and a child rebuild per redraw where this costs one `draw_texture_rect`.

# The wire token `Fill::Transparent` serializes as. Mirrors `FILL_TRANSPARENT` in
# `crates/sstd-core/src/parallax.rs`; a malformed fill cannot reach here, because `Fill`'s
# deserializer rejects anything that is not this token or a `#RRGGBBAA` colour.
const FILL_TRANSPARENT: String = "transparent"

# The characters `#RRGGBBAA` may contain. A separate check is needed because `String.hex_to_int()`
# PARSES UNTIL IT HITS AN INVALID CHARACTER instead of failing: "#8A98ACGG" returns 0x8A98AC and
# silently draws a wrong band. Measured 2026-10-18, issue #107; caught by D4 in
# tests/test_parallax_draw.gd.
const HEX_DIGIT_CHARACTERS: String = "0123456789abcdefABCDEF"
# Alpha below which a fill band has no height worth drawing.
const FILL_MIN_ALPHA: float = 0.0

var _placements: Array = []
var _view_w_px: float = 0.0
var _view_h_px: float = 0.0
var _stack_dir: String = ""
var _textures: Dictionary = {}


func _ready() -> void:
    # Repeat is the layer period, so the tiling has to be enabled at the CanvasItem. NEAREST is
    # what G5 asserts at zoom 1 (a LINEAR filter blends the probe row above the Hills art top and
    # the mutation turns that gate red). Whether a non-integer letterbox scale should instead use
    # LINEAR is an open question for @me, recorded in plan section 8; it does not change this.
    texture_repeat = TEXTURE_REPEAT_ENABLED
    texture_filter = TEXTURE_FILTER_NEAREST


# Replace the drawn set and queue one redraw. `stack_dir` is where `entry.file`'s bare filename
# resolves: the placements carry the authored filename, and rewriting it would break the G1
# equality against the bridge's own array.
func apply(placements: Array, view_w_px: float, view_h_px: float, stack_dir: String) -> void:
    _placements = placements
    _view_w_px = view_w_px
    _view_h_px = view_h_px
    _stack_dir = stack_dir
    queue_redraw()


# The placements last applied, unmodified. G1 compares this against the bridge's array.
func last_placements() -> Array:
    return _placements


func _draw() -> void:
    for entry_index in range(_placements.size()):
        var entry := _placements[entry_index] as Dictionary
        var scale_value: float = float(entry.get("scale", 1.0))
        var art_top_px: float = float(entry.get("screen_top_px", 0))
        var art_bottom_px: float = art_top_px + float(entry.get("art_h_px", 0.0)) * scale_value

        # A fill draws WITH its layer, never in a separate pass: G12 pins that a fills-first pass
        # puts sky between the hills and the ground, because the nearer layer's art would then be
        # painted over the farther layer's already-drawn fill.
        var fill_above := _fill_colour(String(entry.get("fill_above", FILL_TRANSPARENT)))
        if fill_above.a > FILL_MIN_ALPHA and art_top_px > 0.0:
            draw_rect(Rect2(0.0, 0.0, _view_w_px, art_top_px), fill_above)
        var fill_below := _fill_colour(String(entry.get("fill_below", FILL_TRANSPARENT)))
        var below_height_px: float = _view_h_px - art_bottom_px
        if fill_below.a > FILL_MIN_ALPHA and below_height_px > 0.0:
            draw_rect(Rect2(0.0, art_bottom_px, _view_w_px, below_height_px), fill_below)

        var period_px: float = float(entry.get("period_px", 0.0))
        var copy_count: float = float(entry.get("copy_count", 0))
        var texture := _texture_of(String(entry.get("file", "")))
        if texture != null and period_px > 0.0 and copy_count > 0.0:
            # The rect is UNSCALED art pixels and the transform carries the zoom, so the drawn
            # period on screen is period_px * scale — what G5 asserts at zoom 1 (1920 px) and
            # zoom 0.25 (480 px).
            draw_set_transform(
                Vector2(float(entry.get("first_copy_left_px", 0)), art_top_px),
                0.0,
                Vector2(scale_value, scale_value))
            draw_texture_rect(
                texture,
                Rect2(0.0, 0.0, period_px * copy_count, float(entry.get("art_h_px", 0.0))),
                true)
    # Reset before the next caller: a CanvasItem transform is state that outlives this call, and
    # anything else drawing into this node would inherit the last layer's zoom.
    draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)


# A fill is one strict wire value: the transparent token, or `#RRGGBBAA`. Returns fully
# transparent for anything unrecognised so a bad value draws nothing rather than a black band.
func _fill_colour(fill_value: String) -> Color:
    if fill_value == FILL_TRANSPARENT or fill_value.is_empty():
        return Color(0.0, 0.0, 0.0, 0.0)
    if not fill_value.begins_with("#") or fill_value.length() != 9:
        printerr("parallax.draw BAD_FILL value=%s" % fill_value)
        return Color(0.0, 0.0, 0.0, 0.0)
    var hex_digits: String = fill_value.substr(1)
    for digit_index in range(hex_digits.length()):
        if HEX_DIGIT_CHARACTERS.find(hex_digits[digit_index]) == -1:
            printerr("parallax.draw BAD_FILL value=%s" % fill_value)
            return Color(0.0, 0.0, 0.0, 0.0)
    return Color8(
        hex_digits.substr(0, 2).hex_to_int(),
        hex_digits.substr(2, 2).hex_to_int(),
        hex_digits.substr(4, 2).hex_to_int(),
        hex_digits.substr(6, 2).hex_to_int())


# Cached per filename, including the misses: a missing PNG would otherwise be re-attempted on
# every redraw, and a redraw happens on every camera move.
func _texture_of(file_name: String) -> Texture2D:
    if _textures.has(file_name):
        return _textures[file_name] as Texture2D
    var texture: Texture2D = null
    if not file_name.is_empty() and not _stack_dir.is_empty():
        var loaded: Variant = load(_stack_dir.path_join(file_name))
        if loaded is Texture2D:
            texture = loaded as Texture2D
        else:
            printerr("parallax.draw MISSING_TEXTURE file=%s dir=%s" % [file_name, _stack_dir])
    _textures[file_name] = texture
    return texture
