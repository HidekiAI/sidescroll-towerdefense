use serde::{Deserialize, Serialize};

pub const GRAVITY_PPS2: f64 = 980.0;
pub const DEFAULT_AVATAR_SPEED_PPS: f64 = 200.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalkerState {
    pub pos_x: f64,
    pub pos_y: f64,
    pub vel_x: f64,
    pub vel_y: f64,
    pub grounded: bool,
    pub hp: f64,
    pub fall_start_y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalkerInput {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridConfig {
    pub tile_w: f64,
    pub tile_h: f64,
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub world_tile_x: i32,
    pub world_tile_y: i32,
    pub is_walkable: bool,
}

pub fn ground_profile_px(tiles: &[Tile], grid: &GridConfig) -> Vec<Option<f64>> {
    let mut by_col: std::collections::BTreeMap<i32, i32> = std::collections::BTreeMap::new();
    for t in tiles {
        if t.is_walkable {
            by_col
                .entry(t.world_tile_x)
                .and_modify(|e| {
                    if t.world_tile_y < *e {
                        *e = t.world_tile_y;
                    }
                })
                .or_insert(t.world_tile_y);
        }
    }
    // Return sparse representation not required; caller can query by column
    // but we provide a helper via a function that can be used by building array
    // For simplicity, return empty and rely on profile lookup by column
    let _ = by_col;
    Vec::new()
}

pub fn ground_y_for_column(tiles: &[Tile], grid: &GridConfig, col: i32) -> Option<f64> {
    let mut best: Option<i32> = None;
    for t in tiles {
        if t.world_tile_x == col && t.is_walkable {
            match best {
                None => best = Some(t.world_tile_y),
                Some(b) => {
                    if t.world_tile_y < b {
                        best = Some(t.world_tile_y);
                    }
                }
            }
        }
    }
    match best {
        Some(row) => Some((row as f64) * grid.tile_h),
        None => None,
    }
}

pub fn step_walker(
    mut state: WalkerState,
    input: &WalkerInput,
    dt: f64,
    grid: &GridConfig,
    tiles: &[Tile],
    speed_pps: f64,
) -> WalkerState {
    let dt = if dt <= 0.0 { 1.0 / 60.0 } else { dt };
    let mx = input.move_x.max(-1.0).min(1.0);
    state.vel_x = mx * speed_pps;

    // vertical gravity
    state.vel_y += GRAVITY_PPS2 * dt;

    // propose new position
    let mut min_x: f64 = 0.0;
    let mut max_x: f64 = grid.tile_w;
    let mut first = true;
    for t in tiles {
        let wx = t.world_tile_x as f64 * grid.tile_w;
        if first {
            min_x = wx;
            max_x = wx;
            first = false;
        } else {
            min_x = f64::min(min_x, wx);
            max_x = f64::max(max_x, wx);
        }
    }
    let mut next_x = (state.pos_x + state.vel_x * dt).clamp(min_x, max_x);
    let feet_was_y = state.pos_y;
    let next_vel_y = state.vel_y;
    let feet_now_y = feet_was_y + next_vel_y * dt;

    // world bounds from tiles for column clamping
    let last_world_col = tiles.iter().map(|t| t.world_tile_x).max().unwrap_or(0);
    let mut world_col = ((next_x / grid.tile_w).floor() as i32).clamp(0, last_world_col);
    // "Blocked by non-walkable": a column holding only non-walkable tiles has no walkable surface,
    // so a grounded walker that steps into it would fall exactly as if it were a pit. Refuse the
    // step instead, staying on the previous pixel. An ungrounded walker keeps moving (it is
    // already falling), and a walker that does not move is unaffected.
    if mx != 0.0 && state.grounded && ground_y_for_column(tiles, grid, world_col).is_none() {
        next_x = state.pos_x;
        world_col = ((next_x / grid.tile_w).floor() as i32).clamp(0, last_world_col);
    }

    let mut hp = state.hp;
    let mut fall_start_y = state.fall_start_y;
    let mut landed = false;
    let mut ground_y = feet_now_y;

    if let Some(surface_y) = ground_y_for_column(tiles, grid, world_col) {
        ground_y = surface_y;
        // Landing from above onto ground (moving down onto surface)
        if next_vel_y >= 0.0 && feet_was_y <= ground_y && feet_now_y >= ground_y {
            landed = true;
        }
        // Stepping UP onto higher ground (walking up a hill) — only when moving horizontally
        else if ground_y < feet_was_y && mx != 0.0 {
            landed = true;
            fall_start_y = ground_y;
        }
    } else {
        // No ground in this column: hole. Track where the fall began if we were standing.
        if state.grounded {
            fall_start_y = feet_was_y;
        }
    }

    if landed {
        // Fall damage: if we fell from a height above this surface, take 1pt per landing > 32px drop
        if !state.grounded && ground_y > fall_start_y {
            let fall_dist = ground_y - fall_start_y;
            if fall_dist > 32.0 {
                hp = (hp - 1.0).max(0.0);
            }
        }
        state.pos_y = ground_y;
        state.vel_y = 0.0;
        state.grounded = true;
        state.fall_start_y = ground_y;
    } else {
        state.pos_y = feet_now_y;
        state.vel_y = next_vel_y;
        state.grounded = false;
        state.fall_start_y = fall_start_y;
    }

    state.pos_x = next_x;
    state.vel_x = mx * speed_pps;
    state.hp = hp;
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_profile_finds_top_walkable() {
        let grid = GridConfig {
            tile_w: 64.0,
            tile_h: 64.0,
        };
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
        assert_eq!(ground_y_for_column(&tiles, &grid, 1), Some(64.0));
    }

    #[test]
    fn ground_profile_none_for_void() {
        let grid = GridConfig {
            tile_w: 64.0,
            tile_h: 64.0,
        };
        let tiles = vec![Tile {
            world_tile_x: 1,
            world_tile_y: 1,
            is_walkable: false,
        }];
        assert_eq!(ground_y_for_column(&tiles, &grid, 1), None);
    }

    #[test]
    fn walker_snaps_to_ground() {
        let grid = GridConfig {
            tile_w: 64.0,
            tile_h: 64.0,
        };
        let tiles = vec![Tile {
            world_tile_x: 0,
            world_tile_y: 1,
            is_walkable: true,
        }];
        let mut state = WalkerState {
            pos_x: 10.0,
            pos_y: 10.0,
            vel_x: 0.0,
            vel_y: 500.0,
            grounded: false,
            hp: 100.0,
            fall_start_y: 10.0,
        };
        let input = WalkerInput {
            move_x: 0.0,
            move_y: 0.0,
            move_to_world: None,
            scroll_vec: None,
            place: false,
            alt_place: false,
            interact: false,
            zoom_in: false,
            zoom_out: false,
        };
        let mut out = step_walker(state, &input, 1.0 / 60.0, &grid, &tiles, 200.0);
        for _ in 0..60 {
            out = step_walker(out, &input, 1.0 / 60.0, &grid, &tiles, 200.0);
        }
        assert!(out.grounded);
        assert!(out.pos_y >= 0.0);
    }

    #[test]
    fn walker_blocked_by_missing_surface() {
        let grid = GridConfig {
            tile_w: 64.0,
            tile_h: 64.0,
        };
        // Columns 0-2 and 4-5 stand on row 1. Column 3 holds no walkable tile, so a grounded walker
        // that stepped into it would drop exactly as if it were a pit.
        let tiles: Vec<Tile> = [0, 1, 2, 4, 5]
            .iter()
            .map(|column| Tile {
                world_tile_x: *column,
                world_tile_y: 1,
                is_walkable: true,
            })
            .collect();
        let input = WalkerInput {
            move_x: 1.0,
            move_y: 0.0,
            move_to_world: None,
            scroll_vec: None,
            place: false,
            alt_place: false,
            interact: false,
            zoom_in: false,
            zoom_out: false,
        };
        let mut out = WalkerState {
            pos_x: 10.0,
            pos_y: 64.0,
            vel_x: 0.0,
            vel_y: 0.0,
            grounded: true,
            hp: 100.0,
            fall_start_y: 64.0,
        };
        for _ in 0..300 {
            out = step_walker(out, &input, 1.0 / 60.0, &grid, &tiles, 200.0);
        }
        assert!(
            out.pos_x < 3.0 * grid.tile_w,
            "walker entered the surface-less column: {}",
            out.pos_x
        );
        assert!(out.grounded, "walker lost the ground while blocked");
        assert_eq!(out.pos_y, 64.0);
        let blocked_x = out.pos_x;
        for _ in 0..300 {
            out = step_walker(out, &input, 1.0 / 60.0, &grid, &tiles, 200.0);
        }
        assert_eq!(out.pos_x, blocked_x, "walker crept through the block");
    }
}
