class_name CameraController extends Node
# Moves the game camera to follow a target on BOTH axes, clamped to the map's camera bound.
#
# Design of record: PLAN-2026-10-16-parallax-step4.md section 4.5;
# TDD_Parallax-Scrolling.Math.md section 5.
#
# Two refusals it owns outright:
#   - zoom stays 1. `place_layer` derives the whole parallax ladder from the zoom, so a second
#     zoom control is a second source of truth for the same number. `set_zoom()` says no
#     (G6's mutation is "allow zoom").
#   - no tween. A teleport is a cut: set the centre in one frame (G6's other mutation is
#     "add a tween").
#
# "No lag" (G13) is a property of WHAT the caller hands in: `set_target_world_px` must receive the
# interpolated render position, never the tick position, because the tick is up to one tick behind
# what is on screen. This controller applies the target in the same frame it is set, so it adds no
# lag of its own.

# The wire token `parallax.anchor CLAMP` and `parallax.camera REFUSE_ZOOM` lines are logged at the
# same boundary as the bridge's own journal lines, so a camera that will not go where it was told
# is traceable from the output log.
const ZOOM_ALLOWED: float = 1.0

var _camera: Camera2D = null
var _target_world_px: Vector2 = Vector2.ZERO
var _bound_low_px: Vector2 = Vector2.ZERO
var _bound_high_px: Vector2 = Vector2.ZERO
var _view_extent_px: Vector2 = Vector2.ZERO
var _has_target: bool = false


func attach(camera: Camera2D) -> void:
    _camera = camera
    if _camera != null:
        # Enforced, not merely refused: a zoom written by anything else would silently change the
        # parallax ladder, so the controller puts it back every time it takes the camera over.
        _camera.zoom = Vector2(ZOOM_ALLOWED, ZOOM_ALLOWED)


# The map's camera bound in world px, low edge inclusive and high edge exclusive.
func set_map_bound(low_world_px: Vector2, high_world_px: Vector2) -> void:
    _bound_low_px = low_world_px
    _bound_high_px = high_world_px


# The view size in world px, i.e. viewport size divided by zoom. With zoom pinned at 1 that is
# just the viewport size, and it is re-read on resize rather than cached at _ready().
func set_view_extent_px(view_w_px: float, view_h_px: float) -> void:
    _view_extent_px = Vector2(view_w_px, view_h_px)


# Refuse any zoom but 1 and say so. Returns whether it was accepted.
func set_zoom(zoom_value: float) -> bool:
    if zoom_value != ZOOM_ALLOWED:
        print("parallax.camera REFUSE_ZOOM wanted=%s allowed=%s" % [str(zoom_value), str(ZOOM_ALLOWED)])
        if _camera != null:
            _camera.zoom = Vector2(ZOOM_ALLOWED, ZOOM_ALLOWED)
        return false
    return true


# The target for this frame, in world px. Call once per frame with the interpolated position.
func set_target_world_px(world_px_x: float, world_px_y: float) -> void:
    _target_world_px = Vector2(world_px_x, world_px_y)
    _has_target = true


func camera() -> Camera2D:
    return _camera


func _process(_delta_seconds: float) -> void:
    if _camera == null or not _has_target:
        return
    # Both axes, every frame. A controller that only followed x would leave the camera pinned at
    # its start height, which is G13's "follow x only" mutation.
    _camera.position = Vector2(
        clamp_center(_target_world_px.x, _view_extent_px.x, _bound_low_px.x, _bound_high_px.x),
        clamp_center(_target_world_px.y, _view_extent_px.y, _bound_low_px.y, _bound_high_px.y))


# Transcribed from `sstd-core::parallax::clamp_center`, the design of record for this branch
# (crates/sstd-core/src/parallax.rs). It is deliberately not exposed through the bridge: a
# `#[func]` for one call per axis per frame is the cost, and the plan records that choice
# (section 4.5).
#
# Static so the editor camera (`WorldView`) can share this one copy instead of transcribing it a
# second time; the two callers differ only in where their bound comes from.
#
# The small-bound case is the one the source comment calls out as "the one that gets written
# wrong": when the bound is narrower than the view there is no range to clamp into (low would
# exceed high), so the view is lined up with the bound's low edge instead. That is bound_low plus
# half a view, NOT the bound's midpoint -- Math section 5's zoom-0.25 example fixes the centre at
# 1104 where the midpoint would be 1056.
static func clamp_center(center_value: float, view_extent_px: float, bound_low_px: float, bound_high_px: float) -> float:
    var half_extent_px: float = view_extent_px * 0.5
    if bound_high_px - bound_low_px < view_extent_px:
        return bound_low_px + half_extent_px
    return clampf(center_value, bound_low_px + half_extent_px, bound_high_px - half_extent_px)
