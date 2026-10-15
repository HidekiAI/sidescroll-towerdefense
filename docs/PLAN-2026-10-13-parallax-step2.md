# PLAN 2026-10-13 — Step 2 (#105): `gen-backdrop` emits `stack.json`

TL;DR: `tools/gen-backdrop` stops writing the v1 `manifest.json` and builds a
`sstd_core::parallax::Stack`, writing `stack.json` plus 7 PNGs (Sky 1920x1080 without the
sun disc, a new 130x130 `sun.png`, and the existing five art layers) into
`editor/assets/backdrop_stacks/<stack_key>/`. Fills come from the PNGs' real edge rows,
`validate_factors` dies in favour of `sstd_core::parallax::validate`, and the schema
version is single-sourced from a `pub` `MANIFEST_SCHEMA_VERSION`. Closes #90.

## 1. Objective and ticket trace

- Ticket: #105 (`feat(gen-backdrop): emit stack.json with real edge-row fills and a separate sun layer`).
  Closes #90 (the sun must never move on screen). Epic #68. Depends on step 1 (#104) — landed
  (all 17 blocks written, `cargo test -p sstd-core` 134 green).
- Acceptance criteria, one per plan section:
  - "7 PNGs (including `sun.png`) plus `stack.json` at 1920 wide" -> sections 3, 6.
  - "U18: the sun alpha matches the old blend within 1 level at 20 sample points" -> sections 3, 8.
  - "U22: every `fill_above` / `fill_below` equals the real edge-row mean, or `transparent`; the
    sky mean is within 20 levels of the gradient end" -> sections 4, 8.
  - "`sstd-core::parallax::validate` accepts the emitted `neutral_default` stack" -> sections 5, 8.
  - "Tests are mutation-verified" -> section 8.

## 2. Existing structures this builds on (read before coding)

- `sstd_core::parallax` (`crates/sstd-core/src/parallax.rs`):
  - `Stack { schema_version: u32, stack_key: String, palette_key: String, generator_seed: u64,
    ref_view_px: RefView, layers: Vec<Layer> }` — derives `Serialize`/`Deserialize`; this is the
    writer's payload type.
  - `Layer { name, file, plane: Plane, draw_order: i32, parallax_depth: Depth,
    vertical_damping: f64, repeat_x: bool, ref_screen_left_px: f64, ref_screen_top_px: f64,
    fill_above: Fill, fill_below: Fill }` — derives both directions; wire form pinned by the
    step-1 round-trip tests.
  - `Plane { Back, Front }` (`#[serde(rename_all = "lowercase")]` -> `"back"`/`"front"`),
    `Depth { Finite(f64), Infinite }` (tagged `{kind,value}` derive — the `"inf"` string is
    RETIRED; the sun is `Depth::Infinite`), `Fill { Transparent, Rgba(u8,u8,u8,u8) }`
    (hand-written `Serialize`: `"transparent"` / `"#RRGGBBAA"`), `RefView { width: f64, height:
    f64 }`.
  - `LayerArt { width_px, height_px, is_rgba, fully_opaque, top_row_opaque, bottom_row_opaque }`
    — the PNG facts `Stack::validate(&[LayerArt])` needs; gen-backdrop measures these with the
    `image` crate it already depends on.
  - `MANIFEST_SCHEMA_VERSION: u32 = 2` (line 97) — currently file-private. **Must become `pub`**
    so the writer and the reader share one constant (wire-token rule: one authored spot).
  - `Stack::validate(&art) -> ValidationResult` — replaces `validate_factors` entirely.
- `tools/gen-backdrop/src/main.rs` (current):
  - `LayerSpec { name, file, scroll_scale: (f32,f32), height_px, z_index }` — replaced (section 3).
  - `Manifest`/`ManifestLayer` + `build_manifest` + `validate_factors` — deleted.
  - `make_sky` (sun disc baked in: consts `0.68`/`0.24`/`0.06` at lines 352-354), `make_clouds`,
    `make_silhouette`, and the pure staples `lattice_hash`, `value_noise`, `fbm`, `lerp_rgb`,
    `smoothstep` — keep.
  - `generate_layers_and_manifest` (CLI `--out`/`--width`/`--overscan`/`--seed`, default out
    `editor/assets/backdrop_layers`) — reworked (section 6).
- `tools/gen-backdrop/Cargo.toml`: deps `image 0.25`, `serde (derive)`, `serde_json`. Gains
  `sstd-core = { path = "../../crates/sstd-core" }` — the same in-repo path-dep pattern
  `crates/sstd-editor-bridge/Cargo.toml:12` already uses (clone-safe: both members ship in this
  repo's workspace `Cargo.toml`).
- Design of record: wiki `TDD_Parallax-Scrolling.Data` section 1 (the `stack.json` wire form),
  section 4 (validation), section 5 (migration); `TDD_Parallax-Scrolling.Verification` U18/U22;
  `TDD_Parallax-Depth` section 5.2 (the 4-decimal depths/dampings).

## 3. The emitted stack (`stack.json`) — the exact wire truth

Written to `<out>/stack.json` where `<out>` defaults to
`editor/assets/backdrop_stacks/neutral_default`. One `Stack` with `schema_version =
MANIFEST_SCHEMA_VERSION` (via the pub const), `stack_key = "neutral_default"`, `palette_key =
"neutral"`, `generator_seed = <--seed>`, `ref_view_px = { width: 1920, height: 1080 }` (the
parallax constants `VIEW_W_PX`/`VIEW_H_PX`).

Seven layers, in this order (draw_order within plane ascending; matches the step-1 fixture so
`validate` and the fallback agree):

| name | file | plane | draw_order | parallax_depth | vertical_damping | repeat_x | ref left | ref top |
|---|---|---|---|---|---|---|---|---|
| Sky | sky.png | back | 0 | 9 | 0.8 | true | 0 | 0 |
| Sun | sun.png | back | 1 | Infinite | 1.0 | false | 1241 | 194 |
| HighClouds | clouds_high.png | back | 2 | 4 | 0.85 | true | 0 | 760 |
| LowClouds | clouds_low.png | back | 3 | 2.3333 | 0.8333 | true | 0 | 760 |
| Hills | hills.png | back | 4 | 1 | 0.84 | true | 0 | 696 |
| Forest | forest.png | back | 5 | 0.4286 | 0.8571 | true | 0 | 760 |
| Foreground | foreground.png | front | 0 | -0.2308 | 0.8846 | true | 0 | 760 |

- Depth 9 -> rate 0.1, etc.: every row reproduces today's `scroll_scale_x` to within 1e-4
  (U8 provenance, `TDD_Parallax-Depth` section 5.2): 9, 4, 2.3333, 1, 0.4286, -0.2308 are
  `1/rate - 1`; dampings 0.8/0.85/0.8333/0.84/0.8571/0.8846 are `rate_y / rate_x` of the old
  pairs (0.08/0.10, 0.17/0.20, 0.25/0.30, 0.42/0.50, 0.60/0.70, 1.15/1.30). Sun damping 1.0,
  depth Infinite.
- `ref_screen_top_px` is bottom-anchored to the old 1320 px sky, minus 240:
  old sprite y 1000 -> 760 (both clouds, Forest, Foreground), 936 -> 696 (Hills). Sky 0.
- Sun placement derived from the disc constants, not hand-typed: centre
  `(0.68 * 1920, 0.24 * 1080) = (1305.6, 259.2)`, radius `0.06 * 1080 = 64.8`; the 130x130 PNG
  holds the disc centred at `(64.8, 64.8)`; `ref_screen_left_px = round(1305.6 - 64.8) = 1241`,
  `ref_screen_top_px = round(259.2 - 64.8) = 194`.
- Fill values are NOT in this table: they are computed from the emitted PNGs (section 4). The
  wiki example's fill rows (and its `#8A98ACFF`/`#C4CCD2FF` sky guesses) are explicitly
  labelled estimates there ("placeholders on the same footing", Data section 1); the emitted
  file is the truth U22 checks.
- Grounding note (plan-internal, not a deliverable): `RefView` is `f64`, so serde_json writes
  `"width": 1920.0` where the wiki example shows `1920`. The reader parses both; no schema
  change. A custom float serializer would be cosmetics — not in scope.
- Single-source construction (serves the sun-layer scope bullet): the wire tags `"back"`/`"front"`/
  `"finite"`/`"infinite"`/`"transparent"` never appear as literals in gen-backdrop — the code
  constructs `Plane::Back`, `Depth::Infinite` (the sun's depth; the ticket body's `"inf"` string
  predates the tagged-enum decision, so this supersedes it), `Fill::Transparent`, and the tags
  derive from the sstd-core enums. `stack_key`/`palette_key`/file names are data values, not
  parse tokens. The greppable audit of this is a verification-hygiene bar (section 9b), not a
  ticket deliverable.

## 4. Fills: the real edge-row mean (U22)

After each PNG is generated, for every layer compute `fill_above` from row 0 and `fill_below`
from the last row of THAT PNG (post-effect: gradient, haze, clamp, alpha):

- If the row is fully opaque (alpha 255 at every pixel): the per-channel mean over the row's
  width, rounded (`(sum / width).round() as u8`), alpha 255 -> `Fill::Rgba(r, g, b, 255)`.
- Otherwise (any pixel alpha != 255): `Fill::Transparent`.

Why "fully opaque" is the predicate: a fill is that row extended to cover vertical travel and
occludes farther layers like the art (Data section 4), so it must never extend a see-through
edge. This is also what keeps `validate`'s `fill_edge_opaque` rule green by construction.

Which rows are opaque for the emitted art: Sky rows 0 and last are gradient+opaque (alpha 255
everywhere) -> both fills opaque; Sun 130x130 row 0 / last row are outside the disc (alpha 0)
-> both transparent; cloud rows -> not fully opaque -> transparent; Hills/Forest/Foreground
bottom row sits in the opaque ground band (baseline 0.62/0.74/0.88, relief < baseline always)
-> `fill_below` opaque mean, `fill_above` transparent.

Resolved ambiguity (this implements the ticket's fill scope bullet and the U22 AC; it does not
add scope): the Data section 1 example shows `Foreground fill_below: "transparent"`. Under the
uniform U22 rule the Foreground's bottom row is fully opaque, so the emitted value is the
body-colour mean (the row is at the view floor: ref top 760 + height 320 = 1080, so its
fill_below extension is zero-length anyway). The example row is a placeholder, not the rule;
the plan follows U22 uniformly for every layer, no front-plane special case. The Data example
is not edited in this step.

## 5. Validation is sstd-core's, and the emitted stack must pass it

`validate_factors` and its two tests are deleted. In gen-backdrop's tests, build the emitted
stack for a test-size run, measure `LayerArt` from the generated PNGs (width/height, `is_rgba`,
`fully_opaque`, `top_row_opaque`, `bottom_row_opaque`), and assert `Stack::validate(&art).is_valid()`.

Why it passes, rule by rule (sections 3-4 fix the values): schema_version = const 2; layer
count 7 = MAX_LAYERS; art count matches; finite depths all > -1 (Foreground -0.2308), Infinite
passes; dampings all in [0,1]; back layers >= 0 except Sun (Infinite passes), Foreground -0.2308
in [-0.5, 0); draw orders unique per plane; non-increasing depth along back order
(9 -> Sun(Infinite, exempt: Sky is base) -> 4 -> 2.3333 -> 1 -> 0.4286); base_opaque: Sky has
opaque art + opaque fills; fill_edge_opaque: every non-transparent fill sits on a fully-opaque
row by construction. If any of these breaks, the test fails — which is the point: the emitted
fallback stack must never be rejectable (Data section 4).

## 6. Assembly rework

- `LayerSpec` becomes: `name, file, plane: Plane, draw_order: i32, parallax_depth: Depth,
  vertical_damping: f64, repeat_x: bool, ref_screen_left_px: f64, ref_screen_top_px: f64,
  height_px: u32` (fills NOT on the spec — they come from the art).
- `layer_specs()` returns 7 specs per the section 3 table. The Sun row's `ref_screen_*` derive
  from shared disc constants (`SUN_CENTER_FRAC_X = 0.68`, `SUN_CENTER_FRAC_Y = 0.24`,
  `SUN_RADIUS_FRAC = 0.06` — the same three constants today live inside `make_sky`; they move
  out and both `make_sun` and the Sun layer row consume them, so a palette pass cannot drift
  the placement from the disc).
- `make_sky(width, height, seed)`: height supplied by the Sky spec (1080); delete the sun disc
  block (centre/radius/disc lerp). Signature unchanged.
- New `make_sun(ref_height: f64) -> RgbaImage`: `r = SUN_RADIUS_FRAC *
  ref_height`; image size `round(2r)`x`round(2r)` (130x130 at 1080); disc centred at `(r, r)`;
  every pixel RGB `[255, 250, 238]`; `alpha = round(255 * 0.85 * (1 - smoothstep(0.65r, r, dist)))`
  (Data section 1 / U18). `repeat_x` false, so no seam requirement and no `all_layers` seam
  check for it.
- New `edge_row_fill(row: &[Rgba<u8>]) -> Fill` (section 4) and
  `art_facts(image: &RgbaImage) -> LayerArt` (section 5) — both used by the tests, both defined
  here because they are shared with the emit path.
- New `build_stack(specs, images, seed) -> Stack`: fills injected from `edge_row_fill` of each
  layer's generated PNG (the scaffold detail is in the deviations list below).
- `generate_layers_and_manifest` -> keep the name shape (assembly is scaffold, not a block):
  CLI drops `--overscan`, keeps `--out`/`--width`/`--seed`; default out
  `editor/assets/backdrop_stacks/neutral_default`; writes 7 PNGs + `stack.json`
  (`serde_json::to_string_pretty`, trailing newline, same as today's manifest write); journal
  lines updated (per-layer: `plane`, `depth`, `damping`, `ref_screen`, fills; final line names
  `stack.json`). Old `editor/assets/backdrop_layers` output and `manifest.json` writer go away
  (this is the ticket's "stops emitting the v1 manifest.json").
- Document-with-the-change (verification hygiene, section 9b): the module doc header's
  Outputs/Usage lines describe the v1 manifest and 6 layers; they get rewritten with the
  new Outputs (7 PNGs + `stack.json`, no `--overscan`).

## 7. Block map (what the coding-assistant hands to @me)

Assistant writes: the `Cargo.toml` + `sstd-core` dep, the `pub` `MANIFEST_SCHEMA_VERSION`
change, the new `LayerSpec`/disc constants, the assembly rework and the full `#[cfg(test)] mod
tests` scaffold. @me writes the logic:

| Block | Where | What to write | Tests that go green |
|---|---|---|---|
| 1/6 | `make_sun` | the 130x130 disc, alpha formula, RGB const | U18 |
| 2/6 | `edge_row_fill` | the row -> `Fill` predicate (section 4) | U22 |
| 3/6 | `layer_specs` | the 7-row table incl. derived Sun placement | validate-accepts, U8 |
| 4/6 | `make_sky` | remove the sun disc, height from spec | sky_has_no_alpha, seam |
| 5/6 | `art_facts` | `LayerArt` measurement from the PNG | validate-accepts |
| 6/6 | `build_stack` | `Stack`/`Layer` assembly, fills injected | validate-accepts, parse round-trip |

Dependency order: 3 -> 4 -> 1 -> 2 -> 5 -> 6. Each block is one function, independently
checkable; the scaffold provides `TODO(human)` empty bodies (E0308-flagged) exactly as step 1.

### Scaffold deviations from this plan (recorded 2026-10-13, at scaffold time)

TL;DR: four signature/scope adjustments were forced by the real APIs while scaffolding; the
block contracts (1-6) and every test are unchanged. Detail below.

- `make_sun(ref_height: f64)`, not `(ref_width, ref_height)` (section 6): `ref_width` is
  unused in-image — the disc is square, sized from `r = SUN_RADIUS_FRAC * ref_height` — so an
  unused parameter would be clutter (and the unused-arg rule forbids keeping it).
- `build_stack(specs: &[LayerSpec], images: &[(String, RgbaImage)], seed: u64)`, not
  `(specs, art_per_layer, seed, ref_view)`: the fills come from the PNG edge rows
  (`edge_row_fill`), so it needs the images, not pre-measured `LayerArt`; and `ref_view_px` is
  the module const `REF_VIEW_PX` there is exactly one reference view, so passing it would let a
  caller write a second one that drifts.
- `deterministic` keeps NO sun clause (section 8 listed "+ Sun determinism"): `make_sun` has no
  seed, so its output is constant by construction and re-running it cannot prove more than
  comparing the same two constants. The existing `deterministic` test is untouched.
- `ImageBuffer::rows()` yields a `Pixels` iterator per row, NOT a `&[Rgba<u8>]` slice (image
  0.25.10). So block 6 collects each edge row into a `Vec<Rgba<u8>>` before calling
  `edge_row_fill(&row)`, and block 5 uses `rows().next().is_some_and(|row| row.into_iter()...)`.
  The scaffold's block-5/6 samples were corrected to match; `edge_row_fill`'s slice signature
  is kept because it is what makes the fill logic testable without an `image`.
- Added (not in this plan, needed by the assembly): `generate_layer(spec, width, seed) -> Result<RgbaImage, String>`,
  the single by-name dispatch table shared by the emit path; the assembly's inline `match` moved
  in there, so a spec with no generator fails in one place.
- Scope-adjacent, NOT this ticket: `README.md` carries an uncommitted markdown-table reformat
  dated 2026-10-11 (before this branch). It is left unstaged and excluded from every step-2
  commit.

## 8. Test map and mutations

- U18: for the 130x130 sun, at 20 sample points (centre, `0.65r` ring, mid-ring, beyond `r`,
  plus a fixed set of offsets), alpha-over the sun onto a flat `[138, 152, 172]` sky and compare
  against the old `lerp(base, [255,250,238], disc * 0.85)` blend; every channel within 1 level.
  Mutations that must fail: alpha `1 - disc` instead of `0.85 * (1 - smoothstep(...))`; dropping
  the `0.85`.
- U22: for every emitted layer, `fill_above` == mean of emitted PNG row 0 (rounded) when that
  row is fully opaque, else `transparent`; same for `fill_below`/last row; Sky mean within 20
  levels of the gradient end `[138, 152, 172]`. Mutations that must fail: filling with the
  gradient constant instead of the mean; computing the mean over a wrong row with a provably
  different value (e.g. row `height/2` on the sky, mid-gradient); dropping the fully-opaque
  predicate (would emit a mean for cloud rows).
- validate-accepts: emitted test-size stack passes `Stack::validate` with real `LayerArt`;
  plus the emitted `stack.json` string parses back into `sstd_core::parallax::Stack`
  (round-trip through the actual serde impls = the reader). Mutations that must fail: any
  section-3 value changed (a wrong depth, damping, plane, order) -> validate rejects.
- Existing gen-backdrop tests: `seam_is_invisible` (skips `repeat_x: false` layers — the Sun
  is not tiled, so a seam check on it is vacuous; this is test maintenance forced by AC-1's
  7-PNG fixture),
  `sky_has_no_alpha`, `cloud_layers_have_holes`, `relief_is_absolute_not_a_fraction`,
  `deterministic` (no Sun clause — see the deviations list), `lattice_hash_*`,
  `value_noise_stays_in_unit_range` stay;
  `factors_in_unit_range` / `validate_factors_rejects_bad_specs` are deleted with
  `validate_factors`.

## 9. Exit conditions for this step

Ticket deliverables (each maps to an acceptance criterion):

1. `cargo build` (workspace) green.
2. `cargo test -p gen-backdrop` green: U18, U22, validate-accepts, parse round-trip, and all
   surviving art tests (the "tests are mutation-verified" AC; mutation list in section 8).
3. `cargo test -p sstd-core` still 134 green (the only sstd-core change is the `pub` const).
4. Every new U-test proven false under its listed mutation (section 8) — the mutation-verified AC.
5. Real-size run: `editor/assets/backdrop_stacks/neutral_default/` holds exactly 7 PNGs (incl.
   `sun.png`) + `stack.json` at width 1920 — AC-1.

Verification hygiene (plan-internal quality bars, NOT ticket deliverables; none of these opens
a follow-up ticket — they are how this plan verifies its own work):

- `cargo clippy -p gen-backdrop -- -D warnings`.
- No literal wire token in gen-backdrop source (the section-3 greppable audit).
- The emitted `stack.json` has no `"inf"` anywhere (`grep -c '"inf"' <out>/stack.json` == 0) —
  the sun is `Depth::Infinite`, not the retired string.
- Module doc header rewritten (the old Outputs/Usage lines document the v1 manifest the ticket
  retires; see section 6).
- `seam_is_invisible` adapted for the non-tiled Sun (section 8).
- Scope guard: no file outside `tools/gen-backdrop/*` edited (except the single `pub` line in
  `crates/sstd-core/src/parallax.rs`); the four GDScript suites untouched — the v1-manifest
  transcribers (`backdrop_preview.tscn`, `test_parallax_backdrop.gd`) are later steps' work,
  not this ticket's.

## 10. Plan gate

- 2026-10-13, 2 iterations, final score **90/100** (pass; MIN_SCORE 90). Same evaluator session
  both iterations (free long-context model — the preferred paid mid-tier evaluator was
  unavailable, account funds exhausted).
- Iteration 1: **59/100 — hard cap.** CRITICAL items (plan items the ticket does not support):
  `cargo clippy -p gen-backdrop -- -D warnings`; the wire-token grep audit; the module-doc
  header rewrite; the `seam_is_invisible` Sun-skip adaptation; the GDScript/scope guard.
  Disposition: all five were verification bars, not deliverables — moved to the new
  "Verification hygiene" subsection (9b), labelled plan-internal quality bars, none opened a
  follow-up ticket. The RefView `1920.0` note was re-framed as grounding (plan-internal); the
  Foreground fill-below paragraph was re-framed as implementing the fill scope bullet and the
  U22 AC.
- Iteration 2: **90/100, no CRITICAL, no UNCOVERED.** Residual advisory refinements (more
  per-item parenthetical citations beyond the section-1 trace map; verbatim-quote the fill
  bullet in section 4) accepted as-is — the section-1 map already end-to-end-links every AC,
  and re-quoting ticket prose would not change the plan's decisions.
- Ticket amendment: comment posted 2026-10-13 recording that the body's `parallax_depth:
  "inf"` line is superseded by the 2026-10-11 tagged-enum directive; the plan single-sources
  the sun's depth as `Depth::Infinite` (section 3). No scope change.
- Lessons appended to `~/projects/SSTD/.rsi_memory/plan-ticket-gate.md`.

## 11. Completion record (2026-10-15)

All five exit conditions met; the assistant wrote the six blocks per @me's 2026-10-15
no-time-this-week instruction (so the comprehension gate reviews written code). Commit
**`0a3a504`** `feat(gen-backdrop): fill the six step-2 blocks, crate green 28 (#105)`.

| Exit condition | Result |
|---|---|
| 1 `cargo build` (workspace) | green |
| 2 `cargo test -p gen-backdrop` | **28 passed, 0 failed** |
| 3 `cargo test -p sstd-core` | **134 passed, 0 failed** (unchanged) |
| 4 every new U-test false under its listed mutation | **M1-M6 all caught**, real assertions + real values |
| 5 real-size run: 7 PNGs + `stack.json` at width 1920 | met; sky 1920x1080, sun 130x130, others 1920 wide; `grep -c '"inf"'` == 0 |

Verification-hygiene outcomes (section 9b), including the two that could not be met as written:

- `cargo clippy -p gen-backdrop --all-targets` is **clean (0 warnings)**. The bar as written
  (`cargo clippy -p gen-backdrop -- -D warnings`) cannot pass: the package's path dependency
  `sstd-core` is linted too and carries 25 pre-existing warnings in code this plan forbids
  editing, so the bar is read as "gen-backdrop's own targets". Three `needless_range_loop` hits
  were rewritten as `iter_mut()` (one introduced by block 2, two pre-existing: `make_sky`'s haze
  loop and the U22 oracle), and `art_facts` is now `#[cfg(test)]` because the emit path validates
  nothing and the shipped bin otherwise warns on dead code.
- No literal wire token in `gen-backdrop` source; the emitted `stack.json` has no `"inf"`.
- Module doc header rewritten (it already was, at scaffold time).
- `seam_is_invisible` skips `repeat_x: false` layers, so the Sun is not seam-checked.
- Scope guard held: only `tools/gen-backdrop/*` was edited on this branch, plus the single `pub`
  const line in `crates/sstd-core/src/parallax.rs` (landed in step 1). The four GDScript suites
  were not run for step 2 because no editor file changed; `editor/assets/backdrop_layers/` and its
  `manifest.json` consumer are untouched (the v1 transcriber scene is steps 4-5 work).

### Deviation: U18's oracle distance (block 1's sample could not pass)

The scaffolded U18 test evaluated the retired sky's blend at the sample's *nominal* distance
(`dist_fraction * radius_px`) while the sun layer rasterises the disc over integer pixels. The
nearest pixel to a nominal point at `0.85r` — the steepest part of the falloff, sampled at the
diagonal angles (22.5/112.5/202.5/292.5 deg) where the rounding offset is largest — sits ~0.34 px
further out, and `d(composite)/d(dist)` there is ~6.4 levels per px, so the comparison read **2
levels** off against the ticket's 1-level bar.

Measured, not derived — the nominal-distance oracle restored temporarily and only U18 run:

```
thread 'tests::sun_alpha_matches_the_old_baked_disc_within_one_level' panicked at tools/gen-backdrop/src/main.rs:1881:17:
sample dist 0.85r at 22.5deg channel 0: the sun composites to 175 but the old blend was 177
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out
```

The arithmetic behind it: `SUN_RADIUS_FRAC * REF_VIEW_PX.height` gives `r = 64.8` px, so the
nominal point is `(64.8 + 55.08 cos 22.5, 64.8 + 55.08 sin 22.5) = (115.7, 85.9)` and rounds to
pixel `(116, 86)`, whose distance from the centre is `55.415` — **0.34 px further out** than the
nominal `55.08`. That lands `t = 13.295/22.68 = 0.5862` in the smoothstep instead of `0.57143`,
moving alpha from ~85 to `round(255 * 0.85 * 0.372) = 81`, and `(175 - 138) * 255 / 117 = 80.6`
confirms the sampled pixel really carried alpha 81. The same slope gives the 2-level reading:
`216.75 * 6t(1-t) / 22.68 = 14.0` alpha/px, which is `14.0 * 117/255 = 6.4` composite levels/px.

The fix is in the test, not the disc geometry: the oracle is now evaluated at the sampled pixel's
own distance, which is exactly the distance the implementation uses, and the plan's section-3
geometry (disc centred at `(r, r)`, `r = SUN_RADIUS_FRAC * ref_height`) is untouched. Equal
distance is the contract; which pixel it is measured at is whichever is nearest the sample. Both
listed U18 mutations (M1 `1 - disc`, M2 drop the `0.85`) still fail, so the assertion keeps its
discriminating power.

### Generated art: produced and verified, intentionally not committed

`editor/assets/backdrop_stacks/neutral_default/` holds the AC-1 output (7 PNGs + `stack.json`,
~660 KB) in the working tree but is **not** part of the commit: the scope guard above allows only
`tools/gen-backdrop/*`, and the v1 art was committed bundled with the scene that consumes it
(`ba2cbb4`), which for this stack does not exist until steps 4-5. Regenerate with
`cargo run -q -p gen-backdrop`; commit on request.
