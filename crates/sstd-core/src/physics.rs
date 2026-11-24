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

/// On penetration: snap the entity out and stay glued while the feet are within this many pixels
/// above the surface. Half a 32 px tile. (amendment #126 section 4)
pub const GROUND_SNAP_PX: f64 = 16.0;

/// On penetration: the deepest penetration, in pixels, treated as contact before the snap. A
/// quarter tile. (amendment #126 section 4)
pub const EMBED_TOLERANCE_PX: f64 = 8.0;

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
///
/// `solid_mask` is the per-pixel collision datum (one bit per tile pixel, 128 bytes for a 32x32
/// tile, bit `(row * w + col)`), emitted by the tile tool from the art. It is `Vec<u8>` rather than
/// `[u8; 128]` so a non-square tile still fits. Absent on tiles the tool never scanned.
#[derive(Clone, Debug)]
pub struct Tile {
    pub world_tile_x: i32,
    pub world_tile_y: i32,
    pub is_walkable: bool,
    pub solid_mask: Vec<u8>,
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

// The bridge and the Model share one named wire shape: a snapshot carries the world-pixel
// bounds next to the per-pixel-column surface Ys, so no caller unpacks a bare tuple.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroundProfileSnapshot {
    pub world_left_px: f64,
    pub world_right_px: f64,
    pub surface_y_by_column: Vec<Option<f64>>,
}

/// Build the per-pixel-column ground profile from tiles and their solid_masks.
///
/// In the returned snapshot, `surface_y_by_column[i]` is the surface Y at pixel column
/// `(world_left_px + i as f64)`, or `None` if that column is fully air (no walkable tile
/// with a solid pixel in that column).
///
/// For a tile with an empty `solid_mask`, the whole tile is treated as solid (legacy
/// framework tiles that predate the mask tool). For a tile with a mask, the topmost set
/// bit in each pixel column determines the surface; if no bit is set in a column, that
/// tile contributes no surface for that column.
// One named return type replaces the bare tuple, so the bounds travel with the profile.
fn build_ground_profile(tiles: &[Tile], grid: &GridConfig) -> GroundProfileSnapshot {
    let (world_left_px, world_right_px) = world_bounds_px(tiles, grid);
    let tile_w_px = grid.tile_w;
    let tile_h_px = grid.tile_h;
    let left_col = world_left_px.floor() as i32;
    // Cover every pixel column of every tile, so the last tile's right edge is included.
    let right_col = (world_right_px + tile_w_px).ceil() as i32;
    let width = (right_col - left_col).max(1) as usize;
    let mut profile: Vec<Option<f64>> = vec![None; width];

    for tile in tiles {
        if !tile.is_walkable {
            continue;
        }
        let tile_left_px = tile.world_tile_x as f64 * tile_w_px;
        let tile_top_px = tile.world_tile_y as f64 * tile_h_px;

        // Iterate over pixel columns this tile covers.
        let start_px = tile_left_px.floor() as i32;
        let end_px = (tile_left_px + tile_w_px).ceil() as i32;
        for px_col in start_px..end_px {
            let idx = (px_col - left_col) as usize;
            if idx >= profile.len() {
                continue;
            }
            // Determine surface for this pixel column from this tile.
            let surface_y = if tile.solid_mask.is_empty() {
                // Legacy tile: whole tile is solid.
                Some(tile_top_px)
            } else {
                // Masked tile: find topmost solid bit in this pixel column.
                let local_x = px_col - start_px;
                if local_x < 0 || local_x >= tile_w_px as i32 {
                    continue;
                }
                let local_x = local_x as u32;
                let tile_w = tile_w_px as u32;
                let tile_h = tile_h_px as u32;
                let mut top_row: Option<u32> = None;
                for row in 0..tile_h {
                    let bit_idx = row * tile_w + local_x;
                    let byte_idx = (bit_idx / 8) as usize;
                    let bit = bit_idx % 8;
                    if byte_idx < tile.solid_mask.len() {
                        if (tile.solid_mask[byte_idx] >> bit) & 1 != 0 {
                            top_row = Some(row);
                            break;
                        }
                    }
                }
                top_row.map(|r| tile_top_px + r as f64)
            };
            // Keep the highest (smallest Y) surface among overlapping tiles.
            match (profile[idx], surface_y) {
                (None, Some(y)) => {
                    profile[idx] = Some(y);
                }
                (Some(old), Some(y)) if y < old => {
                    profile[idx] = Some(y);
                }
                _ => {}
            }
        }
    }
    // Bundle the three values into the named snapshot; the two bounds describe the
    // pixel-column domain that `surface_y_by_column` indexes into.
    GroundProfileSnapshot {
        world_left_px,
        world_right_px,
        surface_y_by_column: profile,
    }
}

/// Sample the ground profile at a world pixel X position, with linear interpolation
/// between the pixel column and the next.
///
/// Returns `None` if the column is air (hole) or past the profile bounds.
// The snapshot owns its own origin, so callers pass one argument instead of a profile
// plus a separate left edge. The interpolation math itself does not change.
fn ground_y_at(snapshot: &GroundProfileSnapshot, pos_x: f64) -> Option<f64> {
    let profile = &snapshot.surface_y_by_column;
    let col = pos_x.floor() as i32;
    let left = snapshot.world_left_px.floor() as i32;
    let idx = col - left;
    if idx < 0 || idx >= profile.len() as i32 {
        return None;
    }
    let idx = idx as usize;
    let y0 = profile[idx]?;
    // If at the last column, clamp (clamps_right behaviour).
    if idx + 1 >= profile.len() {
        return Some(y0);
    }
    let y1 = profile[idx + 1]?;
    // Linear interpolation between this column and the next.
    let frac = pos_x - col as f64;
    Some(y0 + (y1 - y0) * frac)
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

    // The stepper keeps its own state machine; only the profile plumbing changes to the
    // named snapshot, whose bounds drive the clamp and whose sampler takes one argument.
    let snapshot = build_ground_profile(tiles, grid);
    let proposed_pos_x = (state.pos_x + move_speed_pps * frame_seconds)
        .clamp(snapshot.world_left_px, snapshot.world_right_px);

    // A pixel column holding no walkable solid content has no surface, so a grounded walker stepping
    // into it would fall exactly as if it were a pit. Refuse the step by staying on the previous pixel.
    // An ungrounded walker keeps moving (it is already falling), and a walker that does not move is
    // unaffected. Refusing rather than clamping to the column edge keeps this direction-agnostic:
    // the walker is at most one frame of travel from the boundary, so no visible gap opens.
    let stepping_into_hole =
        move_axis != 0.0 && state.grounded && ground_y_at(&snapshot, proposed_pos_x).is_none();
    let pos_x = if stepping_into_hole {
        state.pos_x
    } else {
        proposed_pos_x
    };

    // Surface at the walker's current pixel column (topmost solid bit of the overlapping tiles).
    let surface_y = ground_y_at(&snapshot, pos_x);

    let contact = match surface_y {
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
    } else if let Some(surface_y) = surface_y {
        // Anti-jitter (amendment #126 section 4): the walker's feet are below the surface.
        // Snap out if the penetration is deep enough; glue if within one tile above it.
        // Zero the offending velocity component (vy for a vertical surface), never gravity.
        if feet_now_y > surface_y + EMBED_TOLERANCE_PX {
            WalkerState {
                pos_x,
                pos_y: surface_y,
                vel_x: move_speed_pps,
                vel_y: 0.0,
                grounded: true,
                hp: state.hp,
                fall_start_y: state.fall_start_y,
            }
        } else if state.grounded
            && feet_now_y <= surface_y
            && surface_y - feet_now_y <= GROUND_SNAP_PX
        {
            // In contact, including stepping DOWN onto lower ground: glued to the surface.
            WalkerState {
                pos_x,
                pos_y: surface_y,
                vel_x: move_speed_pps,
                vel_y: 0.0,
                grounded: true,
                hp: state.hp,
                fall_start_y: surface_y,
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
                solid_mask: vec![],
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
                solid_mask: vec![],
            },
            Tile {
                world_tile_x: 1,
                world_tile_y: 1,
                is_walkable: true,
                solid_mask: vec![],
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
            solid_mask: vec![],
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
                solid_mask: vec![],
            },
            Tile {
                world_tile_x: 1,
                world_tile_y: TEST_GROUND_ROW - 1,
                is_walkable: true,
                solid_mask: vec![],
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
                solid_mask: vec![],
            },
            Tile {
                world_tile_x: 1,
                world_tile_y: TEST_GROUND_ROW,
                is_walkable: true,
                solid_mask: vec![],
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

    const TEST_MASK_TILE_PX: f64 = 32.0;

    fn test_mask_grid() -> GridConfig {
        GridConfig {
            tile_w: TEST_MASK_TILE_PX,
            tile_h: TEST_MASK_TILE_PX,
        }
    }

    /// Build a linear ramp tile: topmost solid row rises from `start_row` (leftmost column)
    /// to `end_row` (rightmost column), linearly. All other rows are air.
    fn ramp_tile(world_tile_x: i32, world_tile_y: i32, start_row: u32, end_row: u32) -> Tile {
        let mut mask = vec![0u8; (32 * 32 + 7) / 8];
        for col in 0..32u32 {
            let row = start_row + (end_row.saturating_sub(start_row) * col) / 31;
            let bit_idx = row * 32 + col;
            mask[(bit_idx / 8) as usize] |= 1 << (bit_idx % 8);
        }
        Tile {
            world_tile_x,
            world_tile_y,
            is_walkable: true,
            solid_mask: mask,
        }
    }

    /// Flat ground tile: every column solid at `surface_row`.
    fn flat_mask_tile(world_tile_x: i32, world_tile_y: i32, surface_row: u32) -> Tile {
        let mut mask = vec![0u8; (32 * 32 + 7) / 8];
        for col in 0..32u32 {
            let bit_idx = surface_row * 32 + col;
            mask[(bit_idx / 8) as usize] |= 1 << (bit_idx % 8);
        }
        Tile {
            world_tile_x,
            world_tile_y,
            is_walkable: true,
            solid_mask: mask,
        }
    }

    #[test]
    fn solid_mask_surface_rule() {
        let grid = test_mask_grid();
        // A tile at (1, row 2) with surface at row 10: topmost solid row = 10 for every column.
        let tiles = vec![flat_mask_tile(1, 2, 10)];
        let snapshot = build_ground_profile(&tiles, &grid);
        // Column under the tile's left edge (global pixel 32): local index = 0 because
        // world_left_px == tile_left_px == 32.
        // surface_y = world_tile_y * tile_h + top_row(PIXEL row) = 2 * 32 + 10 = 74.
        assert_eq!(snapshot.surface_y_by_column[0].unwrap(), 2.0 * 32.0 + 10.0);
    }

    #[test]
    fn interior_air_below_surface_is_solid() {
        // Legacy tile (empty mask) is all-solid; masked topmost bit rule tested elsewhere.
        let tiles = flat_ground(&[0, 1], TEST_GROUND_ROW);
        let grid = test_grid();
        let snapshot = build_ground_profile(&tiles, &grid);
        let profile = &snapshot.surface_y_by_column;
        // Column 0 is solid at row TEST_GROUND_ROW.
        assert_eq!(profile[0], Some(TEST_GROUND_ROW as f64 * TEST_TILE_PX));
        // Column 100 (inside second tile) also solid at same row.
        assert_eq!(profile[100], Some(TEST_GROUND_ROW as f64 * TEST_TILE_PX));
    }

    #[test]
    fn snapshot_builder_exposes_named_shape() {
        let grid = test_mask_grid();
        let tiles = vec![flat_mask_tile(1, 2, 10)];
        let snapshot = build_ground_profile(&tiles, &grid);
        assert_eq!(snapshot.world_left_px, TEST_MASK_TILE_PX as f64);
        // The right bound is the last tile's LEFT edge, not its right edge (today's contract).
        assert_eq!(snapshot.world_right_px, TEST_MASK_TILE_PX as f64);
        assert_eq!(
            snapshot.surface_y_by_column.len(),
            TEST_MASK_TILE_PX as usize
        );
        assert_eq!(snapshot.surface_y_by_column[0], Some(74.0));
    }

    #[test]
    fn snapshot_sampler_interpolates_between_columns() {
        let grid = test_mask_grid();
        let tiles = vec![ramp_tile(0, 0, 0, 31)];
        let snapshot = build_ground_profile(&tiles, &grid);
        // Column 0 tops out at row 0, column 1 at row 1; half a pixel in gives 0.5.
        assert_eq!(ground_y_at(&snapshot, 0.5), Some(0.5));
    }

    #[test]
    fn snapshot_sampler_reports_an_air_column_as_none() {
        let grid = test_mask_grid();
        // Tiles at world columns 0 and 2 leave world pixels 32..63 without a surface.
        let tiles = vec![flat_mask_tile(0, 0, 4), flat_mask_tile(2, 0, 4)];
        let snapshot = build_ground_profile(&tiles, &grid);
        assert_eq!(snapshot.surface_y_by_column[40], None);
        assert_eq!(ground_y_at(&snapshot, 40.5), None);
    }

    #[test]
    fn snapshot_sampler_ends_a_solid_run_at_a_gap() {
        // Today's Rust contract: a solid pixel whose next pixel is air samples as `None`.
        // GDScript holds the plateau at `world_view.gd:604-624`; aligning it needs its own ticket.
        let grid = test_mask_grid();
        let tiles = vec![flat_mask_tile(0, 0, 4), flat_mask_tile(2, 0, 4)];
        let snapshot = build_ground_profile(&tiles, &grid);
        assert_eq!(snapshot.surface_y_by_column[31], Some(4.0));
        assert_eq!(ground_y_at(&snapshot, 31.5), None);
    }

    #[test]
    fn snapshot_wire_shape_uses_null_for_air_columns() {
        let grid = test_mask_grid();
        let tiles = vec![flat_mask_tile(0, 0, 4), flat_mask_tile(2, 0, 4)];
        let snapshot = build_ground_profile(&tiles, &grid);
        let wire: serde_json::Value = serde_json::to_value(&snapshot).unwrap();
        // The first tile owns columns 0..32; the gap starts exactly one tile width in.
        assert_eq!(wire["surface_y_by_column"][0], serde_json::Value::from(4.0));
        assert!(wire["surface_y_by_column"][TEST_MASK_TILE_PX as usize].is_null());
        let decoded: GroundProfileSnapshot = serde_json::from_value(wire).unwrap();
        assert_eq!(decoded, snapshot);
    }
}
