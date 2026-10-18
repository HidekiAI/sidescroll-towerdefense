@tool
class_name WorldView extends Control
# WorldView owns the game viewport: one SubViewport, the Camera2D inside it, and the two
# ParallaxDraw planes that straddle the (empty in #107) World node.
#
# Its only job beyond that ownership is the redraw policy of PLAN-2026-10-16-parallax-step4.md
# section 4.4: poll the ViewState, and re-query the bridge only when it actually changed. Polling
# rather than signals is deliberate — `Camera2D.position` and `SubViewport.size` are plain
# properties with no change signal, and a one-frame latency in a backdrop is invisible while the
# polling cost is a dictionary compare.
#
# Scene shape: section 4.1 of the same plan. Anchors on `SubViewportContainer` are mandatory
# (Godot4-Learnings section 1): without them the rect is 0x0, the SubViewport is driven to 2x2,
# and nothing draws while nothing errors.
#
# G3 (the SubViewport keeps the editor's UI undisturbed) holds because the Camera2D is inside the
# SubViewport: moving it moves that canvas only.

const FRONT_PLANE_KEY: String = "front"
const DEFAULT_STACK_PATH: String = "res://assets/backdrop_stacks/neutral_default/stack.json"

var _loader: ParallaxLoader = null
var _back_draw: ParallaxDraw = null
var _front_draw: ParallaxDraw = null
var _camera: Camera2D = null
var _viewport: SubViewport = null
var _status_label: Label = null

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
