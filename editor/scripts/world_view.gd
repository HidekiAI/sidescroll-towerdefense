@tool
class_name WorldView extends Control
# WorldView owns the game viewport: one SubViewport, the Camera2D inside it, and the two
# ParallaxDraw planes that straddle the (empty in #107) World node.
#
# Its job beyond that ownership is twofold: the redraw policy of
# PLAN-2026-10-16-parallax-step4.md section 4.4 (poll the ViewState, and re-query the bridge only
# when it actually changed), and -- since #119 -- the editor camera's input: middle-drag pan,
# arrow-key pan, Ctrl+wheel and `+` / `-` over the zoom ladder, and centring on the active screen
# when the Simulator tab is entered (TDD_Parallax-Scrolling section 4.1). Polling rather than
# signals is deliberate — `Camera2D.position` and `SubViewport.size` are plain properties with no
# change signal, and a one-frame latency in a backdrop is invisible while the polling cost is a
# dictionary compare.
#
# Scene shape: section 4.1 of the same plan. Anchors on `SubViewportContainer` are mandatory
# (Godot4-Learnings section 1): without them the rect is 0x0, the SubViewport is driven to 2x2,
# and nothing draws while nothing errors.
#
# G3 (the SubViewport keeps the editor's UI undisturbed) holds because the Camera2D is inside the
# SubViewport: moving it moves that canvas only.

const FRONT_PLANE_KEY: String = "front"
const DEFAULT_STACK_PATH: String = "res://assets/backdrop_stacks/neutral_default/stack.json"

# The zoom ladder. Optical zoom, so a rung doubles or halves the drawn size; nothing between rungs
# is reachable, which is what lets the ladder be a find() over this list rather than arithmetic.
const ZOOM_LADDER: Array[float] = [2.0, 1.0, 0.5, 0.25]
# One arrow press moves the view by this fraction of what is on screen (Math section 6.2's 8).
const ARROW_PAN_DIVISOR: float = 8.0

var _loader: ParallaxLoader = null
var _back_draw: ParallaxDraw = null
var _front_draw: ParallaxDraw = null
var _camera: Camera2D = null
var _viewport: SubViewport = null
var _status_label: Label = null

var _screen_store: ScreenStore = null
var _grid_config: Dictionary = {}

var _dragging_middle: bool = false

var _stack_path: String = DEFAULT_STACK_PATH
var _anchor_world_px: Vector2 = Vector2.ZERO
var _anchor_is_authored: bool = false

var _last_view: Dictionary = {}
var _last_anchor: Dictionary = {}
var _last_placements: Array = []
var _status_text: String = ""


func _ready() -> void:
    _viewport = get_node_or_null("SubViewportContainer/Viewport") as SubViewport
    _back_draw = get_node_or_null("SubViewportContainer/Viewport/BackPlane/ParallaxDraw") as ParallaxDraw
    _front_draw = get_node_or_null("SubViewportContainer/Viewport/FrontPlane/ParallaxDraw") as ParallaxDraw
    _camera = get_node_or_null("SubViewportContainer/Viewport/Camera2D") as Camera2D
    _status_label = get_node_or_null("StatusLabel") as Label
    _loader = ParallaxLoader.new()
    # Force one query on the first poll, whatever the camera happens to start at.
    _last_view = {}
    _last_anchor = {}


func _process(_delta_seconds: float) -> void:
    if not is_visible_in_tree():
        # A tab switch hides the PAGE, not this node, so this node's own `visible` never changes
        # and `visibility_changed` never fires here: the latch is cleared per frame instead. A
        # drag that spans a tab switch must not resume panning when the tab comes back.
        _dragging_middle = false
    _poll_view_state()


# Test and tooling seam: hand the loader a stand-in bridge, exactly as `ParallaxLoader.set_bridge`
# documents. Returns false when no loader exists yet, i.e. before `_ready()`.
func set_bridge(bridge: Object) -> bool:
    if _loader == null:
        return false
    _loader.call("set_bridge", bridge)
    return true


# Point the view at a different stack. Clears the last ViewState so the next poll reloads.
func set_stack_path(stack_path: String) -> void:
    _stack_path = stack_path
    _last_view = {}
    _last_anchor = {}


# Fix the parallax anchor. Named for what it fixes because `Control.set_anchor(Side, ...)` is a
# native method and overriding it would never be called by the engine. A world with no anchor
# declaration never calls this: the anchor then
# stays the view centre, which is `sstd-core::parallax::default_anchor`'s value for a one-screen
# map at the ref view size. WorldView supplies it rather than the loader, because the loader has
# no map rectangle to derive it from.
func set_parallax_anchor(world_px_x: float, world_px_y: float) -> void:
    _anchor_world_px = Vector2(world_px_x, world_px_y)
    _anchor_is_authored = true
    _last_anchor = {}


# Move the camera. One frame, no tween: a teleport is a cut (G6's mutation is "add a tween").
func move_camera_to(world_px_x: float, world_px_y: float) -> void:
    if _camera != null:
        _camera.position = Vector2(world_px_x, world_px_y)


func camera() -> Camera2D:
    return _camera


func viewport() -> SubViewport:
    return _viewport


# The occupied screen cells and the grid config, injected by `main.gd`. Both are needed only by the
# editor camera: the bound it clamps to is derived from them on demand and never cached, so adding,
# moving or cloning a screen needs no invalidation call.
func set_screen_store(store: ScreenStore) -> void:
    _screen_store = store


func set_grid_config(config: Dictionary) -> void:
    _grid_config = config


# Jump: put the camera on the centre of one screen. Registered id first -- `position_of_id()`
# answers (-1, -1) for an unknown id, and `screen_store.gd:46-48` is explicit that a screen may BE
# at a negative position, so the sign of a cell must never be read as validity.
func center_on_screen(screen_id: int) -> void:
    if _screen_store == null or not _screen_store.has_id(screen_id):
        return
    var screen_size_px: Vector2 = _screen_size_world_px()
    if screen_size_px.x <= 0.0 or screen_size_px.y <= 0.0:
        return
    var cell: Vector2i = _screen_store.position_of_id(screen_id)
    _apply_cam_center_clamped((Vector2(cell) + Vector2(0.5, 0.5)) * screen_size_px)


# Pan, zoom and jump, per TDD_Parallax-Scrolling section 4.1. `_input` rather than
# `_unhandled_input`: the SubViewportContainer has `stretch = true` and a STOPping `mouse_filter`,
# so it consumes GUI mouse events and they never reach unhandled input.
func _input(event: InputEvent) -> void:
    # @tool: in the Godot editor the same scene is live, and a middle drag over the Simulator tab
    # would write Camera2D.position into the edited scene (marking it dirty) while Godot's own 2D
    # viewport uses that same button to pan. The app run is the only place camera input belongs.
    if Engine.is_editor_hint():
        return
    if _camera == null or _viewport == null:
        return                       # input can arrive before _ready's node lookups resolve
    if not is_visible_in_tree():
        _dragging_middle = false     # tab switched mid-drag: a later motion must not pan
        return

    if event is InputEventMouseButton:
        _handle_mouse_button(event as InputEventMouseButton)
        return
    if event is InputEventMouseMotion:
        _handle_mouse_motion(event as InputEventMouseMotion)
        return
    if event is InputEventKey:
        _handle_key(event as InputEventKey)


func _handle_mouse_button(mouse_button: InputEventMouseButton) -> void:
    if mouse_button.button_index == MOUSE_BUTTON_MIDDLE:
        if not mouse_button.pressed:
            # Clear from ANYWHERE: containment gates the press and the motion, never the release,
            # or a drag that ends outside the panel leaves the latch set and the next motion pans
            # with no button held.
            _dragging_middle = false
        elif get_global_rect().has_point(mouse_button.position):
            _dragging_middle = true
        return
    if not mouse_button.pressed:
        return
    # Wheel events carry no release, so `pressed` is always true for them; they are checked for
    # Ctrl rather than for a press/release pair.
    if not mouse_button.is_command_or_control_pressed():
        return                       # a plain wheel belongs to the Map tab's brush radius
    if not get_global_rect().has_point(mouse_button.position):
        return
    var cursor_from_view_topleft_px: Vector2 = mouse_button.position - get_global_rect().position
    if mouse_button.button_index == MOUSE_BUTTON_WHEEL_UP:
        _step_zoom_rung(-1, cursor_from_view_topleft_px)      # -1: one rung toward 2.0
    elif mouse_button.button_index == MOUSE_BUTTON_WHEEL_DOWN:
        _step_zoom_rung(1, cursor_from_view_topleft_px)


func _handle_mouse_motion(motion: InputEventMouseMotion) -> void:
    if not _dragging_middle:
        return
    if motion.button_mask & MOUSE_BUTTON_MASK_MIDDLE == 0:
        # The release went somewhere we never saw (another window, a consumed event). Stop panning
        # rather than pan on the next bare motion.
        _dragging_middle = false
        return
    # Grab the world: drag right, the world moves right, so the camera moves left. Dividing by the
    # zoom is what makes the world follow the pointer instead of the panel.
    _pan_by_world_px(-motion.relative / _camera.zoom.x)


func _handle_key(key_event: InputEventKey) -> void:
    if not key_event.pressed:
        return
    if _text_entry_has_focus():
        return                       # a visible SpinBox is typing, and arrows are its value step
    if key_event.keycode == KEY_EQUAL or key_event.keycode == KEY_PLUS or key_event.keycode == KEY_KP_ADD:
        if not key_event.is_echo():
            _step_zoom_rung(-1, _viewport.size * 0.5)         # view-centre anchor
        return
    if key_event.keycode == KEY_MINUS or key_event.keycode == KEY_KP_SUBTRACT:
        if not key_event.is_echo():
            _step_zoom_rung(1, _viewport.size * 0.5)
        return
    # Arrows DO accept echo: holding one is a steady pan (Math section 6.2), unlike the ladder.
    #
    # Consumed (issue #120): `ui_left` / `ui_right` are also the focused TabBar's tab-navigation
    # actions, so an unconsumed press pans the camera AND moves the tab selection -- and because
    # the Simulator is the right-most tab, only left-arrow visibly jumps away. Consumed whether or
    # not the camera moved: an arrow at the editor bound is still an arrow this panel handled.
    var pan_direction: Vector2 = (
        Vector2.LEFT if key_event.is_action_pressed("ui_left", true)
        else Vector2.RIGHT if key_event.is_action_pressed("ui_right", true)
        else Vector2.UP if key_event.is_action_pressed("ui_up", true)
        else Vector2.DOWN if key_event.is_action_pressed("ui_down", true)
        else Vector2.ZERO)
    if pan_direction == Vector2.ZERO:
        return
    _pan_by_world_px(pan_direction * (_viewport.size / (ARROW_PAN_DIVISOR * _camera.zoom.x)))
    get_viewport().set_input_as_handled()


# A visible text-entry Control owns the focus and the keys. The visibility test is not cosmetic: a
# SpinBox on a hidden tab can still be the focus owner, and counting that as typing would block
# arrow-pan for the rest of the session after any other tab's field was focused.
func _text_entry_has_focus() -> bool:
    var focus_owner: Control = get_viewport().gui_get_focus_owner()
    if focus_owner == null or not focus_owner.is_visible_in_tree():
        return false
    return focus_owner is LineEdit or focus_owner is SpinBox or focus_owner is TextEdit


func _pan_by_world_px(world_px_delta: Vector2) -> void:
    _apply_cam_center_clamped(_camera.position + world_px_delta)


# One rung, never an intermediate value. `index_delta` is added to the ladder index, so -1 is
# toward 2.0 and +1 toward 0.25.
func _step_zoom_rung(index_delta: int, anchor_from_view_topleft_px: Vector2) -> void:
    var zoom_old: float = _camera.zoom.x
    var ladder_index: int = ZOOM_LADDER.find(zoom_old)
    if ladder_index < 0:
        return                       # not on a rung; the ladder is the only writer, so this is
                                     # unreachable, and G14's "only ever a rung" assertion is the
                                     # tripwire that keeps it so
    var ladder_index_new: int = clampi(ladder_index + index_delta, 0, ZOOM_LADDER.size() - 1)
    if ladder_index_new == ladder_index:
        return                       # at an end of the ladder
    var zoom_new: float = ZOOM_LADDER[ladder_index_new]
    var anchor_offset_px: Vector2 = anchor_from_view_topleft_px - _viewport.size * 0.5
    # Math section 6.1: cam' = cam + (cursor - view/2) * (1/zoom_old - 1/zoom_new). The zoom is
    # written BEFORE the clamp, because the clamp reads it for the snap step and the view extent.
    _camera.zoom = Vector2(zoom_new, zoom_new)
    _apply_cam_center_clamped(
        _camera.position + anchor_offset_px * (1.0 / zoom_old - 1.0 / zoom_new))


func _apply_cam_center_clamped(world_px_center: Vector2) -> void:
    if _camera == null or _viewport == null:
        return
    var bound: Rect2 = _editor_bound_world_px()
    var zoom: float = _camera.zoom.x
    var view_world_px: Vector2 = _viewport.size / zoom     # Camera2D zoom = magnification
    var target: Vector2 = world_px_center
    if bound.size.x > 0.0 and bound.size.y > 0.0:
        target = Vector2(
            CameraController.clamp_center(target.x, view_world_px.x, bound.position.x, bound.end.x),
            CameraController.clamp_center(target.y, view_world_px.y, bound.position.y, bound.end.y))
    # Math section 2: a multiple of 1/zoom world px while zoom is >= 1 (whole px at 1, half px at
    # 2), and NOT snapped below 1, where the screen-pixel grid is already coarser than a world px.
    if zoom >= 1.0:
        var snap_step: float = 1.0 / zoom
        target = (target / snap_step).round() * snap_step
    move_camera_to(target.x, target.y)


# The screen size in world px, from the grid config. One copy: the bound and the centre-on-screen
# move both need it, and the config is the source of truth (Rule of Thumb: Config-Based Constants)
# so no literal fallback is written here. The default is 0 rather than the 60 / 32 several scripts
# spell out, because 0 is detectable and makes the caller skip the clamp; a literal fallback would
# clamp to a bound that is quietly wrong.
func _screen_size_world_px() -> Vector2:
    return Vector2(
        float(_grid_config.get("max_tiles_per_screen_x", 0)) * float(_grid_config.get("tile_width_in_pixels", 0)),
        float(_grid_config.get("max_tiles_per_screen_y", 0)) * float(_grid_config.get("tile_height_in_pixels", 0)))


# The map rectangle grown by one screen on every side (Math section 5, `camera_bound(editor =
# true)`). Derived per call: ScreenStore.bounds() is a pass over the screens map, which the editor
# holds at single-digit size, and deriving it removes every invalidation path.
func _editor_bound_world_px() -> Rect2:
    if _screen_store == null:
        return Rect2()                      # bare scene, store not injected yet: nothing to clamp
    var screen_size_px: Vector2 = _screen_size_world_px()
    if screen_size_px.x <= 0.0 or screen_size_px.y <= 0.0:
        return Rect2()                      # grid config not loaded yet: same case
    var cells: Rect2i = _screen_store.bounds()
    var map_world_px: Rect2 = Rect2(
        Vector2(cells.position) * screen_size_px,
        Vector2(cells.size) * screen_size_px)
    return map_world_px.grow_individual(
        screen_size_px.x, screen_size_px.y, screen_size_px.x, screen_size_px.y)


# The bridge's own `placements` array from the last successful query, unmodified and unsplit.
# G1 compares this, and the two planes' slices of it, against the bridge.
func last_placements() -> Array:
    return _last_placements


func status_text() -> String:
    return _status_text


func _poll_view_state() -> void:
    if _camera == null or _viewport == null or _loader == null:
        return
    var view: Dictionary = {
        "cam_center_world_px_x": _camera.position.x,
        "cam_center_world_px_y": _camera.position.y,
        "zoom": _camera.zoom.x,
        "view_w_px": _viewport.size.x,
        "view_h_px": _viewport.size.y,
    }
    var anchor: Dictionary = _resolve_anchor(view)
    if view == _last_view and anchor == _last_anchor:
        return
    _last_view = view
    _last_anchor = anchor
    _refresh_placements()


# The anchor is compared as part of the changed-set because `place_layer` reads it: an anchor
# move with a still camera is still a different placement set. Only the fields `place_layer`
# reads may trigger a query.
func _resolve_anchor(view: Dictionary) -> Dictionary:
    if _anchor_is_authored:
        return {"world_px_x": _anchor_world_px.x, "world_px_y": _anchor_world_px.y}
    return {
        "world_px_x": float(view.get("view_w_px", 0.0)) * 0.5,
        "world_px_y": float(view.get("view_h_px", 0.0)) * 0.5,
    }


func _refresh_placements() -> void:
    var view_w_px: float = float(_last_view.get("view_w_px", 0.0))
    var view_h_px: float = float(_last_view.get("view_h_px", 0.0))
    var result: Dictionary = _loader.load_stack(_stack_path, _last_view, _last_anchor)
    _status_text = String(result.get("status_text", ""))
    # Section 4.7's HUD. The text is written for BOTH outcomes on purpose: a load that fell back
    # and then succeeded still has to SHOW the fallback, because the rule that rejected the first
    # stack did not stop being real.
    if _status_label != null:
        _status_label.text = _status_text
    if not result.get("ok", false):
        # Both stacks failed. Clear both planes rather than keep drawing the old stack's art at
        # the new ViewState, which would be a backdrop that silently ignores the camera.
        _last_placements = []
        if _back_draw != null:
            _back_draw.apply([], view_w_px, view_h_px, "")
        if _front_draw != null:
            _front_draw.apply([], view_w_px, view_h_px, "")
        return

    _last_placements = result.get("placements", []) as Array
    var stack_dir: String = String(result.get("stack_path", "")).get_base_dir()
    var back_plane: Array = []
    var front_plane: Array = []
    for entry_index in range(_last_placements.size()):
        var entry := _last_placements[entry_index] as Dictionary
        if String(entry.get("plane", "")) == FRONT_PLANE_KEY:
            front_plane.append(entry)
        else:
            back_plane.append(entry)
    if _back_draw != null:
        _back_draw.call("apply", back_plane, view_w_px, view_h_px, stack_dir)
    if _front_draw != null:
        _front_draw.call("apply", front_plane, view_w_px, view_h_px, stack_dir)
