extends SceneTree
# Issue #107 (parallax step 4) -- gate G19: the stretch keeps 1920x1080 as the parallax reference
# coordinate space.
#
# MEASURED values this suite asserts (probe_stretch.gd, Godot 4.7.2.stable.mono, 1920x1080 display):
#   - project.godot stretch mode is DISABLED (mode=0) -- the app does NOT stretch by project
#     setting. The 1920x1080 reference is held by the Runtime CONTENT scale instead.
#   - game_view.gd sets content_scale_mode = CANVAS_ITEMS (1), content_scale_aspect = KEEP (1),
#     and content_scale_size = (1920, 1080) in _ready().
#   - With the window at 1920x1200, get_visible_rect() = 1920x1080 (the content area), and
#     get_final_transform().origin.y = 60 -- the 120 px window surplus split as the KEEP letterbox
#     bar (60 px each side).
#
# So the reference is held by the runtime content scale, NOT by project.godot. The G19 mutation
# ("remove the content_scale_size=1920x1080 set in game_view") fires here: without it the content
# would track the window and the reference size would no longer be 1920x1080.
#
# The suite runs WINDOWED (a real window is required to observe the letterbox), so it is not a
# headless suite:
#   DISPLAY=:0.0 $HOME/bin/godot4 --path editor --script res://tests/test_parallax_stretch.gd
#
# Guards run in the #107 style: each appends its name to _completed, the runner fails on a missing
# name, and the exit code gates -- never the printed counter.

const GAME_VIEW_SCENE: String = "res://scenes/game.tscn"

# The window height the suite opens with -- 120 px taller than the reference, so KEEP letterboxes
# and the 60 px bar is observable. The width matches the reference so the bar is vertical-only.
const WINDOW_SIZE: Vector2i = Vector2i(1920, 1200)
# The reference the parallax math assumes and game_view.gd's content scale holds.
const REFERENCE_SIZE: Vector2i = Vector2i(1920, 1080)
# The KEEP letterbox splits the 120 px window surplus as 60 px top and 60 px bottom.
const LETTERBOX_BAR_PX: int = 60

const GUARDS: Array[String] = ["g19_stretch"]

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
    root.size = WINDOW_SIZE
    var game_view: GameView = (load(GAME_VIEW_SCENE) as PackedScene).instantiate()
    root.add_child(game_view)
    for frame_index in range(4):
        await process_frame

    await _test_g19_stretch()
    _assert_all_guards_completed()
    print("=== test_parallax_stretch: %d failures ===" % _failures)
    quit(0 if _failures == 0 else 1)


func _assert_all_guards_completed() -> void:
    for guard_name in GUARDS:
        if not _completed.has(guard_name):
            _failures += 1
            printerr("FAIL: guard '%s' did not complete" % guard_name)


# --- G19: the runtime content scale holds 1920x1080 as the reference -------------------------

func _test_g19_stretch() -> void:
    print("--- G19 runtime stretch holds the 1920x1080 reference (issue #107) ---")
    # Project stretch is disabled; the reference is not coming from project.godot. The
    # content_scale_* members are Window properties, so the enums live on Window.
    check(root.content_scale_mode == Window.CONTENT_SCALE_MODE_CANVAS_ITEMS,
        "G19: content_scale_mode is CANVAS_ITEMS (set by game_view, not project.godot)")
    check(root.content_scale_aspect == Window.CONTENT_SCALE_ASPECT_KEEP,
        "G19: content_scale_aspect is KEEP, so the reference is letterboxed, not cropped")
    check(root.content_scale_size == REFERENCE_SIZE,
        "G19: content_scale_size is 1920x1080 -- the parallax reference coordinate space")

    # A real window taller than the reference letterboxes: the visible rect is the reference, and
    # the KEEP bar is the surplus split evenly.
    var visible_rect: Rect2 = root.get_visible_rect()
    check(visible_rect.size == Vector2(REFERENCE_SIZE),
        "G19: at a %dx%d window the visible rect is %dx%d (the reference)" % [WINDOW_SIZE.x, WINDOW_SIZE.y, REFERENCE_SIZE.x, REFERENCE_SIZE.y])
    check(int(root.get_final_transform().origin.y) == LETTERBOX_BAR_PX,
        "G19: the KEEP letterbox bar is %d px, so the reference origin is offset into the window" % LETTERBOX_BAR_PX)
    _completed.append("g19_stretch")
