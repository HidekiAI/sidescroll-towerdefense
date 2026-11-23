extends SceneTree
# Issue #122 -- the walker stepper behind the Simulator avatar: ground table, gravity, landing,
# hole fall-through, and the world-edge clamp.
#
# Pure functions only. Nothing here proves anything is DRAWN; that needs a real window.
#
# The three numbers that matter are the ones the flat prototype world actually uses:
# 32 px tiles, ground at tile row 31 (so 992 px from the top of the world), and a world that is
# three screens wide (3 x 60 tiles x 32 px = 5760 px).
#
# Guards run in the repo's usual style: each appends its name to `_completed`, the runner fails
# on a missing name, and the exit code gates -- never the printed counter.

const TILE_W_PX: float = 32.0
const TILE_H_PX: float = 32.0
const WORLD_LEFT_PX: float = 0.0
const WORLD_RIGHT_PX: float = 5760.0
const GROUND_TOP_PX: float = 992.0
const STEP_SECONDS: float = 1.0 / 60.0

# 4:7 slope run geometry: the seven tiles main.gd injects on screen 2, at world columns 60-66.
# The TILE row for local column col is GROUND_ROW - int(4 * col / 7); the SURFACE is the topmost
# solid pixel of that tile's PNG art, which this suite decodes independently as its oracle.
const GROUND_ROW: int = 31
const SLOPE_COLS: int = 7
const SLOPE_START_COL: int = 60  # screen 1 starts at world column 60
const SLOPE_PNG_GREEN_THRESHOLD: float = 0.5  # shipped slope art: ground green 0.702, air 0.0
# Measured from the art by hand, independent of the PNG decoder: surface Y at each tile's left
# seam and at its centre. A bug shared by the decoder and the mask builder cannot hide behind these.
const SLOPE_SEAM_Y_PX: Array[int] = [992, 1005, 987, 968, 950, 932, 914]
const SLOPE_CENTRE_Y_PX: Array[int] = [992, 996, 978, 960, 941, 928, 904]

# Spending longer than this on a pure-function loop means the step is not converging.
const SETTLE_STEPS: int = 180
const TRAVEL_STEPS: int = 600

const GUARDS: Array[String] = [
    "ground_table",
    "falls_then_lands",
    "stays_on_ground",
    "falls_through_void",
    "clamps_left",
    "clamps_right",
    "blocked_by_wall",
    "deterministic",
    "hill_climb",
    "hill_fall_damage",
    "slope_feet_y_matches_drawn_surface",
    "solid_mask_surface_rule",
    "interior_air_below_surface_is_solid",
]

var _failures: int = 0
var _completed: Array[String] = []


func check(condition: bool, message: String) -> void:
    if condition:
        print("ok: " + message)
    else:
        _failures += 1
        printerr("FAIL: " + message)


func _initialize() -> void:
    _run.call_deferred()


func _run() -> void:
    await _test_walker_step()
    _assert_all_guards_completed()
    print("=== test_walker_step: %d failures ===" % _failures)
    quit(0 if _failures == 0 else 1)


func _assert_all_guards_completed() -> void:
    for guard_name in GUARDS:
        if not _completed.has(guard_name):
            _failures += 1
            printerr("FAIL: guard '%s' did not complete" % guard_name)


func _walker(pos_x: float, pos_y: float, grounded: bool) -> Dictionary:
    return {
        "pos_x": pos_x,
        "pos_y": pos_y,
        "vel_x": 0.0,
        "vel_y": 0.0,
        "grounded": grounded,
    }


func _profile_from_tiles(tiles: Array, tile_w_px: float = TILE_W_PX,
        tile_h_px: float = TILE_H_PX) -> Dictionary:
    var result: Array = WorldView.build_ground_profile(tiles, tile_w_px, tile_h_px)
    return {"profile": result[0], "world_left_px": float(result[1]), "world_right_px": float(result[2])}


func _step(state: Dictionary, move_x: float, profile_ctx: Dictionary) -> Dictionary:
    return WorldView.step_walker(
        state,
        move_x,
        STEP_SECONDS,
        profile_ctx.get("profile", []),
        float(profile_ctx.get("world_left_px", WORLD_LEFT_PX)),
        float(profile_ctx.get("world_right_px", WORLD_RIGHT_PX)))


func _step_many(state: Dictionary, move_x: float, profile_ctx: Dictionary,
        step_count: int) -> Dictionary:
    var settled: Dictionary = state
    for step_index in range(step_count):
        settled = _step(settled, move_x, profile_ctx)
    return settled


# Walker with HP and fall tracking, so the hill tests can assert on the new keys.
func _walker_hp(pos_x: float, pos_y: float, grounded: bool, hp: float = 100.0,
        fall_start_y: float = 0.0) -> Dictionary:
    return {
        "pos_x": pos_x,
        "pos_y": pos_y,
        "vel_x": 0.0,
        "vel_y": 0.0,
        "grounded": grounded,
        "hp": hp,
        "fall_start_y": fall_start_y,
    }


func _profile_from_column_table(ground_y_by_col: Dictionary, tile_w_px: float = TILE_W_PX) -> Dictionary:
    # Build a dense profile array from a sparse column->surface_y dictionary. The profile spans
    # from the lowest to highest column key (inclusive) with NAN for missing columns (holes).
    var keys: Array = ground_y_by_col.keys()
    if keys.is_empty():
        return {"profile": [], "world_left_px": 0.0, "world_right_px": 0.0}
    var min_col: int = int(keys.min())
    var max_col: int = int(keys.max())
    var world_left_px: float = float(min_col) * tile_w_px
    var world_right_px: float = float(max_col + 1) * tile_w_px
    var width_px: int = int(world_right_px - world_left_px)
    var profile: Array = []
    profile.resize(width_px)
    for pixel_index in range(width_px):
        profile[pixel_index] = NAN
    for world_col in keys:
        var surface_y: float = float(ground_y_by_col[world_col])
        var col_start_px: int = (int(world_col) - min_col) * int(tile_w_px)
        for local_pixel_x in range(int(tile_w_px)):
            var profile_index: int = col_start_px + local_pixel_x
            if profile_index >= 0 and profile_index < profile.size():
                profile[profile_index] = surface_y
    return {"profile": profile, "world_left_px": world_left_px, "world_right_px": world_right_px}


func _solid_mask_with_top_rows(top_row_by_col: Array, tile_w: int = 32, tile_h: int = 32) -> PackedByteArray:
    # Build a solid_mask (one bit per pixel, row-major, bit = row*w+col) with the given topmost
    # solid row per local pixel column. -1 means fully air in that column.
    var mask: PackedByteArray = PackedByteArray()
    mask.resize(int(ceil(float(tile_w * tile_h) / 8.0)))
    mask.fill(0)
    for local_col in range(mini(top_row_by_col.size(), tile_w)):
        var top_row: int = int(top_row_by_col[local_col])
        if top_row < 0:
            continue
        for local_row in range(top_row, tile_h):
            var bit_index: int = local_row * tile_w + local_col
            var byte_index: int = int(bit_index / 8)
            var bit_offset: int = bit_index % 8
            mask[byte_index] = int(mask[byte_index]) | (1 << bit_offset)
    return mask


func _tile_with_mask(world_tile_x: int, world_tile_y: int, top_row_by_col: Array, is_walkable: bool = true) -> Dictionary:
    return {
        "world_tile_x": world_tile_x,
        "world_tile_y": world_tile_y,
        "is_walkable": is_walkable,
        "terrain": "test",
        "solid_mask": _solid_mask_with_top_rows(top_row_by_col),
    }


# Decode the shipped slope PNG into one topmost-solid-row per local column. The art is the
# independent oracle: green above the threshold is ground, every other pixel is air. Partially
# transparent edge pixels count, the way the committed solid masks do.
func _top_rows_from_png(slope_index: int) -> Array[int]:
    var tile_path: String = "res://assets/tiles/slope_up_%d_32x32.png" % slope_index
    # Read the FILE, not the imported texture: get_image() keeps green under fully transparent
    # pixels, which moves the edge up by six rows and disagrees with the committed solid masks.
    var tile_image: Image = Image.new()
    if tile_image.load(tile_path) != OK:
        return []
    if tile_image.get_width() != int(TILE_W_PX) or tile_image.get_height() != int(TILE_H_PX):
        return []
    var top_rows: Array[int] = []
    for local_col in range(int(TILE_W_PX)):
        var top_row: int = -1
        for local_row in range(int(TILE_H_PX)):
            if tile_image.get_pixel(local_col, local_row).g > SLOPE_PNG_GREEN_THRESHOLD:
                top_row = local_row
                break
        top_rows.append(top_row)
    return top_rows


func _flat_profile_ctx() -> Dictionary:
    var flat_ground: Dictionary = {}
    for world_col in range(int(WORLD_RIGHT_PX / TILE_W_PX)):
        flat_ground[world_col] = GROUND_TOP_PX
    return _profile_from_column_table(flat_ground)


func _test_walker_step() -> void:
    print("--- walker step (issue #122/#126) ---")
    var world_view_script: GDScript = load("res://scripts/world_view.gd") as GDScript
    if world_view_script == null:
        check(false, "world_view.gd failed to load")
        _completed.append_array(GUARDS)
        return

    # --- ground profile from tile masks ---
    # The surface of a column is the topmost solid bit in that column's mask. A fully-air column
    # is absent (NAN in profile). Legacy empty-mask tiles are whole-tile solid.
    var column_tiles: Array = [
        {"world_tile_x": 0, "world_tile_y": 32, "is_walkable": true, "terrain": "ground"},
        {"world_tile_x": 0, "world_tile_y": 31, "is_walkable": true, "terrain": "ground"},
        {"world_tile_x": 0, "world_tile_y": 30, "is_walkable": false, "terrain": "air"},
        {"world_tile_x": 1, "world_tile_y": 30, "is_walkable": false, "terrain": "air"},
    ]
    var ground_ctx: Dictionary = _profile_from_tiles(column_tiles)
    var ground_surface: float = WorldView.ground_y_at(
        ground_ctx["profile"], float(ground_ctx["world_left_px"]), 0.0)
    check(is_equal_approx(ground_surface, GROUND_TOP_PX),
        "ground_table: column 0 surfaces at the highest walkable tile (%.1f, want %.1f)" % [
            ground_surface, GROUND_TOP_PX])
    var no_surface: float = WorldView.ground_y_at(
        ground_ctx["profile"], float(ground_ctx["world_left_px"]), float(TILE_W_PX))
    check(is_nan(no_surface),
        "ground_table: a column of only non-walkable tiles has no surface")
    _completed.append("ground_table")

    # --- #126 solid_mask_surface_rule: topmost set bit per column, air column absent ---
    var top_rows: Array = []
    top_rows.resize(32)
    for i in range(32):
        top_rows[i] = 0
    top_rows[5] = 7
    top_rows[6] = -1
    var mask_tile: Dictionary = _tile_with_mask(0, 1, top_rows)
    var mask_ctx: Dictionary = _profile_from_tiles([mask_tile])
    check(is_equal_approx(WorldView.ground_y_at(mask_ctx["profile"], float(mask_ctx["world_left_px"]), 0.0), 32.0),
        "solid_mask_surface_rule: col 0 top row 0 => surface 32")
    check(is_equal_approx(WorldView.ground_y_at(mask_ctx["profile"], float(mask_ctx["world_left_px"]), 5.0), 39.0),
        "solid_mask_surface_rule: col 5 top row 7 => surface 39")
    check(is_nan(WorldView.ground_y_at(mask_ctx["profile"], float(mask_ctx["world_left_px"]), 6.0)),
        "solid_mask_surface_rule: col 6 all-air => NAN")
    _completed.append("solid_mask_surface_rule")

    # --- #126 interior_air_below_surface_is_solid: topmost solid row wins ---
    var interior_rows: Array = []
    interior_rows.resize(32)
    for i in range(32):
        interior_rows[i] = -1
    # Column 3: solid at rows 31 and 35 is not directly representable in a 32px tile; use rows 2 and 6.
    interior_rows[3] = 2
    var interior_mask: PackedByteArray = _solid_mask_with_top_rows(interior_rows, 32, 32)
    # Set row 6 solid too (below topmost solid row 2): row*w+col = 6*32+3 = 195
    var extra_bit: int = 6 * 32 + 3
    interior_mask[int(extra_bit / 8)] = int(interior_mask[int(extra_bit / 8)]) | (1 << (extra_bit % 8))
    var interior_tile: Dictionary = {"world_tile_x": 0, "world_tile_y": 0, "is_walkable": true, "terrain": "test", "solid_mask": interior_mask}
    var interior_ctx: Dictionary = _profile_from_tiles([interior_tile])
    check(is_equal_approx(WorldView.ground_y_at(interior_ctx["profile"], float(interior_ctx["world_left_px"]), 3.0), 2.0),
        "interior_air_below_surface_is_solid: topmost solid row 2 wins over deeper solid row 6")
    _completed.append("interior_air_below_surface_is_solid")

    # --- gravity lands the avatar on the ground and keeps it there ---
    var flat_ctx: Dictionary = _flat_profile_ctx()
    var falling: Dictionary = _walker(16.0, 900.0, false)
    var landed: Dictionary = _step_many(falling, 0.0, flat_ctx, SETTLE_STEPS)
    check(float(landed.get("pos_y", 0.0)) == GROUND_TOP_PX,
        "falls_then_lands: feet settle exactly on the ground line (%.1f, want %.1f)" % [
            float(landed.get("pos_y", 0.0)), GROUND_TOP_PX])
    check(bool(landed.get("grounded", false)),
        "falls_then_lands: and the walker reports itself grounded")
    _completed.append("falls_then_lands")

    var resting: Dictionary = _step(landed, 0.0, flat_ctx)
    check(float(resting.get("pos_y", 0.0)) == GROUND_TOP_PX,
        "stays_on_ground: a further frame does not sink or lift it (%.1f, want %.1f)" % [
            float(resting.get("pos_y", 0.0)), GROUND_TOP_PX])
    check(bool(resting.get("grounded", false)),
        "stays_on_ground: and it is still grounded")
    _completed.append("stays_on_ground")

    # --- a hole means falling, not snapping back up ---
    var void_ctx: Dictionary = {"profile": [], "world_left_px": 0.0, "world_right_px": 0.0}
    var over_void: Dictionary = _step_many(_walker(16.0, GROUND_TOP_PX, true), 0.0, void_ctx,
        SETTLE_STEPS)
    check(float(over_void.get("pos_y", 0.0)) > GROUND_TOP_PX,
        "falls_through_void: no ground under the feet means the walker falls (%.1f, want > %.1f)" % [
            float(over_void.get("pos_y", 0.0)), GROUND_TOP_PX])
    check(not bool(over_void.get("grounded", false)),
        "falls_through_void: and it is not grounded while falling")
    _completed.append("falls_through_void")

    # --- the world edges stop the walker ---
    var at_left_edge: Dictionary = _step_many(_walker(100.0, GROUND_TOP_PX, true), -1.0, flat_ctx,
        TRAVEL_STEPS)
    check(float(at_left_edge.get("pos_x", -1.0)) == WORLD_LEFT_PX,
        "clamps_left: walking left stops at the first screen's edge (%.1f, want %.1f)" % [
            float(at_left_edge.get("pos_x", -1.0)), WORLD_LEFT_PX])
    check(bool(at_left_edge.get("grounded", false))
        and float(at_left_edge.get("pos_y", 0.0)) == GROUND_TOP_PX,
        "clamps_left: and still stands on the ground line (grounded=%s pos_y=%.1f)" % [
            bool(at_left_edge.get("grounded", false)), float(at_left_edge.get("pos_y", 0.0))])
    _completed.append("clamps_left")

    var at_right_edge: Dictionary = _step_many(_walker(5700.0, GROUND_TOP_PX, true), 1.0, flat_ctx,
        TRAVEL_STEPS)
    check(float(at_right_edge.get("pos_x", -1.0)) == WORLD_RIGHT_PX,
        "clamps_right: walking right stops at the third screen's edge (%.1f, want %.1f)" % [
            float(at_right_edge.get("pos_x", -1.0)), WORLD_RIGHT_PX])
    check(bool(at_right_edge.get("grounded", false))
        and float(at_right_edge.get("pos_y", 0.0)) == GROUND_TOP_PX,
        "clamps_right: and still stands on the ground line (grounded=%s pos_y=%.1f)" % [
            bool(at_right_edge.get("grounded", false)), float(at_right_edge.get("pos_y", 0.0))])
    _completed.append("clamps_right")

    # --- a column with no walkable surface stops a grounded walker ---
    var wall_gap: Dictionary = {
        0: GROUND_TOP_PX, 1: GROUND_TOP_PX, 2: GROUND_TOP_PX,
        4: GROUND_TOP_PX, 5: GROUND_TOP_PX,
    }
    var wall_gap_ctx: Dictionary = _profile_from_column_table(wall_gap)
    var before_wall: Dictionary = _step_many(_walker(16.0, GROUND_TOP_PX, true), 1.0, wall_gap_ctx,
        TRAVEL_STEPS)
    check(float(before_wall.get("pos_x", -1.0)) < 3.0 * TILE_W_PX,
        "blocked_by_wall: walking right never enters the surface-less column (%.9f, want < %.1f)" % [
            float(before_wall.get("pos_x", -1.0)), 3.0 * TILE_W_PX])
    check(bool(before_wall.get("grounded", false))
        and float(before_wall.get("pos_y", 0.0)) == GROUND_TOP_PX,
        "blocked_by_wall: and it still stands on the ground line (grounded=%s pos_y=%.1f)" % [
            bool(before_wall.get("grounded", false)), float(before_wall.get("pos_y", 0.0))])
    var still_before_wall: Dictionary = _step_many(before_wall, 1.0, wall_gap_ctx, 120)
    check(float(still_before_wall.get("pos_x", -1.0)) == float(before_wall.get("pos_x", -2.0)),
        "blocked_by_wall: pushing into it for another 120 frames does not creep")
    var from_far_side: Dictionary = _step_many(
        _walker(4.0 * TILE_W_PX + 16.0, GROUND_TOP_PX, true), -1.0, wall_gap_ctx, TRAVEL_STEPS)
    check(float(from_far_side.get("pos_x", -1.0)) >= 4.0 * TILE_W_PX,
        "blocked_by_wall: walking left stops on its own side (%.2f, want >= %.1f)" % [
            float(from_far_side.get("pos_x", -1.0)), 4.0 * TILE_W_PX])
    check(bool(from_far_side.get("grounded", false))
        and float(from_far_side.get("pos_y", 0.0)) == GROUND_TOP_PX,
        "blocked_by_wall: and the left-moving walker is still grounded at the ground line")
    _completed.append("blocked_by_wall")

    # --- determinism: the same state and input produce the same result ---
    var first_run: Dictionary = _step_many(_walker(100.0, GROUND_TOP_PX, true), 1.0, flat_ctx, 37)
    var second_run: Dictionary = _step_many(_walker(100.0, GROUND_TOP_PX, true), 1.0, flat_ctx, 37)
    check(first_run == second_run,
        "deterministic: identical state and input give an identical walker state")
    _completed.append("deterministic")

    # --- hill climb: walking right onto a column whose surface is above the feet ---
    var hill_up_ctx: Dictionary = _profile_from_column_table({0: GROUND_TOP_PX, 1: 900.0})
    var on_flat: Dictionary = _walker_hp(16.0, GROUND_TOP_PX, true, 100.0, GROUND_TOP_PX)
    on_flat = _step_many(on_flat, 1.0, hill_up_ctx, 12)
    on_flat = _step_many(on_flat, 0.0, hill_up_ctx, SETTLE_STEPS)
    check(bool(on_flat.get("grounded", false)), "hill_climb: stands on the hill surface")
    check(is_equal_approx(float(on_flat.get("pos_y", 0.0)), 900.0),
        "hill_climb: feet settle on the higher surface (%.1f, want %.1f)" % [
            float(on_flat.get("pos_y", 0.0)), 900.0])
    check(float(on_flat.get("hp", 0.0)) == 100.0,
        "hill_climb: stepping up causes no fall damage")
    _completed.append("hill_climb")

    # --- hill fall damage: walking off a hill edge onto lower ground ---
    var hill_down_ctx: Dictionary = _profile_from_column_table({0: 900.0, 1: GROUND_TOP_PX})
    var at_hill_top: Dictionary = _walker_hp(16.0, 900.0, true, 100.0, 900.0)
    at_hill_top = _step_many(at_hill_top, 1.0, hill_down_ctx, 12)
    at_hill_top = _step_many(at_hill_top, 0.0, hill_down_ctx, SETTLE_STEPS)
    check(bool(at_hill_top.get("grounded", false)), "hill_fall_damage: settles on lower ground")
    check(is_equal_approx(float(at_hill_top.get("pos_y", 0.0)), GROUND_TOP_PX),
        "hill_fall_damage: feet settle on the flat ground (%.1f, want %.1f)" % [
            float(at_hill_top.get("pos_y", 0.0)), GROUND_TOP_PX])
    check(float(at_hill_top.get("hp", 0.0)) < 100.0,
        "hill_fall_damage: loses hp falling off the hill edge (hp=%.1f)" % [
            float(at_hill_top.get("hp", 0.0))])
    _completed.append("hill_fall_damage")

    # --- #126 slope fidelity: every column of the injected run matches the drawn PNG art ---
    # The oracle is the shipped slope art itself, decoded here (green over threshold = ground).
    # The profile under test is built from masks derived from those PNGs, so this sweep proves the
    # builder and sampler carry the art's topmost solid row into the walker's collision surface.
    var top_rows_by_tile: Array = []
    for slope_index in range(SLOPE_COLS):
        top_rows_by_tile.append(_top_rows_from_png(slope_index))
    var png_oracle_ok: bool = top_rows_by_tile.size() == SLOPE_COLS
    for decoded_rows in top_rows_by_tile:
        png_oracle_ok = png_oracle_ok and (decoded_rows as Array).size() == int(TILE_W_PX)
    check(png_oracle_ok,
        "slope_feet_y_matches_drawn_surface: all 7 slope PNGs decode to 32 columns")
    if not png_oracle_ok:
        _completed.append("slope_feet_y_matches_drawn_surface")
        return

    # Rebuild the tiles inject_simulator_hills emits, carrying the art-derived solid masks.
    var slope_tiles: Array = []
    for slope_index in range(SLOPE_COLS):
        var world_col: int = SLOPE_START_COL + slope_index
        var tile_row: int = GROUND_ROW - int(4 * slope_index / SLOPE_COLS)
        slope_tiles.append(_tile_with_mask(world_col, tile_row, top_rows_by_tile[slope_index]))
    var slope_ctx: Dictionary = _profile_from_tiles(slope_tiles)
    var slope_profile: Array = slope_ctx["profile"]
    var slope_left_px: float = float(slope_ctx["world_left_px"])

    # Independent surface Y at a world pixel column: tile placement row plus the PNG's topmost
    # solid row at that local column. No 4:7 ladder and no profile lookup in this derivation.
    var fn_png_surface_y := func(world_pixel_x: int) -> float:
        var slope_index: int = int(float(world_pixel_x - SLOPE_START_COL * int(TILE_W_PX)) / TILE_W_PX)
        if slope_index < 0 or slope_index >= SLOPE_COLS:
            return NAN
        var local_col: int = world_pixel_x - (SLOPE_START_COL + slope_index) * int(TILE_W_PX)
        var top_row: int = int(top_rows_by_tile[slope_index][local_col])
        if top_row < 0:
            return NAN
        var tile_row: int = GROUND_ROW - int(4 * slope_index / SLOPE_COLS)
        return float(tile_row * int(TILE_H_PX) + top_row)

    # Sweep all 224 columns: the sampled profile must equal the PNG oracle within 1 px.
    var sweep_start_px: int = SLOPE_START_COL * int(TILE_W_PX)
    var sweep_end_px: int = sweep_start_px + SLOPE_COLS * int(TILE_W_PX)
    var worst_profile_error_px: float = 0.0
    for world_pixel_x in range(sweep_start_px, sweep_end_px):
        var want_y: float = fn_png_surface_y.call(world_pixel_x)
        var got_y: float = WorldView.ground_y_at(slope_profile, slope_left_px, float(world_pixel_x))
        worst_profile_error_px = maxf(worst_profile_error_px, absf(got_y - want_y))
    check(worst_profile_error_px <= 1.0,
        "slope_feet_y_matches_drawn_surface: profile matches the PNG at every column (%.2f px, want <= 1)" % [
            worst_profile_error_px])

    # Second, hand-derived pin: the 7 seams and 7 centres must agree with the PNG oracle, so a
    # bug shared by the decoder and the mask builder cannot pass.
    var worst_hand_pin_error_px: float = 0.0
    for slope_index in range(SLOPE_COLS):
        var seam_x: int = (SLOPE_START_COL + slope_index) * int(TILE_W_PX)
        var centre_x: int = seam_x + int(TILE_W_PX) / 2
        worst_hand_pin_error_px = maxf(worst_hand_pin_error_px,
            absf(fn_png_surface_y.call(seam_x) - float(SLOPE_SEAM_Y_PX[slope_index])))
        worst_hand_pin_error_px = maxf(worst_hand_pin_error_px,
            absf(fn_png_surface_y.call(centre_x) - float(SLOPE_CENTRE_Y_PX[slope_index])))
    check(worst_hand_pin_error_px <= 1.0,
        "slope_feet_y_matches_drawn_surface: hand-measured seams and centres agree with the art (%.2f px, want <= 1)" % [
            worst_hand_pin_error_px])

    # The 13 px notch at world column 61 is drawn art, not noise to smooth away.
    var notch_px: float = absf(fn_png_surface_y.call((SLOPE_START_COL + 1) * int(TILE_W_PX))
        - fn_png_surface_y.call((SLOPE_START_COL + 1) * int(TILE_W_PX) - 1))
    check(is_equal_approx(notch_px, 13.0),
        "slope_feet_y_matches_drawn_surface: 13 px notch at world column 61 (%.1f px)" % [notch_px])

    # Seed at the run's first column centre and climb right: feet track the oracle every frame,
    # with no half-tile hop. This is the smooth-climb acceptance line.
    var first_centre_x: int = sweep_start_px + int(TILE_W_PX) / 2
    var slope_walker: Dictionary = _walker_hp(
        float(first_centre_x), fn_png_surface_y.call(first_centre_x), true)
    var worst_feet_error_px: float = 0.0
    var max_frame_rise_px: float = 0.0
    var walked_frames: int = 0
    for frame_index in range(SETTLE_STEPS):
        var previous_y: float = float(slope_walker.get("pos_y", 0.0))
        slope_walker = _step(slope_walker, 1.0, slope_ctx)
        var walker_x: float = float(slope_walker.get("pos_x", 0.0))
        var want_y: float = fn_png_surface_y.call(int(floor(walker_x)))
        if is_nan(want_y):
            break
        var feet_y: float = float(slope_walker.get("pos_y", 0.0))
        worst_feet_error_px = maxf(worst_feet_error_px, absf(feet_y - want_y))
        max_frame_rise_px = maxf(max_frame_rise_px, previous_y - feet_y)
        walked_frames += 1
    check(walked_frames > 50,
        "slope_feet_y_matches_drawn_surface: walker climbs the whole run (%d frames inside the run)" % [
            walked_frames])
    check(worst_feet_error_px <= 2.0,
        "slope_feet_y_matches_drawn_surface: feet track the PNG art at every walked frame (%.2f px, want <= 2)" % [
            worst_feet_error_px])
    check(max_frame_rise_px < 16.0,
        "slope_feet_y_matches_drawn_surface: no single-frame rise exceeds half a tile (%.1f px, want < 16)" % [
            max_frame_rise_px])
    _completed.append("slope_feet_y_matches_drawn_surface")
