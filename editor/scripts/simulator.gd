extends Control
# ref #68 #74 — Simulator tab: two scrollbars scrub the backdrop strip's Camera2D.
# The horizontal bar scrubs world x, the vertical bar scrubs world y, and each is
# applied on its own axis of one shared Vector2. That is what makes both axes of
# parallax observable: the tutorial's Parallax2D multiplies the camera offset by
# scroll_scale per axis, so a layer at 0.1x drifts a tenth as far as the camera and
# a layer at 1.3x (Foreground) outruns it. Neither bar owns the camera alone.
#
# The scrollbar nodes are declared in main.tscn as "BackdropScroll" and
# "BackdropScrollV"; this script only wires the value edges, it does not own either
# bar's geometry.

@onready var _scroll_x: HScrollBar = %BackdropScroll
@onready var _scroll_v: VScrollBar = %BackdropScrollV

func _ready() -> void:
    var strip := get_node_or_null("BackdropStrip") as Node2D
    if strip == null:
        return
    var camera := strip.get_node("Camera2D") as Camera2D
    _scroll_x.value_changed.connect(
        func(value: float) -> void:
            camera.position = Vector2(value, camera.position.y)
    )
    _scroll_v.value_changed.connect(
        func(value: float) -> void:
            camera.position = Vector2(camera.position.x, value)
    )
    # Strip is 6400px wide; scrub spans the full band.
    _scroll_x.max_value = 6400.0
    # Vertical travel is the generator's 240px overscan, clamped. Beyond it the
    # sky (1320px tall, the tallest band) would reveal its top edge, so the bar
    # stops there instead of letting the camera pan past the art.
    _scroll_v.max_value = 240.0
    _scroll_v.min_value = 0.0
