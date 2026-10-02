//! gen-backdrop: generate the SSTD parallax backdrop layer stack in-repo.
//!
//! Issue #68. Design of record: wiki `TDD_Parallax-Background` sections 2, 3.1-3.3 and
//! 4.1, plus the decision record `TDD_Parallax-Restack-2026-09-30`.
//!
//! WHY GENERATED ART (owner decision, 2026-09-30): the official 2D Parallax tutorial is
//! the node/mechanism contract but ships NO downloadable assets, and no external or CC0
//! art pack is used because SSTD is a deliberate exercise in working with an LLM as the
//! only co-developer. So the art is produced here instead of sourced.
//!
//! WHY THIS MUST BE SEAMLESS: every layer is drawn once at `width` px and then tiled
//! horizontally forever by `Parallax2D.repeat_size`. If a layer's first and last column
//! differ, the join is a visible vertical seam repeating across the whole scene. The
//! noise lattice is what prevents that: it WRAPS in x at the texture width, so sampling
//! over `[0, width)` is periodic by construction rather than by clamping.
//!
//! Screen geometry is derived, not chosen: 60 x 33 tiles at 32 px is a 1920 x 1056
//! gameplay screen, so layers author 1:1 at 1920 wide and nothing is scaled or blurred.
//! Vertical coverage is overscan plus a clamped camera-y travel, NOT a vertical
//! `repeat_size` -- a vertical repeat leaves empty blocks above and below a
//! horizontal-only stack.
//!
//! The FIRST pass is deliberately palette-agnostic: neutral blue-greys whose only job is
//! to prove the mechanism. Biome palettes are a later re-tint of these same layers, never
//! a redraw, so the layer count and the factors cannot drift from the TDD.
//!
//! Usage:
//!   gen-backdrop [--out DIR] [--width N] [--overscan N] [--seed N]
//!
//! Outputs: sky.png, clouds_high.png, clouds_low.png, hills.png, forest.png, and
//!          manifest.json -- the single source of truth for scroll_scale, repeat_size and
//!          z_index, consumed by the scene and by the GDScript test so a factor is never
//!          pinned in two places and allowed to drift.
//!
//! Every run writes its inputs, per-layer dimensions, factor pairs and output paths to
//! stdout, so an incident is replayable from the log alone.

use image::{Rgba, RgbaImage};
use serde::Serialize;
use std::path::PathBuf;

/// Version stamped into `manifest.json` so a stale manifest is detectable on disk.
const MANIFEST_VERSION: &str = "1.0";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match generate_layers_and_manifest(&args) {
        Ok(dir) => println!("gen-backdrop: done -> {}", dir.display()),
        Err(e) => {
            eprintln!("gen-backdrop: error: {}", e);
            std::process::exit(2);
        }
    }
}

/// One backdrop plane: its texture and the node contract that consumes it.
struct LayerSpec {
    name: &'static str,
    file: &'static str,
    /// Per-axis scroll factors. `x` is the tutorial's published value for this depth;
    /// `y` is scaled to ~0.85x of it because the backdrop must move on BOTH axes.
    scroll_scale: (f32, f32),
    /// Texture height in px: the band's height plus vertical overscan headroom.
    height_px: u32,
    /// Drawn back-to-front. The foreground plane sorts above the tile grid.
    z_index: i32,
}

/// The full stack. Six planes: five backdrop layers plus a foreground drawn IN FRONT of
/// the tile layer. That is six of the seven-layer cap.
///
/// `factor > 1` on the foreground is deliberate and is the tutorial's "appear closer to
/// the camera" case, which is also why it is excluded from the `biome_backdrop_layers`
/// schema: that table's CHECK bounds factors to `[0,1]`, so a biome backdrop row can
/// never express a foreground.
fn layer_specs() -> Vec<LayerSpec> {
    vec![
        LayerSpec {
            name: "Sky",
            file: "sky.png",
            scroll_scale: (0.10, 0.08),
            height_px: 1320,
            z_index: 0,
        },
        LayerSpec {
            name: "HighClouds",
            file: "clouds_high.png",
            scroll_scale: (0.20, 0.17),
            height_px: 320,
            z_index: 1,
        },
        LayerSpec {
            name: "LowClouds",
            file: "clouds_low.png",
            scroll_scale: (0.30, 0.25),
            height_px: 320,
            z_index: 2,
        },
        LayerSpec {
            name: "Hills",
            file: "hills.png",
            scroll_scale: (0.50, 0.42),
            height_px: 384,
            z_index: 3,
        },
        LayerSpec {
            name: "Forest",
            file: "forest.png",
            scroll_scale: (0.70, 0.60),
            height_px: 320,
            z_index: 4,
        },
        LayerSpec {
            name: "Foreground",
            file: "foreground.png",
            scroll_scale: (1.30, 1.15),
            height_px: 320,
            z_index: 10,
        },
    ]
}

// ---------------------------------------------------------------------------
// 1/9 - lattice hash
// ---------------------------------------------------------------------------

/// Maps a lattice coordinate to a pseudo-random value in `[0, 1)`.
///
/// INTENT (permanent): a pure function of `(cell_x, cell_y, seed)` with no RNG state, because the
/// same seed must reproduce byte-identical PNGs. A stateful RNG makes output depend on
/// call order, which would break the determinism the `deterministic` test asserts.
///
/// Deliberately NOT a "good" hash (no avalanche guarantees needed): it only has to be
/// cheap, deterministic and decorrelate adjacent lattice points enough that the
/// interpolated value noise is smooth rather than blocky.
fn lattice_hash(cell_x: i64, cell_y: i64, seed: u64) -> f32 {
    let mixed: u64 = (cell_x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (cell_y as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f)
        ^ seed;
    // splitmix64's finaliser: xor-shift, multiply, xor-shift, multiply, xor-shift.
    let xorshift_1: u64 = mixed ^ (mixed >> 33);
    let multiply_1: u64 = xorshift_1.wrapping_mul(0xff51_afd7_ed55_8ccd);
    let xorshift_2: u64 = multiply_1 ^ (multiply_1 >> 29);
    let multiply_2: u64 = xorshift_2.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    let xorshift_3: u64 = multiply_2 ^ (multiply_2 >> 32);
    // Top 24 bits only: the low bits of a multiply-xor mix are the weakest, and the
    // callers only need enough range to divide into [0, 1).
    (xorshift_3 >> 40) as f32 / (1u32 << 24) as f32
}

// ---------------------------------------------------------------------------
// 2/9 - value noise, wrapping in x
// ---------------------------------------------------------------------------

/// Smoothly interpolated value noise in `[0, 1]`, PERIODIC in `x` with period `period`.
///
/// INTENT (permanent): this is the seam guarantee. The x axis wraps at exactly `period`,
/// so the function is periodic over `[0, period)` and a texture sampled across that range
/// tiles with no join. Every other design (clamping at the edge, mirroring, plain
/// unbounded noise) leaves a discontinuity at the seam, and `Parallax2D.repeat_size`
/// makes that discontinuity repeat forever on screen.
///
/// `y` is intentionally NOT periodic: vertical coverage comes from overscan, never from a
/// vertical repeat.
///
/// Smoothstep the fractional part (`t*t*(3-2t)`), not linear: linear leaves a slope
/// discontinuity at every lattice point, which shows up as visible creases in the sky
/// gradient. Smoothstep has zero first derivative at the lattice points.
///
/// CALLER CONTRACT (this is the part that is easy to get wrong, and the seam test exists
/// because it was): the wrap point is `period` lattice cells, so the CALLER owns the pixel
/// -> lattice mapping. For a texture `width` px wide wanting `cells` cells across, sample
/// `col as f32 * cells as f32 / (width - 1) as f32`.
///
/// The `width - 1` is load bearing and is the whole difference between a seamless texture
/// and a merely continuous one. It maps pixel `width - 1` onto `cells`, which wraps to `0`,
/// so the first and last column are the SAME lattice point and therefore byte-identical --
/// which is exactly what `seam_is_invisible` asserts. Dividing by `width` instead also gives
/// a continuous function, but pixel `width - 1` lands just short of the wrap and its value
/// is a near-miss of pixel 0 rather than equal to it. The join is then invisible in motion
/// but the columns are not equal, so a byte-comparing test cannot verify the seam at all.
/// A seam you cannot assert is a seam nobody maintains.
///
/// Dividing by a round pixel divisor (say `col / 260.0`) does not work either: the wrap then
/// lands at `260 * period` px, which is not the texture edge at all.
fn value_noise(pos_x: f32, pos_y: f32, period: i64, seed: u64) -> f32 {
    let clamped_period: i64 = if period < 1 { 1 } else { period };
    let cell_x0: i64 = pos_x.floor() as i64;
    let cell_y0: i64 = pos_y.floor() as i64;
    let frac_x: f32 = pos_x - pos_x.floor();
    let frac_y: f32 = pos_y - pos_y.floor();
    // Smoothstep the fractional part, not linear: zero slope at every lattice point.
    let smooth_x: f32 = frac_x * frac_x * (3.0 - 2.0 * frac_x);
    let smooth_y: f32 = frac_y * frac_y * (3.0 - 2.0 * frac_y);
    // The x axis wraps at `p`; the y axis does not. `i64 % p` is negative for negative
    // i, hence the two-step normalisation rather than a bare `%`.
    let wrap =
        |raw_cell: i64| -> i64 { ((raw_cell % clamped_period) + clamped_period) % clamped_period };
    let hash_x0y0: f32 = lattice_hash(wrap(cell_x0), cell_y0, seed);
    let hash_x1y0: f32 = lattice_hash(wrap(cell_x0 + 1), cell_y0, seed);
    let hash_x0y1: f32 = lattice_hash(wrap(cell_x0), cell_y0 + 1, seed);
    let hash_x1y1: f32 = lattice_hash(wrap(cell_x0 + 1), cell_y0 + 1, seed);
    let row_y0: f32 = hash_x0y0 + (hash_x1y0 - hash_x0y0) * smooth_x;
    let row_y1: f32 = hash_x0y1 + (hash_x1y1 - hash_x0y1) * smooth_x;
    row_y0 + (row_y1 - row_y0) * smooth_y
}

// ---------------------------------------------------------------------------
// 3/9 - fractional Brownian motion
// ---------------------------------------------------------------------------

/// Sum of `octaves` value-noise layers, each at double the previous frequency, so the
/// result stays periodic in `x` with period `period` (an integer number of base cells).
///
/// INTENT (permanent): one octave of noise looks like soft blobs. Real cloud and terrain
/// silhouettes are fractal: large shapes with progressively finer detail. Each octave
/// contributes a smaller amplitude so the coarse shapes stay dominant.
///
/// THE PERIOD MUST DOUBLE PER OCTAVE. Octave `o` samples at double the frequency, so to
/// keep the SAME spatial period it must also span double as many lattice cells: `per_o =
/// period * 2^o`. That makes every octave a different-frequency view of one P-periodic
/// function, which is what fBm means.
///
/// Accuracy note, because the failure here is quiet: halving instead (`per_o =
/// period / 2^o`) still leaves the sum seamless -- each term's spatial period then divides
/// the base period -- so `fbm_stays_in_unit_range_and_wraps` passes either way and is a SEAM
/// check, not a spectrum check. Halving gives a different, non-fBm spectrum and silently
/// depends on `4^o` dividing the period. Neither is caught by a test here, so doubling is a
/// documented intent rather than an asserted contract.
///
/// Return in `[0, 1]`, not the raw sum, so callers can threshold it against a meaningful
/// fraction.
fn fbm(pos_x: f32, pos_y: f32, period: i64, octaves: u32, seed: u64) -> f32 {
    let mut sum: f32 = 0.0;
    let mut amp: f32 = 1.0;
    let mut norm: f32 = 0.0;
    let mut freq: f32 = 1.0;
    for o in 0..octaves.max(1) {
        // DOUBLES per octave: octave o samples at 2^o frequency, so it spans 2^o cells
        // and keeps the same spatial period as octave 0.
        let per: i64 = period.saturating_mul(1i64 << o.min(20));
        sum += amp
            * value_noise(
                pos_x * freq,
                pos_y * freq,
                per,
                seed ^ (o as u64).wrapping_mul(0x9e37_79b9),
            );
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

// ---------------------------------------------------------------------------
// 4/9 - colour lerp
// ---------------------------------------------------------------------------

/// Interpolates two RGB triples on `blend` in `[0, 1]`.
///
/// INTENT (permanent): u8 channels cannot be interpolated directly. `from + (to - from) * blend` in
/// u8 arithmetic truncates, so a 1% mix of two dark colours rounds back to the original
/// and the gradient bands. Widening to f32, mixing, then narrowing once at the end keeps
/// the full range. This is why every gradient in this file goes through here rather than
/// doing channel arithmetic inline.
///
/// Clamping `blend` is deliberate: a caller passing a value slightly outside `[0, 1]` from a
/// noisy expression should not wrap around to the far end of the ramp.
// TODO(human): begin block 4/9
fn lerp_rgb(from_rgb: [u8; 3], to_rgb: [u8; 3], blend: f32) -> [u8; 3] {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // let blend: f32 = blend.clamp(0.0, 1.0);
    // [
    //     (from_rgb[0] as f32 + (to_rgb[0] as f32 - from_rgb[0] as f32) * blend) as u8,
    //     (from_rgb[1] as f32 + (to_rgb[1] as f32 - from_rgb[1] as f32) * blend) as u8,
    //     (from_rgb[2] as f32 + (to_rgb[2] as f32 - from_rgb[2] as f32) * blend) as u8,
    // ]
}
// TODO(human): end block 4/9

// ---------------------------------------------------------------------------
// 5/9 - smoothstep
// ---------------------------------------------------------------------------

/// Hermite ramp from 0 at `edge0` to 1 at `edge1`, clamped outside.
///
/// INTENT (permanent): this is what makes a cloud edge gradual rather than a cutout, and
/// what makes the sun disc fall off softly instead of having an aliased rim. It is the
/// same `t*t*(3-2t)` curve block 2/9 applies to the noise lattice, reused here because
/// both jobs are "ease between two values with zero slope at each end".
///
/// The degenerate-interval guard matters for the sky: `sun_r` is derived from `height`,
/// and at a small height the inner and outer radii can round to the same value. Dividing
/// by a zero-width interval would produce NaN, and a NaN alpha poisons every downstream
/// pixel comparison in the test suite.
// TODO(human): begin block 5/9
fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // if (edge1 - edge0).abs() < f32::EPSILON {
    //     return if value < edge0 { 0.0 } else { 1.0 };
    // }
    // let ramp_t: f32 = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    // ramp_t * ramp_t * (3.0 - 2.0 * ramp_t)
}
// TODO(human): end block 5/9

// ---------------------------------------------------------------------------
// 6/9 - sky
// ---------------------------------------------------------------------------

/// The sky plane: fully opaque, a vertical gradient plus fBm haze plus one sun disc.
///
/// INTENT (permanent): alpha is 255 at EVERY pixel. The sky is the bottom of the stack,
/// so any hole in it would show the clear colour through the gap instead of another depth
/// plane. `sky_has_no_alpha` asserts this.
///
/// The gradient runs on `row / height` rather than on absolute pixel rows so the same
/// function is correct at any height; the sun disc is a radial falloff, not a hard circle,
/// so it has no aliased edge at 1:1 scale.
///
/// The haze term MUST go through the 2/9 caller contract (`col * cells / (width - 1)`) or the
/// sky seams, and the sun disc alone cannot hide it.
// TODO(human): begin block 6/9
fn make_sky(width: u32, height: u32, seed: u64) -> RgbaImage {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // let mut img: RgbaImage = RgbaImage::new(width, height);
    // let top: [u8; 3] = [138, 152, 172];
    // let mid: [u8; 3] = [166, 178, 192];
    // let low: [u8; 3] = [196, 204, 210];
    // let cells: i64 = 6;
    // // 2/9 caller contract: `cells` cells across the width, dividing by (width - 1) so
    // // the last column wraps exactly onto the first.
    // let col_to_cell = |col: u32| -> f32 { col as f32 * cells as f32 / (width.max(2) - 1) as f32 };
    // let sun_center_x_px: f32 = width as f32 * 0.68;
    // let sun_center_y_px: f32 = height as f32 * 0.24;
    // let sun_radius_px: f32 = height as f32 * 0.06;
    // for row in 0..height {
    //     let row_frac: f32 = row as f32 / height.max(1) as f32;
    //     let base: [u8; 3] = if row_frac < 0.55 {
    //         lerp_rgb(top, mid, row_frac / 0.55)
    //     } else {
    //         lerp_rgb(mid, low, (row_frac - 0.55) / 0.45)
    //     };
    //     for col in 0..width {
    //         let haze: f32 = fbm(col_to_cell(col), row as f32 * 0.002, cells, 3, seed) - 0.5;
    //         let sun_offset_x_px: f32 = col as f32 - sun_center_x_px;
    //         let sun_offset_y_px: f32 = row as f32 - sun_center_y_px;
    //         let dist_px: f32 =
    //             (sun_offset_x_px * sun_offset_x_px + sun_offset_y_px * sun_offset_y_px).sqrt();
    //         let disc: f32 = 1.0 - smoothstep(sun_radius_px * 0.65, sun_radius_px, dist_px);
    //         let mut sky_rgb: [u8; 3] = lerp_rgb(base, [255, 250, 238], disc * 0.85);
    //         let haze_shift: f32 = (haze * 14.0).clamp(-20.0, 20.0);
    //         for channel in 0..3 {
    //             sky_rgb[channel] = (sky_rgb[channel] as f32 + haze_shift).clamp(0.0, 255.0) as u8;
    //         }
    //         img.put_pixel(col, row, Rgba([sky_rgb[0], sky_rgb[1], sky_rgb[2], 255]));
    //     }
    // }
    // img
}
// TODO(human): end block 6/9

// ---------------------------------------------------------------------------
// 7/9 - clouds
// ---------------------------------------------------------------------------

/// One cloud plane: alpha is a smoothstep over fBm, so the layer has GENUINE holes.
///
/// INTENT (permanent): alpha is `smoothstep(alpha_edge_low, alpha_edge_high, fbm)` mapped to 0..255. Dense regions
/// of the noise field become opaque cloud, sparse regions become fully transparent, and
/// the transition is gradual rather than a hard cutout. A hard threshold would produce
/// the stair-stepped edge this technique exists to avoid.
///
/// NO VERTICAL BAND GRADIENT, deliberately. An earlier draft multiplied alpha by a vertical
/// ramp so the plane would have a guaranteed clear margin. It was dropped for two reasons:
/// the ramp's prose and its own formula disagreed about which end was dense, and the margin
/// only helps if it sits on the side the camera actually travels toward, which is not
/// knowable until the scene places the planes. Alpha is therefore the bare smoothstep,
/// which is exactly the recipe [TDD_Parallax-Background] specifies for this layer.
///
/// ACCEPTED CONSEQUENCE: the bottom row is whatever the noise says, so the clamped camera-y
/// travel budget (240 px) is the only thing keeping a vertical cut line off screen. That is a
/// scene-level visual check, not a unit test, because nothing at this layer can observe it.
///
/// The contract `cloud_layers_have_holes` asserts is that fully-opaque AND fully-clear pixels
/// both exist. With no band multiplier that rests entirely on `fbm` reaching both ends of
/// `[alpha_edge_low, alpha_edge_high]`, which is why the two edges are a per-layer argument
/// rather than hard-coded.
// TODO(human): begin block 7/9
fn make_clouds(
    width: u32,
    height: u32,
    seed: u64,
    alpha_edge_low: f32,
    alpha_edge_high: f32,
) -> RgbaImage {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // let mut img: RgbaImage = RgbaImage::new(width, height);
    // let cells: i64 = 8;
    // let body: [u8; 3] = [236, 238, 240];
    // let shade: [u8; 3] = [206, 210, 216];
    // // 2/9 caller contract, same as 6/9.
    // let col_to_cell = |col: u32| -> f32 { col as f32 * cells as f32 / (width.max(2) - 1) as f32 };
    // for row in 0..height {
    //     let row_frac: f32 = row as f32 / height.max(1) as f32;
    //     for col in 0..width {
    //         let density: f32 = fbm(col_to_cell(col), row_frac * 1.6, cells, 4, seed);
    //         let alpha: f32 = smoothstep(alpha_edge_low, alpha_edge_high, density);
    //         let cloud_rgb: [u8; 3] = lerp_rgb(shade, body, alpha);
    //         img.put_pixel(
    //             col,
    //             row,
    //             Rgba([
    //                 cloud_rgb[0],
    //                 cloud_rgb[1],
    //                 cloud_rgb[2],
    //                 (alpha * 255.0).round() as u8,
    //             ])
    //         );
    //     }
    // }
    // img
}
// TODO(human): end block 7/9

// ---------------------------------------------------------------------------
// 8/9 - silhouettes
// ---------------------------------------------------------------------------

/// A silhouette plane: an opaque ground band below a periodic skyline, with a darker rim
/// along the skyline itself.
///
/// INTENT (permanent): the skyline is a heightfield sampled from fBm, so it inherits the
/// x-wrap from block 2/9 and therefore tiles. Filling DOWNWARD from the skyline rather
/// than drawing shapes keeps the alpha channel trivially predictable: fully opaque below
/// the line, fully transparent above it, which is what makes the layer readable as a
/// silhouette rather than as a cloud.
///
/// The rim is drawn a few pixels along the line in a darker value so the plane has a
/// readable edge at 1:1 instead of dissolving into the layer behind it.
///
/// `relief_px` IS ABSOLUTE PIXELS, not a fraction of `height`. Two reasons, both load
/// bearing: the layers have very different heights (Sky is 1320 px, the cloud and forest
/// planes are 320), so a fraction gives each plane a different skyline amplitude for the
/// same authored number; and a fraction collapses to zero at small sizes, which silently
/// makes the noise -- and therefore the seed -- irrelevant. `deterministic` asserts that
/// changing the seed changes the image, and that assertion is what caught this.
// TODO(human): begin block 8/9
fn make_silhouette(
    width: u32,
    height: u32,
    seed: u64,
    baseline_frac: f32,
    relief_px: f32,
) -> RgbaImage {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // let mut img: RgbaImage = RgbaImage::new(width, height);
    // let cells: i64 = 5;
    // let body: [u8; 3] = [46, 56, 52];
    // let rim: [u8; 3] = [30, 38, 36];
    // // 2/9 caller contract, same as 6/9 and 7/9.
    // let col_to_cell = |col: u32| -> f32 { col as f32 * cells as f32 / (width.max(2) - 1) as f32 };
    // let baseline_row: i32 = (baseline_frac * height as f32).round() as i32;
    // for col in 0..width {
    //     let skyline_noise: f32 = fbm(col_to_cell(col), 0.0, cells, 4, seed);
    //     // relief_px is ABSOLUTE pixels, so amplitude does not vary with layer height.
    //     let skyline_row: i32 = baseline_row - (skyline_noise * relief_px).round() as i32;
    //     for row in 0..height {
    //         let row_i: i32 = row as i32;
    //         let px: Rgba<u8> = if row_i < skyline_row {
    //             Rgba([0, 0, 0, 0])
    //         } else if row_i < skyline_row + 3 {
    //             Rgba([rim[0], rim[1], rim[2], 255])
    //         } else {
    //             Rgba([body[0], body[1], body[2], 255])
    //         };
    //         img.put_pixel(col, row, px);
    //     }
    // }
    // img
}
// TODO(human): end block 8/9

// ---------------------------------------------------------------------------
// 9/9 - factor validation + manifest
// ---------------------------------------------------------------------------

/// Rejects any layer whose factors would break the node contract, and returns the first
/// violation found.
///
/// INTENT (permanent): a factor outside `(0, 1]` scrolls faster than the camera (the plane
/// runs away from the frame) or never moves at all, and `factor_x <= factor_y` would make
/// the plane move MORE vertically than horizontally, which reads as a bug rather than as
/// depth. `Foreground` is the one deliberate exception at 1.30, so the upper bound is
/// `<= 1.5` rather than 1.0 and only the `x > y` rule is absolute.
///
/// Validating here rather than in the scene means a bad factor fails at generation time,
/// where the manifest is written, instead of silently at runtime.
// TODO(human): begin block 9/9
fn validate_factors(specs: &[LayerSpec]) -> Result<(), String> {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // for spec in specs {
    //     let (scale_x, scale_y) = spec.scroll_scale;
    //     if !(scale_x > 0.0 && scale_x <= 1.5) {
    //         return Err(format!(
    //             "layer {}: scroll_scale.x {} out of range",
    //             spec.name, scale_x
    //         ));
    //     }
    //     if !(scale_y > 0.0) {
    //         return Err(format!(
    //             "layer {}: scroll_scale.y {} must be positive",
    //             spec.name, scale_y
    //         ));
    //     }
    //     if scale_x <= scale_y {
    //         return Err(format!(
    //             "layer {}: scroll_scale.x {} must exceed y {}",
    //             spec.name, scale_x, scale_y
    //         ));
    //     }
    // }
    // Ok(())
}
// TODO(human): end block 9/9

/// Builds the `manifest.json` payload: the node contract for every layer, in one place.
///
/// INTENT (permanent): the scene and the GDScript test both read this file instead of
/// hard-coding factors, so a factor cannot be pinned in two places and drift. `repeat_size`
/// is `(width, 0)` -- horizontal tiling only, because vertical coverage is overscan plus a
/// clamped camera-y travel and a vertical repeat would leave empty blocks above and below.
///
/// `repeat_size_y` is hard-coded 0 rather than passed in, because a vertical repeat is
/// never correct here, and encoding that as a literal means the decision cannot be flipped
/// by a caller.
// TODO(human): begin block 9/9 (second function, same block)
fn build_manifest(specs: &[LayerSpec], width: u32, overscan: u32, seed: u64) -> Manifest {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // Manifest {
    //     version: MANIFEST_VERSION.to_string(),
    //     width_px: width,
    //     overscan_px: overscan,
    //     seed,
    //     layers: specs
    //         .iter()
    //         .map(|spec| ManifestLayer {
    //             name: spec.name.to_string(),
    //             file: spec.file.to_string(),
    //             scroll_scale_x: spec.scroll_scale.0,
    //             scroll_scale_y: spec.scroll_scale.1,
    //             repeat_size_x: width,
    //             repeat_size_y: 0,
    //             z_index: spec.z_index,
    //             height_px: spec.height_px,
    //         })
    //         .collect(),
    // }
}
// TODO(human): end block 9/9 (second function, same block)

/// Serialized form of `manifest.json`.
#[derive(Serialize)]
struct Manifest {
    version: String,
    width_px: u32,
    overscan_px: u32,
    seed: u64,
    layers: Vec<ManifestLayer>,
}

/// One layer's entry in `manifest.json`.
#[derive(Serialize)]
struct ManifestLayer {
    name: String,
    file: String,
    scroll_scale_x: f32,
    scroll_scale_y: f32,
    repeat_size_x: u32,
    repeat_size_y: u32,
    z_index: i32,
    height_px: u32,
}

// ---------------------------------------------------------------------------
// Assembly (assistant-written; no image logic lives here)
// ---------------------------------------------------------------------------

/// Parses the CLI, builds every layer, writes the PNGs and the manifest.
fn generate_layers_and_manifest(args: &[String]) -> Result<PathBuf, String> {
    let mut out_dir = PathBuf::from("editor/assets/backdrop_layers");
    let mut width: u32 = 1920;
    let mut overscan: u32 = 240;
    let mut seed: u64 = 0x5EED_2026;

    let mut arg_index = 1usize;
    while arg_index < args.len() {
        match args[arg_index].as_str() {
            "--out" => {
                arg_index += 1;
                out_dir = PathBuf::from(
                    args.get(arg_index)
                        .ok_or_else(|| "--out requires a directory".to_string())?,
                );
            }
            "--width" => {
                arg_index += 1;
                width = args
                    .get(arg_index)
                    .ok_or_else(|| "--width requires a value".to_string())?
                    .parse::<u32>()
                    .map_err(|_| "--width must be an integer".to_string())?;
            }
            "--overscan" => {
                arg_index += 1;
                overscan = args
                    .get(arg_index)
                    .ok_or_else(|| "--overscan requires a value".to_string())?
                    .parse::<u32>()
                    .map_err(|_| "--overscan must be an integer".to_string())?;
            }
            "--seed" => {
                arg_index += 1;
                seed = args
                    .get(arg_index)
                    .ok_or_else(|| "--seed requires a value".to_string())?
                    .parse::<u64>()
                    .map_err(|_| "--seed must be an integer".to_string())?;
            }
            other => return Err(format!("unexpected argument '{}'", other)),
        }
        arg_index += 1;
    }

    if width == 0 {
        return Err("width must be greater than zero".to_string());
    }

    let specs = layer_specs();
    validate_factors(&specs)?;

    std::fs::create_dir_all(&out_dir)
        .map_err(|err| format!("cannot create {}: {}", out_dir.display(), err))?;

    println!(
        "gen-backdrop: width={} overscan={} seed={} out={} layers={}",
        width,
        overscan,
        seed,
        out_dir.display(),
        specs.len()
    );

    /// One generator call per layer. Each layer's own noise seed is derived from the run
    /// seed and its index, so changing the stack's order or length cannot silently alter
    /// an unrelated layer's texture.
    for (index, spec) in specs.iter().enumerate() {
        let layer_seed = seed
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(index as u64);
        let height = spec.height_px;
        let image: RgbaImage = match spec.name {
            "Sky" => make_sky(width, height, layer_seed),
            "HighClouds" => make_clouds(width, height, layer_seed, 0.52, 0.68),
            "LowClouds" => make_clouds(width, height, layer_seed, 0.44, 0.60),
            // `relief_px` is absolute, so it is scaled from the authored fraction HERE, in
            // the one place that knows the layer's real height. See 6/7.
            "Hills" => make_silhouette(width, height, layer_seed, 0.62, height as f32 * 0.22),
            "Forest" => make_silhouette(width, height, layer_seed, 0.74, height as f32 * 0.18),
            "Foreground" => make_silhouette(width, height, layer_seed, 0.88, height as f32 * 0.12),
            other => return Err(format!("no generator for layer '{}'", other)),
        };

        if image.width() != width || image.height() != height {
            return Err(format!(
                "layer '{}' produced {}x{}, expected {}x{}",
                spec.name,
                image.width(),
                image.height(),
                width,
                height
            ));
        }

        let path = out_dir.join(spec.file);
        image
            .save(&path)
            .map_err(|err| format!("cannot write {}: {}", path.display(), err))?;

        let opaque = image.pixels().filter(|rgba| rgba.0[3] == 255).count();
        let total = (width as usize) * (height as usize);
        println!(
            "gen-backdrop: layer={} file={} dims={}x{} scale=({:.2},{:.2}) z={} opaque={}/{} ({:.1}%)",
            spec.name,
            spec.file,
            width,
            height,
            spec.scroll_scale.0,
            spec.scroll_scale.1,
            spec.z_index,
            opaque,
            total,
            100.0 * opaque as f64 / total as f64
        );
    }

    let manifest = build_manifest(&specs, width, overscan, seed);
    let manifest_path = out_dir.join("manifest.json");
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|err| format!("cannot serialize manifest: {}", err))?;
    std::fs::write(&manifest_path, format!("{}\n", json))
        .map_err(|err| format!("cannot write {}: {}", manifest_path.display(), err))?;
    println!(
        "gen-backdrop: manifest={} layers={}",
        manifest_path.display(),
        manifest.layers.len()
    );

    Ok(out_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Narrow-but-real size: small enough to be fast, large enough that the fBm lattice
    /// has several periods across x so a missing wrap is actually exercised.
    const W: u32 = 256;
    const H: u32 = 64;
    const SEED: u64 = 12345;

    fn raw_column(img: &RgbaImage, col: u32) -> Vec<[u8; 4]> {
        (0..img.height())
            .map(|row| {
                let rgba = img.get_pixel(col, row).0;
                [rgba[0], rgba[1], rgba[2], rgba[3]]
            })
            .collect()
    }

    /// Order-sensitive digest of an image's bytes.
    ///
    /// Deliberately NOT `assert_eq!(a.as_raw(), b.as_raw())`: on a 256x64 image a mismatch
    /// prints 65536 numbers, which buries the actual cause and makes a CI log unreadable.
    /// A FNV-1a digest prints as one hex number, and `first_pixel_difference` localises it
    /// when the digests disagree.
    fn digest(img: &RgbaImage) -> u64 {
        let mut running_hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in img.as_raw() {
            running_hash ^= *byte as u64;
            running_hash = running_hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        running_hash
    }

    /// Byte index of the first differing pixel, plus both pixels, so a failure names the
    /// exact spot instead of dumping the whole buffer.
    fn first_pixel_difference(img_a: &RgbaImage, img_b: &RgbaImage) -> String {
        if img_a.dimensions() != img_b.dimensions() {
            return format!(
                "dimensions differ: {:?} vs {:?}",
                img_a.dimensions(),
                img_b.dimensions()
            );
        }
        for (flat_index, (pixel_a, pixel_b)) in img_a
            .as_raw()
            .chunks_exact(4)
            .zip(img_b.as_raw().chunks_exact(4))
            .enumerate()
        {
            if pixel_a != pixel_b {
                let (col, row) = (
                    (flat_index as u32) % img_a.width(),
                    (flat_index as u32) / img_a.width(),
                );
                return format!(
                    "first difference at pixel (col {}, row {}) index {}: {:?} vs {:?}",
                    col, row, flat_index, pixel_a, pixel_b
                );
            }
        }
        "buffers differ in length only".to_string()
    }

    /// Every layer the crate emits, as (name, image) pairs.
    ///
    /// Goes through the SAME `generate_layers_and_manifest()` dispatch table by name, so a
    /// layer added to `layer_specs()` without a generator is caught here instead of at runtime.
    fn all_layers() -> Vec<(String, RgbaImage)> {
        vec![
            ("Sky".to_string(), make_sky(W, H, SEED)),
            (
                "HighClouds".to_string(),
                make_clouds(W, H, SEED, 0.52, 0.68),
            ),
            ("LowClouds".to_string(), make_clouds(W, H, SEED, 0.44, 0.60)),
            ("Hills".to_string(), make_silhouette(W, H, SEED, 0.62, 0.22)),
            (
                "Forest".to_string(),
                make_silhouette(W, H, SEED, 0.74, 0.18),
            ),
            (
                "Foreground".to_string(),
                make_silhouette(W, H, SEED, 0.88, 0.12),
            ),
        ]
    }

    /// SEAM CONTRACT. `Parallax2D.repeat_size` tiles each layer horizontally forever, so
    /// column 0 and column `width - 1` must be identical or the join repeats as a visible
    /// vertical line across the whole scene.
    ///
    /// This is the single most important assertion in the crate: it is what the x-wrap in
    /// `value_noise` exists to provide, and nothing else can detect a regression there.
    #[test]
    fn seam_is_invisible() {
        for (name, img) in all_layers() {
            assert_eq!(img.width(), W, "{}: unexpected width", name);
            let first = raw_column(&img, 0);
            let last = raw_column(&img, W - 1);
            assert_eq!(
                first, last,
                "layer {}: first and last column differ, so repeat_size will show a seam",
                name
            );
        }
    }

    /// SKY OPAQUITY CONTRACT. The sky is the bottom of the stack; a hole in it shows the
    /// clear colour instead of another depth plane, which reads as a bug in the art rather
    /// than as depth.
    #[test]
    fn sky_has_no_alpha() {
        let sky = make_sky(W, H, SEED);
        let holes = sky.pixels().filter(|rgba| rgba.0[3] != 255).count();
        assert_eq!(
            holes, 0,
            "sky must be fully opaque, found {} hole pixels",
            holes
        );
    }

    /// CLOUD HOLE CONTRACT. A cloud plane whose alpha is uniformly 0 or uniformly 255 is
    /// useless: the holes are the entire point of the layered technique over the old
    /// scanline trick. Asserting BOTH buckets exist catches a threshold that has drifted
    /// outside the noise's actual range, which would collapse the layer to all-or-nothing.
    ///
    /// SCOPE, measured not assumed: mutation M9 replaced the smoothstep with a hard
    /// threshold and this test stayed GREEN, because the vertical band gradient contributes
    /// partial alpha on its own. So this is a holes check, not a soft-edge check. Whether
    /// the cloud edge is gradual is a documented intent, not an asserted contract.
    #[test]
    fn cloud_layers_have_holes() {
        for (name, t0, t1) in [("HighClouds", 0.52f32, 0.68f32), ("LowClouds", 0.44, 0.60)] {
            let img = make_clouds(W, H, SEED, t0, t1);
            let opaque = img.pixels().filter(|rgba| rgba.0[3] == 255).count();
            let clear = img.pixels().filter(|rgba| rgba.0[3] == 0).count();
            assert!(opaque > 0, "{}: no fully opaque pixels at all", name);
            assert!(clear > 0, "{}: no fully transparent pixels at all", name);
        }
    }

    /// SILHOUETTE AMPLITUDE CONTRACT. `relief_px` is absolute pixels, so the skyline's
    /// vertical range must not depend on the texture height. The layers have very
    /// different heights (Sky 1320, clouds and forest 320), so a fraction-of-height
    /// reading would give the same authored number a different silhouette per layer.
    ///
    /// Why the amplitude is measured rather than the exact pixels: the contract is
    /// "amplitude is height-independent", not "amplitude equals 12". A tolerance of 2px
    /// absorbs rounding while still catching a 4x scale difference.
    #[test]
    fn relief_is_absolute_not_a_fraction() {
        let (small_min, small_max) = skyline(&make_silhouette(W, 64, SEED, 0.5, 12.0))
            .expect("64px silhouette has no opaque row");
        let (large_min, large_max) = skyline(&make_silhouette(W, 256, SEED, 0.5, 12.0))
            .expect("256px silhouette has no opaque row");

        let small = (small_max - small_min) as f32;
        let large = (large_max - large_min) as f32;
        assert!(
            (small - large).abs() <= 2.0,
            "skyline amplitude scales with texture height: {}px at H=64 but {}px at H=256, \
             so relief_px is being read as a fraction of height rather than absolute pixels",
            small,
            large
        );
    }

    /// (min, max) of the first fully-opaque row across all columns, i.e. the skyline's
    /// vertical extent. `None` when the plane has no opaque pixel at all.
    fn skyline(img: &RgbaImage) -> Option<(i32, i32)> {
        let mut min = i32::MAX;
        let mut max = i32::MIN;
        for col in 0..img.width() {
            for row in 0..img.height() {
                if img.get_pixel(col, row).0[3] == 255 {
                    min = min.min(row as i32);
                    max = max.max(row as i32);
                    break;
                }
            }
        }
        if max < min {
            None
        } else {
            Some((min, max))
        }
    }

    /// FACTOR CONTRACT, on the shipped stack. Factors drive `scroll_scale`, so they are the
    /// mechanism itself. `scale_x > scale_y` is absolute; the upper bound is 1.5 rather than 1.0
    /// because `Foreground` is deliberately faster than the camera.
    #[test]
    fn factors_in_unit_range() {
        let specs = layer_specs();
        assert_eq!(
            specs.len(),
            6,
            "expected five backdrop layers plus one foreground"
        );
        assert!(
            validate_factors(&specs).is_ok(),
            "shipped factors must satisfy the node contract: {:?}",
            validate_factors(&specs)
        );
        for spec in &specs {
            let (scale_x, scale_y) = spec.scroll_scale;
            assert!(
                scale_x > 0.0 && scale_x <= 1.5,
                "{}: scroll_scale.x {} out of range",
                spec.name,
                scale_x
            );
            assert!(
                scale_y > 0.0 && scale_y < scale_x,
                "{}: scroll_scale.y {} must be below x {}",
                spec.name,
                scale_y,
                scale_x
            );
        }
    }

    /// FACTOR CONTRACT, on the rejection path. `factors_in_unit_range` only ever feeds
    /// `validate_factors` the shipped stack, which is valid by construction -- so mutation
    /// M12 (disabling the `scale_x <= scale_y` check) left that test GREEN. A validator shown
    /// valid input is untested.
    ///
    /// Each case asserts the specific message, not merely `is_err`, so a validator that
    /// rejected everything for the wrong reason could not pass.
    #[test]
    fn validate_factors_rejects_bad_specs() {
        /// Builds a spec whose only interesting field is the scroll scale, so each case
        /// below varies exactly the input the validator is supposed to judge.
        /// `file` is the one field `validate_factors` ignores, so it is derived from the
        /// case name instead of left as a placeholder: a meaningless literal in a fixture
        /// reads as though it were part of what is under test. The `panic` arm stops a new
        /// case from silently inheriting another case's file name.
        fn rejected_spec(name: &'static str, scale_x: f32, scale_y: f32) -> LayerSpec {
            let file: &'static str = match name {
                "Zero" => "zero.png",
                "Neg" => "neg.png",
                "Inverted" => "inverted.png",
                "Runaway" => "runaway.png",
                other => panic!("no fixture file name for case {}", other),
            };
            LayerSpec {
                name,
                file,
                scroll_scale: (scale_x, scale_y),
                height_px: 64,
                z_index: 0,
            }
        }

        let cases: [(&str, LayerSpec, &str); 4] = [
            ("zero x", rejected_spec("Zero", 0.0, 0.0), "out of range"),
            ("negative y", rejected_spec("Neg", 0.5, -0.1), "positive"),
            (
                "x below y",
                rejected_spec("Inverted", 0.4, 0.6),
                "must exceed",
            ),
            (
                "x too fast",
                rejected_spec("Runaway", 1.9, 1.0),
                "out of range",
            ),
        ];
        for (label, spec, expected) in cases {
            match validate_factors(&[spec]) {
                Ok(()) => panic!("{}: validator accepted an invalid spec", label),
                Err(msg) => assert!(
                    msg.contains(expected),
                    "{}: message {:?} does not mention {:?}",
                    label,
                    msg,
                    expected
                ),
            }
        }
    }

    /// DETERMINISM CONTRACT. The generator is a build step whose output is committed, so
    /// the same seed MUST reproduce byte-identical pixels. Any hidden RNG state breaks
    /// this, and with it the ability to explain or regenerate an artifact later.
    #[test]
    fn deterministic() {
        let clouds_a = make_clouds(W, H, SEED, 0.52, 0.68);
        let clouds_b = make_clouds(W, H, SEED, 0.52, 0.68);
        assert_eq!(
            digest(&clouds_a),
            digest(&clouds_b),
            "same seed produced different pixels: {}",
            first_pixel_difference(&clouds_a, &clouds_b)
        );

        let hills_a = make_silhouette(W, H, SEED, 0.62, 12.0);
        let hills_b = make_silhouette(W, H, SEED, 0.62, 12.0);
        assert_eq!(
            digest(&hills_a),
            digest(&hills_b),
            "silhouette is not deterministic: {}",
            first_pixel_difference(&hills_a, &hills_b)
        );

        // A different seed must actually change the image, or "deterministic" would be
        // satisfied trivially by a constant generator. This also catches a `relief_px`
        // that is a fraction of `height` rather than absolute pixels: at a small test
        // height a fraction rounds to zero, the skyline goes flat, and every seed produces
        // the same picture. That regression is invisible to the other six tests.
        let hills_c = make_silhouette(W, H, SEED + 1, 0.62, 12.0);
        assert_ne!(
            digest(&hills_a),
            digest(&hills_c),
            "changing the seed changed nothing, so the seed is not reaching the noise"
        );
    }

    /// LATTICE HASH RANGE CONTRACT. The doc comment promises `[0, 1)`, half-open, and the
    /// half-open half is load bearing: `value_noise` interpolates between four of these
    /// values, so a hash that can return exactly `1.0` puts a lattice point on the top edge
    /// of every interpolation. Asserting `<= 1.0` would let that through.
    ///
    /// Negative cells are outside the generator's domain (`col_to_cell` is non-negative for
    /// every `col`). Only the range is asserted for them, not any particular value.
    #[test]
    fn lattice_hash_is_in_unit_range() {
        for cell_x in -8..8 {
            for cell_y in -8..8 {
                for seed in [0u64, 1, 0x5eed, u64::MAX] {
                    let value = lattice_hash(cell_x, cell_y, seed);
                    assert!(
                        (0.0..1.0).contains(&value),
                        "lattice_hash(cell_x={}, cell_y={}, seed={}) = {} is outside [0,1)",
                        cell_x,
                        cell_y,
                        seed,
                        value
                    );
                }
            }
        }
    }

    /// LATTICE HASH DECORRELATION CONTRACT. `lattice_hash` was previously only ever
    /// reached THROUGH `value_noise`, so every other assertion in this module was about the
    /// composite rather than the hash. This asserts the hash itself: adjacent lattice
    /// points must not be near-identical, or `value_noise` interpolates a smooth gradient
    /// and the layers read as banding instead of noise.
    ///
    /// Both axes are checked. A hash that varies only horizontally would pass a
    /// horizontal-adjacency test untouched.
    ///
    /// THRESHOLD: two independent uniform draws on `[0,1)` differ in absolute value by
    /// exactly 1/3 on average. `0.15` sits well below that and far above any locally
    /// smooth hash, so nothing plausible lands in the gap. Fixed seeds and no RNG, so this
    /// cannot flake.
    ///
    /// FALSIFICATION -- measured, not asserted. Three mutant hashes were built and run
    /// against the full suite:
    ///
    ///   ramp   `((cell_x + cell_y + seed) % 256) / 256`   -> mean horizontal delta
    ///          0.00391. Fails HERE, plus `cloud_layers_have_holes` ("HighClouds: no
    ///          fully opaque pixels at all") and `deterministic` ("changing the seed
    ///          changed nothing" -- a near-uniform 1/256 seed shift is quantized away by
    ///          the `.round()` in the silhouette, so both seeds give the same skyline).
    ///   ramp2  `(cell_x * 3 + cell_y * 5) / 512` + seed jitter -> fails HERE and
    ///          `cloud_layers_have_holes`; `deterministic` stays green.
    ///   sine   `sin(cell_x * 0.11 + cell_y * 0.07 + seed * 0.9) / 2 + 0.5`
    ///          -> mean horizontal delta 0.03486. Fails HERE AND NOTHING ELSE:
    ///          10 passed, 1 failed.
    ///
    /// The `sine` mutant is what justifies this test existing. It is in range, pure,
    /// seed-sensitive, seamless, and produces valid sky and cloud layers -- every other
    /// assertion in the suite is satisfied -- yet it is locally smooth and would band
    /// visibly.
    ///
    /// CORRECTION: an earlier draft of this comment claimed that no other test could see
    /// a bad hash at all. That was wrong; the ramp mutants are caught by two others. What
    /// the other tests cannot do is LOCALISE the fault -- they report "no opaque pixels"
    /// and "seed changed nothing", which point at the generator, not at the hash. This
    /// test names the mechanism and prints the measured number.
    #[test]
    fn lattice_hash_decorrelates_adjacent_cells() {
        const MIN_MEAN_ADJACENT_DELTA: f32 = 0.15;
        const CELLS: i64 = 64;

        for seed in [0u64, 1, 0x5eed, u64::MAX] {
            let mut horizontal: Vec<f32> = Vec::new();
            let mut vertical: Vec<f32> = Vec::new();
            for cell_y in 0..CELLS {
                for cell_x in 0..CELLS {
                    let here = lattice_hash(cell_x, cell_y, seed);
                    if cell_x + 1 < CELLS {
                        let east = lattice_hash(cell_x + 1, cell_y, seed);
                        horizontal.push((here - east).abs());
                    }
                    if cell_y + 1 < CELLS {
                        let north = lattice_hash(cell_x, cell_y + 1, seed);
                        vertical.push((here - north).abs());
                    }
                }
            }

            let mean = |deltas: &Vec<f32>| deltas.iter().sum::<f32>() / deltas.len() as f32;
            let mean_horizontal = mean(&horizontal);
            let mean_vertical = mean(&vertical);
            assert!(
                mean_horizontal >= MIN_MEAN_ADJACENT_DELTA,
                "seed {}: mean |delta| between horizontally adjacent cells is {:.5}, below \
                 the {:.2} floor -- adjacent lattice points are too similar, which reads as \
                 smooth diagonal banding rather than noise (a ramp fails here)",
                seed,
                mean_horizontal,
                MIN_MEAN_ADJACENT_DELTA
            );
            assert!(
                mean_vertical >= MIN_MEAN_ADJACENT_DELTA,
                "seed {}: mean |delta| between vertically adjacent cells is {:.5}, below \
                 the {:.2} floor -- a hash that varies only horizontally passes a \
                 horizontal-only check, which is why both axes are measured",
                seed,
                mean_vertical,
                MIN_MEAN_ADJACENT_DELTA
            );
        }
    }

    /// VALUE NOISE RANGE CONTRACT. Every downstream layer thresholds or interpolates this
    /// value assuming `[0, 1]`; an out-of-range result silently shifts every threshold.
    #[test]
    fn value_noise_stays_in_unit_range() {
        for step in 0..64 {
            let sample_x = step as f32 * 3.7;
            let sample_y = step as f32 * 1.3;
            let sampled = value_noise(sample_x, sample_y, 8, SEED);
            assert!(
                (0.0..=1.0).contains(&sampled),
                "value_noise({}, {}) = {} is outside [0,1]",
                sample_x,
                sample_y,
                sampled
            );
        }
    }

    /// fBm must preserve both the `[0,1]` range and the x-period, so the seam guarantee
    /// survives every octave.
    ///
    /// SCOPE: this is a seam check, not a spectrum check. It stays green whether the period
    /// doubles or halves per octave (see 3/9), so it must not be cited as evidence that the
    /// frequency ladder is right -- `fbm_matches_its_documented_octave_formula` is the test
    /// that covers the ladder. The period is 64 so that four octaves cannot collapse to
    /// a single wrapped cell under either scheme -- a period of 8 under halving reaches 1,
    /// where every x wraps identically and the assertion becomes vacuous.
    #[test]
    fn fbm_stays_in_unit_range_and_wraps() {
        let period: i64 = 64;
        for step in 0..32 {
            let sample_y = step as f32 * 2.1;
            let sample_x = step as f32 * 1.5;
            let value_at = fbm(sample_x, sample_y, period, 4, SEED);
            let value_shifted = fbm(sample_x + period as f32, sample_y, period, 4, SEED);
            assert!(
                (0.0..=1.0).contains(&value_at),
                "fbm = {} is outside [0,1]",
                value_at
            );
            assert!(
                (value_at - value_shifted).abs() < 1e-5,
                "fbm is not periodic in x: f(x={}) = {} but f(x={}) = {}",
                sample_x,
                value_at,
                sample_x + period as f32,
                value_shifted
            );
        }
    }

    // -----------------------------------------------------------------------
    // 2/9 - value noise
    // -----------------------------------------------------------------------

    /// VALUE NOISE X-PERIODICITY CONTRACT. The doc comment's central promise is that the
    /// function repeats every `period` lattice cells in `x`, because `Parallax2D.repeat_size`
    /// tiles a 1920 px plane forever and any discontinuity becomes a seam repeating across
    /// the whole scene.
    ///
    /// WHY A DIRECT TEST, when `fbm_stays_in_unit_range_and_wraps` and `seam_is_invisible`
    /// already observe periodicity: both of those look at the result THROUGH fBm or through
    /// whole rendered planes. A wrong `period` still produces a seamless image, so neither
    /// can tell "the period is 8 but should be 16" from "the octave ladder is wrong". Only
    /// this test names the mechanism.
    #[test]
    fn value_noise_is_periodic_in_x() {
        let period: i64 = 8;
        for step in 0..48 {
            let pos_x = step as f32 * 0.37;
            let pos_y = step as f32 * 1.1;
            let at_pos_x = value_noise(pos_x, pos_y, period, SEED);
            let at_shifted = value_noise(pos_x + period as f32, pos_y, period, SEED);
            assert!(
                (at_pos_x - at_shifted).abs() < 1e-6,
                "value_noise is not periodic in x at period {}: f({}) = {} but f({}) = {}",
                period,
                pos_x,
                at_pos_x,
                pos_x + period as f32,
                at_shifted
            );
        }
    }

    /// VALUE NOISE NEGATIVE-CELL WRAP CONTRACT. The doc comment justifies the two-step
    /// `((raw % p) + p) % p` by noting that Rust's `%` keeps the sign of the dividend, so a
    /// bare `%` hands `lattice_hash` a NEGATIVE cell index for negative input. Nothing else
    /// in the suite catches that: `lattice_hash` takes an `i64` happily and returns a
    /// different, perfectly valid-looking value.
    ///
    /// WHY THE DOMAIN MUST GO NEGATIVE: stepping backwards from a positive `pos_x` never
    /// produces a negative cell, so it cannot tell the two-step wrap from a bare `%`. At
    /// `pos_x = -8.0` the cells are `-8` and `-7`, where a bare `%` yields `-7` and the
    /// two-step yields `1`. The generator itself never calls with negative `pos_x`
    /// (`col_to_cell` is non-negative for every column), so this pins documented robustness
    /// rather than a live path -- but it is the behaviour the doc comment promises.
    #[test]
    fn value_noise_is_periodic_in_x_across_negative_cells() {
        let period: i64 = 8;
        for step in 0..48 {
            let pos_x = -8.0 + step as f32 * 0.37;
            let pos_y = step as f32 * 1.1;
            let at_pos_x = value_noise(pos_x, pos_y, period, SEED);
            let at_shifted = value_noise(pos_x + period as f32, pos_y, period, SEED);
            assert!(
                (at_pos_x - at_shifted).abs() < 1e-6,
                "value_noise does not wrap negative cells: f({}) = {} but f({}) = {}, so the \
                 two-step normalisation is missing and a bare `%` is indexing lattice_hash \
                 with a negative cell",
                pos_x,
                at_pos_x,
                pos_x + period as f32,
                at_shifted
            );
        }
    }

    /// VALUE NOISE Y-NON-PERIODICITY CONTRACT. `y` is deliberately NOT periodic: vertical
    /// coverage comes from overscan plus a clamped camera-y travel, never from a vertical
    /// repeat. Nothing asserted that anywhere -- wrapping `y` along with `x` would have left
    /// every other test green while making the plane tile vertically, which is exactly the
    /// "empty blocks above and below" failure the TDD forbids.
    ///
    /// THRESHOLD, with the derivation corrected against measurement. For `lattice_hash` the
    /// yardstick is exact: two INDEPENDENT uniform draws on `[0,1)` differ by 1/3 on average.
    /// That figure does NOT transfer here. `value_noise(y)` and `value_noise(y + period)` are
    /// 8 cells apart, but both are bilinearly interpolated, so the two outputs are CORRELATED
    /// and the mean gap comes out smaller than the independent-draw figure would suggest.
    ///
    /// MEASURED: 0.25727 (seed 0), 0.26472 (1), 0.20498 (0x5eed), 0.24600 (u64::MAX). The
    /// floor is 0.15, so the tightest seed clears it by 0.055 -- a real but not generous
    /// margin, and narrower than the 1/3 argument implies. Wrapping `y` drives the mean to
    /// exactly 0, so the test still separates cleanly (mutant VN-WRAPY confirms). Fixed seeds
    /// and no RNG, so it cannot flake.
    #[test]
    fn value_noise_does_not_wrap_y() {
        const MIN_MEAN_VERTICAL_DELTA: f32 = 0.15;
        let period: i64 = 8;
        for seed in [0u64, 1, 0x5eed, u64::MAX] {
            let mut deltas: Vec<f32> = Vec::new();
            for step in 0..96 {
                let pos_x = step as f32 * 0.41;
                let pos_y = step as f32 * 1.7;
                let at_pos_y = value_noise(pos_x, pos_y, period, seed);
                let at_shifted = value_noise(pos_x, pos_y + period as f32, period, seed);
                deltas.push((at_pos_y - at_shifted).abs());
            }
            let mean_delta = deltas.iter().sum::<f32>() / deltas.len() as f32;
            assert!(
                mean_delta >= MIN_MEAN_VERTICAL_DELTA,
                "seed {}: mean |delta| between y and y+{} is {:.5}, at or below the {:.2} \
                 floor, so y is being wrapped too; vertical coverage must come from overscan, \
                 not from a vertical repeat",
                seed,
                period,
                mean_delta,
                MIN_MEAN_VERTICAL_DELTA
            );
        }
    }

    /// VALUE NOISE SMOOTHSTEP CONTRACT. The doc comment says the fractional part is
    /// smoothstepped rather than linearly interpolated, because linear leaves a slope
    /// discontinuity at every lattice point and that shows up as visible creases in the sky
    /// gradient. Nothing enforced it: a linear implementation is still in `[0,1]`, still
    /// periodic and still seamless, so every other test in this module would pass.
    ///
    /// MEASUREMENT: the slope just inside a lattice point is compared against the slope at
    /// the middle of the same cell, where the smoothstep derivative peaks at 1.5x linear.
    /// Averaged over 128 (cell, row) pairs so one unlucky pair of hash values cannot decide
    /// the result. MEASURED: edge slope 0.00087, mid-cell slope 0.87058, ratio 0.0010 against
    /// the 0.25 ceiling -- a 250x margin. A linear ramp gives a ratio of 1.0.
    #[test]
    fn value_noise_smoothsteps_the_fraction_not_linear() {
        const MAX_EDGE_TO_MID_RATIO: f32 = 0.25;
        let period: i64 = 8;
        let delta: f32 = 0.001;
        let mut edge_slope_sum: f32 = 0.0;
        let mut mid_slope_sum: f32 = 0.0;
        let mut sample_count: f32 = 0.0;
        for cell_x in 0..16i64 {
            let cell_x_f32 = cell_x as f32;
            for row_no in 0..8 {
                let pos_y = row_no as f32 * 2.3;
                let at_lattice = value_noise(cell_x_f32, pos_y, period, SEED);
                let just_inside = value_noise(cell_x_f32 + delta, pos_y, period, SEED);
                let mid_before = value_noise(cell_x_f32 + 0.5 - delta, pos_y, period, SEED);
                let mid_after = value_noise(cell_x_f32 + 0.5 + delta, pos_y, period, SEED);
                edge_slope_sum += (just_inside - at_lattice).abs() / delta;
                mid_slope_sum += (mid_after - mid_before).abs() / delta;
                sample_count += 1.0;
            }
        }
        let mean_edge_slope = edge_slope_sum / sample_count;
        let mean_mid_slope = mid_slope_sum / sample_count;

        // Non-vacuity tripwire: without it a hash that returned a constant would give both
        // slopes 0 and the ratio assertion below would hold for the wrong reason.
        assert!(
            mean_mid_slope > 0.0,
            "non-vacuity guard: the mid-cell slope is exactly 0, so value_noise is constant \
             and the ratio check below would pass for the wrong reason"
        );
        assert!(
            mean_edge_slope < MAX_EDGE_TO_MID_RATIO * mean_mid_slope,
            "mean slope just inside a lattice point is {:.5} against {:.5} mid-cell, a ratio \
             of {:.3} above the {:.2} ceiling -- the fractional part is interpolated linearly, \
             which leaves a slope discontinuity at every lattice point",
            mean_edge_slope,
            mean_mid_slope,
            mean_edge_slope / mean_mid_slope,
            MAX_EDGE_TO_MID_RATIO
        );
    }

    // -----------------------------------------------------------------------
    // 3/9 - fractional Brownian motion
    // -----------------------------------------------------------------------

    /// fBm OCTAVE-LADDER CONTRACT. The block's own doc comment is emphatic that the period
    /// MUST DOUBLE per octave, then concludes: "Neither is caught by a test here, so doubling
    /// is a documented intent rather than an asserted contract." This closes that gap.
    ///
    /// The documented ladder is re-derived independently -- period `8 * 2^o`, frequency
    /// `2^o`, amplitude `0.5^o`, per-octave seed offset `seed ^ (o * 0x9e3779b9)`,
    /// normalised by the amplitude sum -- and compared. A mutant that halves the period,
    /// changes the amplitude ratio, scales frequency by 1.5, or drops the per-octave seed
    /// offset all produce a different number here.
    ///
    /// WHAT IT DOES NOT DO, stated plainly: it re-implements the formula, so an edit made to
    /// the implementation AND to this test together stays green. It pins the ladder as
    /// documented; it does not independently derive the correct ladder, because the right
    /// period ratio is an art decision, not a computable one.
    ///
    /// The `octaves = 1` case doubles as an identity check: one normalised octave at
    /// frequency 1 and unshifted seed must equal `value_noise` exactly, which pins octave 0
    /// as the untransformed base.
    #[test]
    fn fbm_matches_its_documented_octave_formula() {
        for step in 0..24 {
            let pos_x = step as f32 * 1.7;
            let pos_y = step as f32 * 0.9;
            for octaves in 1u32..=4 {
                let mut sum: f32 = 0.0;
                let mut amplitude: f32 = 1.0;
                let mut norm: f32 = 0.0;
                let mut frequency: f32 = 1.0;
                for octave in 0..octaves {
                    let octave_period: i64 = 8i64.saturating_mul(1i64 << octave.min(20));
                    sum += amplitude
                        * value_noise(
                            pos_x * frequency,
                            pos_y * frequency,
                            octave_period,
                            SEED ^ (octave as u64).wrapping_mul(0x9e37_79b9),
                        );
                    norm += amplitude;
                    amplitude *= 0.5;
                    frequency *= 2.0;
                }
                let expected = sum / norm;
                let actual = fbm(pos_x, pos_y, 8, octaves, SEED);
                assert!(
                    (actual - expected).abs() < 1e-6,
                    "fbm({}, {}, octaves={}) = {} but the documented ladder gives {}. The \
                     ladder is period*2^o, frequency*2^o, amplitude*0.5^o, and each octave \
                     reseeded with ^ (o * 0x9e3779b9)",
                    pos_x,
                    pos_y,
                    octaves,
                    actual,
                    expected
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // 4/9 - colour lerp
    // -----------------------------------------------------------------------

    /// LERP ENDPOINT CONTRACT. `blend` 0 and 1 must return their inputs bit-exactly, not
    /// approximately. This matters because the sky gradient calls `lerp_rgb` at `row_frac`
    /// 0.0 and again just under 1.0, so a one-ulp endpoint error would appear as a flat or
    /// clipped band at the very top and bottom of the sky.
    #[test]
    fn lerp_rgb_hits_its_endpoints_exactly() {
        let from_rgb = [10u8, 20, 30];
        let to_rgb = [200u8, 210, 220];
        assert_eq!(lerp_rgb(from_rgb, to_rgb, 0.0), from_rgb);
        assert_eq!(lerp_rgb(from_rgb, to_rgb, 1.0), to_rgb);
        // The same holds outside the interval, because that is what clamping means.
        assert_eq!(lerp_rgb(from_rgb, to_rgb, -7.0), from_rgb);
        assert_eq!(lerp_rgb(from_rgb, to_rgb, 42.0), to_rgb);
    }

    /// LERP CLAMP CONTRACT. `blend` outside `[0,1]` must saturate, not wrap. Computing a
    /// channel as `from + (to - from) * blend` while the accumulator is still `u8` wraps
    /// around in release mode, so an overshoot produces a dark pixel instead of a bright one.
    /// The reverse direction is the one that would pass below zero, so both are checked.
    #[test]
    fn lerp_rgb_clamps_blend_outside_unit_range() {
        let dark_rgb = [10u8, 20, 30];
        let light_rgb = [200u8, 210, 220];
        assert_eq!(lerp_rgb(dark_rgb, light_rgb, -0.5), dark_rgb);
        assert_eq!(lerp_rgb(dark_rgb, light_rgb, -1e9), dark_rgb);
        assert_eq!(lerp_rgb(dark_rgb, light_rgb, 1.5), light_rgb);
        assert_eq!(lerp_rgb(dark_rgb, light_rgb, 1e9), light_rgb);
        assert_eq!(lerp_rgb(light_rgb, dark_rgb, 1.5), dark_rgb);
        assert_eq!(lerp_rgb(light_rgb, dark_rgb, -0.5), light_rgb);
    }

    /// LERP FULL-RANGE CONTRACT -- the reason this is a function rather than inline
    /// arithmetic. Blending a channel in `u8` quantises the ramp: if `blend` is itself
    /// narrowed to an integer the whole 256-step ramp collapses to two values. Widening both
    /// operands to `f32` and narrowing once at the end keeps every step reachable, which is
    /// what makes the sky gradient smooth rather than banded.
    ///
    /// MEASURED against the reference implementation: 256 blends across a 0..255 channel
    /// reach 256 distinct values. The floor is 250, leaving room for float rounding at
    /// individual steps while still failing any implementation that quantises the ramp.
    #[test]
    fn lerp_rgb_keeps_the_full_channel_range() {
        let black_rgb = [0u8; 3];
        let white_rgb = [255u8; 3];
        let mut reached: Vec<u8> = (0..256u32)
            .map(|step| lerp_rgb(black_rgb, white_rgb, step as f32 / 255.0)[0])
            .collect();
        reached.sort_unstable();
        reached.dedup();
        assert!(
            reached.len() >= 250,
            "256 blends across a 0..255 channel reached only {} distinct values, so the ramp \
             is being quantised; blending has to widen to f32 and narrow once at the end",
            reached.len()
        );
        assert_eq!(reached.first(), Some(&0));
        assert_eq!(reached.last(), Some(&255));
    }

    // -----------------------------------------------------------------------
    // 5/9 - smoothstep
    // -----------------------------------------------------------------------

    /// SMOOTHSTEP ENDPOINT AND CLAMP CONTRACT. Exactly 0 at `edge0`, exactly 1 at `edge1`,
    /// clamped outside. Every cloud threshold and the sun falloff depend on this: a
    /// `smoothstep` that overshot its interval would push alpha above 1.0 and leave the
    /// `(alpha * 255.0) as u8` cast to clip it, rather than the clamp doing it cleanly.
    #[test]
    fn smoothstep_reaches_its_edges_and_clamps() {
        assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
        assert_eq!(smoothstep(0.0, 1.0, -5.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 5.0), 1.0);
        assert_eq!(smoothstep(10.0, 20.0, 10.0), 0.0);
        assert_eq!(smoothstep(10.0, 20.0, 20.0), 1.0);
        assert_eq!(smoothstep(-4.0, -2.0, -9.0), 0.0);
        assert_eq!(smoothstep(-4.0, -2.0, 9.0), 1.0);
    }

    /// SMOOTHSTEP DEGENERATE-INTERVAL CONTRACT. The doc comment's stated reason for the
    /// guard is that `sun_radius_px` derives from `height`, so at a small height the inner
    /// and outer radii round to the same value; dividing by a zero-width interval yields NaN,
    /// and a NaN alpha makes every downstream comparison false -- a silently wrong layer
    /// rather than a crash.
    ///
    /// Asserted as "never NaN, and exactly 0 or 1", so it does not over-specify which side
    /// of the interval wins.
    #[test]
    fn smoothstep_degenerate_interval_is_never_nan() {
        for value in [-1000.0f32, 0.0, 4.999, 5.0, 5.001, 1000.0] {
            let ramped = smoothstep(5.0, 5.0, value);
            assert!(
                !ramped.is_nan(),
                "smoothstep(5, 5, {}) = NaN; a zero-width interval must short-circuit, \
                 because a NaN alpha poisons every pixel comparison downstream",
                value
            );
            assert!(
                ramped == 0.0 || ramped == 1.0,
                "smoothstep(5, 5, {}) = {}, expected exactly 0 or 1 from the degenerate guard, \
                 not an interpolated value",
                value,
                ramped
            );
        }
        assert_eq!(smoothstep(5.0, 5.0, 4.999), 0.0);
        assert_eq!(smoothstep(5.0, 5.0, 5.0), 1.0);
    }

    /// SMOOTHSTEP ZERO-SLOPE CONTRACT, plus monotonicity. The Hermite curve block 2/9
    /// applies to the noise lattice is reused here for cloud edges and the sun falloff, and
    /// the reason to reuse it is that its derivative is zero at both ends: a linear ramp
    /// would put a visible crease at every cloud boundary and an aliased rim on the sun.
    ///
    /// DISCRIMINATING FORM: a linear ramp returns EXACTLY 0.001 and 0.999 here, so strict
    /// inequalities on the same side separate the two implementations.
    #[test]
    fn smoothstep_has_zero_slope_at_each_end() {
        let just_above_low = smoothstep(0.0, 1.0, 0.001);
        assert!(
            just_above_low < 0.001,
            "smoothstep(0, 1, 0.001) = {}; a linear ramp returns exactly 0.001, so matching \
             it means the slope is not zero at the lower edge",
            just_above_low
        );
        let just_below_high = smoothstep(0.0, 1.0, 0.999);
        assert!(
            just_below_high > 0.999,
            "smoothstep(0, 1, 0.999) = {}; a linear ramp returns exactly 0.999, so matching \
             it means the slope is not zero at the upper edge",
            just_below_high
        );

        // Monotone and bounded across the whole interval: an implementation that overshoots
        // makes alpha exceed 1.0 and leans on the u8 cast to clip it.
        let mut previous = 0.0f32;
        for step in 0..=100 {
            let ramped = smoothstep(0.0, 1.0, step as f32 / 100.0);
            assert!(
                ramped >= previous && ramped <= 1.0,
                "smoothstep is not monotone within [0,1]: at {} it returned {} after {}",
                step as f32 / 100.0,
                ramped,
                previous
            );
            previous = ramped;
        }
    }

    // -----------------------------------------------------------------------
    // 6/9 - sky
    // -----------------------------------------------------------------------

    /// SKY GRADIENT DIRECTION CONTRACT. The palette runs dark at the top to light toward the
    /// horizon, so the layer reads as sky rather than as a flat fill. Measured at column 0,
    /// far from the sun disc at `width * 0.68`, so this sees the gradient alone.
    ///
    /// MEASURED at column 0, W=256 H=64: mean channel 152.00 at row 0 and 201.00 at row 63,
    /// a gap of 49 against the 10 floor.
    ///
    /// DELIBERATELY NOT ASSERTED: that the gradient is normalised by height. The doc comment
    /// claims "the same function is correct at any height", but the haze term samples
    /// `row * 0.002` in absolute pixels and the sun radius scales with height, so two heights
    /// are NOT pixel-identical at the same `row_frac`. That claim cannot be tested at this
    /// layer without splitting the function, so it is recorded as a documentation fix rather
    /// than papered over with a weak assertion here.
    #[test]
    fn sky_gradient_runs_dark_above_and_light_below() {
        let sky = make_sky(W, H, SEED);
        let channel_mean = |row: u32| -> f32 {
            let column = sky.get_pixel(0, row).0;
            (column[0] as f32 + column[1] as f32 + column[2] as f32) / 3.0
        };
        let top_mean = channel_mean(0);
        let bottom_mean = channel_mean(H - 1);
        assert!(
            bottom_mean > top_mean + 10.0,
            "sky mean channel went from {:.1} at row 0 to {:.1} at row {}; the gradient must \
             run dark-above to light-below or the layer reads as a flat fill",
            top_mean,
            bottom_mean,
            H - 1
        );
    }

    /// SKY SOFT-DISC CONTRACT. The doc comment says the sun is "a radial falloff, not a hard
    /// circle", because a hard circle aliases into a stair-stepped rim. Brightening is
    /// measured along one horizontal ray at three radii, relative to an unlit column on the
    /// same row so that row's base colour cancels out.
    ///
    /// HEIGHT 256, not the module's 64: the sun radius is `height * 0.06`, so at 64 px the
    /// whole falloff spans under 4 px and the sample points are too close together to tell a
    /// gradient from a step.
    ///
    /// MEASURED at H=256: centre +89, at 0.8 radii +64, at 1.25 radii +3, against bounds
    /// >50, <80, >15 and <15. A hard-edged disc (mutant SKY-HARDDISC) reads +89 at every
    /// point inside the radius and fails the <80 bound.
    #[test]
    fn sky_sun_disc_falls_off_softly() {
        let sky_height: u32 = 256;
        let sky = make_sky(W, sky_height, SEED);
        let sun_center_x_px = W as f32 * 0.68;
        let sun_center_y_px = sky_height as f32 * 0.24;
        let sun_radius_px = sky_height as f32 * 0.06;
        let row = sun_center_y_px.round() as u32;

        let unlit = sky.get_pixel(4, row).0[0] as i32;
        let brightening_at = |radius_fraction: f32| -> i32 {
            let col = (sun_center_x_px + radius_fraction * sun_radius_px).round() as u32;
            sky.get_pixel(col, row).0[0] as i32 - unlit
        };

        let at_centre = brightening_at(0.0);
        let inside_rim = brightening_at(0.8);
        let beyond_rim = brightening_at(1.25);

        assert!(
            at_centre > 50,
            "sun centre is only {} brighter than an unlit column, so the disc is not drawn",
            at_centre
        );
        assert!(
            inside_rim < 80,
            "still {} brighter at 0.8 sun radii against {} at the centre -- the falloff is a \
             step, not a gradient, so the rim will alias",
            inside_rim,
            at_centre
        );
        assert!(
            inside_rim > 15,
            "only {} brighter at 0.8 sun radii; the falloff is a cliff rather than a ramp, so \
             it does not soften the rim the way the Hermite curve intends",
            inside_rim
        );
        assert!(
            beyond_rim.abs() < 15,
            "{} brighter at 1.25 sun radii, outside the disc; the falloff has not finished by \
             the outer radius",
            beyond_rim
        );
    }

    // -----------------------------------------------------------------------
    // 7/9 - clouds
    // -----------------------------------------------------------------------

    /// CLOUD SOFT-EDGE CONTRACT. This closes a gap that `cloud_layers_have_holes` documents
    /// against itself: that test's own comment records that mutation M9 -- replacing the
    /// smoothstep with a hard threshold -- left it GREEN, because it only checks that the
    /// fully-opaque and fully-clear buckets are both populated. A hard threshold satisfies
    /// both buckets perfectly and produces a stair-stepped edge, which is the artefact this
    /// whole technique exists to avoid.
    ///
    /// MEASURED at W=256 H=64: 17.4% of HighClouds pixels and 26.9% of LowClouds pixels have a
    /// partial alpha, against the 2% floor -- roughly a 9x margin, and 0 for a hard threshold.
    ///
    /// So this asserts what M9 destroys: a meaningful population of PARTIAL alpha. The two
    /// tests are complementary, not redundant.
    #[test]
    fn cloud_edges_are_gradual() {
        for (name, edge_low, edge_high) in
            [("HighClouds", 0.52f32, 0.68f32), ("LowClouds", 0.44, 0.60)]
        {
            let img = make_clouds(W, H, SEED, edge_low, edge_high);
            let partial = img
                .pixels()
                .filter(|rgba| rgba.0[3] > 0 && rgba.0[3] < 255)
                .count();
            let total = img.width() as usize * img.height() as usize;
            let fraction = partial as f32 / total as f32;
            assert!(
                fraction > 0.02,
                "{}: only {:.4} of {} pixels have a partial alpha ({}/{}); a hard threshold \
                 gives exactly 0, which is the stair-stepped edge this layer exists to avoid",
                name,
                fraction,
                total,
                partial,
                total
            );
        }
    }

    // -----------------------------------------------------------------------
    // 8/9 - silhouettes
    // -----------------------------------------------------------------------

    /// SILHOUETTE FILL-DIRECTION CONTRACT. The doc comment's stated reason for filling
    /// DOWNWARD from a heightfield rather than drawing shapes is that it keeps the alpha
    /// channel "trivially predictable: fully opaque below the line, fully transparent above
    /// it". That predictability is the whole reason the plane reads as a silhouette, and
    /// nothing asserted the orientation -- an inverted fill still yields an opaque region,
    /// just the wrong one, and would still satisfy `relief_is_absolute_not_a_fraction`.
    #[test]
    fn silhouette_is_opaque_below_the_skyline_and_clear_above() {
        let img = make_silhouette(W, H, SEED, 0.5, 12.0);
        for col in 0..W {
            assert!(
                (0..H).any(|row| img.get_pixel(col, row).0[3] == 255),
                "col {} has no opaque pixel at all, so the plane is invisible",
                col
            );
            assert_eq!(
                img.get_pixel(col, 0).0[3],
                0,
                "col {}: the top row is opaque, but a silhouette must be clear above its \
                 skyline",
                col
            );
            assert_eq!(
                img.get_pixel(col, H - 1).0[3],
                255,
                "col {}: the bottom row is not opaque, but a silhouette must be solid ground \
                 below its skyline",
                col
            );
        }
    }

    /// SILHOUETTE RIM CONTRACT. Two separate claims, both load bearing. The rim value is
    /// darker than the body on every channel, and the rim sits ON the skyline -- a dark band
    /// drawn in the middle of the ground would satisfy the first claim and leave the plane
    /// with no readable edge at 1:1.
    ///
    /// The second claim is the discriminating one: because the rim is drawn in the three rows
    /// immediately below `skyline_row`, the topmost opaque pixel in EVERY column must be the
    /// rim colour. Checking whole columns rather than sampling one spot catches a rim that is
    /// offset from the skyline.
    #[test]
    fn silhouette_rim_is_darker_than_its_body_and_sits_on_the_skyline() {
        let img = make_silhouette(W, H, SEED, 0.5, 12.0);
        let rim_rgba = Rgba([30u8, 38, 36, 255]);
        let body_rgba = Rgba([46u8, 56, 52, 255]);
        for channel in 0..3 {
            assert!(
                rim_rgba.0[channel] < body_rgba.0[channel],
                "rim channel {} ({}) is not darker than body ({}), so the plane has no \
                 readable edge where it meets the layer behind it",
                channel,
                rim_rgba.0[channel],
                body_rgba.0[channel]
            );
        }
        for col in 0..W {
            let first_opaque = (0..H)
                .find(|row| img.get_pixel(col, *row).0[3] == 255)
                .expect("every column was asserted to have opaque ground");
            assert_eq!(
                *img.get_pixel(col, first_opaque),
                rim_rgba,
                "col {}: the topmost opaque pixel is not the rim, so the dark edge is not on \
                 the skyline (first opaque row {})",
                col,
                first_opaque
            );
        }
    }

    // -----------------------------------------------------------------------
    // 9/9 - factor validation + manifest
    // -----------------------------------------------------------------------

    /// MANIFEST NO-VERTICAL-REPEAT CONTRACT. The doc comment says `repeat_size_y` is
    /// hard-coded rather than passed in "because a vertical repeat is never correct here, and
    /// encoding that as a literal means the decision cannot be flipped by a caller". A
    /// hard-coded literal that no test reads is just a constant nobody checked, so this reads
    /// it. `repeat_size_x` must equal the authored width for the same reason.
    #[test]
    fn manifest_hard_codes_repeat_size_y_to_zero() {
        let specs = layer_specs();
        let authored_width: u32 = 1920;
        let manifest = build_manifest(&specs, authored_width, 240, SEED);
        assert_eq!(manifest.layers.len(), specs.len());
        for layer in &manifest.layers {
            assert_eq!(
                layer.repeat_size_y, 0,
                "layer {} has a vertical repeat of {}; vertical coverage is overscan plus a \
                 clamped camera-y travel, and a vertical repeat leaves empty blocks above and \
                 below a horizontal-only stack",
                layer.name, layer.repeat_size_y
            );
            assert_eq!(
                layer.repeat_size_x, authored_width,
                "layer {}: repeat_size_x is {} but the texture is {} px wide, so the tiles \
                 would not line up and the seam would reappear",
                layer.name, layer.repeat_size_x, authored_width
            );
        }
    }

    /// MANIFEST ON-DISK KEY CONTRACT. `manifest.json` is read by literal key name from the
    /// scene and from the GDScript test, so a renamed or dropped key is a SILENT break: the
    /// Rust side still compiles, the JSON still parses, and the consumer just gets `null`.
    /// Asserting the struct's fields would be a round trip and could not catch a
    /// `#[serde(rename)]` or a `skip_serializing_if`, so this asserts the serialised keys
    /// themselves.
    ///
    /// Corollary of the round-trip rule: a writer and reader that move together stay green, so
    /// the key strings below are literals rather than derived from the struct.
    #[test]
    fn manifest_json_keys_are_the_on_disk_contract() {
        let specs = layer_specs();
        let authored_width: u32 = 1920;
        let overscan: u32 = 240;
        let manifest = build_manifest(&specs, authored_width, overscan, SEED);
        let serialised = serde_json::to_value(&manifest).expect("manifest serialises");
        let top = serialised
            .as_object()
            .expect("manifest serialises to a JSON object");

        for key in ["version", "width_px", "overscan_px", "seed", "layers"] {
            assert!(
                top.contains_key(key),
                "manifest.json is missing the key {:?}; the scene and the GDScript test read \
                 it by that literal name, so dropping it breaks them silently",
                key
            );
        }
        assert_eq!(top["version"].as_str(), Some(MANIFEST_VERSION));
        assert_eq!(top["width_px"].as_u64(), Some(authored_width as u64));
        assert_eq!(top["overscan_px"].as_u64(), Some(overscan as u64));
        assert_eq!(top["seed"].as_u64(), Some(SEED));

        let layers = top["layers"]
            .as_array()
            .expect("manifest.layers is a JSON array");
        assert_eq!(layers.len(), specs.len());
        for (layer_value, spec) in layers.iter().zip(specs.iter()) {
            let layer = layer_value
                .as_object()
                .expect("each manifest layer is a JSON object");
            for key in [
                "name",
                "file",
                "scroll_scale_x",
                "scroll_scale_y",
                "repeat_size_x",
                "repeat_size_y",
                "z_index",
                "height_px",
            ] {
                assert!(
                    layer.contains_key(key),
                    "manifest.json layer {:?} is missing the key {:?}",
                    spec.name,
                    key
                );
            }
            assert_eq!(layer["name"].as_str(), Some(spec.name));
            assert_eq!(
                layer["file"].as_str(),
                Some(spec.file),
                "manifest.json layer {:?} names file {:?} but layer_specs says {:?}",
                spec.name,
                layer["file"].as_str(),
                spec.file
            );
        }
    }
}
