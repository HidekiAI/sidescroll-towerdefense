extends SceneTree
# Issue #107 (parallax step 4) -- WorldView against the real bridge.
#
# Rewritten from the Parallax2D suite that used to live here. That suite asserted per-layer
# `Parallax2D.position` drift and two scrollbars scrubbing one camera; both are gone with
# `backdrop_preview.tscn` and `simulator.gd` (plan section 4.9). What is asserted now is the
# claim the design of record makes, at the level the design actually owns:
#
#   G1      the two planes receive the right placements, from ONE bridge call
#   G9      a resize updates view_w_px / view_h_px and queues exactly one redraw
#   policy  an unchanged ViewState queues no redraw at all
#   seam    ParallaxDraw stores what it is handed and draws only that (D1-D5, absorbed here
#           from the working copy test_parallax_draw.gd, which is not one of plan section 6's
#           six suites -- see the section 6 note in the plan)
#
# Headless has no renderer (Math section 7), so this suite proves the DATA path only. Nothing
# here may be cited as proof that something is drawn; G5, G7 and S1 do that, windowed.
#
# WHY THE REDRAW COUNT IS ASSERTED AS A QUERY COUNT. `queue_redraw()` has exactly one call site,
# inside `ParallaxDraw.apply()`, and `apply()` is called only from
# `WorldView._refresh_placements()`, which runs only when the polled ViewState changed. Queries,
# apply() calls and redraws are therefore the same number, and the bridge is the only place that
# number is observable. Adding a second `queue_redraw()` site would break that correspondence, so
# this suite would then need a redraw probe of its own.
#
# WHY A RESIZE IS PERFORMED ON THE CONTROL, NOT ON THE VIEWPORT. `world_view.tscn` sets
# `stretch = true`, which makes SubViewportContainer drive SubViewport.size from its own rect on
# every layout pass: assigning `viewport.size` directly is overwritten on the next frame, so a
# test that did that would be measuring its own assignment. Resizing the WorldView Control is how
# a resize actually reaches the viewport.
#
# Issue #81's silent-truncation mode is guarded the same way the other #107 suites guard it: each
# guard appends its name to `_completed`, and the runner fails if any name is missing. The exit
# code is what gates, never the printed counter.

const WORLD_VIEW_SCENE: String = "res://scenes/world_view.tscn"
const DRAW_SCRIPT_PATH: String = "res://scripts/parallax_draw.gd"
const FIXTURE_DIR: String = "res://tests/fixtures"

# (960, 540) is the view centre at the ref view size, and the default anchor for a one-screen
# map, so it should be accepted unclamped. The anchor is set explicitly rather than left to the
# default because the default DERIVES FROM THE VIEWPORT, which is 189x33 headless -- a derived
# anchor would make the expected placements depend on the window size.
const ANCHOR_WORLD_PX: Vector2 = Vector2(960.0, 540.0)

# The two sizes in G9. Deliberately asymmetric on the axes so a swapped width/height fails.
const RESIZED_VIEW_SIZE: Vector2i = Vector2i(1280, 720)
const FRAMES_TO_SETTLE: int = 3
const FRAMES_TO_PROVE_NOTHING_ELSE_HAPPENS: int = 5

# The transparent wire token, spelled once. `parallax_draw.gd` owns the constant; repeating the
# literal here is the on-disk pin the wire-token rule asks for -- a rename has to be caught
# somewhere that does not move with the code.
const FILL_TRANSPARENT_TOKEN: String = "transparent"
const FILL_SKY_RGBA: String = "#8A98ACFF"

const GUARDS: Array[String] = ["g1", "g9", "policy", "d1", "d2", "d3", "d4", "d5"]

var _failures: int = 0
var _completed: Array[String] = []


# ParallaxLoader with the bridge call recorded. `query_placements` is the loader's public seam --
# `load_stack` is what calls it -- so overriding it records every request WorldView actually sent
# without duplicating any of the request building.
class RecordingLoader extends ParallaxLoader:
    var requests: Array[Dictionary] = []

    func query_placements(request: Dictionary) -> Dictionary:
        requests.append(request)
        return super.query_placements(request)


func check(condition: bool, message: String) -> void:
    if condition:
        print("ok: " + message)
    else:
        _failures += 1
        printerr("FAIL: " + message)


func _initialize() -> void:
    _run.call_deferred()


func _run() -> void:
    await _test_world_view()
    await _test_draw_seam()
    _assert_all_guards_completed()
    print("=== test_parallax_backdrop: %d failures ===" % _failures)
    quit(0 if _failures == 0 else 1)


func _assert_all_guards_completed() -> void:
    for guard_name in GUARDS:
        if not _completed.has(guard_name):
            _failures += 1
            printerr("FAIL: guard '%s' did not complete" % guard_name)


# --- G1, G9, the redraw policy, and placements equality ---------------------------


func _test_world_view() -> void:
    print("--- WorldView against the bridge (issue #107) ---")
    var bridge: Object = ClassDB.instantiate("SstdBridge")
    if bridge == null:
        check(false, "SstdBridge is not registered; run scripts/build-bridge.sh first")
        _completed.append_array(GUARDS)
        return

    var world_view: Control = (load(WORLD_VIEW_SCENE) as PackedScene).instantiate() as Control
    if world_view == null:
        check(false, "world_view.tscn did not instantiate as a Control")
        _completed.append_array(GUARDS)
        return

    # The loader is swapped in AFTER add_child, not before. `_ready()` builds WorldView's own
    # loader, so an assignment made before the node entered the tree was simply overwritten -- the
    # first run of this suite queried through the real loader and recorded nothing, which is what
    # "got 0" looked like. After `_ready()` and before the first `_process` is the window in which
    # nothing has queried yet: `_ready()` only clears the last ViewState to force one query.
    # GDScript has no field privacy, and this is the same seam `set_bridge` documents -- that one
    # sets the bridge on WorldView's loader, while recording the request needs the loader itself.
    var recording := RecordingLoader.new()
    recording.set_bridge(bridge)
    world_view.set_parallax_anchor(ANCHOR_WORLD_PX.x, ANCHOR_WORLD_PX.y)
    root.add_child(world_view)
    world_view._loader = recording
    for frame_index in range(FRAMES_TO_SETTLE):
        await process_frame

    # The planes are CanvasLayers, not Node2Ds: a layer index is what puts the back plane under
    # the (empty in #107) World node and the front plane over it, and a Node2D sibling would be
    # ordered by tree position instead.
    var back_plane: CanvasLayer = world_view.get_node("SubViewportContainer/Viewport/BackPlane") as CanvasLayer
    var front_plane: CanvasLayer = world_view.get_node("SubViewportContainer/Viewport/FrontPlane") as CanvasLayer
    var back_draw: ParallaxDraw = back_plane.get_node("ParallaxDraw") as ParallaxDraw
    var front_draw: ParallaxDraw = front_plane.get_node("ParallaxDraw") as ParallaxDraw
    var camera: Camera2D = world_view.camera() as Camera2D
    var viewport: SubViewport = world_view.viewport() as SubViewport
    if back_draw == null or front_draw == null or camera == null or viewport == null:
        check(false, "WorldView is missing a plane, its camera or its viewport")
        _completed.append_array(["g1", "g9", "policy"])
        return

    # --- G1: both planes visible, and both fed from one call ---
    check(back_plane.visible, "G1: BackPlane is visible (mutation: set it false)")
    check(front_plane.visible, "G1: FrontPlane is visible")

    var held_placements: Array = world_view.last_placements()
    check(held_placements.size() > 0, "G1: WorldView holds placements from the bridge")
    if held_placements.is_empty():
        _completed.append_array(["g1", "g9", "policy"])
        return
    check(recording.requests.size() == 1,
        "G1: ONE bridge call feeds both planes (got %d)" % recording.requests.size())

    # Replay the exact request the loader sent, through the same bridge. Reconstructing it here
    # would duplicate the loader's request building, and the comparison would then only prove the
    # copy agrees with itself.
    var bridge_placements: Array = _bridge_result(bridge, recording.requests.back()).get("placements", []) as Array
    check(held_placements == bridge_placements,
        "G1: WorldView holds the bridge's placements, unmodified")
    check(back_draw.last_placements() + front_draw.last_placements() == held_placements,
        "G1: the two planes' slices concatenate back to the bridge's array, in order")

    # The on-disk pin: `stack.json` names these layers, and the plane each one lands on is the
    # manifest's own `plane` field. A rename of the stack, or of the plane split, has to be
    # caught somewhere that does not move with the code.
    var held_names: Array = []
    for entry_index in range(held_placements.size()):
        held_names.append(String((held_placements[entry_index] as Dictionary).get("name", "")))
    check(held_names.has("Sky") and held_names.has("Foreground"),
        "G1: the neutral_default stack really is the one loaded")
    check(_all_on_plane(back_draw.last_placements(), "back")
        and _all_on_plane(front_draw.last_placements(), "front"),
        "G1: back-plane entries are in BackPlane and front-plane entries in FrontPlane")
    check(_draw_order_ascending(back_draw.last_placements())
        and _draw_order_ascending(front_draw.last_placements()),
        "G1: each plane's entries are in ascending draw_order")

    _completed.append("g1")

    # --- G9 + the redraw policy ---
    #
    # Resized on the CONTROL for the reason in the header: `stretch = true` means the container
    # owns the viewport size, so assigning `viewport.size` here would be overwritten next frame.
    var queries_before_resize: int = recording.requests.size()
    world_view.set_anchors_preset(Control.PRESET_TOP_LEFT)
    world_view.size = Vector2(RESIZED_VIEW_SIZE)
    for frame_index in range(FRAMES_TO_SETTLE):
        await process_frame

    check(viewport.size == RESIZED_VIEW_SIZE,
        "G9: the viewport follows a resize of the Control (got %s)" % viewport.size)
    check(recording.requests.size() == queries_before_resize + 1,
        "G9: a resize queues exactly one redraw, i.e. one new query (got %d new)" % (
            recording.requests.size() - queries_before_resize))
    var resized_request: Dictionary = recording.requests.back()
    var resized_view: Dictionary = resized_request.get("view", {}) as Dictionary
    check(int(resized_view.get("view_w_px", 0)) == RESIZED_VIEW_SIZE.x
        and int(resized_view.get("view_h_px", 0)) == RESIZED_VIEW_SIZE.y,
        "G9: the new query carries the new view size (%s)" % resized_view)
    # The drawer's own copy of the view size is what `_draw()` letterboxes against, so it has to
    # move too. Read from the field because `apply()` takes the size as an argument and stores it.
    check(is_equal_approx(back_draw._view_w_px, float(RESIZED_VIEW_SIZE.x))
        and is_equal_approx(back_draw._view_h_px, float(RESIZED_VIEW_SIZE.y)),
        "G9: the drawer stored the new view size")
    _completed.append("g9")

    var queries_before_idle: int = recording.requests.size()
    for frame_index in range(FRAMES_TO_PROVE_NOTHING_ELSE_HAPPENS):
        await process_frame
    check(recording.requests.size() == queries_before_idle,
        "policy: an unchanged ViewState queues nothing (got %d extra queries)" % (
            recording.requests.size() - queries_before_idle))
    _completed.append("policy")


func _all_on_plane(placements: Array, wanted_plane: String) -> bool:
    for entry_index in range(placements.size()):
        if String((placements[entry_index] as Dictionary).get("plane", "")) != wanted_plane:
            return false
    return true


func _draw_order_ascending(placements: Array) -> bool:
    var previous_order: int = -0x7fffffff
    for entry_index in range(placements.size()):
        var order: int = int((placements[entry_index] as Dictionary).get("draw_order", 0))
        if order < previous_order:
            return false
        previous_order = order
    return true


func _bridge_result(bridge: Object, request: Dictionary) -> Dictionary:
    var response_text: String = String(bridge.call("parallax_placements", JSON.stringify(request)))
    var parsed: Variant = JSON.parse_string(response_text)
    if typeof(parsed) != TYPE_DICTIONARY:
        return {"placements": []}
    return parsed as Dictionary


# --- the ParallaxDraw draw seam (D1-D5) -------------------------------------------


# A minimal placement, shaped like the bridge's own PlacementEntry. Every optional key the
# drawer reads is present; D5 strips them.
func _placement(copy_count: int) -> Dictionary:
    # `file` is the BARE filename: apply() resolves it against stack_dir, because rewriting the
    # placement would break the G1 equality against the bridge's own array.
    return {
        "name": "Sky", "file": "base.png",
        "plane": "back", "copy_count": copy_count,
        "period_px": 1920.0, "first_copy_left_px": 0.0,
        "screen_top_px": 120.0, "art_h_px": 960.0, "scale": 1.0,
        "fill_above": FILL_SKY_RGBA, "fill_below": FILL_TRANSPARENT_TOKEN,
    }


func _new_drawer() -> Node2D:
    var drawer_script: GDScript = load(DRAW_SCRIPT_PATH) as GDScript
    if drawer_script == null:
        check(false, "ParallaxDraw script failed to load from " + DRAW_SCRIPT_PATH)
        return null
    var drawer := drawer_script.new() as Node2D
    if drawer == null:
        check(false, "ParallaxDraw did not instantiate as Node2D")
        return null
    root.add_child(drawer)
    return drawer


func _test_draw_seam() -> void:
    print("--- ParallaxDraw draw seam (issue #107) ---")
    var drawer := _new_drawer()
    if drawer == null:
        _completed.append_array(["d1", "d2", "d3", "d4", "d5"])
        return

    var first_set: Array = [_placement(3)]
    var second_set: Array = [_placement(5)]

    # --- D1: apply() stores, it does not recompute ---
    # The identity half is the point: `==` alone would pass on a rebuilt copy, and the drawer's
    # whole claim is that it owns no maths of its own.
    drawer.call("apply", first_set, 1920.0, 1080.0, FIXTURE_DIR)
    var handed_back: Array = drawer.call("last_placements")
    check(handed_back == first_set, "D1: last_placements() equals the array handed to apply()")
    check(is_same(handed_back[0], first_set[0]),
        "D1: the entries are the SAME dictionaries, not rebuilt copies")
    _completed.append("d1")

    # --- D2: a second apply replaces ---
    drawer.call("apply", second_set, 1920.0, 1080.0, FIXTURE_DIR)
    check(drawer.call("last_placements") == second_set,
        "D2: a second apply() replaces the first, it does not append")
    _completed.append("d2")

    # --- D3: an empty apply clears ---
    drawer.call("apply", [], 1920.0, 1080.0, FIXTURE_DIR)
    check((drawer.call("last_placements") as Array).is_empty(),
        "D3: an empty apply() clears the drawn set")
    _completed.append("d3")

    # --- D4: _fill_colour accepts the two wire spellings only ---
    check(drawer.call("_fill_colour", FILL_TRANSPARENT_TOKEN) == Color(0, 0, 0, 0),
        "D4: the transparent token yields alpha 0")
    check(drawer.call("_fill_colour", "") == Color(0, 0, 0, 0),
        "D4: an empty fill yields alpha 0")
    check(drawer.call("_fill_colour", FILL_SKY_RGBA) == Color(0x8A / 255.0, 0x98 / 255.0, 0xAC / 255.0, 1.0),
        "D4: #RRGGBBAA parses to that colour")
    check(drawer.call("_fill_colour", "#8A98AC") == Color(0, 0, 0, 0),
        "D4: a 6-digit hex is rejected to transparent, never drawn as a band")
    check(drawer.call("_fill_colour", "#8A98ACGG") == Color(0, 0, 0, 0),
        "D4: a non-hex digit is rejected to transparent")
    _completed.append("d4")

    # --- D5: _draw() survives a bare placement ---
    # No file, no period, no fill: the texture lookup misses, both fills are transparent, and
    # nothing draws. The assertion is that the next frame still runs -- a script error here would
    # abort the guard before it appends, which is exactly what the completed-guard check catches.
    drawer.call("apply", [{"name": "bare"}], 1920.0, 1080.0, FIXTURE_DIR)
    await process_frame
    await process_frame
    check((drawer.call("last_placements") as Array).size() == 1,
        "D5: a placement with no optional keys is stored and drawn without error")
    _completed.append("d5")
