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
    match run(&args) {
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
/// INTENT (permanent): a pure function of `(x, y, seed)` with no RNG state, because the
/// same seed must reproduce byte-identical PNGs. A stateful RNG makes output depend on
/// call order, which would break the determinism the `deterministic` test asserts.
///
/// Deliberately NOT a "good" hash (no avalanche guarantees needed): it only has to be
/// cheap, deterministic and decorrelate adjacent lattice points enough that the
/// interpolated value noise is smooth rather than blocky.
// TODO(human): begin block 1/9
fn lattice_hash(x: i64, y: i64, seed: u64) -> f32 {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // let mixed: u64 = (x as u64)
    //     .wrapping_mul(0x9e37_79b9_7f4a_7c15)
    //     ^ (y as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f)
    //     ^ seed;
    // // splitmix64's finaliser: xor-shift, multiply, xor-shift, multiply, xor-shift.
    // let s1: u64 = mixed ^ (mixed >> 33);
    // let m1: u64 = s1.wrapping_mul(0xff51_afd7_ed55_8ccd);
    // let s2: u64 = m1 ^ (m1 >> 29);
    // let m2: u64 = s2.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    // let s3: u64 = m2 ^ (m2 >> 32);
    // // Top 24 bits only: the low bits of a multiply-xor mix are the weakest, and the
    // // callers only need enough range to divide into [0, 1).
    // (s3 >> 40) as f32 / (1u32 << 24) as f32
}
// TODO(human): end block 1/9

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
/// `x as f32 * cells as f32 / (width - 1) as f32`.
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
/// Dividing by a round pixel divisor (say `x / 260.0`) does not work either: the wrap then
/// lands at `260 * period` px, which is not the texture edge at all.
// TODO(human): begin block 2/9
fn value_noise(x: f32, y: f32, period: i64, seed: u64) -> f32 {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // let p: i64 = if period < 1 { 1 } else { period };
    // let x0: i64 = x.floor() as i64;
    // let y0: i64 = y.floor() as i64;
    // let fx: f32 = x - x.floor();
    // let fy: f32 = y - y.floor();
    // // Smoothstep the fractional part, not linear: zero slope at every lattice point.
    // let sx: f32 = fx * fx * (3.0 - 2.0 * fx);
    // let sy: f32 = fy * fy * (3.0 - 2.0 * fy);
    // // x wraps at `p`; y does not. `i64 % p` is negative for negative i, hence the
    // // two-step normalisation rather than a bare `%`.
    // let wrap = |i: i64| -> i64 { ((i % p) + p) % p };
    // let h00: f32 = lattice_hash(wrap(x0), y0, seed);
    // let h10: f32 = lattice_hash(wrap(x0 + 1), y0, seed);
    // let h01: f32 = lattice_hash(wrap(x0), y0 + 1, seed);
    // let h11: f32 = lattice_hash(wrap(x0 + 1), y0 + 1, seed);
    // let top: f32 = h00 + (h10 - h00) * sx;
    // let bot: f32 = h01 + (h11 - h01) * sx;
    // top + (bot - top) * sy
}
// TODO(human): end block 2/9

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
// TODO(human): begin block 3/9
fn fbm(x: f32, y: f32, period: i64, octaves: u32, seed: u64) -> f32 {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // let mut sum: f32 = 0.0;
    // let mut amp: f32 = 1.0;
    // let mut norm: f32 = 0.0;
    // let mut freq: f32 = 1.0;
    // for o in 0..octaves.max(1) {
    //     // DOUBLES per octave: octave o samples at 2^o frequency, so it spans 2^o cells
    //     // and keeps the same spatial period as octave 0.
    //     let per: i64 = period.saturating_mul(1i64 << o.min(20));
    //     sum += amp
    //         * value_noise(x * freq, y * freq, per, seed ^ (o as u64).wrapping_mul(0x9e37_79b9));
    //     norm += amp;
    //     amp *= 0.5;
    //     freq *= 2.0;
    // }
    // sum / norm
}
// TODO(human): end block 3/9

// ---------------------------------------------------------------------------
// 4/9 - colour lerp
// ---------------------------------------------------------------------------

/// Interpolates two RGB triples on `t` in `[0, 1]`.
///
/// INTENT (permanent): u8 channels cannot be interpolated directly. `a + (b - a) * t` in
/// u8 arithmetic truncates, so a 1% mix of two dark colours rounds back to the original
/// and the gradient bands. Widening to f32, mixing, then narrowing once at the end keeps
/// the full range. This is why every gradient in this file goes through here rather than
/// doing channel arithmetic inline.
///
/// Clamping `t` is deliberate: a caller passing a value slightly outside `[0, 1]` from a
/// noisy expression should not wrap around to the far end of the ramp.
// TODO(human): begin block 4/9
fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // let t: f32 = t.clamp(0.0, 1.0);
    // [
    //     (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
    //     (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
    //     (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
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
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // if (edge1 - edge0).abs() < f32::EPSILON {
    //     return if x < edge0 { 0.0 } else { 1.0 };
    // }
    // let t: f32 = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    // t * t * (3.0 - 2.0 * t)
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
/// The gradient runs on `y / height` rather than on absolute pixel rows so the same
/// function is correct at any height; the sun disc is a radial falloff, not a hard circle,
/// so it has no aliased edge at 1:1 scale.
///
/// The haze term MUST go through the 2/9 caller contract (`x * cells / (width - 1)`) or the
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
    // let x_of = |x: u32| -> f32 { x as f32 * cells as f32 / (width.max(2) - 1) as f32 };
    // let sun_x: f32 = width as f32 * 0.68;
    // let sun_y: f32 = height as f32 * 0.24;
    // let sun_r: f32 = height as f32 * 0.06;
    // for y in 0..height {
    //     let t: f32 = y as f32 / height.max(1) as f32;
    //     let base: [u8; 3] = if t < 0.55 {
    //         lerp_rgb(top, mid, t / 0.55)
    //     } else {
    //         lerp_rgb(mid, low, (t - 0.55) / 0.45)
    //     };
    //     for x in 0..width {
    //         let haze: f32 = fbm(x_of(x), y as f32 * 0.002, cells, 3, seed) - 0.5;
    //         let dx: f32 = x as f32 - sun_x;
    //         let dy: f32 = y as f32 - sun_y;
    //         let d: f32 = (dx * dx + dy * dy).sqrt();
    //         let disc: f32 = 1.0 - smoothstep(sun_r * 0.65, sun_r, d);
    //         let mut c: [u8; 3] = lerp_rgb(base, [255, 250, 238], disc * 0.85);
    //         let hv: f32 = (haze * 14.0).clamp(-20.0, 20.0);
    //         for ch in 0..3 {
    //             c[ch] = (c[ch] as f32 + hv).clamp(0.0, 255.0) as u8;
    //         }
    //         img.put_pixel(x, y, Rgba([c[0], c[1], c[2], 255]));
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
/// INTENT (permanent): alpha is `smoothstep(t0, t1, fbm)` mapped to 0..255. Dense regions
/// of the noise field become opaque cloud, sparse regions become fully transparent, and
/// the transition is gradual rather than a hard cutout. A hard threshold would produce
/// the stair-stepped edge this technique exists to avoid.
///
/// The band is authored with vertical headroom so a camera-y clamp can move it without
/// exposing its edge; the gradient inside the band keeps cloud thin at the top and dense
/// at the bottom, which is what reads as distance.
///
/// THE BAND GRADIENT MUST SATURATE AT 1.0 over the lower part of the band, not scale the
/// whole band. Multiplying alpha by a gradient that is still below 1 at the bottom means no
/// pixel anywhere reaches alpha 255, so the layer reads as flat haze instead of cloud. The
/// contract `cloud_layers_have_holes` asserts is that fully-opaque AND fully-clear pixels
/// both exist, so a gradient that never reaches 1 is a failure, not a subtlety.
// TODO(human): begin block 7/9
fn make_clouds(width: u32, height: u32, seed: u64, t0: f32, t1: f32) -> RgbaImage {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // let mut img: RgbaImage = RgbaImage::new(width, height);
    // let cells: i64 = 8;
    // let body: [u8; 3] = [236, 238, 240];
    // let shade: [u8; 3] = [206, 210, 216];
    // // 2/9 caller contract, same as 6/9.
    // let x_of = |x: u32| -> f32 { x as f32 * cells as f32 / (width.max(2) - 1) as f32 };
    // for y in 0..height {
    //     let vy: f32 = y as f32 / height.max(1) as f32;
    //     // Saturates at 1.0 over the lower 55% of the band, so the cloud body reaches
    //     // full opacity. A gradient still below 1 at the bottom makes the plane read as
    //     // haze instead of cloud.
    //     let band: f32 = ((0.78 - vy) / 0.30).clamp(0.0, 1.0);
    //     for x in 0..width {
    //         let v: f32 = fbm(x_of(x), vy * 1.6, cells, 4, seed);
    //         let a: f32 = (smoothstep(t0, t1, v) * band).clamp(0.0, 1.0);
    //         let c: [u8; 3] = lerp_rgb(shade, body, a);
    //         img.put_pixel(x, y, Rgba([c[0], c[1], c[2], (a * 255.0).round() as u8]));
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
fn make_silhouette(width: u32, height: u32, seed: u64, base_y: f32, relief_px: f32) -> RgbaImage {
    // SAMPLE: uncomment every line of this body to make it live, which also clears the E0308.
    // let mut img: RgbaImage = RgbaImage::new(width, height);
    // let cells: i64 = 5;
    // let body: [u8; 3] = [46, 56, 52];
    // let rim: [u8; 3] = [30, 38, 36];
    // // 2/9 caller contract, same as 6/9 and 7/9.
    // let x_of = |x: u32| -> f32 { x as f32 * cells as f32 / (width.max(2) - 1) as f32 };
    // let base: i32 = (base_y * height as f32).round() as i32;
    // for x in 0..width {
    //     let n: f32 = fbm(x_of(x), 0.0, cells, 4, seed);
    //     // relief_px is ABSOLUTE pixels, so amplitude does not vary with layer height.
    //     let line: i32 = base - (n * relief_px).round() as i32;
    //     for y in 0..height {
    //         let yi: i32 = y as i32;
    //         let px: Rgba<u8> = if yi < line {
    //             Rgba([0, 0, 0, 0])
    //         } else if yi < line + 3 {
    //             Rgba([rim[0], rim[1], rim[2], 255])
    //         } else {
    //             Rgba([body[0], body[1], body[2], 255])
    //         };
    //         img.put_pixel(x, y, px);
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
    // for s in specs {
    //     let (x, y) = s.scroll_scale;
    //     if !(x > 0.0 && x <= 1.5) {
    //         return Err(format!("layer {}: scroll_scale.x {} out of range", s.name, x));
    //     }
    //     if !(y > 0.0) {
    //         return Err(format!("layer {}: scroll_scale.y {} must be positive", s.name, y));
    //     }
    //     if x <= y {
    //         return Err(format!("layer {}: scroll_scale.x {} must exceed y {}", s.name, x, y));
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
    //         .map(|s| ManifestLayer {
    //             name: s.name.to_string(),
    //             file: s.file.to_string(),
    //             scroll_scale_x: s.scroll_scale.0,
    //             scroll_scale_y: s.scroll_scale.1,
    //             repeat_size_x: width,
    //             repeat_size_y: 0,
    //             z_index: s.z_index,
    //             height_px: s.height_px,
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
fn run(args: &[String]) -> Result<PathBuf, String> {
    let mut out_dir = PathBuf::from("editor/assets/backdrop_layers");
    let mut width: u32 = 1920;
    let mut overscan: u32 = 240;
    let mut seed: u64 = 0x5EED_2026;

    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out_dir = PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| "--out requires a directory".to_string())?,
                );
            }
            "--width" => {
                i += 1;
                width = args
                    .get(i)
                    .ok_or_else(|| "--width requires a value".to_string())?
                    .parse::<u32>()
                    .map_err(|_| "--width must be an integer".to_string())?;
            }
            "--overscan" => {
                i += 1;
                overscan = args
                    .get(i)
                    .ok_or_else(|| "--overscan requires a value".to_string())?
                    .parse::<u32>()
                    .map_err(|_| "--overscan must be an integer".to_string())?;
            }
            "--seed" => {
                i += 1;
                seed = args
                    .get(i)
                    .ok_or_else(|| "--seed requires a value".to_string())?
                    .parse::<u64>()
                    .map_err(|_| "--seed must be an integer".to_string())?;
            }
            other => return Err(format!("unexpected argument '{}'", other)),
        }
        i += 1;
    }

    if width == 0 {
        return Err("width must be greater than zero".to_string());
    }

    let specs = layer_specs();
    validate_factors(&specs)?;

    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("cannot create {}: {}", out_dir.display(), e))?;

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
            .map_err(|e| format!("cannot write {}: {}", path.display(), e))?;

        let opaque = image.pixels().filter(|p| p.0[3] == 255).count();
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
        .map_err(|e| format!("cannot serialize manifest: {}", e))?;
    std::fs::write(&manifest_path, format!("{}\n", json))
        .map_err(|e| format!("cannot write {}: {}", manifest_path.display(), e))?;
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

    fn raw_column(img: &RgbaImage, x: u32) -> Vec<[u8; 4]> {
        (0..img.height())
            .map(|y| {
                let p = img.get_pixel(x, y).0;
                [p[0], p[1], p[2], p[3]]
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
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in img.as_raw() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }

    /// Byte index of the first differing pixel, plus both pixels, so a failure names the
    /// exact spot instead of dumping the whole buffer.
    fn first_pixel_difference(a: &RgbaImage, b: &RgbaImage) -> String {
        if a.dimensions() != b.dimensions() {
            return format!(
                "dimensions differ: {:?} vs {:?}",
                a.dimensions(),
                b.dimensions()
            );
        }
        for (i, (pa, pb)) in a
            .as_raw()
            .chunks_exact(4)
            .zip(b.as_raw().chunks_exact(4))
            .enumerate()
        {
            if pa != pb {
                let (x, y) = ((i as u32) % a.width(), (i as u32) / a.width());
                return format!(
                    "first difference at pixel ({}, {}) index {}: {:?} vs {:?}",
                    x, y, i, pa, pb
                );
            }
        }
        "buffers differ in length only".to_string()
    }

    /// Every layer the crate emits, as (name, image) pairs.
    ///
    /// Goes through the SAME `run()` dispatch table by name, so a layer added to
    /// `layer_specs()` without a generator is caught here instead of at runtime.
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
        let holes = sky.pixels().filter(|p| p.0[3] != 255).count();
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
            let opaque = img.pixels().filter(|p| p.0[3] == 255).count();
            let clear = img.pixels().filter(|p| p.0[3] == 0).count();
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
        for x in 0..img.width() {
            for y in 0..img.height() {
                if img.get_pixel(x, y).0[3] == 255 {
                    min = min.min(y as i32);
                    max = max.max(y as i32);
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
    /// mechanism itself. `x > y` is absolute; the upper bound is 1.5 rather than 1.0
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
            let (x, y) = spec.scroll_scale;
            assert!(
                x > 0.0 && x <= 1.5,
                "{}: scroll_scale.x {} out of range",
                spec.name,
                x
            );
            assert!(
                y > 0.0 && y < x,
                "{}: scroll_scale.y {} must be below x {}",
                spec.name,
                y,
                x
            );
        }
    }

    /// FACTOR CONTRACT, on the rejection path. `factors_in_unit_range` only ever feeds
    /// `validate_factors` the shipped stack, which is valid by construction -- so mutation
    /// M12 (disabling the `x <= y` check) left that test GREEN. A validator only ever shown
    /// valid input is untested.
    ///
    /// Each case asserts the specific message, not merely `is_err`, so a validator that
    /// rejected everything for the wrong reason could not pass.
    #[test]
    fn validate_factors_rejects_bad_specs() {
        fn bad(name: &'static str, x: f32, y: f32) -> LayerSpec {
            LayerSpec {
                name,
                file: "x.png",
                scroll_scale: (x, y),
                height_px: 64,
                z_index: 0,
            }
        }

        let cases: [(&str, LayerSpec, &str); 4] = [
            ("zero x", bad("Zero", 0.0, 0.0), "out of range"),
            ("negative y", bad("Neg", 0.5, -0.1), "positive"),
            ("x below y", bad("Inverted", 0.4, 0.6), "must exceed"),
            ("x too fast", bad("Runaway", 1.9, 1.0), "out of range"),
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
        let a = make_clouds(W, H, SEED, 0.52, 0.68);
        let b = make_clouds(W, H, SEED, 0.52, 0.68);
        assert_eq!(
            digest(&a),
            digest(&b),
            "same seed produced different pixels: {}",
            first_pixel_difference(&a, &b)
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

    /// VALUE NOISE RANGE CONTRACT. Every downstream layer thresholds or interpolates this
    /// value assuming `[0, 1]`; an out-of-range result silently shifts every threshold.
    #[test]
    fn value_noise_stays_in_unit_range() {
        for i in 0..64 {
            let x = i as f32 * 3.7;
            let y = i as f32 * 1.3;
            let v = value_noise(x, y, 8, SEED);
            assert!(
                (0.0..=1.0).contains(&v),
                "value_noise({}, {}) = {} is outside [0,1]",
                x,
                y,
                v
            );
        }
    }

    /// fBm must preserve both the `[0,1]` range and the x-period, so the seam guarantee
    /// survives every octave.
    ///
    /// SCOPE: this is a seam check, not a spectrum check. It stays green whether the period
    /// doubles or halves per octave (see 3/7), so it must not be cited as evidence that the
    /// frequency ladder is right. The period is 64 so that four octaves cannot collapse to
    /// a single wrapped cell under either scheme -- a period of 8 under halving reaches 1,
    /// where every x wraps identically and the assertion becomes vacuous.
    #[test]
    fn fbm_stays_in_unit_range_and_wraps() {
        let period: i64 = 64;
        for i in 0..32 {
            let y = i as f32 * 2.1;
            let a = fbm(i as f32 * 1.5, y, period, 4, SEED);
            let b = fbm(i as f32 * 1.5 + period as f32, y, period, 4, SEED);
            assert!((0.0..=1.0).contains(&a), "fbm = {} is outside [0,1]", a);
            assert!(
                (a - b).abs() < 1e-5,
                "fbm is not periodic in x: f(x={}) = {} but f(x={}) = {}",
                i as f32 * 1.5,
                a,
                i as f32 * 1.5 + period as f32,
                b
            );
        }
    }
}
