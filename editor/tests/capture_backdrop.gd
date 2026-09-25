extends SceneTree

# Backdrop capture probe for the Map Editor layer stack (issue #68). Probes the
# STANDALONE res://scenes/map_editor.tscn. The running-app capture path is
# capture_all_tabs.gd, which goes through main.tscn instead: issue #75
# established the two MapEditor scene trees as separate copies, so this probe
# covers the scene file on its own.
#
# Run:
#   godot4 --headless --path editor --script res://tests/capture_backdrop.gd
#
# Output: res://captures/map_editor_backdrop.png

const OUT_DIR := "res://captures"
const FRAMES := 3

func _initialize() -> void:
    _run.call_deferred()

func _run() -> void:
    var scene: PackedScene = load("res://scenes/map_editor.tscn")
    var inst: CanvasItem = scene.instantiate()
    root.add_child(inst)
    for _f in FRAMES:
        await self.process_frame

    DirAccess.make_dir_recursive_absolute(OUT_DIR)
    var path: String = "%s/map_editor_backdrop.png" % OUT_DIR
    var err: Error = root.get_texture().get_image().save_png(path)
    if err != OK:
        print("[capture] FAILED %s (err %d)" % [path, err])
        quit(1)
        return
    print("[capture] saved %s" % path)
    quit()
