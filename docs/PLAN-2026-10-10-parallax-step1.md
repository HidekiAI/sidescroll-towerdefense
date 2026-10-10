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

Cost: the wiki wording needs a one-line amendment (`Data` section 4 gains the "facts are
supplied by the caller" sentence, `Math` section 2's signature sample is already correct).
Tracked in section 9.

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
pub const REF_VIEW_W_PX: i32 = 1920;    // Data 1: ref outer view the art was composed for
pub const REF_VIEW_H_PX: i32 = 1080;
pub const MAX_LAYERS: usize = 7;        // Data 4: parallax.max_layers, never seeded in config

// model (serde)
pub enum Depth { Finite(f64), Infinity }
pub enum Plane { Back, Front }
pub enum Fill { Transparent, Rgba(u8, u8, u8, u8) }
pub struct Layer { .. }                 // exactly Data 1's seven fields plus name/file/plane
pub struct Stack { .. }                 // schema_version, generator_seed, layers: Vec<Layer>

// resolved, validated form
pub struct ResolvedLayer { .. }
pub struct ResolvedStack { layers: Vec<ResolvedLayer> }
pub struct LayerArt { .. }              // section 3.1

// view / geometry
pub struct ViewState { cam_center_world_px_x, cam_center_world_px_y, zoom, view_w_px, view_h_px }
pub struct Anchor { world_px_x, world_px_y }
pub struct Placement { art_screen_left: i32, art_screen_top: i32, copy_count: u32, scale: f64 }
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

Deliberate deviations from the names in `Math` section 2 (which is a sample, not a contract):

- `Placement` gains `art_screen_left`/`art_screen_top` and drops `first_copy_left` /
  `screen_top_px`. The sample's names describe the pre-repeat copy, but the drawing step
  (`Math` section 7) needs the *art* left plus the copy count, because
  `draw_set_transform(Vector2(first_copy_left_px, screen_top_px))` then
  `draw_texture_rect(0,0, period_px*copy_count, art_h)` — the single call the design wants.
  Keeping both names for one number invites the two to drift.
- `place_layer` takes `&ResolvedLayer`, not `&Layer`: the rate, the period and the art height
  are derived once in `resolve`, not per frame. `ResolvedLayer` / `ResolvedStack` are derivation
  caches, not ticket-mandated types — the ticket names `Layer`, `Stack`, `ViewState`, `Anchor`
  and `Placement`, and those all exist here unchanged.
- U23's assertion names `first_copy_left_px` / `screen_top_px`; under the names above it asserts
  `art_screen_left` / `art_screen_top` are integers, plus `cam_center` on the zoom grid.

## 5. Algorithm: `place_layer`

Verbatim from `Math` section 2, per axis, with the names of section 1:

```
rate_x        = if Infinity { 0 } else { 1 / (1 + parallax_depth) }
rate_y        = rate_x * vertical_damping
layer_cam     = anchor + (cam_center - anchor) * rate
art_left_world = anchor_x + ref_screen_left_px - REF_VIEW_W_PX / 2
art_screen_left = (art_left_world - layer_cam_x) * zoom + view_w_px / 2
art_screen_top  = (art_top_world  - layer_cam_y) * zoom + view_h_px / 2
if repeat_x:
    period_screen = period_px * zoom
    first_copy    = fposmod(art_screen_left, period_screen)      // = rem_euclid
    copy_count    = ceil((view_w_px - first_copy) / period_screen)
else:
    copy_count    = 1
return Placement { snap(art_screen_left), snap(art_screen_top), copy_count, zoom }
```

Notes that matter for correctness and that a reader will otherwise get wrong:

- The repeat is computed on the **unsnapped** left, and only the returned value is snapped
  (`Math` section 3's zoom-0.25 example: `copy_count` is always 5 there).
- `rem_euclid` is the `fposmod`. It returns the sign of the divisor, so `first_copy` is in
  `[0, period)`; `copy_count` is then `ceil((view_w - first_copy)/period)`, which covers
  `[0, view_w)`.
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
| `parallax_depth > -1`, or the string `"inf"` | `parallax_depth` |
| `vertical_damping` in `[0, 1]` | `vertical_damping` |
| `plane = back` needs depth `>= 0`; `plane = front` needs `-0.5 <= depth < 0` | `plane_depth` |
| within a plane, `draw_order` unique | `draw_order_unique` |
| ascending `draw_order` has non-increasing depth on the back plane (base exempt) | `draw_order_depth` |
| layer count `<= MAX_LAYERS` | `max_layers` |
| the lowest-`draw_order` back layer is fully opaque (PNG + both fills) | `base_opaque` |
| a non-`transparent` fill needs that edge row fully opaque | `fill_edge_opaque` |
| `file` is an RGBA PNG | `file_rgba` |
| `LayerArt` count equals `layers.len()` | `art_count` |

`LayerArt` carries no filename, so "file exists" is not checkable here and is a **step 6**
(loader) and **step 2** (generator) concern; `file_rgba` covers the encodable half of the
rule. Recorded in section 9 so it is not silently dropped.

Ordering rule detail worth pinning in a test: `"inf"` counts as the largest depth, and the
base layer (lowest `draw_order` of the back plane) is exempt from the ordering rule, which is
what lets a `Sun` at `"inf"` sit above the opaque `Sky` and below the rest.

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

The twelve rates of the on-disk manifest (`editor/assets/backdrop_layers/manifest.json`,
6 layers x 2 axes) reproduce the `Data` section 1 depths and dampings to within `1e-4`: e.g.
Sky `0.10` -> `parallax_depth 9`, `0.08/0.10 -> 0.8`; Hills `0.50 -> 1`, `0.42/0.50 -> 0.84`;
Foreground `1.30 -> -0.2308`, `1.15/1.30 -> 0.8846`. That file is the U8 fixture.

`ref_screen_left_px` / `ref_screen_top_px` are **not** derivable from the legacy manifest (the
legacy framing lived in `backdrop_preview.tscn`, not in the manifest) — `Data` section 1 derives
them from the old design frame. So `from_legacy_manifest` sets them to the derived band values
(`old_sprite_y - 240`) and the sun disc is a separate emitted layer (step 2). Recorded so
nobody expects the migration to invent the foreground framing.

## 9. Divergences from the ticket's literal wording, and possible follow-ups (not this ticket)

Every row below is a phrasing difference from the ticket or the wiki, not added scope. Each
is stated so the gate can see it, with the ticket line it still serves. Read this table as the
single callout for "where the plan departs from the ticket's literal wording": two API-shape
changes (`validate` takes the PNG facts; the screen size is a constant rather than a
parameter), one placement-field rename, and two derived helper types.

| Item | Tickets line it serves | Action |
|---|---|---|
| Ticket Scope reads `validate(&Stack) -> ValidationResult`; the plan writes `validate(&Stack, &[LayerArt])` | same ticket line; the extra argument is the PNG facts that line's rules are written against | stated in section 3.1; needs the one-sentence wiki amendment below |
| Ticket Scope reads the screen size is "passed in (1920 x 1056)"; the plan makes it the constants `SCREEN_W_PX` / `SCREEN_H_PX` and passes the *view* size in `ViewState` | same ticket line, and its stated anti-goal: never read the stale `Screen::default()` / `GridConfig::default()` | `R1` fixes the screen at 1920 x 1056, so a parameter would be false flexibility; the anti-goal is met either way |
| Ticket Scope reads `place_layer(view, anchor, layer) -> Placement`; the plan passes a `ResolvedLayer` and adds `ResolvedLayer` / `ResolvedStack` | same ticket line | the rate, period and art height are derived once in `resolve` instead of per frame; same function, same inputs |
| `Fill::from_edge_row` | AC `U22` (the fill equals the edge-row mean, else transparent) | step 1's U22 test is the **pure half only**, permitted by the wiki's own U22 title, which marks the emitted-layer and sky-haze assertions as a `` `gen-backdrop` `` test; that half is #105 |
| The journal formatters (`journal_line`, `anchor_clamp_line`) | AC `validate ... carrying field and value`; `Data` line 146; U7, U19 | they live in `parallax.rs`, so no file outside the new module changes |
| `Data` section 4 implies `validate` reads the PNG | the two rule rows it states (`file_rgba`, `fill_edge_opaque`) | the code side is section 3.1; the wiki sentence is corrected by its own doc follow-up, not by this branch |
| `Math` section 2's `Placement` field names | the placement function's return value | renamed in section 4, for the reason given there |
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
| U16 | `Moon` at `"inf"` | same as U3, name not special-cased |
| U17 | `LayerArt` with one alpha-0 pixel in the bottom row | rejected with `rule=fill_edge_opaque`; same with `transparent` loads |
| U19 | anchor `(5000, -300)` then `(960, 1000)` | clamped to `(960, 540)`, one `anchor_clamp_line` |
| U20 | two-row map, player at y `3000` then `-500` | camera clamp |
| U22 | synthetic RGBA edge rows | `Fill::from_edge_row` returns the mean, else `Transparent`; the emitted-PNG and sky-haze half is #105 |
| U23 | fractional player position | camera snap; `art_screen_left`/`art_screen_top` integers |
| U12 | - | blocked until the Phase 2 simulator exists (ticket says so) |
| U18 | - | not this step: `gen-backdrop` test, excluded by the AC line (#105) |

The order matters: U4 and U17 assert that the *pure* path and the *validated* path disagree by
design, so each test names which one it exercises.

Each test is trusted only after its mutation (last column of `Verification` section 2.1) is
applied and seen red. That verification pass is part of this step, not a follow-up.

## 11. Block map (what the coding-assistant hands to @me)

Assistant writes: the module skeleton (constants, struct and field definitions, `serde`
attributes, the `lib.rs` wiring), and the whole `#[cfg(test)] mod tests`. @me writes the logic:

| Block | Where | What to write (not how) | Depends on |
|---|---|---|---|
| 1/M | `parallax.rs`, `Depth` | deserialize a number or the string `"inf"`, reject the rest | - |
| 2/M | `parallax.rs`, `Fill` | deserialize `"transparent"` or `"#RRGGBBAA"`; `from_edge_row` mean | - |
| 3/M | `parallax.rs`, `Layer::rate_x/rate_y` | the rate from depth, damped on y | 1 |
| 4/M | `parallax.rs`, `snap`, `snap_center` | the pixel-grid snap, camera variant | - |
| 5/M | `parallax.rs`, `place_layer` | the section 5 algorithm, repeat included | 3, 4 |
| 6/M | `parallax.rs`, `map_rectangle` | the screen-set bounding box | - |
| 7/M | `parallax.rs`, `camera_bound`, `clamp_center` | the bound rect and the centre clamp | 6 |
| 8/M | `parallax.rs`, `default_anchor`, `clamp_anchor` | the home-view default and the clamp | 6, 7 |
| 9/M | `parallax.rs`, `journal_line`, `anchor_clamp_line` | the two line formats of section 3.2 | - |
| 10/M | `parallax.rs`, `Stack::validate` | the section 6 rule table | 1, 2, 3 |
| 11/M | `parallax.rs`, `Stack::resolve` | validate, then build the resolved layers | 10 |
| 12/M | `parallax.rs`, `Stack::from_legacy_manifest` | the section 8 table | 1, 3 |

Twelve blocks, dependency-ordered. Approval of this map is required before any marker is
inserted (coding-assistant Phase 1 step 3).

## 12. Exit conditions for this step

1. `cargo test -p sstd-core parallax` (the ticket's command) green, with U1-U11 and U13-U17,
   U19-U23 present and named as above; the full `cargo test -p sstd-core` stays green too.
2. Every U-test named in condition 1 shown to fail under its listed mutation.
3. `cargo clippy -p sstd-core -- -D warnings` and `cargo fmt --check` clean.
4. `cargo test -p sstd-core` still reports the pre-existing 110 tests plus the new ones, no
   test deleted or weakened.
5. No Godot file touched; no new dependency in `crates/sstd-core/Cargo.toml`; no file outside
   `src/parallax.rs` and `src/lib.rs` edited.
6. The four GDScript suites unchanged (this step cannot affect them).

## 13. Plan gate

Run 2026-10-10 against #104 (ticket fetched in full, zero comments). Independent evaluator,
`MIN_SCORE` 90, two iterations, same evaluator session for both so the scores are comparable.

| Iteration | Score | Outcome |
|---|---|---|
| 1 | 86 | below the bar: U22 had no plan item, and section 12's exit list contradicted AC1 |
| 2 | 92 | pass |

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

Lessons for the playbook (`<plan-docs-dir>/.rsi_memory/plan-ticket-gate.md`):
- A ticket AC that names a test range (`U19-U23`) is a per-test contract: enumerate every ID in
  the plan's test map and again in the exit conditions, or the gap only shows up in the gate.
- A plan item that edits a shared file for a formatter or a helper reads as scope even when the
  ticket asks for the output: put it in the new module instead.
- When the ticket's stated signature cannot express what the ticket's own acceptance rules
  need, say so in one visible table row with the ticket line it serves, not in prose.
