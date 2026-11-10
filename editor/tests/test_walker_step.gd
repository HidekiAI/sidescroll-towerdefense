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


func _step(state: Dictionary, move_x: float, ground_y_by_col: Dictionary) -> Dictionary:
    return WorldView.step_walker(
        state, move_x, STEP_SECONDS, ground_y_by_col, TILE_W_PX, WORLD_LEFT_PX, WORLD_RIGHT_PX)


func _step_many(state: Dictionary, move_x: float, ground_y_by_col: Dictionary,
        step_count: int) -> Dictionary:
    var settled: Dictionary = state
    for step_index in range(step_count):
        settled = _step(settled, move_x, ground_y_by_col)
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


func _test_walker_step() -> void:
    print("--- walker step (issue #122) ---")
    var world_view_script: GDScript = load("res://scripts/world_view.gd") as GDScript
    if world_view_script == null:
        check(false, "world_view.gd failed to load")
        _completed.append_array(GUARDS)
        return

    # --- the ground table ---
    #
    # The surface of a column is its HIGHEST walkable tile (smallest row index), and a column
    # with only non-walkable tiles has no surface at all. Row 31 x 32 px = 992, which is the
    # prototype world's ground line.
    var column_tiles: Array = [
        {"world_tile_x": 0, "world_tile_y": 32, "is_walkable": true},
        {"world_tile_x": 0, "world_tile_y": 31, "is_walkable": true},
        {"world_tile_x": 0, "world_tile_y": 30, "is_walkable": false},
        {"world_tile_x": 1, "world_tile_y": 30, "is_walkable": false},
    ]
    var ground_table: Dictionary = WorldView.build_ground_column_table(column_tiles, TILE_H_PX)
    check(is_equal_approx(float(ground_table.get(0, -1.0)), GROUND_TOP_PX),
        "ground_table: column 0 surfaces at the highest walkable tile (%.1f, want %.1f)" % [
            float(ground_table.get(0, -1.0)), GROUND_TOP_PX])
    check(not ground_table.has(1),
        "ground_table: a column of only non-walkable tiles has no surface")
    _completed.append("ground_table")

    # --- gravity lands the avatar on the ground and keeps it there ---
    # The prototype world's real ground: 3 screens x 60 columns, flat on the same line, so a walk
    # from the left edge to the right edge crosses every surface and no seam. Derived from the
    # world width, so widening the prototype cannot leave a stale column count here.
    var flat_ground: Dictionary = {}
    for world_col in range(int(WORLD_RIGHT_PX / TILE_W_PX)):
        flat_ground[world_col] = GROUND_TOP_PX
    var falling: Dictionary = _walker(16.0, 900.0, false)
    var landed: Dictionary = _step_many(falling, 0.0, flat_ground, SETTLE_STEPS)
    check(float(landed.get("pos_y", 0.0)) == GROUND_TOP_PX,
        "falls_then_lands: feet settle exactly on the ground line (%.1f, want %.1f)" % [
            float(landed.get("pos_y", 0.0)), GROUND_TOP_PX])
    check(bool(landed.get("grounded", false)),
        "falls_then_lands: and the walker reports itself grounded")
    _completed.append("falls_then_lands")

    var resting: Dictionary = _step(landed, 0.0, flat_ground)
    check(float(resting.get("pos_y", 0.0)) == GROUND_TOP_PX,
        "stays_on_ground: a further frame does not sink or lift it (%.1f, want %.1f)" % [
            float(resting.get("pos_y", 0.0)), GROUND_TOP_PX])
    check(bool(resting.get("grounded", false)),
        "stays_on_ground: and it is still grounded")
    _completed.append("stays_on_ground")

    # --- a hole means falling, not snapping back up ---
    var over_void: Dictionary = _step_many(_walker(16.0, GROUND_TOP_PX, true), 0.0, {},
        SETTLE_STEPS)
    check(float(over_void.get("pos_y", 0.0)) > GROUND_TOP_PX,
        "falls_through_void: no ground under the feet means the walker falls (%.1f, want > %.1f)" % [
            float(over_void.get("pos_y", 0.0)), GROUND_TOP_PX])
    check(not bool(over_void.get("grounded", false)),
        "falls_through_void: and it is not grounded while falling")
    _completed.append("falls_through_void")

    # --- the world edges stop the walker ---
    var at_left_edge: Dictionary = _step_many(_walker(100.0, GROUND_TOP_PX, true), -1.0, flat_ground,
        TRAVEL_STEPS)
    check(float(at_left_edge.get("pos_x", -1.0)) == WORLD_LEFT_PX,
        "clamps_left: walking left stops at the first screen's edge (%.1f, want %.1f)" % [
            float(at_left_edge.get("pos_x", -1.0)), WORLD_LEFT_PX])
    # The edge is a WALL, not a drop: the boundary pixel belongs to the last column's tile, so the
    # ground lookup must still find a surface there. Without this the walker clamps to the edge and
    # then falls out of the world, which is exactly what a pos_x-only assertion cannot see.
    check(bool(at_left_edge.get("grounded", false))
        and float(at_left_edge.get("pos_y", 0.0)) == GROUND_TOP_PX,
        "clamps_left: and still stands on the ground line (grounded=%s pos_y=%.1f)" % [
            bool(at_left_edge.get("grounded", false)), float(at_left_edge.get("pos_y", 0.0))])
    _completed.append("clamps_left")

    var at_right_edge: Dictionary = _step_many(_walker(5700.0, GROUND_TOP_PX, true), 1.0, flat_ground,
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
    #
    # Columns 0-2 and 4-5 stand at 992; column 3 holds only non-walkable tiles, so it has no entry
    # in the ground table. Walking must stop before column 3 instead of dropping the avatar into
    # it, because a wall and a pit are the same thing to the table.
    var wall_gap: Dictionary = {
        0: GROUND_TOP_PX, 1: GROUND_TOP_PX, 2: GROUND_TOP_PX,
        4: GROUND_TOP_PX, 5: GROUND_TOP_PX,
    }
    var before_wall: Dictionary = _step_many(_walker(16.0, GROUND_TOP_PX, true), 1.0, wall_gap,
        TRAVEL_STEPS)
    check(float(before_wall.get("pos_x", -1.0)) < 3.0 * TILE_W_PX,
        "blocked_by_wall: walking right never enters the surface-less column (%.9f, want < %.1f)" % [
            float(before_wall.get("pos_x", -1.0)), 3.0 * TILE_W_PX])
    check(bool(before_wall.get("grounded", false))
        and float(before_wall.get("pos_y", 0.0)) == GROUND_TOP_PX,
        "blocked_by_wall: and it still stands on the ground line (grounded=%s pos_y=%.1f)" % [
            bool(before_wall.get("grounded", false)), float(before_wall.get("pos_y", 0.0))])
    var still_before_wall: Dictionary = _step_many(before_wall, 1.0, wall_gap, 120)
    check(float(still_before_wall.get("pos_x", -1.0)) == float(before_wall.get("pos_x", -2.0)),
        "blocked_by_wall: pushing into it for another 120 frames does not creep")
    # The reverse direction must block too. A clamp to "one pixel before the column" is
    # direction-blind and would carry a left-moving walker across the gap onto the far side.
    var from_far_side: Dictionary = _step_many(
        _walker(4.0 * TILE_W_PX + 16.0, GROUND_TOP_PX, true), -1.0, wall_gap, TRAVEL_STEPS)
    check(float(from_far_side.get("pos_x", -1.0)) >= 4.0 * TILE_W_PX,
        "blocked_by_wall: walking left stops on its own side (%.2f, want >= %.1f)" % [
            float(from_far_side.get("pos_x", -1.0)), 4.0 * TILE_W_PX])
    check(bool(from_far_side.get("grounded", false))
        and float(from_far_side.get("pos_y", 0.0)) == GROUND_TOP_PX,
        "blocked_by_wall: and the left-moving walker is still grounded at the ground line")
    _completed.append("blocked_by_wall")

    # --- determinism: the same state and input produce the same result ---
    var first_run: Dictionary = _step_many(_walker(100.0, GROUND_TOP_PX, true), 1.0, flat_ground, 37)
    var second_run: Dictionary = _step_many(_walker(100.0, GROUND_TOP_PX, true), 1.0, flat_ground, 37)
    check(first_run == second_run,
        "deterministic: identical state and input give an identical walker state")
    _completed.append("deterministic")

    # --- hill climb: walking right onto a column whose surface is above the feet ---
    var hill_up: Dictionary = {}
    hill_up[0] = GROUND_TOP_PX     # flat ground at 992
    hill_up[1] = 900.0             # hill top at 900 (y-down: smaller y is higher ground)
    var on_flat: Dictionary = _walker_hp(16.0, GROUND_TOP_PX, true, 100.0, GROUND_TOP_PX)
    on_flat = _step_many(on_flat, 1.0, hill_up, 12)           # walk into column 1
    on_flat = _step_many(on_flat, 0.0, hill_up, SETTLE_STEPS) # step up onto the hill surface
    check(bool(on_flat.get("grounded", false)), "hill_climb: stands on the hill surface")
    check(is_equal_approx(float(on_flat.get("pos_y", 0.0)), 900.0),
        "hill_climb: feet settle on the higher surface (%.1f, want %.1f)" % [
            float(on_flat.get("pos_y", 0.0)), 900.0])
    check(float(on_flat.get("hp", 0.0)) == 100.0,
        "hill_climb: stepping up causes no fall damage")
    _completed.append("hill_climb")

    # --- hill fall damage: walking off a hill edge onto lower ground ---
    var hill_down: Dictionary = {}
    hill_down[0] = 900.0             # hill top at 900
    hill_down[1] = GROUND_TOP_PX     # flat ground at 992 below the hill edge
    var at_hill_top: Dictionary = _walker_hp(16.0, 900.0, true, 100.0, 900.0)
    at_hill_top = _step_many(at_hill_top, 1.0, hill_down, 12)   # walk off the hill edge
    at_hill_top = _step_many(at_hill_top, 0.0, hill_down, SETTLE_STEPS)  # fall onto lower ground
    check(bool(at_hill_top.get("grounded", false)), "hill_fall_damage: settles on lower ground")
    check(is_equal_approx(float(at_hill_top.get("pos_y", 0.0)), GROUND_TOP_PX),
        "hill_fall_damage: feet settle on the flat ground (%.1f, want %.1f)" % [
            float(at_hill_top.get("pos_y", 0.0)), GROUND_TOP_PX])
    check(float(at_hill_top.get("hp", 0.0)) < 100.0,
        "hill_fall_damage: loses hp falling off the hill edge (hp=%.1f)" % [
            float(at_hill_top.get("hp", 0.0))])
    _completed.append("hill_fall_damage")
