extends Control
# ref #68 #74 — Simulator tab: a horizontal scrollbar scrubs the backdrop strip's
# Camera2D 1:1, so the near collision band visibly trails at 0.9x.
# The scrollbar node is declared in main.tscn as "BackdropScroll"; this script
# only wires the value edge, it does not own the bar's geometry.

@onready var _scroll: HScrollBar = %BackdropScroll

func _ready() -> void:
    var strip := get_node_or_null("BackdropStrip") as Node2D
    if strip == null:
        return
    var camera := strip.get_node("Camera2D") as Camera2D
    _scroll.value_changed.connect(
        func(value: float) -> void:
            camera.position.x = value
    )
    # Strip is 6400px wide; scrub spans the full band.
    _scroll.max_value = 6400.0
