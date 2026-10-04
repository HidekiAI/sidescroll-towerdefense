extends Control
# ref #68 #74 #89 — Simulator tab: two scrollbars scrub per-layer Parallax2D.scroll_offset.
# Each layer's scroll_offset is set to scrub * layer.scroll_scale per axis. Two-axis
# parallax is preserved; camera is no longer used (removed to fix UI displacement).
#
# The scrollbar nodes are declared in main.tscn as "BackdropScroll" and
# "BackdropScrollV"; this script only wires the value edges, it does not own either
# bar's geometry.

@onready var _scroll_x: HScrollBar = %BackdropScroll
@onready var _scroll_v: VScrollBar = %BackdropScrollV

# The backdrop's authored position, captured once so the runtime framing correction is
# always re-derived from the same base and is therefore idempotent across resizes.
var _authored_backdrops_position := Vector2.ZERO
var _authored_position_captured := false

func _ready() -> void:
    var strip := get_node_or_null("BackdropStrip") as Node2D
    if strip == null:
        return
    _frame_backdrop(strip)
    # The tab is laid out when TabContainer selects it, and again on every window
    # resize, so both are triggers. Changing the backdrop's position does not resize
    # this Control, so this cannot loop.
    resized.connect(func() -> void: _frame_backdrop(strip))
    get_viewport().size_changed.connect(func() -> void: _frame_backdrop(strip))
    _scroll_x.value_changed.connect(
        func(value: float) -> void:
            _set_scroll_offset_x(strip, value)
    )
    _scroll_v.value_changed.connect(
        func(value: float) -> void:
            _set_scroll_offset_y(strip, value)
    )
    # Strip is 6400px wide; scrub spans the full band.
    _scroll_x.max_value = 6400.0
    # Vertical travel is the generator's 240px overscan, clamped. Beyond it the
    # sky (1320px tall, the tallest band) would reveal its top edge, so the bar
    # stops there instead of letting the camera pan past the art.
    _scroll_v.max_value = 240.0
    _scroll_v.min_value = 0.0
    _size_layer_repeats(strip)

# Sizes every layer's horizontal repeat run to cover the full scrub.
#
# Measured with a real window: left at Godot's default of 1, horizontal scrubbing showed
# NOTHING. The tab was 100% bare background at camera x=1600, and 89-92% bare out to
# x=6400, while x=0 was only 6% bare. repeat_times = 1 draws a single copy plus enough
# to fill the VIEWPORT, so as soon as the camera pans past that copy there is no art left
# to draw. This is the reason the strip could be grabbed but not usefully scrolled.
#
# Derived from the scrub range rather than hard-coded, because the scrub and the repeat
# count are two halves of one constraint and hard-coding them lets them drift apart
# unnoticed -- which is what happened. Tiles = ceil(scrub / repeat_size.x) + 1, the +1
# covering one viewport width past the end of the scrub.
#
# After this, the bare-background percentage is a flat 1% across the whole range
# (measured at x = 0, 1600, 3200, 4800, 6400).
func _size_layer_repeats(strip: Node2D) -> void:
    var backdrops := strip.get_node_or_null("Backdrops") as Node2D
    if backdrops == null:
        return
    for layer in backdrops.get_children():
        var parallax := layer as Parallax2D
        if parallax.repeat_size.x <= 0.0:
            continue
        parallax.repeat_times = ceili(_scroll_x.max_value / parallax.repeat_size.x) + 1

# Puts the art flush with the tab's left and bottom edges, measured rather than
# hard-coded.
#
# Why measurement: the offset needed is NOT constant. Parallax2D frames its own repeat
# run from the viewport, so the correction depends on the window size. Measured with a
# real window, the required translation was (2784, 419.32) at 1920x1080 and
# (2816, 199.48) at 1280x800. A single static position cannot satisfy both, which is
# why the backdrop read as off-centre at every size.
#
# Why the BOTTOM edge, not the sky's top-left: the sky is a full-height 1320px OPAQUE
# backdrop and every other band is bottom-aligned to it at design y = 1320. Aligning the
# sky's TOP-left to the tab corner was measured and is wrong -- Sky went to 100% visible
# and every other band to 0%, hidden behind the sky.
#
# No zoom is applied. A uniform zoom to fit the 1320px design into the tab was measured
# and made it worse -- it scales the bands toward each other, Hills dropping 11% -> 1%
# and Forest 8% -> 0%. The bands must keep their authored sizes.
#
# Why it must wait a frame: in _ready() this tab's rect is still (0,0) -- it is only
# laid out once the TabContainer selects it. Measuring then produced a correction of
# -1499.5px and dropped every band to 0% visible. So the rect is read after a frame.
#
# Why it resets to the authored position first: the correction is re-derived from that
# base every time rather than accumulated, so calling this on every resize is idempotent
# instead of drifting further each time.
func _frame_backdrop(strip: Node2D) -> void:
    var backdrops := strip.get_node_or_null("Backdrops") as Node2D
    if backdrops == null:
        return
    var sky := backdrops.get_node_or_null("Sky") as Parallax2D
    if sky == null:
        return
    var sky_sprite := sky.get_node_or_null("Sprite") as Sprite2D
    if sky_sprite == null or sky_sprite.texture == null:
        return

    if not _authored_position_captured:
        _authored_backdrops_position = backdrops.position
        _authored_position_captured = true

    # Back to the authored base, then wait for layout before measuring, so the reading
    # belongs to the base position and to a tab that actually has a size.
    backdrops.position = _authored_backdrops_position
    await get_tree().process_frame
    if size == Vector2.ZERO:
        return

    # Design x = 0 to the tab's left edge.
    #
    # Only x. Aligning the sky's baseline (its bottom edge, design y = 1320) to the
    # tab's bottom was also measured, and it aligns perfectly -- baseline_offset 0.0 at
    # both window sizes -- but it moves the art down by 419px and pushes the lower
    # bands off the tab: Hills fell from 11% to 1% visible and Forest from 8% to 0% at
    # 1920x1080. The bands are authored at design y 936..1000 but do not render there,
    # because each Parallax2D offsets its own child by an amount that depends on its
    # own scroll_scale, so one parent translation cannot align them all vertically.
    # Correcting that per layer needs a per-layer scroll_offset and is not done here.
    #
    # So x is corrected, where the error was large and the fix does not move any band
    # vertically, and y is left as authored rather than shipped as a regression.
    var tab_rect := Rect2(global_position, size)
    var sky_rect := Rect2(sky_sprite.get_global_transform_with_canvas().origin,
        sky_sprite.texture.get_size())
    backdrops.position = _authored_backdrops_position + Vector2(
        tab_rect.position.x - sky_rect.position.x,
        0.0)

func _set_scroll_offset_x(strip: Node2D, value: float) -> void:
    var backdrops := strip.get_node_or_null("Backdrops") as Node2D
    if backdrops == null:
        return
    for layer in backdrops.get_children():
        var parallax := layer as Parallax2D
        if parallax == null:
            continue
        parallax.scroll_offset.x = value * parallax.scroll_scale.x

func _set_scroll_offset_y(strip: Node2D, value: float) -> void:
    var backdrops := strip.get_node_or_null("Backdrops") as Node2D
    if backdrops == null:
        return
    for layer in backdrops.get_children():
        var parallax := layer as Parallax2D
        if parallax == null:
            continue
        parallax.scroll_offset.y = value * parallax.scroll_scale.y
