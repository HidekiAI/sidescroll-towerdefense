class_name GameView extends Node
# The game client's root: it owns the runtime stretch and hosts the game viewport.
#
# Why here and not in `project.godot` (PLAN-2026-10-16-parallax-step4.md section 4.6, gate G19):
# the editor and the game share one `project.godot`. Stretching there would also stretch the
# editor panel, whose `WorldView` must letterbox itself at whatever size the Simulator tab is.
# G19 asserts both halves: the game window letterboxes to 60 px bars at 1920 x 1200, and
# `project.godot` still reads `content_scale_mode = disabled`.

const GAME_LOGICAL_WIDTH_PX: int = 1920
const GAME_LOGICAL_HEIGHT_PX: int = 1080

var _world_view: WorldView = null
var _camera_controller: CameraController = null


func _ready() -> void:
    # A real window is required: the headless run has no window to scale, so G19 runs windowed.
    var game_window := get_window()
    if game_window != null:
        game_window.content_scale_size = Vector2i(GAME_LOGICAL_WIDTH_PX, GAME_LOGICAL_HEIGHT_PX)
        # CANVAS_ITEMS scales the canvas rather than resampling a texture, so the parallax art
        # keeps its NEAREST-filtered pixels at any window size.
        game_window.content_scale_mode = Window.CONTENT_SCALE_MODE_CANVAS_ITEMS
        # KEEP letterboxes: a window that is not 16:9 gains bars instead of stretching.
        game_window.content_scale_aspect = Window.CONTENT_SCALE_ASPECT_KEEP

    _world_view = get_node_or_null("WorldView") as WorldView
    _camera_controller = get_node_or_null("CameraController") as CameraController
    if _camera_controller != null and _world_view != null:
        _camera_controller.attach(_world_view.camera())


func world_view() -> WorldView:
    return _world_view


func camera_controller() -> CameraController:
    return _camera_controller
