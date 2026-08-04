use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{ValidationMessage, ValidationResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainType {
    Ground,
    Wall,
    Air,
    Water,
    Lava,
}

impl TerrainType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ground" => Some(TerrainType::Ground),
            "wall" => Some(TerrainType::Wall),
            "air" => Some(TerrainType::Air),
            "water" => Some(TerrainType::Water),
            "lava" => Some(TerrainType::Lava),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TerrainType::Ground => "ground",
            TerrainType::Wall => "wall",
            TerrainType::Air => "air",
            TerrainType::Water => "water",
            TerrainType::Lava => "lava",
        }
    }

    pub fn is_walkable(self) -> bool {
        match self {
            TerrainType::Ground => true,
            TerrainType::Wall => false,
            TerrainType::Air => false,
            TerrainType::Water => true,
            TerrainType::Lava => false,
        }
    }

    pub fn is_buildable(self) -> bool {
        match self {
            TerrainType::Ground => true,
            TerrainType::Wall => false,
            TerrainType::Air => false,
            TerrainType::Water => false,
            TerrainType::Lava => false,
        }
    }

    pub fn has_inherent_hazard(self) -> bool {
        match self {
            TerrainType::Lava => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceType {
    Normal,
    Ice,
    Mud,
}

impl SurfaceType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(SurfaceType::Normal),
            "ice" => Some(SurfaceType::Ice),
            "mud" => Some(SurfaceType::Mud),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SurfaceType::Normal => "normal",
            SurfaceType::Ice => "ice",
            SurfaceType::Mud => "mud",
        }
    }

    pub fn speed_modifier(self) -> f64 {
        match self {
            SurfaceType::Normal => 1.0,
            SurfaceType::Ice => 1.2,
            SurfaceType::Mud => 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HazardType {
    None,
    Lava,
}

impl HazardType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "none" => Some(HazardType::None),
            "lava" => Some(HazardType::Lava),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            HazardType::None => "none",
            HazardType::Lava => "lava",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneType {
    Normal,
    NoBuild,
    SafeZone,
    SpawnZone,
    PvpZone,
}

impl ZoneType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(ZoneType::Normal),
            "no_build" => Some(ZoneType::NoBuild),
            "safe_zone" => Some(ZoneType::SafeZone),
            "spawn_zone" => Some(ZoneType::SpawnZone),
            "pvp_zone" => Some(ZoneType::PvpZone),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileAffectingEntityType {
    Bridge,
    DestructibleWall,
    TrapDoor,
    Spikes,
    Conveyor,
    Tarpit,
    IcePatch,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TerrainTypeDef {
    pub key: String,
    pub display_name: String,
    pub is_walkable: bool,
    pub is_buildable: bool,
    pub surface: SurfaceType,
    pub hazard: HazardType,
    pub elevation_tiles: i32,
    pub color_hex: String,
    #[serde(default = "default_sub_tile_mask")]
    pub sub_tile_mask: u8,
    #[serde(default)]
    pub hazard_damage_per_tick: i32,
    #[serde(default)]
    pub is_destructible: bool,
    #[serde(default)]
    pub destructible_hp: i32,
    #[serde(default)]
    pub on_destroy_terrain_key: String,
}

fn default_sub_tile_mask() -> u8 {
    0xF
}

impl TerrainTypeDef {
    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::valid();

        if self.key.is_empty() {
            result = result.with_message(
                ValidationMessage::error("TERRAIN_EMPTY_KEY", "Terrain key must not be empty")
                    .with_field("key"),
            );
        }

        if self.color_hex.len() != 7 || !self.color_hex.starts_with('#') {
            result = result.with_message(
                ValidationMessage::error(
                    "TERRAIN_INVALID_COLOR",
                    "Color must be a hex string like #RRGGBB",
                )
                .with_field("color_hex")
                .with_value(&self.color_hex),
            );
        }

        if self.sub_tile_mask > 0xF {
            result = result.with_message(
                ValidationMessage::warning(
                    "TERRAIN_SUBTILE_MASK_INVALID",
                    "Sub-tile mask must be 0x0..0xF, got {}. Clamping to 0xF."
                        .replace("{}", &self.sub_tile_mask.to_string()),
                )
                .with_field("sub_tile_mask")
                .with_value(&self.sub_tile_mask.to_string()),
            );
        }

        if self.is_destructible && self.destructible_hp <= 0 {
            result = result.with_message(
                ValidationMessage::error(
                    "TERRAIN_DESTRUCTIBLE_HP_ZERO",
                    "Destructible terrain must have positive HP",
                )
                .with_field("destructible_hp")
                .with_value(&self.destructible_hp.to_string()),
            );
        }

        if self.is_destructible && self.on_destroy_terrain_key.is_empty() {
            result = result.with_message(
                ValidationMessage::warning(
                    "TERRAIN_DESTRUCTIBLE_NO_FALLBACK",
                    format!(
                        "Destructible terrain '{}' has no on_destroy_terrain_key. \
                         Falling back to 'air'.",
                        self.key
                    ),
                )
                .with_field("on_destroy_terrain_key"),
            );
        }

        result
    }
}

impl Default for TerrainTypeDef {
    fn default() -> Self {
        Self {
            key: String::new(),
            display_name: String::new(),
            is_walkable: false,
            is_buildable: false,
            surface: SurfaceType::Normal,
            hazard: HazardType::None,
            elevation_tiles: 0,
            color_hex: "#888888".into(),
            sub_tile_mask: 0xF,
            hazard_damage_per_tick: 0,
            is_destructible: false,
            destructible_hp: 0,
            on_destroy_terrain_key: String::new(),
        }
    }
}

/// Sub-tile mask helper functions.
pub fn sub_tile_mask_is_set(mask: u8, index: u8) -> bool {
    (mask >> index) & 1 == 1
}

/// Apply horizontal flip to a sub-tile mask (swap TL↔TR, BL↔BR).
pub fn sub_tile_mask_flip_x(mask: u8) -> u8 {
    let tl = (mask >> 0) & 1;
    let tr = (mask >> 1) & 1;
    let bl = (mask >> 2) & 1;
    let br = (mask >> 3) & 1;
    (tr << 0) | (tl << 1) | (br << 2) | (bl << 3)
}

/// Apply vertical flip to a sub-tile mask (swap TL↔BL, TR↔BR).
pub fn sub_tile_mask_flip_y(mask: u8) -> u8 {
    let tl = (mask >> 0) & 1;
    let tr = (mask >> 1) & 1;
    let bl = (mask >> 2) & 1;
    let br = (mask >> 3) & 1;
    (bl << 0) | (br << 1) | (tl << 2) | (tr << 3)
}

/// Sub-tile quadrant indices.
pub const SUB_TL: u8 = 0;
pub const SUB_TR: u8 = 1;
pub const SUB_BL: u8 = 2;
pub const SUB_BR: u8 = 3;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TileSetEntry {
    pub local_x: i32,
    pub local_y: i32,
    pub terrain_key: String,
    #[serde(default)]
    pub elevation_tiles: i32,
    #[serde(default)]
    pub z_depth: i32,
    #[serde(default = "default_sub_tile_mask")]
    pub sub_tile_mask: u8,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TileSet {
    pub key: String,
    pub display_name: String,
    pub width_tiles: i32,
    pub height_tiles: i32,
    #[serde(default)]
    pub tiles: Vec<TileSetEntry>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl TileSet {
    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::valid();

        if self.key.is_empty() {
            result = result.with_message(
                ValidationMessage::error("TILESET_EMPTY_KEY", "TileSet key must not be empty")
                    .with_field("key"),
            );
        }

        if self.width_tiles <= 0 || self.height_tiles <= 0 {
            result = result.with_message(
                ValidationMessage::error(
                    "TILESET_INVALID_DIMS",
                    format!(
                        "TileSet '{}' dimensions {}x{} must be positive",
                        self.key, self.width_tiles, self.height_tiles
                    ),
                )
                .with_field("width_tiles"),
            );
        }

        for entry in &self.tiles {
            if entry.local_x < 0 || entry.local_x >= self.width_tiles {
                result = result.with_message(
                    ValidationMessage::error(
                        "TILESET_ENTRY_OUT_OF_BOUNDS",
                        format!(
                            "TileSet '{}' entry ({}, {}) exceeds bounds {}x{}",
                            self.key,
                            entry.local_x,
                            entry.local_y,
                            self.width_tiles,
                            self.height_tiles
                        ),
                    )
                    .with_field("tiles"),
                );
            }
            if entry.local_y < 0 || entry.local_y >= self.height_tiles {
                result = result.with_message(
                    ValidationMessage::error(
                        "TILESET_ENTRY_OUT_OF_BOUNDS",
                        format!(
                            "TileSet '{}' entry ({}, {}) exceeds bounds {}x{}",
                            self.key,
                            entry.local_x,
                            entry.local_y,
                            self.width_tiles,
                            self.height_tiles
                        ),
                    )
                    .with_field("tiles"),
                );
            }
            if entry.terrain_key.is_empty() {
                result = result.with_message(
                    ValidationMessage::error(
                        "TILESET_ENTRY_EMPTY_KEY",
                        format!(
                            "TileSet '{}' entry ({}, {}) has empty terrain_key",
                            self.key, entry.local_x, entry.local_y
                        ),
                    )
                    .with_field("terrain_key"),
                );
            }
        }

        let has_duplicates = {
            let mut seen = Vec::new();
            self.tiles.iter().any(|e| {
                if seen.contains(&(e.local_x, e.local_y)) {
                    true
                } else {
                    seen.push((e.local_x, e.local_y));
                    false
                }
            })
        };
        if has_duplicates {
            result = result.with_message(
                ValidationMessage::error(
                    "TILESET_DUPLICATE_ENTRY",
                    format!("TileSet '{}' has duplicate local positions", self.key),
                )
                .with_field("tiles"),
            );
        }

        result
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerrainTile {
    pub x: i32,
    pub y: i32,
    pub terrain_type: String,
    pub elevation_tiles: i32,
    pub is_walkable: bool,
    pub is_buildable: bool,
    pub surface: String,
    pub hazard: String,
    pub sub_tile_mask: u8,
    pub hazard_damage_per_tick: i32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub z_depth: i32,
    pub zone: String,
    pub zone_rules: Option<String>,
}

impl TerrainTile {
    pub fn resolved_terrain_type(&self) -> Option<TerrainType> {
        TerrainType::from_str(&self.terrain_type)
    }

    pub fn resolved_surface(&self) -> Option<SurfaceType> {
        SurfaceType::from_str(&self.surface)
    }

    pub fn resolved_hazard(&self) -> Option<HazardType> {
        HazardType::from_str(&self.hazard)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GridConfig {
    pub max_tiles_per_screen_x: i32,
    pub max_tiles_per_screen_y: i32,
    pub tile_width_in_pixels: i32,
    pub tile_height_in_pixels: i32,
    pub max_viewport_pixels_x: i32,
    pub max_viewport_pixels_y: i32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            max_tiles_per_screen_x: 30,
            max_tiles_per_screen_y: 16,
            tile_width_in_pixels: 64,
            tile_height_in_pixels: 64,
            max_viewport_pixels_x: 1920,
            max_viewport_pixels_y: 1080,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimeConfig {
    pub tick_rate: f64,
    pub time_scale: f64,
}

impl Default for TimeConfig {
    fn default() -> Self {
        Self {
            tick_rate: 60.0,
            time_scale: 1.0,
        }
    }
}

impl TimeConfig {
    pub fn speed_per_tick(&self, speed_pps: f64) -> f64 {
        speed_pps * self.time_scale / self.tick_rate
    }

    pub fn cooldown_per_tick(&self, cooldown_seconds: f64) -> f64 {
        cooldown_seconds * self.tick_rate / self.time_scale
    }

    pub fn duration_per_tick(&self, duration_seconds: f64) -> f64 {
        duration_seconds * self.tick_rate / self.time_scale
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiomeType {
    Grassland,
    Cave,
    Dungeon,
    Volcano,
    Crystal,
    Factory,
}

impl BiomeType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "grassland" => Some(BiomeType::Grassland),
            "cave" => Some(BiomeType::Cave),
            "dungeon" => Some(BiomeType::Dungeon),
            "volcano" => Some(BiomeType::Volcano),
            "crystal" => Some(BiomeType::Crystal),
            "factory" => Some(BiomeType::Factory),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnershipType {
    Player1,
    Player2,
    Enemy,
    Neutral,
}

impl OwnershipType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "player.1" => Some(OwnershipType::Player1),
            "player.2" => Some(OwnershipType::Player2),
            "enemy" => Some(OwnershipType::Enemy),
            "neutral" => Some(OwnershipType::Neutral),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityStateType {
    Active,
    Disabled,
    Destroyed,
}

impl EntityStateType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(EntityStateType::Active),
            "disabled" => Some(EntityStateType::Disabled),
            "destroyed" => Some(EntityStateType::Destroyed),
            _ => None,
        }
    }
}
