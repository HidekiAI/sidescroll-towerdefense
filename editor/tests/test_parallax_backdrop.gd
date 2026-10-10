extends SceneTree
# ref #68 — the parallax backdrop restack contract.
#
# WHY DELTAS, NOT ABSOLUTE POSITIONS. The first spike read Parallax2D.position
# absolutely and got a ~2028px common constant on x that had nothing to do with the
# layer under test: Parallax2D's repeat wrap offsets the node by whole repeats of
# repeat_size, so an absolute reading measures where the wrap happened to sit. Two
# successive camera steps of equal size cancel that constant, leaving only the
# linear (1 - scroll_scale) term, and the measurement confirmed both samples agree
# exactly. So every drift assertion here is step-to-step.
#
# WHY NOT get_screen_offset(). It returned (88, 39) identically for Sky and
# Foreground — identical values for two layers with different scroll_scale prove it
# is a viewport-derived value, not per-layer drift. Asserting on it would pass
# identically for a working and a completely broken parallax.
#
# EXPECTED FORM. Measured: delta == camera_step * (1 - scroll_scale) per axis. A
# layer at scroll_scale 1.0 rides with the camera and shows zero drift; a layer at
# 0.1 barely moves; a layer at 1.3 (Foreground) moves backwards. The Foreground
# case matters because its drift has the OPPOSITE sign to the camera step, so a
# test that only checked magnitude would accept a sign error here.

const MAIN := "res://scenes/main.tscn"
const SIM_PATH := "TabContainer/Simulator"
const STRIP := "BackdropStrip"
const BACKDROPS := "Backdrops"

# Deliberately asymmetric: a swapped or copied axis fails instead of cancelling.
const CAMERA_STEP := Vector2(120.0, 40.0)
const DRIFT_TOLERANCE_PX := 0.5

# (layer node name, texture file, scroll_scale_x, scroll_scale_y)
# The file name is asserted literally, because a round-trip through the scene and
# manifest can rename a shared constant on both sides together and stay
# self-consistent while reading nothing. The on-disk name is the contract.
#
# The scale is asserted LITERALLY for the same reason, and this is not theoretical:
# mutation testing refuted two early drafts of this test for exactly this reason.
# When expected drift was derived as CAMERA_STEP * (1 - layer.scroll_scale), the
# expectation read the value under test, so editing Hills to (0.42, 0.5) or
# Foreground to (1.3, 1.3) changed both sides together and the suite stayed green —
# a perfect round-trip that asserted nothing. The drift assertions below are
# therefore about the RELATIONSHIP (1 - scale); these literals are what pin the
# relationship to the manifest's actual numbers.
const EXPECTED_LAYERS := [
    ["Sky", "sky.png", 0.1, 0.08],
    ["HighClouds", "clouds_high.png", 0.2, 0.17],
    ["LowClouds", "clouds_low.png", 0.3, 0.25],
    ["Hills", "hills.png", 0.5, 0.42],
    ["Forest", "forest.png", 0.7, 0.6],
    ["Foreground", "foreground.png", 1.3, 1.15],
]

var _failures: int = 0

func _init() -> void:
    var main: Node = (load(MAIN) as PackedScene).instantiate()
    root.add_child(main)
    var tabs: TabContainer = main.get_node("TabContainer") as TabContainer
    tabs.set_current_tab(4)
    await process_frame

    var sim: Control = main.get_node(SIM_PATH) as Control

    # --- the strip must actually be shown; this is the bug that started it all ---
    var strip: Node2D = sim.get_node("BackdropView/Viewport/BackdropStrip") as Node2D
    if not strip.visible:
        _fail("BackdropStrip is hidden; the backdrop renders nothing at all")
    var camera: Camera2D = sim.get_node_or_null("BackdropView/Viewport/Camera2D") as Camera2D
    if camera == null:
        _fail("BackdropStrip has no Camera2D to drive")
        _report()
        return

    # --- Part 1: both scrollbars reach the camera, on their own axis ---
    var scroll_x: HScrollBar = sim.get_node("BackdropScroll") as HScrollBar
    var scroll_v: VScrollBar = sim.get_node("BackdropScrollV") as VScrollBar
    if scroll_x == null:
        _fail("Simulator tab has no BackdropScroll bar for the horizontal axis")
    if scroll_v == null:
        _fail("Simulator tab has no BackdropScrollV bar for the vertical axis")
    if scroll_x == null or scroll_v == null:
        _report()
        return

    # Scrubbing one axis must leave the other alone. This is what makes the two-axis
    # wiring real rather than one bar writing a whole shared Vector2. Each axis is
    # checked from a neutral baseline, because the point is what a single scrub
    # CHANGES, not what the camera happens to hold afterwards: the x bar must move
    # x while y stays exactly at zero, and the y bar must then move y while x stays
    # at whatever x was (proving the y handler preserves x rather than resetting it).
    scroll_x.value = 640.0
    await process_frame
    if not is_equal_approx(camera.position.x, 640.0):
        _fail("BackdropScroll did not set camera.x: got %.2f want 640.0" % camera.position.x)
    if not is_zero_approx(camera.position.y):
        _fail("BackdropScroll leaked into camera.y: got %.2f want 0.0" % camera.position.y)
    var held_x: float = camera.position.x
    scroll_v.value = 96.0
    await process_frame
    if not is_equal_approx(camera.position.x, held_x):
        _fail("BackdropScrollV disturbed camera.x: got %.2f want %.2f (held)" % [camera.position.x, held_x])
    if not is_equal_approx(camera.position.y, 96.0):
        _fail("BackdropScrollV did not set camera.y: got %.2f want 96.0" % camera.position.y)

    # Vertical travel is clamped to the generator's 240px overscan: past it the
    # 1320px sky band would reveal its own top edge.
    if not is_equal_approx(scroll_v.max_value, 240.0):
        _fail("BackdropScrollV max_value is not the 240px overscan clamp: got %.2f want 240.0" % scroll_v.max_value)
    # Horizontal scrub spans the full 6400px strip.
    if not is_equal_approx(scroll_x.max_value, 6400.0):
        _fail("BackdropScroll max_value is not the 6400px strip: got %.2f want 6400.0" % scroll_x.max_value)

    # --- Part 2: measured per-layer drift, both axes, all six layers ---
    var layers: Array[Parallax2D] = []
    for entry in EXPECTED_LAYERS:
        var layer_name: String = entry[0]
        var found: Parallax2D = sim.get_node_or_null(
            "BackdropView/Viewport/BackdropStrip/Backdrops/%s" % layer_name) as Parallax2D
        if found == null:
            _fail("backdrop layer %s is missing from the stack" % layer_name)
            layers.append(null)
            continue
        layers.append(found)

    if _failures > 0:
        _report()
        return

    # Two equal steps; their difference cancels Parallax2D's repeat-wrap constant.
    var reading_first: Array[Vector2] = []
    for layer in layers:
        reading_first.append(layer.position)
    camera.position += CAMERA_STEP
    await process_frame
    await process_frame
    var reading_second: Array[Vector2] = []
    for layer in layers:
        reading_second.append(layer.position)

    camera.position += CAMERA_STEP
    await process_frame
    await process_frame
    var reading_third: Array[Vector2] = []
    for layer in layers:
        reading_third.append(layer.position)

    for entry_index in EXPECTED_LAYERS.size():
        var layer: Parallax2D = layers[entry_index]
        var scale: Vector2 = layer.scroll_scale
        # Pin the literal value BEFORE deriving the expectation from it. Without
        # this, every drift assertion below is self-consistent under any scale and
        # proves nothing (see the mutation refutations recorded above).
        if not is_equal_approx(scale.x, float(EXPECTED_LAYERS[entry_index][2])):
            _fail("%s scroll_scale.x is %.4f but the manifest says %.4f" % [
                layer.name, scale.x, float(EXPECTED_LAYERS[entry_index][2])])
        if not is_equal_approx(scale.y, float(EXPECTED_LAYERS[entry_index][3])):
            _fail("%s scroll_scale.y is %.4f but the manifest says %.4f" % [
                layer.name, scale.y, float(EXPECTED_LAYERS[entry_index][3])])
        var expected_delta: Vector2 = CAMERA_STEP * Vector2(1.0 - scale.x, 1.0 - scale.y)
        var delta_a: Vector2 = reading_second[entry_index] - reading_first[entry_index]
        # Consecutive windows of the SAME width. This must be reading_third minus
        # reading_second: reading_third minus reading_first spans TWO steps and is
        # therefore twice as large, so comparing it against delta_a would fail on a
        # correct implementation for the wrong reason.
        var delta_b: Vector2 = reading_third[entry_index] - reading_second[entry_index]
        # Two independent samples of the same quantity: if these disagree, the
        # quantity is not actually a function of the step and the assertion below
        # would be measuring noise.
        if delta_a.distance_to(delta_b) > DRIFT_TOLERANCE_PX:
            _fail("%s drift is not reproducible: A=%s B=%s" % [layer.name, delta_a, delta_b])
        if delta_a.distance_to(expected_delta) > DRIFT_TOLERANCE_PX:
            _fail("%s drift is %.2f,%.2f but %s * (1 - scale) wants %.2f,%.2f" % [
                layer.name, delta_a.x, delta_a.y, CAMERA_STEP, expected_delta.x, expected_delta.y])
        # The sign must be right too. Foreground at 1.3 rides backwards, so a
        # magnitude-only check would accept a sign flip there.
        if not is_equal_approx(delta_a.x, expected_delta.x):
            _fail("%s x drift sign/value wrong: got %.2f want %.2f" % [layer.name, delta_a.x, expected_delta.x])
        # Vertical must be real, not just a copy of x.
        if not is_equal_approx(delta_a.y, expected_delta.y):
            _fail("%s y drift sign/value wrong: got %.2f want %.2f" % [layer.name, delta_a.y, expected_delta.y])

    # --- Part 3: on-disk contract, asserted literally ---
    # A round-trip test cannot catch a rename of a constant shared by writer and
    # reader, because both sides move together. The file names are asserted here.
    for entry_index in EXPECTED_LAYERS.size():
        var layer: Parallax2D = layers[entry_index]
        var sprite: Sprite2D = layer.get_node("Sprite") as Sprite2D
        var want_file: String = EXPECTED_LAYERS[entry_index][1]
        var got_path: String = sprite.texture.resource_path
        if not got_path.ends_with("/" + want_file):
            _fail("%s draws the wrong texture: got %s want *%s" % [layer.name, got_path, want_file])
        if sprite.centered:
            _fail("%s sprite is centered; repeat tiling needs centered = false" % layer.name)
        if not is_equal_approx(layer.repeat_size.x, 1920.0):
            _fail("%s repeat_size.x is not the authored 1920px: got %.2f" % [layer.name, layer.repeat_size.x])
        if not is_zero_approx(layer.repeat_size.y):
            _fail("%s repeat_size.y must be 0 (no vertical repeat): got %.2f" % [layer.name, layer.repeat_size.y])

    _report()

func _fail(message: String) -> void:
    _failures += 1
    print("[FAIL] ", message)

func _report() -> void:
    if _failures == 0:
        print("[GREEN] 6 layers drift per (1 - scroll_scale) on BOTH axes; both bars drive their own axis")
        quit(0)
    else:
        print("[RED] ", _failures, " failure(s)")
        quit(1)
