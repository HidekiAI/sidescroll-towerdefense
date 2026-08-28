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
    #[serde(
        default = "default_sub_tile_mask",
        deserialize_with = "deserialize_sub_tile_mask"
    )]
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

// Godot's JSON.stringify serializes float Variants as `15.0` (see
// ~/Documents/opencode/godot-rust-json-float-serde-u8.md). Such integral
// floats must not hard-fail a u8 field at load; accept an integral float
// (fractional part == 0) and cast to u8. Non-integral or out-of-range values
// remain hard errors and are surfaced by validation.
fn deserialize_sub_tile_mask<'de, D>(de: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct SubTileMaskVisitor;
    impl<'de> serde::de::Visitor<'de> for SubTileMaskVisitor {
        type Value = u8;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an integer 0x0..0xFF, or an integral float like 15.0")
        }

        fn visit_u8<E>(self, v: u8) -> Result<u8, E>
        where
            E: serde::de::Error,
        {
            Ok(v)
        }

        fn visit_u64<E>(self, v: u64) -> Result<u8, E>
        where
            E: serde::de::Error,
        {
            u8::try_from(v).map_err(|_| E::custom(format!("sub_tile_mask out of u8 range: {v}")))
        }

        fn visit_i64<E>(self, v: i64) -> Result<u8, E>
        where
            E: serde::de::Error,
        {
            u8::try_from(v).map_err(|_| E::custom(format!("sub_tile_mask out of u8 range: {v}")))
        }

        fn visit_f64<E>(self, v: f64) -> Result<u8, E>
        where
            E: serde::de::Error,
        {
            if v.fract() != 0.0 {
                return Err(E::custom(format!(
                    "sub_tile_mask float must be integral, got {v}"
                )));
            }
            u8::try_from(v as i64)
                .map_err(|_| E::custom(format!("sub_tile_mask out of u8 range: {v}")))
        }
    }
    de.deserialize_any(SubTileMaskVisitor)
}

// Option variant of `deserialize_sub_tile_mask` (used by TerrainTypeOverride:
// nothing, or an integer / integral float).
fn deserialize_sub_tile_mask_option<'de, D>(de: D) -> Result<Option<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct OptVisitor;
    impl<'de> serde::de::Visitor<'de> for OptVisitor {
        type Value = Option<u8>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an optional integer 0x0..0xFF, or an integral float")
        }

        fn visit_none<E>(self) -> Result<Option<u8>, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Option<u8>, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D2>(self, de: D2) -> Result<Option<u8>, D2::Error>
        where
            D2: serde::Deserializer<'de>,
        {
            deserialize_sub_tile_mask(de).map(Some)
        }
    }
    de.deserialize_option(OptVisitor)
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
    #[serde(
        default = "default_sub_tile_mask",
        deserialize_with = "deserialize_sub_tile_mask"
    )]
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

/// Prototype-based terrain override: every field is optional.
/// A world zip may contain `terrain_overrides.json` with partial patches keyed
/// by terrain key.  The engine merges each override onto the matching
/// framework prototype, keeping unspecified fields from the prototype.
///
/// Schema verification: `#[serde(deny_unknown_fields)]` rejects typos
/// (`colour_hex`), `Option<T>` types enforce exact type matches, and
/// enum types (`SurfaceType`, `HazardType`) reject unknown values at parse
/// time.
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct TerrainTypeOverride {
    pub key: Option<String>,
    pub display_name: Option<String>,
    pub is_walkable: Option<bool>,
    pub is_buildable: Option<bool>,
    pub surface: Option<SurfaceType>,
    pub hazard: Option<HazardType>,
    pub elevation_tiles: Option<i32>,
    pub color_hex: Option<String>,
    #[serde(default, deserialize_with = "deserialize_sub_tile_mask_option")]
    pub sub_tile_mask: Option<u8>,
    pub hazard_damage_per_tick: Option<i32>,
    pub is_destructible: Option<bool>,
    pub destructible_hp: Option<i32>,
    pub on_destroy_terrain_key: Option<String>,
}

impl TerrainTypeOverride {
    /// Merge this override onto a framework prototype, returning a new
    /// `TerrainTypeDef` where specified fields override the prototype and
    /// unspecified fields keep their prototype values.
    pub fn merge(&self, prototype: &TerrainTypeDef) -> TerrainTypeDef {
        TerrainTypeDef {
            key: self.key.clone().unwrap_or_else(|| prototype.key.clone()),
            display_name: self
                .display_name
                .clone()
                .unwrap_or_else(|| prototype.display_name.clone()),
            is_walkable: self.is_walkable.unwrap_or(prototype.is_walkable),
            is_buildable: self.is_buildable.unwrap_or(prototype.is_buildable),
            surface: self.surface.unwrap_or(prototype.surface),
            hazard: self.hazard.unwrap_or(prototype.hazard),
            elevation_tiles: self.elevation_tiles.unwrap_or(prototype.elevation_tiles),
            color_hex: self
                .color_hex
                .clone()
                .unwrap_or_else(|| prototype.color_hex.clone()),
            sub_tile_mask: self.sub_tile_mask.unwrap_or(prototype.sub_tile_mask),
            hazard_damage_per_tick: self
                .hazard_damage_per_tick
                .unwrap_or(prototype.hazard_damage_per_tick),
            is_destructible: self.is_destructible.unwrap_or(prototype.is_destructible),
            destructible_hp: self.destructible_hp.unwrap_or(prototype.destructible_hp),
            on_destroy_terrain_key: self
                .on_destroy_terrain_key
                .clone()
                .unwrap_or_else(|| prototype.on_destroy_terrain_key.clone()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn grass_prototype() -> TerrainTypeDef {
        TerrainTypeDef {
            key: "grass".into(),
            display_name: "Grass".into(),
            is_walkable: true,
            is_buildable: true,
            surface: SurfaceType::Normal,
            hazard: HazardType::None,
            elevation_tiles: 0,
            color_hex: "#4a7c3f".into(),
            sub_tile_mask: 0xF,
            hazard_damage_per_tick: 0,
            is_destructible: false,
            destructible_hp: 0,
            on_destroy_terrain_key: String::new(),
        }
    }

    #[test]
    fn terrain_override_merge_inherits_unspecified_fields() {
        let proto = grass_prototype();
        // Override only color + walkability; everything else must inherit.
        let ov = TerrainTypeOverride {
            key: Some("grass".into()),
            display_name: None,
            is_walkable: None,
            is_buildable: Some(false),
            surface: None,
            hazard: None,
            elevation_tiles: None,
            color_hex: Some("#8a8a8a".into()),
            sub_tile_mask: None,
            hazard_damage_per_tick: None,
            is_destructible: None,
            destructible_hp: None,
            on_destroy_terrain_key: None,
        };
        let merged = ov.merge(&proto);
        assert_eq!(merged.color_hex, "#8a8a8a");
        assert!(!merged.is_buildable);
        // Unspecified fields inherit from the prototype.
        assert_eq!(merged.key, "grass");
        assert_eq!(merged.display_name, "Grass");
        assert!(merged.is_walkable);
        assert_eq!(merged.surface, SurfaceType::Normal);
        assert_eq!(merged.hazard, HazardType::None);
        assert_eq!(merged.sub_tile_mask, 0xF);
    }

    #[test]
    fn terrain_override_emerges_lava_hazard() {
        let proto = grass_prototype();
        let ov = TerrainTypeOverride {
            key: None, // key comes from the prototype when unspecified
            display_name: Some("Lava".into()),
            is_walkable: None,
            is_buildable: None,
            surface: None,
            hazard: Some(HazardType::Lava),
            elevation_tiles: None,
            color_hex: Some("#e34a1f".into()),
            sub_tile_mask: None,
            hazard_damage_per_tick: Some(15),
            is_destructible: None,
            destructible_hp: None,
            on_destroy_terrain_key: None,
        };
        let merged = ov.merge(&proto);
        assert_eq!(merged.hazard, HazardType::Lava);
        assert_eq!(merged.hazard_damage_per_tick, 15);
        assert_eq!(merged.color_hex, "#e34a1f");
        // Key inherited from prototype when Some-less.
        assert_eq!(merged.key, "grass");
    }

    // Godot's JSON.stringify writes float-typed Variants as `15.0`; the reader
    // must accept an integral float for a u8 sub_tile_mask instead of failing
    // (see docs/PLAN-2026-08-27-terrain-brush-and-subtile-float.md).
    #[test]
    fn override_deserializes_integral_float_sub_tile_mask() {
        let json = r#"{"key":"air","sub_tile_mask":15.0}"#;
        let ov: TerrainTypeOverride = serde_json::from_str(json).unwrap();
        assert_eq!(ov.sub_tile_mask, Some(15));
    }

    #[test]
    fn override_deserializes_integer_sub_tile_mask() {
        let json = r#"{"key":"dirt","sub_tile_mask":0}"#;
        let ov: TerrainTypeOverride = serde_json::from_str(json).unwrap();
        assert_eq!(ov.sub_tile_mask, Some(0));
    }

    #[test]
    fn override_rejects_non_integral_float_sub_tile_mask() {
        let json = r#"{"sub_tile_mask":15.5}"#;
        let err = serde_json::from_str::<TerrainTypeOverride>(json);
        assert!(err.is_err(), "non-integral float must be rejected: {err:?}");
    }

    #[test]
    fn type_def_deserializes_integral_float_sub_tile_mask() {
        let json = "{\"key\":\"air\",\"display_name\":\"Air\",\"is_walkable\":true,\"is_buildable\":false,\"surface\":\"normal\",\"hazard\":\"none\",\"elevation_tiles\":0,\"color_hex\":\"#111111\",\"sub_tile_mask\":15.0}";
        let def: TerrainTypeDef = serde_json::from_str(json).unwrap();
        assert_eq!(def.sub_tile_mask, 15);
    }
}
