class_name ParallaxLoader extends RefCounted
# ParallaxLoader reads one backdrop stack from disk and returns placements from the bridge.
#
# It is the only component that reads `stack.json`, because `sstd-core::parallax::Stack::validate`
# cannot: that module "has no filename and no filesystem". Everything downstream (the drawer, the
# simulator) consumes the placements this returns and never opens the stack itself.
#
# Wire contract: `SstdBridge.parallax_placements`, `crates/sstd-editor-bridge/src/lib.rs`.
# Design of record: PLAN-2026-10-16-parallax-step4.md section 4.2;
# TDD_Parallax-Scrolling.Math.md sections 5 and 7.
#
# Every step emits one `parallax.load` journal line carrying its inputs and outcome, so a broken
# backdrop can be re-traced from the output log alone.
#
# `load_stack()` result shape — the only public contract:
#   ok             bool     placements landed
#   placements     Array    bridge placements, back plane then front, ascending draw_order
#   status_text    String   what the status HUD shows (section 4.7)
#   stack_key      String   the stack actually used
#   stack_path     String   the stack FILE actually used, so `entry.file` resolves (it differs
#                           from the requested path after a fallback)
#   used_fallback  bool     the requested stack was rejected and the default took over
#   error          String   present only when ok == false, i.e. both stacks failed

# Where a stack with no declaration of its own loads from. This is also the fallback stack, so a
# rejected custom stack still renders a backdrop instead of nothing.
const DEFAULT_STACK_PATH: String = "res://assets/backdrop_stacks/neutral_default/stack.json"
# The wire token `{"error": "stack_invalid"}`. Named here because a rename on the Rust side would
# otherwise turn the fallback into a silent no-op: the loader would read an unknown tag, report a
# plain failure and never reach the `journal` array it is supposed to log.
const ERROR_STACK_INVALID: String = "stack_invalid"
# The stack that takes over when the requested one is rejected, and the wire token the status HUD
# names. It is the same file as `DEFAULT_STACK_PATH`; kept separate because the two travel to
# different places (one is a path, one is HUD text).
const FALLBACK_STACK_KEY: String = "neutral_default"
# `Image.get_pixel(...).a8` is the integer alpha, 0..255; a row is opaque at full alpha only.
const FULL_ALPHA: int = 255

var _bridge: Object = null


# Test seam: hand the loader a stand-in for the GDExtension so a rejected stack can be provoked
# without shipping a broken one. Production never calls this — `_resolve_bridge()` finds the real
# `SstdBridge` in `ClassDB`.
func set_bridge(bridge: Object) -> void:
    _bridge = bridge


# Load `stack_path` and return placements, falling back to `DEFAULT_STACK_PATH` when the requested
# stack is rejected. Never asserts: a rejected stack is a logged fallback, not a crash (G15).
#
# `anchor` is required. `sstd-core` holds `default_anchor`, the value a world with no anchor
# declaration should pass (the home view centre), and the caller supplies it rather than this file
# re-deriving it — a second computation of the same value is a second source of truth for exactly
# the branch `TDD_Parallax-Scrolling.Math` section 5 says gets written wrong.
func load_stack(stack_path: String, view: Dictionary, anchor: Dictionary) -> Dictionary:
    var requested_key: String = _stack_key_of(stack_path)
    print("parallax.load BEGIN path=%s stack=%s anchor=%s view=%s" % [
        stack_path, requested_key, JSON.stringify(anchor), JSON.stringify(view)])

    var primary: Dictionary = _load_one(stack_path, view, anchor)
    if primary.get("ok", false):
        print("parallax.load OK path=%s stack=%s placements=%d" % [
            stack_path, String(primary.get("stack_key", "")), (primary.get("placements", []) as Array).size()])
        return _result(primary, stack_path, String(primary.get("stack_key", "")), false, "")

    # A rejected stack: log the rule lines the bridge already formatted, then exactly one
    # FALLBACK line, then try the default. Reaching here twice means both stacks are broken.
    for journal_line in (primary.get("journal", []) as Array):
        print(String(journal_line))
    var failure_detail: String = String(primary.get("error", "unknown"))
    print("parallax.load FALLBACK from=%s stack=%s reason=%s" % [stack_path, FALLBACK_STACK_KEY, failure_detail])

    if stack_path == DEFAULT_STACK_PATH:
        print("parallax.load FAILED path=%s reason=%s (fallback IS the requested stack)" % [
            stack_path, failure_detail])
        return {
            "ok": false,
            "placements": [],
            "status_text": "%s unavailable" % FALLBACK_STACK_KEY,
            "stack_key": "",
            "used_fallback": false,
            "stack_path": "",
            "error": failure_detail,
        }

    var fallback: Dictionary = _load_one(DEFAULT_STACK_PATH, view, anchor)
    if not fallback.get("ok", false):
        for journal_line in (fallback.get("journal", []) as Array):
            print(String(journal_line))
        print("parallax.load FAILED path=%s reason=%s" % [
            DEFAULT_STACK_PATH, String(fallback.get("error", "unknown"))])
        return {
            "ok": false,
            "placements": [],
            "status_text": "%s unavailable" % FALLBACK_STACK_KEY,
            "stack_key": "",
            "used_fallback": true,
            "error": String(fallback.get("error", "unknown")),
        }

    # Section 4.7: the HUD names the stack that rendered AND the rule that rejected the first one,
    # because the fallback succeeding does not make the rejected stack's rule less real.
    var status_text: String = "%s (fallback from %s: %s)" % [
        String(fallback.get("stack_key", "")), requested_key, failure_detail]
    print("parallax.load OK path=%s stack=%s placements=%d status=%s" % [
        DEFAULT_STACK_PATH, String(fallback.get("stack_key", "")),
        (fallback.get("placements", []) as Array).size(), status_text])
    return _result(fallback, DEFAULT_STACK_PATH, String(fallback.get("stack_key", "")), true, status_text)


# Read, parse, measure and query one stack. Returns
# `{ ok, placements, stack_key, error, journal }`; `journal` holds the bridge's pre-formatted
# `parallax.validate FAIL` lines, which the caller logs verbatim.
func _load_one(stack_path: String, view: Dictionary, anchor: Dictionary) -> Dictionary:
    var read: Dictionary = read_stack(stack_path)
    if not read.get("ok", false):
        return {"ok": false, "placements": [], "stack_key": "", "error": String(read.get("error", "")), "journal": []}
    var stack := read.get("stack", {}) as Dictionary
    var stack_key: String = String(stack.get("stack_key", ""))

    var measured: Dictionary = measure_art(stack_path.get_base_dir(), stack)
    if not measured.get("ok", false):
        return {"ok": false, "placements": [], "stack_key": stack_key, "error": String(measured.get("error", "")), "journal": []}

    var queried: Dictionary = query_placements(build_request(stack, view, measured.get("art", []) as Array, anchor))
    if not queried.get("ok", false):
        return {
            "ok": false,
            "placements": [],
            "stack_key": stack_key,
            "error": String(queried.get("error", "")),
            "journal": queried.get("journal", []) as Array,
        }
    return {
        "ok": true,
        "placements": queried.get("placements", []) as Array,
        "stack_key": stack_key,
        "error": "",
        "journal": [],
    }


# Read and parse one `stack.json`. The parsed object is kept, not the text: the request's `stack`
# key takes the object, so parsing once here is the whole read.
func read_stack(stack_path: String) -> Dictionary:
    if not FileAccess.file_exists(stack_path):
        return {"ok": false, "error": "no such stack file: %s" % stack_path}
    var file := FileAccess.open(stack_path, FileAccess.READ)
    if file == null:
        return {"ok": false, "error": "cannot open %s (FileAccess error %d)" % [
            stack_path, FileAccess.get_open_error()]}
    var parsed: Variant = JSON.parse_string(file.get_as_text())
    if typeof(parsed) != TYPE_DICTIONARY:
        return {"ok": false, "error": "not a JSON object: %s" % stack_path}
    var stack := parsed as Dictionary
    if typeof(stack.get("layers", null)) != TYPE_ARRAY:
        return {"ok": false, "error": "stack has no layers array: %s" % stack_path}
    return {"ok": true, "stack": stack}


# Measure every layer PNG into the `art` array the bridge expects, index-aligned with
# `stack.layers`. The bridge documents these as "the PNG decoder the caller already has", so
# `sstd-core` stays free of an image dependency.
func measure_art(assets_dir: String, stack: Dictionary) -> Dictionary:
    var layers := stack.get("layers", []) as Array
    var art: Array = []
    for layer_index in range(layers.size()):
        var layer := layers[layer_index] as Dictionary
        var png_path: String = assets_dir.path_join(String(layer.get("file", "")))
        var measured: Dictionary = measure_png(png_path)
        if not measured.get("ok", false):
            return {"ok": false, "error": measured.get("error", "")}
        art.append(measured.get("facts", {}))
    return {"ok": true, "art": art}


# The six facts `LayerArt` carries. `detect_alpha()` is a C++ scan, so a 1920 x ~1080 layer costs
# one call rather than two million pixel reads; only the two edge rows are walked in GDScript.
func measure_png(png_path: String) -> Dictionary:
    if not FileAccess.file_exists(png_path):
        return {"ok": false, "error": "no such layer PNG: %s" % png_path}
    var image := _decode_png(png_path)
    if image == null:
        return {"ok": false, "error": "decoder rejected %s" % png_path}
    var width_px: int = image.get_width()
    var height_px: int = image.get_height()
    if width_px <= 0 or height_px <= 0:
        return {"ok": false, "error": "zero-sized layer PNG: %s" % png_path}
    return {
        "ok": true,
        "facts": {
            "width_px": width_px,
            "height_px": height_px,
            "is_rgba": image.get_format() == Image.FORMAT_RGBA8,
            # `Image.AlphaMode.ALPHA_NONE` means the PNG carries no transparency at all, which is
            # what "fully opaque" means on the wire.
            "fully_opaque": image.detect_alpha() == Image.ALPHA_NONE,
            "top_row_opaque": _row_is_opaque(image, 0, width_px),
            "bottom_row_opaque": _row_is_opaque(image, height_px - 1, width_px),
        },
    }


# Decode a layer PNG to pixels. Two paths because `res://` files are IMPORTED: the resource path
# loads as a `Texture2D` (which is what exports keep), and `Image.load_from_file` on a `res://`
# path logs "will not work on export" and fails in a packed build. The direct decoder stays for
# paths outside the import pipeline (a user:// or absolute stack).
func _decode_png(png_path: String) -> Image:
    if png_path.begins_with("res://"):
        var texture: Variant = load(png_path)
        if texture is Texture2D:
            return (texture as Texture2D).get_image()
        return null
    return Image.load_from_file(png_path)


# One row of `width_px` pixels, all at full alpha.
func _row_is_opaque(image: Image, row: int, width_px: int) -> bool:
    for col in range(width_px):
        if image.get_pixel(col, row).a8 != FULL_ALPHA:
            return false
    return true


# The request object, spelled exactly as `PlacementsRequest` deserializes it. A key renamed on the
# Rust side fails loudly here as `missing field`, which is why these keys are written out rather
# than named as constants: a wrong structural key is an immediate error, not a silent misread.
func build_request(stack: Dictionary, view: Dictionary, art: Array, anchor: Dictionary) -> Dictionary:
    return {"view": view, "anchor": anchor, "stack": stack, "art": art}


# Ask the bridge for placements. `{"error": "stack_invalid"}` is the rejection that carries the
# formatted rule lines and the message list; a malformed-request `{"error": "<detail>"}` carries
# neither, so its journal stays empty. The rule CODE becomes the caller's detail -- that is what
# the status HUD names (section 4.7) -- while the journal lines are what gets logged verbatim.
func query_placements(request: Dictionary) -> Dictionary:
    var bridge := _resolve_bridge()
    if bridge == null or not bridge.has_method("parallax_placements"):
        return {"ok": false, "error": "bridge unavailable: SstdBridge.parallax_placements", "journal": []}
    var response_text := String(bridge.call("parallax_placements", JSON.stringify(request)))
    var response: Variant = JSON.parse_string(response_text)
    if typeof(response) != TYPE_DICTIONARY:
        return {"ok": false, "error": "bridge returned non-object JSON: %s" % response_text.left(120), "journal": []}
    var response_object := response as Dictionary
    if not response_object.has("error"):
        if typeof(response_object.get("placements", null)) != TYPE_ARRAY:
            return {"ok": false, "error": "bridge response has no placements array", "journal": []}
        return {"ok": true, "placements": response_object.get("placements", []) as Array, "journal": []}
    var error_tag: String = String(response_object.get("error", ""))
    if error_tag != ERROR_STACK_INVALID:
        return {"ok": false, "error": error_tag, "journal": []}
    var journal: Array = response_object.get("journal", []) as Array
    var errors: Array = response_object.get("errors", []) as Array
    var first_rule: String = error_tag
    if not errors.is_empty() and errors[0] is Dictionary:
        first_rule = String((errors[0] as Dictionary).get("code", error_tag))
    return {"ok": false, "error": first_rule, "journal": journal}


func _resolve_bridge() -> Object:
    if _bridge != null:
        return _bridge
    if not ClassDB.class_exists("SstdBridge"):
        return null
    var bridge_instance: Variant = ClassDB.instantiate("SstdBridge")
    if bridge_instance == null:
        return null
    _bridge = bridge_instance as Object
    return _bridge


func _stack_key_of(stack_path: String) -> String:
    var file_name: String = stack_path.get_file()
    if file_name.get_basename() == "stack":
        return stack_path.get_base_dir().get_file()
    return file_name.get_basename()


# `stack_path` is the file the placements actually came from, which is NOT the requested one
# after a fallback: `entry.file` is a bare filename, so the caller has to know which directory it
# resolves against.
func _result(primary: Dictionary, stack_path: String, stack_key: String, used_fallback: bool, status_text: String) -> Dictionary:
    return {
        "ok": true,
        "placements": primary.get("placements", []) as Array,
        "status_text": status_text if used_fallback else stack_key,
        "stack_key": stack_key,
        "stack_path": stack_path,
        "used_fallback": used_fallback,
        "error": "",
    }
