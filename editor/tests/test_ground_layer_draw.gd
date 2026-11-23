extends SceneTree
# Guard for #126 step 8: the ground layer must render each tile's own sprite and must not
# fall back to the hand-authored 4:7 polygon (or the flat-tile rects) as its drawn surface.
# The polygon's top edge was AC1's old oracle; leaving it in place lets the ladder return
# silently. Headless cannot capture CanvasItem draw calls, so the guard reads the source text,
# exactly as the plan's ground_layer_draws_sprites_not_a_polygon row specifies.
#
# Pure source read on one thread, no race. The current (pre-change) file contains both
# draw_polygon and draw_rect and no sprite load, so every contract assertion is RED here;
# mutant M8 (restore the polygon) is the same state and must turn the polygon guard red.

var _failures: int = 0

func _initialize() -> void:
    _run.call_deferred()

func _check(condition: bool, description: String) -> void:
    if not condition:
        _failures += 1
        printerr("FAIL: " + description)

func _run() -> void:
    # Scan only live lines: a TODO(human) SAMPLE carries the same tokens as comments, so a raw
    # text scan would pass on an unfilled scaffold. Stripping comments keeps the guard RED until
    # the draw calls are actually code.
    var raw_source: String = FileAccess.get_file_as_string("res://scripts/world_view_ground.gd")
    var live_lines: PackedStringArray = PackedStringArray()
    for line in raw_source.split("\n"):
        if not line.strip_edges().begins_with("#"):
            live_lines.append(line)
    var source: String = "\n".join(live_lines)
    _check(source != "", "world_view_ground.gd source is readable")

    # The 4:7 ladder and the flat rects were AC1's old drawn surface; both are retired.
    _check(not source.contains("draw_polygon"), "the 4:7 draw_polygon is removed")
    _check(not source.contains("draw_rect("), "the flat-tile draw_rect calls are removed")

    # Sprites are the new drawn surface: a _32x32.png texture is loaded and drawn per cell.
    _check(source.contains("_32x32.png"), "tile sprites are loaded from _32x32.png paths")
    _check(source.contains("load("), "sprite textures are loaded at runtime")
    _check(source.contains("draw_texture"), "sprite textures are drawn, not merely loaded")

    print("ground_layer_draw failures=%d" % _failures)
    quit(0 if _failures == 0 else 1)
