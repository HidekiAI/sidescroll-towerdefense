use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// =========================================================================
// Config
// =========================================================================

#[derive(Deserialize, Debug, Clone)]
struct ImportConfig {
    #[serde(default)]
    input: Option<String>,

    #[serde(default = "default_out_dir")]
    output_dir: String,

    #[serde(default = "default_tile_size")]
    tile_size_px: u32,

    grid_cols: Option<u32>,

    grid_rows: Option<u32>,

    #[serde(default)]
    offset_x: u32,

    #[serde(default)]
    offset_y: u32,

    #[serde(default)]
    margin: u32,

    #[serde(default = "default_true")]
    auto_bounding: bool,

    #[serde(default = "default_true")]
    auto_scale: bool,

    #[serde(default = "default_true")]
    auto_detect_collision: bool,

    #[serde(default = "default_luminance_threshold")]
    luminance_threshold: f64,

    #[serde(default)]
    terrain_map: HashMap<String, TerrainMapEntry>,

    #[serde(default)]
    tile_sets: Vec<TileSetDef>,
}

fn default_out_dir() -> String {
    "editor/assets/imported".into()
}
fn default_tile_size() -> u32 {
    32
}
fn default_true() -> bool {
    true
}
fn default_luminance_threshold() -> f64 {
    0.5
}

#[derive(Deserialize, Debug, Clone)]
struct TerrainMapEntry {
    key: String,
    display_name: String,
    #[serde(default = "default_true")]
    is_walkable: bool,
    #[serde(default = "default_true")]
    is_buildable: bool,
    #[serde(default = "default_surface")]
    surface: String,
    #[serde(default)]
    hazard: String,
    color_hex: String,
    indices: Vec<usize>,
}

fn default_surface() -> String {
    "normal".into()
}

#[derive(Deserialize, Debug, Clone)]
struct TileSetDef {
    key: String,
    display_name: String,
    col_start: u32,
    row_start: u32,
    width: u32,
    height: u32,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    cell_overrides: HashMap<String, String>,
}

// =========================================================================
// SSTD Output structures
// =========================================================================

#[derive(Serialize, Debug, Clone)]
struct TerrainTypesFile {
    version: String,
    tiles: Vec<TerrainTypeDef>,
}

#[derive(Serialize, Debug, Clone)]
struct TerrainTypeDef {
    key: String,
    display_name: String,
    is_walkable: bool,
    is_buildable: bool,
    surface: String,
    hazard: String,
    elevation_tiles: i32,
    color_hex: String,
    sub_tile_mask: u8,
    hazard_damage_per_tick: i32,
    is_destructible: bool,
    destructible_hp: i32,
    on_destroy_terrain_key: String,
}

#[derive(Serialize, Debug, Clone)]
struct TileSetsFile {
    version: String,
    tile_sets: Vec<TileSetOutput>,
}

#[derive(Serialize, Debug, Clone)]
struct TileSetOutput {
    key: String,
    display_name: String,
    width_tiles: i32,
    height_tiles: i32,
    tiles: Vec<TileSetEntry>,
    tags: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
struct TileSetEntry {
    local_x: i32,
    local_y: i32,
    terrain_key: String,
    elevation_tiles: i32,
    z_depth: i32,
    sub_tile_mask: u8,
}

// =========================================================================
// Content boundary auto-detection
// =========================================================================

struct ContentBounds {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    pitch_w: u32,
    pitch_h: u32,
}

/// Returns true if a tile-sized window has at least `threshold` levels of
/// luma variation (i.e. is actual content, not a uniform border/background).
fn window_has_content(
    gray: &image::GrayImage,
    x0: u32,
    y0: u32,
    win_w: u32,
    win_h: u32,
    threshold: i32,
) -> bool {
    let (iw, ih) = (gray.width(), gray.height());
    let x1 = (x0 + win_w).min(iw);
    let y1 = (y0 + win_h).min(ih);
    let (mut mn, mut mx) = (255u8, 0u8);
    for y in y0..y1 {
        for x in x0..x1 {
            let l = gray.get_pixel(x, y).0[0];
            mn = mn.min(l);
            mx = mx.max(l);
            if (mx as i32 - mn as i32) > threshold {
                return true;
            }
        }
    }
    false
}

/// Scan a full row at pixel resolution (used for fine-grained bottom/right
/// edge detection within the last content window).
fn row_has_content(gray: &image::GrayImage, y: u32, x0: u32, x1: u32, threshold: i32) -> bool {
    let (mut mn, mut mx) = (255u8, 0u8);
    for x in x0..x1.min(gray.width()) {
        let l = gray.get_pixel(x, y).0[0];
        mn = mn.min(l);
        mx = mx.max(l);
        if (mx as i32 - mn as i32) > threshold {
            return true;
        }
    }
    false
}

fn col_has_content(gray: &image::GrayImage, x: u32, y0: u32, y1: u32, threshold: i32) -> bool {
    let (mut mn, mut mx) = (255u8, 0u8);
    for y in y0..y1.min(gray.height()) {
        let l = gray.get_pixel(x, y).0[0];
        mn = mn.min(l);
        mx = mx.max(l);
        if (mx as i32 - mn as i32) > threshold {
            return true;
        }
    }
    false
}

fn detect_content_bounds(img: &RgbaImage) -> ContentBounds {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return ContentBounds {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
            pitch_w: 0,
            pitch_h: 0,
        };
    }

    let gray = image::imageops::grayscale(img);
    let win = 32u32;
    let threshold = 3i32;

    // Top: scan windowed rows from top
    let top_win = (0..h)
        .step_by(win as usize)
        .find(|&y| window_has_content(&gray, 0, y, w, win, threshold))
        .unwrap_or(0);

    // Refine top: pixel scan within the found window
    let top_pixel = (top_win..(top_win + win).min(h))
        .find(|&y| row_has_content(&gray, y, 0, w, threshold))
        .unwrap_or(top_win);

    // Bottom: scan windowed rows from bottom
    let bottom_win = {
        let mut y = if h >= win { h - win } else { 0 };
        loop {
            if window_has_content(&gray, 0, y, w, win, threshold) {
                break;
            }
            if y < win {
                break;
            }
            y = y.saturating_sub(win);
        }
        y
    };

    // Refine bottom: pixel scan within the found window (descending)
    let bottom_pixel = {
        let start = (bottom_win + win - 1).min(h - 1);
        let mut by = start;
        loop {
            if row_has_content(&gray, by, 0, w, threshold) {
                break by + 1;
            }
            if by == bottom_win {
                break bottom_win + 1;
            }
            by -= 1;
        }
    };

    // Left: scan windowed columns from left
    let left_win = (0..w)
        .step_by(win as usize)
        .find(|&x| window_has_content(&gray, x, 0, win, h, threshold))
        .unwrap_or(0);

    // Refine left: pixel scan within the found window
    let left_pixel = (left_win..(left_win + win).min(w))
        .find(|&x| col_has_content(&gray, x, 0, h, threshold))
        .unwrap_or(left_win);

    // Right: scan windowed columns from right
    let right_win = {
        let mut x = if w >= win { w - win } else { 0 };
        loop {
            if window_has_content(&gray, x, 0, win, h, threshold) {
                break;
            }
            if x < win {
                break;
            }
            x = x.saturating_sub(win);
        }
        x
    };

    // Refine right: pixel scan within the found window (descending)
    let right_pixel = {
        let start = (right_win + win - 1).min(w - 1);
        let mut rx = start;
        loop {
            if col_has_content(&gray, rx, 0, h, threshold) {
                break rx + 1;
            }
            if rx == right_win {
                break right_win + 1;
            }
            rx -= 1;
        }
    };

    // Detect tile pitch by scanning for first blank column/row after content start
    let max_pitch = 256u32.min(w).min(h);
    let pitch_w = if left_pixel + 1 < w {
        ((left_pixel + 1)..w.min(left_pixel + max_pitch + 1))
            .find(|&x| !col_has_content(&gray, x, top_pixel, bottom_pixel, threshold))
            .map(|blank_x| blank_x - left_pixel)
            .unwrap_or(max_pitch)
    } else {
        1
    };

    let pitch_h = if top_pixel + 1 < h {
        ((top_pixel + 1)..h.min(top_pixel + max_pitch + 1))
            .find(|&y| !row_has_content(&gray, y, left_pixel, right_pixel, threshold))
            .map(|blank_y| blank_y - top_pixel)
            .unwrap_or(max_pitch)
    } else {
        1
    };

    ContentBounds {
        left: left_pixel,
        top: top_pixel,
        right: right_pixel,
        bottom: bottom_pixel,
        pitch_w,
        pitch_h,
    }
}

// =========================================================================
// Luminance collision mask detection
// =========================================================================

fn auto_detect_sub_tile_mask(img: &RgbaImage, threshold: f64) -> u8 {
    let w = img.width();
    let h = img.height();
    if w == 0 || h == 0 {
        return 0xF;
    }
    let hw = w / 2;
    let hh = h / 2;

    let quadrants: [(u32, u32, u32, u32, u8); 4] = [
        (0, 0, hw, hh, 0),
        (hw, 0, w, hh, 1),
        (0, hh, hw, h, 2),
        (hw, hh, w, h, 3),
    ];

    let mut mask: u8 = 0;
    for (x0, y0, x1, y1, bit) in quadrants {
        let (mut total, mut count) = (0.0, 0u32);
        for y in y0..y1 {
            for x in x0..x1 {
                let px = img.get_pixel(x, y);
                let lum = 0.299 * px[0] as f64 / 255.0
                    + 0.587 * px[1] as f64 / 255.0
                    + 0.114 * px[2] as f64 / 255.0;
                total += lum;
                count += 1;
            }
        }
        let mean = if count > 0 { total / count as f64 } else { 0.0 };
        if mean > threshold {
            mask |= 1 << bit;
        }
    }
    mask
}

fn auto_detect_terrain_key(img: &RgbaImage) -> &'static str {
    let w = img.width();
    let h = img.height();
    let (mut total, mut count) = (0.0, 0u32);
    let mut dark_pixels = 0u32;
    let total_pixels = w * h;
    if total_pixels == 0 {
        return "air";
    }

    for y in 0..h {
        for x in 0..w {
            let px = img.get_pixel(x, y);
            let alpha = px[3] as f64 / 255.0;
            let lum = (0.299 * px[0] as f64 + 0.587 * px[1] as f64 + 0.114 * px[2] as f64) / 255.0;
            total += lum * alpha;
            if alpha > 0.5 && lum < 0.25 {
                dark_pixels += 1;
            }
            count += 1;
        }
    }
    let mean = if count > 0 { total / count as f64 } else { 0.0 };

    let trans = img.pixels().filter(|p| p[3] < 128).count() as f64;
    let trans_ratio = trans / total_pixels as f64;

    if trans_ratio > 0.6 {
        return "air";
    }
    if mean > 0.45 {
        return "grass";
    }
    if dark_pixels as f64 / total_pixels as f64 > 0.4 {
        return "wall";
    }
    "stone"
}

// =========================================================================
// CLI
// =========================================================================

fn print_usage() {
    eprintln!(
        "Usage: import-tiles --config <config.json> [--input <spritesheet.png>] [--out-dir <dir>]"
    );
    eprintln!();
    eprintln!("Reads a gridded spritesheet and slices it into SSTD-native individual");
    eprintln!("32x32 tile PNGs + terrain_types.json + tile_sets.json.");
    eprintln!();
    eprintln!("Flags:");
    eprintln!("  --config       Path to import config JSON (required)");
    eprintln!("  --input        Override spritesheet PNG path from config");
    eprintln!("  --out-dir      Override output directory from config");
    eprintln!("  --help         Show this help");
}

fn parse_args() -> (String, Option<String>, Option<String>) {
    let args: Vec<String> = std::env::args().collect();
    let mut config_path = None;
    let mut input_override = None;
    let mut out_override = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                i += 1;
                config_path = Some(args[i].clone());
            }
            "--input" => {
                i += 1;
                input_override = Some(args[i].clone());
            }
            "--out-dir" => {
                i += 1;
                out_override = Some(args[i].clone());
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let config_path = config_path.unwrap_or_else(|| {
        eprintln!("Error: --config is required");
        print_usage();
        std::process::exit(1);
    });

    (config_path, input_override, out_override)
}

// =========================================================================
// Main
// =========================================================================

fn main() {
    let (config_path, input_override, out_override) = parse_args();

    let config_text = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("Failed to read config '{}': {}", config_path, e));
    let mut config: ImportConfig = serde_json::from_str(&config_text)
        .unwrap_or_else(|e| panic!("Failed to parse config '{}': {}", config_path, e));

    if let Some(ref path) = input_override {
        config.input = Some(path.clone());
    }
    if let Some(ref path) = out_override {
        config.output_dir = path.clone();
    }

    let input_path = config
        .input
        .as_ref()
        .expect("Either set 'input' in config or pass --input");

    let sheet = image::open(input_path)
        .unwrap_or_else(|e| panic!("Failed to load spritesheet '{}': {}", input_path, e));

    let sheet_w = sheet.width();
    let sheet_h = sheet.height();
    let ts = config.tile_size_px;

    // Auto-detect content bounding box
    let bounds = if config.auto_bounding {
        let sheet_rgba = sheet.to_rgba8();
        let b = detect_content_bounds(&sheet_rgba);
        eprintln!(
            "Auto-bounding: content_area=({},{})->({},{})  pitch=({},{})",
            b.left, b.top, b.right, b.bottom, b.pitch_w, b.pitch_h,
        );
        Some(b)
    } else {
        None
    };

    // Determine source pitch (native tile size in the sheet).
    // When grid dims are provided, pitch = tile_size + margin (most common).
    // When absent, fall back to blank-space scanning for pitch detection.
    let source_pitch_w = if config.grid_cols.is_some() {
        ts + config.margin
    } else if let Some(ref b) = bounds {
        b.pitch_w.max(1)
    } else {
        ts + config.margin
    };

    let source_pitch_h = if config.grid_rows.is_some() {
        ts + config.margin
    } else if let Some(ref b) = bounds {
        b.pitch_h.max(1)
    } else {
        ts + config.margin
    };

    // Infer grid and offset from content bounds (only when grid dims are absent)
    if config.auto_bounding && config.grid_cols.is_none() && config.grid_rows.is_none() {
        if let Some(ref b) = bounds {
            config.offset_x = (b.left / source_pitch_w) * source_pitch_w;
            config.offset_y = (b.top / source_pitch_h) * source_pitch_h;
            config.grid_cols =
                Some(((b.right - config.offset_x + source_pitch_w - 1) / source_pitch_w).max(1));
            config.grid_rows =
                Some(((b.bottom - config.offset_y + source_pitch_h - 1) / source_pitch_h).max(1));
        }
    }

    let grid_cols = config.grid_cols.unwrap();
    let grid_rows = config.grid_rows.unwrap();

    if config.auto_bounding {
        eprintln!(
            "Auto-bounding: pitch={}x{}, offset=({},{}), grid={}x{}",
            source_pitch_w, source_pitch_h, config.offset_x, config.offset_y, grid_cols, grid_rows,
        );
    }

    let expected_w = config.offset_x + grid_cols * source_pitch_w;
    let expected_h = config.offset_y + grid_rows * source_pitch_h;
    if sheet_w < expected_w || sheet_h < expected_h {
        eprintln!(
            "Warning: spritesheet {}x{} is smaller than expected {}x{} \
             (grid {}x{} at source pitch {}x{} with offset {}, {})",
            sheet_w,
            sheet_h,
            expected_w,
            expected_h,
            grid_cols,
            grid_rows,
            source_pitch_w,
            source_pitch_h,
            config.offset_x,
            config.offset_y,
        );
    }

    // Build reverse index: grid index -> terrain map key
    let mut index_to_terrain: HashMap<usize, TerrainMapEntry> = HashMap::new();
    for entry in config.terrain_map.values() {
        for &idx in &entry.indices {
            index_to_terrain.insert(idx, entry.clone());
        }
    }

    let tiles_dir = PathBuf::from(&config.output_dir).join("tiles");
    std::fs::create_dir_all(&tiles_dir).unwrap_or_else(|e| {
        panic!(
            "Failed to create tiles dir '{}': {}",
            tiles_dir.display(),
            e
        )
    });

    let mut terrain_defs: HashMap<String, TerrainTypeDef> = HashMap::new();
    let mut total_tiles = 0;

    for row in 0..grid_rows {
        for col in 0..grid_cols {
            let x = config.offset_x + col * source_pitch_w;
            let y = config.offset_y + row * source_pitch_h;
            let mut tile_dyn = sheet.crop_imm(x, y, source_pitch_w, source_pitch_h);

            // Auto-scale to target tile size if needed
            if config.auto_scale && (tile_dyn.width() != ts || tile_dyn.height() != ts) {
                tile_dyn = tile_dyn.resize_exact(ts, ts, image::imageops::FilterType::Nearest);
            }

            let tile_rgba = tile_dyn.to_rgba8();
            let idx = (row * grid_cols + col) as usize;

            let terrain_key = if let Some(entry) = index_to_terrain.get(&idx) {
                entry.key.clone()
            } else {
                auto_detect_terrain_key(&tile_rgba).to_string()
            };

            let mask = if config.auto_detect_collision {
                auto_detect_sub_tile_mask(&tile_rgba, config.luminance_threshold)
            } else {
                0xF
            };

            let tile_key = format!("{}_{}_{}", terrain_key, row, col);
            let tile_path = tiles_dir.join(format!("{}_32x32.png", tile_key));
            tile_dyn
                .save(&tile_path)
                .unwrap_or_else(|e| panic!("Failed to save tile '{}': {}", tile_path.display(), e));

            if !terrain_defs.contains_key(&terrain_key) {
                let entry = index_to_terrain.get(&idx);
                let td = TerrainTypeDef {
                    key: terrain_key.clone(),
                    display_name: entry
                        .map(|e| e.display_name.clone())
                        .unwrap_or_else(|| terrain_key.clone()),
                    is_walkable: entry.map(|e| e.is_walkable).unwrap_or_else(|| {
                        mask != 0 && terrain_key != "air" && terrain_key != "wall"
                    }),
                    is_buildable: entry.map(|e| e.is_buildable).unwrap_or_else(|| {
                        mask != 0
                            && terrain_key != "air"
                            && terrain_key != "wall"
                            && terrain_key != "water"
                            && terrain_key != "lava"
                    }),
                    surface: entry
                        .map(|e| e.surface.clone())
                        .unwrap_or_else(|| "normal".into()),
                    hazard: entry
                        .map(|e| e.hazard.clone())
                        .unwrap_or_else(|| "none".into()),
                    elevation_tiles: 0,
                    color_hex: entry.map(|e| e.color_hex.clone()).unwrap_or_else(|| {
                        let (r, g, b, _) = average_color(&tile_rgba);
                        format!("#{:02x}{:02x}{:02x}", r, g, b)
                    }),
                    sub_tile_mask: mask,
                    hazard_damage_per_tick: 0,
                    is_destructible: false,
                    destructible_hp: 0,
                    on_destroy_terrain_key: String::new(),
                };
                terrain_defs.insert(terrain_key.clone(), td);
            }

            total_tiles += 1;
            println!(
                "  [{:3}] {} -> {}  mask=0x{:X}",
                idx,
                tile_key,
                tile_path.file_name().unwrap().to_string_lossy(),
                mask
            );
        }
    }

    // Write terrain_types.json
    let mut tiles_sorted: Vec<TerrainTypeDef> = terrain_defs.into_values().collect();
    tiles_sorted.sort_by(|a, b| a.key.cmp(&b.key));
    let terrain_file = TerrainTypesFile {
        version: "0.1.0".into(),
        tiles: tiles_sorted,
    };
    let terrain_json = serde_json::to_string_pretty(&terrain_file)
        .expect("Failed to serialize terrain_types.json");
    let terrain_json_path = PathBuf::from(&config.output_dir).join("terrain_types.json");
    std::fs::write(&terrain_json_path, &terrain_json)
        .unwrap_or_else(|e| panic!("Failed to write '{}': {}", terrain_json_path.display(), e));
    println!("\nWrote {}", terrain_json_path.display());

    // Write tile_sets.json
    if !config.tile_sets.is_empty() {
        let mut sets_out = Vec::new();
        for ts_def in &config.tile_sets {
            let mut entries = Vec::new();
            for local_y in 0..ts_def.height {
                for local_x in 0..ts_def.width {
                    let grid_row = ts_def.row_start + local_y;
                    let grid_col = ts_def.col_start + local_x;
                    let idx = (grid_row * grid_cols + grid_col) as usize;

                    let cell_key = format!("{},{}", local_x, local_y);
                    let terrain_key =
                        if let Some(override_key) = ts_def.cell_overrides.get(&cell_key) {
                            override_key.clone()
                        } else if let Some(entry) = index_to_terrain.get(&idx) {
                            entry.key.clone()
                        } else {
                            let x = config.offset_x + grid_col * source_pitch_w;
                            let y = config.offset_y + grid_row * source_pitch_h;
                            if x + source_pitch_w <= sheet_w && y + source_pitch_h <= sheet_h {
                                let tile_dyn = sheet.crop_imm(x, y, source_pitch_w, source_pitch_h);
                                auto_detect_terrain_key(&tile_dyn.to_rgba8()).to_string()
                            } else {
                                "air".into()
                            }
                        };

                    let x = config.offset_x + grid_col * source_pitch_w;
                    let y = config.offset_y + grid_row * source_pitch_h;
                    let mask = if config.auto_detect_collision
                        && x + source_pitch_w <= sheet_w
                        && y + source_pitch_h <= sheet_h
                    {
                        let tile_dyn = sheet.crop_imm(x, y, source_pitch_w, source_pitch_h);
                        auto_detect_sub_tile_mask(&tile_dyn.to_rgba8(), config.luminance_threshold)
                    } else {
                        0xF
                    };

                    entries.push(TileSetEntry {
                        local_x: local_x as i32,
                        local_y: local_y as i32,
                        terrain_key,
                        elevation_tiles: 0,
                        z_depth: 0,
                        sub_tile_mask: mask,
                    });
                }
            }

            sets_out.push(TileSetOutput {
                key: ts_def.key.clone(),
                display_name: ts_def.display_name.clone(),
                width_tiles: ts_def.width as i32,
                height_tiles: ts_def.height as i32,
                tiles: entries,
                tags: ts_def.tags.clone(),
            });
        }

        let tile_sets_file = TileSetsFile {
            version: "0.1.0".into(),
            tile_sets: sets_out,
        };
        let sets_json = serde_json::to_string_pretty(&tile_sets_file)
            .expect("Failed to serialize tile_sets.json");
        let sets_json_path = PathBuf::from(&config.output_dir).join("tile_sets.json");
        std::fs::write(&sets_json_path, &sets_json)
            .unwrap_or_else(|e| panic!("Failed to write '{}': {}", sets_json_path.display(), e));
        println!("Wrote {}", sets_json_path.display());
    }

    println!(
        "\nDone. {} tiles extracted to {}",
        total_tiles,
        tiles_dir.display()
    );
}

fn average_color(img: &RgbaImage) -> (u8, u8, u8, u8) {
    let (mut r, mut g, mut b, mut a, mut count) = (0u64, 0u64, 0u64, 0u64, 0u64);
    for px in img.pixels() {
        r += px[0] as u64;
        g += px[1] as u64;
        b += px[2] as u64;
        a += px[3] as u64;
        count += 1;
    }
    if count == 0 {
        return (128, 128, 128, 255);
    }
    (
        (r / count) as u8,
        (g / count) as u8,
        (b / count) as u8,
        (a / count) as u8,
    )
}
