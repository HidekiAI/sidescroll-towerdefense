extends SceneTree
# Issue #119 (parallax step 6) -- the Simulator camera controls: G8, G14, G18 and G20.
#
# The camera used to have no control at all: #107 deleted the scrollbars that were its only
# input. This suite is the gate for the replacement (TDD_Parallax-Scrolling section 4.1):
# middle-drag pan, arrow-key pan, Ctrl+wheel and `+` / `-` over the zoom ladder, and centring on
# the active screen when the Simulator tab is entered.
#
# Two fixtures, and the split is deliberate:
#   - G8 / G14 / G18 use a BARE `world_view.tscn` with an injected `ScreenStore` and grid
#     config, so nothing is read from `user://data/` and a world on the host machine cannot move
#     an expected centre. They also set the panel to a fixed 1920x1080, because every bound
#     expectation below is `bound +- view / 2` and a window-manager-dependent panel size would
#     silently change `view`.
#   - G20 uses `main.tscn`, because "entering the tab centres on the active screen" is a rule
#     about `main.gd`'s wiring. It selects the tab through `TabContainer.current_tab` and never
#     by calling `_on_tab_changed` directly: a direct call leaves the tab unselected, the
#     SubViewport at 0x0, and the clamp an identity -- a measurement of nothing.
#
# Guards run in the #107 style: each appends its name to `_completed`, the runner fails on a
# missing name, and the EXIT CODE gates -- never the printed counter.
#
# Windowed, not headless: with no renderer (Math section 7) the SubViewport has no size and every
# view-dependent expectation is a measurement of the harness. Run with `DISPLAY=:0.0`.

const WORLD_VIEW_SCENE: String = "res://scenes/world_view.tscn"
const MAIN_SCENE: String = "res://scenes/main.tscn"
const SIMULATOR_TAB_INDEX: int = 4
const MAP_TAB_INDEX: int = 2
const PLACEMENT_TAB_INDEX: int = 3

# 60 x 33 tiles at 32 px, the grid the project ships (project.godot's 1920x1080 viewport).
const SCREEN_SIZE_PX: Vector2 = Vector2(1920.0, 1056.0)
const GRID_CONFIG: Dictionary = {
    "max_tiles_per_screen_x": 60,
    "max_tiles_per_screen_y": 33,
    "tile_width_in_pixels": 32,
    "tile_height_in_pixels": 32,
}
const PANEL_SIZE_PX: Vector2 = Vector2(1920.0, 1080.0)
const FRAMES_TO_SETTLE: int = 2

const GUARDS: Array[String] = ["g8", "g14", "g18", "g20"]

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
    await _test_g8()
    await _test_g14()
    await _test_g18()
    await _test_g20()
    _assert_all_guards_completed()
    print("=== test_world_view_controls: %d failures ===" % _failures)
    quit(0 if _failures == 0 else 1)


func _assert_all_guards_completed() -> void:
    for guard_name in GUARDS:
        if not _completed.has(guard_name):
            _failures += 1
            printerr("FAIL: guard '%s' did not complete" % guard_name)


# --- the fixture ------------------------------------------------------------------


# A bare WorldView with a fixed panel size and an injected store. `cells` are the screen grid
# positions to register, numbered from 1 in the order given.
func _make_world_view(cells: Array[Vector2i]) -> Control:
    var world_view: Control = (load(WORLD_VIEW_SCENE) as PackedScene).instantiate() as Control
    root.add_child(world_view)
    # Detach from the window's layout, then size the panel by hand: the bound expectations are
    # arithmetic on `view`, so a window-manager-dependent size would move them.
    world_view.set_anchors_preset(Control.PRESET_TOP_LEFT)
    world_view.size = PANEL_SIZE_PX
    world_view.set_grid_config(GRID_CONFIG)
    world_view.set_screen_store(_store_for(cells))
    await process_frame
    return world_view


func _store_for(cells: Array[Vector2i]) -> ScreenStore:
    var store: ScreenStore = ScreenStore.new()
    var screen_id: int = 1
    for cell in cells:
        store.register(cell.x, cell.y, screen_id, "s%d.gd" % screen_id)
        screen_id += 1
    return store


func _drop(world_view: Control) -> void:
    root.remove_child(world_view)
    world_view.queue_free()


# --- event helpers ----------------------------------------------------------------


func _wheel_event(ctrl: bool, wheel_up: bool, at_px: Vector2) -> InputEventMouseButton:
    var wheel: InputEventMouseButton = InputEventMouseButton.new()
    wheel.button_index = MOUSE_BUTTON_WHEEL_UP if wheel_up else MOUSE_BUTTON_WHEEL_DOWN
    # A wheel has no release event; Godot sends it with pressed = true only.
    wheel.pressed = true
    wheel.position = at_px
    wheel.ctrl_pressed = ctrl
    wheel.command_or_control_autoremap = false
    return wheel


func _middle_event(pressed: bool, at_px: Vector2) -> InputEventMouseButton:
    var middle: InputEventMouseButton = InputEventMouseButton.new()
    middle.button_index = MOUSE_BUTTON_MIDDLE
    middle.pressed = pressed
    middle.position = at_px
    return middle


func _motion_event(relative_px: Vector2, at_px: Vector2, middle_held: bool = true) -> InputEventMouseMotion:
    var motion: InputEventMouseMotion = InputEventMouseMotion.new()
    motion.relative = relative_px
    motion.position = at_px
    if middle_held:
        motion.button_mask = MOUSE_BUTTON_MASK_MIDDLE
    return motion


func _key_event(keycode_value: int, pressed: bool, echo: bool) -> InputEventKey:
    var key: InputEventKey = InputEventKey.new()
    key.keycode = keycode_value
    key.physical_keycode = keycode_value
    key.pressed = pressed
    key.echo = echo
    return key


# Deliver one event the way the engine would, then let the camera write settle. `push_input` is
# used rather than calling `_input` directly so the visibility and rect guards run as shipped.
func _send(world_view: Control, event: InputEvent) -> void:
    world_view.get_viewport().push_input(event)


# The world point under a point inside the panel, at the camera's current zoom. This is the
# invariant an anchored zoom must preserve, so it is computed from the camera rather than from
# the implementation's own formula.
func _world_point_at(world_view: Control, point_in_panel_px: Vector2) -> Vector2:
    var camera: Camera2D = world_view.camera() as Camera2D
    var view_px: Vector2 = (world_view.viewport() as SubViewport).size
    var point_from_panel_topleft_px: Vector2 = point_in_panel_px - world_view.get_global_rect().position
    return camera.position + (point_from_panel_topleft_px - view_px * 0.5) / camera.zoom.x


# --- G8: zoom keeps the world point under the cursor fixed ------------------------


func _test_g8() -> void:
    print("--- G8: anchored zoom ---")
    var world_view: Control = await _make_world_view(
        [Vector2i(0, 0), Vector2i(1, 0), Vector2i(2, 0), Vector2i(3, 0),
         Vector2i(0, 1), Vector2i(1, 1), Vector2i(2, 1), Vector2i(3, 1),
         Vector2i(0, 2), Vector2i(1, 2), Vector2i(2, 2), Vector2i(3, 2),
         Vector2i(0, 3), Vector2i(1, 3), Vector2i(2, 3), Vector2i(3, 3)])
    var viewport_size: Vector2i = (world_view.viewport() as SubViewport).size
    check(viewport_size == Vector2i(PANEL_SIZE_PX), "g8: the panel is the fixed %s (got %s)" % [
        Vector2i(PANEL_SIZE_PX), viewport_size])
    check(world_view.is_processing_input(), "g8: the view processes input")

    world_view.move_camera_to(960.0, 528.0)
    var cursor_px: Vector2 = world_view.get_global_rect().position + Vector2(1500.0, 300.0)
    var world_point_before: Vector2 = _world_point_at(world_view, cursor_px)

    _send(world_view, _wheel_event(true, false, cursor_px))
    await process_frame

    var camera: Camera2D = world_view.camera() as Camera2D
    check(is_equal_approx(camera.zoom.x, 0.5), "g8: Ctrl+wheel down reaches the next rung (zoom %s)" % camera.zoom.x)
    var world_point_after: Vector2 = _world_point_at(world_view, cursor_px)
    check(world_point_before.distance_to(world_point_after) < 0.01,
        "g8: the world point under the cursor did not move (%s -> %s)" % [world_point_before, world_point_after])
    check(camera.position != Vector2(960.0, 528.0),
        "g8: the camera actually moved (a view-centre anchor would leave it at 960,528)")

    _drop(world_view)
    _completed.append("g8")


# --- G14: the ladder, the keys, and what must NOT zoom ----------------------------


func _test_g14() -> void:
    print("--- G14: the zoom ladder and the keys ---")
    var world_view: Control = await _make_world_view([Vector2i(0, 0), Vector2i(1, 0), Vector2i(2, 0),
        Vector2i(0, 1), Vector2i(1, 1), Vector2i(2, 1), Vector2i(0, 2), Vector2i(1, 2), Vector2i(2, 2)])
    var camera: Camera2D = world_view.camera() as Camera2D
    # Parked mid-map, well inside the bound at every rung of the ladder.
    world_view.move_camera_to(3000.0, 1600.0)
    var centre_before: Vector2 = camera.position

    var rungs: Array[float] = []
    for step_index in range(3):
        _send(world_view, _key_event(KEY_EQUAL, true, false))
        await process_frame
        rungs.append(camera.zoom.x)
    check(rungs == [2.0, 2.0, 2.0], "g14: `+` climbs the ladder and stops at 2.0 (got %s)" % [rungs])
    check(camera.position == centre_before, "g14: the view centre stays put while zooming in (%s)" % [camera.position])

    rungs.clear()
    for step_index in range(5):
        _send(world_view, _key_event(KEY_MINUS, true, false))
        await process_frame
        rungs.append(camera.zoom.x)
    check(rungs == [1.0, 0.5, 0.25, 0.25, 0.25], "g14: `-` descends and stops at 0.25 (got %s)" % [rungs])
    check(camera.position == centre_before, "g14: the view centre stays put while zooming out (%s)" % [camera.position])

    # A held key echoes, and an echo must not step the ladder again: at an end of the ladder an
    # echo is indistinguishable from a step, so this is asserted at a rung with a neighbour.
    _send(world_view, _key_event(KEY_EQUAL, true, false))
    await process_frame
    var zoom_before_echo: float = camera.zoom.x
    _send(world_view, _key_event(KEY_EQUAL, true, true))
    await process_frame
    check(camera.zoom.x == zoom_before_echo,
        "g14: a key echo does not step the ladder (%s -> %s)" % [zoom_before_echo, camera.zoom.x])
    _send(world_view, _key_event(KEY_EQUAL, false, false))
    await process_frame
    check(camera.zoom.x == zoom_before_echo, "g14: a key release does not step the ladder")

    # A plain wheel belongs to the Map tab's brush radius and must not zoom.
    var zoom_before_wheel: float = camera.zoom.x
    _send(world_view, _wheel_event(false, true, world_view.get_global_rect().position + Vector2(400.0, 400.0)))
    await process_frame
    check(camera.zoom.x == zoom_before_wheel,
        "g14: a plain wheel leaves the zoom alone (%s -> %s)" % [zoom_before_wheel, camera.zoom.x])

    # A visible text-entry Control owns the keys: a SpinBox steps its own value with the arrows.
    var spin_box: SpinBox = SpinBox.new()
    spin_box.focus_mode = Control.FOCUS_ALL
    world_view.add_child(spin_box)
    spin_box.grab_focus()
    await process_frame
    var centre_before_focus: Vector2 = camera.position
    _send(world_view, _key_event(KEY_LEFT, true, false))
    await process_frame
    check(camera.position == centre_before_focus,
        "g14: arrow keys do not pan while a visible SpinBox has focus")
    # ... but a hidden one must NOT block arrow-pan for the rest of the session, which is the
    # divergence section 9.1 records from the design's literal "no text-entry focus" wording.
    spin_box.hide()
    await process_frame
    _send(world_view, _key_event(KEY_LEFT, true, false))
    await process_frame
    check(camera.position.x < centre_before_focus.x,
        "g14: arrow-pan works again once that SpinBox is hidden (%s -> %s)" % [
            centre_before_focus.x, camera.position.x])

    _drop(world_view)
    _completed.append("g14")


# --- G18: pan, clamp, snapping, latch --------------------------------------------


func _test_g18() -> void:
    print("--- G18: pan, clamp, snapping, drag latch ---")
    var world_view: Control = await _make_world_view([Vector2i(0, 0), Vector2i(1, 0), Vector2i(2, 0),
        Vector2i(0, 1), Vector2i(1, 1), Vector2i(2, 1), Vector2i(0, 2), Vector2i(1, 2), Vector2i(2, 2)])
    var camera: Camera2D = world_view.camera() as Camera2D
    var viewport_size: Vector2 = (world_view.viewport() as SubViewport).size
    # The 3 x 3 fixture: map 5760 x 3168 at (0, 0), grown one screen on every side, so the bound
    # SPANS 9600 x 5280 world px, from (-1920, -1056) to (7680, 4224). The span is the bound's
    # SIZE; its high corner is the size minus the low offset, and the clamp compares the corner.
    var bound_low: Vector2 = -SCREEN_SIZE_PX
    var bound_high: Vector2 = Vector2(7680.0, 4224.0)
    var panel_centre: Vector2 = world_view.get_global_rect().position + viewport_size * 0.5

    # The bound is WIDER than the view at every rung, so `bound +- view/2` is a real range and
    # one range form covers the ladder. The width is the span, not the high corner.
    var bound_span: Vector2 = bound_high - bound_low
    check(bound_span.x > viewport_size.x / 0.25 and bound_span.y > viewport_size.y / 0.25,
        "g18: the fixture bound is wider than the view at every rung")

    # Middle drag at zoom 1: the world follows the pointer, so the camera moves the other way.
    world_view.move_camera_to(4800.0, 2640.0)
    _send(world_view, _middle_event(true, panel_centre))
    _send(world_view, _motion_event(Vector2(100.0, 50.0), panel_centre))
    await process_frame
    check(camera.position == Vector2(4700.0, 2590.0),
        "g18: a drag of (100, 50) at zoom 1 moves the camera by (-100, -50) (got %s)" % [camera.position])

    # The same drag at zoom 0.5 moves twice as far: dividing by the zoom is what keeps the world
    # under the pointer instead of the panel.
    camera.zoom = Vector2(0.5, 0.5)
    world_view.move_camera_to(4800.0, 2640.0)
    _send(world_view, _motion_event(Vector2(100.0, 50.0), panel_centre))
    await process_frame
    check(camera.position == Vector2(4600.0, 2540.0),
        "g18: the same drag at zoom 0.5 moves twice as far (got %s)" % [camera.position])

    # One arrow press moves view_px / (8 * zoom) world px (Math section 6.2).
    camera.zoom = Vector2(1.0, 1.0)
    world_view.move_camera_to(4800.0, 2640.0)
    var pan_step: Vector2 = viewport_size / (8.0 * camera.zoom.x)
    _send(world_view, _key_event(KEY_LEFT, true, false))
    await process_frame
    check(camera.position == Vector2(4800.0 - pan_step.x, 2640.0),
        "g18: one left arrow moves view/(8*zoom) = %s (got %s)" % [pan_step, camera.position])
    # Consumed (issue #120): `ui_left` / `ui_right` are also the TabBar's tab-navigation actions,
    # so an unconsumed press pans AND changes the selected tab. `push_input` resets the handled
    # flag per event, so this reads only the arrow's own result.
    check(world_view.get_viewport().is_input_handled(),
        "g18: a pan arrow is consumed, so the TabBar cannot also act on it (issue #120)")
    _send(world_view, _key_event(KEY_DOWN, true, true))
    await process_frame
    check(camera.position == Vector2(4800.0 - pan_step.x, 2640.0 + pan_step.y),
        "g18: a key echo pans too, by the same step (got %s)" % [camera.position])
    # ... and only the arrows are consumed: a key this panel ignores stays available to the GUI.
    _send(world_view, _key_event(KEY_Q, true, false))
    await process_frame
    check(not world_view.get_viewport().is_input_handled(),
        "g18: a key that is not a pan arrow is not consumed (issue #120)")

    # Clamped at every rung, and the expectation is arithmetic on the live view.
    for zoom_rung in [2.0, 1.0, 0.5, 0.25]:
        camera.zoom = Vector2(zoom_rung, zoom_rung)
        world_view.move_camera_to(4800.0, 2640.0)
        _send(world_view, _middle_event(true, panel_centre))
        _send(world_view, _motion_event(Vector2(-100000.0, -100000.0), panel_centre))
        await process_frame
        var view_at_rung: Vector2 = viewport_size / zoom_rung
        check(camera.position == bound_high - view_at_rung * 0.5,
            "g18: at zoom %s the drag stops at bound +- view/2 (got %s, wanted %s)" % [
                zoom_rung, camera.position, bound_high - view_at_rung * 0.5])
        _send(world_view, _middle_event(false, panel_centre))

    # Math section 2, asserted by CALLING the clamp directly: every synthetic event carries an
    # integer position, so an event-driven assertion would land on the grid with or without the
    # snap and prove nothing.
    camera.zoom = Vector2(1.0, 1.0)
    world_view._apply_cam_center_clamped(Vector2(100.4, 700.6))
    check(camera.position == Vector2(100.0, 701.0), "g18: at zoom 1 the centre snaps to whole world px (got %s)" % [camera.position])
    camera.zoom = Vector2(2.0, 2.0)
    world_view._apply_cam_center_clamped(Vector2(100.4, 700.6))
    check(camera.position == Vector2(100.5, 700.5), "g18: at zoom 2 it snaps to half px (got %s)" % [camera.position])
    camera.zoom = Vector2(0.5, 0.5)
    world_view._apply_cam_center_clamped(Vector2(100.4, 700.6))
    check(camera.position == Vector2(100.4, 700.6), "g18: at zoom 0.5 it is NOT snapped (got %s)" % [camera.position])

    # A middle drag that ends outside the panel still releases the latch: containment gates the
    # press and the motion, never the release.
    camera.zoom = Vector2(1.0, 1.0)
    world_view.move_camera_to(4800.0, 2640.0)
    _send(world_view, _middle_event(true, panel_centre))
    _send(world_view, _middle_event(false, world_view.get_global_rect().position + Vector2(-500.0, -500.0)))
    _send(world_view, _motion_event(Vector2(100.0, 50.0), panel_centre))
    await process_frame
    check(camera.position == Vector2(4800.0, 2640.0), "g18: a release outside the panel clears the drag latch")

    # A tab switch mid-drag clears it too, or the next motion after returning would pan with no
    # button held.
    _send(world_view, _middle_event(true, panel_centre))
    world_view.hide()
    await process_frame
    world_view.show()
    await process_frame
    _send(world_view, _motion_event(Vector2(100.0, 50.0), panel_centre))
    await process_frame
    check(camera.position == Vector2(4800.0, 2640.0), "g18: hiding the panel mid-drag clears the latch")

    # A hidden view takes no input at all, keys included: the Simulator tab is not showing, so an
    # arrow press must not move a camera the user cannot see. This is the `_input` visibility
    # gate, which the per-frame latch clear does not cover.
    world_view.hide()
    await process_frame
    _send(world_view, _key_event(KEY_LEFT, true, false))
    await process_frame
    world_view.show()
    await process_frame
    check(camera.position == Vector2(4800.0, 2640.0), "g18: a hidden view does not pan")

    # A press outside the panel does not start a drag; neither motion nor release moves anything.
    _send(world_view, _middle_event(true, world_view.get_global_rect().position + Vector2(-500.0, -500.0)))
    _send(world_view, _motion_event(Vector2(100.0, 50.0), panel_centre))
    await process_frame
    check(camera.position == Vector2(4800.0, 2640.0), "g18: a press outside the panel does not pan")

    _drop(world_view)

    # Math section 5's small-bound form: a one-screen world with a 5760 x 3168 bound, which the
    # 0.25 view (7680 x 4320) exceeds on both axes. The view is lined up with the bound's low
    # edge, NOT centred on the bound -- Math's own example puts the centre at 1104 where the
    # bound's midpoint is 1056.
    var small_world: Control = await _make_world_view([Vector2i(0, 0)])
    var small_camera: Camera2D = small_world.camera() as Camera2D
    small_camera.zoom = Vector2(0.25, 0.25)
    small_world.move_camera_to(0.0, 0.0)
    var small_panel_centre: Vector2 = small_world.get_global_rect().position + (small_world.viewport() as SubViewport).size * 0.5
    _send(small_world, _middle_event(true, small_panel_centre))
    _send(small_world, _motion_event(Vector2(-100000.0, -100000.0), small_panel_centre))
    await process_frame
    var small_view: Vector2 = (small_world.viewport() as SubViewport).size / 0.25
    check(small_camera.position == Vector2(-SCREEN_SIZE_PX.x + small_view.x * 0.5, -SCREEN_SIZE_PX.y + small_view.y * 0.5),
        "g18: on a one-screen world at 0.25 the bound is narrower than the view and the centre is bound_low + view/2 (got %s)" % [small_camera.position])
    _drop(small_world)

    # No store injected at all: nothing to clamp to, so a pan lands exactly where it was asked.
    var bare_view: Control = (load(WORLD_VIEW_SCENE) as PackedScene).instantiate() as Control
    root.add_child(bare_view)
    bare_view.set_anchors_preset(Control.PRESET_TOP_LEFT)
    bare_view.size = PANEL_SIZE_PX
    await process_frame
    var bare_camera: Camera2D = bare_view.camera() as Camera2D
    bare_view.move_camera_to(0.0, 0.0)
    var bare_panel_centre: Vector2 = bare_view.get_global_rect().position + (bare_view.viewport() as SubViewport).size * 0.5
    _send(bare_view, _middle_event(true, bare_panel_centre))
    _send(bare_view, _motion_event(Vector2(-100000.0, -100000.0), bare_panel_centre))
    await process_frame
    check(bare_camera.position == Vector2(100000.0, 100000.0),
        "g18: with no store injected the pan is unclamped (got %s)" % [bare_camera.position])
    _drop(bare_view)

    # An empty store is NOT the same case: `bounds()` returns Rect2i(0, 0, 1, 1), so the origin
    # screen and its one-screen margin apply and the ordinary clamp runs: the bound spans
    # 5760 x 3168 world px, from (-1920, -1056) to (3840, 2112).
    var empty_view: Control = await _make_world_view([])
    var empty_camera: Camera2D = empty_view.camera() as Camera2D
    empty_camera.zoom = Vector2(1.0, 1.0)
    empty_view.move_camera_to(0.0, 0.0)
    var empty_panel_centre: Vector2 = empty_view.get_global_rect().position + (empty_view.viewport() as SubViewport).size * 0.5
    _send(empty_view, _middle_event(true, empty_panel_centre))
    _send(empty_view, _motion_event(Vector2(-100000.0, -100000.0), empty_panel_centre))
    await process_frame
    var empty_view_px: Vector2 = (empty_view.viewport() as SubViewport).size
    check(empty_camera.position == Vector2(3840.0, 2112.0) - empty_view_px * 0.5,
        "g18: an empty store still clamps, to the origin screen plus the margin (got %s)" % [empty_camera.position])
    _drop(empty_view)

    _completed.append("g18")


# --- G20: entering the tab centres on the active screen --------------------------


func _test_g20() -> void:
    print("--- G20: Start / Jump on tab entry ---")
    var main_root: Node = (load(MAIN_SCENE) as PackedScene).instantiate()
    if main_root == null:
        check(false, "g20: main.tscn did not instantiate")
        _completed.append("g20")
        return
    root.add_child(main_root)
    var tab_container: TabContainer = main_root.get_node("TabContainer") as TabContainer
    var world_view: Control = main_root.get_node("TabContainer/Simulator/WorldView") as Control
    if world_view == null:
        check(false, "g20: main.tscn's Simulator tab has no WorldView instance")
        _completed.append("g20")
        return

    # A known world, replacing whatever the host's `user://data/world.json` holds: (1,1) is id 2
    # and (-1,-1) is id 3, and the negative cell is the case a sign check would reject.
    var store: ScreenStore = ScreenStore.new()
    store.register(0, 0, 1, "a.gd")
    store.register(1, 1, 2, "b.gd")
    store.register(-1, -1, 3, "c.gd")
    world_view.set_grid_config(GRID_CONFIG)
    world_view.set_screen_store(store)
    var camera: Camera2D = world_view.camera() as Camera2D

    # Coming from the Map tab, the Map editor's pick is the active screen.
    main_root.map_editor._screen_id = 2
    tab_container.current_tab = MAP_TAB_INDEX
    await process_frame
    tab_container.current_tab = SIMULATOR_TAB_INDEX
    for settle in range(FRAMES_TO_SETTLE):
        await process_frame
    check(camera.position == Vector2(1.5, 1.5) * SCREEN_SIZE_PX,
        "g20: entering from the Map tab centres on the Map editor's screen (got %s)" % [camera.position])

    # Coming from Placement, Placement's pick is. It is set AFTER the tab entry because
    # `_on_tab_changed` copies the Map id into the Placement editor when that tab is entered.
    tab_container.current_tab = PLACEMENT_TAB_INDEX
    await process_frame
    main_root.placement_editor._screen_id = 1
    tab_container.current_tab = SIMULATOR_TAB_INDEX
    for settle in range(FRAMES_TO_SETTLE):
        await process_frame
    check(camera.position == Vector2(0.5, 0.5) * SCREEN_SIZE_PX,
        "g20: entering from the Placement tab centres on the Placement editor's screen (got %s)" % [camera.position])

    # A screen at a negative cell is legal (Math section 5): centring on it must work, which it
    # cannot if validity is inferred from the sign.
    world_view.center_on_screen(3)
    check(camera.position == Vector2(-0.5, -0.5) * SCREEN_SIZE_PX,
        "g20: a screen registered at (-1, -1) is a valid jump target (got %s)" % [camera.position])

    # An unregistered id leaves the camera alone rather than jumping to the origin.
    var position_before: Vector2 = camera.position
    world_view.center_on_screen(99)
    check(camera.position == position_before,
        "g20: an unregistered id does not move the camera (got %s)" % [camera.position])

    root.remove_child(main_root)
    main_root.queue_free()
    _completed.append("g20")
