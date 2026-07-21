use crate::error::{ValidationMessage, ValidationResult};

#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    pub screen_id: i32,
    pub width_tiles: i32,
    pub height_tiles: i32,
    pub elevation_floor_tiles: i32,
    pub elevation_ceiling_tiles: i32,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            screen_id: 0,
            width_tiles: 30,
            height_tiles: 16,
            elevation_floor_tiles: 0,
            elevation_ceiling_tiles: 4,
        }
    }
}

impl Screen {
    pub fn tile_count(&self) -> usize {
        (self.width_tiles * self.height_tiles) as usize
    }

    pub fn is_valid_tile(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width_tiles && y >= 0 && y < self.height_tiles
    }

    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::valid();

        if self.width_tiles <= 0 {
            result = result.with_message(
                ValidationMessage::error("SCREEN_INVALID_WIDTH", "Screen width must be positive")
                    .with_field("width_tiles")
                    .with_value(&self.width_tiles.to_string()),
            );
        }

        if self.height_tiles <= 0 {
            result = result.with_message(
                ValidationMessage::error("SCREEN_INVALID_HEIGHT", "Screen height must be positive")
                    .with_field("height_tiles")
                    .with_value(&self.height_tiles.to_string()),
            );
        }

        result
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TileEntry {
    pub x: i32,
    pub y: i32,
    pub terrain: String,
}

impl TileEntry {
    pub fn validate(&self, screen: &Screen) -> ValidationResult {
        let mut result = ValidationResult::valid();

        if !screen.is_valid_tile(self.x, self.y) {
            result = result.with_message(
                ValidationMessage::error(
                    "TILE_OUT_OF_BOUNDS",
                    format!("Tile ({}, {}) is out of screen bounds", self.x, self.y),
                )
                .with_field("tiles"),
            );
        }

        if self.terrain.is_empty() {
            result = result.with_message(
                ValidationMessage::error("TILE_EMPTY_TERRAIN", "Terrain key must not be empty")
                    .with_field("terrain")
                    .with_value(&format!("({}, {})", self.x, self.y)),
            );
        }

        result
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedEntity {
    pub id: Option<i64>,
    pub screen_id: i32,
    pub entity_key: String,
    pub tile_x: i32,
    pub tile_y: f64,
    pub rotation: f64,
    pub properties: Option<String>,
}

impl PlacedEntity {
    pub fn validate(&self, screen: &Screen) -> ValidationResult {
        let mut result = ValidationResult::valid();

        if self.entity_key.is_empty() {
            result = result.with_message(
                ValidationMessage::error("PLACEMENT_EMPTY_KEY", "Entity key must not be empty")
                    .with_field("entity_key"),
            );
        }

        let tile_y_int = self.tile_y as i32;
        if !screen.is_valid_tile(self.tile_x, tile_y_int) {
            result = result.with_message(
                ValidationMessage::error(
                    "PLACEMENT_OUT_OF_BOUNDS",
                    format!(
                        "Placement ({}, {}) is out of screen bounds",
                        self.tile_x, self.tile_y
                    ),
                )
                .with_field("placement"),
            );
        }

        result
    }

    pub fn occupied_tile_keys(&self, width_tiles: f64, height_tiles: f64) -> Vec<(i32, i32)> {
        let w = width_tiles.ceil() as i32;
        let h = height_tiles.ceil() as i32;
        let base_y = self.tile_y.floor() as i32;

        let mut tiles = Vec::with_capacity((w * h) as usize);
        for dx in 0..w {
            for dy in 0..h {
                tiles.push((self.tile_x + dx, base_y + dy));
            }
        }
        tiles
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TileGrid {
    pub tiles: Vec<TileEntry>,
    pub screen: Screen,
}

impl TileGrid {
    pub fn new(tiles: Vec<TileEntry>, screen: Screen) -> Self {
        Self { tiles, screen }
    }

    pub fn get(&self, x: i32, y: i32) -> Option<&TileEntry> {
        self.tiles.iter().find(|t| t.x == x && t.y == y)
    }

    pub fn terrain_at(&self, x: i32, y: i32) -> Option<&str> {
        self.get(x, y).map(|t| t.terrain.as_str())
    }

    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::valid();

        result = result.merge(self.screen.validate());

        for tile in &self.tiles {
            result = result.merge(tile.validate(&self.screen));
        }

        let has_duplicates = {
            let mut seen = Vec::with_capacity(self.tiles.len());
            self.tiles.iter().any(|t| {
                if seen.contains(&(t.x, t.y)) {
                    true
                } else {
                    seen.push((t.x, t.y));
                    false
                }
            })
        };

        if has_duplicates {
            result = result.with_message(
                ValidationMessage::error(
                    "TILE_DUPLICATE",
                    "Tile grid contains duplicate positions",
                )
                .with_field("tiles"),
            );
        }

        result
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClearanceResult {
    pub is_clear: bool,
    pub blockers: Vec<String>,
}

impl ClearanceResult {
    pub fn clear() -> Self {
        Self {
            is_clear: true,
            blockers: Vec::new(),
        }
    }

    pub fn blocked(reason: impl Into<String>) -> Self {
        Self {
            is_clear: false,
            blockers: vec![reason.into()],
        }
    }
}

pub fn tile_to_world(screen_id: i32, local_tile_x: i32, max_tiles_per_screen_x: i32) -> i32 {
    screen_id * max_tiles_per_screen_x + local_tile_x
}

pub fn world_to_screen(world_tile_x: i32, max_tiles_per_screen_x: i32) -> (i32, i32) {
    let screen_id = world_tile_x / max_tiles_per_screen_x;
    let local_tile_x = world_tile_x % max_tiles_per_screen_x;
    (screen_id, local_tile_x)
}

pub fn pixel_to_tile(world_pixel_x: f64, tile_width_in_pixels: i32) -> (i32, f64) {
    let tw = tile_width_in_pixels as f64;
    let tile_x = (world_pixel_x / tw).floor() as i32;
    let offset = world_pixel_x % tw;
    (tile_x, offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_to_world_roundtrip() {
        let result = tile_to_world(2, 5, 30);
        assert_eq!(result, 65);

        let (screen_id, local_x) = world_to_screen(65, 30);
        assert_eq!(screen_id, 2);
        assert_eq!(local_x, 5);
    }

    #[test]
    fn test_screen_valid_tile() {
        let screen = Screen::default();
        assert!(screen.is_valid_tile(0, 0));
        assert!(screen.is_valid_tile(29, 15));
        assert!(!screen.is_valid_tile(30, 0));
        assert!(!screen.is_valid_tile(0, 16));
        assert!(!screen.is_valid_tile(-1, 0));
    }

    #[test]
    fn test_tile_grid_get() {
        let screen = Screen::default();
        let tiles = vec![TileEntry {
            x: 5,
            y: 3,
            terrain: "grass".into(),
        }];
        let grid = TileGrid::new(tiles, screen);

        assert_eq!(grid.terrain_at(5, 3), Some("grass"));
        assert_eq!(grid.terrain_at(0, 0), None);
    }

    #[test]
    fn test_tile_grid_duplicate_detection() {
        let screen = Screen::default();
        let tiles = vec![
            TileEntry {
                x: 5,
                y: 3,
                terrain: "grass".into(),
            },
            TileEntry {
                x: 5,
                y: 3,
                terrain: "dirt".into(),
            },
        ];
        let grid = TileGrid::new(tiles, screen);
        let result = grid.validate();
        assert!(result.has_errors());
    }

    #[test]
    fn test_placed_entity_occupied_tiles() {
        let entity = PlacedEntity {
            id: None,
            screen_id: 0,
            entity_key: "arrow_tower".into(),
            tile_x: 5,
            tile_y: 3.0,
            rotation: 0.0,
            properties: None,
        };

        let tiles = entity.occupied_tile_keys(1.0, 1.5);
        assert!(tiles.contains(&(5, 3)));
        assert!(tiles.contains(&(5, 4))); // fractional height rounds up
        assert_eq!(tiles.len(), 2);
    }
}
