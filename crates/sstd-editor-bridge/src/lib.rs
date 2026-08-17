use std::path::Path;

use godot::prelude::*;
use sstd_core::config::ConfigStore;
use sstd_core::storage::{EntityDefsFile, ScreenFile, TerrainTypesFile};
use sstd_core::terrain::GridConfig;

struct SstdEditorBridge;

#[gdextension]
unsafe impl ExtensionLibrary for SstdEditorBridge {}

#[derive(GodotClass)]
#[class(init, base=Node)]
struct SstdBridge {
    base: Base<Node>,
    config_store: Option<ConfigStore>,
}

#[godot_api]
impl SstdBridge {
    #[func]
    fn validate_placement(
        &self,
        map_json: GString,
        entity_key: GString,
        tile_x: i32,
        tile_y: i32,
    ) -> GString {
        let result =
            validate_placement_impl(map_json.to_string(), entity_key.to_string(), tile_x, tile_y);
        GString::from(result.as_str())
    }

    #[func]
    fn import_terrain_types(&self, json: GString) -> GString {
        let result = match TerrainTypesFile::from_json(&json.to_string()) {
            Ok(file) => serde_json::to_string_pretty(&file)
                .unwrap_or_else(|e| format!("{{\"error\": \"serialize failed: {}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn import_entity_defs(&self, json: GString) -> GString {
        let result = match EntityDefsFile::from_json(&json.to_string()) {
            Ok(file) => serde_json::to_string_pretty(&file)
                .unwrap_or_else(|e| format!("{{\"error\": \"serialize failed: {}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn import_screen(&self, json: GString) -> GString {
        let result = match ScreenFile::from_json(&json.to_string()) {
            Ok(file) => serde_json::to_string_pretty(&file)
                .unwrap_or_else(|e| format!("{{\"error\": \"serialize failed: {}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn validate_screen(&self, json: GString) -> GString {
        let result = match ScreenFile::from_json(&json.to_string()) {
            Ok(file) => {
                let validation = file.validate();
                if validation.is_valid() {
                    "{\"valid\": true}".to_string()
                } else {
                    format!(
                        "{{\"valid\": false, \"errors\": {}}}",
                        serde_json::to_string(&validation.messages)
                            .unwrap_or_else(|_| { "[]".to_string() })
                    )
                }
            }
            Err(e) => format!("{{\"valid\": false, \"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn init_config_db(&mut self, path: GString) -> GString {
        let result = match ConfigStore::open_or_create(Path::new(&path.to_string())) {
            Ok(store) => {
                self.config_store = Some(store);
                "{\"ok\": true}".to_string()
            }
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn set_tile_set_category(&self, tile_set_key: GString, terrain_key: GString) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from("{\"error\": \"config store not initialized\"}"),
        };
        match store.set_tile_set_category(&tile_set_key.to_string(), &terrain_key.to_string()) {
            Ok(_) => GString::from("{\"ok\": true}"),
            Err(e) => GString::from(format!("{{\"error\": \"{}\"}}", e).as_str()),
        }
    }

    #[func]
    fn remove_tile_set_category(&self, tile_set_key: GString, terrain_key: GString) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from("{\"error\": \"config store not initialized\"}"),
        };
        match store.remove_tile_set_category(&tile_set_key.to_string(), &terrain_key.to_string()) {
            Ok(_) => GString::from("{\"ok\": true}"),
            Err(e) => GString::from(format!("{{\"error\": \"{}\"}}", e).as_str()),
        }
    }

    #[func]
    fn get_tile_sets_for_terrain(&self, terrain_key: GString) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from("[]"),
        };
        let result = match store.get_tile_sets_for_terrain(&terrain_key.to_string()) {
            Ok(keys) => serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string()),
            Err(_) => "[]".to_string(),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn get_all_tile_set_categories(&self) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from("{}"),
        };
        let result = match store.get_all_tile_set_categories() {
            Ok(entries) => {
                let mut map: std::collections::BTreeMap<String, Vec<String>> =
                    std::collections::BTreeMap::new();
                for (ts_key, t_key) in &entries {
                    map.entry(ts_key.clone()).or_default().push(t_key.clone());
                }
                serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
            }
            Err(_) => "{}".to_string(),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn set_config_value(&self, key: GString, value: GString) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from("{\"error\": \"config store not initialized\"}"),
        };
        match store.set_config(&key.to_string(), &value.to_string()) {
            Ok(_) => GString::from("{\"ok\": true}"),
            Err(e) => GString::from(format!("{{\"error\": \"{}\"}}", e).as_str()),
        }
    }

    #[func]
    fn get_config_value(&self, key: GString) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from(""),
        };
        match store.get_str(&key.to_string()) {
            Ok(v) => GString::from(v.as_str()),
            Err(_) => GString::from(""),
        }
    }

    #[func]
    fn get_grid_config(&self) -> GString {
        let config = match &self.config_store {
            Some(store) => store.grid_config().ok().unwrap_or_default(),
            None => GridConfig::default(),
        };
        GString::from(
            serde_json::to_string(&config)
                .unwrap_or_else(|_| "{\"error\": \"serialize failed\"}".to_string())
                .as_str(),
        )
    }

    #[func]
    fn check_dimension_compatibility(&self, screen_json: GString) -> GString {
        let current = match &self.config_store {
            Some(store) => store.grid_config().ok().unwrap_or_default(),
            None => GridConfig::default(),
        };
        let result = match ScreenFile::from_json(&screen_json.to_string()) {
            Ok(file) => {
                let same_w = file.tile_width_px == current.tile_width_in_pixels;
                let same_h = file.tile_height_px == current.tile_height_in_pixels;
                if same_w && same_h {
                    format!(
                        "{{\"compatible\": true, \"saved_px\": {{\"w\": {}, \"h\": {}}}, \"current_px\": {{\"w\": {}, \"h\": {}}}}}",
                        file.tile_width_px, file.tile_height_px,
                        current.tile_width_in_pixels, current.tile_height_in_pixels,
                    )
                } else {
                    format!(
                        "{{\"compatible\": false, \"saved_px\": {{\"w\": {}, \"h\": {}}}, \"current_px\": {{\"w\": {}, \"h\": {}}}, \"message\": \"Tile dimensions changed from {}x{} to {}x{}. Use porting to rescale.\"}}",
                        file.tile_width_px, file.tile_height_px,
                        current.tile_width_in_pixels, current.tile_height_in_pixels,
                        file.tile_width_px, file.tile_height_px,
                        current.tile_width_in_pixels, current.tile_height_in_pixels,
                    )
                }
            }
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    /// Native parallel exact-diff over gate-surviving duplicate pairs.
    ///
    /// `variants_flat` is the concatenation of every tile's 4 flip variants
    /// (N * 4 * 4096 bytes, variant order: identity, h-flip, v-flip, hv-flip).
    /// `bytes_flat` is the concatenation of every tile's canonical RGBA bytes
    /// (N * 4096). `pairs` is a flat [base, candidate] index stream. Returns a
    /// flat [base, candidate, flip_h, flip_v] match stream. The scan splits
    /// the pair range across `worker_count` native threads (no GDScript
    /// serialization of packed-array access, which the measured 0.72x result
    /// showed is the GDScript bottleneck).
    #[func]
    fn scan_exact_matches(
        &self,
        variants_flat: PackedByteArray,
        bytes_flat: PackedByteArray,
        pairs: PackedInt32Array,
        tolerance: f64,
        worker_count: i32,
    ) -> PackedInt32Array {
        let variants = variants_flat.to_vec();
        let bytes = bytes_flat.to_vec();
        let pair_stream = pairs.to_vec();
        let matches =
            scan_exact_matches_impl(&variants, &bytes, &pair_stream, tolerance, worker_count);
        PackedInt32Array::from(matches)
    }
}

fn validate_placement_impl(
    map_json: String,
    entity_key: String,
    tile_x: i32,
    tile_y: i32,
) -> String {
    let screen: ScreenFile = match serde_json::from_str(&map_json) {
        Ok(s) => s,
        Err(e) => return format!("{{\"valid\": false, \"error\": \"{}\"}}", e),
    };

    if tile_x < 0 || tile_x >= screen.width_tiles {
        return format!(
            "{{\"valid\": false, \"error\": \"tile_x {} out of bounds (0-{})\"}}",
            tile_x,
            screen.width_tiles - 1
        );
    }

    if tile_y < 0 || tile_y >= screen.height_tiles {
        return format!(
            "{{\"valid\": false, \"error\": \"tile_y {} out of bounds (0-{})\"}}",
            tile_y,
            screen.height_tiles - 1
        );
    }

    for placed in &screen.placed_entities {
        if placed.entity_key == entity_key
            && placed.world_tile_x == tile_x
            && placed.world_tile_y == tile_y
        {
            return format!(
                "{{\"valid\": false, \"error\": \"entity '{}' already placed at ({}, {})\"}}",
                entity_key, tile_x, tile_y
            );
        }
    }

    format!(
        "{{\"valid\": true, \"entity\": \"{}\", \"tile_x\": {}, \"tile_y\": {}}}",
        entity_key, tile_x, tile_y
    )
}

const TILE_BYTES: usize = 32 * 32 * 4; // RGBA8 stamp, 4096 bytes.
const VARIANTS_PER_TILE: usize = 4; // identity, h-flip, v-flip, hv-flip.
const MAX_WORKERS: usize = 32;

fn scan_exact_matches_impl(
    variants_flat: &[u8],
    bytes_flat: &[u8],
    pair_stream: &[i32],
    tolerance: f64,
    worker_count: i32,
) -> Vec<i32> {
    let n_pairs = pair_stream.len() / 2;
    if n_pairs == 0 {
        return Vec::new();
    }
    if pair_stream.len() % 2 != 0 || bytes_flat.len() % TILE_BYTES != 0 {
        return Vec::new();
    }
    let n_tiles = bytes_flat.len() / TILE_BYTES;
    if variants_flat.len() != n_tiles * VARIANTS_PER_TILE * TILE_BYTES {
        return Vec::new();
    }

    let workers = (worker_count as usize).clamp(1, MAX_WORKERS).min(n_pairs);
    let chunk = n_pairs.div_ceil(workers);

    // Scoped threads: each worker owns a disjoint slice of the pair stream and
    // shares read-only access to the byte buffers by reference.
    let matches: Vec<i32> = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for w in 0..workers {
            let start = w * chunk * 2;
            let end = ((start + chunk * 2).min(pair_stream.len())).min(n_pairs * 2);
            handles.push(scope.spawn(move || {
                let mut local = Vec::new();
                let mut i = start;
                while i < end {
                    let base = pair_stream[i] as usize;
                    let cand = pair_stream[i + 1] as usize;
                    if base >= n_tiles || cand >= n_tiles {
                        i += 2;
                        continue;
                    }
                    let match_flip = flip_of_variants(
                        &variants_flat[base * VARIANTS_PER_TILE * TILE_BYTES..],
                        &bytes_flat[cand * TILE_BYTES..],
                        tolerance,
                    );
                    if let Some((flip_h, flip_v)) = match_flip {
                        local.extend_from_slice(&[
                            pair_stream[i],
                            pair_stream[i + 1],
                            flip_h as i32,
                            flip_v as i32,
                        ]);
                    }
                    i += 2;
                }
                local
            }));
        }
        let mut all = Vec::new();
        for handle in handles {
            all.extend(handle.join().unwrap_or_default());
        }
        all
    });
    matches
}

/// Port of the GDScript `_diff_capped`: running mean of per-byte abs diff with
/// early exit once the running mean exceeds `limit`. Returns `limit + 1.0` on
/// early exit so callers can reject without a full pass.
fn diff_capped(a: &[u8], b: &[u8], limit: f64) -> f64 {
    let n = a.len();
    let mut sum: i64 = 0;
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        sum += (*x as i64 - *y as i64).abs();
        if (sum as f64) / ((i + 1) as f64) > limit {
            return limit + 1.0;
        }
    }
    (sum as f64) / (n as f64).max(1.0)
}

/// Port of the GDScript `_flip_of_variants`: try all 4 flip variants of `base`
/// against the candidate's canonical bytes and return the best flip on match.
/// `variants_base` must hold the 4 variants contiguously (4096 bytes each).
fn flip_of_variants(variants_base: &[u8], cand: &[u8], tolerance: f64) -> Option<(bool, bool)> {
    let flip_list: [[bool; 2]; 4] = [[false, false], [true, false], [false, true], [true, true]];
    let mut best_diff: f64 = f64::MAX;
    let mut best: (bool, bool) = (false, false);
    for v in 0..VARIANTS_PER_TILE {
        let variant = &variants_base[v * TILE_BYTES..(v + 1) * TILE_BYTES];
        let d = diff_capped(cand, variant, tolerance);
        if d < best_diff {
            best_diff = d;
            best = (flip_list[v][0], flip_list[v][1]);
        }
    }
    if best_diff <= tolerance {
        Some(best)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_capped_identical_is_zero() {
        let a: Vec<u8> = (0u8..64).collect();
        let b: Vec<u8> = a.clone();
        assert_eq!(diff_capped(&a, &b, 4.0), 0.0);
    }

    #[test]
    fn diff_capped_heavy_deviation_exceeds_limit() {
        let mut a: Vec<u8> = (0u8..64).collect();
        let b: Vec<u8> = a.clone();
        a[0] = 200;
        assert!(diff_capped(&a, &b, 4.0) > 4.0);
    }

    #[test]
    fn diff_capped_small_deviation_under_limit() {
        let mut a: Vec<u8> = (0u8..64).collect();
        let b: Vec<u8> = a.clone();
        a[0] = 1;
        assert!(diff_capped(&a, &b, 4.0) <= 4.0);
    }

    #[test]
    fn flip_of_variants_matches_identity() {
        let mut tile = vec![0u8; TILE_BYTES];
        for (i, byte) in tile.iter_mut().enumerate() {
            *byte = (i % 255) as u8;
        }
        let mut variants = Vec::new();
        variants.extend_from_slice(&tile); // identity
        variants.extend_from_slice(&tile); // placeholders for h/v/hv flips
        variants.extend_from_slice(&tile);
        variants.extend_from_slice(&tile);
        assert_eq!(
            flip_of_variants(&variants, &tile, 4.0),
            Some((false, false))
        );
    }

    #[test]
    fn scan_exact_matches_parallel_finds_flipped_pair() {
        // Three 32x32 tiles: canonical, its h+v flip, and a distinct tile.
        // Byte layout is row-major RGBA; flipping both axes maps (x,y) ->
        // (31-x, 31-y). Variant order is identity, h-flip, v-flip, hv-flip.
        let mut canonical = vec![0u8; TILE_BYTES];
        for y in 0..32 {
            for x in 0..32 {
                let idx = (y * 32 + x) * 4;
                canonical[idx] = (x + y) as u8;
                canonical[idx + 1] = 255;
                canonical[idx + 2] = 0;
                canonical[idx + 3] = 255;
            }
        }
        let flip = |tile: &[u8], flip_h: bool, flip_v: bool| -> Vec<u8> {
            let mut out = vec![0u8; TILE_BYTES];
            for y in 0..32 {
                for x in 0..32 {
                    let sx = if flip_h { 31 - x } else { x };
                    let sy = if flip_v { 31 - y } else { y };
                    let src = (sy * 32 + sx) * 4;
                    let dst = (y * 32 + x) * 4;
                    out[dst..dst + 4].copy_from_slice(&tile[src..src + 4]);
                }
            }
            out
        };
        let flipped = flip(&canonical, true, true);
        let mut distinct = vec![0u8; TILE_BYTES];
        for (i, byte) in distinct.iter_mut().enumerate() {
            *byte = (i as u8).wrapping_mul(7);
        }

        let bytes_flat: Vec<u8> = [&canonical[..], &flipped[..], &distinct[..]].concat();
        let mut variants_flat: Vec<u8> = Vec::new();
        for tile in [&canonical[..], &flipped[..], &distinct[..]] {
            variants_flat.extend_from_slice(&flip(tile, false, false));
            variants_flat.extend_from_slice(&flip(tile, true, false));
            variants_flat.extend_from_slice(&flip(tile, false, true));
            variants_flat.extend_from_slice(&flip(tile, true, true));
        }
        let pair_stream = vec![0i32, 1, 0, 2];
        let matches = scan_exact_matches_impl(&variants_flat, &bytes_flat, &pair_stream, 4.0, 4);
        assert_eq!(matches, vec![0, 1, 1, 1]); // base 0 matches cand 1 via h+v flip
    }
}
