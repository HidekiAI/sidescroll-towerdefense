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

# Issue #122 -- the prototype walker. These two numbers are `sstd-core::physics`'s
# `DEFAULT_AVATAR_SPEED_PPS` and `GRAVITY_PPS2`, so a later port to `physics.rs` moves the code
# without moving the feel. 200 px/s crosses one 1920 px screen in 9.6 s, which is slow enough to
# see every ground column go by.
const WALKER_SPEED_PPS: float = 200.0
const WALKER_GRAVITY_PPS2: float = 980.0
# Where the camera looks when the avatar first lands: the prototype world's ground line (tile row
# 31 of 32 px tiles = 992 px) minus half a 1080 px view plus a 200 px margin, so the ground sits
# about a fifth of a screen above the bottom edge and there is room to see the avatar fall.
const GROUND_VIEW_CENTER_Y_PX: float = 652.0

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
var _ground_layer: Node2D = null
var _avatar: Node2D = null
var _walker_state: Dictionary = {"pos_x":0,"pos_y":0,"vel_x":0,"vel_y":0,"grounded":false}
var _tiles_cache: Array = []
# Column index -> the y (px) of that column's surface. Built once per `_rebuild_ground_tiles`
# instead of rescanning `_tiles_cache` per frame: the prototype world is 180 columns wide.
var _ground_y_by_col: Dictionary = {}
# #122 -- the prototype follows the avatar. OFF by default: #119/#120's camera belongs to the
# user (pan, zoom, screen jump) and `test_world_view` / `test_world_view_controls` pin exactly
# that, so a follow that always won would silently delete twenty assertions. `main.gd` turns it
# on for the Simulator, which is the one place the walker runs.
var _camera_follow_enabled: bool = false
var _walk_input: Dictionary = {"move_x":0.0,"move_y":0.0,"move_to_world":null,"scroll_vec":null,"place":false,"alt_place":false,"interact":false,"zoom_in":false,"zoom_out":false}
var _last_log_time: int = 0
var _log_interval: int = 1000  # milliseconds


func _ready() -> void:
    _viewport = get_node_or_null("SubViewportContainer/Viewport") as SubViewport
    _back_draw = get_node_or_null("SubViewportContainer/Viewport/BackPlane/ParallaxDraw") as ParallaxDraw
    _front_draw = get_node_or_null("SubViewportContainer/Viewport/FrontPlane/ParallaxDraw") as ParallaxDraw
    _camera = get_node_or_null("SubViewportContainer/Viewport/Camera2D") as Camera2D
    _status_label = get_node_or_null("StatusLabel") as Label
    _loader = ParallaxLoader.new()
    # try to grab bridge from root
    var root = get_tree().root
    if root != null:
        var main = root.get_node_or_null("Main")
        if main != null:
            if main.get("_bridge") != null:
                _bridge_obj = main.get("_bridge")
            elif main.has_method("get_bridge"):
                var b = main.call("get_bridge")
                if b != null:
                    _bridge_obj = b
        if _bridge_obj == null:
            # search recursively
            _bridge_obj = try_find_bridge(root)
    if _loader != null and _bridge_obj != null:
        _loader.call("set_bridge", _bridge_obj)
    var world_node := get_node_or_null("SubViewportContainer/Viewport/World") as Node2D
    if world_node != null:
        _ground_layer = Node2D.new()
        _ground_layer.name = "GroundLayer"
        _ground_layer.set_script(load("res://scripts/world_view_ground.gd"))
        world_node.add_child(_ground_layer)
        _avatar = Node2D.new()
        _avatar.name = "Avatar"
        _avatar.position = Vector2(100, -200)
        _avatar.set_script(load("res://scripts/avatar_stickman.gd"))
        _avatar.z_index = 10
        world_node.add_child(_avatar)
        # Placeholder: `_rebuild_ground_tiles` puts the walker on the first ground column as soon
        # as the ScreenStore arrives (#122). Without ground there is nothing to stand on, so this
        # start point is the one that falls.
        _walker_state = {"pos_x":100.0,"pos_y":-200.0,"vel_x":0.0,"vel_y":0.0,"grounded":false,
            "hp":100.0,"fall_start_y":-200.0}
    # Force one query on the first poll, whatever the camera happens to start at.
    _last_view = {}
    _last_anchor = {}


func _process(delta_seconds: float) -> void:
    if not is_visible_in_tree():
        # A tab switch hides the PAGE, not this node, so this node's own `visible` never changes
        # and `visibility_changed` never fires here: the latch is cleared per frame instead. A
        # drag that spans a tab switch must not resume panning when the tab comes back.
        _dragging_middle = false
    _poll_view_state()
    _update_walker(delta_seconds)


# Test and tooling seam: hand the loader a stand-in bridge, exactly as `ParallaxLoader.set_bridge`
# documents. Returns false when no loader exists yet, i.e. before `_ready()`.
func set_bridge(bridge: Object) -> bool:
    if _loader == null:
        return false
    _loader.call("set_bridge", bridge)
    return true


var _bridge_obj: Object = null

func set_bridge_obj(bridge: Object) -> void:
    _bridge_obj = bridge


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
    _rebuild_ground_tiles()


# Public seam for main.gd: the store is shared and mutated in place (a world load replaces its
# screens, a paint replaces its tiles), so nothing has to hand it over again -- the ground table
# just has to be derived from it again.
func refresh_ground() -> void:
    _rebuild_ground_tiles()


func set_grid_config(config: Dictionary) -> void:
    _grid_config = config


# #122 -- opt in to the follow camera. See `_camera_follow_enabled` for why it is not the default.
func set_camera_follow(enabled: bool) -> void:
    _camera_follow_enabled = enabled
    # The placeholder foreground is opaque over the walking surface. Its CanvasLayer draws
    # above every world z-index, so omit it in the walker prototype, not in parallax previews.
    var front_draw: Node2D = get_node("SubViewportContainer/Viewport/FrontPlane/ParallaxDraw") as Node2D
    front_draw.visible = not enabled


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
        var mb = event as InputEventMouseButton
        if mb.pressed and mb.button_index == MOUSE_BUTTON_LEFT:
            _handle_place_interact(true, false)
        if mb.pressed and mb.button_index == MOUSE_BUTTON_RIGHT:
            _handle_place_interact(false, true)
        return
    if event is InputEventMouseMotion:
        _handle_mouse_motion(event as InputEventMouseMotion)
        return
    if event is InputEventKey:
        _handle_key(event as InputEventKey)
        _handle_movement_keys(event as InputEventKey)
        return



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


# The map rectangle in world px: the occupied screen cells times one screen. Empty when the store
# is not injected yet or the grid config is not loaded, which is what tells callers "nothing to
# clamp to" instead of handing them a rectangle at the origin.
func _map_world_px() -> Rect2:
    if _screen_store == null:
        return Rect2()
    var screen_size_px: Vector2 = _screen_size_world_px()
    if screen_size_px.x <= 0.0 or screen_size_px.y <= 0.0:
        return Rect2()
    var cells: Rect2i = _screen_store.bounds()
    return Rect2(
        Vector2(cells.position) * screen_size_px,
        Vector2(cells.size) * screen_size_px)


# The map rectangle grown by one screen on every side (Math section 5, `camera_bound(editor =
# true)`). Derived per call: ScreenStore.bounds() is a pass over the screens map, which the editor
# holds at single-digit size, and deriving it removes every invalidation path.
func _editor_bound_world_px() -> Rect2:
    var map_world_px: Rect2 = _map_world_px()
    if map_world_px.size.x <= 0.0 or map_world_px.size.y <= 0.0:
        return Rect2()
    var screen_size_px: Vector2 = _screen_size_world_px()
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
    if _bridge_obj != null and _loader != null and _loader.has_method("set_bridge"):
        _loader.call("set_bridge", _bridge_obj)
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
    if _loader == null:
        return
    if _bridge_obj != null and _loader.has_method("set_bridge"):
        _loader.call("set_bridge", _bridge_obj)
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


func _handle_movement_keys(key_event: InputEventKey) -> void:
    if not key_event.pressed:
        return
    if Engine.is_editor_hint():
        return
    if _text_entry_has_focus():
        return
    if key_event.keycode == KEY_SPACE:
        return


# Issue #122 -- "what is the surface of column N", derived from the same tile list the ground
# layer draws. Pure and static so the walker test can call it with a hand-built tile array.
#
# The prototype world has 3 screens x 60 columns of 32 px tiles with ground at rows 31-32, so the
# table is 180 entries all holding 992.0 -- the flat, seam-free ground across all three screens.
#
# `tile_h_px` rather than `tile_w_px` because a surface is a ROW: a non-square tile grid would
# make the two differ, and the caller passes both.
static func build_ground_column_table(tiles: Array, tile_h_px: float) -> Dictionary:
    # Intent: one pass over the tiles -> "what is the surface of column N". The surface is the
    # SMALLEST `world_tile_y` among that column's walkable tiles, converted to px (row *
    # tile_h_px). A column whose tiles are all non-walkable has NO surface at all, so it is
    # simply ABSENT from the table -- and `step_walker` reads absence as a hole.
    #
    # Why a column table and not the tile list: `step_walker` runs 60 times a second and needs one
    # answer per frame. Scanning 360 tiles a frame is exactly the cost the Rust bridge was paying
    # as 29 KB of JSON per frame; the table pays it once, per rebuild.
     var surface_row_by_col: Dictionary = {}
     for tile in tiles:
         if not bool(tile.get("is_walkable", false)):
             continue
         var world_col: int = int(tile.get("world_tile_x", 0))
         var world_row: int = int(tile.get("world_tile_y", 0))
         surface_row_by_col[world_col] = mini(
             int(surface_row_by_col.get(world_col, world_row)), world_row)
     var ground_y_by_col: Dictionary = {}
     for world_col in surface_row_by_col:
         ground_y_by_col[world_col] = float(surface_row_by_col[world_col]) * tile_h_px
     return ground_y_by_col


# Issue #122 -- one frame of walker physics. Pure and static for the same reason: the test drives
# it 600 steps with no scene, no input device and no Godot frame loop.
#
# Not modelled on purpose, because the prototype world is flat and the ticket is not: stepping UP
# onto a higher surface (the walker walks into it), slopes, walls, jumping, and anything living in
# the column above the feet. Only the seam behaviour is real, and it is real because the table has
# no gap at a screen boundary: column 59 and column 60 share one ground line.
static func step_walker(state: Dictionary, move_x: float, dt: float, ground_y_by_col: Dictionary,
        tile_w_px: float, world_left_px: float, world_right_px: float) -> Dictionary:
    # Intent: advance `state` by one frame. Gravity always pulls; the horizontal move is applied
    # and clamped to the world edges; then the feet snap onto the surface of the column they are
    # now in. Handles both falling DOWN onto ground (from above) and stepping UP onto higher
    # ground (walking up a hill). A column with no walkable tile means a hole: the walker falls.
    #
    # Contract: returns a NEW dictionary and never mutates `state`, keys stay pos_x, pos_y,
    # vel_x, vel_y, grounded, hp, fall_start_y.
    var next_state: Dictionary = state.duplicate()
    var feet_was_y: float = float(next_state.get("pos_y", 0.0))
    var next_vel_y: float = float(next_state.get("vel_y", 0.0)) + WALKER_GRAVITY_PPS2 * dt
    var feet_now_y: float = feet_was_y + next_vel_y * dt
    var next_pos_x: float = clampf(
        float(next_state.get("pos_x", 0.0)) + move_x * WALKER_SPEED_PPS * dt,
        world_left_px, world_right_px)
    var last_world_col: int = int(ceil(world_right_px / tile_w_px)) - 1
    var world_col: int = mini(int(floor(next_pos_x / tile_w_px)), last_world_col)
    var hp: float = float(next_state.get("hp", 100.0))
    var fall_start_y: float = float(next_state.get("fall_start_y", feet_was_y))
    var landed: bool = false
    var ground_y: float = feet_now_y

    if ground_y_by_col.has(world_col):
        ground_y = float(ground_y_by_col[world_col])
        # Landing from above onto ground (moving down onto surface)
        if next_vel_y >= 0.0 and feet_was_y <= ground_y and feet_now_y >= ground_y:
            landed = true
        # Stepping UP onto higher ground (walking up a hill) -- only when moving horizontally
        elif ground_y < feet_was_y and move_x != 0.0:
            landed = true
            feet_now_y = ground_y
            next_vel_y = 0.0
            # reset fall start: we were standing, now stepped up onto higher ground
            fall_start_y = feet_now_y
    else:
        # No ground in this column: hole. Track where the fall began if we were standing.
        if bool(next_state.get("grounded", false)):
            fall_start_y = feet_was_y

    if landed:
        # Fall damage: if we fell from a height above this surface, take 1pt per landing beyond a 32px drop
        if not bool(state.get("grounded", false)) and ground_y > fall_start_y:
            var fall_dist: float = ground_y - fall_start_y
            if fall_dist > 32.0:
                hp = max(0.0, hp - 1.0)
        next_state["pos_y"] = ground_y
        next_state["vel_y"] = 0.0
        next_state["grounded"] = true
        next_state["fall_start_y"] = ground_y
    else:
        next_state["pos_y"] = feet_now_y
        next_state["vel_y"] = next_vel_y
        next_state["grounded"] = false
        next_state["fall_start_y"] = fall_start_y

    next_state["pos_x"] = next_pos_x
    next_state["vel_x"] = move_x * WALKER_SPEED_PPS
    next_state["hp"] = hp
    return next_state


func _rebuild_ground_tiles() -> void:
    _tiles_cache.clear()
    if _screen_store == null:
        return
    var tw: int = int(_grid_config.get("tile_width_in_pixels", 64))
    var th: int = int(_grid_config.get("tile_height_in_pixels", 64))
    var grid_dict: Dictionary = {"tile_w": float(tw), "tile_h": float(th)}
    # default grid dimensions if missing
    var max_w: int = int(_grid_config.get("max_tiles_per_screen_x", 60))
    var max_h: int = int(_grid_config.get("max_tiles_per_screen_y", 33))
    for pos in _screen_store.occupied_positions():
        var cached: Dictionary = _screen_store.get_cache(pos.x, pos.y)
        var screen_data: Dictionary = cached
        if screen_data.is_empty():
            screen_data = _screen_store.screen_at(pos.x, pos.y)
        var tiles_arr: Array = screen_data.get("tiles", []) as Array
        var sx: int = pos.x
        var sy: int = pos.y
        for td in tiles_arr:
            var wx_local = int(td.get("x", 0))
            var wy_local = int(td.get("y", 0))
            var terr = String(td.get("terrain", ""))
            # walkable heuristic: terrain types with is_walkable true by default
            var walkable = true
            if terr == "air" or terr == "wall" or terr == "lava":
                walkable = false
            _tiles_cache.append({"world_tile_x": sx * max_w + wx_local, "world_tile_y": sy * max_h + wy_local, "is_walkable": walkable})
    if _ground_layer != null and _ground_layer.has_method("set_data"):
        _ground_layer.call("set_data", _tiles_cache, grid_dict)
    _ground_y_by_col = build_ground_column_table(_tiles_cache, float(th))
    _spawn_walker_on_ground(tw)


# Issue #122 -- put the walker on the leftmost column that has ground, feet exactly ON the
# surface, and aim the camera at it once so the ground line is on screen at all.
#
# Only when the walker is not already standing: `_rebuild_ground_tiles` also runs on every tab
# switch, and respawning there would teleport the avatar back mid-walk. A walker that fell
# through a hole counts as not standing, so switching tabs recovers it.
func _spawn_walker_on_ground(tile_w: int) -> void:
    if _avatar == null or _ground_y_by_col.is_empty():
        return
    if bool(_walker_state.get("grounded", false)):
        return
    var first_col: int = int(_ground_y_by_col.keys().min())
    _place_walker_on_col(first_col, tile_w)
    if _camera_follow_enabled:
        move_camera_to(float(_walker_state["pos_x"]), GROUND_VIEW_CENTER_Y_PX)
        follow_avatar()


# Entering the Simulator starts the walk on the screen the other tabs were on. The camera jump
# already centred there, so the avatar has to stand there too: #119/#120 pin that jump, and a tab
# that opens on a view without the avatar looks broken even though the walker is fine.
func reset_walker_on_screen(screen_id: int) -> void:
    if _avatar == null or _screen_store == null or not _screen_store.has_id(screen_id):
        return
    var cols_per_screen: int = int(_grid_config.get("max_tiles_per_screen_x", 60))
    var first_col_of_screen: int = _screen_store.position_of_id(screen_id).x * cols_per_screen
    var cols_on_screen: Array = _ground_y_by_col.keys().filter(
        func(world_col: int) -> bool:
            return world_col >= first_col_of_screen and world_col < first_col_of_screen + cols_per_screen)
    if cols_on_screen.is_empty():
        return                                  # ground exists, but not on this screen
    _place_walker_on_col(int(cols_on_screen.min()), int(_grid_config.get("tile_width_in_pixels", 32)))
    _frame_walker_on_entry.call_deferred()


# Screen-centred cameras can exclude ground near the bottom of a screen entirely. Frame the
# actual feet after tab layout, leaving one quarter of the visible height beneath the ground.
func _frame_walker_on_entry() -> void:
    if not _camera_follow_enabled or _camera == null or _viewport == null:
        return
    var ground_clearance_fraction: float = 0.25
    var visible_height_px: float = float(_viewport.size.y) / _camera.zoom.y
    follow_avatar()
    move_camera_to(_camera.position.x,
        float(_walker_state["pos_y"]) - visible_height_px * ground_clearance_fraction)
    print("walker.frame feet=%s camera=%s viewport=%s zoom=%s" % [
        _avatar.position, _camera.position, _viewport.size, _camera.zoom])


# Both spawn paths end here: state on the surface of `world_col`, node mirroring it. The camera is
# the caller's business, because boot and tab entry want different camera behaviour.
func _place_walker_on_col(world_col: int, tile_w: int) -> void:
    _walker_state = {
        "pos_x": (float(world_col) + 0.5) * float(tile_w),   # cell centre, not the cell edge
        "pos_y": float(_ground_y_by_col[world_col]),
        "vel_x": 0.0,
        "vel_y": 0.0,
        "grounded": true,
        "hp": 100.0,
        "fall_start_y": float(_ground_y_by_col[world_col]),
    }
    _avatar.position = Vector2(float(_walker_state["pos_x"]), float(_walker_state["pos_y"]))


func _update_walker(dt: float) -> void:
    if Engine.is_editor_hint():
        return
    if not is_visible_in_tree():
        return
    if _avatar == null:
        return
    # A/D only. The arrows belong to the editor camera (#120) -- holding one pans the view, and
    # the avatar must not move at the same time. W/S are read by nothing: no jump, no crouch.
    var move_x: float = 0.0
    if Input.is_key_pressed(KEY_A):
        move_x -= 1.0
    if Input.is_key_pressed(KEY_D):
        move_x += 1.0
    _walker_state = step_walker(
        _walker_state,
        move_x,
        dt,
        _ground_y_by_col,
        float(_grid_config.get("tile_width_in_pixels", 32)),
        _map_world_px().position.x,
        _map_world_px().end.x)
    _avatar.position = Vector2(
        float(_walker_state.get("pos_x", 0.0)), float(_walker_state.get("pos_y", 0.0)))
    if move_x != 0.0:
        follow_avatar()
    # Periodic log every _log_interval ms: position + right edge terrain
    var now_ms = Time.get_ticks_msec()
    if now_ms - _last_log_time >= _log_interval:
        _last_log_time = now_ms
        _log_avatar_position()


func _log_avatar_position() -> void:
    if _avatar == null:
        return
    var pos = _avatar.position
    var map_world_px = _map_world_px()
    if map_world_px.size.x <= 0.0:
        return
    var screen_right_edge = map_world_px.end.x
    print("AVATAR_POS: %.1f, %.1f | RIGHT_EDGE_TERRAIN: %s" % [
        pos.x, pos.y,
        get_terrain_at_position(screen_right_edge, pos.y)
    ])
    
func get_terrain_at_position(world_x: float, world_y: float) -> String:
    if _screen_store == null:
        return "no_store"
    var tile_w = int(_grid_config.get("tile_width_in_pixels", 32))
    var tile_h = int(_grid_config.get("tile_height_in_pixels", 32))
    var col = int(world_x / tile_w)
    var row = int(world_y / tile_h)
    var screen_col = col / 60  # SCREEN_COLS = 60
    var local_col = col % 60
    var screen_data = _screen_store.get_cache(screen_col, 0)
    if screen_data.is_empty():
        screen_data = _screen_store.screen_at(screen_col, 0)
    var tiles_arr = screen_data.get("tiles", []) as Array
    for td in tiles_arr:
        if int(td.get("x", 0)) == local_col and int(td.get("y", 0)) == row:
            return String(td.get("terrain", "unknown"))
    return "none_at_pos"

# The game camera follows the avatar along x and leaves y alone, so a middle-drag pan or a zoom
# survives the next frame. While walking, called only while a direction key is held: standing
# still must not re-centre the view, or the arrow-key pan and the entry jump would be undone one
# frame after the user made them. Entering the Simulator calls it once explicitly, because the
# avatar is what the Simulator is for and it may not be on the screen the other tabs were on.
func follow_avatar() -> void:
    if not _camera_follow_enabled:
        return
    if _camera == null or _viewport == null:
        return
    var map_world_px: Rect2 = _map_world_px()
    if map_world_px.size.x <= 0.0:
        return
    var view_world_px: Vector2 = _viewport.size / _camera.zoom.x   # Camera2D zoom = magnification
    move_camera_to(
        CameraController.clamp_center(
            float(_walker_state.get("pos_x", 0.0)),
            view_world_px.x,
            map_world_px.position.x,
            map_world_px.end.x),
        _camera.position.y)


func _handle_place_interact(place_flag: bool, interact_flag: bool) -> void:
    if Engine.is_editor_hint():
        return
    if _text_entry_has_focus():
        return
    if _bridge_obj == null or not _bridge_obj.has_method("walk_step"):
        return
    var tw: int = int(_grid_config.get("tile_width_in_pixels", 64))
    var th: int = int(_grid_config.get("tile_height_in_pixels", 64))
    var inp = _walk_input.duplicate()
    inp["place"] = place_flag
    inp["alt_place"] = interact_flag
    inp["interact"] = interact_flag
    var req: Dictionary = {
        "grid": {"tile_w": float(tw), "tile_h": float(th)},
        "tiles": _tiles_cache,
        "input": inp,
        "walker": _walker_state,
        "dt": 0.0
    }
    var res_json = _bridge_obj.call("walk_step", JSON.stringify(req))
    # response ignored for now; placement logic will be expanded
    var _parsed = JSON.parse_string(res_json)


func _notification(what: int) -> void:
    if what == NOTIFICATION_VISIBILITY_CHANGED:
        if is_visible_in_tree():
            _rebuild_ground_tiles()
            if _bridge_obj == null:
                var root = get_tree().root
                if root != null:
                    var main = root.get_node_or_null("Main")
                    if main != null and main.get("_bridge") != null:
                        _bridge_obj = main.get("_bridge")


func try_find_bridge(node: Node) -> Object:
    if node == null:
        return null
    if node.get("_bridge") != null:
        return node.get("_bridge")
    if node.has_method("get_bridge"):
        var b = node.call("get_bridge")
        if b != null:
            return b
    for c in node.get_children():
        var r = try_find_bridge(c)
        if r != null:
            return r
    return null


# helper to set bridge from outside
func try_get_bridge() -> void:
    if _bridge_obj != null:
        return
    var root = get_tree().root
    if root != null:
        _bridge_obj = try_find_bridge(root)


func get_avatar_position() -> Vector2:
    if _avatar == null:
        return Vector2.ZERO
    return _avatar.position
