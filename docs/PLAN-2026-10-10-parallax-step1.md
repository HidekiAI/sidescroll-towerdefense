# Plan: step 1 of the parallax restructure (`sstd-core::parallax`, #104)

Design of record: [TDD_Parallax-Scrolling](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TechnicalDesign/TDD_Parallax-Scrolling)
and its [.Math](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TechnicalDesign/TDD_Parallax-Scrolling.Math),
[.Data](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TechnicalDesign/TDD_Parallax-Scrolling.Data) and
[.Verification](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TechnicalDesign/TDD_Parallax-Scrolling.Verification)
sub-pages. Ticket: #104. Closes #91.

Status: **pre-implementation**. No code for this step exists yet. The seven steps are
#104-#110; this plan covers only #104.

## 1. Scope

In scope (from #104 and `Math` section 2 / `Data` sections 1, 4, 5):

- a new module `crates/sstd-core/src/parallax.rs`, pure Rust, no Godot types
- the stack data model: `Depth`, `Layer`, `Plane`, `Fill`, `Stack`, `ViewState`, `Anchor`,
  `Placement`, plus the resolved forms
- `place_layer`, the one placement function
- load-time validation of every rule in `Data` section 4
- the camera bounds, default anchor and anchor clamp of `Math` section 5
- the pixel-snap helpers of `Math` section 2 and test U23
- the `manifest.json` -> `stack.json` migration of `Data` section 5
- unit tests U1-U11 and U13-U17, U19-U23 (the ticket AC's list; U12 is blocked until a Phase 2
  simulator exists and U18 is excluded by that same AC line)

Out of scope, deliberately:

- emitting `stack.json` (step 2, #105) and reading PNGs to fill it; U18 is a `gen-backdrop`
  test and the ticket's AC excludes it for exactly that reason
- the Related-section outcomes (the #101 `Parallax2D` formula superseded, SubViewport isolation
  kept, the #88/#99/#90 root causes removed) are served by this module existing and by steps
  4-5 (#107/#108) wiring it; nothing in step 1 asserts them
- the bridge `#[func]` surface (step 3, #106)
- any Godot node, scene or drawing (steps 4-5)
- `backdrop.json`, the legacy `layers.json` reader and `editor_prefs.json` (step 6, #109)
- the G5 / letterbox / Fyrox spikes (`Math` section 7, `Evaluation` section 4)

## 2. What this builds on (existing code, read before writing)

| Existing thing | File | How step 1 uses it |
|---|---|---|
| `ValidationResult`, `ValidationMessage`, `ValidationSeverity` | `src/error.rs` | the return type of `validate`; `with_field` / `with_value` carry the journal fields |
| The `ValidationMessage::error(code, msg).with_field(..).with_value(..)` builder idiom | `src/error.rs:22`, used by `src/map.rs:36`, `src/terrain.rs` | the shape every rule is written in; no new error enum |
| `Screen`, `TileGrid::validate` merging sub-results with `result.merge(..)` | `src/map.rs:33`, `src/map.rs:162` | the merge-in-order, never-early-return idiom for the rule table |
| `tile_to_world` / `world_to_screen` signed-coordinate comment block | `src/map.rs:219` | the precedent for negative world coordinates (westward corridors) |
| `pub use` re-export list and one `mod` line per module | `src/lib.rs` | add `pub mod parallax;` plus a `pub use parallax::{..}` block |
| In-file `#[cfg(test)] mod tests` in every `src/*.rs` | all 9 files | the tests live in `parallax.rs`, not a `tests/` dir (the crate has none) |
| `CURRENT_SCHEMA_VERSION: &str` + version-mismatch warning | `src/storage.rs:11`, `src/storage.rs:53` | the precedent for `schema_version`; **but** `Data` section 5 asks for a rejection here, not a warning - see section 6 |
| `LegacyManifest` field names (`version`, `width_px`, `overscan_px`, `seed`, `layers[].scroll_scale_x/_y`, `repeat_size_x/_y`, `z_index`, `height_px`) | `tools/gen-backdrop/src/main.rs:71-90`, and the artifact at `editor/assets/backdrop_layers/manifest.json` | the input shape of the migration; the on-disk file is the U8 fixture |
| Crate deps: `serde` (derive), `serde_json` | `crates/sstd-core/Cargo.toml` | sufficient; **no new dependency is added** (see section 3) |

## 3. Two design decisions this plan has to settle

Both are gaps in the wiki pages, found by reading the code. Neither changes a formula.

### 3.1 The PNG facts are an input, not something `sstd-core` reads

`Data` section 4 says validation checks that `file` exists and is an RGBA PNG, and that a
non-`transparent` fill needs that PNG's edge row fully opaque. `Math` section 1 says
`period_px` is the PNG width, and `Data` section 1 says art size is read from the PNG and
never stored.

`sstd-core` has **no `image` dependency** and no art-loading path; its only dependency beyond
`serde`/`serde_json` is `rusqlite`. Adding `image` to a pure math crate, and giving it
filesystem policy, is the wrong trade for four booleans and two integers.

**Decision:** the facts are an explicit input.

```rust
pub struct LayerArt {
    pub width_px: u32,
    pub height_px: u32,
    pub is_rgba: bool,
    pub fully_opaque: bool,       // alpha 255 at every pixel of the whole PNG
    pub top_row_opaque: bool,     // alpha 255 at every pixel of row 0
    pub bottom_row_opaque: bool,  // alpha 255 at every pixel of the last row
}
```

`validate(&Stack, &[LayerArt])` and `Stack::resolve(&Stack, &[LayerArt])` take one `LayerArt`
per layer, index-aligned; index `i` describes `stack.layers[i]`. The caller that measured the
PNG builds both together, so the alignment has one owner.

Who measures them: `gen-backdrop` (step 2, it already depends on `image` 0.25 and it writes
the fills), and the loader in step 6. The bridge (step 3) passes them through.

The one pixel computation that is pure lives here so #105 only has to hand over rows:
`Fill::from_edge_row(row: &[u8], width_px: usize) -> Fill` returns the per-channel mean of a
fully opaque row, else `Transparent`. That is the rule U22 asserts (`fill_above` equals the
per-channel mean of row 0 ... else `transparent`) and the rule `fill_edge_opaque` validates
against, so the two cannot drift. U22's other half (the emitted PNGs, and the sky haze clamp)
is `gen-backdrop`'s assertion and lands in #105.

Why this is better than the literal wiki wording: it keeps the module a pure function of its
inputs, which is the property the whole design rests on (`Math` section 2's "one pure
function of the view"), it makes U17's fixture constructible without a PNG on disk, and it
makes "PNG is RGBA" a measurement made once by the tool that produced the PNG rather than
re-derived per load.

Cost: the wiki wording needs a one-line amendment, which is tracked as its own doc follow-up
(section 9), not as an action in this branch.

### 3.2 Journal lines are formatted by the caller, from `ValidationMessage`

`Data` line 146 states it exactly:

```
parallax.validate FAIL stack=<key> layer=<name> field=<field> value=<value> rule=<rule>
```

and U7 asserts it ("each must be rejected with its journal line"). Core logs nowhere today:
`map.rs` and `terrain.rs` return a `ValidationResult` and the bridge formats it. Keep that, and
keep the formatter **inside `parallax.rs`** so this step adds one module and edits nothing else
(the ticket Scope names one new file). `ValidationMessage` already carries exactly the three
variables (`code` is the rule, `field`, `value`), so the mapping is total and needs no new type:

```rust
/// `parallax.validate FAIL stack=<key> layer=<layer> field=<field> value=<value> rule=<code>`
pub fn journal_line(msg: &ValidationMessage, stack_key: &str, layer: &str) -> String;
/// `parallax.anchor CLAMP stack=<key> from=<x,y> to=<x,y>`   (U19, Data line 156)
pub fn anchor_clamp_line(stack_key: &str, from: &Anchor, to: &Anchor) -> String;
```

A message with no `field`/`value` renders `field=-` / `value=-`, so the line is always
well-formed. `rule=` takes `code`, which is already the rule name in this crate's style
(`SCREEN_INVALID_WIDTH`, `TILE_OUT_OF_BOUNDS`). Rule codes for this module are listed in
section 6.

## 4. Module layout and public API

`crates/sstd-core/src/parallax.rs`, one file, in-file tests. Types that are private to this
file stay in it, above their first use (workspace rule: private = same file, shared = own
file). Everything below is `pub` because steps 2-6 consume it, and is re-exported from
`lib.rs`.

```rust
// constants
pub const SCREEN_W_PX: i32 = 1920;      // R1: 60 x 33 tiles at 32 px
pub const SCREEN_H_PX: i32 = 1056;
pub const VIEW_W_PX: i32 = 1920;        // R2: view is one screen wide plus 24 px
pub const VIEW_H_PX: i32 = 1080;
pub const MAX_LAYERS: usize = 7;        // Data 4: parallax.max_layers, never seeded in config
const LEGACY_FRONT_Z_INDEX: i32 = 10;   // Data 5: foreground threshold in manifest.json

// model (serde), exactly Data 1's shape
pub enum Depth { Finite(f64), Infinite }        // tagged: {"kind":"finite","value":n} / {"kind":"infinite"}
pub enum Plane { Back, Front }
pub enum Fill { Transparent, Rgba(u8, u8, u8, u8) }
pub struct RefView { pub width: f64, pub height: f64 }
pub struct Layer { .. }
pub struct Stack { .. }   // schema_version, stack_key, palette_key, generator_seed, ref_view, layers

// resolved, validated form: Math section 2's `Layer`, with the rate and the period precomputed
pub struct ResolvedLayer { .. }
pub struct ResolvedStack { ref_view: RefView, layers: Vec<ResolvedLayer> }
pub struct LayerArt { .. }              // section 3.1

// view / geometry
pub struct ViewState { cam_center_world_px_x, cam_center_world_px_y, zoom, view_w_px, view_h_px }
pub struct Anchor { world_px_x, world_px_y }
pub struct Placement { first_copy_left_px: i32, copy_count: u32, screen_top_px: i32, scale: f64 }
pub struct MapRect { left, top, right, bottom }   // world px, right/bottom exclusive

// functions
pub fn place_layer(view: &ViewState, anchor: &Anchor, layer: &ResolvedLayer) -> Placement;
pub fn snap(v: f64) -> i32;                       // snap(10.5) = 11, snap(-10.5) = -10 (U13)
pub fn snap_center(v: f64, zoom: f64) -> f64;     // U23
pub fn map_rectangle(screens: &[(i32, i32)]) -> Option<MapRect>;   // U10
pub fn camera_bound(map: &MapRect, view: &ViewState, editor: bool) -> MapRect;  // U11
pub fn clamp_center(center: f64, view_extent: f64, bound_lo: f64, bound_hi: f64) -> f64;  // U20
pub fn default_anchor(map: &MapRect, view: &ViewState) -> Anchor;  // U14, Data 3
pub fn clamp_anchor(anchor: Anchor, map: &MapRect, view: &ViewState) -> (Anchor, bool);  // U19

// journal lines (section 3.2), formatter inside this module so no other file changes
pub fn journal_line(msg: &ValidationMessage, stack_key: &str, layer: &str) -> String;   // U7
pub fn anchor_clamp_line(stack_key: &str, from: &Anchor, to: &Anchor) -> String;        // U19

impl Fill {
    /// Per-channel mean of a fully opaque edge row, else `Transparent` (U22).
    pub fn from_edge_row(row: &[u8], width_px: usize) -> Fill;
}

// entry points
impl Stack {
    pub fn validate(&self, art: &[LayerArt]) -> ValidationResult;
    pub fn resolve(&self, art: &[LayerArt]) -> Result<ResolvedStack, ValidationResult>;
    pub fn from_legacy_manifest(m: &LegacyManifest) -> Self;   // U8
}
pub struct LegacyManifest { .. }        // pub so step 2/6 and the U8 fixture can build it
```

Departures from `Math` section 2's sketch, all collected in section 9's callout table:

- `ResolvedLayer` keeps the sketch's field names and variants (`Depth::Infinite`,
  `first_copy_left_px`, `screen_top_px`) but is the resolved form: `rate_x`/`rate_y`
  precomputed from `depth` + `vertical_damping`, `period_px` from the PNG width, the stack's
  `ref_view` copied in, plus `art_h_px` and the two fills, which section 7's drawing needs.
  `ResolvedLayer` / `ResolvedStack` are derivation caches, not ticket-mandated types.
- `place_layer` takes `&ResolvedLayer`, not the raw `&Layer`, so nothing is recomputed per
  frame; otherwise the body is the sketch line for line (section 5).

## 5. Algorithm: `place_layer`

Verbatim from `Math` section 2's Rust sketch (lines 80-94), with `ref_view` read from the
resolved layer instead of a module constant:

```rust
pub fn place_layer(view: &ViewState, anchor: &Anchor, layer: &ResolvedLayer) -> Placement {
    let layer_cam_x = anchor.world_px_x + (view.cam_center_world_px_x - anchor.world_px_x) * layer.rate_x;
    let layer_cam_y = anchor.world_px_y + (view.cam_center_world_px_y - anchor.world_px_y) * layer.rate_y;
    let art_left_world = anchor.world_px_x + layer.ref_screen_left_px - layer.ref_view.width / 2.0;
    let art_top_world  = anchor.world_px_y + layer.ref_screen_top_px  - layer.ref_view.height / 2.0;
    let art_screen_left = (art_left_world - layer_cam_x) * view.zoom + view.view_w_px / 2.0;
    let art_screen_top  = (art_top_world  - layer_cam_y) * view.zoom + view.view_h_px / 2.0;
    let (first_copy_left, copy_count) = if layer.repeat_x {
        let period_screen = layer.period_px * view.zoom;
        let first_copy_left = art_screen_left.rem_euclid(period_screen) - period_screen;
        (first_copy_left, ((view.view_w_px - first_copy_left) / period_screen).ceil() as u32)
    } else {
        (art_screen_left, 1)
    };
    Placement { first_copy_left_px: snap(first_copy_left), copy_count,
                screen_top_px: snap(art_screen_top), scale: view.zoom }
}
```

Notes that matter for correctness and that a reader will otherwise get wrong:

- The repeat is computed on the **unsnapped** left, and only the returned value is snapped
  (`Math` section 3's zoom-0.25 example: `copy_count` is always 5 there).
- `rem_euclid` is the `fposmod`, and it is then shifted by one period: `first_copy_left` lies in
  `[-period_screen, 0)`, never `[0, period_screen)`. `copy_count` is
  `ceil((view_w - first_copy_left) / period_screen)`, which covers `[0, view_w)`.
- `zoom` is optical (`Math` section 6): it multiplies only the projection, never the rate,
  so relative motion is zoom-independent and a `rate 0` layer is pinned on pan but scales on
  zoom (U21).
- Nothing here reads `Screen::default()` or `GridConfig::default()`. Those are stale
  (`30 x 16` at `64 px`, `src/terrain.rs:564`); the sizes are this module's constants and the
  caller passes the real `ViewState`. See section 9.

## 6. Validation rules and their codes

`Stack::validate` walks `Data` section 4 in table order, merges every failure
(`result.merge(..)`, as `map.rs` does) and never early-returns, so one load reports every
problem. `code` is the `rule=` value in the journal line.

| Rule (`Data` 4) | `code` |
|---|---|
| `schema_version` is 2 | `schema_version` |
| `parallax_depth` finite with `value > -1`, or the `infinite` tag | `parallax_depth` |
| `vertical_damping` in `[0, 1]` | `vertical_damping` |
| `plane = back` needs depth `>= 0`; `plane = front` needs `-0.5 <= depth < 0` | `plane_depth` |
| within a plane, `draw_order` unique | `draw_order_unique` |
| ascending `draw_order` has non-increasing depth on the back plane (base exempt) | `draw_order_depth` |
| layer count `<= MAX_LAYERS` | `max_layers` |
| the lowest-`draw_order` back layer has an all-opaque PNG (`fully_opaque`) and opaque `fill_above` / `fill_below` | `base_opaque` |
| a non-`transparent` fill needs that edge row fully opaque | `fill_edge_opaque` |
| `file` is an RGBA PNG | `file_rgba` |
| `LayerArt` count equals `layers.len()` | `art_count` |

`LayerArt` carries no filename, so "file exists" is not checkable here and is a **step 6**
(loader) and **step 2** (generator) concern; `file_rgba` covers the encodable half of the
rule. Recorded in section 9 so it is not silently dropped.

Ordering rule detail worth pinning in a test: the `infinite` tag counts as the largest depth,
and the base layer (lowest `draw_order` of the back plane) is exempt from the ordering rule,
which is what lets a `Sun` tagged `infinite` sit above the opaque `Sky` and below the rest.

## 7. Bounds, anchor and snap (`Math` section 5)

- `map_rectangle` takes the occupied `(col, row)` cells from the archive's screens table and returns
  `(min_col*1920, min_row*1056)` to `((max_col+1)*1920, (max_row+1)*1056)`, exclusive on the
  right/bottom. Columns may be negative. `None` for an empty screen set (the caller decides
  what to do; `Data` section 3 says a world with no stack gets `neutral_default`).
- `camera_bound(map, view, editor)`: game = the map rect; editor = the map rect grown by one
  screen (1920 x 1056) per side.
- `clamp_center`: bound the centre to `[bound_lo + view_extent/(2*zoom), bound_hi - view_extent/(2*zoom)]`;
  when the bound is smaller than the view on that axis, fix the centre so the view's top-left
  equals the bound's top-left. That single rule produces the two-row clamp (U20) and the
  one-row fixed centre (U9).
- `default_anchor`: the camera centre at the home view — screen column 0 (or the leftmost
  column when there is no column 0), bottom row, then clamped. Yields `(960, 1572)` for the
  `"0,0"`/`"0,1"` map (U14).
- `clamp_anchor` returns the clamped anchor plus a `bool` the caller turns into the
  `parallax.anchor CLAMP stack=<key> from=<x,y> to=<x,y>` line (U19); the clamp is a warning,
  not a rejection.
- `snap(v) = floor(v + 0.5)` — half away from the zero floor, so `snap(-10.5) = -10` (U13).
- `snap_center(v, zoom) = snap(v * zoom) / zoom` — the camera is rounded to the pixel grid at
  zoom `>= 1` and left alone at zoom `< 1` (U23: at zoom `0.5` the camera is not rounded).

## 8. Migration (`Data` section 5)

`Stack::from_legacy_manifest(&LegacyManifest) -> Stack`:

| legacy | new | rule |
|---|---|---|
| `version: "1.0"` | `schema_version: 2` | constant |
| `seed` | `generator_seed` | kept |
| `scroll_scale_x` | `parallax_depth` | `1 / rate - 1` |
| `scroll_scale_y` | `vertical_damping` | `rate_y / rate_x` |
| `repeat_size_x = 1920` | `repeat_x: true` | `repeat_size_x > 0` |
| `z_index < 0 or >= 1` up to the foreground | `draw_order` | order preserved |
| the foreground (`z_index 10`) | `plane: front` | `z_index >= 10` is a documented threshold, not a magic number in the body; it gets a named constant |
| `width_px`, `overscan_px`, `height_px` | dropped | read from the PNG, fills replace overscan |
| (absent in the legacy file) | `stack_key`, `palette_key` | parameters: `from_legacy_manifest(m, stack_key, palette_key)`. The legacy file has neither, and only the caller knows the stack's key and palette |
| (absent in the legacy file) | `ref_screen_left_px`, `ref_screen_top_px`, `fill_above`, `fill_below` | neutral values (`0`, `Transparent`); `gen-backdrop` (step 2) supplies the derived placements and fills, which the legacy file does not contain |
| `width_px` | `ref_view.width` | the legacy ref width is `1920` |
| (absent in the legacy file) | `ref_view.height` | `VIEW_H_PX` (1080): the legacy sky is 1320 = 1080 + the 240 px overscan, so 1080 is the view height the old art was composed against |

The twelve rates of the on-disk manifest (`editor/assets/backdrop_layers/manifest.json`,
6 layers x 2 axes) reproduce the `Data` section 1 depths and dampings to within `1e-4`: e.g.
Sky `0.10` -> `parallax_depth 9`, `0.08/0.10 -> 0.8`; Hills `0.50 -> 1`, `0.42/0.50 -> 0.84`;
Foreground `1.30 -> -0.2308`, `1.15/1.30 -> 0.8846`. That file is the U8 fixture.

`ref_screen_left_px` / `ref_screen_top_px` and the fills are **not** derivable from the legacy
manifest (the legacy framing lived in `backdrop_preview.tscn`, not in the manifest, and the
fills want the PNG's edge rows) — `Data` section 1 derives them from the old design frame, and
`Data` section 5 gives `ref_screen_top_px = old_sprite_y - 240` with the sprite y read from the
scene. The migration therefore sets them to their neutral values (`0`, `Transparent`) and
`gen-backdrop` (step 2) supplies the derived ones, which is why U8 asserts the twelve **rates**
and nothing about placements. Stated here so nobody expects `from_legacy_manifest` to invent
the foreground framing or the fills.

## 9. Divergences from the ticket's literal wording, and possible follow-ups (not this ticket)

Every row below is a phrasing difference from the ticket or the wiki, not added scope. Each
is stated so the gate can see it, with the ticket line it still serves. Read this table as the
single callout for "where the plan departs from the ticket's literal wording": two API-shape
changes (`validate` takes the PNG facts; the screen size is a constant rather than a
parameter), one placement-field rename, and two derived helper types.

| Item | Tickets line it serves | Action |
|---|---|---|
| Ticket Scope reads `validate(&Stack) -> ValidationResult`; the plan writes `validate(&Stack, &[LayerArt])`. The literal `&Stack` signature cannot check `file_rgba` / `fill_edge_opaque` (Data section 4): those rules read each layer's PNG, and the PNG facts are the only engine-free input that can express them, so the parameter is forced by the ticket's own referenced rules, not chosen | that Scope line, plus AC `validate emits one ValidationMessage per failing rule, carrying field and value`, and `Data` section 4's `file` / `fill_edge_opaque` rows | stated in section 3.1; the wiki sentence is corrected by its own doc follow-up, not by this branch |
| Ticket Scope reads the screen size is "passed in (1920 x 1056)"; the plan makes it the constants `SCREEN_W_PX` / `SCREEN_H_PX` and passes the *view* size in `ViewState` | that Scope line, plus its stated anti-goal: never read the stale `Screen::default()` / `GridConfig::default()` | `R1` fixes the screen at 1920 x 1056, so a parameter would be false flexibility; the anti-goal is met either way |
| Ticket Scope reads `place_layer(view, anchor, layer) -> Placement`; the plan passes a `ResolvedLayer` and adds `ResolvedLayer` / `ResolvedStack` | that Scope line | same function, same field names and variants as `Math` section 2's sketch; only the type *name* differs, because the resolved form drops `depth`/`vertical_damping` in favour of the rates the sketch's own `impl` computes |
| `Fill::from_edge_row` | AC `U22` (the fill equals the edge-row mean, else transparent) | step 1's U22 test is the **pure half only**, permitted by the wiki's own U22 title, which marks the emitted-layer and sky-haze assertions as a `` `gen-backdrop` `` test; that half is #105 |
| The journal formatters (`journal_line`, `anchor_clamp_line`) | AC `validate ... carrying field and value`; `Data` line 146; U7, U19 | they live in `parallax.rs`, so no file outside the new module changes |
| `Data` section 4 implies `validate` reads the PNG | the two rule rows it states (`file_rgba`, `fill_edge_opaque`) | the code side is section 3.1; the wiki sentence is corrected by its own doc follow-up, not by this branch |
| `Math` section 2's sketch and `Math` section 7's draw call both use `first_copy_left_px` / `screen_top_px`; the plan keeps those names and variants verbatim (`Depth::Infinite`, `rem_euclid(..) - period`) | the placement function's return value; U23 asserts those exact names | no divergence: the first draft of this plan renamed them and was corrected against the sketch |
| config keys `parallax.*` | the `max_layers` rule | never seeded (`Data` section 5), so `MAX_LAYERS` is a module constant |

### Possible follow-ups (not this ticket)

- `file` exists on disk is not checkable in core; owned by step 2 (#105) and step 6 (#109).
  Noted here so the rule is not lost, not implemented here.
- U18 (`sun.png` alpha matches the old blend) is a `gen-backdrop` test; the ticket AC excludes
  it, so this step does not plan it. It lands in #105.
- `TDD_Parallax-Scrolling.Data` section 4 should say the caller supplies `LayerArt` rather than
  implying core reads the PNG. Its own doc ticket (wiki edit), not this branch.
- `Screen::default()` is `30 x 16` and `GridConfig::default()` is `64 px`, while `R1` says
  60 x 33 at 32 px. This module consumes neither. The stale defaults are pre-existing and get
  their own ticket; this branch does not touch them.

## 10. Test map (`Verification` section 2.1)

Assistant writes all of these, in-file, red first. Where a test needs a fixture the mutation
would break, the fixture is built in code (U4) or read from the tracked
`editor/assets/backdrop_layers/manifest.json` (U8) — never from an untracked artifact.

The ticket's AC names `U1-U11 and U13-U17, U19-U23`. Every one of those has a row below;
U12 is blocked and U18 is excluded by that same AC line (it is a `gen-backdrop` test, #105).

| Test | Fixture | Note |
|---|---|---|
| U1 | 7 layers, anchor `(960, 1572)`, three view sizes | both axes asserted |
| U2 | camera `+1000` | ratio == rate |
| U3, U21 | a `Depth::Infinity` layer | fixed over 50 camera positions and 4 zooms |
| U4 | two-row map, camera sweep, `place_layer` called directly | bypasses `validate` on purpose (the mutated fixture would be rejected) |
| U5 | zoom ladder + 1000 deterministic camera x values | copies cover `[0, view_w)` |
| U6 | camera moved by `period_px / rate` | placement identical |
| U7 | one bad fixture per rule in section 6 | each rejected, asserting its `journal_line` |
| U8 | the tracked legacy manifest | twelve rates within `1e-4` |
| U9 | one-row map, camera centre pinned | small-map branch |
| U10 | screens `"-2,0"`, `"0,0"`, `"0,1"` | `[-3840, 1920) x [0, 2112)` |
| U11 | editor bound grown by a screen per side; game bound == map | margin 0 |
| U13 | `10.5`, `-10.5`, `10.49` | each layer independently |
| U14 | screens `"0,0"`, `"0,1"`, view 1920 x 1080 | anchor `(960, 1572)` |
| U15 | `front` at `-0.5` and `-0.51` | ceiling |
| U16 | `Moon` at the `infinite` tag | same as U3, name not special-cased |
| U17 | `LayerArt` with one alpha-0 pixel in the bottom row | rejected with `rule=fill_edge_opaque`; same with `transparent` loads |
| U19 | anchor `(5000, -300)` then `(960, 1000)` | clamped to `(960, 540)`, one `anchor_clamp_line` |
| U20 | two-row map, player at y `3000` then `-500` | camera clamp |
| U22 | synthetic RGBA edge rows | step 1 satisfies AC1's `U22` by its pure half: `Fill::from_edge_row` returns the mean, else `Transparent`. The wiki's U22 title marks the emitted-PNG and sky-haze half a `` `gen-backdrop` `` test, which is #105. AC1 names `U22` unqualified; the split is the wiki's own title (cited in section 9), so the gate chain AC1 -> U22 -> pure half is explicit |
| U23 | fractional player position | camera snap; `first_copy_left_px`/`screen_top_px` integers |
| U12 | - | blocked until the Phase 2 simulator exists (ticket says so) |
| U18 | - | not this step: `gen-backdrop` test, excluded by the AC line (#105) |

The order matters: U4 and U17 assert that the *pure* path and the *validated* path disagree by
design, so each test names which one it exercises.

Each test is trusted only after its mutation (last column of `Verification` section 2.1) is
applied and seen red. That verification pass is part of this step, not a follow-up.

## 11. Block map (what the coding-assistant hands to @me)

Assistant writes: the module skeleton (constants, struct and field definitions, `serde`
attributes, `Fill`'s `Serialize` impl, the `lib.rs` wiring), and the whole
`#[cfg(test)] mod tests`. `Depth` carries no hand-written impls at all: a `tag`/`content`
derive on the enum generates both, which is what the tagged wire form means (amended
2026-10-11, note below the table). @me writes the logic:

| Block | Where | What to write (not how) | Depends on | Tests that go green |
|---|---|---|---|---|
| 1/17 | `Fill` `Deserialize` (done 2026-10-11) | `"transparent"` or `"#RRGGBBAA"` | - | `fill_parses_transparent_and_hex` |
| 2/17 | `Fill::from_edge_row` (done 2026-10-12) | the edge-row mean, else `Transparent` | - | U22 |
| 3/17 | `Layer::rate_x` (done 2026-10-12) | the rate from depth | - | U2, U6 |
| 4/17 | `Layer::rate_y` (done 2026-10-12) | damped on y | 3 | U1, U4 |
| 5/17 | `snap` (done 2026-10-12) | the pixel-grid snap | - | U13 |
| 6/17 | `snap_center` (done 2026-10-13) | the camera variant, pass-through below zoom 1 | 5 | U23 |
| 7/17 | `place_layer` (done 2026-10-13) | section 5, repeat included | 3, 4, 5 | U1-U6, U16, U21 |
| 8/17 | `map_rectangle` (done 2026-10-13) | the screen-set bounding box | - | U10 |
| 9/17 | `camera_bound` (done 2026-10-13) | the one-screen editor margin | - | U11 |
| 10/17 | `clamp_center` (done 2026-10-13) | the bound clamp and the small-bound branch | - | U9, U20 |
| 11/17 | `default_anchor` (done 2026-10-13) | the home-view default | 10, 12 | U14 |
| 12/17 | `clamp_anchor` (done 2026-10-13) | clamp the authored anchor, report the move | 10 | U19 |
| 13/17 | `journal_line` (done 2026-10-13) | the `Data` line 146 format | - | U7 |
| 14/17 | `anchor_clamp_line` (done 2026-10-13) | the `Data` line 156 format | - | U19 |
| 15/17 | `Stack::validate` (done 2026-10-13) | the section 6 rule table, all failures at once | 1, 2, 3, 4 | U7, U15, U17 |
| 16/17 | `Stack::resolve` (done 2026-10-13) | validate, then build the resolved layers | 15 | every placement test |
| 17/17 | `Stack::from_legacy_manifest` (done 2026-10-13) | the section 8 table | 3, 4 | U8 |

Seventeen blocks, dependency-ordered - one per function, so each block is independently
checkable and none is a two-function edit. `Fill`'s `Serialize` impl and everything in the
type definitions are scaffolding the assistant writes, not blocks: they are declared as such
in the handoff. `Depth` is the one type whose serialization is a plain derive: the wire form
is the tagged object from `Data` section 1, and the derive needs no `alloc` feature on serde
1.0.229 (verified 2026-10-11). The tag costs a map and a key per layer at load time only;
kind-first order (as serde emits) buffers nothing, so the runtime cost is a one-time
per-stack parse, never in the frame loop. Approval of this map is required before any marker
is inserted (coding-assistant Phase 1 step 3); the markers are inserted only after the tests
exist and have been seen red (Phase 1 step 0, "tests come first").

The handed-off state was observed exactly as this section predicts: `cargo test -p sstd-core
parallax` gave 22 failed / 2 passed (the two passes are the round-trip tests for the
assistant-written `Serialize` impls), and `cargo check` then reported exactly 18
`mismatched types` errors, one per block, on the block lines (17 once the user had written
block 1/18). The 2026-10-11 amendment deletes that block, so after the re-scaffold the same
check reports 17 `mismatched types` errors, and the derive is covered by assistant-written
round-trip tests, not by a block.

## 12. Exit conditions for this step

1. `cargo build -p sstd-core` green, with no Godot linkage (AC4 verbatim).
2. `cargo test -p sstd-core parallax` (the ticket's command) green, with U1-U11 and U13-U17,
   U19-U23 present and named as above; the full `cargo test -p sstd-core` stays green too.
3. Every U-test named in condition 2 shown to fail under its listed mutation.
4. `cargo clippy -p sstd-core -- -D warnings` and `cargo fmt --check` clean.
5. `cargo test -p sstd-core` still reports the pre-existing 110 tests plus the new ones, no
   test deleted or weakened.
6. No Godot file touched; no new dependency in `crates/sstd-core/Cargo.toml`; no file outside
   `src/parallax.rs` and `src/lib.rs` edited.
7. The four GDScript suites unchanged (this step cannot affect them).

## 13. Plan gate

Run 2026-10-10 against #104 (ticket fetched in full, zero comments). Independent evaluator,
`MIN_SCORE` 90, two iterations, same evaluator session for both so the scores are comparable.

| Iteration | Score | Outcome |
|---|---|---|
| 1 | 86 | below the bar: U22 had no plan item, and section 12's exit list contradicted AC1 |
| 2 | 92 | pass |
| 3 | 94 | pass (2026-10-11: tagged wire-form amendment; no CRITICAL, no UNCOVERED, three REFINEMENTS applied) |

Iteration 1 findings and how each was closed:

| Finding | Closed by |
|---|---|
| U22 present in AC1 but in no plan item | a U22 row in section 10 and a `Fill::from_edge_row` item (section 3.1, section 4, block 2/M) |
| Section 12 exit list dropped U19-U23 and used a different cargo command | exit condition 1 now names the AC list and the ticket's command |
| Journal formatter would have edited `src/error.rs` | moved into `parallax.rs` as free functions; the step now edits no file but `parallax.rs` and `lib.rs` |
| Two API-shape divergences argued from the wiki, in two places | one callout table (section 9) naming both, each with the ticket line it serves |
| `ResolvedLayer` / `place_layer(&ResolvedLayer)` unnamed as a deviation | a section 9 row, and section 4 now says they are derivation caches |
| Related-section outcomes never referenced | section 1 states they are served by the module existing and by #107/#108 |

Iteration 2 left three residual deficiencies, all accepted and fixed in the same pass: section
1's test bullet now matches AC1, U22 is explicitly the pure half (the wiki's own U22 title marks
the rest a `gen-backdrop` test), and the wiki `Data` section 4 amendment is listed as its own
doc follow-up rather than an action in this branch.

No CRITICAL item was reported in either iteration, so nothing was removed for being
ticket-unsupported.

### Addendum: post-gate correction (same day)

Reading `Math` section 2's sketch and section 7's draw call side by side showed the plan's
first draft had paraphrased the algorithm wrongly: it invented `art_screen_left` /
`art_screen_top` in `Placement` where the sketch and U23 both say `first_copy_left_px` /
`screen_top_px`, used `Depth::Infinity` for `Depth::Infinite`, and dropped the
`rem_euclid(..) - period` shift that puts `first_copy_left` in `[-period, 0)`. All three are
now verbatim from the sketch, and the placement function is copied line for line from it.

A third scoring pass after the correction returned 92 again, with the `Placement` rename
reported resolved and two new items raised, both fixed: section 8 now states how
`from_legacy_manifest` fills `stack_key`, `palette_key` and `ref_view` (parameters plus
`VIEW_H_PX`), and the `Layer` -> `ResolvedLayer` rename moved into section 9's callout table.

Lesson worth keeping: a plan that paraphrases a cited code sketch can diverge from it in ways
the gate cannot see, because the gate reads the plan, not the sketch. Copy the sketch.

Lessons for the playbook (`<plan-docs-dir>/.rsi_memory/plan-ticket-gate.md`):
- A ticket AC that names a test range (`U19-U23`) is a per-test contract: enumerate every ID in
  the plan's test map and again in the exit conditions, or the gap only shows up in the gate.
- A plan item that edits a shared file for a formatter or a helper reads as scope even when the
  ticket asks for the output: put it in the new module instead.
- When the ticket's stated signature cannot express what the ticket's own acceptance rules
  need, say so in one visible table row with the ticket line it serves, not in prose.

### Amendment 2026-10-11 (tagged wire form), gate iteration 3

The 2026-10-11 amendment retired the bare `"inf"` token in favour of the tagged object
`{ "kind": "finite", "value": n }` / `{ "kind": "infinite" }` (user directive; the ticket
Scope line was amended and a comment posted the same day). The plan was rewritten in five
places (the `Depth` sketch, the validation rule, the ordering note, U16, the block map) and
block 1/18 (`Depth` `Deserialize`) was deleted: `Depth` is now a plain `tag`/`content`
derive, so the map is 17 blocks and the check target is 17 `mismatched types` errors.

Iteration 3 (fresh evaluator, same rubric, MIN_SCORE 90) reported no CRITICAL and no
UNCOVERED at 94. Three REFINEMENTS were applied in the same pass: exit condition 1 now gates
`cargo build -p sstd-core` green (AC4 verbatim), section 9's `validate` row leads with the
inoperability argument (the literal signature cannot check `file_rgba` / `fill_edge_opaque`),
and section 10's U22 row records the AC1 -> U22 -> pure-half chain explicitly.

Wire-form facts verified 2026-10-11 against serde 1.0.229 (scratch `tagprobe` probe): a unit
variant serializes with no `value` key; an unintended `value` on the `infinite` tag and an
unknown `kind` are rejected; the derive needs no `alloc` feature (the earlier claim was
wrong); kind-first order buffers nothing.
