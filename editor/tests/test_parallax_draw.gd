extends SceneTree
# Issue #107 (parallax step 4) -- the ParallaxDraw draw seam.
#
# ParallaxDraw's contract with WorldView is deliberately thin: `apply()` stores what it is handed
# and `_draw()` reads only that. So this suite asserts the DATA contract, not pixels: G5, G7 and
# G12 in test_parallax_pixels.gd are where pixels are asserted, and they need a real window --
# headless Godot runs RendererDummy, which has no framebuffer to read back.
#
# Guards, issue #81's silent-truncation mode: a guard that dies partway would otherwise let the
# suite print failures=0, so every guard appends its name to `_completed` and the runner asserts
# all of them are present. The exit code is what gates, never the printed counter.
#
#   D1  apply() -> last_placements() hands back the SAME dictionaries, unchanged: the drawer
#       performs no recomputation of its own. Half of G1; WorldView's split is the other half.
#   D2  a second apply() REPLACES the first -- clear-on-redraw, asserted at the data level.
#   D3  an empty apply() clears the drawn set.
#   D4  `_fill_colour` accepts exactly the two wire spellings and falls back to transparent.
#   D5  `_draw()` survives a placement missing every optional key (defensive null path).

var _failures: int = 0
var _completed: Array[String] = []

const DRAW_SCRIPT_PATH: String = "res://scripts/parallax_draw.gd"
# The transparent wire token, spelled once. `parallax_draw.gd` owns the constant; repeating the
# literal here is the on-disk pin the wire-token rule asks for -- a rename has to be caught
# somewhere that does not move with the code.
const FILL_TRANSPARENT_TOKEN: String = "transparent"
const FILL_SKY_RGBA: String = "#8A98ACFF"

func check(condition: bool, message: String) -> void:
    if condition:
        print("ok: " + message)
    else:
        _failures += 1
        printerr("FAIL: " + message)

func _initialize() -> void:
    _run.call_deferred()

# A minimal placement, shaped like the bridge's own PlacementEntry. Every optional key the
# drawer reads is present; D5 strips them.
func _placement(copy_count: int) -> Dictionary:
    # `file` is the BARE filename: apply() resolves it against stack_dir, because rewriting the
    # placement would break the G1 equality against the bridge's own array.
    return {
        "name": "Sky", "file": "base.png",
        "plane": "back", "copy_count": copy_count,
        "period_px": 1920.0, "first_copy_left_px": 0.0,
        "screen_top_px": 120.0, "art_h_px": 960.0, "scale": 1.0,
        "fill_above": FILL_SKY_RGBA, "fill_below": FILL_TRANSPARENT_TOKEN,
    }

func _new_drawer() -> Node2D:
    var script: GDScript = load(DRAW_SCRIPT_PATH) as GDScript
    if script == null:
        check(false, "ParallaxDraw script failed to load from " + DRAW_SCRIPT_PATH)
        return null
    var drawer := script.new() as Node2D
    if drawer == null:
        check(false, "ParallaxDraw did not instantiate as Node2D")
        return null
    root.add_child(drawer)
    return drawer

func _run() -> void:
    # `_test_draw_seam` awaits (D5 needs two real frames), so it must be awaited here: calling a
    # coroutine without `await` returns at its first suspension and the guard check below would
    # run before D5 appended.
    await _test_draw_seam()
    _assert_all_guards_completed()
    print("=== test_parallax_draw: %d failures ===" % _failures)
    quit(0 if _failures == 0 else 1)

func _assert_all_guards_completed() -> void:
    for guard_name in ["d1", "d2", "d3", "d4", "d5"]:
        if not _completed.has(guard_name):
            _failures += 1
            printerr("FAIL: guard '%s' did not complete" % guard_name)

func _test_draw_seam() -> void:
    print("--- ParallaxDraw draw seam (issue #107) ---")
    var drawer := _new_drawer()
    if drawer == null:
        _completed.append_array(["d1", "d2", "d3", "d4", "d5"])
        return

    var first_set: Array = [_placement(3)]
    var second_set: Array = [_placement(5)]

    # --- D1: apply() stores, it does not recompute ---
    drawer.call("apply", first_set, 1920.0, 1080.0, "res://tests/fixtures")
    var handed_back: Array = drawer.call("last_placements")
    check(handed_back == first_set,
        "D1: last_placements() equals the array handed to apply()")
    check(is_same(handed_back[0], first_set[0]),
        "D1: the entries are the SAME dictionaries, not rebuilt copies")
    _completed.append("d1")

    # --- D2: a second apply replaces ---
    drawer.call("apply", second_set, 1920.0, 1080.0, "res://tests/fixtures")
    check(drawer.call("last_placements") == second_set,
        "D2: a second apply() replaces the first, it does not append")
    _completed.append("d2")

    # --- D3: an empty apply clears ---
    drawer.call("apply", [], 1920.0, 1080.0, "res://tests/fixtures")
    check((drawer.call("last_placements") as Array).is_empty(),
        "D3: an empty apply() clears the drawn set")
    _completed.append("d3")

    # --- D4: _fill_colour accepts the two wire spellings only ---
    check(drawer.call("_fill_colour", FILL_TRANSPARENT_TOKEN) == Color(0, 0, 0, 0),
        "D4: the transparent token yields alpha 0")
    check(drawer.call("_fill_colour", "") == Color(0, 0, 0, 0),
        "D4: an empty fill yields alpha 0")
    check(drawer.call("_fill_colour", FILL_SKY_RGBA) == Color(0x8A / 255.0, 0x98 / 255.0, 0xAC / 255.0, 1.0),
        "D4: #RRGGBBAA parses to that colour")
    check(drawer.call("_fill_colour", "#8A98AC") == Color(0, 0, 0, 0),
        "D4: a 6-digit hex is rejected to transparent, never drawn as a band")
    check(drawer.call("_fill_colour", "#8A98ACGG") == Color(0, 0, 0, 0),
        "D4: a non-hex digit is rejected to transparent")
    _completed.append("d4")

    # --- D5: _draw() survives a bare placement ---
    # No file, no period, no fill: the texture lookup misses, both fills are transparent, and
    # nothing draws. The assertion is that the next frame still runs -- a script error here would
    # abort the guard before it appends, which is exactly what the completed-guard check catches.
    drawer.call("apply", [{"name": "bare"}], 1920.0, 1080.0, "res://tests/fixtures")
    await process_frame
    await process_frame
    check((drawer.call("last_placements") as Array).size() == 1,
        "D5: a placement with no optional keys is stored and drawn without error")
    _completed.append("d5")
