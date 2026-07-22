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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TerrainTypeDef {
    pub key: String,
    pub display_name: String,
    pub is_walkable: bool,
    pub is_buildable: bool,
    pub surface: String,
    pub hazard: String,
    pub elevation_tiles: i32,
    pub color_hex: String,
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

        if SurfaceType::from_str(&self.surface).is_none() {
            result = result.with_message(
                ValidationMessage::warning(
                    "TERRAIN_UNKNOWN_SURFACE",
                    format!("Unknown surface type: {}", self.surface),
                )
                .with_field("surface")
                .with_value(&self.surface),
            );
        }

        if HazardType::from_str(&self.hazard).is_none() {
            result = result.with_message(
                ValidationMessage::warning(
                    "TERRAIN_UNKNOWN_HAZARD",
                    format!("Unknown hazard type: {}", self.hazard),
                )
                .with_field("hazard")
                .with_value(&self.hazard),
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

        result
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
