extends SceneTree
# Guard for #126 step 7: main.gd must hand each terrain's collision mask to the simulator's
# world view, keyed by terrain key, before the ground profile builder reads it by key.
#
# Pure synchronous wiring on one thread, so a race cannot occur. The transform is applied twice
# to prove it is idempotent; a terrain with no solid_mask must stay absent so the profile builder
# keeps its documented whole-tile fallback.

var _failures: int = 0

class FakeTerrainEditor extends Node:
    var terrain_types: Array[Dictionary] = []

    func get_terrain_types() -> Array[Dictionary]:
        return terrain_types

class FakeWorldView extends Node:
    var received_masks: Dictionary = {}

    func set_terrain_masks(masks: Dictionary) -> void:
        received_masks = masks

func _initialize() -> void:
    _run.call_deferred()

func _check(condition: bool, description: String) -> void:
    if not condition:
        _failures += 1
        printerr("FAIL: " + description)

func _run() -> void:
    var mask_bytes: Array[int] = [255, 128, 0]
    var main_instance: Control = (load("res://scripts/main.gd") as GDScript).new()
    var fake_editor: FakeTerrainEditor = FakeTerrainEditor.new()
    fake_editor.terrain_types = [
        {"key": "slope_up_0", "display_name": "slope up 0", "solid_mask": mask_bytes},
        {"key": "air", "display_name": "Air"},
    ]
    var fake_world_view: FakeWorldView = FakeWorldView.new()
    main_instance.set("terrain_editor", fake_editor)
    main_instance.set("simulator_world_view", fake_world_view)

    # Guard the call so a missing method records a clean assertion failure instead of aborting
    # _run() before quit() -- GDScript kills the function on a nonexistent-method call.
    var can_inject: bool = main_instance.has_method("_inject_terrain_masks")
    _check(can_inject, "main.gd exposes _inject_terrain_masks")
    if can_inject:
        main_instance.call("_inject_terrain_masks")

    var masks: Dictionary = fake_world_view.received_masks
    _check(masks.size() == 1, "only terrains carrying a solid_mask are injected (got %d)" % masks.size())
    _check(masks.has("slope_up_0"), "mask is keyed by terrain key")
    if masks.has("slope_up_0"):
        var bytes: Variant = masks["slope_up_0"]
        _check(bytes is PackedByteArray, "mask value is a PackedByteArray")
        if bytes is PackedByteArray:
            _check(bytes == PackedByteArray(mask_bytes), "mask bytes survive the transform")

    if can_inject:
        main_instance.call("_inject_terrain_masks")
        _check(fake_world_view.received_masks == masks, "repeating the injection yields the same map")

    fake_editor.free()
    fake_world_view.free()
    main_instance.free()
    print("terrain_mask_wiring failures=%d" % _failures)
    quit(0 if _failures == 0 else 1)
