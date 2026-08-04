extends Node

func _ready():
    var main_tscn = load("res://scenes/main.tscn")
    var main = main_tscn.instantiate()
    add_child(main)
    var tabs: TabContainer = main.get_node("TabContainer")
    tabs.current_tab = 2
    var map_editor = tabs.get_child(2)
    if map_editor.has_method("_populate_grid"):
        map_editor._populate_grid()
    await get_tree().process_frame
    await get_tree().process_frame
    var img = get_viewport().get_texture().get_image()
    var path := "map_editor_screenshot.png"
    img.save_png(path)
    print("Saved " + path)
    get_tree().quit()