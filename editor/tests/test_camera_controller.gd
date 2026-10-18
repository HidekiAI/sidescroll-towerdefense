extends SceneTree
# Issue #107 (parallax step 4) -- the game CameraController: G6, G13, and the small-bound clamp.
#
# The controller is driven through its PUBLIC API only (set_map_bound / set_view_extent_px /
# set_zoom / set_target_world_px). `_clamp_center` is transcribed from
# `sstd-core::parallax::clamp_center` and is private, so the small-bound case is asserted by
# handing the controller a bound narrower than the view and reading the camera -- which is also
# the only way a caller can reach it.
#
# The map numbers are the ones Math section 5 uses: a one-screen-wide, two-screen-tall map is
# `[0, 1920) x [0, 2112)` in world px, and the view is the ref 1920 x 1080. G13's own example --
# the player render position moving from (960, 1572) to (2000, 700) -- is reproduced literally,
# because those two points are what the wiki row pins.
#
# Headless has no renderer (Math section 7). Nothing here proves anything is drawn.
#
# Guards run in the #107 style: each appends its name to `_completed`, the runner fails on a
# missing name, and the exit code gates -- never the printed counter.

const CAMERA_SCRIPT_PATH: String = "res://scripts/camera_controller.gd"

# One screen wide, two screens tall: `[0, 1920) x [0, 2112)`.
const MAP_BOUND_LOW_PX: Vector2 = Vector2(0.0, 0.0)
const MAP_BOUND_HIGH_PX: Vector2 = Vector2(1920.0, 2112.0)
const VIEW_EXTENT_PX: Vector2 = Vector2(1920.0, 1080.0)

# G13's own two endpoints. With the bound above, x is pinned at 960 (the bound is exactly one
# view wide) while y is free in [540, 1572], so the y half of the move is what proves BOTH axes
# are followed -- "follow x only" leaves the camera at 1572.
const G13_START_PX: Vector2 = Vector2(960.0, 1572.0)
const G13_END_PX: Vector2 = Vector2(2000.0, 700.0)
const G13_STEPS: int = 10

# Math section 5's zoom-0.25 example: view 4320 against a bound of 4224, so there is no range to
# clamp into and the centre is fixed at bound_low + half a view = 1104. The bound's MIDPOINT is
# 1056, which is what the source comment calls "the one that gets written wrong" -- asserting the
# exact 1104 is what separates the two.
const SMALL_BOUND_LOW_Y_PX: float = -1056.0
const SMALL_BOUND_HIGH_Y_PX: float = 3168.0
const SMALL_BOUND_VIEW_EXTENT_PX: float = 4320.0
const SMALL_BOUND_EXPECTED_CENTER_Y_PX: float = 1104.0
const SMALL_BOUND_MIDPOINT_Y_PX: float = 1056.0

const FRAMES_TO_SETTLE: int = 2

const GUARDS: Array[String] = ["g6", "g13", "small_bound"]

var _failures: int = 0
var _completed: Array[String] = []


func check(condition: bool, message: String) -> void:
    if condition:
        print("ok: " + message)
    else:
        _failures += 1
        printerr("FAIL: " + message)


func _initialize() -> void:
    _run.call_deferred()


func _run() -> void:
    await _test_camera_controller()
    _assert_all_guards_completed()
    print("=== test_camera_controller: %d failures ===" % _failures)
    quit(0 if _failures == 0 else 1)


func _assert_all_guards_completed() -> void:
    for guard_name in GUARDS:
        if not _completed.has(guard_name):
            _failures += 1
            printerr("FAIL: guard '%s' did not complete" % guard_name)


func _test_camera_controller() -> void:
    print("--- CameraController (issue #107) ---")
    var controller_script: GDScript = load(CAMERA_SCRIPT_PATH) as GDScript
    if controller_script == null:
        check(false, "camera_controller.gd failed to load from " + CAMERA_SCRIPT_PATH)
        _completed.append_array(GUARDS)
        return
    var controller := controller_script.new() as Node
    if controller == null:
        check(false, "camera_controller.gd did not instantiate as a Node")
        _completed.append_array(GUARDS)
        return
    root.add_child(controller)

    var camera: Camera2D = Camera2D.new()
    root.add_child(camera)
    controller.call("attach", camera)
    controller.call("set_map_bound", MAP_BOUND_LOW_PX, MAP_BOUND_HIGH_PX)
    controller.call("set_view_extent_px", VIEW_EXTENT_PX.x, VIEW_EXTENT_PX.y)

    # --- G6: zoom stays 1, and the refusal restores it ---
    #
    # "Enforced, not merely refused" is the controller's own claim: it puts the zoom back, because
    # a zoom written by anything else would silently change the parallax ladder.
    camera.zoom = Vector2(2.0, 2.0)
    var accepted: bool = controller.call("set_zoom", 0.5)
    check(accepted == false, "G6: set_zoom(0.5) is refused")
    check(camera.zoom == Vector2(1.0, 1.0),
        "G6: the refusal RESTORES zoom to 1, it does not only say no (got %s)" % camera.zoom)
    check(controller.call("set_zoom", 1.0) == true, "G6: set_zoom(1.0) is accepted")
    _completed.append("g6")

    # One frame boundary before the loop, and it is load-bearing: `SceneTree.process_frame` is
    # emitted at the START of the frame, BEFORE the node `_process` callbacks run. A target set
    # and then awaited once therefore gets zero `_process` frames -- the first check would read a
    # camera the controller has not had a chance to move (measured: step 0 reported (0, 0) while
    # every later step was correct, because each later step inherits the previous frame).
    await process_frame

    # --- G6 + G13: one target, one frame, both axes ---
    #
    # The steps are the interpolated render position a game loop would hand in. After each
    # set_target_world_px, ONE process frame later the camera must already be there: a second
    # frame of movement would be the tween G6 forbids, and a camera still at its previous
    # position would be the lag G13 forbids.
    for step_index in range(G13_STEPS + 1):
        var fraction: float = float(step_index) / float(G13_STEPS)
        var target: Vector2 = G13_START_PX.lerp(G13_END_PX, fraction)
        controller.call("set_target_world_px", target.x, target.y)
        await process_frame
        var expected: Vector2 = Vector2(
            clampf(target.x, MAP_BOUND_LOW_PX.x + VIEW_EXTENT_PX.x * 0.5,
                MAP_BOUND_HIGH_PX.x - VIEW_EXTENT_PX.x * 0.5),
            clampf(target.y, MAP_BOUND_LOW_PX.y + VIEW_EXTENT_PX.y * 0.5,
                MAP_BOUND_HIGH_PX.y - VIEW_EXTENT_PX.y * 0.5))
        check(camera.position == expected,
            "G13: step %d follows on BOTH axes in the same frame (target %s -> camera %s, want %s)" % [
                step_index, target, camera.position, expected])

    # The y half really moved, so "follow x only" could not have passed the loop above.
    check(G13_START_PX.y != G13_END_PX.y,
        "G13: the fixture's y really changes, so the loop can tell x-only following from both-axis")
    # And the x target was OUTSIDE the bound on purpose: it is clamped to the pinned 960 rather
    # than followed to 2000, which is the bound doing its job on a real input.
    check(camera.position.x == MAP_BOUND_HIGH_PX.x * 0.5,
        "G13: an out-of-bound target is clamped on x (got %s)" % camera.position.x)
    _completed.append("g13")

    # --- the small-bound clamp ---
    #
    # A bound narrower than the view has no range to clamp into, so the centre is fixed at
    # bound_low + half a view rather than following the target or sitting at the midpoint.
    controller.call("set_map_bound", Vector2(MAP_BOUND_LOW_PX.x, SMALL_BOUND_LOW_Y_PX),
        Vector2(MAP_BOUND_HIGH_PX.x, SMALL_BOUND_HIGH_Y_PX))
    controller.call("set_view_extent_px", SMALL_BOUND_VIEW_EXTENT_PX, SMALL_BOUND_VIEW_EXTENT_PX)
    controller.call("set_target_world_px", 3000.0, 3000.0)
    await process_frame
    check(is_equal_approx(camera.position.y, SMALL_BOUND_EXPECTED_CENTER_Y_PX),
        "small_bound: a bound narrower than the view fixes the centre at bound_low + half a view (%.1f, want %.1f)" % [
            camera.position.y, SMALL_BOUND_EXPECTED_CENTER_Y_PX])
    check(not is_equal_approx(camera.position.y, SMALL_BOUND_MIDPOINT_Y_PX),
        "small_bound: and NOT at the bound's midpoint %.1f, which is the documented wrong answer" %
            SMALL_BOUND_MIDPOINT_Y_PX)
    _completed.append("small_bound")

    for frame_index in range(FRAMES_TO_SETTLE):
        await process_frame
