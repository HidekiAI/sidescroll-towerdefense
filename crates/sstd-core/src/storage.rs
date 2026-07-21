use serde::{Deserialize, Serialize};

use crate::error::{SstdResult, StorageError, ValidationResult};
use crate::map::{Screen, TileEntry};
use crate::terrain::TerrainTypeDef;

pub const CURRENT_SCHEMA_VERSION: &str = "0.1.0";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ScreenFile {
    pub version: String,
    pub screen_id: i32,
    pub width_tiles: i32,
    pub height_tiles: i32,
    pub elevation_floor_tiles: i32,
    pub elevation_ceiling_tiles: i32,
    pub tiles: Vec<ScreenTileEntry>,
    pub placed_entities: Vec<ScreenPlacedEntity>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ScreenTileEntry {
    pub x: i32,
    pub y: i32,
    pub terrain: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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

    #[test]
    fn test_terrain_types_json_roundtrip() {
        let file = TerrainTypesFile {
            version: CURRENT_SCHEMA_VERSION.to_string(),
            tiles: vec![TerrainTypeDef {
                key: "grass".into(),
                display_name: "Grass".into(),
                is_walkable: true,
                is_buildable: true,
                surface: "normal".into(),
                hazard: "none".into(),
                elevation_tiles: 0,
                color_hex: "#4a7c3f".into(),
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
                class: "tower".into(),
                width_tiles: 1.0,
                height_tiles: 1.5,
                max_hp: 500,
                speed_pps: 0,
                attack_range_tiles: 5,
                attack_power: 100,
                element: "physical".into(),
                action_cooldown_ticks: 60,
                projectile_type: "arrow".into(),
                requires_ground: true,
                requires_ceiling: false,
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
            elevation_floor_tiles: 0,
            elevation_ceiling_tiles: 4,
            tiles: vec![ScreenTileEntry {
                x: 0,
                y: 0,
                terrain: "grass".into(),
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
                    surface: "normal".into(),
                    hazard: "none".into(),
                    elevation_tiles: 0,
                    color_hex: "#4a7c3f".into(),
                },
                TerrainTypeDef {
                    key: "grass".into(),
                    display_name: "Grass Alt".into(),
                    is_walkable: true,
                    is_buildable: true,
                    surface: "normal".into(),
                    hazard: "none".into(),
                    elevation_tiles: 0,
                    color_hex: "#3f7c4a".into(),
                },
            ],
        };
        let result = file.validate();
        assert!(result.has_errors());
    }
}
