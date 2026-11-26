use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// =========================================================================
// Config
// =========================================================================

/// Minimum separation, in greyscale values, between the two clusters of a tile's
/// greyscale for the art to count as air-versus-ground. Below it the tile has no
/// split at all and every heuristic is inconclusive, so the tile is flagged.
const DEFAULT_MIN_CLUSTER_GAP: u8 = 32;

/// The window a region colour is read through: 4x4 is the ceiling the plan allows for the
/// uniform-patch sample (2x2 is its floor), so a lone anti-aliased pixel cannot move it.
const PATCH_SIZE_PX: u32 = 4;

/// The 1px border baked into a tile image whose mask could not be resolved, so a
/// hand-edit is visible in the emitted PNG.
const UNRESOLVED_BORDER_RGBA: [u8; 4] = [255, 0, 0, 255];

/// The Rec.601 luma weights (ITU-R BT.601) that collapse a pixel's R, G and B channels into
/// one greyscale value. Named because the detector's thresholds are all derived from them,
/// and the three sum to just under 1.0 in f64, so a rounded sum keeps a pure grey exact.
const REC601_LUMA_RED_WEIGHT: f64 = 0.299;
const REC601_LUMA_GREEN_WEIGHT: f64 = 0.587;
const REC601_LUMA_BLUE_WEIGHT: f64 = 0.114;

/// The legacy 4-bit mask value for a wholly solid sub-tile, used by the importer path that
/// predates the per-pixel mask. Named so the coarse and fine paths cannot respell it apart.
const SUB_TILE_MASK_ALL_SOLID: u8 = 0xF;

/// The byte written for every pixel of a single-grey tile's packed solid mask.
const PACKED_MASK_BYTE_ALL_SOLID: u8 = 0xFF;

/// The file version stamped on every emitted JSON document; a persisted format token.
const FILE_VERSION: &str = "0.1.0";

/// A pixel is transparent enough to ignore when its alpha is below this byte value.
const TRANSPARENT_ALPHA_CUTOFF: u8 = 128;

/// The luminance checks that name a coarse terrain class from a tile's pixels.
const DARK_LUMA_CUTOFF: f64 = 0.25;
const TRANSPARENT_RATIO_CUTOFF: f64 = 0.6;
const GRASS_MEAN_LUMA_CUTOFF: f64 = 0.45;
const DARK_PIXEL_RATIO_CUTOFF: f64 = 0.4;

/// The coarse content detector scans in fixed windows, requiring this much luma variation
/// inside a window and allowing at most this pitch before it gives up.
const CONTENT_WINDOW_PX: u32 = 32;
const CONTENT_LUMA_VARIATION_THRESHOLD: i32 = 3;
const MAX_DETECTED_PITCH_PX: u32 = 256;

/// Neutral colour used when an empty image has no average to report.
const AVERAGE_COLOR_FALLBACK_RGBA: (u8, u8, u8, u8) = (128, 128, 128, 255);

/// Eight pixels share one packed mask byte.
const BITS_PER_BYTE: usize = 8;

/// Terrain keys and default field values written into the emitted catalog. These are
/// persisted tokens, so they live in one place instead of being respelled at each site.
const AIR_TERRAIN_KEY: &str = "air";
const WALL_TERRAIN_KEY: &str = "wall";
const WATER_TERRAIN_KEY: &str = "water";
const LAVA_TERRAIN_KEY: &str = "lava";
const GRASS_TERRAIN_KEY: &str = "grass";
const STONE_TERRAIN_KEY: &str = "stone";
const NORMAL_SURFACE: &str = "normal";
const NO_HAZARD: &str = "none";

/// A pixel contributes to the dark-pixel count only when it is this opaque or more.
const DARK_PIXEL_ALPHA_CUTOFF: f64 = 0.5;

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

    #[serde(default = "default_min_cluster_gap")]
    min_cluster_gap: u8,

    /// When set, `main()` runs the tile-scan mode instead of the spritesheet-slice mode:
    /// it scans this directory for `{key}_32x32.png` and writes `terrain_types.json`.
    #[serde(default)]
    scan_tiles: Option<String>,

    /// Only filenames starting with this prefix are scanned (e.g. `slope_up_`). Empty scans all.
    #[serde(default)]
    scan_prefix: Option<String>,

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
fn default_min_cluster_gap() -> u8 {
    DEFAULT_MIN_CLUSTER_GAP
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
    /// Per-pixel solid-air bitfield of the tile art (one bit per pixel, row-major).
    /// The tool emits it as serde_json's own `Vec<u8>` form (a JSON array of byte numbers);
    /// the final on-disk encoding is settled together with the core field, so nothing here
    /// commits the format yet.
    /// Empty means "not scanned" -> whole tile solid (legacy fallback).
    solid_mask: Vec<u8>,
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
    window_x0: u32,
    window_y0: u32,
    window_width: u32,
    window_height: u32,
    threshold: i32,
) -> bool {
    let window_x1: u32 = (window_x0 + window_width).min(gray.width());
    let window_y1: u32 = (window_y0 + window_height).min(gray.height());
    let (mut mut_min_luma, mut mut_max_luma) = (u8::MAX, 0u8);
    // Leave as soon as any pixel pair clears the range, since more pixels can only widen it.
    for pixel_y in window_y0..window_y1 {
        for pixel_x in window_x0..window_x1 {
            let pixel_luma: u8 = gray.get_pixel(pixel_x, pixel_y).0[0];
            mut_min_luma = mut_min_luma.min(pixel_luma);
            mut_max_luma = mut_max_luma.max(pixel_luma);
            if i32::from(mut_max_luma) - i32::from(mut_min_luma) > threshold {
                return true;
            }
        }
    }
    false
}

/// Scan a full row at pixel resolution (used for fine-grained bottom/right
/// edge detection within the last content window).
fn row_has_content(
    gray: &image::GrayImage,
    row_y: u32,
    row_x0: u32,
    row_x1: u32,
    threshold: i32,
) -> bool {
    let (mut mut_min_luma, mut mut_max_luma) = (u8::MAX, 0u8);
    for pixel_x in row_x0..row_x1.min(gray.width()) {
        let pixel_luma: u8 = gray.get_pixel(pixel_x, row_y).0[0];
        mut_min_luma = mut_min_luma.min(pixel_luma);
        mut_max_luma = mut_max_luma.max(pixel_luma);
        if i32::from(mut_max_luma) - i32::from(mut_min_luma) > threshold {
            return true;
        }
    }
    false
}

fn col_has_content(
    gray: &image::GrayImage,
    col_x: u32,
    col_y0: u32,
    col_y1: u32,
    threshold: i32,
) -> bool {
    let (mut mut_min_luma, mut mut_max_luma) = (u8::MAX, 0u8);
    for pixel_y in col_y0..col_y1.min(gray.height()) {
        let pixel_luma: u8 = gray.get_pixel(col_x, pixel_y).0[0];
        mut_min_luma = mut_min_luma.min(pixel_luma);
        mut_max_luma = mut_max_luma.max(pixel_luma);
        if i32::from(mut_max_luma) - i32::from(mut_min_luma) > threshold {
            return true;
        }
    }
    false
}

// Locate the top edge with a coarse window scan, then refine inside that window.
fn fn_find_content_top_edge(gray: &image::GrayImage, image_width: u32, image_height: u32) -> u32 {
    let content_window_y: u32 = (0..image_height)
        .step_by(CONTENT_WINDOW_PX as usize)
        .find(|&window_y| {
            window_has_content(
                gray,
                0,
                window_y,
                image_width,
                CONTENT_WINDOW_PX,
                CONTENT_LUMA_VARIATION_THRESHOLD,
            )
        })
        .unwrap_or(0);
    (content_window_y..(content_window_y + CONTENT_WINDOW_PX).min(image_height))
        .find(|&pixel_y| {
            row_has_content(
                gray,
                pixel_y,
                0,
                image_width,
                CONTENT_LUMA_VARIATION_THRESHOLD,
            )
        })
        .unwrap_or(content_window_y)
}

// Walk windows up from the bottom, then refine downward to the first content row.
fn fn_find_content_bottom_edge(
    gray: &image::GrayImage,
    image_width: u32,
    image_height: u32,
) -> u32 {
    let mut mut_content_window_y: u32 = match image_height >= CONTENT_WINDOW_PX {
        true => image_height - CONTENT_WINDOW_PX,
        false => 0,
    };
    loop {
        if window_has_content(
            gray,
            0,
            mut_content_window_y,
            image_width,
            CONTENT_WINDOW_PX,
            CONTENT_LUMA_VARIATION_THRESHOLD,
        ) {
            break;
        }
        if mut_content_window_y < CONTENT_WINDOW_PX {
            break;
        }
        mut_content_window_y = mut_content_window_y.saturating_sub(CONTENT_WINDOW_PX);
    }
    let start_row: u32 = (mut_content_window_y + CONTENT_WINDOW_PX - 1).min(image_height - 1);
    let mut mut_pixel_y: u32 = start_row;
    loop {
        if row_has_content(
            gray,
            mut_pixel_y,
            0,
            image_width,
            CONTENT_LUMA_VARIATION_THRESHOLD,
        ) {
            break mut_pixel_y + 1;
        }
        if mut_pixel_y == mut_content_window_y {
            break mut_content_window_y + 1;
        }
        mut_pixel_y -= 1;
    }
}

// Locate the left edge with a coarse window scan, then refine inside that window.
fn fn_find_content_left_edge(gray: &image::GrayImage, image_width: u32, image_height: u32) -> u32 {
    let content_window_x: u32 = (0..image_width)
        .step_by(CONTENT_WINDOW_PX as usize)
        .find(|&window_x| {
            window_has_content(
                gray,
                window_x,
                0,
                CONTENT_WINDOW_PX,
                image_height,
                CONTENT_LUMA_VARIATION_THRESHOLD,
            )
        })
        .unwrap_or(0);
    (content_window_x..(content_window_x + CONTENT_WINDOW_PX).min(image_width))
        .find(|&pixel_x| {
            col_has_content(
                gray,
                pixel_x,
                0,
                image_height,
                CONTENT_LUMA_VARIATION_THRESHOLD,
            )
        })
        .unwrap_or(content_window_x)
}

// Walk windows left from the right edge, then refine to the first content column.
fn fn_find_content_right_edge(gray: &image::GrayImage, image_width: u32, image_height: u32) -> u32 {
    let mut mut_content_window_x: u32 = match image_width >= CONTENT_WINDOW_PX {
        true => image_width - CONTENT_WINDOW_PX,
        false => 0,
    };
    loop {
        if window_has_content(
            gray,
            mut_content_window_x,
            0,
            CONTENT_WINDOW_PX,
            image_height,
            CONTENT_LUMA_VARIATION_THRESHOLD,
        ) {
            break;
        }
        if mut_content_window_x < CONTENT_WINDOW_PX {
            break;
        }
        mut_content_window_x = mut_content_window_x.saturating_sub(CONTENT_WINDOW_PX);
    }
    let start_col: u32 = (mut_content_window_x + CONTENT_WINDOW_PX - 1).min(image_width - 1);
    let mut mut_pixel_x: u32 = start_col;
    loop {
        if col_has_content(
            gray,
            mut_pixel_x,
            0,
            image_height,
            CONTENT_LUMA_VARIATION_THRESHOLD,
        ) {
            break mut_pixel_x + 1;
        }
        if mut_pixel_x == mut_content_window_x {
            break mut_content_window_x + 1;
        }
        mut_pixel_x -= 1;
    }
}

fn detect_content_bounds(img: &RgbaImage) -> ContentBounds {
    let image_width: u32 = img.width();
    let image_height: u32 = img.height();
    if image_width == 0 || image_height == 0 {
        return ContentBounds {
            left: 0,
            top: 0,
            right: image_width,
            bottom: image_height,
            pitch_w: 0,
            pitch_h: 0,
        };
    }
    let gray: image::GrayImage = image::imageops::grayscale(img);

    // Four edge scans read one greyscale view and never mutate it.
    let top_pixel: u32 = fn_find_content_top_edge(&gray, image_width, image_height);
    let bottom_pixel: u32 = fn_find_content_bottom_edge(&gray, image_width, image_height);
    let left_pixel: u32 = fn_find_content_left_edge(&gray, image_width, image_height);
    let right_pixel: u32 = fn_find_content_right_edge(&gray, image_width, image_height);

    // The first empty column or row after the content start gives the cell pitch.
    let max_pitch_px: u32 = MAX_DETECTED_PITCH_PX.min(image_width).min(image_height);
    let pitch_w: u32 = match left_pixel + 1 < image_width {
        true => ((left_pixel + 1)..image_width.min(left_pixel + max_pitch_px + 1))
            .find(|&blank_col| {
                !col_has_content(
                    &gray,
                    blank_col,
                    top_pixel,
                    bottom_pixel,
                    CONTENT_LUMA_VARIATION_THRESHOLD,
                )
            })
            .map(|blank_col| blank_col - left_pixel)
            .unwrap_or(max_pitch_px),
        false => 1,
    };
    let pitch_h: u32 = match top_pixel + 1 < image_height {
        true => ((top_pixel + 1)..image_height.min(top_pixel + max_pitch_px + 1))
            .find(|&blank_row| {
                !row_has_content(
                    &gray,
                    blank_row,
                    left_pixel,
                    right_pixel,
                    CONTENT_LUMA_VARIATION_THRESHOLD,
                )
            })
            .map(|blank_row| blank_row - top_pixel)
            .unwrap_or(max_pitch_px),
        false => 1,
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
    let image_width: u32 = img.width();
    let image_height: u32 = img.height();
    if image_width == 0 || image_height == 0 {
        return SUB_TILE_MASK_ALL_SOLID;
    }
    let half_width: u32 = image_width / 2;
    let half_height: u32 = image_height / 2;

    // Each bit describes one quadrant of the finished tile, in reading order.
    let quadrants: [(u32, u32, u32, u32, u8); 4] = [
        (0, 0, half_width, half_height, 0),
        (half_width, 0, image_width, half_height, 1),
        (0, half_height, half_width, image_height, 2),
        (half_width, half_height, image_width, image_height, 3),
    ];

    let mut mut_mask: u8 = 0;
    for (quad_x0, quad_y0, quad_x1, quad_y1, bit_index) in quadrants {
        // Average Rec.601 luma over the quadrant; a bright quadrant is solid ground.
        let mut mut_luma_sum: f64 = 0.0;
        let mut mut_pixel_count: u32 = 0;
        for pixel_y in quad_y0..quad_y1 {
            for pixel_x in quad_x0..quad_x1 {
                let pixel: &image::Rgba<u8> = img.get_pixel(pixel_x, pixel_y);
                let pixel_luma: f64 = REC601_LUMA_RED_WEIGHT * f64::from(pixel[0]) / f64::from(u8::MAX)
                    + REC601_LUMA_GREEN_WEIGHT * f64::from(pixel[1]) / f64::from(u8::MAX)
                    + REC601_LUMA_BLUE_WEIGHT * f64::from(pixel[2]) / f64::from(u8::MAX);
                mut_luma_sum += pixel_luma;
                mut_pixel_count += 1;
            }
        }
        let mean_luma: f64 = match mut_pixel_count > 0 {
            true => mut_luma_sum / f64::from(mut_pixel_count),
            false => 0.0,
        };
        if mean_luma > threshold {
            mut_mask |= 1 << bit_index;
        }
    }
    mut_mask
}

fn auto_detect_terrain_key(img: &RgbaImage) -> &'static str {
    let image_width: u32 = img.width();
    let image_height: u32 = img.height();
    let total_pixels: u32 = image_width * image_height;
    if total_pixels == 0 {
        return AIR_TERRAIN_KEY;
    }

    // Accumulate alpha-weighted luma and count dark opaque pixels for the coarse classes.
    let mut mut_weighted_luma_sum: f64 = 0.0;
    let mut mut_dark_pixel_count: u32 = 0;
    for pixel_y in 0..image_height {
        for pixel_x in 0..image_width {
            let pixel: &image::Rgba<u8> = img.get_pixel(pixel_x, pixel_y);
            let pixel_alpha: f64 = f64::from(pixel[3]) / f64::from(u8::MAX);
            let pixel_luma: f64 = (REC601_LUMA_RED_WEIGHT * f64::from(pixel[0])
                + REC601_LUMA_GREEN_WEIGHT * f64::from(pixel[1])
                + REC601_LUMA_BLUE_WEIGHT * f64::from(pixel[2]))
                / f64::from(u8::MAX);
            mut_weighted_luma_sum += pixel_luma * pixel_alpha;
            if pixel_alpha > DARK_PIXEL_ALPHA_CUTOFF && pixel_luma < DARK_LUMA_CUTOFF {
                mut_dark_pixel_count += 1;
            }
        }
    }
    let mean_luma: f64 = mut_weighted_luma_sum / f64::from(total_pixels);

    // A mostly transparent tile is air before any colour class is considered.
    let transparent_pixel_count: u32 = img
        .pixels()
        .filter(|pixel| pixel[3] < TRANSPARENT_ALPHA_CUTOFF)
        .count() as u32;
    let transparent_ratio: f64 = f64::from(transparent_pixel_count) / f64::from(total_pixels);
    match transparent_ratio > TRANSPARENT_RATIO_CUTOFF {
        true => AIR_TERRAIN_KEY,
        false => match mean_luma > GRASS_MEAN_LUMA_CUTOFF {
            true => GRASS_TERRAIN_KEY,
            false => match f64::from(mut_dark_pixel_count) / f64::from(total_pixels)
                > DARK_PIXEL_RATIO_CUTOFF
            {
                true => WALL_TERRAIN_KEY,
                false => STONE_TERRAIN_KEY,
            },
        },
    }
}

// =========================================================================
// Two-phase conversion contract (Rust side of the GDScript bridge)
// =========================================================================

// Every name below is the persisted bridge contract, declared once here and pinned by the guards
// rather than duplicated in the GDScript orchestrator. Field units are carried in the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
struct PixelPoint {
    x: i32,
    y: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
struct PixelRect {
    x: u32,
    y: u32,
    width_px: u32,
    height_px: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
struct CellDimensions {
    width_px: u32,
    height_px: u32,
}
#[derive(Deserialize, Debug, Clone)]
struct ConversionRequest {
    version: String,
    source_image: String,
    margins_px: PixelPoint,
    cell_size_px: PixelPoint,
    separation_px: PixelPoint,
    width_px: u32,
    height_px: u32,
    cells: Vec<CellRequest>,
}
#[derive(Deserialize, Debug, Clone)]
struct CellRequest {
    local_x: i32,
    local_y: i32,
    source_col: i32,
    source_row: i32,
    flip_h: bool,
    generated_key: String,
}
// The emitted image path is skipped on serialization: it is a runtime handle, not bridge data,
// so two runs in different output directories serialize byte-identically.
#[derive(Serialize, Debug, Clone)]
struct ConversionOutput {
    generated_key: String,
    #[serde(skip)]
    image_path: PathBuf,
    solid_mask: Vec<u8>,
    resolved: bool,
}
#[derive(Serialize, Debug, Clone)]
struct ConversionResult {
    version: String,
    outputs: Vec<ConversionOutput>,
}
#[derive(Debug, Clone, PartialEq)]
enum ConvertError {
    Io(String),
    Decode(String),
    Version(String),
    Unsigned,
    UnresolvedCells(Vec<String>),
}
#[derive(Debug, Clone, PartialEq)]
struct CleanedCell {
    source_col: i32,
    source_row: i32,
    pixel_rect: PixelRect,
    dimensions: CellDimensions,
    solid_mask: Vec<u8>,
    resolved: bool,
}
#[derive(Debug, Clone, PartialEq)]
struct PreprocessResult {
    crop_rect: PixelRect,
    cleaned_image_path: PathBuf,
    cells: Vec<CleanedCell>,
    cleared_grid_line_pixels: u32,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct SidecarRow {
    source_col: i32,
    source_row: i32,
    pixel_rect: PixelRect,
    dimensions: CellDimensions,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ConversionSidecar {
    version: String,
    tile_width_px: u32,
    tile_height_px: u32,
    mask_encoding: String,
    tiles: Vec<SidecarRow>,
}
const REQUEST_VERSION: &str = "1.0.0";
const RESULT_VERSION: &str = "1.0.0";
const MASK_ENCODING_GREYSCALE: &str = "greyscale";

// =========================================================================
// solid-mask pipeline (ART -> DATA, step 1)
// =========================================================================
// The tile's GREYSCALE is the detector; the emitted mask is the product. Stages, in order:
// greyscale -> uniform-patch region read -> floating-speck clean -> two-cluster mask with a
// best guess -> red-border flag -> per-tile entry -> directory scan. `auto_detect_sub_tile_mask`
// above stays as the coarse 4-bit emission path; these functions are the per-pixel path.

// The detector reads greyscale, so alpha is ignored: a transparent air pixel and an opaque
// black pixel both read as 0 (air). Row-major order, so index (row * width + col) is the
// pixel at (col, row). The Rec.601 weights sum to just under 1.0 in f64, so the sum is
// ROUNDED: truncating would turn the committed art's pure grey 155 into 154.
fn to_greyscale(img: &RgbaImage) -> Vec<u8> {
    img.pixels()
        .map(|pixel| {
            (REC601_LUMA_RED_WEIGHT * pixel[0] as f64
                + REC601_LUMA_GREEN_WEIGHT * pixel[1] as f64
                + REC601_LUMA_BLUE_WEIGHT * pixel[2] as f64)
                .round() as u8
        })
        .collect()
}

// A region's colour is read through a patch, never a single pixel: an anti-aliased edge
// pixel inside the patch is outvoted by its uniform neighbours. The window is
// `patch_size_px` square, centred on (patch_x, patch_y) and clamped to the tile; the MODE
// grey is the region's colour.
fn uniform_patch_grey(
    greyscale: &[u8],
    width_px: u32,
    height_px: u32,
    patch_x: u32,
    patch_y: u32,
    patch_size_px: u32,
) -> u8 {
    let half: u32 = patch_size_px / 2;
    let col_start: u32 = patch_x.saturating_sub(half);
    let row_start: u32 = patch_y.saturating_sub(half);
    let col_end: u32 = (col_start + patch_size_px).min(width_px);
    let row_end: u32 = (row_start + patch_size_px).min(height_px);
    let mut mut_histogram: [u32; 256] = [0; 256];
    for row in row_start..row_end {
        for col in col_start..col_end {
            mut_histogram[greyscale[(row * width_px + col) as usize] as usize] += 1;
        }
    }
    mut_histogram
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .map(|(grey, _)| grey as u8)
        .unwrap_or(0)
}

// A solid run that does not reach the tile's bottom row is dust, not ground: the bottom-row
// anchor says the ground is the mass the tile stands on. Flood from every solid pixel of the
// bottom row and clear whatever the flood never reaches.
fn remove_floating_specks(binary: &mut [u8], width_px: u32, height_px: u32) {
    let width: usize = width_px as usize;
    let bottom_row_start: usize = (height_px as usize - 1) * width;
    let mut mut_reached: Vec<bool> = vec![false; binary.len()];
    let mut mut_stack: Vec<usize> = (0..width)
        .filter(|col| binary[bottom_row_start + col] != 0)
        .map(|col| bottom_row_start + col)
        .collect();
    while let Some(index) = mut_stack.pop() {
        if mut_reached[index] {
            continue;
        }
        mut_reached[index] = true;
        let col: usize = index % width;
        let row: usize = index / width;
        let neighbours: [Option<usize>; 4] = [
            (col > 0).then(|| index - 1),
            (col + 1 < width).then(|| index + 1),
            (row > 0).then(|| index - width),
            (row + 1 < height_px as usize).then(|| index + width),
        ];
        for neighbour in neighbours.into_iter().flatten() {
            if binary[neighbour] != 0 && !mut_reached[neighbour] {
                mut_stack.push(neighbour);
            }
        }
    }
    for (index, solid) in binary.iter_mut().enumerate() {
        if *solid != 0 && !mut_reached[index] {
            *solid = 0;
        }
    }
}

// Count how often each grey level occurs. The most frequent grey is the background and a
// distant second grey is the art's other region; a histogram avoids comparing every pixel
// against every candidate grey.
fn fn_grey_histogram(greyscale: &[u8]) -> [u32; 256] {
    let mut mut_histogram: [u32; 256] = [0; 256];
    for grey in greyscale {
        mut_histogram[*grey as usize] += 1;
    }
    mut_histogram
}

// List only the grey levels that occur, ascending. One level means there is nothing to split.
fn fn_present_greys(histogram: &[u32; 256]) -> Vec<u8> {
    (0..=255u8)
        .filter(|grey| histogram[*grey as usize] > 0)
        .collect()
}

// Pair the dominant grey with the grey furthest from it. A pair closer than the minimum gap
// cannot be named air and ground, so the caller flags the tile instead of guessing a mask.
fn fn_choose_cluster_greys(
    present: &[u8],
    histogram: &[u32; 256],
    min_cluster_gap: u8,
) -> Option<(u8, u8)> {
    let dominant_grey: u8 = *present
        .iter()
        .max_by_key(|grey| histogram[**grey as usize])
        .unwrap();
    let opposite_grey: u8 = *present
        .iter()
        .max_by_key(|grey| (**grey as i32 - dominant_grey as i32).abs())
        .unwrap();
    ((opposite_grey as i32 - dominant_grey as i32).abs() >= min_cluster_gap as i32)
        .then_some((dominant_grey, opposite_grey))
}

// The bottom row stands on ground, so the cluster its patch reads becomes solid and the other
// cluster becomes air. A bottom row carrying both greys is settled by this one patch read.
fn fn_anchor_ground_grey(
    greyscale: &[u8],
    width_px: u32,
    height_px: u32,
    dominant_grey: u8,
    opposite_grey: u8,
) -> (u8, u8) {
    let bottom_grey: u8 = uniform_patch_grey(
        greyscale,
        width_px,
        height_px,
        width_px / 2,
        height_px - 1,
        PATCH_SIZE_PX,
    );
    let bottom_is_dominant: bool = (bottom_grey as i32 - dominant_grey as i32).abs()
        <= (bottom_grey as i32 - opposite_grey as i32).abs();
    if bottom_is_dominant {
        (dominant_grey, opposite_grey)
    } else {
        (opposite_grey, dominant_grey)
    }
}

// Classify each pixel by whichever cluster grey it is nearer to. Specks that never connect
// back to the bottom-row ground mass are cleared by the caller.
fn fn_classify_solid_pixels(greyscale: &[u8], ground_grey: u8, air_grey: u8) -> Vec<u8> {
    greyscale
        .iter()
        .map(|grey| {
            ((*grey as i32 - ground_grey as i32).abs() <= (*grey as i32 - air_grey as i32).abs())
                as u8
        })
        .collect()
}

// Pack one bit per pixel, least significant bit first, so eight pixels share a byte.
fn fn_pack_solid_bits(binary: &[u8]) -> Vec<u8> {
    let mut mut_mask: Vec<u8> = vec![0u8; (binary.len() + BITS_PER_BYTE - 1) / BITS_PER_BYTE];
    for (index, solid) in binary.iter().enumerate() {
        if *solid != 0 {
            mut_mask[index / BITS_PER_BYTE] |= 1 << (index % BITS_PER_BYTE);
        }
    }
    mut_mask
}

// Resolve one tile's pixels into a packed solid mask plus a `resolved` verdict. A single-grey
// tile is all solid, and a pair closer than `min_cluster_gap` is flagged rather than guessed.
// Otherwise the bottom row anchors the ground cluster and nearest-cluster classification does
// the rest.
fn derive_solid_mask(
    greyscale: &[u8],
    width_px: u32,
    height_px: u32,
    min_cluster_gap: u8,
) -> (Vec<u8>, bool) {
    let packed_len: usize = (greyscale.len() + BITS_PER_BYTE - 1) / BITS_PER_BYTE;
    let histogram: [u32; 256] = fn_grey_histogram(greyscale);
    let present: Vec<u8> = fn_present_greys(&histogram);
    // One grey level offers no anchor, so the whole tile is emitted as solid.
    if present.len() <= 1 {
        return (vec![PACKED_MASK_BYTE_ALL_SOLID; packed_len], true);
    }
    // A pair failing the gap test is unresolved; refuse to emit a guessed mask.
    let (dominant_grey, opposite_grey): (u8, u8) =
        match fn_choose_cluster_greys(&present, &histogram, min_cluster_gap) {
            Some(cluster_greys) => cluster_greys,
            None => return (vec![PACKED_MASK_BYTE_ALL_SOLID; packed_len], false),
        };
    let (ground_grey, air_grey): (u8, u8) =
        fn_anchor_ground_grey(greyscale, width_px, height_px, dominant_grey, opposite_grey);
    let mut mut_binary: Vec<u8> = fn_classify_solid_pixels(greyscale, ground_grey, air_grey);
    // Clear specks that never connect to the bottom-row ground mass.
    remove_floating_specks(&mut mut_binary, width_px, height_px);
    (fn_pack_solid_bits(&mut_binary), true)
}

// A tile no heuristic could resolve gets a red 1px border baked into the emitted PNG, so a
// human sees it and hand-edits the art. The interior is never repainted.
fn flag_unresolved(img: &mut RgbaImage) {
    use image::Rgba;
    let width_px: u32 = img.width();
    let height_px: u32 = img.height();
    let border: Rgba<u8> = Rgba(UNRESOLVED_BORDER_RGBA);
    for col in 0..width_px {
        img.put_pixel(col, 0, border);
        img.put_pixel(col, height_px.saturating_sub(1), border);
    }
    for row in 0..height_px {
        img.put_pixel(0, row, border);
        img.put_pixel(width_px.saturating_sub(1), row, border);
    }
}

// Turn one committed tile PNG into the `terrain_types.json` entry the framework reads: the
// key, a walkable flag, and the per-pixel `solid_mask` derived from the art. The `resolved`
// verdict is returned so the caller can flag an inconclusive tile.
fn terrain_entry_from_image(
    key: &str,
    img: &RgbaImage,
    min_cluster_gap: u8,
) -> (TerrainTypeDef, bool) {
    let greyscale: Vec<u8> = to_greyscale(img);
    let (mask, resolved): (Vec<u8>, bool) =
        derive_solid_mask(&greyscale, img.width(), img.height(), min_cluster_gap);
    let (red, green, blue, _) = average_color(img);
    let entry: TerrainTypeDef = TerrainTypeDef {
        key: key.to_string(),
        display_name: key.replace('_', " "),
        is_walkable: true,
        is_buildable: true,
        surface: NORMAL_SURFACE.into(),
        hazard: NO_HAZARD.into(),
        elevation_tiles: 0,
        color_hex: format!("#{:02x}{:02x}{:02x}", red, green, blue),
        sub_tile_mask: SUB_TILE_MASK_ALL_SOLID,
        solid_mask: mask,
        hazard_damage_per_tick: 0,
        is_destructible: false,
        destructible_hp: 0,
        on_destroy_terrain_key: String::new(),
    };
    (entry, resolved)
}

// Scan a directory of `{key}_32x32.png` tiles (optionally filtered by `key_prefix`) into
// terrain entries, sorted by key. A tile whose mask cannot be resolved is written into
// `output_dir` with the red border so the human sees it, and logged.
fn scan_tile_directory(
    dir: &Path,
    key_prefix: &str,
    min_cluster_gap: u8,
    output_dir: &Path,
) -> Vec<TerrainTypeDef> {
    let mut mut_entries: Vec<TerrainTypeDef> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Failed to read tile dir '{}': {}", dir.display(), e))
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let file_name: String = entry.file_name().to_string_lossy().into_owned();
            let key: &str = file_name
                .strip_suffix("_32x32.png")
                .filter(|stripped| stripped.starts_with(key_prefix))?;
            let mut mut_img: RgbaImage = image::open(entry.path())
                .unwrap_or_else(|e| panic!("Failed to open '{}': {}", entry.path().display(), e))
                .to_rgba8();
            let (terrain_entry, resolved): (TerrainTypeDef, bool) =
                terrain_entry_from_image(key, &mut_img, min_cluster_gap);
            if !resolved {
                flag_unresolved(&mut mut_img);
                let flagged_path: PathBuf = output_dir.join(format!("{}_unresolved.png", key));
                mut_img.save(&flagged_path).unwrap_or_else(|e| {
                    panic!("Failed to save '{}': {}", flagged_path.display(), e)
                });
                eprintln!("UNRESOLVED {} -> {}", file_name, flagged_path.display());
            }
            Some(terrain_entry)
        })
        .collect();
    mut_entries.sort_by(|a, b| a.key.cmp(&b.key));
    mut_entries
}

// =========================================================================
// Two-phase conversion core (ART -> cleaned data -> collision -> alpha)
// =========================================================================

// A packed mask bit is only meaningful together with its alpha: air is transparent, ground solid.
const ALPHA_AIR: u8 = 0x00;
const ALPHA_SOLID: u8 = 0xFF;

// Bake the packed mask into the image alpha channel. A flagged cell keeps its art, gets a red
// border for the human, and emits no collision bytes at all -- not even the classifier's guess.
fn fn_bake_alpha_or_flag(image: RgbaImage, solid_mask: &[u8], resolved: bool) -> RgbaImage {
    if !resolved {
        let mut mut_flagged: RgbaImage = image;
        flag_unresolved(&mut mut_flagged);
        return mut_flagged;
    }
    let mut mut_baked: RgbaImage = image;
    (0..mut_baked.height()).for_each(|row| {
        (0..mut_baked.width()).for_each(|col| {
            // One bit per pixel in row-major order, so mask length is independent of pixel width.
            let bit_index: usize = (row * mut_baked.width() + col) as usize;
            let solid: bool = solid_mask[bit_index / 8] & (1 << (bit_index % 8)) != 0;
            let pixel: image::Rgba<u8> = *mut_baked.get_pixel(col, row);
            let alpha: u8 = match solid {
                true => ALPHA_SOLID,
                false => ALPHA_AIR,
            };
            mut_baked.put_pixel(col, row, image::Rgba([pixel[0], pixel[1], pixel[2], alpha]));
        });
    });
    mut_baked
}

// Crop one authored cell at the grid pitch, resize it to the configured tile size, then apply the
// authored flip. Collision is derived from this final art, so left and right sets cannot disagree.
fn fn_crop_resize_flip_cell(
    cell: &CellRequest,
    request: &ConversionRequest,
    sheet: &RgbaImage,
) -> RgbaImage {
    let pitch_x_px: i32 = request.cell_size_px.x + request.separation_px.x;
    let pitch_y_px: i32 = request.cell_size_px.y + request.separation_px.y;
    let origin_x_px: u32 = (request.margins_px.x + cell.source_col * pitch_x_px) as u32;
    let origin_y_px: u32 = (request.margins_px.y + cell.source_row * pitch_y_px) as u32;
    let cropped: RgbaImage = image::imageops::crop_imm(
        sheet,
        origin_x_px,
        origin_y_px,
        request.cell_size_px.x as u32,
        request.cell_size_px.y as u32,
    )
    .to_image();
    let resized: RgbaImage = image::imageops::resize(
        &cropped,
        request.width_px,
        request.height_px,
        image::imageops::FilterType::Nearest,
    );
    if cell.flip_h {
        image::imageops::flip_horizontal(&resized)
    } else {
        resized
    }
}

// Derive the mask from finished art and bake it into alpha. An inconclusive cell returns an empty
// mask so no caller can mistake the classifier's best guess for a resolved collision value.
fn fn_mask_and_bake_cell(
    artwork: RgbaImage,
    request: &ConversionRequest,
) -> (RgbaImage, Vec<u8>, bool) {
    let greyscale: Vec<u8> = to_greyscale(&artwork);
    let (solid_mask, resolved): (Vec<u8>, bool) = derive_solid_mask(
        &greyscale,
        request.width_px,
        request.height_px,
        DEFAULT_MIN_CLUSTER_GAP,
    );
    let baked: RgbaImage = fn_bake_alpha_or_flag(artwork, &solid_mask, resolved);
    let emitted_mask: Vec<u8> = match resolved {
        true => solid_mask,
        false => Vec::new(),
    };
    (baked, emitted_mask, resolved)
}

// Save each finished cell and collect its output plus any flagged key. A flagged key is recorded
// so the whole commit can be refused before a caller trusts a half-converted set.
fn fn_emit_cell_outputs(
    request: &ConversionRequest,
    sheet: &RgbaImage,
    output_dir: &Path,
) -> (Vec<ConversionOutput>, Vec<String>) {
    let mut mut_outputs: Vec<ConversionOutput> = Vec::new();
    let mut mut_unresolved: Vec<String> = Vec::new();
    request.cells.iter().for_each(|cell| {
        let artwork: RgbaImage = fn_crop_resize_flip_cell(cell, request, sheet);
        let (cell_image, solid_mask, resolved): (RgbaImage, Vec<u8>, bool) =
            fn_mask_and_bake_cell(artwork, request);
        let image_path: PathBuf = output_dir.join(format!(
            "{}_{}x{}.png",
            cell.generated_key, request.width_px, request.height_px
        ));
        cell_image
            .save(&image_path)
            .unwrap_or_else(|e| panic!("Failed to save '{}': {}", image_path.display(), e));
        eprintln!(
            "CONVERT key={} path={} mask_bytes={} resolved={}",
            cell.generated_key,
            image_path.display(),
            solid_mask.len(),
            resolved
        );
        if !resolved {
            mut_unresolved.push(cell.generated_key.clone());
        }
        mut_outputs.push(ConversionOutput {
            generated_key: cell.generated_key.clone(),
            image_path,
            solid_mask,
            resolved,
        });
    });
    (mut_outputs, mut_unresolved)
}

// Convert every cell into its final PNG and mask. A stale request or one flagged cell refuses the
// whole result; the outputs are sorted by key so two runs serialize byte-identically.
fn fn_convert_resource(
    request: &ConversionRequest,
    output_dir: &Path,
) -> Result<ConversionResult, ConvertError> {
    if request.version != REQUEST_VERSION {
        return Err(ConvertError::Version(request.version.clone()));
    }
    let sheet: RgbaImage = image::open(&request.source_image)
        .map_err(|e| ConvertError::Io(format!("{}: {}", request.source_image, e)))?
        .to_rgba8();
    std::fs::create_dir_all(output_dir)
        .map_err(|e| ConvertError::Io(format!("{}: {}", output_dir.display(), e)))?;
    let (mut mut_outputs, unresolved): (Vec<ConversionOutput>, Vec<String>) =
        fn_emit_cell_outputs(request, &sheet, output_dir);
    if !unresolved.is_empty() {
        return Err(ConvertError::UnresolvedCells(unresolved));
    }
    mut_outputs.sort_by(|left, right| left.generated_key.cmp(&right.generated_key));
    Ok(ConversionResult {
        version: RESULT_VERSION.to_string(),
        outputs: mut_outputs,
    })
}

fn fn_merge_terrain_entries(
    base: &[TerrainTypeDef],
    outputs: &[ConversionOutput],
) -> Vec<TerrainTypeDef> {
    let mut mut_by_key: std::collections::BTreeMap<String, TerrainTypeDef> = base
        .iter()
        .cloned()
        .map(|entry| (entry.key.clone(), entry))
        .collect();
    outputs.iter().for_each(|output| {
        let cell_image: RgbaImage = image::open(&output.image_path)
            .unwrap_or_else(|e| panic!("Failed to open '{}': {}", output.image_path.display(), e))
            .to_rgba8();
        let entry: TerrainTypeDef = TerrainTypeDef {
            solid_mask: output.solid_mask.clone(),
            ..terrain_entry_from_image(&output.generated_key, &cell_image, DEFAULT_MIN_CLUSTER_GAP)
                .0
        };
        mut_by_key.insert(entry.key.clone(), entry);
        eprintln!(
            "REGISTER key={} mask_bytes={}",
            output.generated_key,
            output.solid_mask.len()
        );
    });
    mut_by_key.into_values().collect()
}

// One grid's measured geometry, carried between the crop, seam-clear and cell-assembly nodes.
struct SheetGrid {
    regions: Vec<(i32, i32)>,
    cols: i32,
    rows: i32,
    pitch_x_px: i32,
    pitch_y_px: i32,
    crop_rect: PixelRect,
}

// The authored left and right cells share source art, so measure each distinct region once and
// derive the whole-grid crop rectangle from the largest source column and row.
fn fn_measure_grid(request: &ConversionRequest) -> SheetGrid {
    let mut mut_seen: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
    let regions: Vec<(i32, i32)> = request
        .cells
        .iter()
        .filter_map(|cell| {
            mut_seen
                .insert((cell.source_col, cell.source_row))
                .then_some((cell.source_col, cell.source_row))
        })
        .collect();
    let cols: i32 = regions
        .iter()
        .map(|(source_col, _)| *source_col)
        .max()
        .unwrap_or(0)
        + 1;
    let rows: i32 = regions
        .iter()
        .map(|(_, source_row)| *source_row)
        .max()
        .unwrap_or(0)
        + 1;
    let pitch_x_px: i32 = request.cell_size_px.x + request.separation_px.x;
    let pitch_y_px: i32 = request.cell_size_px.y + request.separation_px.y;
    let crop_rect: PixelRect = PixelRect {
        x: request.margins_px.x as u32,
        y: request.margins_px.y as u32,
        width_px: (cols * request.cell_size_px.x + (cols - 1) * request.separation_px.x) as u32,
        height_px: (rows * request.cell_size_px.y + (rows - 1) * request.separation_px.y) as u32,
    };
    SheetGrid {
        regions,
        cols,
        rows,
        pitch_x_px,
        pitch_y_px,
        crop_rect,
    }
}

// Clear the authored separation strips to transparent air. Count from geometry, not pixels, so a
// strip intersection is counted once per strip rather than once per cell it touches.
fn fn_clear_grid_lines(crop: &mut RgbaImage, grid: &SheetGrid, request: &ConversionRequest) -> u32 {
    let air_pixel: image::Rgba<u8> = image::Rgba([0, 0, 0, ALPHA_AIR]);
    (0..grid.crop_rect.height_px).for_each(|row| {
        (0..grid.crop_rect.width_px).for_each(|col| {
            let in_vertical_seam: bool = (col as i32) % grid.pitch_x_px >= request.cell_size_px.x;
            let in_horizontal_seam: bool = (row as i32) % grid.pitch_y_px >= request.cell_size_px.y;
            if in_vertical_seam || in_horizontal_seam {
                crop.put_pixel(col, row, air_pixel);
            }
        });
    });
    ((grid.cols - 1) * request.separation_px.x * grid.crop_rect.height_px as i32
        + (grid.rows - 1) * request.separation_px.y * grid.crop_rect.width_px as i32) as u32
}

// Slice one cleaned region at the grid pitch and resize it to the configured tile size, then reuse
// the conversion mask-and-bake node so Phase I and Phase II cannot derive different collisions.
fn fn_clean_cell(
    source_col: i32,
    source_row: i32,
    crop: &RgbaImage,
    grid: &SheetGrid,
    request: &ConversionRequest,
) -> (RgbaImage, Vec<u8>, bool) {
    let origin_x_px: u32 = (source_col * grid.pitch_x_px) as u32;
    let origin_y_px: u32 = (source_row * grid.pitch_y_px) as u32;
    let sliced: RgbaImage = image::imageops::crop_imm(
        crop,
        origin_x_px,
        origin_y_px,
        request.cell_size_px.x as u32,
        request.cell_size_px.y as u32,
    )
    .to_image();
    let resized: RgbaImage = image::imageops::resize(
        &sliced,
        request.width_px,
        request.height_px,
        image::imageops::FilterType::Nearest,
    );
    fn_mask_and_bake_cell(resized, request)
}

// Assemble the cleaned cells into one sheet at the configured tile size and attach the rect and
// dimensions the sidecar needs. Placement follows region order, not the authored source order.
fn fn_assemble_cells(
    crop: &RgbaImage,
    grid: &SheetGrid,
    request: &ConversionRequest,
) -> (RgbaImage, Vec<CleanedCell>) {
    let mut mut_cleaned: RgbaImage = RgbaImage::from_pixel(
        grid.cols as u32 * request.width_px,
        grid.rows as u32 * request.height_px,
        image::Rgba([0, 0, 0, ALPHA_AIR]),
    );
    let cells: Vec<CleanedCell> = grid
        .regions
        .iter()
        .enumerate()
        .map(|(index, (source_col, source_row))| {
            let (cell_image, solid_mask, resolved): (RgbaImage, Vec<u8>, bool) =
                fn_clean_cell(*source_col, *source_row, crop, grid, request);
            let local_col: u32 = index as u32 % grid.cols as u32;
            let local_row: u32 = index as u32 / grid.cols as u32;
            image::imageops::overlay(
                &mut mut_cleaned,
                &cell_image,
                (local_col * request.width_px) as i64,
                (local_row * request.height_px) as i64,
            );
            eprintln!(
                "CLEAN col={} row={} mask_bytes={} resolved={}",
                source_col,
                source_row,
                solid_mask.len(),
                resolved
            );
            CleanedCell {
                source_col: *source_col,
                source_row: *source_row,
                pixel_rect: PixelRect {
                    x: local_col * request.width_px,
                    y: local_row * request.height_px,
                    width_px: request.width_px,
                    height_px: request.height_px,
                },
                dimensions: CellDimensions {
                    width_px: request.width_px,
                    height_px: request.height_px,
                },
                solid_mask,
                resolved,
            }
        })
        .collect();
    (mut_cleaned, cells)
}

// Phase I crops the whole grid once, clears its separation lines, then slices each region into a
// configured-size cell and assembles one cleaned sheet for a human to inspect before Phase II.
fn preprocess_source_sheet(
    request: &ConversionRequest,
    output_dir: &Path,
) -> Result<PreprocessResult, ConvertError> {
    if request.version != REQUEST_VERSION {
        return Err(ConvertError::Version(request.version.clone()));
    }
    let sheet: RgbaImage = image::open(&request.source_image)
        .map_err(|e| ConvertError::Io(format!("{}: {}", request.source_image, e)))?
        .to_rgba8();
    std::fs::create_dir_all(output_dir)
        .map_err(|e| ConvertError::Io(format!("{}: {}", output_dir.display(), e)))?;
    let grid: SheetGrid = fn_measure_grid(request);
    let mut mut_crop: RgbaImage = image::imageops::crop_imm(
        &sheet,
        grid.crop_rect.x,
        grid.crop_rect.y,
        grid.crop_rect.width_px,
        grid.crop_rect.height_px,
    )
    .to_image();
    let cleared_grid_line_pixels: u32 = fn_clear_grid_lines(&mut mut_crop, &grid, request);
    let (mut_cleaned, cells): (RgbaImage, Vec<CleanedCell>) =
        fn_assemble_cells(&mut_crop, &grid, request);
    let cleaned_image_path: PathBuf = output_dir.join("slope_cleaned.png");
    mut_cleaned
        .save(&cleaned_image_path)
        .unwrap_or_else(|e| panic!("Failed to save '{}': {}", cleaned_image_path.display(), e));
    Ok(PreprocessResult {
        crop_rect: grid.crop_rect,
        cleaned_image_path,
        cells,
        cleared_grid_line_pixels,
    })
}

// Phase II is a human gate, not a timer. Without sign-off, or with a flagged cell, it refuses and
// writes no sidecar, so an approved annotation can never be mistaken for an unresolved conversion.
fn fn_phase_two_gate(
    preprocessed: &PreprocessResult,
    signed_off: bool,
    sidecar_path: &Path,
) -> Result<ConversionSidecar, ConvertError> {
    if !signed_off {
        return Err(ConvertError::Unsigned);
    }
    let unresolved: Vec<String> = preprocessed
        .cells
        .iter()
        .filter(|cell| !cell.resolved)
        .map(|cell| format!("({}, {})", cell.source_col, cell.source_row))
        .collect();
    if !unresolved.is_empty() {
        return Err(ConvertError::UnresolvedCells(unresolved));
    }
    let sidecar: ConversionSidecar = ConversionSidecar {
        version: REQUEST_VERSION.to_string(),
        tile_width_px: preprocessed
            .cells
            .first()
            .map(|cell| cell.dimensions.width_px)
            .unwrap_or(0),
        tile_height_px: preprocessed
            .cells
            .first()
            .map(|cell| cell.dimensions.height_px)
            .unwrap_or(0),
        mask_encoding: MASK_ENCODING_GREYSCALE.to_string(),
        tiles: preprocessed
            .cells
            .iter()
            .map(|cell| SidecarRow {
                source_col: cell.source_col,
                source_row: cell.source_row,
                pixel_rect: cell.pixel_rect,
                dimensions: cell.dimensions,
            })
            .collect(),
    };
    let text: String =
        serde_json::to_string_pretty(&sidecar).map_err(|e| ConvertError::Decode(e.to_string()))?;
    std::fs::write(sidecar_path, text)
        .map_err(|e| ConvertError::Io(format!("{}: {}", sidecar_path.display(), e)))?;
    eprintln!(
        "SIDECAR path={} rows={}",
        sidecar_path.display(),
        sidecar.tiles.len()
    );
    Ok(sidecar)
}

// Read the human-edited JSONC sidecar: strip line comments so an annotation never breaks the Rust
// consumer, and never recompute geometry from the source art.
fn read_sidecar(path: &Path) -> Result<ConversionSidecar, ConvertError> {
    let raw_text: String = std::fs::read_to_string(path)
        .map_err(|e| ConvertError::Io(format!("{}: {}", path.display(), e)))?;
    let json_text: String = raw_text
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<&str>>()
        .join("\n");
    let sidecar: ConversionSidecar = serde_json::from_str(&json_text)
        .map_err(|e| ConvertError::Decode(format!("{}: {}", path.display(), e)))?;
    eprintln!(
        "SIDECAR-READ path={} rows={}",
        path.display(),
        sidecar.tiles.len()
    );
    Ok(sidecar)
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
    let mut mut_config_path: Option<String> = None;
    let mut mut_input_override: Option<String> = None;
    let mut mut_out_override: Option<String> = None;

    // Walk argv once, pairing each value-taking flag with the token immediately after it.
    let mut mut_arg_index: usize = 1;
    while mut_arg_index < args.len() {
        match args[mut_arg_index].as_str() {
            // The config names every default the other two flags can replace.
            "--config" => {
                mut_arg_index += 1;
                mut_config_path = Some(args[mut_arg_index].clone());
            }
            "--input" => {
                mut_arg_index += 1;
                mut_input_override = Some(args[mut_arg_index].clone());
            }
            "--out-dir" => {
                mut_arg_index += 1;
                mut_out_override = Some(args[mut_arg_index].clone());
            }
            // Help is the only flag that can succeed without a config.
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            // An unrecognised flag is a usage error, so fail before any file is opened.
            _ => {
                eprintln!("Unknown argument: {}", args[mut_arg_index]);
                print_usage();
                std::process::exit(1);
            }
        }
        mut_arg_index += 1;
    }

    // A missing config is fatal; absent overrides simply keep the config's own values.
    let config_path: String = mut_config_path.unwrap_or_else(|| {
        eprintln!("Error: --config is required");
        print_usage();
        std::process::exit(1);
    });

    (config_path, mut_input_override, mut_out_override)
}

// Derived grid geometry shared by both importer emitters. Inferred offsets and dimensions
// are copied here instead of being written back into the parsed config.
struct ImportSheetGrid {
    sheet_width_px: u32,
    sheet_height_px: u32,
    tile_size_px: u32,
    source_pitch_w: u32,
    source_pitch_h: u32,
    grid_cols: u32,
    grid_rows: u32,
    offset_x_px: u32,
    offset_y_px: u32,
}

// Read the config once and let the two command-line overrides replace its defaults.
fn fn_load_import_config(
    config_path: &str,
    input_override: Option<String>,
    out_override: Option<String>,
) -> ImportConfig {
    let config_text: String = std::fs::read_to_string(config_path)
        .unwrap_or_else(|e| panic!("Failed to read config '{}': {}", config_path, e));
    let mut mut_config: ImportConfig = serde_json::from_str(&config_text)
        .unwrap_or_else(|e| panic!("Failed to parse config '{}': {}", config_path, e));
    if let Some(ref path) = input_override {
        mut_config.input = Some(path.clone());
    }
    if let Some(ref path) = out_override {
        mut_config.output_dir = path.clone();
    }
    mut_config
}

// Tile-scan mode writes the catalog and reports whether it handled this run; it needs no sheet.
fn fn_run_tile_scan(config: &ImportConfig) -> bool {
    let scan_dir: &String = match config.scan_tiles.as_ref() {
        Some(scan_dir) => scan_dir,
        None => return false,
    };
    let key_prefix: &str = config.scan_prefix.as_deref().unwrap_or("");
    std::fs::create_dir_all(&config.output_dir)
        .unwrap_or_else(|e| panic!("Failed to create '{}': {}", config.output_dir, e));
    let entries: Vec<TerrainTypeDef> = scan_tile_directory(
        Path::new(scan_dir),
        key_prefix,
        config.min_cluster_gap,
        Path::new(&config.output_dir),
    );
    let terrain_file: TerrainTypesFile = TerrainTypesFile {
        version: FILE_VERSION.into(),
        tiles: entries,
    };
    let terrain_json: String = serde_json::to_string_pretty(&terrain_file)
        .expect("Failed to serialize terrain_types.json");
    let terrain_json_path: PathBuf = PathBuf::from(&config.output_dir).join("terrain_types.json");
    std::fs::write(&terrain_json_path, &terrain_json)
        .unwrap_or_else(|e| panic!("Failed to write '{}': {}", terrain_json_path.display(), e));
    println!(
        "\nWrote {} ({} tiles)",
        terrain_json_path.display(),
        terrain_file.tiles.len()
    );
    true
}

// Coarse content bounds and explicit grid settings resolve to one read-only grid description.
fn fn_build_grid(config: &ImportConfig, sheet: &image::DynamicImage) -> ImportSheetGrid {
    let sheet_width_px: u32 = sheet.width();
    let sheet_height_px: u32 = sheet.height();
    let tile_size_px: u32 = config.tile_size_px;

    // Auto-bounding reads a greyscale view only when it is enabled; explicit grids skip it.
    let bounds: Option<ContentBounds> = match config.auto_bounding {
        true => {
            let detected: ContentBounds = detect_content_bounds(&sheet.to_rgba8());
            eprintln!(
                "Auto-bounding: content_area=({},{})->({},{})  pitch=({},{})",
                detected.left,
                detected.top,
                detected.right,
                detected.bottom,
                detected.pitch_w,
                detected.pitch_h,
            );
            Some(detected)
        }
        false => None,
    };

    // An authored grid fixes the pitch; otherwise the blank-space scan supplies it.
    let source_pitch_w: u32 = match (config.grid_cols.is_some(), bounds.as_ref()) {
        (true, _) => tile_size_px + config.margin,
        (false, Some(detected)) => detected.pitch_w.max(1),
        (false, None) => tile_size_px + config.margin,
    };
    let source_pitch_h: u32 = match (config.grid_rows.is_some(), bounds.as_ref()) {
        (true, _) => tile_size_px + config.margin,
        (false, Some(detected)) => detected.pitch_h.max(1),
        (false, None) => tile_size_px + config.margin,
    };

    let (mut mut_offset_x_px, mut mut_offset_y_px) = (config.offset_x, config.offset_y);
    let (mut mut_grid_cols, mut mut_grid_rows) = (config.grid_cols, config.grid_rows);
    // Without an authored grid, snap the detected content area out to whole cells.
    if config.auto_bounding && mut_grid_cols.is_none() && mut_grid_rows.is_none() {
        if let Some(detected) = bounds.as_ref() {
            mut_offset_x_px = (detected.left / source_pitch_w) * source_pitch_w;
            mut_offset_y_px = (detected.top / source_pitch_h) * source_pitch_h;
            mut_grid_cols = Some(
                ((detected.right - mut_offset_x_px + source_pitch_w - 1) / source_pitch_w).max(1),
            );
            mut_grid_rows = Some(
                ((detected.bottom - mut_offset_y_px + source_pitch_h - 1) / source_pitch_h).max(1),
            );
        }
    }
    let grid_cols: u32 = mut_grid_cols.expect("Either set grid dims or enable auto-bounding");
    let grid_rows: u32 = mut_grid_rows.expect("Either set grid dims or enable auto-bounding");

    // Report the resolved geometry, then warn rather than fail on an undersized sheet.
    if config.auto_bounding {
        eprintln!(
            "Auto-bounding: pitch={}x{}, offset=({},{}), grid={}x{}",
            source_pitch_w, source_pitch_h, mut_offset_x_px, mut_offset_y_px, grid_cols, grid_rows,
        );
    }
    let expected_width_px: u32 = mut_offset_x_px + grid_cols * source_pitch_w;
    let expected_height_px: u32 = mut_offset_y_px + grid_rows * source_pitch_h;
    if sheet_width_px < expected_width_px || sheet_height_px < expected_height_px {
        eprintln!(
            "Warning: spritesheet {}x{} is smaller than expected {}x{} \
             (grid {}x{} at source pitch {}x{} with offset {}, {})",
            sheet_width_px,
            sheet_height_px,
            expected_width_px,
            expected_height_px,
            grid_cols,
            grid_rows,
            source_pitch_w,
            source_pitch_h,
            mut_offset_x_px,
            mut_offset_y_px,
        );
    }

    ImportSheetGrid {
        sheet_width_px,
        sheet_height_px,
        tile_size_px,
        source_pitch_w,
        source_pitch_h,
        grid_cols,
        grid_rows,
        offset_x_px: mut_offset_x_px,
        offset_y_px: mut_offset_y_px,
    }
}

// Reverse index lets each grid cell resolve an authored terrain entry in one lookup.
fn fn_build_terrain_index(config: &ImportConfig) -> HashMap<usize, TerrainMapEntry> {
    let mut mut_index: HashMap<usize, TerrainMapEntry> = HashMap::new();
    for entry in config.terrain_map.values() {
        for &grid_index in &entry.indices {
            mut_index.insert(grid_index, entry.clone());
        }
    }
    mut_index
}

// Walk one grid cell: crop at pitch, scale the shipped art, then save PNG and catalog entry.
fn fn_emit_tiles(
    sheet: &image::DynamicImage,
    config: &ImportConfig,
    grid: &ImportSheetGrid,
    index_to_terrain: &HashMap<usize, TerrainMapEntry>,
    tiles_dir: &Path,
) -> (HashMap<String, TerrainTypeDef>, u32) {
    let mut mut_terrain_defs: HashMap<String, TerrainTypeDef> = HashMap::new();
    let mut mut_total_tiles: u32 = 0;

    for grid_row in 0..grid.grid_rows {
        for grid_col in 0..grid.grid_cols {
            // Cells sit at pitch intervals from the content origin, not from authoring metadata.
            let cell_origin_x_px: u32 = grid.offset_x_px + grid_col * grid.source_pitch_w;
            let cell_origin_y_px: u32 = grid.offset_y_px + grid_row * grid.source_pitch_h;
            let mut mut_cell_image = sheet.crop_imm(
                cell_origin_x_px,
                cell_origin_y_px,
                grid.source_pitch_w,
                grid.source_pitch_h,
            );

            // Scale before collision detection so the mask describes the art that ships.
            if config.auto_scale
                && (mut_cell_image.width() != grid.tile_size_px
                    || mut_cell_image.height() != grid.tile_size_px)
            {
                mut_cell_image = mut_cell_image.resize_exact(
                    grid.tile_size_px,
                    grid.tile_size_px,
                    image::imageops::FilterType::Nearest,
                );
            }

            // Both the key lookup and the mask detector read the final RGBA view.
            let cell_rgba: image::RgbaImage = mut_cell_image.to_rgba8();
            let grid_index: usize = (grid_row * grid.grid_cols + grid_col) as usize;
            let terrain_key: String = match index_to_terrain.get(&grid_index) {
                Some(entry) => entry.key.clone(),
                None => auto_detect_terrain_key(&cell_rgba).to_string(),
            };

            // Collision follows the final art; cells without detection stay fully solid.
            let mask: u8 = match config.auto_detect_collision {
                true => auto_detect_sub_tile_mask(&cell_rgba, config.luminance_threshold),
                false => SUB_TILE_MASK_ALL_SOLID,
            };

            // The key is the catalog identity, so filename and JSON entry share it.
            let tile_key: String = format!("{}_{}_{}", terrain_key, grid_row, grid_col);
            let tile_path: PathBuf = tiles_dir.join(format!("{}_32x32.png", tile_key));
            mut_cell_image.save(&tile_path).unwrap_or_else(|error| {
                panic!("Failed to save tile '{}': {}", tile_path.display(), error)
            });

            // Register each key once; later cells of the same key only add their PNG.
            if !mut_terrain_defs.contains_key(&terrain_key) {
                let entry: Option<&TerrainMapEntry> = index_to_terrain.get(&grid_index);
                let terrain_def: TerrainTypeDef = TerrainTypeDef {
                    key: terrain_key.clone(),
                    display_name: entry
                        .map(|authored| authored.display_name.clone())
                        .unwrap_or_else(|| terrain_key.clone()),
                    is_walkable: entry
                        .map(|authored| authored.is_walkable)
                        .unwrap_or_else(|| {
                            mask != 0
                                && terrain_key != AIR_TERRAIN_KEY
                                && terrain_key != WALL_TERRAIN_KEY
                        }),
                    is_buildable: entry.map(|authored| authored.is_buildable).unwrap_or_else(
                        || {
                            mask != 0
                                && terrain_key != AIR_TERRAIN_KEY
                                && terrain_key != WALL_TERRAIN_KEY
                                && terrain_key != WATER_TERRAIN_KEY
                                && terrain_key != LAVA_TERRAIN_KEY
                        },
                    ),
                    surface: entry
                        .map(|authored| authored.surface.clone())
                        .unwrap_or_else(|| NORMAL_SURFACE.into()),
                    hazard: entry
                        .map(|authored| authored.hazard.clone())
                        .unwrap_or_else(|| NO_HAZARD.into()),
                    elevation_tiles: 0,
                    color_hex: entry
                        .map(|authored| authored.color_hex.clone())
                        .unwrap_or_else(|| {
                            let (red, green, blue, _) = average_color(&cell_rgba);
                            format!("#{:02x}{:02x}{:02x}", red, green, blue)
                        }),
                    sub_tile_mask: mask,
                    solid_mask: Vec::new(),
                    hazard_damage_per_tick: 0,
                    is_destructible: false,
                    destructible_hp: 0,
                    on_destroy_terrain_key: String::new(),
                };
                mut_terrain_defs.insert(terrain_key.clone(), terrain_def);
            }

            // Log key, path and mask so a finished run can be traced from the console.
            mut_total_tiles += 1;
            println!(
                "  [{:3}] {} -> {}  mask=0x{:X}",
                grid_index,
                tile_key,
                tile_path.file_name().unwrap().to_string_lossy(),
                mask
            );
        }
    }
    (mut_terrain_defs, mut_total_tiles)
}

// Sort by key and write the catalog the framework merges into its palette.
fn fn_write_terrain_types(config: &ImportConfig, terrain_defs: HashMap<String, TerrainTypeDef>) {
    let mut mut_tiles_sorted: Vec<TerrainTypeDef> = terrain_defs.into_values().collect();
    mut_tiles_sorted.sort_by(|left, right| left.key.cmp(&right.key));
    let terrain_file: TerrainTypesFile = TerrainTypesFile {
        version: FILE_VERSION.into(),
        tiles: mut_tiles_sorted,
    };
    let terrain_json: String = serde_json::to_string_pretty(&terrain_file)
        .expect("Failed to serialize terrain_types.json");
    let terrain_json_path: PathBuf = PathBuf::from(&config.output_dir).join("terrain_types.json");
    std::fs::write(&terrain_json_path, &terrain_json)
        .unwrap_or_else(|e| panic!("Failed to write '{}': {}", terrain_json_path.display(), e));
    println!("\nWrote {}", terrain_json_path.display());
}

// Classify a cell's own art only when neither an override nor an authored index names it.
fn fn_terrain_key_at_cell(
    sheet: &image::DynamicImage,
    grid: &ImportSheetGrid,
    grid_col: u32,
    grid_row: u32,
) -> String {
    let cell_origin_x_px: u32 = grid.offset_x_px + grid_col * grid.source_pitch_w;
    let cell_origin_y_px: u32 = grid.offset_y_px + grid_row * grid.source_pitch_h;
    let cell_within_sheet: bool = cell_origin_x_px + grid.source_pitch_w <= grid.sheet_width_px
        && cell_origin_y_px + grid.source_pitch_h <= grid.sheet_height_px;
    match cell_within_sheet {
        true => {
            let cell_image = sheet.crop_imm(
                cell_origin_x_px,
                cell_origin_y_px,
                grid.source_pitch_w,
                grid.source_pitch_h,
            );
            auto_detect_terrain_key(&cell_image.to_rgba8()).to_string()
        }
        false => AIR_TERRAIN_KEY.into(),
    }
}

// Collision for tile-set cells reads the unscaled sheet art; outside it, the cell is solid.
fn fn_cell_sub_tile_mask(
    sheet: &image::DynamicImage,
    config: &ImportConfig,
    grid: &ImportSheetGrid,
    grid_col: u32,
    grid_row: u32,
) -> u8 {
    let cell_origin_x_px: u32 = grid.offset_x_px + grid_col * grid.source_pitch_w;
    let cell_origin_y_px: u32 = grid.offset_y_px + grid_row * grid.source_pitch_h;
    let collision_in_bounds: bool = config.auto_detect_collision
        && cell_origin_x_px + grid.source_pitch_w <= grid.sheet_width_px
        && cell_origin_y_px + grid.source_pitch_h <= grid.sheet_height_px;
    match collision_in_bounds {
        true => {
            let cell_image = sheet.crop_imm(
                cell_origin_x_px,
                cell_origin_y_px,
                grid.source_pitch_w,
                grid.source_pitch_h,
            );
            auto_detect_sub_tile_mask(&cell_image.to_rgba8(), config.luminance_threshold)
        }
        false => SUB_TILE_MASK_ALL_SOLID,
    }
}

// Emit one tile set in local row-major order: override, authored index, then visual guess.
fn fn_emit_tile_set(
    tile_set_def: &TileSetDef,
    config: &ImportConfig,
    sheet: &image::DynamicImage,
    grid: &ImportSheetGrid,
    index_to_terrain: &HashMap<usize, TerrainMapEntry>,
) -> TileSetOutput {
    let mut mut_entries: Vec<TileSetEntry> = Vec::new();
    for local_y in 0..tile_set_def.height {
        for local_x in 0..tile_set_def.width {
            let grid_row: u32 = tile_set_def.row_start + local_y;
            let grid_col: u32 = tile_set_def.col_start + local_x;
            let grid_index: usize = (grid_row * grid.grid_cols + grid_col) as usize;
            let cell_key: String = format!("{},{}", local_x, local_y);
            let terrain_key: String = match tile_set_def.cell_overrides.get(&cell_key) {
                Some(override_key) => override_key.clone(),
                None => match index_to_terrain.get(&grid_index) {
                    Some(entry) => entry.key.clone(),
                    None => fn_terrain_key_at_cell(sheet, grid, grid_col, grid_row),
                },
            };
            mut_entries.push(TileSetEntry {
                local_x: local_x as i32,
                local_y: local_y as i32,
                terrain_key,
                elevation_tiles: 0,
                z_depth: 0,
                sub_tile_mask: fn_cell_sub_tile_mask(sheet, config, grid, grid_col, grid_row),
            });
        }
    }
    TileSetOutput {
        key: tile_set_def.key.clone(),
        display_name: tile_set_def.display_name.clone(),
        width_tiles: tile_set_def.width as i32,
        height_tiles: tile_set_def.height as i32,
        tiles: mut_entries,
        tags: tile_set_def.tags.clone(),
    }
}

// Write tile_sets.json only when the config authored at least one grouped tile set.
fn fn_emit_tile_sets(
    config: &ImportConfig,
    sheet: &image::DynamicImage,
    grid: &ImportSheetGrid,
    index_to_terrain: &HashMap<usize, TerrainMapEntry>,
) {
    if config.tile_sets.is_empty() {
        return;
    }
    let mut mut_sets_out: Vec<TileSetOutput> = Vec::new();
    for tile_set_def in &config.tile_sets {
        mut_sets_out.push(fn_emit_tile_set(
            tile_set_def,
            config,
            sheet,
            grid,
            index_to_terrain,
        ));
    }
    let tile_sets_file: TileSetsFile = TileSetsFile {
        version: FILE_VERSION.into(),
        tile_sets: mut_sets_out,
    };
    let sets_json: String =
        serde_json::to_string_pretty(&tile_sets_file).expect("Failed to serialize tile_sets.json");
    let sets_json_path: PathBuf = PathBuf::from(&config.output_dir).join("tile_sets.json");
    std::fs::write(&sets_json_path, &sets_json)
        .unwrap_or_else(|e| panic!("Failed to write '{}': {}", sets_json_path.display(), e));
    println!("Wrote {}", sets_json_path.display());
}

// =========================================================================
// Main
// =========================================================================

fn main() {
    // The orchestrator invokes `convert-slope` with a request path, an output directory and an
    // explicit sign-off flag. It is the only entry that runs both phases and prints the result.
    fn fn_run_convert_slope(
        request_path: &Path,
        output_dir: &Path,
        signed_off: bool,
    ) -> Result<(), ConvertError> {
        let request_text: String = std::fs::read_to_string(request_path)
            .map_err(|e| ConvertError::Io(format!("{}: {}", request_path.display(), e)))?;
        let request: ConversionRequest = serde_json::from_str(&request_text)
            .map_err(|e| ConvertError::Decode(format!("{}: {}", request_path.display(), e)))?;
        let preprocessed: PreprocessResult = preprocess_source_sheet(&request, output_dir)?;
        let sidecar_path: PathBuf = output_dir.join("slope.jsonc");
        let _sidecar: ConversionSidecar =
            fn_phase_two_gate(&preprocessed, signed_off, &sidecar_path)?;
        let result: ConversionResult = fn_convert_resource(&request, output_dir)?;
        println!(
            "{}",
            serde_json::to_string(&result).map_err(|e| ConvertError::Decode(e.to_string()))?
        );
        Ok(())
    }
    // One binary serves two argv contracts: the config-driven importer and the slope
    // conversion request. Conversion prints JSON to stdout, so it must return here;
    // falling through makes parse_args reject the positional request path.
    if std::env::args().nth(1).as_deref() == Some("convert-slope") {
        // The request path is positional; the importer's --config parser would reject it.
        let request_path: PathBuf = PathBuf::from(
            std::env::args()
                .nth(2)
                .expect("convert-slope needs a request JSON path"),
        );
        // Tiles and the sidecar both land under this directory.
        let output_dir: PathBuf = PathBuf::from(
            std::env::args()
                .nth(3)
                .expect("convert-slope needs an output directory"),
        );
        // Phase II refuses without this explicit sign-off, so a missing flag fails the
        // command instead of emitting a converted set nobody reviewed.
        let signed_off: bool = std::env::args().nth(4).as_deref() == Some("--signed-off");
        // Fail loudly on stderr with a non-zero exit so no partial result is read from stdout.
        fn_run_convert_slope(&request_path, &output_dir, signed_off).unwrap_or_else(|error| {
            eprintln!("CONVERT-SLOPE FAILED: {:?}", error);
            std::process::exit(1);
        });
        // Never fall through to the importer's parse_args in conversion mode.
        return;
    }

    // Parse argv, then resolve config defaults against the two command-line overrides.
    let (config_path, input_override, out_override) = parse_args();
    let config: ImportConfig = fn_load_import_config(&config_path, input_override, out_override);

    // The tile-scan path reads committed PNGs and needs no sheet, so it handles the run first.
    if fn_run_tile_scan(&config) {
        return;
    }

    // The spritesheet-slice path needs an explicit source sheet.
    let input_path = config
        .input
        .as_ref()
        .expect("Either set 'input' in config or pass --input");
    let sheet = image::open(input_path)
        .unwrap_or_else(|e| panic!("Failed to load spritesheet '{}': {}", input_path, e));

    // Resolve bounds, pitch and grid once so both emitters read the same geometry.
    let grid: ImportSheetGrid = fn_build_grid(&config, &sheet);
    let index_to_terrain: HashMap<usize, TerrainMapEntry> = fn_build_terrain_index(&config);

    // Every extracted PNG lands in one directory, created before the first cell is emitted.
    let tiles_dir: PathBuf = PathBuf::from(&config.output_dir).join("tiles");
    std::fs::create_dir_all(&tiles_dir).unwrap_or_else(|e| {
        panic!(
            "Failed to create tiles dir '{}': {}",
            tiles_dir.display(),
            e
        )
    });

    // Walk the grid once: one PNG per cell, one catalog entry per distinct terrain key.
    let (terrain_defs, total_tiles): (HashMap<String, TerrainTypeDef>, u32) =
        fn_emit_tiles(&sheet, &config, &grid, &index_to_terrain, &tiles_dir);

    // Persist the catalog, then any authored tile sets, against the same grid geometry.
    fn_write_terrain_types(&config, terrain_defs);
    fn_emit_tile_sets(&config, &sheet, &grid, &index_to_terrain);

    println!(
        "\nDone. {} tiles extracted to {}",
        total_tiles,
        tiles_dir.display()
    );
}

fn average_color(img: &RgbaImage) -> (u8, u8, u8, u8) {
    let (mut mut_red_sum, mut mut_green_sum, mut mut_blue_sum, mut mut_alpha_sum, mut mut_count) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    // Sum each channel once so an empty image needs only one fallback branch.
    for pixel in img.pixels() {
        mut_red_sum += u64::from(pixel[0]);
        mut_green_sum += u64::from(pixel[1]);
        mut_blue_sum += u64::from(pixel[2]);
        mut_alpha_sum += u64::from(pixel[3]);
        mut_count += 1;
    }
    match mut_count == 0 {
        true => AVERAGE_COLOR_FALLBACK_RGBA,
        false => (
            (mut_red_sum / mut_count) as u8,
            (mut_green_sum / mut_count) as u8,
            (mut_blue_sum / mut_count) as u8,
            (mut_alpha_sum / mut_count) as u8,
        ),
    }
}

// =========================================================================
// Solid-mask pipeline tests (#126, ART -> DATA)
// =========================================================================
// Synthetic RGBA tiles keep the fixtures self-contained; the committed `slope_up_*` art is pinned separately by the greys
// recorded in the repo AGENTS.md (air 0, ground 155, bottom-row anchor).
#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, Rgba};

    /// The committed slope art's two greys: air is 0, ground is 155 (MEASURED, AGENTS.md).
    const GROUND_GREY: u8 = 155;
    const AIR_GREY: u8 = 0;

    /// A lone anti-aliased edge grey: in neither cluster, so a single-pixel read is wrong.
    const EDGE_GREY: u8 = 78;

    /// A smooth ramp whose adjacent greys are all closer than `DEFAULT_MIN_CLUSTER_GAP`,
    /// so the tile has no two-cluster split at all.
    const RAMP_START_GREY: u8 = 100;
    const RAMP_STEP_GREY: u8 = 4;

    /// An opaque tile whose bottom `ground_rows` rows are ground and the rest air.
    fn tile_with_ground_rows(width_px: u32, height_px: u32, ground_rows: u32) -> RgbaImage {
        RgbaImage::from_fn(width_px, height_px, |_, row| {
            let grey = match row >= height_px - ground_rows {
                true => GROUND_GREY,
                false => AIR_GREY,
            };
            Rgba([grey, grey, grey, ALPHA_SOLID])
        })
    }

    /// Overwrite one pixel with a grey, keeping it opaque.
    fn paint_grey(img: &mut RgbaImage, col: u32, row: u32, grey: u8) {
        img.put_pixel(col, row, Rgba([grey, grey, grey, ALPHA_SOLID]));
    }

    /// Read one bit of the packed mask `derive_solid_mask` returns.
    fn mask_bit(mask: &[u8], width_px: u32, col: u32, row: u32) -> bool {
        let bit_index = (row * width_px + col) as usize;
        mask[bit_index / 8] & (1 << (bit_index % 8)) != 0
    }

    #[test]
    fn mask_two_cluster_bottom_anchor() {
        let img = tile_with_ground_rows(8, 8, 4);
        let greyscale = to_greyscale(&img);
        let (mask, resolved) = derive_solid_mask(&greyscale, 8, 8, DEFAULT_MIN_CLUSTER_GAP);

        assert!(resolved, "two clear clusters must resolve");
        (4..8).for_each(|row| {
            (0..8).for_each(|col| assert!(mask_bit(&mask, 8, col, row), "ground at ({col},{row})"));
        });
        (0..4).for_each(|row| {
            (0..8).for_each(|col| assert!(!mask_bit(&mask, 8, col, row), "air at ({col},{row})"));
        });
    }

    #[test]
    fn mask_all_grey_is_all_solid() {
        let img = tile_with_ground_rows(8, 8, 8);
        let greyscale = to_greyscale(&img);
        let (mask, resolved) = derive_solid_mask(&greyscale, 8, 8, DEFAULT_MIN_CLUSTER_GAP);

        assert!(resolved, "a single-grey tile is all solid, never flagged");
        (0..8).for_each(|row| {
            (0..8).for_each(|col| assert!(mask_bit(&mask, 8, col, row), "solid at ({col},{row})"));
        });
    }

    #[test]
    fn mask_removes_floating_solid_specks() {
        let mut mut_img = tile_with_ground_rows(8, 8, 4);
        paint_grey(&mut mut_img, 5, 1, GROUND_GREY); // a speck of ground floating in the air
        let greyscale = to_greyscale(&mut_img);
        let (mask, resolved) = derive_solid_mask(&greyscale, 8, 8, DEFAULT_MIN_CLUSTER_GAP);

        assert!(resolved);
        assert!(
            !mask_bit(&mask, 8, 5, 1),
            "a floating speck is dust, not ground"
        );
        (0..8).for_each(|col| {
            assert!(
                mask_bit(&mask, 8, col, 7),
                "the ground survives at col {col}"
            );
        });
    }

    #[test]
    fn mask_best_guess_resolves_ambiguous_sample() {
        // The bottom row carries BOTH clusters, so the anchor is ambiguous; the best guess
        // (the bottom-centre patch read) resolves it instead of flagging the tile.
        let mut mut_img = tile_with_ground_rows(8, 8, 4);
        paint_grey(&mut mut_img, 0, 7, AIR_GREY);
        let greyscale = to_greyscale(&mut_img);
        let (_, resolved) = derive_solid_mask(&greyscale, 8, 8, DEFAULT_MIN_CLUSTER_GAP);

        assert!(
            resolved,
            "an ambiguous bottom row is guessed, never flagged"
        );

        let mut mut_emitted = mut_img.clone();
        if !resolved {
            flag_unresolved(&mut mut_emitted);
        }
        assert_ne!(
            mut_emitted.get_pixel(0, 0),
            &Rgba(UNRESOLVED_BORDER_RGBA),
            "a resolved tile carries no red border"
        );
    }

    #[test]
    fn mask_flags_only_when_all_heuristics_inconclusive() {
        // Every distinct grey is within `DEFAULT_MIN_CLUSTER_GAP` of the next, so no
        // two-cluster split exists for any heuristic to anchor on: the tile is flagged.
        let img = RgbaImage::from_fn(8, 8, |_, row| {
            let grey = RAMP_START_GREY + (row as u8) * RAMP_STEP_GREY;
            Rgba([grey, grey, grey, ALPHA_SOLID])
        });
        let greyscale = to_greyscale(&img);
        let (_, resolved) = derive_solid_mask(&greyscale, 8, 8, DEFAULT_MIN_CLUSTER_GAP);

        assert!(!resolved, "a tile with no cluster split is flagged");

        let mut mut_emitted = img.clone();
        flag_unresolved(&mut mut_emitted);
        assert_eq!(
            mut_emitted.get_pixel(0, 0),
            &Rgba(UNRESOLVED_BORDER_RGBA),
            "the emitted PNG carries the red 1px border"
        );
        assert_eq!(
            mut_emitted.get_pixel(4, 4)[0],
            greyscale[(4 * 8 + 4) as usize],
            "the border never repaints the tile interior"
        );
    }

    #[test]
    fn mask_uniform_patch_ignores_edge_pixel() {
        // One anti-aliased pixel sits inside an otherwise uniform air patch: reading that
        // single pixel gives the edge grey, reading the patch gives the region's air grey.
        let mut mut_img = tile_with_ground_rows(8, 8, 4);
        paint_grey(&mut mut_img, 2, 2, EDGE_GREY);
        let greyscale = to_greyscale(&mut_img);

        assert_ne!(
            greyscale[(2 * 8 + 2) as usize],
            AIR_GREY,
            "the lone edge pixel is not the region's air grey"
        );
        assert_eq!(
            uniform_patch_grey(&greyscale, 8, 8, 2, 2, PATCH_SIZE_PX),
            AIR_GREY,
            "a 4x4 patch outvotes the lone edge pixel"
        );
    }

    #[test]
    fn terrain_entry_from_image_carries_the_mask() {
        let img = tile_with_ground_rows(32, 32, 16);
        let (entry, resolved) =
            terrain_entry_from_image("slope_up_0", &img, DEFAULT_MIN_CLUSTER_GAP);

        assert!(resolved);
        assert_eq!(entry.key, "slope_up_0");
        assert!(entry.is_walkable);
        assert_eq!(
            entry.solid_mask.len(),
            (32 * 32 + 7) / 8,
            "packed 32x32 mask"
        );
    }

    /// Copy the committed `{prefix}*_32x32.png` art into a fresh temp dir so a scan test reads
    /// the real committed art (in `git ls-files`) without writing into the source tree.
    fn temp_copy_of_tiles(prefix: &str) -> PathBuf {
        let source_dir: PathBuf =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../editor/assets/tiles");
        let temp_dir: PathBuf = std::env::temp_dir().join("import_tiles_scan_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("temp tiles dir");
        std::fs::read_dir(&source_dir)
            .expect("committed tiles dir")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|file_name| file_name.starts_with(prefix) && file_name.ends_with("_32x32.png"))
            .for_each(|file_name| {
                std::fs::copy(source_dir.join(&file_name), temp_dir.join(&file_name))
                    .expect("copy committed tile");
            });
        temp_dir
    }

    #[test]
    fn scan_tile_directory_resolves_the_committed_slope_tiles() {
        // Scans a temp COPY of the committed art (in git ls-files), not a fixture. The
        // measured greys (air 0 / ground 155, ground on the bottom row) resolve all 7, so the
        // flag never fires on this family and no `*_unresolved.png` is written.
        let tiles_dir: PathBuf = temp_copy_of_tiles("slope_up_");
        let output_dir: PathBuf = std::env::temp_dir().join("import_tiles_scan_out_test");
        let _ = std::fs::remove_dir_all(&output_dir);
        std::fs::create_dir_all(&output_dir).expect("temp out dir");
        let entries: Vec<TerrainTypeDef> = scan_tile_directory(
            &tiles_dir,
            "slope_up_",
            DEFAULT_MIN_CLUSTER_GAP,
            &output_dir,
        );

        assert_eq!(entries.len(), 7, "the committed slope_up_0..6 family");
        let keys: Vec<&str> = entries.iter().map(|entry| entry.key.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "slope_up_0",
                "slope_up_1",
                "slope_up_2",
                "slope_up_3",
                "slope_up_4",
                "slope_up_5",
                "slope_up_6",
            ],
            "the keys are the committed family (a wrong strip_suffix leaves '.png' on them)"
        );
        let flagged: Vec<String> = std::fs::read_dir(&output_dir)
            .expect("temp out dir")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|file_name| file_name.ends_with("_unresolved.png"))
            .collect();
        assert!(flagged.is_empty(), "no tile should be flagged: {flagged:?}");
        entries.iter().for_each(|entry| {
            assert_eq!(
                entry.solid_mask.len(),
                (32 * 32 + 7) / 8,
                "{} packed mask",
                entry.key
            );
            assert!(entry.is_walkable, "{} is walkable", entry.key);
        });
    }

    // ---- Two-phase preprocessing and conversion guards ----

    const FIXTURE_GRID_COLS: i32 = 4;
    const FIXTURE_GRID_ROWS: i32 = 2;
    const FIXTURE_GENERATED_KEYS: usize = 16;
    const FIXTURE_BASE_ENTRIES: usize = 14;
    const EXPECTED_TERRAIN_ENTRIES: usize = 30;

    /// Build a 4x2 request over one sheet. Right cells use the measured key suffix; left cells
    /// repeat the same source regions with the authored horizontal flip.
    fn fn_fixture_request(
        source_path: &Path,
        margin_x_px: i32,
        margin_y_px: i32,
        cell_width_px: i32,
        cell_height_px: i32,
        separation_px: i32,
        width_px: u32,
        height_px: u32,
    ) -> ConversionRequest {
        let mut mut_cells: Vec<CellRequest> = Vec::new();
        for source_row in 0..FIXTURE_GRID_ROWS {
            for source_col in 0..FIXTURE_GRID_COLS {
                let cell_index = source_row * FIXTURE_GRID_COLS + source_col;
                mut_cells.push(CellRequest {
                    local_x: source_col,
                    local_y: source_row,
                    source_col,
                    source_row,
                    flip_h: false,
                    generated_key: format!("slope_grass_30deg_4x2_r{cell_index}"),
                });
                mut_cells.push(CellRequest {
                    local_x: source_col,
                    local_y: source_row,
                    source_col,
                    source_row,
                    flip_h: true,
                    generated_key: format!("slope_grass_30deg_4x2_l{cell_index}"),
                });
            }
        }
        ConversionRequest {
            version: REQUEST_VERSION.to_string(),
            source_image: source_path.to_string_lossy().into_owned(),
            margins_px: PixelPoint {
                x: margin_x_px,
                y: margin_y_px,
            },
            cell_size_px: PixelPoint {
                x: cell_width_px,
                y: cell_height_px,
            },
            separation_px: PixelPoint {
                x: separation_px,
                y: separation_px,
            },
            width_px,
            height_px,
            cells: mut_cells,
        }
    }

    /// Paint a 4x2 sheet with a diagonal air-to-ground seam. The seam makes a dropped flip
    /// observable without introducing a third grey that could disturb cluster resolution.
    fn fn_two_grey_sheet(
        margin_x_px: u32,
        margin_y_px: u32,
        cell_width_px: u32,
        cell_height_px: u32,
        separation_px: u32,
        ground_grey: u8,
        air_grey: u8,
    ) -> RgbaImage {
        let pitch_x_px: u32 = cell_width_px + separation_px;
        let pitch_y_px: u32 = cell_height_px + separation_px;
        let sheet_width_px: u32 = 2 * margin_x_px
            + FIXTURE_GRID_COLS as u32 * cell_width_px
            + (FIXTURE_GRID_COLS as u32 - 1) * separation_px;
        let sheet_height_px: u32 = 2 * margin_y_px
            + FIXTURE_GRID_ROWS as u32 * cell_height_px
            + (FIXTURE_GRID_ROWS as u32 - 1) * separation_px;
        let mut mut_sheet: RgbaImage =
            RgbaImage::from_fn(sheet_width_px, sheet_height_px, |_, _| {
                Rgba([0, 0, 0, ALPHA_SOLID])
            });
        for source_row in 0..FIXTURE_GRID_ROWS as u32 {
            for source_col in 0..FIXTURE_GRID_COLS as u32 {
                let origin_x_px: u32 = margin_x_px + source_col * pitch_x_px;
                let origin_y_px: u32 = margin_y_px + source_row * pitch_y_px;
                for pixel_y in 0..cell_height_px {
                    for pixel_x in 0..cell_width_px {
                        let seam_row: u32 = cell_height_px / 4 + pixel_x / 2;
                        let grey: u8 = match pixel_y >= seam_row {
                            true => ground_grey,
                            false => air_grey,
                        };
                        mut_sheet.put_pixel(
                            origin_x_px + pixel_x,
                            origin_y_px + pixel_y,
                            Rgba([grey, grey, grey, ALPHA_SOLID]),
                        );
                    }
                }
            }
        }
        mut_sheet
    }

    /// Repaint one cell with a near-continuous grey ramp whose gap is below the classifier floor.
    fn fn_paint_unresolved_cell(
        sheet: &mut RgbaImage,
        origin_x_px: u32,
        origin_y_px: u32,
        width_px: u32,
        height_px: u32,
    ) {
        (0..height_px).for_each(|pixel_y| {
            (0..width_px).for_each(|pixel_x| {
                let grey: u8 = RAMP_START_GREY + ((pixel_x + pixel_y) % 5) as u8 * RAMP_STEP_GREY;
                sheet.put_pixel(
                    origin_x_px + pixel_x,
                    origin_y_px + pixel_y,
                    Rgba([grey, grey, grey, ALPHA_SOLID]),
                );
            });
        });
    }

    /// Paint the separation strips with a colour no cell uses, so a leak into a resized cell
    /// cannot hide behind the two-grey silhouette.
    fn fn_paint_grid_lines(sheet: &mut RgbaImage, request: &ConversionRequest) {
        let margin_x_px: i32 = request.margins_px.x;
        let margin_y_px: i32 = request.margins_px.y;
        let cell_width_px: i32 = request.cell_size_px.x;
        let cell_height_px: i32 = request.cell_size_px.y;
        let separation_px: i32 = request.separation_px.x;
        let pitch_x_px: i32 = cell_width_px + separation_px;
        let pitch_y_px: i32 = cell_height_px + separation_px;
        let crop_width_px: i32 = FIXTURE_GRID_COLS * pitch_x_px - separation_px;
        let crop_height_px: i32 = FIXTURE_GRID_ROWS * pitch_y_px - separation_px;
        (0..FIXTURE_GRID_COLS - 1).for_each(|col| {
            (0..crop_height_px).for_each(|row| {
                (0..separation_px).for_each(|offset| {
                    sheet.put_pixel(
                        (margin_x_px + col * pitch_x_px + cell_width_px + offset) as u32,
                        (margin_y_px + row) as u32,
                        Rgba(UNRESOLVED_BORDER_RGBA),
                    );
                });
            });
        });
        (0..FIXTURE_GRID_ROWS - 1).for_each(|row| {
            (0..crop_width_px).for_each(|col| {
                (0..separation_px).for_each(|offset| {
                    sheet.put_pixel(
                        (margin_x_px + col) as u32,
                        (margin_y_px + row * pitch_y_px + cell_height_px + offset) as u32,
                        Rgba(UNRESOLVED_BORDER_RGBA),
                    );
                });
            });
        });
    }

    /// Persist a fixture and return its path, so the tool's own file decode is exercised.
    fn fn_write_source(sheet: &RgbaImage, dir: &Path) -> PathBuf {
        let source_path: PathBuf = dir.join("source.png");
        sheet.save(&source_path).expect("write fixture source PNG");
        source_path
    }

    /// Fresh throwaway directory for one guard, never inside the committed asset tree.
    fn fn_test_dir(name: &str) -> PathBuf {
        let dir: PathBuf = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create guard temp dir");
        dir
    }

    #[test]
    fn conversion_expands_all_cells_at_configured_size() {
        let work_dir: PathBuf = fn_test_dir("import_tiles_conversion_expand_test");
        let sheet: RgbaImage = fn_two_grey_sheet(2, 2, 16, 16, 1, GROUND_GREY, AIR_GREY);
        let source_path: PathBuf = fn_write_source(&sheet, &work_dir);
        let request: ConversionRequest = fn_fixture_request(&source_path, 2, 2, 16, 16, 1, 8, 8);
        let result: ConversionResult =
            fn_convert_resource(&request, &work_dir).expect("a resolvable fixture converts");

        assert_eq!(
            result.outputs.len(),
            FIXTURE_GENERATED_KEYS,
            "8 right + 8 left cells"
        );
        let keys: Vec<&str> = result
            .outputs
            .iter()
            .map(|output| output.generated_key.as_str())
            .collect();
        (0..8).for_each(|cell_index| {
            assert!(keys.contains(&format!("slope_grass_30deg_4x2_r{cell_index}").as_str()));
            assert!(keys.contains(&format!("slope_grass_30deg_4x2_l{cell_index}").as_str()));
        });
        result.outputs.iter().for_each(|output| {
            let tile = image::open(&output.image_path).expect("saved generated PNG");
            assert_eq!(
                tile.dimensions(),
                (8, 8),
                "{} uses configured W x H",
                output.generated_key
            );
            assert_eq!(
                output.solid_mask.len(),
                (8 * 8 + 7) / 8,
                "{} packed mask",
                output.generated_key
            );
            assert!(output.resolved, "{} resolves", output.generated_key);
        });
    }

    #[test]
    fn alpha_matches_greyscale_mask_per_pixel() {
        let work_dir: PathBuf = fn_test_dir("import_tiles_alpha_oracle_test");
        let sheet: RgbaImage = fn_two_grey_sheet(2, 2, 16, 16, 1, GROUND_GREY, AIR_GREY);
        let source_path: PathBuf = fn_write_source(&sheet, &work_dir);
        let request: ConversionRequest = fn_fixture_request(&source_path, 2, 2, 16, 16, 1, 8, 8);
        let result: ConversionResult =
            fn_convert_resource(&request, &work_dir).expect("a resolvable fixture converts");

        // The oracle decodes the saved file itself; it never shares the tool's greyscale helper.
        let fn_assert_alpha = |key: &str| {
            let output = result
                .outputs
                .iter()
                .find(|candidate| candidate.generated_key == key)
                .expect("generated key exists");
            let decoded = image::open(&output.image_path).expect("saved generated PNG");
            let rgba = decoded.clone().to_rgba8();
            let luma = decoded.to_luma8();
            (0..8).for_each(|row| {
                (0..8).for_each(|col| {
                    let expected_solid: bool = luma.get_pixel(col, row).0[0] == GROUND_GREY;
                    assert_eq!(
                        rgba.get_pixel(col, row).0[3] == ALPHA_SOLID,
                        expected_solid,
                        "alpha at ({col},{row}) of {key} follows the independent grey oracle"
                    );
                    assert_eq!(
                        mask_bit(&output.solid_mask, 8, col, row),
                        expected_solid,
                        "packed mask at ({col},{row}) of {key} follows the same oracle"
                    );
                });
            });
        };
        fn_assert_alpha("slope_grass_30deg_4x2_r0");
        fn_assert_alpha("slope_grass_30deg_4x2_l0");

        // The left output is the mirror of the right output, so a missing flip is visible.
        let fn_luma_of = |key: &str| -> image::GrayImage {
            let output = result
                .outputs
                .iter()
                .find(|candidate| candidate.generated_key == key)
                .expect("generated key exists");
            image::open(&output.image_path)
                .expect("saved generated PNG")
                .to_luma8()
        };
        let right_luma: image::GrayImage = fn_luma_of("slope_grass_30deg_4x2_r0");
        let left_luma: image::GrayImage = fn_luma_of("slope_grass_30deg_4x2_l0");
        (0..8).for_each(|row| {
            (0..8).for_each(|col| {
                assert_eq!(
                    left_luma.get_pixel(col, row).0[0],
                    right_luma.get_pixel(7 - col, row).0[0],
                    "the left cell is flipped after resize at ({col},{row})"
                );
            });
        });
    }

    #[test]
    fn derive_mask_flags_unresolved_without_emitting_collision() {
        let work_dir: PathBuf = fn_test_dir("import_tiles_unresolved_test");
        let mut mut_sheet: RgbaImage = fn_two_grey_sheet(2, 2, 16, 16, 1, GROUND_GREY, AIR_GREY);
        fn_paint_unresolved_cell(&mut mut_sheet, 2, 2, 16, 16);
        let source_path: PathBuf = fn_write_source(&mut_sheet, &work_dir);
        let request: ConversionRequest = fn_fixture_request(&source_path, 2, 2, 16, 16, 1, 8, 8);

        let first_dir: PathBuf = work_dir.join("first");
        std::fs::create_dir_all(&first_dir).expect("first run dir");
        let failure: ConvertError = fn_convert_resource(&request, &first_dir)
            .expect_err("a cell without a cluster gap must abort the commit");
        assert!(
            matches!(failure, ConvertError::UnresolvedCells(_)),
            "failure names unresolved cells"
        );
        assert!(
            !first_dir.join("terrain_types.json").exists(),
            "a refused run emits no collision entry"
        );

        // Repeat-after-refusal: repair the SAME source file and keep the request unchanged.
        let repaired: RgbaImage = fn_two_grey_sheet(2, 2, 16, 16, 1, GROUND_GREY, AIR_GREY);
        repaired
            .save(&source_path)
            .expect("repair fixture in place");
        let retry_dir: PathBuf = work_dir.join("retry");
        std::fs::create_dir_all(&retry_dir).expect("retry run dir");
        let result: ConversionResult = fn_convert_resource(&request, &retry_dir)
            .expect("corrected same-path fixture converts on the second run");
        assert_eq!(result.outputs.len(), FIXTURE_GENERATED_KEYS);
        assert!(result.outputs.iter().all(|output| output.resolved));
    }

    #[test]
    fn two_conversion_runs_are_byte_identical() {
        let work_dir: PathBuf = fn_test_dir("import_tiles_byte_identity_test");
        let sheet: RgbaImage = fn_two_grey_sheet(2, 2, 16, 16, 1, GROUND_GREY, AIR_GREY);
        let source_path: PathBuf = fn_write_source(&sheet, &work_dir);
        let request: ConversionRequest = fn_fixture_request(&source_path, 2, 2, 16, 16, 1, 8, 8);
        let first_dir: PathBuf = work_dir.join("first");
        let second_dir: PathBuf = work_dir.join("second");
        std::fs::create_dir_all(&first_dir).expect("first run dir");
        std::fs::create_dir_all(&second_dir).expect("second run dir");
        let first: ConversionResult = fn_convert_resource(&request, &first_dir).expect("first run");
        let second: ConversionResult =
            fn_convert_resource(&request, &second_dir).expect("second run");

        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first result"),
            serde_json::to_vec(&second).expect("serialize second result"),
            "result JSON is byte-identical"
        );
        first.outputs.iter().for_each(|output| {
            let first_bytes: Vec<u8> = std::fs::read(&output.image_path).expect("first PNG bytes");
            let second_path: PathBuf =
                second_dir.join(output.image_path.file_name().expect("PNG file name"));
            let second_bytes: Vec<u8> = std::fs::read(&second_path).expect("second PNG bytes");
            assert_eq!(
                first_bytes, second_bytes,
                "{} PNG bytes match",
                output.generated_key
            );
        });
    }

    #[test]
    fn terrain_entries_register_all_sixteen_generated_keys() {
        let work_dir: PathBuf = fn_test_dir("import_tiles_registration_test");
        let sheet: RgbaImage = fn_two_grey_sheet(2, 2, 16, 16, 1, GROUND_GREY, AIR_GREY);
        let source_path: PathBuf = fn_write_source(&sheet, &work_dir);
        let request: ConversionRequest = fn_fixture_request(&source_path, 2, 2, 16, 16, 1, 8, 8);
        let result: ConversionResult =
            fn_convert_resource(&request, &work_dir).expect("conversion");
        let base: Vec<TerrainTypeDef> = (0..FIXTURE_BASE_ENTRIES)
            .map(|index| {
                terrain_entry_from_image(&format!("base_{index}"), &sheet, DEFAULT_MIN_CLUSTER_GAP)
                    .0
            })
            .collect();
        let merged: Vec<TerrainTypeDef> = fn_merge_terrain_entries(&base, &result.outputs);

        assert_eq!(
            merged.len(),
            EXPECTED_TERRAIN_ENTRIES,
            "14 existing + 16 generated"
        );
        let generated: Vec<&TerrainTypeDef> = merged
            .iter()
            .filter(|entry| entry.key.starts_with("slope_grass_30deg_4x2_"))
            .collect();
        assert_eq!(
            generated.len(),
            FIXTURE_GENERATED_KEYS,
            "no generated key is lost"
        );
        let distinct_keys: std::collections::HashSet<&str> =
            generated.iter().map(|entry| entry.key.as_str()).collect();
        assert_eq!(
            distinct_keys.len(),
            FIXTURE_GENERATED_KEYS,
            "no generated key is duplicated"
        );
        generated.iter().for_each(|entry| {
            assert!(entry.is_walkable, "{} is walkable", entry.key);
            assert_eq!(
                entry.solid_mask.len(),
                (8 * 8 + 7) / 8,
                "{} packed mask",
                entry.key
            );
        });
        assert!(
            (0..FIXTURE_BASE_ENTRIES).all(|index| merged
                .iter()
                .any(|entry| entry.key == format!("base_{index}"))),
            "every pre-existing entry survives the merge"
        );
    }

    #[test]
    fn preprocess_crops_whole_grid_then_slices_eight_cells() {
        let work_dir: PathBuf = fn_test_dir("import_tiles_preprocess_crop_test");
        let sheet: RgbaImage = fn_two_grey_sheet(104, 65, 126, 126, 1, GROUND_GREY, AIR_GREY);
        let source_path: PathBuf = fn_write_source(&sheet, &work_dir);
        let request: ConversionRequest =
            fn_fixture_request(&source_path, 104, 65, 126, 126, 1, 32, 32);
        let result: PreprocessResult =
            preprocess_source_sheet(&request, &work_dir).expect("Phase I succeeds");

        assert_eq!(
            result.crop_rect,
            PixelRect {
                x: 104,
                y: 65,
                width_px: 507,
                height_px: 253
            },
            "the outer 4x2 grid rect is cropped once"
        );
        let cleaned = image::open(&result.cleaned_image_path).expect("cleaned PNG saved");
        assert_eq!(
            cleaned.dimensions(),
            (128, 64),
            "8 cells assembled at 4W x 2H, not 512x256"
        );
        assert_eq!(result.cells.len(), 8, "one cleaned cell per source region");
        let expected_origins: [(u32, u32); 8] = [
            (0, 0),
            (32, 0),
            (64, 0),
            (96, 0),
            (0, 32),
            (32, 32),
            (64, 32),
            (96, 32),
        ];
        result.cells.iter().zip(expected_origins.iter()).for_each(
            |(cell, (origin_x, origin_y))| {
                assert_eq!(
                    cell.pixel_rect,
                    PixelRect {
                        x: *origin_x,
                        y: *origin_y,
                        width_px: 32,
                        height_px: 32
                    },
                    "cell [{},{}] keeps its own W x H pixel rect",
                    cell.source_col,
                    cell.source_row
                );
                assert_eq!(cell.dimensions.width_px, 32, "cell mask width is W, not 4W");
                assert_eq!(
                    cell.dimensions.height_px, 32,
                    "cell mask height is H, not 2H"
                );
            },
        );
    }

    #[test]
    fn preprocess_clears_every_grid_line_pixel() {
        let work_dir: PathBuf = fn_test_dir("import_tiles_grid_line_test");
        let mut mut_sheet: RgbaImage =
            fn_two_grey_sheet(104, 65, 126, 126, 1, GROUND_GREY, AIR_GREY);
        let layout_request: ConversionRequest =
            fn_fixture_request(&Path::new("unused.png"), 104, 65, 126, 126, 1, 32, 32);
        fn_paint_grid_lines(&mut mut_sheet, &layout_request);
        let source_path: PathBuf = fn_write_source(&mut_sheet, &work_dir);
        let request: ConversionRequest =
            fn_fixture_request(&source_path, 104, 65, 126, 126, 1, 32, 32);
        let result: PreprocessResult =
            preprocess_source_sheet(&request, &work_dir).expect("Phase I succeeds");

        let vertical_pixels: u32 = 3 * result.crop_rect.height_px;
        let horizontal_pixels: u32 = result.crop_rect.width_px;
        assert_eq!(
            result.cleared_grid_line_pixels,
            vertical_pixels + horizontal_pixels,
            "every vertical and horizontal separation pixel is cleared"
        );
        assert!(
            result.cells.iter().all(|cell| cell.resolved),
            "no painted grid-line colour leaks into a cell mask"
        );
        let cleaned = image::open(&result.cleaned_image_path)
            .expect("cleaned PNG saved")
            .to_rgba8();
        (0..cleaned.height()).for_each(|row| {
            (0..cleaned.width()).for_each(|col| {
                let alpha: u8 = cleaned.get_pixel(col, row).0[3];
                assert!(
                    alpha == ALPHA_AIR || alpha == ALPHA_SOLID,
                    "alpha at ({col},{row}) is air or ground"
                );
            });
        });
    }

    #[test]
    fn preprocess_flags_unresolved_and_blocks_phase_ii() {
        let work_dir: PathBuf = fn_test_dir("import_tiles_preprocess_gate_test");
        let mut mut_sheet: RgbaImage = fn_two_grey_sheet(2, 2, 16, 16, 1, GROUND_GREY, AIR_GREY);
        fn_paint_unresolved_cell(&mut mut_sheet, 2, 2, 16, 16);
        let source_path: PathBuf = fn_write_source(&mut_sheet, &work_dir);
        let request: ConversionRequest = fn_fixture_request(&source_path, 2, 2, 16, 16, 1, 8, 8);
        let sidecar_path: PathBuf = work_dir.join("slope.jsonc");

        let flagged: PreprocessResult =
            preprocess_source_sheet(&request, &work_dir).expect("Phase I flags but does not abort");
        let unresolved: &CleanedCell = flagged
            .cells
            .iter()
            .find(|cell| !cell.resolved)
            .expect("the ramp cell is flagged");
        assert!(
            unresolved.solid_mask.is_empty(),
            "no collision value is invented for a flagged cell"
        );
        assert!(
            fn_phase_two_gate(&flagged, true, &sidecar_path).is_err(),
            "Phase II refuses even when the human sign-off flag is set"
        );
        assert!(
            !sidecar_path.exists(),
            "the sidecar is not written before repair"
        );

        // Same request path and same config; only the offending source pixels are corrected.
        let repaired: RgbaImage = fn_two_grey_sheet(2, 2, 16, 16, 1, GROUND_GREY, AIR_GREY);
        repaired
            .save(&source_path)
            .expect("repair fixture in place");
        let signed: PreprocessResult =
            preprocess_source_sheet(&request, &work_dir).expect("corrected Phase I succeeds");
        assert!(signed.cells.iter().all(|cell| cell.resolved));
        let sidecar: ConversionSidecar = fn_phase_two_gate(&signed, true, &sidecar_path)
            .expect("signed-off Phase II emits the sidecar");
        assert_eq!(sidecar.tiles.len(), 8, "one sidecar row per cleaned cell");
        assert!(
            sidecar_path.exists(),
            "the sidecar is on disk after sign-off"
        );
    }

    #[test]
    fn jsonc_sidecar_round_trips_after_human_signoff() {
        let work_dir: PathBuf = fn_test_dir("import_tiles_sidecar_test");
        let sheet: RgbaImage = fn_two_grey_sheet(104, 65, 126, 126, 1, GROUND_GREY, AIR_GREY);
        let source_path: PathBuf = fn_write_source(&sheet, &work_dir);
        let request: ConversionRequest =
            fn_fixture_request(&source_path, 104, 65, 126, 126, 1, 32, 32);
        let result: PreprocessResult =
            preprocess_source_sheet(&request, &work_dir).expect("Phase I");
        let sidecar_path: PathBuf = work_dir.join("slope.jsonc");
        let emitted: ConversionSidecar =
            fn_phase_two_gate(&result, true, &sidecar_path).expect("signed-off sidecar emitted");

        // Inject a line comment after emission: the reader must strip it without recomputing
        // any geometry from the source sheet.
        let raw_text: String = std::fs::read_to_string(&sidecar_path).expect("sidecar text");
        let commented_text: String = format!("// human inspected and signed off\n{raw_text}");
        std::fs::write(&sidecar_path, commented_text).expect("write commented sidecar");
        let decoded: ConversionSidecar =
            read_sidecar(&sidecar_path).expect("JSONC comments stripped");

        assert_eq!(
            decoded.version, emitted.version,
            "version survives the round trip"
        );
        assert_eq!(
            decoded.tile_width_px, 32,
            "tile width is read, not recomputed"
        );
        assert_eq!(
            decoded.tile_height_px, 32,
            "tile height is read, not recomputed"
        );
        assert_eq!(
            decoded.mask_encoding, MASK_ENCODING_GREYSCALE,
            "mask encoding token survives"
        );
        assert_eq!(
            decoded.tiles.len(),
            result.cells.len(),
            "every cleaned cell has a sidecar row"
        );
        decoded
            .tiles
            .iter()
            .zip(result.cells.iter())
            .for_each(|(row, cell)| {
                assert_eq!(row.source_col, cell.source_col, "source column survives");
                assert_eq!(row.source_row, cell.source_row, "source row survives");
                assert_eq!(
                    row.pixel_rect, cell.pixel_rect,
                    "pixel rect is read, not recomputed"
                );
                assert_eq!(row.dimensions, cell.dimensions, "cell dimensions survive");
            });
    }
}
