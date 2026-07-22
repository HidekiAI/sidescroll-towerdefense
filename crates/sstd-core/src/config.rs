use std::path::Path;

use rusqlite::Connection;

use crate::entity::EntityLimits;
use crate::error::SstdResult;
use crate::terrain::{GridConfig, TimeConfig};

pub const DEFAULT_CONFIG_DB_NAME: &str = "sstd_config.db";

pub struct ConfigStore {
    conn: Connection,
}

impl ConfigStore {
    pub fn open_or_create(path: &Path) -> SstdResult<Self> {
        let conn =
            Connection::open(path).map_err(|e| crate::error::StorageError::Io(e.to_string()))?;

        let store = Self { conn };
        store.ensure_schema()?;
        store.seed_defaults()?;
        Ok(store)
    }

    pub fn in_memory() -> SstdResult<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;

        let store = Self { conn };
        store.ensure_schema()?;
        store.seed_defaults()?;
        Ok(store)
    }

    fn ensure_schema(&self) -> SstdResult<()> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS config (
                    key             TEXT PRIMARY KEY,
                    default_value   TEXT NOT NULL,
                    description_id  TEXT
                );

                CREATE TABLE IF NOT EXISTS meta (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', '0.1.0');",
            )
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))
    }

    fn seed_defaults(&self) -> SstdResult<()> {
        let grid = GridConfig::default();
        let time = TimeConfig::default();
        let limits = EntityLimits::default();

        let entries: [(&str, &str); 12] = [
            ("grid.max_tiles_per_screen_x", "30"),
            ("grid.max_tiles_per_screen_y", "16"),
            ("grid.tile_width_in_pixels", "64"),
            ("grid.tile_height_in_pixels", "64"),
            ("grid.max_viewport_pixels_x", "1920"),
            ("grid.max_viewport_pixels_y", "1080"),
            ("time.tick_rate", "60"),
            ("time.time_scale", "1"),
            ("entity.max_per_world", "500"),
            ("entity.max_on_screen", "50"),
            ("entity.max_process_per_frame", "30"),
            ("entity.max_off_screen_process", "10"),
        ];

        for (key, value) in &entries {
            self.conn
                .execute(
                    "INSERT OR IGNORE INTO config (key, default_value) VALUES (?1, ?2)",
                    rusqlite::params![key, value],
                )
                .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;
        }

        // Validate that struct defaults match the DB values
        assert_eq!(grid.max_tiles_per_screen_x, 30);
        assert_eq!(grid.max_tiles_per_screen_y, 16);
        assert_eq!(grid.tile_width_in_pixels, 64);
        assert_eq!(grid.tile_height_in_pixels, 64);
        assert_eq!(grid.max_viewport_pixels_x, 1920);
        assert_eq!(grid.max_viewport_pixels_y, 1080);
        assert!((time.tick_rate - 60.0).abs() < f64::EPSILON);
        assert!((time.time_scale - 1.0).abs() < f64::EPSILON);
        assert_eq!(limits.max_entities_per_world, 500);
        assert_eq!(limits.max_entities_on_screen, 50);
        assert_eq!(limits.max_process_per_frame, 30);
        assert_eq!(limits.max_off_screen_process, 10);

        Ok(())
    }

    fn get_str(&self, key: &str) -> SstdResult<String> {
        self.conn
            .query_row(
                "SELECT default_value FROM config WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get::<_, String>(0),
            )
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))
    }

    pub fn grid_config(&self) -> SstdResult<GridConfig> {
        Ok(GridConfig {
            max_tiles_per_screen_x: self
                .get_str("grid.max_tiles_per_screen_x")?
                .parse()
                .unwrap_or(30),
            max_tiles_per_screen_y: self
                .get_str("grid.max_tiles_per_screen_y")?
                .parse()
                .unwrap_or(16),
            tile_width_in_pixels: self
                .get_str("grid.tile_width_in_pixels")?
                .parse()
                .unwrap_or(64),
            tile_height_in_pixels: self
                .get_str("grid.tile_height_in_pixels")?
                .parse()
                .unwrap_or(64),
            max_viewport_pixels_x: self
                .get_str("grid.max_viewport_pixels_x")?
                .parse()
                .unwrap_or(1920),
            max_viewport_pixels_y: self
                .get_str("grid.max_viewport_pixels_y")?
                .parse()
                .unwrap_or(1080),
        })
    }

    pub fn time_config(&self) -> SstdResult<TimeConfig> {
        Ok(TimeConfig {
            tick_rate: self.get_str("time.tick_rate")?.parse().unwrap_or(60.0),
            time_scale: self.get_str("time.time_scale")?.parse().unwrap_or(1.0),
        })
    }

    pub fn entity_limits(&self) -> SstdResult<EntityLimits> {
        Ok(EntityLimits {
            max_entities_per_world: self.get_str("entity.max_per_world")?.parse().unwrap_or(500),
            max_entities_on_screen: self.get_str("entity.max_on_screen")?.parse().unwrap_or(50),
            max_process_per_frame: self
                .get_str("entity.max_process_per_frame")?
                .parse()
                .unwrap_or(30),
            max_off_screen_process: self
                .get_str("entity.max_off_screen_process")?
                .parse()
                .unwrap_or(10),
        })
    }

    pub fn set_config(&self, key: &str, value: &str) -> SstdResult<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO config (key, default_value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            )
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn all_config(&self) -> SstdResult<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, default_value FROM config ORDER BY key")
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| crate::error::StorageError::Io(e.to_string()))?);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_store_in_memory() {
        let store = ConfigStore::in_memory().unwrap();
        let configs = store.all_config().unwrap();
        assert_eq!(configs.len(), 12);
    }

    #[test]
    fn test_grid_config_from_db() {
        let store = ConfigStore::in_memory().unwrap();
        let grid = store.grid_config().unwrap();
        assert_eq!(grid.max_tiles_per_screen_x, 30);
        assert_eq!(grid.tile_width_in_pixels, 64);
    }

    #[test]
    fn test_time_config_from_db() {
        let store = ConfigStore::in_memory().unwrap();
        let time = store.time_config().unwrap();
        assert!((time.tick_rate - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_entity_limits_from_db() {
        let store = ConfigStore::in_memory().unwrap();
        let limits = store.entity_limits().unwrap();
        assert_eq!(limits.max_entities_per_world, 500);
        assert_eq!(limits.max_entities_on_screen, 50);
    }

    #[test]
    fn test_set_and_get_config() {
        let store = ConfigStore::in_memory().unwrap();
        store.set_config("test.key", "42").unwrap();
        let val = store.get_str("test.key").unwrap();
        assert_eq!(val, "42");
    }

    #[test]
    fn test_seed_does_not_overwrite() {
        let store = ConfigStore::in_memory().unwrap();
        store
            .set_config("grid.max_tiles_per_screen_x", "99")
            .unwrap();
        // Re-seed (should not overwrite existing)
        store.seed_defaults().unwrap();
        let val = store.get_str("grid.max_tiles_per_screen_x").unwrap();
        assert_eq!(val, "99");
    }
}
