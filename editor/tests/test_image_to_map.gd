extends SceneTree

# Image -> terrain import regression guard.
#
# Validates `_nearest_terrain_key` (pure color distance mapping against the
# terrain palette) and `_import_image_to_tiles` (scales a source PNG to the
# grid, averages each tile cell, writes editable tiles). See issue #76.

var _failures := 0
var _oks := 0

func check(cond: bool, msg: String) -> void:
    if cond:
        _oks += 1
        print("ok: " + msg)
    else:
        _failures += 1
        print("FAIL: " + msg)

func _initialize() -> void:
    _run.call_deferred()

func _run() -> void:
    var scene := load("res://scenes/map_editor.tscn") as PackedScene
    var ed := scene.instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame

    check(ed._nearest_terrain_key(Color("#4a7c3f")) == "grass",
        "exact grass color -> grass")
    check(ed._nearest_terrain_key(Color("#3a7bd5")) == "water",
        "exact water color -> water")
    check(ed._nearest_terrain_key(Color("#21f7ff")) == "",
        "sky-ish far color -> empty (clears to air)")
    var near_green := Color("#3e6b34")
    check(ed._nearest_terrain_key(near_green) == "grass",
        "near-grass shade -> grass (got %s)" % ed._nearest_terrain_key(near_green))

    var png_path := ProjectSettings.globalize_path("res://../editor/tests/tmp_imgmap.png")
    var img := Image.create(64, 64, false, Image.FORMAT_RGBA8)
    img.fill(Color("#4a7c3f"))
    for sy in 64:
        for sx in 32:
            img.set_pixel(sx, sy, Color("#87ceeb"))
    img.save_png(png_path)

    ed._import_image_to_tiles(png_path)

    check(ed.get_tile(0, 0) == "air", "left half sky -> air (got %s)" % ed.get_tile(0, 0))
    check(ed.get_tile(8, 8) == "air", "left half sky cell -> air (got %s)" % ed.get_tile(8, 8))
    check(ed.get_tile(31, 15) == "grass", "right half grass cell -> grass (got %s)" % ed.get_tile(31, 15))
    check(ed.get_tile(40, 20) == "grass", "right half grass cell -> grass (got %s)" % ed.get_tile(40, 20))
    check(ed.get_tile(59, 32) == "grass", "bottom-right grass cell -> grass (got %s)" % ed.get_tile(59, 32))

    DirAccess.remove_absolute(png_path)

    var failures := _failures
    print("test_image_to_map : %s (%d assertions)" % [
        "PASS (failures=0)" if failures == 0 else "FAIL (failures=%d)" % failures,
        _oks + failures,
    ])
    quit(0 if failures == 0 else 1)