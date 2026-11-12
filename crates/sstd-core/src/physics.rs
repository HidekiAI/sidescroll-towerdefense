//! Ground-walking physics for the avatar.
//!
//! The live per-frame walker in the editor is the GDScript `world_view.gd::step_walker`; this
//! module is the Rust model behind the bridge's `walk_step` call, and the two are not identical
//! yet. The GDScript walker interpolates the feet across a column when the next column's surface
//! differs (the 30-degree slope); this one snaps to the column's own surface. A change here does
//! not reach the running editor, and the two must not be "fixed" to match each other before
//! deciding which one is authoritative.
//!
//! Coordinate spaces, because the body mixes all of them:
//!
//! - `world_tile_x` / `world_tile_y` are tile columns and rows, integers.
//! - `pos_x` / `pos_y` are world pixels of the model's ground-contact line, `y` growing downward.
//!   The field names are the wire contract; the live editor writes `pos_y` to the avatar's `Node2D`
//!   origin, which sits mid-torso, not at the drawn feet (see the note in [`step_walker`]).
//! - a surface is a row's top edge in pixels, `world_tile_y * tile_h`.
//!
//! Design: issue #125 (legibility only, no behaviour change).

use serde::{Deserialize, Serialize};

/// Downward acceleration, pixels per second squared.
pub const GRAVITY_PPS2: f64 = 980.0;

/// Default horizontal walk speed, pixels per second.
pub const DEFAULT_AVATAR_SPEED_PPS: f64 = 200.0;

/// Frame time used when the caller passes a non-positive `dt`.
const DEFAULT_FRAME_SECONDS: f64 = 1.0 / 60.0;

/// A landing costs hit points once the fall exceeds this many pixels.
const FALL_DAMAGE_THRESHOLD_PX: f64 = 32.0;

/// Hit points one damaging landing costs.
const FALL_DAMAGE_POINTS: f64 = 1.0;

/// Walker state carried from one frame to the next.
///
/// The field names are the wire contract: they are serialized into the bridge's `walk_step`
/// JSON and mirrored by the `_walker_state` dictionary in `world_view.gd`, so renaming one needs
/// both sides changed in the same commit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalkerState {
    /// Feet position, world pixels.
    pub pos_x: f64,
    pub pos_y: f64,
    /// Velocity, pixels per second.
    pub vel_x: f64,
    pub vel_y: f64,
    /// True when the feet rest on a surface at the end of the frame.
    pub grounded: bool,
    pub hp: f64,
    /// Height the current fall started from, used for fall damage.
    pub fall_start_y: f64,
}

/// One frame of player intent, as the editor sends it.
///
/// Only [`WalkerInput::move_x`] is read by [`step_walker`]. The rest are carried for the editor's
/// input contract and are not modelled here; they exist so the JSON shape stays stable when the
/// remaining actions move to Rust.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalkerInput {
    /// Horizontal intent as an axis fraction: -1 left, +1 right.
    pub move_x: f64,
    pub move_y: f64,
    pub move_to_world: Option<(f64, f64)>,
    pub scroll_vec: Option<(f64, f64)>,
    pub place: bool,
    pub alt_place: bool,
    pub interact: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
}

/// Tile size in pixels.
///
/// Named the same as `terrain::GridConfig`, which is a different type; the bridge passes this one
/// as `{"tile_w": .., "tile_h": ..}` under `walk_step`, so its field names are a wire contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridConfig {
    pub tile_w: f64,
    pub tile_h: f64,
}

/// One tile of the ground, in tile-grid coordinates.
#[derive(Clone, Debug)]
pub struct Tile {
    pub world_tile_x: i32,
    pub world_tile_y: i32,
    pub is_walkable: bool,
}

/// Leftmost and rightmost tile left edge, in world pixels.
///
/// An empty tile list falls back to one tile starting at 0. Note that the right bound is the
/// last tile's LEFT edge, not its right edge, so the walker stops one tile short of the tile
/// list's true extent; that is today's behaviour and this ticket does not change it.
fn world_bounds_px(tiles: &[Tile], grid: &GridConfig) -> (f64, f64) {
    let fn_tile_left_edges_px = || {
        tiles
            .iter()
            .map(|tile| tile.world_tile_x as f64 * grid.tile_w)
    };
    let leftmost_px = fn_tile_left_edges_px().reduce(f64::min);
    let rightmost_px = fn_tile_left_edges_px().reduce(f64::max);
    // Both bounds come from the same list, so `zip` is `None` exactly when the list is empty.
    leftmost_px.zip(rightmost_px).unwrap_or((0.0, grid.tile_w))
}

/// Tile column a world-pixel `pos_x` falls in, clamped to the columns the tile list covers.
fn world_tile_col_at(pos_x: f64, grid: &GridConfig, last_world_tile_col: i32) -> i32 {
    ((pos_x / grid.tile_w).floor() as i32).clamp(0, last_world_tile_col)
}

/// Hit points one landing costs, or `None` when the fall was safe.
fn fall_damage_points(fall_start_y: f64, landing_y: f64) -> Option<f64> {
    (landing_y - fall_start_y > FALL_DAMAGE_THRESHOLD_PX).then_some(FALL_DAMAGE_POINTS)
}

/// Top edge of the highest walkable tile in `world_tile_col`, in world pixels.
///
/// "Highest" is the SMALLEST `world_tile_y`, because `y` grows downward. A column whose tiles are
/// all non-walkable has no surface at all, so `None` means a hole and the walker falls through it.
pub fn ground_y_for_column(tiles: &[Tile], grid: &GridConfig, world_tile_col: i32) -> Option<f64> {
    let surface_row = tiles
        .iter()
        .filter(|tile| tile.is_walkable && tile.world_tile_x == world_tile_col)
        .map(|tile| tile.world_tile_y)
        .min()?;
    Some(surface_row as f64 * grid.tile_h)
}

/// What one frame's ground test found.
///
/// Declared here, above its first use, because it never leaves this file.
struct GroundContact {
    /// True when the feet end the frame resting on a surface.
    landed: bool,
    /// The surface the feet end on, or the free-fall position when the column is a hole.
    ground_y: f64,
    /// Height the current fall started from, used for fall damage.
    fall_start_y: f64,
}

/// Advance the walker by one frame.
///
/// Modelled: gravity, landing from above, stepping up onto a higher surface while moving
/// horizontally, one hit point of fall damage past [`FALL_DAMAGE_THRESHOLD_PX`], the world-edge
/// clamp, and a grounded walker refusing to step into a column with no walkable surface.
///
/// Not modelled: a collider smaller than a whole tile, jumping, and anything above the feet.
pub fn step_walker(
    state: WalkerState,
    input: &WalkerInput,
    dt: f64,
    grid: &GridConfig,
    tiles: &[Tile],
    speed_pps: f64,
) -> WalkerState {
    let frame_seconds = if dt <= 0.0 { DEFAULT_FRAME_SECONDS } else { dt };
    // A held key is an axis fraction; clamping keeps a caller from asking for more than full speed.
    let move_axis = input.move_x.clamp(-1.0, 1.0); // NaN stays NaN here; the old max/min form returned -1.0 (unreachable via JSON)
    let move_speed_pps = move_axis * speed_pps;

    // NOTE: the avatar has no collision bounding rect/capsule/sphere yet, so this model treats
    // `pos_y` as a precomputed "feet" line. A sprite hotspot and a collision boundary bottom are
    // not the same point, and a collision body is struck from the sides too, not only from the
    // bottom.
    let feet_was_y = state.pos_y;
    let vel_y_after_gravity = state.vel_y + GRAVITY_PPS2 * frame_seconds;
    let feet_now_y = feet_was_y + vel_y_after_gravity * frame_seconds;

    let (world_left_px, world_right_px) = world_bounds_px(tiles, grid);
    let last_world_tile_col = tiles
        .iter()
        .map(|tile| tile.world_tile_x)
        .max()
        .unwrap_or(0);
    let proposed_pos_x =
        (state.pos_x + move_speed_pps * frame_seconds).clamp(world_left_px, world_right_px);
    let proposed_col = world_tile_col_at(proposed_pos_x, grid, last_world_tile_col);

    // A column holding only non-walkable tiles has no surface, so a grounded walker stepping into
    // it would fall exactly as if it were a pit. Refuse the step by staying on the previous pixel.
    // An ungrounded walker keeps moving (it is already falling), and a walker that does not move is
    // unaffected. Refusing rather than clamping to the column edge keeps this direction-agnostic:
    // the walker is at most one frame of travel from the boundary, so no visible gap opens.
    let stepping_into_hole = move_axis != 0.0
        && state.grounded
        && ground_y_for_column(tiles, grid, proposed_col).is_none();
    let (pos_x, world_tile_col) = if stepping_into_hole {
        (
            state.pos_x,
            world_tile_col_at(state.pos_x, grid, last_world_tile_col),
        )
    } else {
        (proposed_pos_x, proposed_col)
    };

    let contact = match ground_y_for_column(tiles, grid, world_tile_col) {
        Some(surface_y) => {
            let landed_from_above =
                vel_y_after_gravity >= 0.0 && feet_was_y <= surface_y && feet_now_y >= surface_y;
            let stepped_up = surface_y < feet_was_y && move_axis != 0.0;
            GroundContact {
                // The two cases are mutually exclusive: one needs the surface at or below the
                // feet, the other needs it above them.
                landed: landed_from_above || stepped_up,
                ground_y: surface_y,
                // A step up is not a fall: the damage test reads this BEFORE the landing rewrites
                // `fall_start_y` to `ground_y`, so it must not see the fall still in progress.
                fall_start_y: if stepped_up {
                    surface_y
                } else {
                    state.fall_start_y
                },
            }
        }
        // No ground in this column: a hole. Remember where the fall began if we were standing.
        None => GroundContact {
            landed: false,
            ground_y: feet_now_y,
            fall_start_y: if state.grounded {
                feet_was_y
            } else {
                state.fall_start_y
            },
        },
    };

    if contact.landed {
        // Fall damage applies only to a walker that was airborne; walking along a floor never hurts.
        // A grounded walker's fall start is its own surface, so the drop test would refuse anyway;
        // this guard states the intent instead of leaning on that invariant.
        let hp = if state.grounded {
            state.hp
        } else {
            fall_damage_points(contact.fall_start_y, contact.ground_y)
                .map_or(state.hp, |damage_points| {
                    (state.hp - damage_points).max(0.0)
                })
        };
        WalkerState {
            pos_x,
            pos_y: contact.ground_y,
            vel_x: move_speed_pps,
            vel_y: 0.0,
            grounded: true,
            hp,
            fall_start_y: contact.ground_y,
        }
    } else {
        WalkerState {
            pos_x,
            pos_y: feet_now_y,
            vel_x: move_speed_pps,
            vel_y: vel_y_after_gravity,
            grounded: false,
            hp: state.hp,
            fall_start_y: contact.fall_start_y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TILE_PX: f64 = 64.0;
    const TEST_SPEED_PPS: f64 = 200.0;
    const TEST_FRAME_SECONDS: f64 = 1.0 / 60.0;
    const TEST_HP: f64 = 100.0;
    const TEST_GROUND_ROW: i32 = 1;

    fn test_grid() -> GridConfig {
        GridConfig {
            tile_w: TEST_TILE_PX,
            tile_h: TEST_TILE_PX,
        }
    }

    /// One walkable tile per given column, all on `world_tile_row`.
    fn flat_ground(columns: &[i32], world_tile_row: i32) -> Vec<Tile> {
        columns
            .iter()
            .map(|&world_tile_x| Tile {
                world_tile_x,
                world_tile_y: world_tile_row,
                is_walkable: true,
            })
            .collect()
    }

    fn walker_input(move_x: f64) -> WalkerInput {
        WalkerInput {
            move_x,
            move_y: 0.0,
            move_to_world: None,
            scroll_vec: None,
            place: false,
            alt_place: false,
            interact: false,
            zoom_in: false,
            zoom_out: false,
        }
    }

    fn standing_walker(pos_x: f64, surface_y: f64) -> WalkerState {
        WalkerState {
            pos_x,
            pos_y: surface_y,
            vel_x: 0.0,
            vel_y: 0.0,
            grounded: true,
            hp: TEST_HP,
            fall_start_y: surface_y,
        }
    }

    #[test]
    fn surface_lookup_returns_highest_walkable_tile() {
        let tiles = vec![
            Tile {
                world_tile_x: 1,
                world_tile_y: 2,
                is_walkable: true,
            },
            Tile {
                world_tile_x: 1,
                world_tile_y: 1,
                is_walkable: true,
            },
        ];
        assert_eq!(
            ground_y_for_column(&tiles, &test_grid(), 1),
            Some(TEST_TILE_PX)
        );
    }

    #[test]
    fn surface_lookup_is_none_when_column_has_no_walkable_tile() {
        let tiles = vec![Tile {
            world_tile_x: 1,
            world_tile_y: 1,
            is_walkable: false,
        }];
        assert_eq!(ground_y_for_column(&tiles, &test_grid(), 1), None);
    }

    #[test]
    fn walker_snaps_to_ground() {
        let grid = test_grid();
        let tiles = flat_ground(&[0], TEST_GROUND_ROW);
        let falling_walker = WalkerState {
            pos_x: 10.0,
            pos_y: 10.0,
            vel_x: 0.0,
            vel_y: 500.0,
            grounded: false,
            hp: TEST_HP,
            fall_start_y: 10.0,
        };
        let walker = (0..60).fold(falling_walker, |walker, _| {
            step_walker(
                walker,
                &walker_input(0.0),
                TEST_FRAME_SECONDS,
                &grid,
                &tiles,
                TEST_SPEED_PPS,
            )
        });
        assert!(walker.grounded);
        assert_eq!(walker.pos_y, TEST_TILE_PX);
    }

    #[test]
    fn walker_refuses_to_step_into_a_hole() {
        let grid = test_grid();
        // Columns 0-2 and 4-5 stand on row 1. Column 3 holds no walkable tile, so a grounded walker
        // that stepped into it would drop exactly as if it were a pit.
        let tiles = flat_ground(&[0, 1, 2, 4, 5], TEST_GROUND_ROW);
        let walker = (0..300).fold(standing_walker(10.0, TEST_TILE_PX), |walker, _| {
            step_walker(
                walker,
                &walker_input(1.0),
                TEST_FRAME_SECONDS,
                &grid,
                &tiles,
                TEST_SPEED_PPS,
            )
        });
        assert!(
            walker.pos_x < 3.0 * grid.tile_w,
            "walker entered the surface-less column: {}",
            walker.pos_x
        );
        assert!(walker.grounded, "walker lost the ground while blocked");
        assert_eq!(walker.pos_y, TEST_TILE_PX);
        let blocked_pos_x = walker.pos_x;
        let walker = (0..300).fold(walker, |walker, _| {
            step_walker(
                walker,
                &walker_input(1.0),
                TEST_FRAME_SECONDS,
                &grid,
                &tiles,
                TEST_SPEED_PPS,
            )
        });
        assert_eq!(
            walker.pos_x, blocked_pos_x,
            "walker crept through the block"
        );
    }

    #[test]
    fn walking_a_flat_floor_keeps_the_feet_and_hit_points() {
        let grid = test_grid();
        let tiles = flat_ground(&[0, 1, 2], TEST_GROUND_ROW);
        let surface_y = TEST_GROUND_ROW as f64 * TEST_TILE_PX;
        let walker = (0..120).fold(standing_walker(10.0, surface_y), |walker, _| {
            step_walker(
                walker,
                &walker_input(1.0),
                TEST_FRAME_SECONDS,
                &grid,
                &tiles,
                TEST_SPEED_PPS,
            )
        });
        assert!(walker.grounded);
        assert_eq!(walker.pos_y, surface_y);
        assert_eq!(walker.hp, TEST_HP);
        // The right bound is the last tile's left edge, so the walker stops there; see #125.
        assert_eq!(walker.pos_x, 2.0 * TEST_TILE_PX);
    }

    /// Stepping up is not a fall: the walker must land on the higher surface, take no damage, and
    /// carry the higher surface as the new fall start.
    #[test]
    fn stepping_up_onto_a_higher_surface_does_not_start_a_fall() {
        let grid = test_grid();
        // Column 0 stands on row 1; column 1 is one tile higher, on row 0.
        let tiles = vec![
            Tile {
                world_tile_x: 0,
                world_tile_y: TEST_GROUND_ROW,
                is_walkable: true,
            },
            Tile {
                world_tile_x: 1,
                world_tile_y: TEST_GROUND_ROW - 1,
                is_walkable: true,
            },
        ];
        let walker = (0..30).fold(standing_walker(10.0, TEST_TILE_PX), |walker, _| {
            step_walker(
                walker,
                &walker_input(1.0),
                TEST_FRAME_SECONDS,
                &grid,
                &tiles,
                TEST_SPEED_PPS,
            )
        });
        assert!(walker.grounded);
        assert_eq!(walker.pos_y, 0.0);
        assert_eq!(walker.hp, TEST_HP);
        assert_eq!(walker.fall_start_y, 0.0);
    }

    /// Falling beside a taller column and drifting into its face is a step up, not a landing from a
    /// height: the fall in progress must not be charged to it.
    #[test]
    fn an_airborne_walker_stepping_onto_a_ledge_takes_no_fall_damage() {
        let grid = test_grid();
        // Column 0 stands four tiles lower than column 1, so a walker falling down that cliff face
        // reaches column 1's surface from BELOW it while still airborne.
        let tiles = vec![
            Tile {
                world_tile_x: 0,
                world_tile_y: 4,
                is_walkable: true,
            },
            Tile {
                world_tile_x: 1,
                world_tile_y: TEST_GROUND_ROW,
                is_walkable: true,
            },
        ];
        let walker = step_walker(
            WalkerState {
                pos_x: 61.0,
                pos_y: 200.0,
                vel_x: 0.0,
                vel_y: 0.0,
                grounded: false,
                hp: TEST_HP,
                fall_start_y: 10.0,
            },
            &walker_input(1.0),
            TEST_FRAME_SECONDS,
            &grid,
            &tiles,
            TEST_SPEED_PPS,
        );
        assert!(walker.grounded);
        assert_eq!(walker.pos_y, TEST_TILE_PX);
        assert_eq!(walker.hp, TEST_HP);
    }

    #[test]
    fn landing_past_the_safe_drop_costs_one_hit_point() {
        let grid = test_grid();
        let tiles = flat_ground(&[0], 4);
        let surface_y = 4.0 * TEST_TILE_PX;
        let walker = step_walker(
            WalkerState {
                pos_x: 10.0,
                pos_y: 250.0,
                vel_x: 0.0,
                vel_y: 600.0,
                grounded: false,
                hp: TEST_HP,
                fall_start_y: 100.0,
            },
            &walker_input(0.0),
            TEST_FRAME_SECONDS,
            &grid,
            &tiles,
            TEST_SPEED_PPS,
        );
        assert!(walker.grounded);
        assert_eq!(walker.pos_y, surface_y);
        // The literal is deliberate: this is the test that pins a damaging landing at exactly one
        // hit point. Comparing against FALL_DAMAGE_POINTS would move the expectation with the
        // constant and pass whatever the constant became.
        assert_eq!(walker.hp, TEST_HP - 1.0);
    }

    #[test]
    fn landing_within_the_safe_drop_costs_nothing() {
        let grid = test_grid();
        let tiles = flat_ground(&[0], 4);
        let surface_y = 4.0 * TEST_TILE_PX;
        let walker = step_walker(
            WalkerState {
                pos_x: 10.0,
                pos_y: 250.0,
                vel_x: 0.0,
                vel_y: 600.0,
                grounded: false,
                hp: TEST_HP,
                fall_start_y: 250.0,
            },
            &walker_input(0.0),
            TEST_FRAME_SECONDS,
            &grid,
            &tiles,
            TEST_SPEED_PPS,
        );
        assert!(walker.grounded);
        assert_eq!(walker.pos_y, surface_y);
        assert_eq!(walker.hp, TEST_HP);
    }
}
