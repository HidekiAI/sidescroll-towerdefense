extends SceneTree

# Phase 1 of #64: headless all-tab capture. Instantiates the full editor scene
# tree, switches through every tab, and saves a viewport screenshot for each.
# No gRPC — a one-shot script for CI snapshot tests.
#
# Run:
#   godot4 --headless --path editor --script res://capture_all_tabs.gd
#
# Output: res://captures/{tile,entity,map,placement,simulator}.png

const OUT_DIR := "res://captures"
const FRAMES_PER_TAB := 2

func _initialize() -> void:
    _run.call_deferred()

func _run() -> void:
    var main_tscn: PackedScene = load("res://scenes/main.tscn")
    var main: Node = main_tscn.instantiate()
    root.add_child(main)
    var tabs: TabContainer = main.get_node("TabContainer")
    var tab_names: Array[String] = main.TAB_NAMES

    DirAccess.make_dir_recursive_absolute(OUT_DIR)
    var saved: int = 0
    for i in tabs.get_child_count():
        tabs.current_tab = i
        for _f in FRAMES_PER_TAB:
            await self.process_frame
        var img: Image = root.get_texture().get_image()
        var tab_name: String = tab_names[i] if i < tab_names.size() else tabs.get_child(i).name
        var path: String = "%s/%s.png" % [OUT_DIR, String(tab_name).to_lower()]
        img.save_png(path)
        print("[capture] saved %s (tab %d)" % [path, i])
        saved += 1

    print("[capture] done, %d tabs captured to %s" % [saved, OUT_DIR])
    quit()
