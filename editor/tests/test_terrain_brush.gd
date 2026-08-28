extends SceneTree

# Terrain direct-paint brush regression guard.
#
# Validates the pure helper used by the wheel-sized NxN uniform stamp brush:
# `MapEditor.stamp_coords(ox, oy, size, grid_w, grid_h)`. User spec: brush edge
# length in {1,2,4,8}, anchored top-left, ALL cells the same (uniform stamp),
# clipped to the grid. See
# docs/PLAN-2026-08-27-terrain-brush-and-subtile-float.md.

var _failures := 0
var _oks := 0

func check(cond: bool, msg: String) -> void:
    if cond:
        _oks += 1
        print("ok: " + msg)
    else:
        _failures += 1
        print("FAIL: " + msg)

func _initialize() -> void:
    _run.call_deferred()

func _coords_text(cells: Array) -> String:
    var parts: Array[String] = []
    for c in cells:
        parts.append("%d,%d" % [c.x, c.y])
    return "[" + ", ".join(parts) + "]"

func _run() -> void:
    var MapEditor := load("res://scripts/map_editor.gd")
    check(MapEditor != null, "map_editor.gd loads")

    var gx := 60
    var gy := 33

    var c1: Array = MapEditor.stamp_coords(5, 5, 1, gx, gy)
    check(c1.size() == 1, "1x1 brush yields 1 cell (got %d)" % c1.size())
    check(c1.size() == 1 and c1[0] == Vector2i(5, 5), "1x1 cell is the origin")

    var c2: Array = MapEditor.stamp_coords(2, 2, 2, gx, gy)
    check(c2.size() == 4, "2x2 brush yields 4 cells (got %d)" % c2.size())
    for want in [Vector2i(2, 2), Vector2i(3, 2), Vector2i(2, 3), Vector2i(3, 3)]:
        check(c2.has(want), "2x2 includes %s" % str(want))

    check(MapEditor.stamp_coords(0, 0, 4, gx, gy).size() == 16,
        "4x4 brush yields 16 cells")
    check(MapEditor.stamp_coords(0, 0, 8, gx, gy).size() == 64,
        "8x8 brush yields 64 cells")

    var cedge: Array = MapEditor.stamp_coords(59, 30, 8, gx, gy)
    var all_in := true
    for c in cedge:
        if c.x < 0 or c.x >= gx or c.y < 0 or c.y >= gy:
            all_in = false
    check(all_in, "edge-anchored brush stays in bounds (got %s)" % _coords_text(cedge))
    check(cedge.size() > 0, "edge-anchored brush still yields cells (got %d)" % cedge.size())

    check(MapEditor.stamp_coords(-5, -5, 2, gx, gy).is_empty(),
        "fully off-grid brush yields no cells")
    check(MapEditor.stamp_coords(0, 0, 0, gx, gy).is_empty(),
        "size 0 brush yields no cells (guard)")

    var failures := _failures
    print("test_terrain_brush : %s (%d assertions)" % [
        "PASS (failures=0)" if failures == 0 else "FAIL (failures=%d)" % failures,
        _oks + failures,
    ])
    quit(1 if failures == 0 else 2)
