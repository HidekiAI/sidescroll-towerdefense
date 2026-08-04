use std::path::Path;

use rusqlite::Connection;

use crate::entity::EntityLimits;
use crate::error::SstdResult;
use crate::terrain::{GridConfig, TimeConfig};

pub const DEFAULT_CONFIG_DB_NAME: &str = "sstd_config.sqlite3";

pub struct ConfigStore {
    conn: Connection,
}

type PopulateFn = fn(&Connection) -> SstdResult<()>;

struct Population {
    id: &'static str,
    description: &'static str,
    func: PopulateFn,
}

const POPULATIONS: &[Population] = &[Population {
    id: "001",
    description: "Initial core config: grid, time, entity limits",
    func: populate_001,
}];

const POST_POPULATIONS: &[Population] = &[];

// ---------------------------------------------------------------------------
// Population: populate_001 — seed the 12 core config keys
// ---------------------------------------------------------------------------
fn populate_001(conn: &Connection) -> SstdResult<()> {
    let entries: [(&str, &str); 12] = [
        ("grid.max_tiles_per_screen_x", "60"),
        ("grid.max_tiles_per_screen_y", "33"),
        ("grid.tile_width_in_pixels", "32"),
        ("grid.tile_height_in_pixels", "32"),
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
        conn.execute(
            "INSERT OR IGNORE INTO config (key, default_value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )
        .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Version helpers
// ---------------------------------------------------------------------------
fn version_to_int(ver: &str) -> u32 {
    let parts: Vec<&str> = ver.split('.').collect();
    if parts.len() != 3 {
        return 0;
    }
    let major: u32 = parts[0].parse().unwrap_or(0);
    let minor: u32 = parts[1].parse().unwrap_or(0);
    let sub: u32 = parts[2].parse().unwrap_or(0);
    major * 100 + minor * 10 + sub
}

fn pop_id_to_version(id: &str) -> u32 {
    id.parse().unwrap_or(0)
}

impl ConfigStore {
    pub fn open_or_create(path: &Path) -> SstdResult<Self> {
        let conn =
            Connection::open(path).map_err(|e| crate::error::StorageError::Io(e.to_string()))?;

        let store = Self { conn };
        store.initial()?;
        store.run_populations()?;
        store.run_post_populations()?;
        Ok(store)
    }

    pub fn in_memory() -> SstdResult<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;

        let store = Self { conn };
        store.initial()?;
        store.run_populations()?;
        store.run_post_populations()?;
        Ok(store)
    }

    /// Category 1: Initial schema creation — runs once (idempotent via IF NOT EXISTS).
    fn initial(&self) -> SstdResult<()> {
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

                CREATE TABLE IF NOT EXISTS populations (
                    id          TEXT PRIMARY KEY,
                    description TEXT,
                    applied_at  DATETIME DEFAULT CURRENT_TIMESTAMP
                );

                CREATE TABLE IF NOT EXISTS tile_set_categories (
                    tile_set_key TEXT NOT NULL,
                    terrain_key  TEXT NOT NULL,
                    PRIMARY KEY (tile_set_key, terrain_key)
                );

                INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', '0.0.0');",
            )
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))
    }

    /// Category 2: Run pending numbered populations in order.
    fn run_populations(&self) -> SstdResult<()> {
        let current_ver = self.get_meta("schema_version")?;
        let current_int = version_to_int(&current_ver);

        for pop in POPULATIONS {
            let pop_ver = pop_id_to_version(pop.id);
            if pop_ver <= current_int {
                continue;
            }
            if self.population_applied(pop.id)? {
                continue;
            }
            (pop.func)(&self.conn)?;
            self.record_population(pop.id, pop.description)?;
            self.set_meta("schema_version", &format_version(pop_ver))?;
        }
        Ok(())
    }

    /// Category 3: Post-populate — same semantics, runs after all populations.
    fn run_post_populations(&self) -> SstdResult<()> {
        let current_ver = self.get_meta("schema_version")?;
        let current_int = version_to_int(&current_ver);

        for pop in POST_POPULATIONS {
            let pop_ver = pop_id_to_version(pop.id);
            if pop_ver <= current_int {
                continue;
            }
            if self.population_applied(pop.id)? {
                continue;
            }
            (pop.func)(&self.conn)?;
            self.record_population(pop.id, pop.description)?;
            self.set_meta("schema_version", &format_version(pop_ver))?;
        }
        Ok(())
    }

    fn get_meta(&self, key: &str) -> SstdResult<String> {
        self.conn
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get::<_, String>(0),
            )
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))
    }

    fn set_meta(&self, key: &str, value: &str) -> SstdResult<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            )
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;
        Ok(())
    }

    fn population_applied(&self, id: &str) -> SstdResult<bool> {
        let count: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM populations WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;
        Ok(count > 0)
    }

    fn record_population(&self, id: &str, description: &str) -> SstdResult<()> {
        self.conn
            .execute(
                "INSERT INTO populations (id, description) VALUES (?1, ?2)",
                rusqlite::params![id, description],
            )
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn get_str(&self, key: &str) -> SstdResult<String> {
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

    pub fn set_tile_set_category(&self, tile_set_key: &str, terrain_key: &str) -> SstdResult<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO tile_set_categories (tile_set_key, terrain_key) VALUES (?1, ?2)",
                rusqlite::params![tile_set_key, terrain_key],
            )
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn remove_tile_set_category(
        &self,
        tile_set_key: &str,
        terrain_key: &str,
    ) -> SstdResult<()> {
        self.conn
            .execute(
                "DELETE FROM tile_set_categories WHERE tile_set_key = ?1 AND terrain_key = ?2",
                rusqlite::params![tile_set_key, terrain_key],
            )
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn get_tile_sets_for_terrain(&self, terrain_key: &str) -> SstdResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tile_set_key FROM tile_set_categories WHERE terrain_key = ?1 ORDER BY tile_set_key")
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![terrain_key], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| crate::error::StorageError::Io(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| crate::error::StorageError::Io(e.to_string()))?);
        }
        Ok(result)
    }

    pub fn get_all_tile_set_categories(&self) -> SstdResult<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tile_set_key, terrain_key FROM tile_set_categories ORDER BY tile_set_key, terrain_key")
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

    pub fn applied_populations(&self) -> SstdResult<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, description FROM populations ORDER BY id")
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

fn format_version(v: u32) -> String {
    let major = v / 100;
    let minor = (v % 100) / 10;
    let sub = v % 10;
    format!("{}.{}.{}", major, minor, sub)
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
        assert_eq!(grid.max_tiles_per_screen_x, 60);
        assert_eq!(grid.tile_width_in_pixels, 32);
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
    fn test_custom_value_not_overwritten() {
        let store = ConfigStore::in_memory().unwrap();
        store
            .set_config("grid.max_tiles_per_screen_x", "99")
            .unwrap();
        // Re-run initial + populations (idempotent — should NOT overwrite)
        store.initial().unwrap();
        store.run_populations().unwrap();
        let val = store.get_str("grid.max_tiles_per_screen_x").unwrap();
        assert_eq!(val, "99");
    }

    #[test]
    fn test_population_applied() {
        let store = ConfigStore::in_memory().unwrap();
        let pops = store.applied_populations().unwrap();
        assert!(pops.iter().any(|(id, _)| id == "001"));
    }

    #[test]
    fn test_schema_version_after_population() {
        let store = ConfigStore::in_memory().unwrap();
        let ver = store.get_meta("schema_version").unwrap();
        assert_eq!(ver, "0.0.1");
    }

    #[test]
    fn test_format_version() {
        assert_eq!(format_version(0), "0.0.0");
        assert_eq!(format_version(1), "0.0.1");
        assert_eq!(format_version(10), "0.1.0");
        assert_eq!(format_version(100), "1.0.0");
        assert_eq!(format_version(123), "1.2.3");
    }

    #[test]
    fn test_version_to_int() {
        assert_eq!(version_to_int("0.0.0"), 0);
        assert_eq!(version_to_int("0.0.1"), 1);
        assert_eq!(version_to_int("0.1.0"), 10);
        assert_eq!(version_to_int("1.0.0"), 100);
        assert_eq!(version_to_int("0.0.9"), 9);
        assert_eq!(version_to_int("0.1.5"), 15);
    }

    #[test]
    fn test_pop_id_to_version() {
        assert_eq!(pop_id_to_version("001"), 1);
        assert_eq!(pop_id_to_version("010"), 10);
        assert_eq!(pop_id_to_version("100"), 100);
    }
}
