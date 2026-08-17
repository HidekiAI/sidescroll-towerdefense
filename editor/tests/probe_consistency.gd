extends SceneTree

func _initialize() -> void:
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    ed._stamp_catalog.clear()
    ed._tile_images.clear()
    ed._rebuild_stamp_index()

    # Synthetic catalog: clusters of near-identical tiles + flips + distinct.
    var rng := RandomNumberGenerator.new()
    rng.seed = 42
    for i in 120:
        var shade := rng.randf_range(0.03, 0.9)
        var img := Image.create(32, 32, false, Image.FORMAT_RGBA8)
        img.fill(Color(shade, shade, shade, 1.0))
        var key := "stamp_9%03d" % i
        ed._stamp_catalog.append({"key": key, "cell": img})
        ed._tile_images[key] = img
        for d in 4:
            var dup := Image.create(32, 32, false, Image.FORMAT_RGBA8)
            dup.fill(Color(clampf(shade + 0.001 * (d + 1), 0.0, 1.0), clampf(shade + 0.001 * (d + 1), 0.0, 1.0), clampf(shade + 0.001 * (d + 1), 0.0, 1.0), 1.0))
            var dkey := "stamp_9%03d" % (i * 10 + d)
            ed._stamp_catalog.append({"key": dkey, "cell": dup})
            ed._tile_images[dkey] = dup
    ed._rebuild_stamp_index()
    print("synthetic catalog: %d tiles" % ed._stamp_catalog.size())

    # No-bridge serial reference.
    var serial: Dictionary = ed._grayscale_projection_candidates(256)
    print("serial: exact=%d candidates=%d" % [serial["exact_matches"].size(), serial["candidate_count"]])

    var bridge = ClassDB.instantiate("SstdBridge")
    root.add_child(bridge)
    ed.set_bridge(bridge)
    var bridged: Dictionary = ed._grayscale_projection_candidates(256)
    print("bridge: exact=%d candidates=%d" % [bridged["exact_matches"].size(), bridged["candidate_count"]])

    check(bridged["exact_matches"].size() == serial["exact_matches"].size(),
        "exact match count equal (%d vs %d)" % [bridged["exact_matches"].size(), serial["exact_matches"].size()])
    var ok := true
    for i in serial["exact_matches"].size():
        var s: Dictionary = serial["exact_matches"][i]
        var b: Dictionary = bridged["exact_matches"][i]
        if s["base"] != b["base"] or s["candidate"] != b["candidate"] or s["flip_h"] != b["flip_h"] or s["flip_v"] != b["flip_v"]:
            ok = false
            print("MISMATCH at %d: serial %s vs bridge %s" % [i, s, b])
            break
    check(ok, "exact match stream identical (incl flips)")

    # Full prune plan end-to-end (with bridge).
    var t0 := Time.get_ticks_msec()
    var plan: Dictionary = ed._build_prune_plan()
    var dt := Time.get_ticks_msec() - t0
    print("build_prune_plan (bridge): %d ms, plan groups=%d" % [dt, plan.size()])
    check(plan.size() > 0, "prune plan produced")
    print("=== consistency probe done ===")
    quit(0)

func check(cond: bool, msg: String) -> void:
    print(("ok: " if cond else "FAIL: ") + msg)