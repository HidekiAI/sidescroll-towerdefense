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
    var bytes: Array = []
    bytes.resize(n)
    var variants: Array = []
    variants.resize(n)
    for i in n:
        var img: Image = ed._as_rgba8(ed._stamp_catalog[i]["cell"])
        bytes[i] = ed._cell_bytes(img, false, false)
        variants[i] = ed._flip_variants(bytes[i])
    var exact_pairs: Array = []
    var max_delta := 256
    var signatures: Array = []
    signatures.resize(n)
    var ordered: Array = []
    for i in n:
        var img: Image = ed._as_rgba8(ed._stamp_catalog[i]["cell"])
        signatures[i] = ed._grayscale_sig(img)
        ordered.append({"index": i, "projection": ed._grayscale_luminance_projection(img)})
    ordered.sort_custom(func(a, b):
        if a["projection"] == b["projection"]:
            return str(ed._stamp_catalog[a["index"]]["key"]) < str(ed._stamp_catalog[b["index"]]["key"])
        return a["projection"] < b["projection"]
    )
    for position in ordered.size():
        var current: Dictionary = ordered[position]
        var target_projection: int = current["projection"] - max_delta
        var window_start: int = ed._projection_lower_bound(ordered, target_projection)
        for previous_position in range(window_start, position):
            var previous: Dictionary = ordered[previous_position]
            if ed._grayscale_sig_near(signatures[previous["index"]], signatures[current["index"]]):
                exact_pairs.append([previous["index"], current["index"]])

    var variants_flat := PackedByteArray()
    var bytes_flat := PackedByteArray()
    for i in bytes.size():
        bytes_flat.append_array(bytes[i])
        for v in variants[i].size():
            variants_flat.append_array(variants[i][v])
    var pair_stream := PackedInt32Array()
    pair_stream.resize(exact_pairs.size() * 2)
    for i in exact_pairs.size():
        pair_stream[i * 2] = int(exact_pairs[i][0])
        pair_stream[i * 2 + 1] = int(exact_pairs[i][1])

    var bridge = ClassDB.instantiate("SstdBridge")
    root.add_child(bridge)
    ed.set_bridge(bridge)
    print("processor_count=%d pairs=%d" % [OS.get_processor_count(), pair_stream.size() / 2])
    for w in [1, 2, 4, 8, 16, 32]:
        var t0 := Time.get_ticks_msec()
        var matches_flat: PackedInt32Array = bridge.scan_exact_matches(
            variants_flat, bytes_flat, pair_stream, ed.STAMP_TOLERANCE, w)
        var dt := Time.get_ticks_msec() - t0
        print("workers=%d: %d ms (%d matches)" % [w, dt, matches_flat.size() / 4])
    print("=== scaling probe done ===")
    quit(0)