extends SceneTree

const N := 2700

func check(cond: bool, msg: String) -> void:
    print(("ok: " if cond else "FAIL: ") + msg)

func _initialize() -> void:
    var ed = (load("res://scenes/map_editor.tscn") as PackedScene).instantiate()
    root.add_child(ed)
    while not ed._ready_done:
        await process_frame
    ed._stamp_catalog.clear()
    ed._tile_images.clear()
    ed._rebuild_stamp_index()
    # Mimic the real catalog: many near-black tiles (dense candidate region)
    # plus a scattering of distinct colors.
    for i in N:
        var img := Image.create(32, 32, false, Image.FORMAT_RGBA8)
        if i < N * 4 / 5:
            img.fill(Color(0.03 + (i % 8) * 0.004, 0.03 + (i % 5) * 0.004, 0.03, 1.0))
        else:
            img.fill(Color(0.4 + (i % 7) * 0.08, 0.2, 0.2, 1.0))
        ed._stamp_catalog.append({"key": "stamp_8000%d" % i, "cell": img, "flip_h": false, "flip_v": false})
        ed._tile_images["stamp_8000%d" % i] = img
    ed._rebuild_stamp_index()

    # Phase A: per-tile bytes + sig + projection + sort.
    var t0 := Time.get_ticks_msec()
    var ordered: Array = []
    var bytes: Array = []
    var signatures: Array = []
    bytes.resize(N)
    signatures.resize(N)
    for i in N:
        var img: Image = ed._as_rgba8(ed._stamp_catalog[i]["cell"])
        bytes[i] = ed._cell_bytes(img, false, false)
        signatures[i] = ed._grayscale_sig(img)
        ordered.append({"index": i, "projection": ed._grayscale_luminance_projection(img)})
    ordered.sort_custom(func(a, b):
        if a["projection"] == b["projection"]:
            return str(ed._stamp_catalog[a["index"]]["key"]) < str(ed._stamp_catalog[b["index"]]["key"])
        return a["projection"] < b["projection"]
    )
    var t_phase_a := Time.get_ticks_msec() - t0
    print("phase A (bytes+sig+projection+sort): %d ms" % t_phase_a)

    # Phase B: flip variants per tile.
    t0 = Time.get_ticks_msec()
    var variants: Array = []
    variants.resize(N)
    for i in N:
        variants[i] = ed._flip_variants(bytes[i])
    var t_phase_b := Time.get_ticks_msec() - t0
    print("phase B (flip variants): %d ms" % t_phase_b)

    # Phase C: window scan + grayscale gate.
    t0 = Time.get_ticks_msec()
    var max_delta := 256
    var candidate_count := 0
    var gate_survivors := 0
    var exact_pairs: Array = []
    for position in ordered.size():
        var current: Dictionary = ordered[position]
        var target_projection: int = current["projection"] - max_delta
        var window_start: int = ed._projection_lower_bound(ordered, target_projection)
        for previous_position in range(window_start, position):
            var previous: Dictionary = ordered[previous_position]
            candidate_count += 1
            if not ed._grayscale_sig_near(signatures[previous["index"]], signatures[current["index"]]):
                continue
            gate_survivors += 1
            exact_pairs.append([previous["index"], current["index"]])
    var t_phase_c := Time.get_ticks_msec() - t0
    print("phase C (window+gate): %d ms, candidates=%d survivors=%d" % [t_phase_c, candidate_count, gate_survivors])

    # Phase D: exact diff, bridge (native). Serial path intentionally skipped —
    # it is the known ~131x-slower reference and would dominate the wall time.
    if ClassDB.class_exists("SstdBridge"):
        var bridge = ClassDB.instantiate("SstdBridge")
        root.add_child(bridge)
        ed.set_bridge(bridge)
        t0 = Time.get_ticks_msec()
        var bridged: Array = ed._exact_matches_bridge(exact_pairs, variants, bytes)
        var t_bridge := Time.get_ticks_msec() - t0
        print("phase D bridge exact: %d ms (%d matches, %d pairs)" % [t_bridge, bridged.size(), exact_pairs.size()])
        print("totals (A+B+C+D bridge): %d ms" % (t_phase_a + t_phase_b + t_phase_c + t_bridge))
    else:
        print("no bridge extension")
    ed.queue_free()
    await process_frame
    print("=== profile done ===")
    quit(0)