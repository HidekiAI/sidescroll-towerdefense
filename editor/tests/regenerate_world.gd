extends SceneTree

# Regenerate the world archive so every integer-typed field (notably
# sub_tile_mask) is persisted as an integer rather than a float Variant.
#
# Godot's JSON.stringify writes float Variants as 15.0 (to a Rust u8 field that
# serde used to reject). The producer fix in map_editor::_serialize() and
# terrain_editor::collect_terrain_overrides() coerces sub_tile_mask to int; this
# script instantiates the editor's real main scene, waits for the deferred world
# load, then calls map_editor._save_world() so the on-disk archive is
# regenerated cleanly (see docs/PLAN-2026-08-27-terrain-brush-and-subtile-float.md).
#
# Usage (headless, no display):
#   ~/bin/godot4 --headless --path editor --script res://tests/regenerate_world.gd

func _initialize() -> void:
    _go.call_deferred()

func _go() -> void:
    var main_scene := load("res://scenes/main.tscn")
    if main_scene == null:
        print("FAIL: main.tscn not found")
        quit(2)
        return
    root.add_child(main_scene.instantiate())
    call_deferred("_wait_for_world")

# Give the boot a few frames via process callbacks (map load is coroutine-driven).
func _process(_delta: float) -> bool:
    return false

func _wait_for_world() -> void:
    for _i in range(120):
        await process_frame
    _save()

func _save() -> void:
    var main := root.get_node_or_null("Main")
    if main == null:
        print("FAIL: Main node not found")
        quit(2)
        return
    var map_editor := main.get_node_or_null("TabContainer/MapEditor")
    if map_editor == null:
        print("FAIL: MapEditor node not found at TabContainer/MapEditor")
        quit(2)
        return
    if not map_editor.has_method("_save_world"):
        print("FAIL: MapEditor has no _save_world")
        quit(2)
        return
    map_editor._save_world()
    print("REGENERATE: _save_world() called; archive rewritten with integer masks")
    quit(0)
