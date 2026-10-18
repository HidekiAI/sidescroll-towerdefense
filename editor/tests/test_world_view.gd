extends SceneTree
# Issue #107 (parallax step 4) -- WorldView's scene shape, its anchors, and G3.
#
# G3 is the claim that cost #89 its camera: a Camera2D anywhere in the tree rewrites the canvas
# transform of WHATEVER CANVAS IT LIVES IN, so a camera in the root canvas moves the editor UI
# with the world. The fix is structural, not behavioural -- the camera lives inside the
# SubViewport -- which is why this suite asserts the STRUCTURE (the camera's parent is a
# SubViewport) together with the outcome (no editor Control's rect changes when the camera moves
# and zooms). Asserting only the outcome would pass for a camera that never moved at all.
#
# Headless has no renderer (Math section 7). Nothing here proves anything is drawn.
#
# Guards run in the #107 style: each appends its name to `_completed`, the runner fails on a
# missing name, and the exit code gates -- never the printed counter.

const MAIN_SCENE: String = "res://scenes/main.tscn"
const WORLD_VIEW_SCENE: String = "res://scenes/world_view.tscn"
const SIMULATOR_TAB_INDEX: int = 4

# The camera move and zoom are deliberately large: a transform that leaks into the root canvas
# moves the editor by hundreds of pixels, so a small step would hide a small leak.
const CAMERA_MOVE_TO: Vector2 = Vector2(4000.0, -1200.0)
const CAMERA_ZOOM: Vector2 = Vector2(0.25, 0.25)
const FRAMES_TO_SETTLE: int = 3

const GUARDS: Array[String] = ["shape", "anchors", "g3"]

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
    _test_scene_shape()
    await _test_g3()
    _assert_all_guards_completed()
    print("=== test_world_view: %d failures ===" % _failures)
    quit(0 if _failures == 0 else 1)


func _assert_all_guards_completed() -> void:
    for guard_name in GUARDS:
        if not _completed.has(guard_name):
            _failures += 1
            printerr("FAIL: guard '%s' did not complete" % guard_name)


# --- the scene shape and the anchors (section 4.1) --------------------------------


func _test_scene_shape() -> void:
    print("--- WorldView scene shape (issue #107) ---")
    var world_view: Control = (load(WORLD_VIEW_SCENE) as PackedScene).instantiate() as Control
    if world_view == null:
        check(false, "world_view.tscn did not instantiate as a Control")
        _completed.append_array(["shape", "anchors"])
        return

    # The root is a Control on purpose (section 4.1's recorded deviation): a Node2D in the parent
    # chain makes a child Control resolve its anchors against the ROOT VIEWPORT, so a Node2D root
    # would fill the whole editor window instead of the tab.
    check(world_view is Control, "shape: the root is a Control, so the Control chain stays intact")

    var container: SubViewportContainer = world_view.get_node_or_null("SubViewportContainer") as SubViewportContainer
    var viewport: SubViewport = world_view.get_node_or_null("SubViewportContainer/Viewport") as SubViewport
    var back_plane: CanvasLayer = world_view.get_node_or_null("SubViewportContainer/Viewport/BackPlane") as CanvasLayer
    var front_plane: CanvasLayer = world_view.get_node_or_null("SubViewportContainer/Viewport/FrontPlane") as CanvasLayer
    var world_node: Node2D = world_view.get_node_or_null("SubViewportContainer/Viewport/World") as Node2D
    var camera: Camera2D = world_view.get_node_or_null("SubViewportContainer/Viewport/Camera2D") as Camera2D
    var overlay: CanvasLayer = world_view.get_node_or_null("Overlay") as CanvasLayer
    var status_label: Label = world_view.get_node_or_null("StatusLabel") as Label

    check(container != null and viewport != null, "shape: SubViewportContainer > Viewport")
    check(back_plane != null and front_plane != null,
        "shape: the two planes exist as CanvasLayers")
    check(world_node != null, "shape: the (empty in #107) World node sits between the planes")
    check(camera != null, "shape: the camera exists")
    check(overlay != null, "shape: the #108 Overlay exists")
    check(status_label != null, "shape: the section 4.7 status HUD exists")

    # The layer indices are what make the split mean anything: back under the world, front over it.
    if back_plane != null and front_plane != null:
        check(back_plane.layer < 0 and front_plane.layer > 0,
            "shape: back plane draws under the World node and front plane over it (layers %d / %d)" % [
                back_plane.layer, front_plane.layer])
    if back_plane != null and world_node != null and front_plane != null:
        check(back_plane.get_index() < world_node.get_index()
            and world_node.get_index() < front_plane.get_index(),
            "shape: tree order matches the layer order")

    # A drawer on each plane, and nothing else that draws: the Sprite2D alternative (section 2.2)
    # is rejected, and this is the assertion that keeps it rejected.
    if back_plane != null and front_plane != null:
        var sprite_count: int = 0
        for plane in [back_plane, front_plane]:
            for child_index in range(plane.get_child_count()):
                var child: Node = plane.get_child(child_index)
                if child is Sprite2D:
                    sprite_count += 1
        check(sprite_count == 0,
            "shape: the planes hold no Sprite2D children (the rejected alternative stays rejected)")
        check(_count_drawers(back_plane) == 1 and _count_drawers(front_plane) == 1,
            "shape: exactly one ParallaxDraw per plane")

    # The camera must be INSIDE the SubViewport. This is G3's mechanism, asserted directly: a
    # camera in the root canvas is what moved the editor UI in #89.
    if camera != null and viewport != null:
        check(camera.get_parent() == viewport,
            "shape: the camera's parent is the SubViewport, so it cannot reach the root canvas")

    _completed.append("shape")

    # --- anchors ---
    #
    # Measured as mandatory in Godot4-Learnings section 1: without them the container's rect is
    # 0x0, the SubViewport follows it to 2x2, and nothing draws while nothing errors.
    if container != null:
        check(is_equal_approx(container.anchor_right, 1.0)
            and is_equal_approx(container.anchor_bottom, 1.0),
            "anchors: SubViewportContainer is anchored to the full rect (right %.2f bottom %.2f)" % [
                container.anchor_right, container.anchor_bottom])
        check(container.stretch,
            "anchors: stretch is on, so the container owns the viewport size")
    if viewport != null:
        check(viewport.size.x > 2 and viewport.size.y > 2,
            "anchors: the viewport is not the 2x2 a 0x0 container would drive (%s)" % viewport.size)
    if status_label != null:
        check(status_label.mouse_filter == Control.MOUSE_FILTER_IGNORE,
            "anchors: the status HUD cannot swallow the clicks meant for the view")
    _completed.append("anchors")


func _count_drawers(plane: Node) -> int:
    var count: int = 0
    for child_index in range(plane.get_child_count()):
        if plane.get_child(child_index) is ParallaxDraw:
            count += 1
    return count


# --- G3: the editor UI never moves -------------------------------------------------


func _test_g3() -> void:
    print("--- G3: the camera cannot reach the editor canvas ---")
    var main_root: Node = (load(MAIN_SCENE) as PackedScene).instantiate()
    if main_root == null:
        check(false, "main.tscn did not instantiate")
        _completed.append("g3")
        return
    root.add_child(main_root)
    var tab_container: TabContainer = main_root.get_node("TabContainer") as TabContainer
    # A TabContainer lays out only the selected child, so the tab has to be selected before any
    # rect is measured. An unselected tab reports 0x0 -- a false measurement, not a defect.
    tab_container.current_tab = SIMULATOR_TAB_INDEX
    for frame_index in range(FRAMES_TO_SETTLE):
        await process_frame

    var world_view: Control = main_root.get_node("TabContainer/Simulator/WorldView") as Control
    if world_view == null:
        check(false, "main.tscn's Simulator tab has no WorldView instance (section 4.9)")
        _completed.append("g3")
        return
    var camera: Camera2D = world_view.camera() as Camera2D
    if camera == null:
        check(false, "WorldView has no camera to move")
        _completed.append("g3")
        return

    # The tab really is filled, which is the same claim the probe measured: an instance whose
    # anchors resolved against the root viewport instead of the tab would fill the window.
    var tab_rect: Rect2 = (tab_container.get_child(SIMULATOR_TAB_INDEX) as Control).get_global_rect()
    check(world_view.get_global_rect().size == tab_rect.size and tab_rect.size.x > 1.0,
        "g3: the WorldView instance fills the Simulator tab (%s)" % world_view.get_global_rect().size)

    # Every editor Control in sight, from four different tabs so a leak in one is not mistaken
    # for a leak in all of them.
    var watched_paths: Array[String] = [
        "TabContainer",
        "TabContainer/TerrainEditor",
        "TabContainer/MapEditor",
        "TabContainer/PlacementEditor",
        "TabContainer/Simulator",
        "TabContainer/Simulator/WorldView",
    ]
    var before: Dictionary = {}
    for path_entry in watched_paths:
        var watched: Control = main_root.get_node_or_null(path_entry) as Control
        if watched == null:
            check(false, "g3: no Control at " + path_entry)
            continue
        before[path_entry] = watched.get_global_rect()

    var camera_before: Vector2 = camera.position
    world_view.move_camera_to(CAMERA_MOVE_TO.x, CAMERA_MOVE_TO.y)
    camera.zoom = CAMERA_ZOOM
    for frame_index in range(FRAMES_TO_SETTLE):
        await process_frame

    check(camera.position == CAMERA_MOVE_TO,
        "g3: the camera actually moved (it was at %s)" % camera_before)
    for path_entry in watched_paths:
        if not before.has(path_entry):
            continue
        var watched: Control = main_root.get_node_or_null(path_entry) as Control
        var after_rect: Rect2 = watched.get_global_rect()
        var before_rect: Rect2 = before[path_entry]
        check(after_rect == before_rect,
            "g3: %s kept its rect across the camera move and zoom (was %s now %s)" % [
                path_entry, before_rect, after_rect])

    _completed.append("g3")
