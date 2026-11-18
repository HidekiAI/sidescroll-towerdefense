use image::{Rgba, RgbaImage};
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
    /// Serialized as a JSON array of bytes, matching the core `TerrainTypeDef.solid_mask`
    /// (`Vec<u8>` with a serde default), so no base64 dependency is needed on either side.
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

// TODO(human): begin block 2/8 - uniform_patch_grey
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
    // let half: u32 = patch_size_px / 2;
    // let col_start: u32 = patch_x.saturating_sub(half);
    // let row_start: u32 = patch_y.saturating_sub(half);
    // let col_end: u32 = (col_start + patch_size_px).min(width_px);
    // let row_end: u32 = (row_start + patch_size_px).min(height_px);
    // let mut mut_histogram: [u32; 256] = [0; 256];
    // for row in row_start..row_end {
    //     for col in col_start..col_end {
    //         mut_histogram[greyscale[(row * width_px + col) as usize] as usize] += 1;
    //     }
    // }
    // mut_histogram
    //     .iter()
    //     .enumerate()
    //     .max_by_key(|(_, count)| **count)
    //     .map(|(grey, _)| grey as u8)
    //     .unwrap_or(0)
    let hole: u8 = ();
}
// TODO(human): end block 2/8

// TODO(human): begin block 3/8 - remove_floating_specks
// A solid run that does not reach the tile's bottom row is dust, not ground: the bottom-row
// anchor says the ground is the mass the tile stands on. Flood from every solid pixel of the
// bottom row and clear whatever the flood never reaches.
fn remove_floating_specks(binary: &mut [u8], width_px: u32, height_px: u32) {
    // let width: usize = width_px as usize;
    // let bottom_row_start: usize = (height_px as usize - 1) * width;
    // let mut mut_reached: Vec<bool> = vec![false; binary.len()];
    // let mut mut_stack: Vec<usize> = (0..width)
    //     .filter(|col| binary[bottom_row_start + col] != 0)
    //     .map(|col| bottom_row_start + col)
    //     .collect();
    // while let Some(index) = mut_stack.pop() {
    //     if mut_reached[index] {
    //         continue;
    //     }
    //     mut_reached[index] = true;
    //     let col: usize = index % width;
    //     let row: usize = index / width;
    //     let neighbours: [Option<usize>; 4] = [
    //         (col > 0).then(|| index - 1),
    //         (col + 1 < width).then(|| index + 1),
    //         (row > 0).then(|| index - width),
    //         (row + 1 < height_px as usize).then(|| index + width),
    //     ];
    //     for neighbour in neighbours.into_iter().flatten() {
    //         if binary[neighbour] != 0 && !mut_reached[neighbour] {
    //             mut_stack.push(neighbour);
    //         }
    //     }
    // }
    // for (index, solid) in binary.iter_mut().enumerate() {
    //     if *solid != 0 && !mut_reached[index] {
    //         *solid = 0;
    //     }
    // }
    let hole: () = 0u8;
}
// TODO(human): end block 3/8

// TODO(human): begin block 4/8 - derive_solid_mask
// Resolve one tile's pixels into a packed solid mask plus a `resolved` verdict. The two
// clusters are the tile's dominant grey and the grey furthest from it; a gap under
// `min_cluster_gap` means the art does not split into air and ground at all, so no heuristic
// can anchor and the tile is returned unresolved. GROUND is the cluster the tile's bottom
// row stands on (the bottom-row anchor); a bottom row carrying both clusters is settled by
// the patch read, which is the best guess. A single-grey tile is all solid.
fn derive_solid_mask(
    greyscale: &[u8],
    width_px: u32,
    height_px: u32,
    min_cluster_gap: u8,
) -> (Vec<u8>, bool) {
    // let packed_len: usize = (greyscale.len() + 7) / 8;
    // let mut mut_histogram: [u32; 256] = [0; 256];
    // for grey in greyscale {
    //     mut_histogram[*grey as usize] += 1;
    // }
    // let present: Vec<u8> = (0..=255u8)
    //     .filter(|grey| mut_histogram[*grey as usize] > 0)
    //     .collect();
    // if present.len() <= 1 {
    //     return (vec![0xFFu8; packed_len], true);
    // }
    // let dominant_grey: u8 = *present
    //     .iter()
    //     .max_by_key(|grey| mut_histogram[**grey as usize])
    //     .unwrap();
    // let opposite_grey: u8 = *present
    //     .iter()
    //     .max_by_key(|grey| (**grey as i32 - dominant_grey as i32).abs())
    //     .unwrap();
    // if (opposite_grey as i32 - dominant_grey as i32).abs() < min_cluster_gap as i32 {
    //     return (vec![0xFFu8; packed_len], false);
    // }
    // let bottom_grey: u8 = uniform_patch_grey(
    //     greyscale,
    //     width_px,
    //     height_px,
    //     width_px / 2,
    //     height_px - 1,
    //     PATCH_SIZE_PX,
    // );
    // let (ground_grey, air_grey): (u8, u8) =
    //     if (bottom_grey as i32 - dominant_grey as i32).abs()
    //         <= (bottom_grey as i32 - opposite_grey as i32).abs()
    //     {
    //         (dominant_grey, opposite_grey)
    //     } else {
    //         (opposite_grey, dominant_grey)
    //     };
    // let mut mut_binary: Vec<u8> = greyscale
    //     .iter()
    //     .map(|grey| {
    //         ((*grey as i32 - ground_grey as i32).abs() <= (*grey as i32 - air_grey as i32).abs()) as u8
    //     })
    //     .collect();
    // remove_floating_specks(&mut mut_binary, width_px, height_px);
    // let mut mut_mask: Vec<u8> = vec![0u8; packed_len];
    // for (index, solid) in mut_binary.iter().enumerate() {
    //     if *solid != 0 {
    //         mut_mask[index / 8] |= 1 << (index % 8);
    //     }
    // }
    // (mut_mask, true)
    let hole: (Vec<u8>, bool) = ();
}
// TODO(human): end block 4/8

// TODO(human): begin block 5/8 - flag_unresolved
// A tile no heuristic could resolve gets a red 1px border baked into the emitted PNG, so a
// human sees it and hand-edits the art. The interior is never repainted.
fn flag_unresolved(img: &mut RgbaImage) {
    // let width_px: u32 = img.width();
    // let height_px: u32 = img.height();
    // let border: Rgba<u8> = Rgba(UNRESOLVED_BORDER_RGBA);
    // for col in 0..width_px {
    //     img.put_pixel(col, 0, border);
    //     img.put_pixel(col, height_px.saturating_sub(1), border);
    // }
    // for row in 0..height_px {
    //     img.put_pixel(0, row, border);
    //     img.put_pixel(width_px.saturating_sub(1), row, border);
    // }
    let hole: () = 0u8;
}
// TODO(human): end block 5/8

// TODO(human): begin block 6/8 - terrain_entry_from_image
// Turn one committed tile PNG into the `terrain_types.json` entry the framework reads: the
// key, a walkable flag, and the per-pixel `solid_mask` derived from the art. The `resolved`
// verdict is returned so the caller can flag an inconclusive tile.
fn terrain_entry_from_image(
    key: &str,
    img: &RgbaImage,
    min_cluster_gap: u8,
) -> (TerrainTypeDef, bool) {
    // let greyscale: Vec<u8> = to_greyscale(img);
    // let (mask, resolved): (Vec<u8>, bool) =
    //     derive_solid_mask(&greyscale, img.width(), img.height(), min_cluster_gap);
    // let (red, green, blue, _) = average_color(img);
    // let entry: TerrainTypeDef = TerrainTypeDef {
    //     key: key.to_string(),
    //     display_name: key.replace('_', " "),
    //     is_walkable: true,
    //     is_buildable: true,
    //     surface: "normal".into(),
    //     hazard: "none".into(),
    //     elevation_tiles: 0,
    //     color_hex: format!("#{:02x}{:02x}{:02x}", red, green, blue),
    //     sub_tile_mask: 0xF,
    //     solid_mask: mask,
    //     hazard_damage_per_tick: 0,
    //     is_destructible: false,
    //     destructible_hp: 0,
    //     on_destroy_terrain_key: String::new(),
    // };
    // (entry, resolved)
    let hole: (TerrainTypeDef, bool) = ();
}
// TODO(human): end block 6/8

// TODO(human): begin block 7/8 - scan_tile_directory
// Scan a directory of `{key}_32x32.png` tiles (optionally filtered by `key_prefix`) into
// terrain entries, sorted by key. A tile whose mask cannot be resolved is written back out
// with the red border so the human sees it, and logged.
fn scan_tile_directory(dir: &Path, key_prefix: &str, min_cluster_gap: u8) -> Vec<TerrainTypeDef> {
    // let mut mut_entries: Vec<TerrainTypeDef> = std::fs::read_dir(dir)
    //     .unwrap_or_else(|e| panic!("Failed to read tile dir '{}': {}", dir.display(), e))
    //     .filter_map(|entry| entry.ok())
    //     .filter_map(|entry| {
    //         let file_name: String = entry.file_name().to_string_lossy().into_owned();
    //         let key: &str = file_name
    //             .strip_suffix("_32x32.png")
    //             .filter(|stripped| stripped.starts_with(key_prefix))?;
    //         let mut mut_img: RgbaImage = image::open(entry.path())
    //             .unwrap_or_else(|e| panic!("Failed to open '{}': {}", entry.path().display(), e))
    //             .to_rgba8();
    //         let (terrain_entry, resolved): (TerrainTypeDef, bool) =
    //             terrain_entry_from_image(key, &mut_img, min_cluster_gap);
    //         if !resolved {
    //             flag_unresolved(&mut mut_img);
    //             let flagged_path: PathBuf = dir.join(format!("{}_unresolved.png", key));
    //             mut_img
    //                 .save(&flagged_path)
    //                 .unwrap_or_else(|e| panic!("Failed to save '{}': {}", flagged_path.display(), e));
    //             eprintln!("UNRESOLVED {} -> {}", file_name, flagged_path.display());
    //         }
    //         Some(terrain_entry)
    //     })
    //     .collect();
    // mut_entries.sort_by(|a, b| a.key.cmp(&b.key));
    // mut_entries
    let hole: Vec<TerrainTypeDef> = ();
}
// TODO(human): end block 7/8

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

    // TODO(human): begin block 8/8 - tile-scan mode in main()
    // The spritesheet-slice path below needs an `input` sheet; the tile-scan path does not
    // (it reads committed `{key}_32x32.png` tiles), so it runs first and returns. Its
    // `terrain_types.json` lands in `output_dir` for the framework merge.
    // if let Some(scan_dir) = &config.scan_tiles {
    //     let key_prefix: &str = config.scan_prefix.as_deref().unwrap_or("");
    //     let entries: Vec<TerrainTypeDef> =
    //         scan_tile_directory(Path::new(scan_dir), key_prefix, config.min_cluster_gap);
    //     let terrain_file: TerrainTypesFile = TerrainTypesFile {
    //         version: "0.1.0".into(),
    //         tiles: entries,
    //     };
    //     let terrain_json: String = serde_json::to_string_pretty(&terrain_file)
    //         .expect("Failed to serialize terrain_types.json");
    //     let terrain_json_path: PathBuf =
    //         PathBuf::from(&config.output_dir).join("terrain_types.json");
    //     std::fs::write(&terrain_json_path, &terrain_json)
    //         .unwrap_or_else(|e| panic!("Failed to write '{}': {}", terrain_json_path.display(), e));
    //     println!(
    //         "\nWrote {} ({} tiles)",
    //         terrain_json_path.display(),
    //         terrain_file.tiles.len()
    //     );
    //     return;
    // }
    let hole: () = 0u8;
    // TODO(human): end block 8/8

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
                    solid_mask: Vec::new(),
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

// =========================================================================
// Solid-mask pipeline tests (#126, ART -> DATA)
// =========================================================================
// These pin the contract the TODO(human) blocks implement. Synthetic RGBA tiles keep the
// fixtures self-contained; the committed `slope_up_*` art is pinned separately by the greys
// recorded in the repo AGENTS.md (air 0, ground 155, bottom-row anchor).
#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

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
            let grey = if row >= height_px - ground_rows {
                GROUND_GREY
            } else {
                AIR_GREY
            };
            Rgba([grey, grey, grey, 255])
        })
    }

    /// Overwrite one pixel with a grey, keeping it opaque.
    fn paint_grey(img: &mut RgbaImage, col: u32, row: u32, grey: u8) {
        img.put_pixel(col, row, Rgba([grey, grey, grey, 255]));
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
        let mut img = tile_with_ground_rows(8, 8, 4);
        paint_grey(&mut img, 5, 1, GROUND_GREY); // a speck of ground floating in the air
        let greyscale = to_greyscale(&img);
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
        let mut img = tile_with_ground_rows(8, 8, 4);
        paint_grey(&mut img, 0, 7, AIR_GREY);
        let greyscale = to_greyscale(&img);
        let (_, resolved) = derive_solid_mask(&greyscale, 8, 8, DEFAULT_MIN_CLUSTER_GAP);

        assert!(
            resolved,
            "an ambiguous bottom row is guessed, never flagged"
        );

        let mut emitted = img.clone();
        if !resolved {
            flag_unresolved(&mut emitted);
        }
        assert_ne!(
            emitted.get_pixel(0, 0),
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
            Rgba([grey, grey, grey, 255])
        });
        let greyscale = to_greyscale(&img);
        let (_, resolved) = derive_solid_mask(&greyscale, 8, 8, DEFAULT_MIN_CLUSTER_GAP);

        assert!(!resolved, "a tile with no cluster split is flagged");

        let mut emitted = img.clone();
        flag_unresolved(&mut emitted);
        assert_eq!(
            emitted.get_pixel(0, 0),
            &Rgba(UNRESOLVED_BORDER_RGBA),
            "the emitted PNG carries the red 1px border"
        );
        assert_eq!(
            emitted.get_pixel(4, 4)[0],
            greyscale[(4 * 8 + 4) as usize],
            "the border never repaints the tile interior"
        );
    }

    #[test]
    fn mask_uniform_patch_ignores_edge_pixel() {
        // One anti-aliased pixel sits inside an otherwise uniform air patch: reading that
        // single pixel gives the edge grey, reading the patch gives the region's air grey.
        let mut img = tile_with_ground_rows(8, 8, 4);
        paint_grey(&mut img, 2, 2, EDGE_GREY);
        let greyscale = to_greyscale(&img);

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

    /// Copy the committed `{prefix}*_32x32.png` art into a fresh temp dir. The scan writes a
    /// flagged tile's `*_unresolved.png` beside its input, so scanning a COPY keeps a mutation
    /// experiment from dirtying the source tree.
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
        let entries: Vec<TerrainTypeDef> =
            scan_tile_directory(&tiles_dir, "slope_up_", DEFAULT_MIN_CLUSTER_GAP);

        assert_eq!(entries.len(), 7, "the committed slope_up_0..6 family");
        let flagged: Vec<String> = std::fs::read_dir(&tiles_dir)
            .expect("temp tiles dir")
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
}
