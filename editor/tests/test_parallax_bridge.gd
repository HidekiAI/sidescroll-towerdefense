extends SceneTree

# Issue #106 (parallax step 3) -- the bridge's `parallax_placements` seam must be callable from
# GDScript and must carry the placement values through unchanged.
#
# This is the half of the contract no Rust test can prove. A `#[func]` is reachable only if the
# BUILT extension exports it, which is a property of the shared object under `editor/rust/`, not
# of the pure function the Rust tests cover. The `.so` on this checkout predates the method, so a
# failure here means "rebuild the bridge": `scripts/build-bridge.sh`.
#
# Deliberately stricter than the four older bridge-dependent tests (`test_screen_store.gd:510`
# and siblings), which `print("skip: ...")` and continue. A skip would pass on exactly the state
# this test exists to catch, so:
#   - the extension absent as a whole -> skip (the method cannot exist without it),
#   - the extension loaded but the METHOD missing -> FAIL (a stale `.so`).
#
# The request mirrors the Rust `base_request()` fixture in
# `crates/sstd-editor-bridge/src/lib.rs`, so the two halves of the seam are exercised by the same
# inputs.

var _failures := 0

# Each guard appends its name on its final line. A guard that dies partway would otherwise let the
# suite report `failures=0`; this turns a silently-truncated guard into a failure. Same pattern as
# `test_override_merge.gd` (issue #81).
var _completed: Array[String] = []

func check(cond: bool, msg: String) -> void:
    if cond:
        print("ok: " + msg)
    else:
        _failures += 1
        print("FAIL: " + msg)

func _initialize() -> void:
    _run.call_deferred()

# A three-layer request: two `back` layers authored OUT of `draw_order` (so the response's
# ordering is load-bearing) plus one `front` layer (so the plane filter is exercised).
#
# The art widths differ per layer (1500 / 1920 / 640) so `period_px` identifies which art entry a
# placement was built from.
func _parallax_request() -> Dictionary:
    return {
        "view": {
            "cam_center_world_px_x": 960.0,
            "cam_center_world_px_y": 540.0,
            "zoom": 1.0,
            "view_w_px": 1920.0,
            "view_h_px": 1080.0,
        },
        "anchor": {"world_px_x": 960.0, "world_px_y": 528.0},
        "stack": {
            "schema_version": 2,
            "stack_key": "fixture",
            "palette_key": "neutral",
            "generator_seed": 1,
            "ref_view_px": {"width": 1920.0, "height": 1080.0},
            "layers": [
                {
                    "name": "Near", "file": "near.png", "plane": "back", "draw_order": 1,
                    "parallax_depth": {"kind": "finite", "value": 1.0},
                    "vertical_damping": 0.8, "repeat_x": true,
                    "ref_screen_left_px": 0.0, "ref_screen_top_px": 0.0,
                    "fill_above": "transparent", "fill_below": "transparent",
                },
                {
                    "name": "Base", "file": "base.png", "plane": "back", "draw_order": 0,
                    "parallax_depth": {"kind": "finite", "value": 9.0},
                    "vertical_damping": 0.8, "repeat_x": true,
                    "ref_screen_left_px": 0.0, "ref_screen_top_px": 0.0,
                    "fill_above": "#8896AAFF", "fill_below": "#C2CAD0FF",
                },
                {
                    "name": "Fore", "file": "fore.png", "plane": "front", "draw_order": 0,
                    "parallax_depth": {"kind": "finite", "value": -0.25},
                    "vertical_damping": 0.8, "repeat_x": true,
                    "ref_screen_left_px": 0.0, "ref_screen_top_px": 0.0,
                    "fill_above": "transparent", "fill_below": "transparent",
                },
            ],
        },
        "art": [
            {
                "width_px": 1500, "height_px": 1080, "is_rgba": true,
                "fully_opaque": false, "top_row_opaque": true, "bottom_row_opaque": true,
            },
            {
                "width_px": 1920, "height_px": 1080, "is_rgba": true,
                "fully_opaque": true, "top_row_opaque": true, "bottom_row_opaque": true,
            },
            {
                "width_px": 640, "height_px": 1080, "is_rgba": true,
                "fully_opaque": false, "top_row_opaque": true, "bottom_row_opaque": true,
            },
        ],
    }

func _placements_from(bridge: Object, request: Dictionary) -> Dictionary:
    var response_text: String = String(bridge.call("parallax_placements", JSON.stringify(request)))
    var response: Variant = JSON.parse_string(response_text)
    if typeof(response) != TYPE_DICTIONARY:
        return {}
    return response

func _typed_placements(response: Dictionary) -> Array[Dictionary]:
    # JSON gives an untyped Array; convert element by element rather than trusting the shape.
    var typed: Array[Dictionary] = []
    for entry: Variant in (response.get("placements", []) as Array):
        typed.append(entry as Dictionary)
    return typed

func _entry_named(placements: Array[Dictionary], layer_name: String) -> Dictionary:
    for entry: Dictionary in placements:
        if String(entry.get("name", "")) == layer_name:
            return entry
    return {}

func _test_bridge_method_is_callable_and_matches_core() -> void:
    print("--- bridge parallax_placements seam (issue #106) ---")

    if not ClassDB.class_exists("SstdBridge"):
        print("skip: SstdBridge extension not loaded (cannot be built in this environment)")
        _completed.append("callable")
        return

    var bridge_variant: Variant = ClassDB.instantiate("SstdBridge")
    if bridge_variant == null:
        check(false, "SstdBridge class exists but did not instantiate")
        _completed.append("callable")
        return
    var bridge: Object = bridge_variant

    # The real risk this test exists for: the extension loads, but the shared object is stale and
    # does not carry the new method.
    if not bridge.has_method("parallax_placements"):
        check(false, "SstdBridge is loaded but has no parallax_placements (stale .so? run scripts/build-bridge.sh)")
        _completed.append("callable")
        bridge.free()
        return

    var response := _placements_from(bridge, _parallax_request())
    check(not response.has("error"), "valid request answers without an error (got %s)" % str(response))

    var placements := _typed_placements(response)
    check(placements.size() == 3, "all three layers placed (got %d)" % placements.size())

    var names: Array[String] = []
    for entry: Dictionary in placements:
        names.append(String(entry.get("name", "")))
    # Compared untyped: `Array == Array` is element-wise, and `x == y as Array[String]` would
    # parse as `(x == y) as Array[String]` because `as` binds after `==`.
    check(names == ["Base", "Near", "Fore"],
        "sorted back-plane ascending then front (got %s)" % str(names))

    # `period_px` is the art width, so it identifies which art entry each placement came from; a
    # misaligned zip would show another layer's width here.
    check(int(_entry_named(placements, "Base").get("period_px", 0) as float) == 1920,
        "Base period_px is its art width")
    check(int(_entry_named(placements, "Near").get("period_px", 0) as float) == 1500,
        "Near period_px is its art width")
    check(int(_entry_named(placements, "Fore").get("period_px", 0) as float) == 640,
        "Fore period_px is its art width")
    check(String(_entry_named(placements, "Base").get("file", "")) == "base.png",
        "Base carries its own file")
    check(String(_entry_named(placements, "Fore").get("plane", "")) == "front",
        "Fore is on the front plane")

    # `scale` is the view zoom, so it must survive the wire.
    check(is_equal_approx(float(_entry_named(placements, "Base").get("scale", 0.0)), 1.0),
        "scale equals the request zoom")

    # Exact copy counts, derived from the request numbers alone at zoom 1: a copy start lies in
    # [-period, 0), so covering [0, view_w_px) needs ceil(view_w_px / period) or one more. For
    # Base the period equals the view width, which pins it to exactly 2; for Fore (640) to 4.
    check(int(_entry_named(placements, "Base").get("copy_count", 0)) == 2,
        "Base covers a one-period-wide view with 2 copies")
    check(int(_entry_named(placements, "Fore").get("copy_count", 0)) == 4,
        "Fore covers the view with 4 copies")
    var near_copies := int(_entry_named(placements, "Near").get("copy_count", 0))
    check(near_copies == 2 or near_copies == 3,
        "Near needs 2 or 3 copies of a 1500px period across 1920px (got %d)" % near_copies)

    # The plane filter crosses the wire too.
    var front_request := _parallax_request()
    front_request["plane"] = "front"
    var front_response := _placements_from(bridge, front_request)
    var front_placements := _typed_placements(front_response)
    check(front_placements.size() == 1, "plane=front returns only the front layer")
    check(String(front_placements[0].get("name", "")) == "Fore", "plane=front returns Fore")

    # Request-level guards cross the wire on the same error key.
    var zero_zoom_request := _parallax_request()
    (zero_zoom_request["view"] as Dictionary)["zoom"] = 0.0
    var zero_zoom_response := _placements_from(bridge, zero_zoom_request)
    check(String(zero_zoom_response.get("error", "")).contains("zoom"),
        "zoom=0 is rejected, not silently drawn (got %s)" % str(zero_zoom_response))

    var malformed_response := _placements_from(bridge, {"view": "not an object"})
    check(malformed_response.has("error"), "a malformed request returns an error, not a crash")

    _completed.append("callable")
    # Never added to the tree, so `free()` rather than `queue_free()`; without it ObjectDB
    # reports a leaked instance at exit.
    bridge.free()

func _run() -> void:
    _test_bridge_method_is_callable_and_matches_core()

    # Truncation check: without this, a guard that aborted on a runtime error reads as a pass.
    for guard_name: String in ["callable"]:
        check(_completed.has(guard_name), "guard '%s' ran to completion" % guard_name)

    print("=== parallax bridge done, failures=%d ===" % _failures)
    quit(0 if _failures == 0 else 1)
