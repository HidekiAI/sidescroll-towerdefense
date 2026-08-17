extends SceneTree

# Issue #53 profile: per-stage timing of the native prune scan + a3 + finalize
# (release .so). The "prep" step now goes through the native front-end
# (scan_signatures / scan_projections) over the canonical bytes_flat.

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

    var t := Time.get_ticks_msec()
    var bytes_flat := PackedByteArray()
    for i in n:
        bytes_flat.append_array(ed._cell_bytes(ed._as_rgba8(ed._stamp_catalog[i]["cell"]), false, false))
    print("bytes_flat (_cell_bytes loop): %d ms" % (Time.get_ticks_msec() - t))

    t = Time.get_ticks_msec()
    var sigs_flat: PackedByteArray = bridge.scan_signatures(bytes_flat)
    var projections_flat: PackedInt32Array = bridge.scan_projections(bytes_flat)
    print("native sigs+projections: %d ms" % (Time.get_ticks_msec() - t))

    var ordered: Array = []
    for i in range(n):
        ordered.append({"index": i, "projection": projections_flat[i]})
    t = Time.get_ticks_msec()
    ordered.sort_custom(func(a, b):
        if a["projection"] == b["projection"]:
            return str(ed._stamp_catalog[a["index"]]["key"]) < str(ed._stamp_catalog[b["index"]]["key"])
        return a["projection"] < b["projection"]
    )
    print("sort_custom: %d ms" % (Time.get_ticks_msec() - t))

    t = Time.get_ticks_msec()
    var ordered_projections := PackedInt32Array()
    var ordered_indices := PackedInt32Array()
    ordered_projections.resize(ordered.size())
    ordered_indices.resize(ordered.size())
    for position in ordered.size():
        ordered_projections[position] = int(ordered[position]["projection"])
        ordered_indices[position] = int(ordered[position]["index"])
    print("flatten arrays: %d ms" % (Time.get_ticks_msec() - t))

    var workers := mini(OS.get_processor_count(), ed.BRIDGE_MAX_WORKERS)
    t = Time.get_ticks_msec()
    var pair_stream: PackedInt32Array = bridge.scan_gate_pairs(
        sigs_flat, ordered_projections, ordered_indices, 256, workers)
    var pair_span := Time.get_ticks_msec() - t
    print("scan_gate_pairs: %d ms, pairs=%d" % [pair_span, pair_stream.size() / 2])

    t = Time.get_ticks_msec()
    var variants_flat: PackedByteArray = bridge.build_variants(bytes_flat, workers)
    print("build_variants: %d ms" % (Time.get_ticks_msec() - t))

    t = Time.get_ticks_msec()
    var exact_matches: Array = ed._exact_matches_bridge_stream(pair_stream, variants_flat, bytes_flat)
    print("exact_matches_bridge_stream: %d ms, matches=%d" % [Time.get_ticks_msec() - t, exact_matches.size()])

    var uf_parent: Array = []
    for i in n:
        uf_parent.append(i)
    for match in exact_matches:
        ed._uf_union(uf_parent, int(match["base"]), int(match["candidate"]))

    t = Time.get_ticks_msec()
    var a3: Array = ed._a3_discover(uf_parent)
    print("a3_discover: %d ms, matches=%d" % [Time.get_ticks_msec() - t, a3.size()])
    t = Time.get_ticks_msec()
    for match in a3:
        ed._uf_union(uf_parent, int(match["base"]), int(match["candidate"]))
    var plan: Dictionary = ed._finalize_prune_plan(uf_parent)
    print("finalize: %d ms, dup_keys=%d" % [Time.get_ticks_msec() - t, plan["dup_keys"].size()])

    t = Time.get_ticks_msec()
    var plan2: Dictionary = ed._build_prune_plan()
    print("full build_prune_plan: %d ms, dup_keys=%d" % [Time.get_ticks_msec() - t, plan2["dup_keys"].size()])
    print("=== breakdown done ===")
    quit(0)