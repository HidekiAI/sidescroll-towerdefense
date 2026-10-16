# PLAN 2026-10-15 — Step 3 (#106): the bridge exposes `parallax_placements`

Epic #68, step 3 of 7. Steps: 1 #104 (`sstd-core::parallax`), 2 #105 (`gen-backdrop` emits
`stack.json`), **3 #106 (this plan)**, 4 #107 (`WorldView` + `ParallaxDraw` in the Simulator),
5 #108 (Map Editor live parallax), 6 #109 (world archive `backdrop.json` + legacy reader),
7 #110 (per-corridor stack and anchor, deferred).

Design of record: `TDD_Parallax-Scrolling` and its `.Math` / `.Data` / `.Verification`
sub-pages. Plans of record for the predecessor steps: `PLAN-2026-10-10-parallax-step1.md`,
`PLAN-2026-10-13-parallax-step2.md`.

## 1. Objective and ticket trace

Add one JSON-in / JSON-out `#[func]` to the GDExtension bridge so GDScript can ask
`sstd-core::parallax` for this frame's placements, without a second implementation of the math
and without a new transport. The bridge is already the Godot↔Rust seam
(`validate_screen`, `import_screen`); this is one more method on it.

| Ticket item | Where this plan satisfies it |
|---|---|
| S1 `#[func] fn parallax_placements(&self, request_json: GString) -> GString`, same shape as `validate_screen` / `import_screen` | §6.2 (the `#[func]`), §6.3 (the pure `*_impl`), §4.6 (response shape) |
| S2 input `{ view, anchor, stack, plane }`; output `{ placements: [...] }` or `{ error }` | §4.6, §5 — **plus one documented refinement: the request also carries `art`**, §4.1. The input list was amended on the ticket for this (comment 2026-10-15, `#issuecomment-6205748430`), because the input as originally listed is not implementable; see §4.1 |
| S3 optional companion `validate_stack(json)`, matching `validate_screen` | §4.4 — deliberately not added, and why |
| S4 no file loading here; the archive reader (step 6) owns loading and fallback | §3 (the bridge opens no file; the art facts arrive in the request), §2 (out of scope) |
| A1 "GDScript can call `SstdBridge.parallax_placements(...)` and receives the same values `place_layer` produces" | **both halves.** "receives the same values" → §6.3 calls `place_layer` itself; tests B1/B6 (§7) assert equality against a direct `place_layer` call. "GDScript can call" → the new `editor/tests/test_parallax_bridge.gd` (§2, §7; test function `_test_bridge_method_is_callable_and_matches_core`) calls the method from GDScript and asserts the response |
| S2's `plane` request field | §4.5 (the filter), tests B2/B5 (§7) |
| Ticket's "Design of record: `TDD_Parallax-Scrolling` and its `.Math` ... sub-pages", for the draw order | §4.5's sort — Math §7 draws ascending `draw_order` within a plane, so the response order is part of the design of record, not an invention |
| Ticket's "*(G1 depends on this)*" | §7's `bridge_method_is_callable_and_matches_core` test. G1 is the design-of-record verification item ("`ParallaxDraw.last_placements()` equals the bridge result for the same `ViewState`") owned by step 4 / #107; this step makes it computable and proves the call path it needs, while G1 itself is instantiated in #107 |
| A2 "An invalid request returns `{ error }`, never a crash" | §4.3 (the request guards) plus tests B4 and B3 (§7) — B4 covers the request-level guards, B3 the core-owned `stack_invalid` path |
| A3 "`cargo build -p sstd-editor-bridge` passes and the bridge regenerates into `editor/rust/`" | §8 exit conditions |
| Depends on step 1 | §3 reads only step 1's public API; `sstd-core` is not modified by this step (§2) |
| Consumed by step 4 | §4.2 (`file`), §4.6 (fills and `period_px`/`art_h_px`) make the response enough for `ParallaxDraw._draw()` to draw with no recomputation (Math §7) |

## 2. Scope

**In:**

- `crates/sstd-editor-bridge/src/lib.rs`:

  - the `#[func] fn parallax_placements`
  - a pure `fn parallax_placements_impl(request_json: &str) -> String`
  - bridge-local serde structs for the two core types that have no serde derives
  - tests in the existing `#[cfg(test)] mod tests`

- a new `editor/tests/test_parallax_bridge.gd` (about 40 lines): the GDScript half of
  acceptance criterion A1, which no Rust test can prove

- `crates/sstd-editor-bridge/Cargo.toml`: a `serde` derive dependency, added in the
  implementation (the bridge-local request structs need the macros and the bridge carried only
  `serde_json`). Recorded as deviation 1 in §10.

**Out** (each deliberate, each with its reason):

- **`sstd-core` is not touched.** Step 1 froze its API; this step consumes it. If a
  placement needs a fact core does not expose, the bridge composes it (§4.2) rather than
  widening core.
- **No file I/O.** No PNG is opened and no stack file is read: art facts arrive in the
  request (§4.1). The archive reader (step 6) owns loading and fallback.
- **No GDScript, no scene.** Step 4 owns the draw. The one GDScript file in scope is the
  A1 test above, which draws nothing.
- **No `validate_stack` companion** (§4.4).
- **No story-level items**: no anchor default, no anchor clamping, no map rectangle. The
  request carries an anchor; a caller that needs the default computes it (step 6, `Data`
  §3). `default_anchor`/`clamp_anchor`/`map_rectangle` stay unused by this step.

## 3. What this builds on (existing code, read before writing)

From `crates/sstd-core/src/parallax.rs` (all `pub`, all re-exported by `sstd-core::lib`):

| Item | Why this plan needs it |
|---|---|
| `Stack` (`#[derive(Serialize, Deserialize)]`, field `layers: Vec<Layer>`) | the request's `stack` deserializes straight into it; `validate`/`resolve` are methods on it |
| `Layer` (has `name`, `file`, `plane`, `draw_order`) | `file` comes from here (§4.2) |
| `RefView`, `Anchor` (both `Serialize + Deserialize`) | `Anchor` deserializes straight from the request |
| `Plane` (`#[serde(rename_all = "lowercase")]`, `Back`/`Front`) | the request's `plane` deserializes straight from it |
| `LayerArt` (**no serde derives**) | forces the bridge-local input struct (§4.1) |
| `ViewState` (**no serde derives**) | forces the bridge-local view struct |
| `Fill` (`impl Serialize` → `"transparent"` or `"#RRGGBBAA"`) | fills serialize through this one spelling, never a second one |
| `Stack::resolve(&self, art: &[LayerArt]) -> Result<ResolvedStack, ValidationResult>` | validates then derives; `Err` carries every failure |
| `ResolvedLayer` (`name`, `plane`, `draw_order`, `rate_x`, `rate_y`, `repeat_x`, `period_px`, `art_h_px`, `ref_*`, `fill_above`, `fill_below`) | the per-layer facts the response reports |
| `place_layer(&ViewState, &Anchor, &ResolvedLayer) -> Placement` | **the** math; A1 requires the bridge call it, not re-derive it |
| `Placement` (`first_copy_left_px: i32`, `copy_count: u32`, `screen_top_px: i32`, `scale: f64`) | the four values `place_layer` produces |
| `ValidationResult::has_errors()` / `.messages`; `ValidationMessage` (`Serialize`) | the failure path serializes the messages directly |

From `crates/sstd-editor-bridge/src/lib.rs` (the patterns this must match):

- the `#[godot_api] impl SstdBridge` block (`:38`) holds thin `#[func]` wrappers that delegate
  to pure `*_impl` functions — `validate_placement` (`:41`) → `validate_placement_impl` (`:574`).
- error JSON shapes in use: `{"error": "<text>"}` (`import_screen` `:78`,
  `validate_placement_impl` `:585`) and `{"valid": false, "errors": [<messages>]}`
  (`validate_screen` `:91`).
- `#[cfg(test)] mod tests` at `:1075` (20 tests) — plain `#[test]` fns, no Godot runtime.
- `serde_json` 1 is already a dependency (`Cargo.toml`), as is `sstd-core`.

## 4. Design decisions

### 4.1 The request also carries `art`, and that is required, not scope creep

`Stack::resolve` takes `&[LayerArt]` — the measured PNG facts. They are needed **inside** the
math the ticket asks for, not only by the drawer: `period_px` is the PNG width
(`Data` §1) and it feeds the repeat in `Math` §2 (4), which `place_layer` computes to produce
`first_copy_left_px` / `copy_count`. So the facts cannot be deferred to the GDScript draw.

The ticket's input sketch lists `{ view, anchor, stack, plane }`. **That list was amended on the
ticket the same day** (comment 2026-10-15, `#issuecomment-6205748430`), because the input as
originally listed cannot be implemented: the acceptance criterion "receives the same values
`place_layer` produces" is a `ResolvedLayer`-based requirement, and a `ResolvedLayer` only
exists after `Stack::resolve(art)`. The authoritative design already assigns the missing piece
a home:

- `PLAN-2026-10-10-parallax-step1.md` §3.1, "The PNG facts are an input, not something
  `sstd-core` reads": *"Who measures them: `gen-backdrop` (step 2), and the loader in step 6.
  **The bridge (step 3) passes them through.**"*
- `Data` §1: *"Art size is read from the PNG, never stored, so it cannot drift."*
- Ticket S4: *"No file loading here."*

**Decision:** the request carries `art`, a JSON array index-aligned with `stack.layers`,
whose entries are exactly `LayerArt`'s six fields. The bridge deserializes it and passes it
through to `resolve` — it measures nothing and opens nothing.

Rejected alternatives:

- **the bridge loads the PNGs** (adds an `image` dependency and filesystem policy to a bridge
  whose ticket says "No file loading here", and re-introduces in the bridge exactly the
  dependency `sstd-core` refused in step 1 §3.1);
- **`period_px`/`art_h_px` are omitted and the drawer uses `texture.get_width()`** — does not
  work: the repeat math needs the period *before* the response can be produced.

`LayerArt` and `ViewState` have no serde derives, and `sstd-core` is not modified (§2), so the
bridge owns two small `#[derive(Deserialize)]` input structs and maps them.

### 4.2 `file` is added to each placement, by index

`ResolvedLayer` carries no `file` (core has no reason to know a filename; `Layer` does).
Without it the response is not enough for `ParallaxDraw` to pick a texture and the drawer
would need a second `name → file` lookup.

`resolve()` builds its layers with `self.layers.iter().zip(art.iter())` and filters nothing, so
`stack.layers[i]` and `resolved.layers[i]` are the same layer, and the lengths are equal
(validation enforces that). The bridge zips the two by index.

### 4.3 Request guards (a trust boundary)

A malformed request must produce `{"error": ...}` and never a crash or a silently wrong draw
(ticket A2). Guards, each returning a distinguishable message:

| Guard | Without it |
|---|---|
| JSON parses (`PlacementsRequest`) | serde error text is the message |
| `zoom` is finite and `> 0` | `rem_euclid(period * 0)` is NaN, and `NaN as u32` saturates to `0` — an empty draw, not a diagnosable error |
| `view_w_px` / `view_h_px` finite and `> 0` | negative or zero extents flow into `copy_count` and the snap |
| every `art[i].width_px > 0` | the same NaN class as `zoom == 0`: a `repeat_x` layer with `period_px = 0` makes `place_layer` evaluate `rem_euclid(0.0)` → NaN → `0`, a silently wrong draw. Core has **no** width rule, so nothing upstream catches this one |
| every `art[i].height_px > 0` | `art_h_px: 0` is reported to the drawer, which then draws the art and both fills as zero-height rectangles — the layer loses its cover with no error anywhere |

**`art`'s length is deliberately not guarded here.** Core already owns it completely:
`Stack::validate` emits `art_count` whenever `art.len() != self.layers.len()`
(`parallax.rs:639-646`, both directions), and `resolve` returns `Err` before it zips. A
mismatch therefore reaches the caller as §4.6's `stack_invalid` body carrying core's own
message. A bridge-level duplicate would give one failure two wire spellings for no gain.

These are input validation at a trust boundary (GDScript-produced JSON), which is explicitly
not something to simplify away.

### 4.4 `validate_stack` is deliberately not added

The ticket calls it "optional". `parallax_placements` already validates — `Stack::resolve`
returns `Err(ValidationResult)` carrying **every** failure — and §4.6 reports those messages
to the caller. It has no consumer of its own, so it is not built now; if a caller ever needs
to validate a stack without placing it, it is a few lines on top of §6.3's steps 1-3.

### 4.5 Plane filter and draw order

`plane` is the ticket's request field (S2). When present, only that plane's layers are
returned. When absent the response carries every layer — an accepted **superset** of the
ticket's input list, so a caller that wants both planes does not have to ask twice.

The response is sorted by (plane: `back` before `front`, then `draw_order` ascending) rather
than left in the order `stack.json` happens to author. The order is part of the design of
record the ticket names — `Math` §7 draws ascending `draw_order` within a plane — so the
response carries it instead of each drawer re-deriving it. One `sort_by_key`.

### 4.6 Response shape

```json
{ "placements": [ { "name", "file", "plane", "draw_order",
                    "first_copy_left_px", "copy_count", "screen_top_px", "scale",
                    "period_px", "art_h_px", "fill_above", "fill_below" } ] }
```

- **invalid stack** → `{"error": "stack_invalid", "errors": [<ValidationMessage>...]}` — the
  `errors` key and the serialized message objects match `validate_screen` (`:91`). An
  `art`-length mismatch arrives by this path too, as core's `art_count` message (§4.3), so the
  failure has exactly one spelling on the wire.
- **bad request** (§4.3) → `{"error": "<detail>"}` — matches `import_screen` (`:78`).

Fills are serialized through the existing `impl Serialize for Fill`, so the wire spelling
(`"transparent"` / `"#RRGGBBAA"`) has one authored spot and the drawer parses one form.

## 5. The wire truth (a worked request/response)

Request (a 2-layer fixture, abbreviated for the page; the real emitted stack has 7):

```json
{ "view":   { "cam_center_world_px_x": 960.0, "cam_center_world_px_y": 540.0,
              "zoom": 1.0, "view_w_px": 1920.0, "view_h_px": 1080.0 },
  "anchor": { "world_px_x": 960.0, "world_px_y": 528.0 },
  "plane":  "back",
  "stack":  { "schema_version": 2, "stack_key": "fixture", "palette_key": "neutral",
              "generator_seed": 1, "ref_view_px": { "width": 1920.0, "height": 1080.0 },
              "layers": [ { "name": "Sky", "file": "sky.png", "plane": "back", "draw_order": 0,
                            "parallax_depth": { "kind": "finite", "value": 9.0 },
                            "vertical_damping": 0.8, "repeat_x": true,
                            "ref_screen_left_px": 0.0, "ref_screen_top_px": 0.0,
                            "fill_above": "#8896AAFF", "fill_below": "#C2CAD0FF" } ] },
  "art":    [ { "width_px": 1920, "height_px": 1080, "is_rgba": true, "fully_opaque": true,
                "top_row_opaque": true, "bottom_row_opaque": true } ] }
```

Response — every numeric field is exactly what `place_layer` returns for the same
`(view, anchor, resolved layer)`:

```json
{ "placements": [ { "name": "Sky", "file": "sky.png", "plane": "back", "draw_order": 0,
                    "first_copy_left_px": 0, "copy_count": 1, "screen_top_px": 0, "scale": 1.0,
                    "period_px": 1920.0, "art_h_px": 1080.0,
                    "fill_above": "#8896AAFF", "fill_below": "#C2CAD0FF" } ] }
```

At the anchor, registration (U1) puts the art left at `ref_screen_left_px = 0` and the art
top at `0`, and `copy_count` is 1 for a 1920-wide period in a 1920-wide view — which is the
`Math` §2 sanity case rather than a coincidence.

Failure, a stack whose deepest layer is not opaque:

```json
{ "error": "stack_invalid",
  "errors": [ { "severity": "error", "code": "sky_not_opaque",
                "message": "Sky must be fully opaque", "field": "file", "value": "sky.png" } ] }
```

(the exact codes/messages are core's, step 1's; the bridge reports them unchanged.)

## 6. Implementation map

### 6.1 Bridge-local types (in `lib.rs`, above their first use)

```rust
#[derive(Deserialize)] struct ViewInput { cam_center_world_px_x: f64, cam_center_world_px_y: f64,
                                          zoom: f64, view_w_px: f64, view_h_px: f64 }
impl ViewInput { fn to_view_state(&self) -> ViewState { ... } }

#[derive(Deserialize)] struct LayerArtInput { width_px: u32, height_px: u32, is_rgba: bool,
                                              fully_opaque: bool, top_row_opaque: bool,
                                              bottom_row_opaque: bool }
impl LayerArtInput { fn to_layer_art(&self) -> LayerArt { ... } }

#[derive(Deserialize)] struct PlacementsRequest { view: ViewInput, anchor: Anchor,
                                                  stack: Stack, art: Vec<LayerArtInput>,
                                                  plane: Option<Plane> }

#[derive(Serialize)] struct PlacementEntry { name: String, file: String, plane: Plane,
                                             draw_order: i32, first_copy_left_px: i32,
                                             copy_count: u32, screen_top_px: i32, scale: f64,
                                             period_px: f64, art_h_px: f64,
                                             fill_above: Fill, fill_below: Fill }

#[derive(Serialize)] struct PlacementsResponse { placements: Vec<PlacementEntry> }
```

### 6.2 The `#[func]`, beside `validate_screen`

```rust
#[func]
fn parallax_placements(&self, request_json: GString) -> GString {
    GString::from(parallax_placements_impl(&request_json.to_string()).as_str())
}
```

### 6.3 The pure impl, beside `validate_placement_impl`

`fn parallax_placements_impl(request_json: &str) -> String`, in order:

1. `serde_json::from_str::<PlacementsRequest>` → on `Err`, `{"error": "<serde text>"}`.
2. the §4.3 guards → on failure, `{"error": "<detail>"}`.
3. map `art` to `Vec<LayerArt>`; `stack.resolve(&art)` → on `Err(result)`,
   `{"error": "stack_invalid", "errors": <serialized result.messages>}`.
4. zip `stack.layers.iter().zip(resolved.layers.iter())`, drop entries whose plane is filtered
   out, and for each keep `place_layer(&view, &anchor, resolved_layer)` plus `layer.name`,
   `layer.file`, `resolved.plane`, `resolved.draw_order`, `resolved.period_px`,
   `resolved.art_h_px`, `resolved.fill_above`, `resolved.fill_below`.
5. sort by (plane rank, `draw_order`); serialize `PlacementsResponse` with
   `serde_json::to_string`.

## 7. Test map and mutations

All tests are plain `#[test]` fns on `parallax_placements_impl` (no Godot runtime, like the 20
existing tests). The fixture is a **hand-built** stack string inside the test module.

> **Why not the emitted `editor/assets/backdrop_stacks/neutral_default/stack.json`:** it is
> untracked (step 2's completion record: *"Generated art: produced and verified, intentionally
> not committed"*). A test reading an untracked artifact asserts machine state and fails on a
> clean clone — the "wrong input" falsifiability failure. `include_str!` of an untracked path
> is worse: it breaks `cargo test` on a fresh clone.

| # | Test | Fails when | Mutation that must turn it red |
|---|---|---|---|
| B1 | `placements_match_place_layer_values` — for each placement, `first_copy_left_px`, `copy_count`, `screen_top_px`, `scale` equal a direct `place_layer` call on the same `(view, anchor, resolved)` (A1) | the bridge re-derives rather than calls `place_layer`, or drops a field | add `+ 1` to `first_copy_left_px` at the composition site |
| B2 | `plane_filter_returns_only_that_plane` | the filter is ignored or inverted | delete the `plane` predicate from the filter closure |
| B3 | `invalid_stack_returns_stack_invalid_with_messages` (a `repeat_x: false` layer with `transparent` both sides, a duplicate `draw_order`, and an `art` array one short → core's own `art_count` message) | validation is skipped before `place_layer` | replace the `Err(result)` early return with `unwrap_or_default()` |
| B4 | `bad_request_returns_error_never_panics` — malformed JSON, `zoom: 0`, negative `view_w_px`, `art[i].width_px: 0`, and `art[i].height_px: 0` (A2) | a guard is missing, or `place_layer` is reached with `zoom = 0` / a zero period / a zero height | remove the `zoom > 0` guard, then `width_px > 0`, then `height_px > 0`; each must be caught by the assertion, not by a panic |
| B5 | `placements_are_sorted_by_plane_then_draw_order` (fixture authors the JSON out of order) | the sort is dropped | delete the `sort_by_key` |
| B6 | `response_carries_file_and_period_from_the_matching_layer` — `file` is `stack.layers[i].file`, `period_px == art[i].width_px` | the zip is misaligned or `period_px` taken from `ref_view` | zip `stack.layers.iter().rev()` |

Mutation method and validation follows the repo's recorded practice (step 1/step 2): fire the
substitution, confirm the mutant **parses**, then read the full result — a non-firing or
unparseable mutation is a **void experiment**, not a passed guard.

### `editor/tests/test_parallax_bridge.gd` — the GDScript half of A1

This test is **not** the design-of-record verification item G1 (§1, §7). G1 is step 4's
`ParallaxDraw` comparison; this one only proves the call path exists.

No Rust test can prove "GDScript can call it", because that is a property of the built
extension (`ClassDB` visibility), not of the pure function. It follows the repo's seven
existing `SstdBridge` tests exactly: a `ClassDB.class_exists("SstdBridge")` guard at the top,
then `var bridge: Node = ClassDB.instantiate("SstdBridge")`, then
`bridge.has_method("parallax_placements")` before any call. Its single test function is
`_test_bridge_method_is_callable_and_matches_core`; it builds the same request B1 uses, calls
`bridge.parallax_placements(request)`, and asserts the returned `placements` array's values —
so it covers A1's first half **and** A3's second half (the regenerated `.so` actually carries
the symbol; the `.so` on disk predates this method).

Two traps it must avoid, both recorded in the repo's test-hygiene notes:

- **No silent skip.** The four existing bridge-dependent tests (`test_screen_store.gd:510`
  and siblings) `print("skip: ...")` and continue when the extension is not loaded. That is
  correct for them, but if this test did the same it would pass on exactly the broken state it
  exists to catch. It skips **only** when the extension is absent *as a whole*, prints why,
  and **fails** when the extension is loaded but the method is missing — the case a stale
  `.so` produces, which is the real risk here.
- **Gate on the exit code**, not a printed line (the repo's convention: `quit(0 if failures
  == 0 else 1)`).

It runs after `scripts/build-bridge.sh` in §8, which is where the fresh `.so` comes from.

## 8. Exit conditions

1. `cargo test -p sstd-editor-bridge` — 20 existing + the new B-tests, all green.
2. `cargo test -p sstd-core --tests` — still 134 (this step does not touch core).
3. `cargo build -p sstd-editor-bridge` passes (AC A3, first half).
4. `scripts/build-bridge.sh` regenerates the artifacts under `editor/rust/` (AC A3, second
   half). Only `editor_bridge.gdextension` / `.uid` are tracked; the built `.so` is not.
5. `$HOME/bin/godot4 --headless --path editor --script res://tests/test_parallax_bridge.gd`
   exits 0 and reports a real run, not a skip — this is the proof of A1's "GDScript can call"
   half (§7), and it can only pass against a freshly built bridge.
6. The four GDScript suites (`test_screen_store`, `test_image_to_map`, `test_terrain_brush`,
   `test_override_merge`) and `test_parallax_backdrop.gd` keep their current exit codes (0).
   The bridge is additive, so a change here is a regression.
7. The mutation battery of §7: every new guard has a confirmed red.
8. No change outside `crates/sstd-editor-bridge/src/lib.rs`, the new
   `editor/tests/test_parallax_bridge.gd`, and this plan plus the checkpoint and the wiki TODO
   row appears in the step's diff.

## 9. Plan gate

- 2026-10-15, 3 iterations, final score **93/100** (pass; MIN_SCORE 90). One evaluator session
  for all three iterations (independent agent, given only the ticket text, the plan and the
  rubric — never this session's reasoning).
- Ticket comments read before gating: none existed. **One comment was added during this gate**,
  `#issuecomment-6205748430`, amending the input list to include `art`; it is the ticket line
  §4.1 and §1's S2 row trace to.
- **Iteration 1: 83/100.** No CRITICAL. One UNCOVERED — A1's "GDScript *can call*" half had no
  plan item. Six deficiencies: §4.3's `art.len()` row had a wrong rationale; §4.3 missed the
  `width_px == 0` NaN path; §4.5 and tests B2/B5 had no trace row; `plane`-optional was not
  recorded as a superset; §4.2/§4.6's extra response fields were defended only thinly; §4.4's
  justification over-argued.
- **Iteration 2: 87/100.** All six addressed, but five new items: the plan's new GDScript test
  was labelled `G1`, an existing design-of-record verification item belonging to step 4 — a
  real traceability hazard, since SSTD's own rules ban opaque-label references; §2's "`lib.rs`
  only" contradicted the added test file; `height_px > 0` named no harm; the retained
  `art.len()` guard gave one failure two wire spellings; the comment id was missing from §1's
  S2 row.
- **Iteration 3: 93/100.** No CRITICAL, no UNCOVERED. All five addressed; the `art.len()` guard
  was **removed** rather than re-worded, because core's `validate` owns it completely
  (`parallax.rs:639-646`, `!=`, returns before zipping), which also collapsed the failure to a
  single wire spelling. Residual advisory items (the test-function name, the `height_px` B4
  case, B3 in §1's A2 row, the extent-guard wording) **were applied as a post-pass tidy**, so
  the scored text differs from this one only in those five small ways.
- **Nothing was moved to a follow-up list**: no iteration produced a CRITICAL, so no plan item
  was ever found unsupported by the ticket.

## 10. Completion record

Landed 2026-10-16 as `feat(parallax): expose parallax_placements over the GDScript bridge (#106)`.
Everything in §6 and §7 shipped as designed, with the deviations below.

### What the gates measured

| Gate | Result |
|---|---|
| `cargo test -p sstd-editor-bridge` | 26 passed (20 existing + 6 new), rc 0 |
| `cargo test -p sstd-core --tests` | 134 passed, rc 0 (core untouched, as planned) |
| `cargo build -p sstd-editor-bridge` | rc 0 |
| `scripts/build-bridge.sh` | rc 0; `editor/rust/libsstd_editor_bridge.so` regenerated |
| `test_screen_store` / `test_image_to_map` / `test_terrain_brush` / `test_override_merge` | rc 0, `failures=0` each |
| `test_parallax_backdrop` | rc 0, prints `[GREEN]` |
| `test_parallax_bridge` (new) | rc 0, `failures=0`, 16 checks |

### The GDScript test is not vacuous

Run against the **stale** `.so` (the state this checkout was in before the rebuild), it exits
**1** with `FAIL: SstdBridge is loaded but has no parallax_placements (stale .so? run
scripts/build-bridge.sh)`. A skip-on-absent would have passed on exactly the state the test
exists to catch. This is the one check in the step that could not be verified any other way.

Measured placement values for the fixture (`zoom 1.0`, view 1920x1080), quoted from the run:

| Layer | file | plane | draw_order | first_copy_left_px | copy_count | period_px |
|---|---|---|---|---|---|---|
| Base | `base.png` | back | 0 | -1920 | 2 | 1920 |
| Near | `near.png` | back | 1 | -1500 | 3 | 1500 |
| Fore | `fore.png` | front | 0 | -640 | 4 | 640 |

The authored order was Near, Base, Fore, so the response order proves the sort. `first_copy_left_px`
landing exactly on `-period_px` for every layer is the `rem_euclid(...) - period` step doing what
it should at this camera position.

### Deviations from §2 and §6

1. **`serde` added to the bridge's `Cargo.toml`.** §2's file list named only `lib.rs` and the
   GDScript test. The bridge-local request structs need the derive macros, and the bridge had
   only `serde_json` (which does not re-export the derive). One line, version `"1"` with
   `features = ["derive"]`, matching `sstd-core`'s own spec; the workspace has no
   `[workspace.dependencies]` to inherit from. `Cargo.lock` gained one entry.
2. **`art[i].width_px` / `height_px` are `u32`, not the `f64` §4.1 sketched.** They already are
   `u32` in core's `LayerArt`, and a JSON unsigned integer deserializes into `u32` without a
   lossy cast. The guard is `== 0`, which is the same hazard §4.3 named.
3. **The guard is written `!(view.zoom > 0.0)`, not `zoom > 0.0 && zoom.is_finite()`.** §4.3
   asked for both. A single negated comparison rejects `NaN`, `0` and negatives together, so the
   `is_finite()` clause would have been unreachable belt-and-braces: `serde_json` cannot encode
   `NaN` or `Infinity` from any JSON text, and a reject that cannot fire reads as a guard while
   being dead code.
4. **`plane` defaults to `None`; `anchor` does not default.** §4.1 listed both as optional. A
   default anchor is the loader's decision (step 6, `Data` section 3), so the seam requires the
   caller to state it rather than inventing one here.

### Mutation verification

A 15-mutant sweep (`/tmp/user/1000/opencode/mutate-bridge.sh`) over every new guard, the sort,
the plane filter, the zip alignment, the art mapping, the two error keys and the `art_count`
delegation: **15 confirmed, 0 void, 0 refuted**. Each mutant asserted its substitution fired,
that the change touched a non-comment line, and that the mutant compiled before any result was
read.

**Two harness faults were caught and fixed during that sweep, and both are worth recording:**
the first sweep grepped `^error` to detect a compile failure, but `cargo test` prints
`error: test failed, to rerun pass ...` for a *failing test*, so every real mutant was misread
as a compile error; and the `stack_invalid` mutant's pattern matched the `#[func]` **doc
comment** (line 108) before the code (line 783), so it respelled only the prose and the test
rightly passed. A mutation that edits documentation proves nothing about the code -- the
harness now rejects a mutant whose changed lines are all comments.

### Residual, deliberately not fixed here

- **The wire tokens `"error"`, `"stack_invalid"` and `"placements"` are literals**, in the code
  and in the `#[func]` doc comment. This matches the two neighbouring handlers (`validate_screen`,
  `validate_placement`), which hand-write `"valid"` and `"error"` the same way, and the repo has
  no named form for wire tokens at this seam -- so introducing a one-off constants trio inside
  this file would be the "invent a constant nothing else uses" case the magic-string rule warns
  against. Recorded as a finding for the step that finalizes the wire contract (step 4's drawer
  spells the same tokens from GDScript, and its test is the pin). The consequence, measured by
  the sweep above: respelling a token needs an edit in the code *and* in the doc comment, and a
  stale doc comment is not caught by any test.
- **A pathological `zoom` (for example `1e-300`) inflates `copy_count`**, because
  `ceil(view_w_px / (period_px * zoom))` grows without bound and saturates at `u32::MAX` in the
  drawer's loop. `Math` section 7 documents the editor's zoom-out extreme as 0.25 with at most 5
  copies per layer, and §6 documents zoom as the editor's own camera, not authored data, so this
  is not a trust boundary. No floor guard was added: a documented *example* is not a documented
  *limit*, and a 0.25 floor would reject a legitimate 0.2. The drawer (step 4) is where a
  draw-count cap would belong if it ever matters.
- **`test_screen_store` is known RED on Godot 4.7.2** (issue #85, `ZIPPacker` now emits a
  `tiles/` directory entry). It exited 0 here, matching this session's measured baseline; it is
  not a regression from this step and was not touched.

