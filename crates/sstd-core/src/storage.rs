use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{SstdResult, StorageError, ValidationResult};
use crate::map::{Screen, TileEntry};
#[cfg(test)]
use crate::terrain::TileSetEntry;
use crate::terrain::{TerrainTypeDef, TileSet};

pub const CURRENT_SCHEMA_VERSION: &str = "0.1.0";

/// Deserialize an optional integer that tolerates legacy editors' JSON
/// formatting (integral floats such as `15.0`). Strings, non-integral numbers,
/// and out-of-range values are rejected — no silent fallback.
fn de_opt_i32_flex<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;
    match v {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if (i32::MIN as i64..=i32::MAX as i64).contains(&i) {
                    return Ok(Some(i as i32));
                }
            }
            match n.as_f64() {
                Some(f) if f.fract() == 0.0 && f >= i32::MIN as f64 && f <= i32::MAX as f64 => {
                    Ok(Some(f as i32))
                }
                _ => Err(serde::de::Error::custom(
                    "expected integer (or integral number) within i32 range",
                )),
            }
        }
        _ => Err(serde::de::Error::custom("expected integer or null")),
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TerrainTypesFile {
    pub version: String,
    pub tiles: Vec<TerrainTypeDef>,
}

impl TerrainTypesFile {
    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::valid();

        if self.version != CURRENT_SCHEMA_VERSION {
            result = result.with_message(
                crate::error::ValidationMessage::warning(
                    "VERSION_MISMATCH",
                    format!(
                        "Expected version {}, got {}",
                        CURRENT_SCHEMA_VERSION, self.version
                    ),
                )
                .with_field("version")
                .with_value(&self.version),
            );
        }

        for tile in &self.tiles {
            result = result.merge(tile.validate());
        }

        let has_duplicates = {
            let mut keys: Vec<&str> = self.tiles.iter().map(|t| t.key.as_str()).collect();
            keys.sort();
            keys.windows(2).any(|w| w[0] == w[1])
        };

        if has_duplicates {
            result = result.with_message(
                crate::error::ValidationMessage::error(
                    "TERRAIN_DUPLICATE_KEY",
                    "Duplicate terrain keys found",
                )
                .with_field("tiles"),
            );
        }

        result
    }

    pub fn from_json(json: &str) -> SstdResult<Self> {
        let parsed: TerrainTypesFile =
            serde_json::from_str(json).map_err(|e| StorageError::Parse(e.to_string()))?;

        let validation = parsed.validate();
        if validation.has_errors() {
            return Err(StorageError::Validation(validation));
        }

        Ok(parsed)
    }

    pub fn to_json(&self) -> SstdResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| StorageError::Io(e.to_string()))
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EntityDefsFile {
    pub version: String,
    pub entities: Vec<crate::entity::EntityDefEditor>,
}

impl EntityDefsFile {
    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::valid();

        if self.version != CURRENT_SCHEMA_VERSION {
            result = result.with_message(
                crate::error::ValidationMessage::warning(
                    "VERSION_MISMATCH",
                    format!(
                        "Expected version {}, got {}",
                        CURRENT_SCHEMA_VERSION, self.version
                    ),
                )
                .with_field("version")
                .with_value(&self.version),
            );
        }

        for entity in &self.entities {
            result = result.merge(entity.validate());
        }

        let has_duplicates = {
            let mut keys: Vec<&str> = self.entities.iter().map(|e| e.key.as_str()).collect();
            keys.sort();
            keys.windows(2).any(|w| w[0] == w[1])
        };

        if has_duplicates {
            result = result.with_message(
                crate::error::ValidationMessage::error(
                    "ENTITY_DUPLICATE_KEY",
                    "Duplicate entity keys found",
                )
                .with_field("entities"),
            );
        }

        result
    }

    pub fn from_json(json: &str) -> SstdResult<Self> {
        let parsed: EntityDefsFile =
            serde_json::from_str(json).map_err(|e| StorageError::Parse(e.to_string()))?;

        let validation = parsed.validate();
        if validation.has_errors() {
            return Err(StorageError::Validation(validation));
        }

        Ok(parsed)
    }

    pub fn to_json(&self) -> SstdResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| StorageError::Io(e.to_string()))
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScreenFile {
    pub version: String,
    pub screen_id: i32,
    pub width_tiles: i32,
    pub height_tiles: i32,
    pub tile_width_px: i32,
    pub tile_height_px: i32,
    pub elevation_floor_tiles: i32,
    pub elevation_ceiling_tiles: i32,
    pub tiles: Vec<ScreenTileEntry>,
    pub placed_entities: Vec<ScreenPlacedEntity>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ScreenTileEntry {
    pub x: i32,
    pub y: i32,
    pub terrain: String,
    /// Opaque extension bag written by the Map editor (contains `sub_tile_mask`,
    /// `elevation_tiles`, `z_depth`, `flip_h`/`flip_v`, `source_image`, `source_rect`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile_data: Option<serde_json::Value>,
    /// Placement-editor flattened metadata. Accepts legacy integral-float formatting.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_i32_flex"
    )]
    pub sub_tile_mask: Option<i32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_i32_flex"
    )]
    pub elevation_tiles: Option<i32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_i32_flex"
    )]
    pub z_depth: Option<i32>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScreenPlacedEntity {
    pub entity_key: String,
    pub world_tile_x: i32,
    pub world_tile_y: i32,
}

impl ScreenFile {
    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::valid();

        if self.version != CURRENT_SCHEMA_VERSION {
            result = result.with_message(
                crate::error::ValidationMessage::warning(
                    "VERSION_MISMATCH",
                    format!(
                        "Expected version {}, got {}",
                        CURRENT_SCHEMA_VERSION, self.version
                    ),
                )
                .with_field("version")
                .with_value(&self.version),
            );
        }

        let default_grid = crate::terrain::GridConfig::default();
        if self.tile_width_px != default_grid.tile_width_in_pixels
            || self.tile_height_px != default_grid.tile_height_in_pixels
        {
            result = result.with_message(
                crate::error::ValidationMessage::warning(
                    "TILE_DIMENSION_MISMATCH",
                    format!(
                        "Saved tile dimensions {}x{} differ from current {}x{}. Porting recommended.",
                        self.tile_width_px, self.tile_height_px,
                        default_grid.tile_width_in_pixels, default_grid.tile_height_in_pixels,
                    ),
                )
                .with_field("tile_width_px")
                .with_value(&format!("{}x{}", self.tile_width_px, self.tile_height_px)),
            );
        }

        let screen = Screen {
            screen_id: self.screen_id,
            width_tiles: self.width_tiles,
            height_tiles: self.height_tiles,
            elevation_floor_tiles: self.elevation_floor_tiles,
            elevation_ceiling_tiles: self.elevation_ceiling_tiles,
        };

        result = result.merge(screen.validate());

        for tile in &self.tiles {
            let entry = TileEntry {
                x: tile.x,
                y: tile.y,
                terrain: tile.terrain.clone(),
            };
            result = result.merge(entry.validate(&screen));
        }

        let has_duplicate_tiles = {
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

        if has_duplicate_tiles {
            result = result.with_message(
                crate::error::ValidationMessage::error(
                    "SCREEN_DUPLICATE_TILE",
                    "Screen contains duplicate tile positions",
                )
                .with_field("tiles"),
            );
        }

        result
    }

    pub fn from_json(json: &str) -> SstdResult<Self> {
        let parsed: ScreenFile =
            serde_json::from_str(json).map_err(|e| StorageError::Parse(e.to_string()))?;

        let validation = parsed.validate();
        if validation.has_errors() {
            return Err(StorageError::Validation(validation));
        }

        Ok(parsed)
    }

    pub fn to_json(&self) -> SstdResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| StorageError::Io(e.to_string()))
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TileSetsFile {
    pub version: String,
    #[serde(default)]
    pub tile_sets: Vec<TileSet>,
}

impl TileSetsFile {
    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::valid();

        if self.version != CURRENT_SCHEMA_VERSION {
            result = result.with_message(
                crate::error::ValidationMessage::warning(
                    "VERSION_MISMATCH",
                    format!(
                        "Expected version {}, got {}",
                        CURRENT_SCHEMA_VERSION, self.version
                    ),
                )
                .with_field("version")
                .with_value(&self.version),
            );
        }

        for set in &self.tile_sets {
            result = result.merge(set.validate());
        }

        let has_duplicates = {
            let mut keys: Vec<&str> = self.tile_sets.iter().map(|s| s.key.as_str()).collect();
            keys.sort();
            keys.windows(2).any(|w| w[0] == w[1])
        };

        if has_duplicates {
            result = result.with_message(
                crate::error::ValidationMessage::error(
                    "TILESET_DUPLICATE_KEY",
                    "Duplicate TileSet keys found",
                )
                .with_field("tile_sets"),
            );
        }

        result
    }

    pub fn from_json(json: &str) -> SstdResult<Self> {
        let parsed: TileSetsFile =
            serde_json::from_str(json).map_err(|e| StorageError::Parse(e.to_string()))?;

        let validation = parsed.validate();
        if validation.has_errors() {
            return Err(StorageError::Validation(validation));
        }

        Ok(parsed)
    }

    pub fn to_json(&self) -> SstdResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| StorageError::Io(e.to_string()))
    }
}

pub fn check_schema_version(version: &str) -> SstdResult<()> {
    if version != CURRENT_SCHEMA_VERSION {
        Err(StorageError::UnsupportedVersion {
            found: version.to_string(),
            expected: CURRENT_SCHEMA_VERSION.to_string(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::{HazardType, SurfaceType};

    #[test]
    fn test_terrain_types_json_roundtrip() {
        let file = TerrainTypesFile {
            version: CURRENT_SCHEMA_VERSION.to_string(),
            tiles: vec![TerrainTypeDef {
                key: "grass".into(),
                display_name: "Grass".into(),
                is_walkable: true,
                is_buildable: true,
                surface: SurfaceType::Normal,
                hazard: HazardType::None,
                elevation_tiles: 0,
                color_hex: "#4a7c3f".into(),
                ..Default::default()
            }],
        };

        let json = file.to_json().unwrap();
        let parsed = TerrainTypesFile::from_json(&json).unwrap();
        assert_eq!(file, parsed);
    }

    #[test]
    fn test_entity_defs_json_roundtrip() {
        let file = EntityDefsFile {
            version: CURRENT_SCHEMA_VERSION.to_string(),
            entities: vec![crate::entity::EntityDefEditor {
                key: "arrow_tower".into(),
                class: crate::entity::EntityClass::Stationary(
                    crate::entity::StationarySubClass::Tower,
                ),
                width_tiles: 1.0,
                height_tiles: 1.5,
                max_hp: 500,
                speed_pps: 0,
                attack_range_tiles: 5,
                attack_power: 100,
                element: crate::entity::Element::Physical,
                action_cooldown_ticks: 60,
                projectile_type: "arrow".into(),
                requires_ground: true,
                requires_ceiling: false,
                weight: 0.0,
                max_weight: 0.0,
            }],
        };

        let json = file.to_json().unwrap();
        let parsed = EntityDefsFile::from_json(&json).unwrap();
        assert_eq!(file, parsed);
    }

    #[test]
    fn test_screen_file_json_roundtrip() {
        let file = ScreenFile {
            version: CURRENT_SCHEMA_VERSION.to_string(),
            screen_id: 1,
            width_tiles: 30,
            height_tiles: 16,
            tile_width_px: 64,
            tile_height_px: 64,
            elevation_floor_tiles: 0,
            elevation_ceiling_tiles: 4,
            tiles: vec![ScreenTileEntry {
                x: 0,
                y: 0,
                terrain: "grass".into(),
                ..Default::default()
            }],
            placed_entities: vec![ScreenPlacedEntity {
                entity_key: "arrow_tower".into(),
                world_tile_x: 5,
                world_tile_y: 3,
            }],
        };

        let json = file.to_json().unwrap();
        let parsed = ScreenFile::from_json(&json).unwrap();
        assert_eq!(file, parsed);
    }

    #[test]
    fn test_tile_sets_json_roundtrip() {
        let file = TileSetsFile {
            version: CURRENT_SCHEMA_VERSION.to_string(),
            tile_sets: vec![TileSet {
                key: "slope_30deg_4x2".into(),
                display_name: "30° Slope 4×2".into(),
                width_tiles: 4,
                height_tiles: 2,
                tiles: vec![
                    TileSetEntry {
                        local_x: 0,
                        local_y: 0,
                        terrain_key: "grass".into(),
                        elevation_tiles: 0,
                        z_depth: 0,
                        sub_tile_mask: 0xF,
                    },
                    TileSetEntry {
                        local_x: 1,
                        local_y: 0,
                        terrain_key: "grass".into(),
                        elevation_tiles: 0,
                        z_depth: 0,
                        sub_tile_mask: 0xA,
                    },
                    TileSetEntry {
                        local_x: 2,
                        local_y: 0,
                        terrain_key: "grass".into(),
                        elevation_tiles: 1,
                        z_depth: 0,
                        sub_tile_mask: 0x5,
                    },
                    TileSetEntry {
                        local_x: 3,
                        local_y: 0,
                        terrain_key: "grass".into(),
                        elevation_tiles: 1,
                        z_depth: 0,
                        sub_tile_mask: 0xF,
                    },
                    TileSetEntry {
                        local_x: 0,
                        local_y: 1,
                        terrain_key: "air".into(),
                        elevation_tiles: 0,
                        z_depth: -1,
                        sub_tile_mask: 0x0,
                    },
                    TileSetEntry {
                        local_x: 1,
                        local_y: 1,
                        terrain_key: "air".into(),
                        elevation_tiles: 0,
                        z_depth: -1,
                        sub_tile_mask: 0x0,
                    },
                    TileSetEntry {
                        local_x: 2,
                        local_y: 1,
                        terrain_key: "air".into(),
                        elevation_tiles: 0,
                        z_depth: -1,
                        sub_tile_mask: 0x0,
                    },
                    TileSetEntry {
                        local_x: 3,
                        local_y: 1,
                        terrain_key: "wall".into(),
                        elevation_tiles: 0,
                        z_depth: 0,
                        sub_tile_mask: 0xF,
                    },
                ],
                tags: vec!["slope".into(), "grassland".into()],
            }],
        };

        let json = file.to_json().unwrap();
        let parsed = TileSetsFile::from_json(&json).unwrap();
        assert_eq!(file, parsed);
    }

    #[test]
    fn test_invalid_schema_version() {
        let file = TerrainTypesFile {
            version: "999.0.0".to_string(),
            tiles: vec![],
        };
        let result = file.validate();
        assert!(result.messages.iter().any(|m| m.code == "VERSION_MISMATCH"));
        assert!(!result.has_errors()); // version mismatch is a warning, not error
    }

    #[test]
    fn test_duplicate_terrain_key_detected() {
        let file = TerrainTypesFile {
            version: CURRENT_SCHEMA_VERSION.to_string(),
            tiles: vec![
                TerrainTypeDef {
                    key: "grass".into(),
                    display_name: "Grass".into(),
                    is_walkable: true,
                    is_buildable: true,
                    surface: SurfaceType::Normal,
                    hazard: HazardType::None,
                    elevation_tiles: 0,
                    color_hex: "#4a7c3f".into(),
                    ..Default::default()
                },
                TerrainTypeDef {
                    key: "grass".into(),
                    display_name: "Grass Alt".into(),
                    is_walkable: true,
                    is_buildable: true,
                    surface: SurfaceType::Normal,
                    hazard: HazardType::None,
                    elevation_tiles: 0,
                    color_hex: "#3f7c4a".into(),
                    ..Default::default()
                },
            ],
        };
        let result = file.validate();
        assert!(result.has_errors());
    }

    #[test]
    fn test_unknown_key_rejected() {
        let json = serde_json::json!({
            "version": CURRENT_SCHEMA_VERSION,
            "tiles": [],
            "tilez": [],
        });
        let parsed = TerrainTypesFile::from_json(&serde_json::to_string(&json).unwrap());
        assert!(parsed.is_err(), "typoed key must hard-fail");
    }

    #[test]
    fn test_missing_required_field_rejected() {
        let json = serde_json::json!({
            "version": CURRENT_SCHEMA_VERSION,
            "tiles": [{ "key": "grass" }],
        });
        let parsed = TerrainTypesFile::from_json(&serde_json::to_string(&json).unwrap());
        assert!(parsed.is_err(), "missing required field must hard-fail");
    }

    #[test]
    fn test_wrong_case_surface_rejected() {
        let json = serde_json::json!({
            "version": CURRENT_SCHEMA_VERSION,
            "tiles": [{
                "key": "grass", "display_name": "Grass",
                "is_walkable": true, "is_buildable": true,
                "surface": "Normal", "hazard": "none",
                "elevation_tiles": 0, "color_hex": "#4a7c3f",
            }],
        });
        let parsed = TerrainTypesFile::from_json(&serde_json::to_string(&json).unwrap());
        assert!(parsed.is_err(), "mis-cased enum value must hard-fail");
    }

    #[test]
    fn test_unknown_entity_class_rejected() {
        let json = serde_json::json!({
            "version": CURRENT_SCHEMA_VERSION,
            "entities": [{
                "key": "x", "class": "tyrannosaurus",
                "width_tiles": 1.0, "height_tiles": 1.0,
                "max_hp": 1, "speed_pps": 0, "attack_range_tiles": 0,
                "attack_power": 0, "element": "physical",
                "action_cooldown_ticks": 0, "projectile_type": "",
                "requires_ground": false, "requires_ceiling": false,
            }],
        });
        let parsed = EntityDefsFile::from_json(&serde_json::to_string(&json).unwrap());
        assert!(parsed.is_err(), "unknown entity class must hard-fail");
    }

    #[test]
    fn test_screen_editor_shapes_parse() {
        let json = serde_json::json!({
            "version": CURRENT_SCHEMA_VERSION,
            "screen_id": 1,
            "width_tiles": 30, "height_tiles": 16,
            "tile_width_px": 64, "tile_height_px": 64,
            "elevation_floor_tiles": 0, "elevation_ceiling_tiles": 4,
            "tiles": [
                { "x": 0, "y": 0, "terrain": "grass",
                  "tile_data": { "sub_tile_mask": 15, "flip_h": false } },
                { "x": 1, "y": 0, "terrain": "grass",
                  "sub_tile_mask": 15.0, "elevation_tiles": 0.0, "z_depth": 0.0 }
            ],
            "placed_entities": [],
        });
        let parsed = ScreenFile::from_json(&serde_json::to_string(&json).unwrap()).unwrap();
        assert_eq!(
            parsed.tiles[0].tile_data.as_ref().unwrap()["sub_tile_mask"],
            15
        );
        assert_eq!(parsed.tiles[1].sub_tile_mask, Some(15));
        assert_eq!(parsed.tiles[1].elevation_tiles, Some(0));
        assert_eq!(parsed.tiles[1].z_depth, Some(0));
    }

    #[test]
    fn test_screen_tile_unknown_key_rejected() {
        let json = serde_json::json!({
            "version": CURRENT_SCHEMA_VERSION,
            "screen_id": 1,
            "width_tiles": 30, "height_tiles": 16,
            "tile_width_px": 64, "tile_height_px": 64,
            "elevation_floor_tiles": 0, "elevation_ceiling_tiles": 4,
            "tiles": [{ "x": 0, "y": 0, "terrain": "grass", "terrain2": "dirt" }],
            "placed_entities": [],
        });
        let parsed = ScreenFile::from_json(&serde_json::to_string(&json).unwrap());
        assert!(parsed.is_err(), "unknown tile key must hard-fail");
    }

    #[test]
    fn test_world_coordinates_can_be_negative() {
        let json = serde_json::json!({
            "version": CURRENT_SCHEMA_VERSION,
            "screen_id": 2,
            "width_tiles": 30, "height_tiles": 16,
            "tile_width_px": 64, "tile_height_px": 64,
            "elevation_floor_tiles": 0, "elevation_ceiling_tiles": 4,
            "tiles": [],
            "placed_entities": [
                { "entity_key": "mine", "world_tile_x": -25, "world_tile_y": 3 }
            ],
        });
        let parsed = ScreenFile::from_json(&serde_json::to_string(&json).unwrap()).unwrap();
        assert_eq!(parsed.placed_entities[0].world_tile_x, -25);
    }
}
