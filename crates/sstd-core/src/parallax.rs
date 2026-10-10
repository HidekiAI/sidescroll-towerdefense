//! Engine-free parallax placement.
//!
//! One pure function, [`place_layer`], decides where a backdrop layer lands from the view, the
//! map's anchor and the layer's depth. Where a `Parallax2D` layer landed used to depend on
//! engine state SSTD did not own (the camera's canvas, its centred `half_viewport * rate` term,
//! `screen_offset`, `repeat_times`), so every attempt fixed one window size and broke the next.
//! Registration is defined at the anchor, around the view centre, so all layers line up at the
//! anchor for every window size.
//!
//! No Godot types appear here, and no file is read: the art facts a stack is validated against
//! are supplied by the caller, so this module stays a pure function of its inputs.
//!
//! Design: TDD_Parallax-Scrolling and its `.Math` / `.Data` / `.Verification` sub-pages.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{ValidationMessage, ValidationResult};

/// One screen of tiles: 60 x 33 tiles at 32 px (`TDD_Map-World` R1).
pub const SCREEN_W_PX: i32 = 1920;
/// One screen of tiles, tall.
pub const SCREEN_H_PX: i32 = 1056;
/// The view is one screen wide plus 24 px, so the row boundary below the map is visible.
pub const VIEW_W_PX: i32 = 1920;
/// The view height, including the 24 px below a screen.
pub const VIEW_H_PX: i32 = 1080;
/// Performance budget, `Data` section 4. Never seeded in config, so it lives here.
pub const MAX_LAYERS: usize = 7;
/// The legacy manifest's foreground threshold: `z_index` at or above this is the front plane.
const LEGACY_FRONT_Z_INDEX: i32 = 10;
/// The one string `parallax_depth` accepts besides a number.
const DEPTH_INFINITE: &str = "inf";
/// The one word `fill_above` / `fill_below` accept besides an RGBA colour.
const FILL_TRANSPARENT: &str = "transparent";

/// Which side of the tile layer a layer is drawn on. `Back` is drawn before the tiles, `Front`
/// after, so a front layer covers entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Plane {
    Back,
    Front,
}

/// Distance from the play plane. `Finite` drives the rate through `1 / (1 + depth)`;
/// `Infinite` is written as the string `"inf"` and pins the layer (the sun and moon).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Depth {
    Finite(f64),
    Infinite,
}

/// A layer's declared edge fill: one RGBA colour drawn from the art edge to the view edge, or
/// nothing. Written as `"transparent"` or `"#RRGGBBAA"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    Transparent,
    Rgba(u8, u8, u8, u8),
}

/// The outer view a stack's art was composed against, so `ref_screen_*` placements mean
/// something. `Data` section 1.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RefView {
    pub width: f64,
    pub height: f64,
}

/// One backdrop plane as stored in `stack.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub file: String,
    pub plane: Plane,
    /// Compositing order within the plane, ascending: drawn later is drawn on top.
    pub draw_order: i32,
    pub parallax_depth: Depth,
    pub vertical_damping: f64,
    /// The art repeats horizontally forever; the period is the PNG width.
    pub repeat_x: bool,
    pub ref_screen_left_px: f64,
    pub ref_screen_top_px: f64,
    pub fill_above: Fill,
    pub fill_below: Fill,
}

/// A stack: the layers once, independent of which world uses them. `Data` section 1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stack {
    pub schema_version: u32,
    pub stack_key: String,
    pub palette_key: String,
    pub generator_seed: u64,
    pub ref_view_px: RefView,
    pub layers: Vec<Layer>,
}

/// One layer of the retired `manifest.json` v1.0.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct LegacyLayer {
    pub name: String,
    pub file: String,
    pub scroll_scale_x: f64,
    pub scroll_scale_y: f64,
    pub repeat_size_x: u32,
    pub repeat_size_y: u32,
    pub z_index: i32,
    pub height_px: u32,
}

/// The retired `manifest.json` v1.0, the migration's input. `Data` section 5.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct LegacyManifest {
    pub version: String,
    pub width_px: u32,
    pub overscan_px: u32,
    pub seed: u64,
    pub layers: Vec<LegacyLayer>,
}

/// What measurement of a layer's PNG validation needs: its size, its encoding, and whether the
/// edge rows are fully opaque. The caller measures these (it already has a PNG decoder); this
/// module must not gain an image dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerArt {
    pub width_px: u32,
    pub height_px: u32,
    pub is_rgba: bool,
    /// Alpha 255 at every pixel of the PNG.
    pub fully_opaque: bool,
    /// Alpha 255 at every pixel of row 0.
    pub top_row_opaque: bool,
    /// Alpha 255 at every pixel of the last row.
    pub bottom_row_opaque: bool,
}

/// A layer with everything derived once: the rates, the repeat period, the art height and the
/// stack's reference view. Nothing here is recomputed per frame.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedLayer {
    pub name: String,
    pub plane: Plane,
    pub draw_order: i32,
    pub rate_x: f64,
    pub rate_y: f64,
    pub repeat_x: bool,
    /// The PNG width, the horizontal repeat period in art pixels.
    pub period_px: f64,
    pub art_h_px: f64,
    pub ref_screen_left_px: f64,
    pub ref_screen_top_px: f64,
    pub ref_view: RefView,
    pub fill_above: Fill,
    pub fill_below: Fill,
}

/// A validated stack, ready to place.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedStack {
    pub ref_view_px: RefView,
    pub layers: Vec<ResolvedLayer>,
}

/// The camera, as the placement function sees it. `zoom` is optical: it scales the projection
/// and never the rate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewState {
    pub cam_center_world_px_x: f64,
    pub cam_center_world_px_y: f64,
    pub zoom: f64,
    pub view_w_px: f64,
    pub view_h_px: f64,
}

/// The world point that every layer's camera coincides with, so all layers register there for
/// any window size. It is not the same as "screen centre": a screen is not square.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    pub world_px_x: f64,
    pub world_px_y: f64,
}

/// Where a layer is drawn this frame: the left of its first (left-most) art copy, how many
/// copies cover the view, the top of the art, and the zoom to draw it at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub first_copy_left_px: i32,
    pub copy_count: u32,
    pub screen_top_px: i32,
    pub scale: f64,
}

/// The bounding box of a world's occupied screens, in world px. `right` and `bottom` are
/// exclusive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapRect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Serialize for Depth {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Depth::Finite(depth) => serializer.serialize_f64(*depth),
            Depth::Infinite => serializer.serialize_str(DEPTH_INFINITE),
        }
    }
}

impl<'de> Deserialize<'de> for Depth {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // TODO(human) begin 1/18: Depth deserialization
        //
        // Intent: a depth is written either as a JSON number, or as the exact string "inf".
        // Anything else (null, "infinite", a bool, an object) is a load error, so the reader
        // must be strict and not fall back to a default. This is the only place the two wire
        // forms are turned into the enum; value rules (depth > -1, plane vs depth) belong to
        // validate, not here, so that one stack can be rejected with every rule listed.
        //
        // Why a visitor and not #[serde(untagged)]: untagged cannot match a unit variant to the
        // string "inf" - it only ever yields the payload form, and a failed untagged match
        // reports "data did not match any variant" with no mention of the offending value.
        //
        // Sample (uncomment, then adapt):
        //
        //     struct DepthVisitor;
        //
        //     impl<'de> Visitor<'de> for DepthVisitor {
        //         type Value = Depth;
        //
        //         fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        //             formatter.write_str("a number or the string \"inf\"")
        //         }
        //
        //         fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        //             Ok(Depth::Finite(value))
        //         }
        //
        //         fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        //             Ok(Depth::Finite(value as f64))
        //         }
        //
        //         fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        //             Ok(Depth::Finite(value as f64))
        //         }
        //
        //         fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        //             if value == DEPTH_INFINITE {
        //                 Ok(Depth::Infinite)
        //             } else {
        //                 Err(E::custom(format!("expected \"inf\", found {value:?}")))
        //             }
        //         }
        //     }
        //
        //     deserializer.deserialize_any(DepthVisitor)
        //
        // Test: depth_parses_a_number_and_infinity_only (red until this is written).
        // TODO(human) end 1/18
    }
}

impl Serialize for Fill {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Fill::Transparent => serializer.serialize_str(FILL_TRANSPARENT),
            Fill::Rgba(red, green, blue, alpha) => serializer.serialize_str(&format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                red, green, blue, alpha
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Fill {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // TODO(human) begin 2/18: Fill deserialization
        //
        // Intent: a fill is written as the exact string "transparent", or as one "#RRGGBBAA"
        // colour (hash, then eight hex digits, alpha last). Anything else is a load error.
        // The parse must be strict: "#8A98AC" (six digits) and "#8A98ACGG" (not hex) are
        // rejected, not defaulted, or a typo silently becomes a transparent band.
        //
        // Sample (uncomment, then adapt):
        //
        //     struct FillVisitor;
        //
        //     impl<'de> Visitor<'de> for FillVisitor {
        //         type Value = Fill;
        //
        //         fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        //             formatter.write_str("\"transparent\" or a #RRGGBBAA colour")
        //         }
        //
        //         fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        //             if value == FILL_TRANSPARENT {
        //                 return Ok(Fill::Transparent);
        //             }
        //             let digits = value.strip_prefix('#').ok_or_else(|| {
        //                 E::custom(format!("\"transparent\" or #RRGGBBAA, found {value:?}"))
        //             })?;
        //             if digits.len() != 8 {
        //                 return Err(E::custom(format!("#RRGGBBAA takes 8 digits, found {digits:?}")));
        //             }
        //             let channel = |start: usize| -> Result<u8, E> {
        //                 u8::from_str_radix(&digits[start..start + 2], 16)
        //                     .map_err(|_| E::custom(format!("not a hex colour: {value:?}")))
        //             };
        //             Ok(Fill::Rgba(channel(0)?, channel(2)?, channel(4)?, channel(6)?))
        //         }
        //     }
        //
        //     deserializer.deserialize_str(FillVisitor)
        //
        // Test: fill_parses_transparent_and_hex.
        // TODO(human) end 2/18
    }
}

impl Fill {
    /// The fill a PNG edge row implies: the per-channel mean of the row, rounded, when every
    /// pixel is opaque; `Transparent` otherwise. `Data` section 1, test U22.
    pub fn from_edge_row(row: &[u8], width_px: usize) -> Fill {
        // TODO(human) begin 3/18: Fill::from_edge_row
        //
        // Intent: the layout tool records a generated PNG's edge fill so the loader can trust
        // it. A fill is only legal if the row it extends is fully opaque (Data section 4,
        // rule fill_edge_opaque), so a row with any pixel below alpha 255 yields Transparent
        // rather than a colour that would hide a see-through edge.
        //
        // Rounding is half-up on the per-channel mean, which is what makes the value stable to
        // re-derive: the emitted art and the recorded fill agree.
        //
        // Sample (uncomment, then adapt):
        //
        //     if width_px == 0 || row.len() != width_px * 4 {
        //         return Fill::Transparent;
        //     }
        //     let mut red_total: u32 = 0;
        //     let mut green_total: u32 = 0;
        //     let mut blue_total: u32 = 0;
        //     for pixel in row.chunks_exact(4) {
        //         if pixel[3] != u8::MAX {
        //             return Fill::Transparent;
        //         }
        //         red_total += u32::from(pixel[0]);
        //         green_total += u32::from(pixel[1]);
        //         blue_total += u32::from(pixel[2]);
        //     }
        //     let count = width_px as u32;
        //     let mean = |total: u32| ((total + count / 2) / count) as u8;
        //     Fill::Rgba(mean(red_total), mean(green_total), mean(blue_total), u8::MAX)
        //
        // Test: u22_fill_is_the_edge_row_mean.
        // TODO(human) end 3/18
    }
}

impl Layer {
    /// The horizontal rate: how far the layer's camera moves for one pixel of real camera
    /// movement. `1 / (1 + depth)`, and `0` at infinity.
    pub fn rate_x(&self) -> f64 {
        // TODO(human) begin 4/18: Layer::rate_x
        //
        // Intent: the one formula the rest of the module is built on, from Math section 2:
        // a layer sees a camera that moved `rate` times as far, and `rate` falls off with
        // depth. Depth is signed: 0 is the tile plane (rate 1), positive is farther (slower),
        // negative is nearer (faster). A depth of "inf" is not "very far", it is the limit:
        // rate exactly 0, which is what pins the sun so it never moves on pan (U3, U16).
        //
        // Sample (uncomment, then adapt):
        //
        //     match self.parallax_depth {
        //         Depth::Finite(depth) => 1.0 / (1.0 + depth),
        //         Depth::Infinite => 0.0,
        //     }
        //
        // Tests: u2_shift_ratio_is_the_rate, u3_u21_..., u6_repeat_is_seamless_in_motion.
        // TODO(human) end 4/18
    }

    /// The vertical rate, damped so the backdrop does not move as far vertically as it does
    /// horizontally.
    pub fn rate_y(&self) -> f64 {
        // TODO(human) begin 5/18: Layer::rate_y
        //
        // Intent: vertical travel is deliberately a fraction of horizontal travel, otherwise a
        // tall backdrop band swings out of view on the first jump. The damping factor is the
        // layer's own `vertical_damping`, and the result still goes to 0 at infinity, so a
        // pinned layer is pinned on both axes.
        //
        // Sample (uncomment, then adapt):
        //
        //     self.rate_x() * self.vertical_damping
        //
        // Tests: u1 (top axis), u2 (y ratio), u4 (coverage across the camera range).
        // TODO(human) end 5/18
    }
}

/// The pixel snap: to the nearest whole pixel, ties away from the zero floor, so `-10.5` snaps
/// to `-10` (test U13).
pub fn snap(value: f64) -> i32 {
    // TODO(human) begin 6/18: pixel snap
    //
    // Intent: every layer's final left/top is rounded to a whole screen pixel so the art does
    // not shimmer as the camera creeps. `floor(value + 0.5)` is the rule Math section 2 states,
    // and its tie behaviour is the interesting part: with the +0.5-then-floor form, -10.5
    // snaps to -10, so a tie always moves toward +infinity rather than away from zero. That
    // is deliberate and asserted, not incidental.
    //
    // Do not snap anything but the final left/top: the repeat is computed on the unsnapped
    // value (see place_layer), or the copy count drifts.
    //
    // Sample (uncomment, then adapt):
    //
    //     (value + 0.5).floor() as i32
    //
    // Test: u13_pixel_snap_ties_away_from_the_floor.
    // TODO(human) end 6/18
}

/// The camera snap: round the camera centre to a multiple of `1 / zoom` world px, so the tile
/// layer and the backdrops land on one screen pixel grid. Left alone at `zoom < 1`, which is
/// editor-only (test U23).
pub fn snap_center(value: f64, zoom: f64) -> f64 {
    // TODO(human) begin 7/18: camera snap
    //
    // Intent: the tiles are drawn on the pixel grid, so if the camera centre is fractional the
    // tiles land off-grid and the backdrops land on a second, different grid - two grids, which
    // is the shimmer U23 exists to stop. Rounding the centre to a multiple of 1/zoom puts both
    // on one grid, because at zoom 2 the screen pixel is half a world pixel.
    //
    // Below zoom 1 the editor zooms out and there is no grid worth honouring, so the value is
    // passed through untouched. Reuse the pixel snap rather than writing a second rounding rule,
    // so the two can never disagree at a tie.
    //
    // Sample (uncomment, then adapt):
    //
    //     if zoom < 1.0 {
    //         return value;
    //     }
    //     snap(value * zoom) as f64 / zoom
    //
    // Test: u23_camera_snap_puts_one_grid_under_everything.
    // TODO(human) end 7/18
}

/// Where a layer is drawn this frame. The one function the whole design rests on.
pub fn place_layer(view: &ViewState, anchor: &Anchor, layer: &ResolvedLayer) -> Placement {
    // TODO(human) begin 8/18: place_layer
    //
    // Intent: the whole point of owning the math. Registration (U1) is defined at the anchor:
    // at the anchor every layer must land at the same screen place for any window size, which
    // is what a Parallax2D could not guarantee because where it landed depended on engine state
    // SSTD did not own. Read Math section 2 line by line; this is that sketch.
    //
    // Three lines carry the design:
    //   (1) layer_cam: a layer sees a camera that moved `rate` times as far from the anchor.
    //   (2) art_left_world / art_top_world: turn the author's reference-view placement into
    //       world px. The -ref_view/2 is what "centred on the anchor" means.
    //   (3) the projection: the ordinary camera projection, with the view centre at half the
    //       view size. For the tile layer (rate 1) this reduces to the normal Camera2D mapping.
    //
    // Then the repeat, x only. The shift by one whole period is not cosmetic:
    // `rem_euclid` alone lands in [0, period), and the caller draws from the returned left
    // rightward, so the first copy must start at or before 0. Taking one period off puts the
    // left in [-period, 0) and the ceil() then covers [0, view_w). Compute it on the UNSNAPPED
    // left, then snap only the values that are returned, or the copy count drifts by one at
    // some zooms (Math section 3's zoom 0.25 example has copy_count 5).
    //
    // Sample (uncomment, then adapt) - copied from the sketch in Math section 2:
    //
    //     let layer_cam_x = anchor.world_px_x
    //         + (view.cam_center_world_px_x - anchor.world_px_x) * layer.rate_x;
    //     let layer_cam_y = anchor.world_px_y
    //         + (view.cam_center_world_px_y - anchor.world_px_y) * layer.rate_y;
    //     let art_left_world =
    //         anchor.world_px_x + layer.ref_screen_left_px - layer.ref_view.width / 2.0;
    //     let art_top_world =
    //         anchor.world_px_y + layer.ref_screen_top_px - layer.ref_view.height / 2.0;
    //     let art_screen_left = (art_left_world - layer_cam_x) * view.zoom + view.view_w_px / 2.0;
    //     let art_screen_top = (art_top_world - layer_cam_y) * view.zoom + view.view_h_px / 2.0;
    //     let (first_copy_left, copy_count) = if layer.repeat_x {
    //         let period_screen = layer.period_px * view.zoom;
    //         let first_copy_left = art_screen_left.rem_euclid(period_screen) - period_screen;
    //         (
    //             first_copy_left,
    //             ((view.view_w_px - first_copy_left) / period_screen).ceil() as u32,
    //         )
    //     } else {
    //         (art_screen_left, 1)
    //     };
    //     Placement {
    //         first_copy_left_px: snap(first_copy_left),
    //         copy_count,
    //         screen_top_px: snap(art_screen_top),
    //         scale: view.zoom,
    //     }
    //
    // Tests: u1 (registration), u2 (ratio), u3/u21 (infinity), u4 (coverage),
    // u5 (copies cover the view), u6 (seamless), u16 (a moon), u23 (one grid).
    // TODO(human) end 8/18
}

/// The bounding box of the occupied screens, from their `(col, row)` cells. Columns may be
/// negative; the rectangle is exclusive on the right and bottom.
pub fn map_rectangle(screens: &[(i32, i32)]) -> Option<MapRect> {
    // TODO(human) begin 9/18: map_rectangle
    //
    // Intent: the map's extent comes from which screens exist, not from a hand-typed size
    // (`Data` section 3). A screen at cell (col, row) occupies world px [col*1920, (col+1)*1920)
    // x [row*1056, (row+1)*1056), so the rectangle is just the bounding box. Columns can be
    // negative: a world can extend west of the origin, and (col+1) is what keeps the right edge
    // exclusive rather than one screen short.
    //
    // A world with no screens has no rectangle; return None and let the caller decide, rather
    // than inventing a one-screen world here.
    //
    // Note: use min()/max() over the iterator. A running min/max needs `let mut`, and this
    // module has no other mutable state to justify it.
    //
    // Sample (uncomment, then adapt):
    //
    //     let min_col = screens.iter().map(|screen| screen.0).min()?;
    //     let max_col = screens.iter().map(|screen| screen.0).max()?;
    //     let min_row = screens.iter().map(|screen| screen.1).min()?;
    //     let max_row = screens.iter().map(|screen| screen.1).max()?;
    //     Some(MapRect {
    //         left: f64::from(min_col) * f64::from(SCREEN_W_PX),
    //         top: f64::from(min_row) * f64::from(SCREEN_H_PX),
    //         right: f64::from(max_col + 1) * f64::from(SCREEN_W_PX),
    //         bottom: f64::from(max_row + 1) * f64::from(SCREEN_H_PX),
    //     })
    //
    // Tests: u10_map_rectangle_is_the_screen_bounding_box.
    // TODO(human) end 9/18
}

/// The camera's bound: the map in the game, the map grown by one screen on every side in the
/// editor, so the map's edges are visible with the margin cells around them.
pub fn camera_bound(map: &MapRect, editor: bool) -> MapRect {
    // TODO(human) begin 10/18: camera_bound
    //
    // Intent: the game camera is pinned to the map, so the player can never see past the first
    // or last screen row. The editor is different on purpose: it grows the bound by one screen
    // on every side so the margin cells around the map can be authored, which is the whole
    // reason an editor has a camera bound at all. The margin is one *screen* (1920 x 1056), not
    // one view, so it does not move with the zoom.
    //
    // Sample (uncomment, then adapt):
    //
    //     if !editor {
    //         return *map;
    //     }
    //     MapRect {
    //         left: map.left - f64::from(SCREEN_W_PX),
    //         top: map.top - f64::from(SCREEN_H_PX),
    //         right: map.right + f64::from(SCREEN_W_PX),
    //         bottom: map.bottom + f64::from(SCREEN_H_PX),
    //     }
    //
    // Tests: u11_editor_bound_grows_by_one_screen, u4 (uses the editor bound).
    // TODO(human) end 10/18
}

/// Clamp a centre to a bound. When the bound is smaller than the view on this axis, the centre
/// is fixed so the view's top-left equals the bound's top-left.
pub fn clamp_center(center: f64, view_extent: f64, bound_low: f64, bound_high: f64) -> f64 {
    // TODO(human) begin 11/18: clamp_center
    //
    // Intent: two cases, and the second is the one that gets written wrong.
    //
    // Normal case: the camera centre may range so that the view stays inside the bound, so the
    // centre's own limits are the bound pulled in by half a view.
    //
    // Small-bound case: when the bound is narrower than the view (a one-row map seen from a
    // tall view, or the editor at zoom 0.25 where the view is 4320 px against a 4224 px bound),
    // there is no such range - `clamp` would be called with low > high and is undefined. Line
    // the view up with the bound instead: the centre sits half a view in from the low edge.
    // Note this is NOT the bound's midpoint; Math section 5's zoom-0.25 example fixes the
    // centre at 1104, which is low + half, not (low + high) / 2 = 1056.
    //
    // `view_extent` arrives already scaled by the zoom (the caller passes view/zoom), so the
    // fractional half-view at zoom 2 needs no special case here.
    //
    // Sample (uncomment, then adapt):
    //
    //     let half_extent = view_extent / 2.0;
    //     if bound_high - bound_low < view_extent {
    //         return bound_low + half_extent;
    //     }
    //     center.clamp(bound_low + half_extent, bound_high - half_extent)
    //
    // Tests: u9 (one-row map pins the centre), u20 (game camera clamped).
    // TODO(human) end 11/18
}

/// The anchor a world gets when it declares none: the camera centre at the home view, which is
/// screen column 0 (or the left-most column), bottom row, clamped.
pub fn default_anchor(map: &MapRect, view: &ViewState) -> Anchor {
    // TODO(human) begin 12/18: default_anchor
    //
    // Intent: a world that declares no anchor gets a sensible one, so a fresh world already
    // looks right (Data section 3). The "home view" is the map's left-most screen column and
    // its bottom row - you start at the bottom of the map on the left - so the centre is half a
    // view in from the left edge and half a view up from the bottom edge.
    //
    // Then clamp it: the answe must be a camera position the game would allow, not merely a
    // point derived from the bound, so a one-row map gets the same centre the clamps produce.
    // Reuse clamp_anchor rather than re-deriving limits, and keep the first element of its pair.
    //
    // Sample (uncomment, then adapt):
    //
    //     let home = Anchor {
    //         world_px_x: map.left + view.view_w_px / 2.0,
    //         world_px_y: map.bottom - view.view_h_px / 2.0,
    //     };
    //     clamp_anchor(home, map, view).0
    //
    // Test: u14_default_anchor_is_the_home_view_center.
    // TODO(human) end 12/18
}

/// Clamp an authored anchor into the game camera bound, returning whether it moved.
pub fn clamp_anchor(anchor: Anchor, map: &MapRect, view: &ViewState) -> (Anchor, bool) {
    // TODO(human) begin 13/18: clamp_anchor
    //
    // Intent: an authored anchor is a request, not a guarantee - a world carries an anchor the
    // editor never validated against the map it later became. Clamp it into the *game* bound
    // (not the editor's), and report whether it moved, because the loader logs one
    // `parallax.anchor CLAMP` line per move (Data line 156/U19) and a silently-moved anchor is
    // exactly the class of bug this design exists to remove.
    //
    // Each axis clamps with its own extent, and the extent is view/zoom because the world-space
    // view is bigger when zoomed out. Compare the clamped pair against the input to answer
    // "moved", rather than tracking flags.
    //
    // Sample (uncomment, then adapt):
    //
    //     let clamped = Anchor {
    //         world_px_x: clamp_center(
    //             anchor.world_px_x,
    //             view.view_w_px / view.zoom,
    //             map.left,
    //             map.right,
    //         ),
    //         world_px_y: clamp_center(
    //             anchor.world_px_y,
    //             view.view_h_px / view.zoom,
    //             map.top,
    //             map.bottom,
    //         ),
    //     };
    //     let moved = clamped != anchor;
    //     (clamped, moved)
    //
    // Tests: u19_anchor_override_is_clamped, u14 (via default_anchor).
    // TODO(human) end 13/18
}

/// One validation failure as the loader's journal line, `Data` line 146 (test U7).
pub fn journal_line(msg: &ValidationMessage, stack_key: &str, layer: &str) -> String {
    // TODO(human) begin 14/18: journal_line
    //
    // Intent: a rejected stack must be diagnosable from the journal alone, without a debugger:
    // which stack, which layer, which field, what value, which rule. Data line 146 fixes the
    // shape, so this is formatting, not design - but the field order and the "-" for an absent
    // field/value are part of the contract a log reader greps for.
    //
    // A message without field/value renders "-", never an empty gap, so the line always has
    // five key=value pairs and is machine-readable.
    //
    // Sample (uncomment, then adapt):
    //
    //     format!(
    //         "parallax.validate FAIL stack={} layer={} field={} value={} rule={}",
    //         stack_key,
    //         layer,
    //         msg.field.as_deref().unwrap_or("-"),
    //         msg.value.as_deref().unwrap_or("-"),
    //         msg.code,
    //     )
    //
    // Test: u7_each_validation_rule_fires (asserts the prefix and the rule= field).
    // TODO(human) end 14/18
}

/// One anchor clamp as the loader's journal line, `Data` line 156 (test U19).
pub fn anchor_clamp_line(stack_key: &str, from: &Anchor, to: &Anchor) -> String {
    // TODO(human) begin 15/18: anchor_clamp_line
    //
    // Intent: the same journalling rule as journal_line, for the one *warning* this module
    // produces: an anchor that had to be moved. Record where it was asked to be and where it
    // ended up, so the journal shows both the author's intent and the clamped result - a line
    // that only printed the result would hide the bug it exists to reveal.
    //
    // Render the coordinates as whole numbers: they are world px, and a fractional anchor is
    // the same defect the clamp is fixing.
    //
    // Sample (uncomment, then adapt):
    //
    //     format!(
    //         "parallax.anchor CLAMP stack={} from={},{} to={},{}",
    //         stack_key,
    //         from.world_px_x as i64,
    //         from.world_px_y as i64,
    //         to.world_px_x as i64,
    //         to.world_px_y as i64,
    //     )
    //
    // Test: u19_anchor_override_is_clamped (asserts the exact line).
    // TODO(human) end 15/18
}

impl Stack {
    /// Every rule of `Data` section 4. One message per failure, carrying the field and the
    /// value, so the loader can log one journal line each.
    pub fn validate(&self, art: &[LayerArt]) -> ValidationResult {
        // TODO(human) begin 16/18: Stack::validate
        //
        // Intent: every rule in Data section 4, and *all* of them in one pass. Do not return on
        // the first failure - a stack with three problems must be loggable in one go, which is
        // the merge-never-early-return idiom map.rs uses (`let mut result =
        // ValidationResult::valid(); result = result.with_message(...)`).
        //
        // The rules, in this module's rule codes:
        //   schema_version   self.schema_version == 2
        //   max_layers       self.layers.len() <= MAX_LAYERS
        //   art_count        art.len() == self.layers.len(). Report it and skip the per-layer
        //                    art checks; there is no art to check them against. Everything that
        //                    does not need art still runs.
        //   file_rgba        per layer, art.is_rgba
        //   fill_edge_opaque per layer, a non-transparent fill_above needs art.top_row_opaque,
        //                    non-transparent fill_below needs art.bottom_row_opaque
        //   parallax_depth   Depth::Finite(depth) needs depth > -1 (the string "inf" is fine;
        //                    a depth of -1 is a layer at the camera)
        //   vertical_damping in [0, 1] - it damps, it is not a rate
        //   plane_depth      Back needs depth >= 0; Front needs -0.5 <= depth < 0. The front
        //                    ceiling exists because the rate runs away as depth nears -1
        //                    (-0.99 gives rate 100)
        //   draw_order_unique within a plane, draw_order is unique
        //   draw_order_depth  within a plane, ascending draw_order has non-increasing depth,
        //                    with "inf" counting as the largest depth; EXEMPT the base layer,
        //                    which is the lowest draw_order of the back plane (it is the opaque
        //                    canvas under everything, so a sun at "inf" may sit above it)
        //   base_opaque      the base layer has art.fully_opaque AND both fills non-transparent
        //
        // Every message carries .with_field(...) and .with_value(...) so journal_line can render
        // a complete line; put the layer's name in the message for the per-layer rules.
        // Two rules from Data section 4 are deliberately not here: "file exists" (this module
        // has no filename and no filesystem) and the backdrop.json anchor warning (that is the
        // loader's, in step 6).
        //
        // Sample (uncomment, then adapt) - the shape, with two rules written out:
        //
        //     let mut result = ValidationResult::valid();
        //     if self.schema_version != 2 {
        //         result = result.with_message(
        //             ValidationMessage::error("schema_version", "stack schema_version must be 2")
        //                 .with_field("schema_version")
        //                 .with_value(self.schema_version.to_string()),
        //         );
        //     }
        //     if self.layers.len() > MAX_LAYERS {
        //         result = result.with_message(
        //             ValidationMessage::error("max_layers", "too many layers")
        //                 .with_field("layers")
        //                 .with_value(self.layers.len().to_string()),
        //         );
        //     }
        //     let art_matches = art.len() == self.layers.len();
        //     if !art_matches {
        //         result = result.with_message(
        //             ValidationMessage::error("art_count", "art facts do not match the layers")
        //                 .with_field("art")
        //                 .with_value(format!("{} for {} layers", art.len(), self.layers.len())),
        //         );
        //     }
        //     for (layer_index, layer) in self.layers.iter().enumerate() {
        //         if let Some(facts) = art.get(layer_index) {
        //             if !facts.is_rgba {
        //                 result = result.with_message(
        //                     ValidationMessage::error(
        //                         "file_rgba",
        //                         format!("{} must be an RGBA PNG", layer.name),
        //                     )
        //                     .with_field("file")
        //                     .with_value(layer.file.clone()),
        //                 );
        //             }
        //             if layer.fill_above != Fill::Transparent && !facts.top_row_opaque {
        //                 result = result.with_message(
        //                     ValidationMessage::error(
        //                         "fill_edge_opaque",
        //                         format!("{} fill_above needs an opaque top row", layer.name),
        //                     )
        //                     .with_field("fill_above"),
        //                 );
        //             }
        //             if layer.fill_below != Fill::Transparent && !facts.bottom_row_opaque {
        //                 result = result.with_message(
        //                     ValidationMessage::error(
        //                         "fill_edge_opaque",
        //                         format!("{} fill_below needs an opaque bottom row", layer.name),
        //                     )
        //                     .with_field("fill_below"),
        //                 );
        //             }
        //         }
        //         // ... and the rules that need no art, in the same loop ...
        //     }
        //     result
        //
        // Tests: u7_each_validation_rule_fires (one fixture per rule), u15 (front ceiling),
        // u17 (opaque edge row), u8 (the migrated stack is not validated here).
        // TODO(human) end 16/18
    }

    /// Validate, then derive. `Err` carries every failure rather than the first, so one load
    /// reports the whole stack.
    pub fn resolve(&self, art: &[LayerArt]) -> Result<ResolvedStack, ValidationResult> {
        // TODO(human) begin 17/18: Stack::resolve
        //
        // Intent: validate first, and if anything is wrong hand back the whole result so the
        // caller logs every failure at once - never a partial set of layers. `Err` carries the
        // ValidationResult itself, not a string, so the loader still has the codes and fields
        // for its journal lines.
        //
        // On success, derive once what place_layer would otherwise recompute every frame: the
        // two rates, the repeat period and the art height. The period is the PNG *width* - not
        // repeat_size_x, which the new schema drops - so it comes from the art facts. The
        // layer's own ref_view is copied into each resolved layer so place_layer needs no
        // stack argument.
        //
        // Sample (uncomment, then adapt):
        //
        //     let result = self.validate(art);
        //     if result.has_errors() {
        //         return Err(result);
        //     }
        //     let layers = self
        //         .layers
        //         .iter()
        //         .zip(art.iter())
        //         .map(|(layer, facts)| ResolvedLayer {
        //             name: layer.name.clone(),
        //             plane: layer.plane,
        //             draw_order: layer.draw_order,
        //             rate_x: layer.rate_x(),
        //             rate_y: layer.rate_y(),
        //             repeat_x: layer.repeat_x,
        //             period_px: f64::from(facts.width_px),
        //             art_h_px: f64::from(facts.height_px),
        //             ref_screen_left_px: layer.ref_screen_left_px,
        //             ref_screen_top_px: layer.ref_screen_top_px,
        //             ref_view: self.ref_view_px,
        //             fill_above: layer.fill_above,
        //             fill_below: layer.fill_below,
        //         })
        //         .collect();
        //     Ok(ResolvedStack {
        //         ref_view_px: self.ref_view_px,
        //         layers,
        //     })
        //
        // Tests: every placement test goes through this (u1..u6), u7/u15/u17 (rejection).
        // TODO(human) end 17/18
    }

    /// Migrate the retired `manifest.json` v1.0. `Data` section 5; test U8.
    pub fn from_legacy_manifest(
        manifest: &LegacyManifest,
        stack_key: &str,
        palette_key: &str,
    ) -> Stack {
        // TODO(human) begin 18/18: Stack::from_legacy_manifest
        //
        // Intent: Data section 5's table, and only the rows the legacy file actually contains.
        // The legacy file stores rates, not depths, so invert the relationship
        // (Data section 1): depth = 1 / rate_x - 1 and damping = rate_y / rate_x. It stores no
        // placements and no fills - those lived in the preview scene and in the PNGs - so the
        // neutral values go in here and gen-backdrop supplies the derived ones. That is why U8
        // asserts the twelve rates and nothing about placement.
        //
        // Watch three things:
        //   - the plane split. z_index >= LEGACY_FRONT_Z_INDEX is the front plane, and the
        //     front plane's draw_order restarts at 0 (the legacy 10 is not an order).
        //   - repeat_x is `repeat_size_x > 0`; the period is not stored here, it becomes the
        //     PNG width when the art is measured.
        //   - a legacy rate_x of 0 would be a division by zero; "inf" is the honest reading.
        //     Guard both the depth and the damping.
        //
        // ref_view comes from the legacy width and VIEW_H_PX (1080): the legacy sky is 1320 px
        // tall, which is 1080 plus the 240 px overscan, so 1080 is the view the art was made
        // against.
        //
        // Sample (uncomment, then adapt):
        //
        //     let layers = manifest
        //         .layers
        //         .iter()
        //         .map(|legacy| {
        //             let rate_x = legacy.scroll_scale_x;
        //             let parallax_depth = if rate_x == 0.0 {
        //                 Depth::Infinite
        //             } else {
        //                 Depth::Finite(1.0 / rate_x - 1.0)
        //             };
        //             let vertical_damping = if rate_x == 0.0 {
        //                 1.0
        //             } else {
        //                 legacy.scroll_scale_y / rate_x
        //             };
        //             let (plane, draw_order) = if legacy.z_index >= LEGACY_FRONT_Z_INDEX {
        //                 (Plane::Front, legacy.z_index - LEGACY_FRONT_Z_INDEX)
        //             } else {
        //                 (Plane::Back, legacy.z_index)
        //             };
        //             Layer {
        //                 name: legacy.name.clone(),
        //                 file: legacy.file.clone(),
        //                 plane,
        //                 draw_order,
        //                 parallax_depth,
        //                 vertical_damping,
        //                 repeat_x: legacy.repeat_size_x > 0,
        //                 ref_screen_left_px: 0.0,
        //                 ref_screen_top_px: 0.0,
        //                 fill_above: Fill::Transparent,
        //                 fill_below: Fill::Transparent,
        //             }
        //         })
        //         .collect();
        //     Stack {
        //         schema_version: 2,
        //         stack_key: stack_key.to_string(),
        //         palette_key: palette_key.to_string(),
        //         generator_seed: manifest.seed,
        //         ref_view_px: RefView {
        //             width: f64::from(manifest.width_px),
        //             height: f64::from(VIEW_H_PX),
        //         },
        //         layers,
        //     }
        //
        // Tests: u8_migration_reproduces_twelve_rates (reads the tracked manifest.json).
        // TODO(human) end 18/18
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SKY_RGBA: Fill = Fill::Rgba(0x8A, 0x98, 0xAC, 0xFF);
    const SKY_BELOW_RGBA: Fill = Fill::Rgba(0xC4, 0xCC, 0xD2, 0xFF);
    const HILLS_BELOW_RGBA: Fill = Fill::Rgba(0x3C, 0x46, 0x50, 0xFF);
    const FOREST_BELOW_RGBA: Fill = Fill::Rgba(0x2D, 0x32, 0x38, 0xFF);

    fn fixture_layers() -> Vec<Layer> {
        vec![
            Layer {
                name: "Sky".to_string(),
                file: "sky.png".to_string(),
                plane: Plane::Back,
                draw_order: 0,
                parallax_depth: Depth::Finite(9.0),
                vertical_damping: 0.8,
                repeat_x: true,
                ref_screen_left_px: 0.0,
                ref_screen_top_px: 0.0,
                fill_above: SKY_RGBA,
                fill_below: SKY_BELOW_RGBA,
            },
            Layer {
                name: "Sun".to_string(),
                file: "sun.png".to_string(),
                plane: Plane::Back,
                draw_order: 1,
                parallax_depth: Depth::Infinite,
                vertical_damping: 1.0,
                repeat_x: false,
                ref_screen_left_px: 1241.0,
                ref_screen_top_px: 194.0,
                fill_above: Fill::Transparent,
                fill_below: Fill::Transparent,
            },
            Layer {
                name: "HighClouds".to_string(),
                file: "clouds_high.png".to_string(),
                plane: Plane::Back,
                draw_order: 2,
                parallax_depth: Depth::Finite(4.0),
                vertical_damping: 0.85,
                repeat_x: true,
                ref_screen_left_px: 0.0,
                ref_screen_top_px: 760.0,
                fill_above: Fill::Transparent,
                fill_below: Fill::Transparent,
            },
            Layer {
                name: "LowClouds".to_string(),
                file: "clouds_low.png".to_string(),
                plane: Plane::Back,
                draw_order: 3,
                parallax_depth: Depth::Finite(2.3333),
                vertical_damping: 0.8333,
                repeat_x: true,
                ref_screen_left_px: 0.0,
                ref_screen_top_px: 760.0,
                fill_above: Fill::Transparent,
                fill_below: Fill::Transparent,
            },
            Layer {
                name: "Hills".to_string(),
                file: "hills.png".to_string(),
                plane: Plane::Back,
                draw_order: 4,
                parallax_depth: Depth::Finite(1.0),
                vertical_damping: 0.84,
                repeat_x: true,
                ref_screen_left_px: 0.0,
                ref_screen_top_px: 696.0,
                fill_above: Fill::Transparent,
                fill_below: HILLS_BELOW_RGBA,
            },
            Layer {
                name: "Forest".to_string(),
                file: "forest.png".to_string(),
                plane: Plane::Back,
                draw_order: 5,
                parallax_depth: Depth::Finite(0.4286),
                vertical_damping: 0.8571,
                repeat_x: true,
                ref_screen_left_px: 0.0,
                ref_screen_top_px: 760.0,
                fill_above: Fill::Transparent,
                fill_below: FOREST_BELOW_RGBA,
            },
            Layer {
                name: "Foreground".to_string(),
                file: "foreground.png".to_string(),
                plane: Plane::Front,
                draw_order: 0,
                parallax_depth: Depth::Finite(-0.2308),
                vertical_damping: 0.8846,
                repeat_x: true,
                ref_screen_left_px: 0.0,
                ref_screen_top_px: 760.0,
                fill_above: Fill::Transparent,
                fill_below: Fill::Transparent,
            },
        ]
    }

    /// Art facts for [`fixture_layers`], in the same order.
    fn fixture_art() -> Vec<LayerArt> {
        vec![
            LayerArt {
                width_px: 1920,
                height_px: 1080,
                is_rgba: true,
                fully_opaque: true,
                top_row_opaque: true,
                bottom_row_opaque: true,
            },
            LayerArt {
                width_px: 130,
                height_px: 130,
                is_rgba: true,
                fully_opaque: false,
                top_row_opaque: false,
                bottom_row_opaque: false,
            },
            LayerArt {
                width_px: 1920,
                height_px: 320,
                is_rgba: true,
                fully_opaque: false,
                top_row_opaque: false,
                bottom_row_opaque: false,
            },
            LayerArt {
                width_px: 1920,
                height_px: 320,
                is_rgba: true,
                fully_opaque: false,
                top_row_opaque: false,
                bottom_row_opaque: false,
            },
            LayerArt {
                width_px: 1920,
                height_px: 900,
                is_rgba: true,
                fully_opaque: false,
                top_row_opaque: false,
                bottom_row_opaque: true,
            },
            LayerArt {
                width_px: 1920,
                height_px: 900,
                is_rgba: true,
                fully_opaque: false,
                top_row_opaque: false,
                bottom_row_opaque: true,
            },
            LayerArt {
                width_px: 1920,
                height_px: 900,
                is_rgba: true,
                fully_opaque: false,
                top_row_opaque: false,
                bottom_row_opaque: false,
            },
        ]
    }

    fn fixture_stack() -> Stack {
        Stack {
            schema_version: 2,
            stack_key: "neutral_default".to_string(),
            palette_key: "neutral".to_string(),
            generator_seed: 1592598566,
            ref_view_px: RefView {
                width: 1920.0,
                height: 1080.0,
            },
            layers: fixture_layers(),
        }
    }

    fn resolved() -> ResolvedStack {
        let stack = fixture_stack()
            .resolve(&fixture_art())
            .expect("the fixture stack must validate");
        // Tripwire: a test that iterates the layers must fail, not pass, when resolve hands
        // back nothing. Guards U1, U2 and U5 against passing vacuously over an empty list.
        assert_eq!(
            stack.layers.len(),
            MAX_LAYERS,
            "the fixture must resolve all seven layers"
        );
        stack
    }

    fn resolved_layer<'a>(stack: &'a ResolvedStack, name: &str) -> &'a ResolvedLayer {
        stack
            .layers
            .iter()
            .find(|layer| layer.name == name)
            .unwrap_or_else(|| panic!("no layer named {}", name))
    }

    fn view(cam_center_world_px_x: f64, cam_center_world_px_y: f64, zoom: f64) -> ViewState {
        ViewState {
            cam_center_world_px_x,
            cam_center_world_px_y,
            zoom,
            view_w_px: VIEW_W_PX as f64,
            view_h_px: VIEW_H_PX as f64,
        }
    }

    fn home_anchor() -> Anchor {
        Anchor {
            world_px_x: 960.0,
            world_px_y: 1572.0,
        }
    }

    fn validation_codes(result: &ValidationResult) -> Vec<String> {
        result
            .errors()
            .map(|message| message.code.clone())
            .collect()
    }

    fn has_code(result: &ValidationResult, code: &str) -> bool {
        validation_codes(result).iter().any(|found| found == code)
    }

    /// Wrap a signed distance into `(-period/2, period/2]`, so a placement compared across
    /// frames can be compared modulo the repeat period. A repeating layer's left moves by whole
    /// periods when the camera moves, which is invisible on screen but not to `assert_eq!`.
    fn wrap_signed(value: f64, period_px: f64) -> f64 {
        let wrapped = value.rem_euclid(period_px);
        if wrapped > period_px / 2.0 {
            wrapped - period_px
        } else {
            wrapped
        }
    }

    #[test]
    fn depth_parses_a_number_and_infinity_only() {
        assert_eq!(
            serde_json::from_str::<Depth>("9").expect("a number parses"),
            Depth::Finite(9.0)
        );
        assert_eq!(
            serde_json::from_str::<Depth>("\"inf\"").expect("\"inf\" parses"),
            Depth::Infinite
        );
        assert!(serde_json::from_str::<Depth>("null").is_err());
        assert!(serde_json::from_str::<Depth>("\"infinite\"").is_err());
    }

    #[test]
    fn depth_serializes_back_to_its_wire_form() {
        assert_eq!(
            serde_json::to_string(&Depth::Finite(2.5)).expect("serializes"),
            "2.5"
        );
        assert_eq!(
            serde_json::to_string(&Depth::Infinite).expect("serializes"),
            "\"inf\""
        );
    }

    #[test]
    fn fill_parses_transparent_and_hex() {
        assert_eq!(
            serde_json::from_str::<Fill>("\"transparent\"").expect("transparent parses"),
            Fill::Transparent
        );
        assert_eq!(
            serde_json::from_str::<Fill>("\"#8A98ACFF\"").expect("hex parses"),
            Fill::Rgba(0x8A, 0x98, 0xAC, 0xFF)
        );
        assert!(serde_json::from_str::<Fill>("\"#8A98AC\"").is_err());
        assert!(serde_json::from_str::<Fill>("\"#8A98ACGG\"").is_err());
        assert!(serde_json::from_str::<Fill>("255").is_err());
    }

    #[test]
    fn fill_serializes_back_to_its_wire_form() {
        assert_eq!(
            serde_json::to_string(&Fill::Transparent).expect("serializes"),
            "\"transparent\""
        );
        assert_eq!(
            serde_json::to_string(&Fill::Rgba(0x8A, 0x98, 0xAC, 0xFF)).expect("serializes"),
            "\"#8A98ACFF\""
        );
    }

    /// U22: the fill a row implies is that row's per-channel mean when the row is fully opaque,
    /// and `Transparent` when it is not.
    #[test]
    fn u22_fill_is_the_edge_row_mean() {
        let opaque_row = [138u8, 152, 172, 255, 138, 152, 172, 255, 138, 152, 172, 255];
        assert_eq!(
            Fill::from_edge_row(&opaque_row, 3),
            Fill::Rgba(138, 152, 172, 255)
        );

        let mean_row = [100u8, 200, 0, 255, 102, 202, 0, 255];
        assert_eq!(
            Fill::from_edge_row(&mean_row, 2),
            Fill::Rgba(101, 201, 0, 255)
        );

        let translucent_row = [138u8, 152, 172, 254, 138, 152, 172, 255];
        assert_eq!(
            Fill::from_edge_row(&translucent_row, 2),
            Fill::Transparent,
            "a row with one non-opaque pixel cannot be extended into a fill"
        );
    }

    /// U1: at the anchor, every layer lands at the same place for any window size, on both
    /// axes. A repeating layer's left is compared modulo its period, since whole periods of
    /// shift are invisible on screen.
    #[test]
    fn u1_registration_holds_for_any_window() {
        let stack = resolved();
        for (view_w_px, view_h_px) in [(800.0, 800.0), (1920.0, 1080.0), (2560.0, 1440.0)] {
            let mut state = view(960.0, 1572.0, 1.0);
            state.view_w_px = view_w_px;
            state.view_h_px = view_h_px;
            for layer in &stack.layers {
                let placement = place_layer(&state, &home_anchor(), layer);
                let expected_left =
                    (layer.ref_screen_left_px - 960.0) * state.zoom + state.view_w_px / 2.0;
                let expected_top =
                    (layer.ref_screen_top_px - 540.0) * state.zoom + state.view_h_px / 2.0;
                assert_eq!(
                    placement.screen_top_px,
                    snap(expected_top),
                    "{} top at view {}x{}",
                    layer.name,
                    view_w_px,
                    view_h_px
                );
                let left_offset = placement.first_copy_left_px as f64 - expected_left;
                if layer.repeat_x {
                    assert_eq!(
                        wrap_signed(left_offset, layer.period_px),
                        0.0,
                        "{} left offset from registration at view {}x{}",
                        layer.name,
                        view_w_px,
                        view_h_px
                    );
                } else {
                    assert_eq!(
                        placement.first_copy_left_px,
                        snap(expected_left),
                        "{} left at view {}x{}",
                        layer.name,
                        view_w_px,
                        view_h_px
                    );
                }
            }
        }
    }

    /// U2: a camera moved by `delta` moves each layer by `-delta * rate`, so the ratio of a
    /// layer's shift to the tile shift is its rate.
    #[test]
    fn u2_shift_ratio_is_the_rate() {
        let stack = resolved();
        let before = view(960.0, 1572.0, 1.0);
        let after = view(1960.0, 1572.0, 1.0);
        for layer in &stack.layers {
            let start = place_layer(&before, &home_anchor(), layer);
            let end = place_layer(&after, &home_anchor(), layer);
            let expected = -1000.0 * layer.rate_x;
            let actual = (end.first_copy_left_px - start.first_copy_left_px) as f64;
            if layer.repeat_x {
                assert_eq!(
                    wrap_signed(actual, layer.period_px),
                    wrap_signed(expected, layer.period_px),
                    "{} shift at rate {}",
                    layer.name,
                    layer.rate_x
                );
            } else {
                assert!(
                    (actual - expected).abs() <= 1.0,
                    "{} shift {} should be {}",
                    layer.name,
                    actual,
                    expected
                );
            }
        }
    }

    /// U3 and U21: a layer at infinity is fixed on pan at every zoom, and its screen position
    /// scales about the view centre on zoom.
    #[test]
    fn u3_u21_infinite_layer_is_fixed_on_pan_and_scales_on_zoom() {
        let stack = resolved();
        let sun = resolved_layer(&stack, "Sun");
        for zoom in [2.0, 1.0, 0.5, 0.25] {
            let reference = place_layer(&view(960.0, 1572.0, zoom), &home_anchor(), sun);
            for step in 0..50 {
                let cam_center_world_px_x = 960.0 + (step as f64) * 137.0;
                let cam_center_world_px_y = 540.0 + (step as f64) * 61.0;
                let placement = place_layer(
                    &view(cam_center_world_px_x, cam_center_world_px_y, zoom),
                    &home_anchor(),
                    sun,
                );
                assert_eq!(placement, reference, "Sun moved at zoom {}", zoom);
            }
            let expected_left = (1241.0 - 960.0) * zoom + 960.0;
            assert_eq!(reference.first_copy_left_px, snap(expected_left));
            assert_eq!(reference.scale, zoom);
        }
        assert_eq!(
            place_layer(&view(960.0, 1572.0, 0.5), &home_anchor(), sun).first_copy_left_px,
            1101,
            "Math section 6 worked example"
        );
    }

    /// U16: the same holds for a stack that names its celestial layer `Moon`; the name is not
    /// special-cased anywhere.
    #[test]
    fn u16_a_moon_layer_behaves_like_the_sun() {
        let mut layers = fixture_layers();
        for layer in layers.iter_mut() {
            if layer.name == "Sun" {
                layer.name = "Moon".to_string();
                layer.file = "moon.png".to_string();
            }
        }
        let mut stack = fixture_stack();
        stack.layers = layers;
        let solved = stack
            .resolve(&fixture_art())
            .expect("the moon stack validates");
        let moon = resolved_layer(&solved, "Moon");
        let reference = place_layer(&view(960.0, 1572.0, 1.0), &home_anchor(), moon);
        for step in 0..50 {
            let placement = place_layer(
                &view(960.0 + (step as f64) * 137.0, 1572.0, 1.0),
                &home_anchor(),
                moon,
            );
            assert_eq!(placement, reference);
        }
        assert_eq!(reference.first_copy_left_px, 1241);
    }

    /// U4: for the whole camera range of a two-row map, the sky art plus its fills cover every
    /// screen row, at every zoom the editor uses. Calls `place_layer` directly: the mutated
    /// fixture would be rejected by `validate` before the test could run.
    #[test]
    fn u4_vertical_coverage_holds_at_every_camera_row() {
        let stack = resolved();
        let sky = resolved_layer(&stack, "Sky");
        let map = MapRect {
            left: 0.0,
            top: 0.0,
            right: 1920.0,
            bottom: 2112.0,
        };
        for (label, zoom, cam_low, cam_high) in [
            ("game zoom 1", 1.0, 540.0, 1572.0),
            ("editor zoom 2", 2.0, -786.0, 2898.0),
            ("editor zoom 1", 1.0, -516.0, 2628.0),
            ("editor zoom 0.5", 0.5, 24.0, 2088.0),
            ("editor zoom 0.25", 0.25, 1104.0, 1104.0),
        ] {
            let mut cam_center_world_px_y = cam_low;
            loop {
                let mut state = view(960.0, cam_center_world_px_y, zoom);
                state.view_h_px = VIEW_H_PX as f64;
                let bound = camera_bound(&map, true);
                let half = state.view_h_px / (2.0 * zoom);
                if bound.bottom - bound.top < state.view_h_px / zoom {
                    state.cam_center_world_px_y = bound.top + half;
                }
                let placement = place_layer(&state, &home_anchor(), sky);
                let art_bottom = placement.screen_top_px as f64 + sky.art_h_px * zoom;
                let covered_above =
                    placement.screen_top_px <= 0 || sky.fill_above != Fill::Transparent;
                let covered_below =
                    art_bottom >= state.view_h_px || sky.fill_below != Fill::Transparent;
                assert!(
                    covered_above,
                    "{}: sky top {} leaves row 0 uncovered",
                    label, placement.screen_top_px
                );
                assert!(
                    covered_below,
                    "{}: sky bottom {} leaves the last row uncovered",
                    label, art_bottom
                );
                if cam_center_world_px_y >= cam_high {
                    break;
                }
                cam_center_world_px_y += 1.0;
            }
        }
    }

    /// U5: the copies cover `[0, view_w)` for the editor's zoom ladder and a spread of camera
    /// positions.
    #[test]
    fn u5_copies_cover_the_view() {
        let stack = resolved();
        for zoom in [2.0, 1.0, 0.5, 0.25] {
            for step in 0..1000 {
                let cam_center_world_px_x = -5000.0 + (step as f64) * 10.0;
                let state = view(cam_center_world_px_x, 1572.0, zoom);
                for layer in stack.layers.iter().filter(|layer| layer.repeat_x) {
                    let placement = place_layer(&state, &home_anchor(), layer);
                    let period_screen = layer.period_px * zoom;
                    let last_right = placement.first_copy_left_px as f64
                        + (placement.copy_count as f64) * period_screen;
                    assert!(
                        placement.first_copy_left_px as f64 <= 0.0,
                        "{}: first copy starts at {}",
                        layer.name,
                        placement.first_copy_left_px
                    );
                    assert!(
                        last_right >= state.view_w_px,
                        "{}: copies end at {} before the view's {}",
                        layer.name,
                        last_right,
                        state.view_w_px
                    );
                    assert!(placement.copy_count >= 1);
                }
            }
        }
    }

    /// U6: moving the camera by one whole period of layer movement leaves the placement
    /// unchanged, which is what "seamless" means.
    #[test]
    fn u6_repeat_is_seamless_in_motion() {
        let stack = resolved();
        for name in ["Sky", "Hills", "Foreground"] {
            let layer = resolved_layer(&stack, name);
            let shift = layer.period_px / layer.rate_x;
            let start = place_layer(&view(960.0, 1572.0, 1.0), &home_anchor(), layer);
            let moved = place_layer(&view(960.0 + shift, 1572.0, 1.0), &home_anchor(), layer);
            assert_eq!(start, moved, "{} moved by one period", name);
        }
    }

    /// U7: every rule of `Data` section 4 rejects its own fixture, with its own rule code, and
    /// every rejection renders one journal line.
    #[test]
    fn u7_each_validation_rule_fires() {
        let art = fixture_art();

        let wrong_version = Stack {
            schema_version: 3,
            ..fixture_stack()
        };
        assert!(has_code(&wrong_version.validate(&art), "schema_version"));

        let mut bad_depth = fixture_stack();
        bad_depth.layers[0].parallax_depth = Depth::Finite(-1.0);
        assert!(has_code(&bad_depth.validate(&art), "parallax_depth"));

        let mut bad_damping = fixture_stack();
        bad_damping.layers[0].vertical_damping = 1.5;
        assert!(has_code(&bad_damping.validate(&art), "vertical_damping"));

        let mut back_too_near = fixture_stack();
        back_too_near.layers[6].plane = Plane::Back;
        assert!(has_code(&back_too_near.validate(&art), "plane_depth"));

        let mut front_too_far = fixture_stack();
        front_too_far.layers[0].plane = Plane::Front;
        assert!(has_code(&front_too_far.validate(&art), "plane_depth"));

        let mut duplicate_order = fixture_stack();
        duplicate_order.layers[3].draw_order = 4;
        assert!(has_code(
            &duplicate_order.validate(&art),
            "draw_order_unique"
        ));

        let mut inverted_order = fixture_stack();
        inverted_order.layers[4].draw_order = 5;
        inverted_order.layers[5].draw_order = 4;
        assert!(has_code(&inverted_order.validate(&art), "draw_order_depth"));

        let mut too_many = fixture_stack();
        let extra = too_many.layers[0].clone();
        too_many.layers.push(extra);
        assert!(has_code(&too_many.validate(&art), "max_layers"));

        let mut base_transparent = fixture_stack();
        base_transparent.layers[0].fill_above = Fill::Transparent;
        assert!(has_code(&base_transparent.validate(&art), "base_opaque"));

        let mut art_count_mismatch = fixture_stack();
        art_count_mismatch.layers.pop();
        assert!(has_code(&art_count_mismatch.validate(&art), "art_count"));

        let mut not_rgba = fixture_art();
        not_rgba[0].is_rgba = false;
        assert!(has_code(&fixture_stack().validate(&not_rgba), "file_rgba"));

        let mut fill_edge = fixture_art();
        fill_edge[4].bottom_row_opaque = false;
        assert!(has_code(
            &fixture_stack().validate(&fill_edge),
            "fill_edge_opaque"
        ));

        let failing = bad_depth.validate(&art);
        let message = failing.errors().next().expect("one failure");
        let line = journal_line(message, "neutral_default", "Sky");
        assert!(
            line.starts_with("parallax.validate FAIL stack=neutral_default layer=Sky"),
            "journal line was {:?}",
            line
        );
        assert!(
            line.contains("rule=parallax_depth"),
            "journal line was {:?}",
            line
        );
    }

    /// U17: a non-`transparent` fill needs a fully opaque edge row; the same art with a
    /// transparent fill loads.
    #[test]
    fn u17_a_fill_needs_an_opaque_edge_row() {
        let mut art = fixture_art();
        art[4].bottom_row_opaque = false;
        assert!(has_code(
            &fixture_stack().validate(&art),
            "fill_edge_opaque"
        ));

        let mut stack = fixture_stack();
        stack.layers[4].fill_below = Fill::Transparent;
        assert!(
            stack.validate(&art).is_valid(),
            "a transparent fill needs no opaque row"
        );
    }

    /// U8: migrating the retired manifest preserves all twelve rates.
    #[test]
    fn u8_migration_reproduces_twelve_rates() {
        let raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../editor/assets/backdrop_layers/manifest.json"
        ));
        let legacy: LegacyManifest =
            serde_json::from_str(raw).expect("the tracked manifest parses");
        let migrated = Stack::from_legacy_manifest(&legacy, "neutral_default", "neutral");

        assert_eq!(migrated.schema_version, 2);
        assert_eq!(migrated.stack_key, "neutral_default");
        assert_eq!(migrated.palette_key, "neutral");
        assert_eq!(migrated.generator_seed, legacy.seed);
        assert_eq!(migrated.layers.len(), legacy.layers.len());

        let mut checked = 0;
        for (migrated_layer, legacy_layer) in migrated.layers.iter().zip(legacy.layers.iter()) {
            assert_eq!(migrated_layer.name, legacy_layer.name);
            assert_eq!(
                migrated_layer.repeat_x,
                legacy_layer.repeat_size_x > 0,
                "{} repeat",
                legacy_layer.name
            );
            let rate_x = migrated_layer.rate_x();
            let rate_y = migrated_layer.rate_y();
            assert!(
                (rate_x - legacy_layer.scroll_scale_x).abs() < 1e-4,
                "{} rate_x {} vs legacy {}",
                legacy_layer.name,
                rate_x,
                legacy_layer.scroll_scale_x
            );
            assert!(
                (rate_y - legacy_layer.scroll_scale_y).abs() < 1e-4,
                "{} rate_y {} vs legacy {}",
                legacy_layer.name,
                rate_y,
                legacy_layer.scroll_scale_y
            );
            checked += 2;
        }
        assert_eq!(checked, 12, "six layers x two axes");

        let foreground = migrated
            .layers
            .iter()
            .find(|layer| layer.name == "Foreground")
            .expect("Foreground migrated");
        assert_eq!(foreground.plane, Plane::Front);
        let sky = migrated
            .layers
            .iter()
            .find(|layer| layer.name == "Sky")
            .expect("Sky migrated");
        assert_eq!(sky.plane, Plane::Back);
        assert_eq!(sky.draw_order, 0);
    }

    /// U9: on a one-row map the camera centre is fixed, so looking up cannot scroll off the
    /// map.
    #[test]
    fn u9_one_row_map_pins_the_camera_center() {
        let map = map_rectangle(&[(0, 0)]).expect("one screen");
        assert_eq!(map.bottom - map.top, SCREEN_H_PX as f64);
        for player_y in [-5000.0, -1.0, 0.0, 540.0, 1055.0, 5000.0] {
            let clamped = clamp_center(player_y, VIEW_H_PX as f64, map.top, map.bottom);
            assert_eq!(clamped, 540.0, "player y {}", player_y);
        }
    }

    /// U10: the map rectangle is the bounding box of the occupied screens, including westward
    /// columns.
    #[test]
    fn u10_map_rectangle_is_the_screen_bounding_box() {
        let map = map_rectangle(&[(-2, 0), (0, 0), (0, 1)]).expect("three screens");
        assert_eq!(map.left, -3840.0);
        assert_eq!(map.right, 1920.0);
        assert_eq!(map.top, 0.0);
        assert_eq!(map.bottom, 2112.0);
        assert_eq!(map_rectangle(&[]), None);
    }

    /// U11: the editor's bound is the map grown by one screen on every side; the game's is the
    /// map.
    #[test]
    fn u11_editor_bound_grows_by_one_screen() {
        let map = map_rectangle(&[(0, 0), (0, 1)]).expect("two screens");
        assert_eq!(camera_bound(&map, false), map);
        let editor = camera_bound(&map, true);
        assert_eq!(editor.left, map.left - SCREEN_W_PX as f64);
        assert_eq!(editor.right, map.right + SCREEN_W_PX as f64);
        assert_eq!(editor.top, map.top - SCREEN_H_PX as f64);
        assert_eq!(editor.bottom, map.bottom + SCREEN_H_PX as f64);
    }

    /// U13: each layer snaps on its own, and a half pixel snaps away from the zero floor.
    #[test]
    fn u13_pixel_snap_ties_away_from_the_floor() {
        assert_eq!(snap(10.5), 11);
        assert_eq!(snap(-10.5), -10);
        assert_eq!(snap(10.49), 10);
        assert_eq!(snap(-0.5), 0);
    }

    /// U14: the default anchor is the camera centre at the home view: screen column 0, bottom
    /// row, clamped.
    #[test]
    fn u14_default_anchor_is_the_home_view_center() {
        let map = map_rectangle(&[(0, 0), (0, 1)]).expect("two screens");
        let anchor = default_anchor(&map, &view(0.0, 0.0, 1.0));
        assert_eq!(anchor.world_px_x, 960.0);
        assert_eq!(anchor.world_px_y, 1572.0);
    }

    /// U15: the front plane's ceiling: rate 2 loads, anything nearer is a rejection.
    #[test]
    fn u15_front_plane_ceiling() {
        let art = fixture_art();
        let mut at_ceiling = fixture_stack();
        at_ceiling.layers[6].parallax_depth = Depth::Finite(-0.5);
        assert!(
            at_ceiling.validate(&art).is_valid(),
            "-0.5 is rate 2, the ceiling and still allowed"
        );

        let mut beyond = fixture_stack();
        beyond.layers[6].parallax_depth = Depth::Finite(-0.51);
        let result = beyond.validate(&art);
        assert!(has_code(&result, "plane_depth"));
        let message = result.errors().next().expect("one failure");
        let line = journal_line(message, "neutral_default", "Foreground");
        assert!(
            line.contains("rule=plane_depth"),
            "journal line was {:?}",
            line
        );
    }

    /// U19: an authored anchor outside the camera bound is clamped, with one clamp line; an
    /// anchor inside it is untouched.
    #[test]
    fn u19_anchor_override_is_clamped() {
        let map = map_rectangle(&[(0, 0), (0, 1)]).expect("two screens");
        let state = view(960.0, 1572.0, 1.0);

        let (clamped, moved) = clamp_anchor(
            Anchor {
                world_px_x: 5000.0,
                world_px_y: -300.0,
            },
            &map,
            &state,
        );
        assert!(moved);
        assert_eq!(clamped.world_px_x, 960.0);
        assert_eq!(clamped.world_px_y, 540.0);
        let line = anchor_clamp_line(
            "neutral_default",
            &Anchor {
                world_px_x: 5000.0,
                world_px_y: -300.0,
            },
            &clamped,
        );
        assert_eq!(
            line,
            "parallax.anchor CLAMP stack=neutral_default from=5000,-300 to=960,540"
        );

        let (untouched, moved) = clamp_anchor(
            Anchor {
                world_px_x: 960.0,
                world_px_y: 1000.0,
            },
            &map,
            &state,
        );
        assert!(!moved);
        assert_eq!(untouched.world_px_y, 1000.0);
    }

    /// U20: the game's camera centre is clamped to the map, so the player can never see past
    /// the first or last row.
    #[test]
    fn u20_game_camera_is_clamped_to_the_map() {
        let map = map_rectangle(&[(0, 0), (0, 1)]).expect("two screens");
        let bound_y = (map.top, map.bottom);
        assert_eq!(
            clamp_center(3000.0, VIEW_H_PX as f64, bound_y.0, bound_y.1),
            1572.0
        );
        assert_eq!(
            clamp_center(-500.0, VIEW_H_PX as f64, bound_y.0, bound_y.1),
            540.0
        );
        let bound_x = (map.left, map.right);
        assert_eq!(
            clamp_center(9000.0, VIEW_W_PX as f64, bound_x.0, bound_x.1),
            960.0
        );
        assert_eq!(
            clamp_center(-9000.0, VIEW_W_PX as f64, bound_x.0, bound_x.1),
            960.0
        );
    }

    /// U23: the camera snaps to the pixel grid at zoom 1 and to half pixels at zoom 2, so the
    /// tiles and the backdrops land on one grid; below zoom 1 it is left alone.
    #[test]
    fn u23_camera_snap_puts_one_grid_under_everything() {
        assert_eq!(snap_center(1000.4, 1.0), 1000.0);
        assert_eq!(snap_center(700.6, 1.0), 701.0);
        let at_zoom_two = snap_center(700.6, 2.0);
        assert_eq!(at_zoom_two, (at_zoom_two * 2.0).round() / 2.0);
        assert_eq!(snap_center(700.6, 0.5), 700.6);
        assert_eq!(snap_center(-0.5, 1.0), 0.0);
    }
}
