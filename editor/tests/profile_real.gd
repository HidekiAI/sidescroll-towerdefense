extends SceneTree

func _initialize() -> void:
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    ed._stamp_catalog.clear()
    ed._tile_images.clear()
    ed._rebuild_stamp_index()
    await ed._load_stamp_catalog()
    var n: int = ed._stamp_catalog.size()
    var bridge = ClassDB.instantiate("SstdBridge")
    root.add_child(bridge)
    ed.set_bridge(bridge)
    print("real catalog: %d tiles" % n)

    var t0 := Time.get_ticks_msec()
    var scan: Dictionary = ed._grayscale_projection_candidates(256)
    print("scan: %d ms" % (Time.get_ticks_msec() - t0))

    var uf_parent: Array = []
    for i in n:
        uf_parent.append(i)
    for match in scan["exact_matches"]:
        ed._uf_union(uf_parent, int(match["base"]), int(match["candidate"]))
    t0 = Time.get_ticks_msec()
    var a3: Array = ed._a3_discover(uf_parent)
    print("a3: %d ms, matches=%d" % [Time.get_ticks_msec() - t0, a3.size()])
    for match in a3:
        ed._uf_union(uf_parent, int(match["base"]), int(match["candidate"]))
    t0 = Time.get_ticks_msec()
    var plan: Dictionary = ed._finalize_prune_plan(uf_parent)
    print("finalize: %d ms, dup_keys=%d" % [Time.get_ticks_msec() - t0, plan["dup_keys"].size()])
    t0 = Time.get_ticks_msec()
    var plan2: Dictionary = ed._build_prune_plan()
    print("full build_prune_plan: %d ms, dup_keys=%d" % [Time.get_ticks_msec() - t0, plan2["dup_keys"].size()])
    print("=== breakdown done ===")
    quit(0)