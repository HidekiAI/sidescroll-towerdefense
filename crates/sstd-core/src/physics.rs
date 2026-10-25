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
    // horizontal
    let mut mx = input.move_x;
    let my = input.move_y;
    // click-to-move simple: if move_to_world set, point toward it horizontally if grounded-like
    if let Some((tx, ty)) = input.move_to_world {
        let dx = tx - state.pos_x;
        let dy = ty - state.pos_y;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 1.0 {
            mx = if dx.abs() > 0.001 { dx.signum() } else { 0.0 };
            // vertical not directly driven by click in this slice; gravity handles
            let _ = dy;
            let _ = my;
        } else {
            mx = 0.0;
        }
    }
    mx = mx.max(-1.0).min(1.0);
    state.vel_x = mx * speed_pps;

    // vertical gravity
    state.vel_y += GRAVITY_PPS2 * dt;

    // propose
    let mut nx = state.pos_x + state.vel_x * dt;
    let mut ny = state.pos_y + state.vel_y * dt;

    // wall/block horizontally by checking target column tiles
    let col_nx = ((nx / grid.tile_w).floor() as i32);
    let row_ny = ((ny / grid.tile_h).floor() as i32);
    // block if target tile not walkable
    let mut blocked_x = false;
    for t in tiles {
        if t.world_tile_x == col_nx && t.world_tile_y == row_ny && !t.is_walkable {
            blocked_x = true;
            break;
        }
    }
    if blocked_x {
        nx = state.pos_x;
        state.vel_x = 0.0;
    }

    // vertical: ground snap
    let feet_col = ((nx / grid.tile_w).floor() as i32);
    match ground_y_for_column(tiles, grid, feet_col) {
        Some(gy) => {
            // ground is at gy (top of walkable tile); avatar feet snap to gy
            if ny >= gy - 1.0 {
                ny = gy;
                state.vel_y = 0.0;
                state.grounded = true;
            } else {
                state.grounded = false;
            }
        }
        None => {
            state.grounded = false;
            // fall through void
        }
    }

    // vertical wall check
    let col_nx2 = ((nx / grid.tile_w).floor() as i32);
    let row_ny2 = ((ny / grid.tile_h).floor() as i32);
    let mut blocked_y = false;
    for t in tiles {
        if t.world_tile_x == col_nx2 && t.world_tile_y == row_ny2 && !t.is_walkable {
            blocked_y = true;
            break;
        }
    }
    if blocked_y {
        ny = state.pos_y;
        state.vel_y = 0.0;
    }

    state.pos_x = nx;
    state.pos_y = ny;
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
}
