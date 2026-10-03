# PLAN: #89 replace camera-driven parallax scrolling with per-layer `scroll_offset`

**Date:** 2026-10-03
**Ticket:** [#89](https://github.com/HidekiAI/sidescroll-towerdefense/issues/89) (OPEN, bug)
**Parent feature:** [#68](https://github.com/HidekiAI/sidescroll-towerdefense/issues/68) (OPEN)
**Branch:** `feat/parallax-restack` (code repo), currently level with `trunk`
**Design of record (read-only, FROZEN):** wiki `TDD_Parallax-Background`
**Status at time of writing:** design derived from the engine source, **no code written yet**

---

## 1. Objective

Delete the `Camera2D` that displaces the entire editor UI, drive each
`Parallax2D.scroll_offset` from the scrollbar values instead, and re-derive the backdrop
framing from rendered-frame measurement under the corrected canvas.

---

## 2. What the ticket requires, verbatim

#89 carries no `Expected Result` heading. Its normative clauses are B1-B8 below, quoted from
the ticket. The D clauses are **not** from the ticket -- they are the task prompt's
Definition of done, and they are held to a stricter rule below.

### 2.1 Ticket clauses (normative authority)

From the issue body:

- **B1** (Consequence): "`simulator.gd _frame_backdrop()` and `Backdrops.position =
  Vector2(-960, -780)` were both tuned against this displaced canvas ... So those two
  values must be re-derived."
- **B2** (Consequence): "Anything else calibrated through the same lens is suspect,
  specifically:" -- three enumerated items, all re-derive or re-test, none delete:
  1. "the runtime horizontal alignment added in #68's work"
  2. "the 240px vertical travel clamp and its justification"
  3. "the conclusion that vertical travel opens a gap needing a **generator change** for
     taller art. That conclusion was measured through the wrong canvas and should be
     re-tested before acting on it."
- **B3** (Verification method), four numbered clauses: "Real window, not headless";
  "Assert a control's canvas-space rect intersects `get_visible_rect()`"; "For anything
  expected to be visible, hide it and diff the frame"; "Include a positive control that is
  known to draw".
- **B4** (Notes): "Both bars being `z_index = 20` remains correct and is worth keeping",
  and "this is a node/canvas structure issue, not an authoring one".
- **B5** (correction comment): "The camera cannot stay in the default canvas, and it cannot
  simply be switched off."
- **B6** (correction comment): "stop using a camera to scroll the strip at all, and drive
  each `Parallax2D.scroll_offset` directly from the scrollbar value times that layer's own
  `scroll_scale`."
- **B7** (correction comment): "No camera means no canvas transform, so the UI is never
  displaced."
- **B8** (Consequence, final paragraph): "#88 (per-band stagger) was also derived from a
  half_viewport * scroll_scale formula; that relationship needs re-measuring under the
  corrected canvas, since a camera-induced half-viewport offset and a parallax-induced
  offset would present identically."

### 2.2 Task-prompt clauses (acceptance contract, weaker authority)

- D1: "The `Camera2D` node and every reference to it are gone."
- D2: "Both scrollbars render real pixels, proven by hide-and-diff with a positive
  control, at 1920x1080 and at 1280x800."
- D3: "Horizontal scrub shows art across the full 6400 px range, at both window sizes."
- D4: "Each layer moves at its own `scroll_scale` -- the two-axis parallax is still
  observable."
- D5: "Framing constants are re-derived from measurement, and the comments stating them
  are corrected to match what was actually measured."

**The authority rule, and why it exists.** A D clause alone does not authorise a change.
Every implementation item in section 4 cites a **B** clause, and cites a D clause only in
addition. D5's "comments ... are corrected" is the reason this matters: on its own it would
authorise rewriting any comment anywhere, including a design document with no bearing on the
bug. That is the scope-creep path, and section 4.2a and #92 exist because the rule was
applied after a gate run caught it, not before.

Explicitly NOT required, and therefore not planned (section 8):

- N1: #88's per-band stagger **implementation**. B8 asks for the `half_viewport *
  scroll_scale` relationship to be addressed under the corrected canvas, and 4.2b
  discharges that -- not by measuring it, which is impossible once the input is gone, but by
  recording why and posting it to #88.
- N2: A `tools/gen-backdrop` change for taller art. B2 item 3 asks for the *conclusion* to
  be re-tested; M-e is that re-test. No generator change is planned and no generator file is
  touched, so the gate is `sstd-core`, not `gen-backdrop` (section 7).
- N3: Any change to the scrollbars' anchors or `z_index`. B4 says keep.
- N4: #87, or the retirement of the 13 old tracked PNGs.
- N5: **#92** -- the four code comments and one design document that describe the deleted
  `Camera2D`, plus `backdrop_preview.tscn:12-14`, whose premise is separately wrong today.
  #89 re-derives the *values*; it does not rewrite prose around them (section 4.2a).
- N6: **#90** -- the sky's celestial body. **#91** -- deriving `scroll_scale` from a
  per-layer depth, filed as a sub-issue of #90.

---

## 3. Mechanism, derived from the engine source (not assumed)

Read from [godotengine/godot `4.7/scene/2d/parallax_2d.cpp`](https://github.com/godotengine/godot/blob/4.7/scene/2d/parallax_2d.cpp),
`Parallax2D::_update_scroll()`. This is the derivation the plan rests on, so it is
written out in full.

```cpp
if (!is_inside_tree()) { return; }
Point2 scroll_ofs = screen_offset;                      // (1)
if (!is_editor_hint()) {                                // (1b) limit CLAMP, see M7
    if (limit_begin.x <= limit_end.x - vps.x) { scroll_ofs.x = CLAMP(...); }
    if (limit_begin.y <= limit_end.y - vps.y) { scroll_ofs.y = CLAMP(...); }
}
scroll_ofs *= scroll_scale;                             // (2)  scale applied ONCE, here
if (repeat_size.x) {                                    // (3a) wrapped branch
    real_t mod = Math::fposmod(scroll_ofs.x - scroll_offset.x,
                               repeat_size.x * get_scale().x);
    scroll_ofs.x = screen_offset.x - mod;
} else {                                                // (3b) unwrapped branch
    scroll_ofs.x = screen_offset.x + scroll_offset.x - scroll_ofs.x;
}
// y: identical structure on the y axis
if (!follow_viewport) { scroll_ofs -= screen_offset; }  // (4)
set_position(scroll_ofs);                               // (5)
```

**M9 -- `get_scale()` is (1,1) for every layer here, so the modulus is exactly
`repeat_size.x`.** No `[node ...]` block in `backdrop_preview.tscn` sets `scale` on any
`Parallax2D` or on any `Sprite2D`, and `Backdrops` and `BackdropStrip` set none either, so
every layer's own `scale` is the Node2D default (1,1) and `repeat_size.x * get_scale().x`
substitutes to `1920`. This is recorded because it is the one term that would stop being
1:1 if a layer were ever scaled, at which point the period in scrub terms becomes
`repeat_size.x * get_scale().x / scroll_scale.x` and the wrap-window arithmetic in M10
changes.

and the member defaults, from `parallax_2d.h`:

```cpp
Size2 scroll_scale = Size2(1, 1);
Point2 scroll_offset;                                   // (0,0)
Point2 screen_offset;                                   // (0,0)
Vector2 repeat_size;                                    // (0,0)
int repeat_times = 1;
bool follow_viewport = true;                            // <-- TRUE, see M8
```

Four consequences, each of which changes the design:

**M1 -- `screen_offset` is camera-only, so without a camera it is permanently (0,0).**
`_camera_moved()` is the sole writer of `screen_offset`. Delete the `Camera2D` and it is
never called. So (1) is (0,0), (2) is (0,0), and (4) subtracts (0,0). This is the whole
point of B7: the canvas transform is the identity, so no `Control` is displaced.

**M2 -- `scroll_scale` is applied to `screen_offset`, never to `scroll_offset`.** In (3b)
the scaled value cancels: with `screen_offset == 0`, `scroll_ofs.y` computes to
`0 + scroll_offset.y - 0`, i.e. exactly `scroll_offset.y`. The engine no longer scales
anything once the camera is gone.

**M3 -- the engine therefore applies NO per-layer scale of its own, and `simulator.gd`
must apply it.** This is why B6 says "scrollbar value times that layer's own
`scroll_scale`": the write must be `layer.scroll_offset = scrub * layer.scroll_scale`,
per layer. Writing the unscaled `scrub` would make all six layers move identically and
kill the parallax outright (a direct violation of D4).

**M4 -- the two axes take DIFFERENT branches, so they behave differently.** Substituting
`screen_offset == (0,0)` and `follow_viewport == true` (so (4) is skipped) into the code
above:

| axis | `repeat_size` | branch | resulting `position` | range as `scroll_offset` sweeps one period | visual rate per px of scrub |
|---|---|---|---|---|---|
| x | `1920` | (3a) | `-fposmod(-scroll_offset.x, 1920)` | `(-1920, 0]` | `scroll_scale.x`, wrapping |
| y | `0` | (3b) | `scroll_offset.y` | unbounded, linear | `scroll_scale.y` |

The x branch is `screen_offset.x - fposmod(scroll_ofs.x - scroll_offset.x, 1920)` with both
`screen_offset.x` and `scroll_ofs.x` zero, i.e. `-fposmod(-scroll_offset.x, 1920)`. Its
sign and its one-repeat offset are **invisible**, because `canvas_set_item_repeat` tiles
the authored copy seamlessly at `repeat_size.x = 1920`; what the eye reads is the rate
(`scroll_scale.x`), not the absolute value. This is also why the existing test reads
DELTAS between two equal steps (its header comment, "WHY DELTAS, NOT ABSOLUTE
POSITIONS") -- that idiom stays valid under both models and must be kept.

**M4b -- the camera-era formula, for contrast.** With a camera, `screen_offset` was the
camera's adjusted screen position `S` and `scroll_offset` was 0, so the x branch gave
`position.x = S.x - fposmod(S.x * scroll_scale.x, 1920)` -- a copy that TRAVELLED with the
camera, whose linear term `S.x * (1 - scroll_scale.x)` is exactly what
`test_parallax_backdrop.gd:17` records, and whose y term was
`S.y * (1 - scroll_scale.y)`. The new x branch replaces that travelling copy with a
stationary one spanning `(-1920, 0]`. Both are seamless; the difference is only how far
`repeat_times` must reach, which is section 3.1.

**M5 -- at the neutral scrub, every layer's `position` is exactly (0, 0).** With
`scroll_offset == (0,0)`, (3a) gives `fposmod(0, 1920) == 0` and (3b) gives `0`. So all
six layers share one origin and a single parent translation aligns them all on BOTH
axes. `simulator.gd:130` currently says the opposite is impossible:

> "Correcting that per layer needs a per-layer scroll_offset and is not done here."

Under M5 that is no longer true, which is what makes B1's "re-derive" achievable for
both axes rather than x only.

**M6 -- `autoscroll` must stay (0,0).** `_update_process()` enables internal processing
only when `autoscroll` is non-zero; when it is, `NOTIFICATION_INTERNAL_PROCESS` re-`fposmod`s
`scroll_offset` every frame. Leaving it at zero keeps `scroll_offset` exactly as written.
No change needed -- recorded so it is not disturbed.

**M7 -- `limit_begin`/`limit_end` clamping is inert.** The clamp in (1)-(2) is applied to
`scroll_ofs`, which is (0,0) at that point, and `CLAMP(0, -1e7, 1e7 - vps) == 0`. No
change needed.

**M8 -- `follow_viewport` is left at its default, which is `true`.** `parallax_2d.h` reads
`bool follow_viewport = true;`, so (4) -- `if (!follow_viewport) scroll_ofs -= screen_offset`
-- is **skipped**, not executed. That is the same model the camera era ran under, which is
why `test_parallax_backdrop.gd:17`'s `camera_step * (1 - scroll_scale)` form was ever
measured (M4b). No change is planned, and the justification is not the default value: it
is that with no camera `screen_offset` is (0,0) by M1, so (4) would subtract zero if it
did run. The flag is recorded because it is the one that would silently cancel the whole
offset if a camera ever returned to this canvas.

**M10 -- the x wrap is invisible in RENDERED PIXELS but it is NOT invisible in
`layer.position`, which is what the test reads.** `position.x` sweeps `(-1920, 0]` and
jumps by +1920 at each period boundary. `test_parallax_backdrop.gd:170-171` compares two
successive deltas at a 0.5 px tolerance and reports "drift is not reproducible" if they
disagree, so a wrap landing inside its two-step window produces a **false red on a
correct implementation**. The period in scrub terms is `1920 / scroll_scale.x`, and the
window the test needs clear is `base * scroll_scale.x` through
`(base + 2 * step) * scroll_scale.x`.

Two consequences, both of which the rewritten suite must handle:

- The measurement must NOT start at `scroll_x = 0`. `fposmod(0, 1920) == 0` is exactly a
  period boundary, so the first step off zero reads as a jump of ~-1920 rather than
  `step * scroll_scale.x`. Starting at the `scroll_x = 640` the axis-isolation check
  already leaves behind clears the window for all six layers: the widest is Foreground at
  `640 * 1.3 = 832` through `(640 + 240) * 1.3 = 1144`, and the narrowest is Sky at
  `64` through `88` -- every one strictly inside `(0, 1920)`.
- The suite must **assert** that condition rather than rely on it, so that a future
  `scroll_scale` change which narrows some layer's period turns the test red with a
  legible "the measurement window straddles a repeat boundary" instead of a misleading
  reproducibility complaint. `CAMERA_STEP` itself is not a wrap risk: `120 *
  1.3 = 156` is far inside any period.

The y axis has no equivalent hazard, because `repeat_size.y == 0` routes to the unwrapped
branch (M4).

**M11 -- `Foreground`'s sign convention INVERTS, so one existing comment must change
rather than be kept.** `test_parallax_backdrop.gd:19` records that at
`scroll_scale 1.3` the layer "moves backwards", and `:175-178` leans on that to reject a
sign flip. That was true of the camera model, where drift was `step * (1 - 1.3)`, i.e.
negative. Under M3 every layer moves FORWARD; what separates Foreground is only that it
moves FASTER than the scrub (`+1.3x` against Sky's `+0.1x`). The rewritten suite must
therefore assert the forward rate and keep an explicit ordering check
(`Foreground` travels further than `Sky` over the same scrub) to retain the sign
sensitivity the old negative-drift case provided. That ordering assertion is what makes a
swapped `scroll_scale.x` / `scroll_scale.y` detectable on the x axis alone.

### 3.1 Consequence for the two existing runtime helpers

**RESOLVED BY MEASUREMENT 2026-10-03 (M-k): `_size_layer_repeats()` is dead code and is
deleted.** It derived `repeat_times = ceil(scrub / repeat_size.x) + 1 = ceil(6400/1920) + 1
= 5`, a formula that was correct only for the camera model, where (per M4b) the authored
copy sat at `S.x - fposmod(S.x * scale.x, 1920)` and travelled with the camera across the
whole scrub.

M-k compared the rendered frame at `repeat_times` 1, 2, 3 and 5, at scrub 0, 1600 and 6400,
against a positive control that displaces every layer by +3000 px:

| scrub | rt1 vs rt2 | rt1 vs rt3 | rt1 vs rt5 | positive control |
|---|---|---|---|---|
| 0 | 0 px | 0 px | 0 px | 1975680 px |
| 1600 | 0 px | 0 px | 0 px | 1891321 px |
| 6400 | 0 px | 0 px | 0 px | 1950260 px |

`repeat_times` does not change a single pixel at 1920 wide, and the positive control proves
the diff can see a gap when one exists. Two facts from `4.7/scene/2d/parallax_2d.cpp`
explain it: `set_repeat_times()` clamps with `repeat_times = MAX(p_repeat_times, 1)`, and
`_update_repeat()` delegates to `RenderingServer.canvas_set_item_repeat(...)`, so the
renderer already covers the viewport at the minimum. Nothing under- or over-provisions, so
there is no value to choose and the helper has nothing left to do.

This also retires the tutorial's objection in
[2D Parallax](https://docs.godotengine.org/en/stable/tutorials/2d/2d_parallax.html)
("Increasing `repeat_times` technically *would* work in some scenarios, but is a brute
force solution"). The plan no longer proposes raising it.

`_frame_backdrop()` (`simulator.gd:99`) becomes correctable on both axes (M5), so its
"Only x" comment and its measured constants are replaced.

### 3.2 Alternatives considered and rejected

Two of these the ticket measured itself, on rendered frames. They are not candidates this
plan weighed and declined -- they are recorded results, and re-testing them would spend runs
re-deriving a finding the issue already carries.

| Alternative | Disposition | Basis |
|---|---|---|
| **Keep the `Camera2D`, move `BackdropStrip` to its own `CanvasLayer`** | **MEASURED, DOES NOT WORK** | #89's correction comment, candidate A: "A does not work at all. Moving `BackdropStrip` -- and therefore the camera -- onto its own `CanvasLayer` should have confined the camera to the art's canvas. Measured: the UI stays displaced in the bottom-right quadrant and both bars still draw 0 pixels. So in this scene structure the camera keeps transforming the default canvas." The body adds 63% bare tab against 1% before. |
| **Keep the `Camera2D` and switch it off** | **MEASURED, KILLS SCROLLING** | #89's correction comment, candidate B: restores the UI and both bars (30442 / 15690 px) but "the strip stops scrolling. Measured, not assumed." Candidate B is a proof of cause, not a fix. |
| **Make the camera harmless in place** | No setting achieves it | A `Camera2D` transforms its entire canvas layer, and every `Control` in `main.tscn` is in that layer. `ANCHOR_MODE_FIXED_TOP_LEFT` re-anchors the view but still displaces the canvas. B5 states the constraint directly: "The camera cannot stay in the default canvas, and it cannot simply be switched off." |
| **Use `ParallaxBackground` + `follow_viewport_enabled`** | Not adopted | The tutorial itself calls this recipe "tricky to get right", and it is the pre-4.3 approach it recommends moving away from. Adopting it would be a step backwards, not a fix. |
| **KoBeWi's Parallax2D Preview addon** | Not adopted | A third-party dependency in the editor for a problem this change solves outright. Recorded as the tutorial's own recommendation for editor preview, not taken. |

**Correction to an earlier draft of this plan.** It proposed re-testing the `CanvasLayer`
route with a positive control, on the grounds that an earlier session had recorded it as
failing and that three of this plan's own metrics had proven vacuous, so the rejection was
not trusted. That was the wrong instinct. The rejection rests on the ticket's own rendered-
frame measurement, which is the strongest evidence in the issue, and #89's body independently
reports the same 63% figure. Vacuous metrics elsewhere in a plan are not a reason to doubt a
measurement recorded by a different observer with different evidence; they are a reason to
check which measurement a claim rests on. Claim checked: it rests on the ticket.

**The load-bearing consequence.** Once the camera is gone, nothing in the engine applies
per-layer rates, because `_update_scroll()` scales `screen_offset` (M2) and with no camera
there is no `screen_offset`. Something must write `scroll_offset` per layer per frame. That
is not a design choice so much as the remaining degree of freedom, and it is why the
tutorial's silence on camera-less parallax (4.2a) is survivable rather than blocking.

### 3.3 Why `scroll_offset` is written by hand when the engine could do it

Worth stating because it is the one place this plan adds code the engine arguably owns. Under
the camera model, the engine multiplied `screen_offset` by `scroll_scale` at (2) and the
editor only had to move the camera. With no camera, `screen_offset` is (0,0) by M1, so (2)
multiplies zero and the engine's scaling becomes a no-op -- there is no camera reading left
for it to scale. The per-layer write is not a workaround bolted on; it is the same
multiplication, relocated to the caller because the caller's input no longer exists.

The alternative reading -- keep a camera purely as a scalar input, and ignore its canvas
transform on the layers -- was not pursued. `Parallax2D` has no per-instance opt-out of the
canvas transform, so it would mean putting a dummy camera on a `CanvasLayer` of its own to
keep its transform off the default layer, which is the candidate-A shape the ticket measured
as non-functional.

---

## 4. Target design

### 4.1 `editor/scenes/backdrop_preview.tscn`

- **Delete** the `[node name="Camera2D" type="Camera2D" parent="."]` block. B5, B6, B7, D1.
- **Re-derive** `Backdrops.position`. The authored `(-960, -780)` encodes the camera's
  centred view (`540 - 1320 = -780`, per its own comment) and a `-960` half-viewport that
  has no meaning once there is no centring. B1 names this value explicitly. B1 voids it.
  **M-b measured `(0, -291)` at 1920x1080**: it is the only candidate in the sweep that puts
  every band inside the frame with the sky's headroom intact, and the whole sweep is
  predicted to rounding by sky height 1320 against frame 1029 plus the `+31` Simulator-tab
  offset. 1280x800 is measured before the constant is written; if the two sizes disagree the
  value becomes size-dependent and the plan changes.
- **The three comment blocks in this file are NOT corrected here.** Lines 12-14 (which
  justify `repeat_size_x = 1920` with "the camera can pan forever"), 32-44 (which derive the
  offset from `ANCHOR_MODE_DRAG_CENTER`) and 38-42 (which size the sky's overshoot against
  "the camera's 240px vertical travel") all describe a node B5 deletes, so all three are
  stale. None of them justifies a value B1 or B2 re-derives, so no ticket clause reaches
  them; D5's "comments ... are corrected" is the task prompt's clause, not the ticket's,
  and on its own it would authorise rewriting any comment in the repo. Tracked as **#92**
  with the locations and reasons, to be done after this ticket lands so the replacement text
  can state measured numbers rather than the old ones. Section 4.2c.
- Untouched: the six `Parallax2D` layers, their `scroll_scale`, `repeat_size`, `z_index`,
  and their `Sprite2D` children. Those are transcribed from `manifest.json`, which
  `backdrop_preview.tscn:4-6` names as the single source of truth and which this ticket
  does not change. Note `repeat_times` is never authored in this scene at all -- it defaults
  to `1`, which is the state M-k measured as fully covering.

### 4.2 `editor/scripts/simulator.gd`

- `_ready()`: drop `var camera := strip.get_node("Camera2D") as Camera2D` and both
  `camera.position = ...` closures. Replace with per-layer writes driven by both bars
  (M3). D1, B6.
- Both bars must keep writing **their own axis only**. The current closures already do
  that on the shared `camera.position` Vector2; the replacement must preserve it, because
  that is what the existing test's "held_x" assertion pins.
- `_frame_backdrop()`: re-derive on both axes from M5, replacing the x-only correction and
  its stale measured prose. B1, D5.
- `_frame_backdrop()`'s doc block at `simulator.gd:74-98`: its opening claim, "Parallax2D
  frames its own repeat run from the viewport, so the correction depends on the window
  size", is the reason a single static position was rejected before. Under M4 the x
  position is stationary in `(-1920, 0]`, so that reasoning no longer explains the x
  correction's size dependence and must be re-derived rather than carried forward. Its
  inline `// Only x. ...` note and the closing sentence about aligning the sky baseline
  are rewritten. B1, D5.
- **`_size_layer_repeats()` is RE-DERIVED, and removal is the conditional follow-through.**
  The primary disposition is the one B2 asks for; the deletion is what the re-derivation turns
  up, and it is separable. B2's lead-in is "Anything else calibrated through the same lens is
  suspect", and this helper is calibrated through the same lens: it computes `repeat_times =
  ceil(scrub / repeat_size.x) + 1`, which reasons about how far the authored copy has to travel,
  and under the camera model (M4b) that copy travelled with the camera across the whole scrub.
  So B2's instruction applies -- **re-derive** -- and the re-derivation is M-k:
  `repeat_times` 1, 2, 3 and 5 render pixel-identical at scrub 0, 1600 and 6400 against a
  positive control that diverges ~1.9M px; `set_repeat_times()` clamps with
  `MAX(p_repeat_times, 1)`; and `_update_repeat()` delegates to
  `RenderingServer.canvas_set_item_repeat`, so the renderer covers the viewport at the minimum.
  The quantity is unobservable, so a function whose stated purpose is now false is removed
  rather than re-tuned.
  **Two honest qualifications.** B2's own three enumerated items are *not* this one, so the
  enumeration alone does not reach it -- the lead-in does, and that is an argued reading, not a
  quoted one. And nothing depends on the outcome either way: M-k shows the function cannot
  affect the frame, so leaving it in place costs nothing. It is not a correctness dependency and
  block 1/6 is the one block that can be dropped without touching anything else.
  One factual consequence, not an argument for the deletion: the figures an earlier draft said
  must not be restated are *in* this doc block -- `simulator.gd:53` ("x=0 was only 6% bare")
  and `simulator.gd:63` ("measured at x = 0, 1600, 3200, 4800, 6400") -- and both come from the
  void M-d metric (5.1).
- `_scroll_v.max_value`: the 240px clamp stays until measurement says otherwise, and its
  justification comment at `simulator.gd:42-44` is replaced by the M-e measurement. B2,
  D5. The `(2784, 419.32)` / `(2816, 199.48)` figures at `simulator.gd:79-81` are
  comment prose, not code constants, and are removed with that block.
- **File header (`simulator.gd:2-11`): NOT corrected here.** It explains the `Camera2D`
  contract in the first person -- "the tutorial's Parallax2D multiplies the camera offset by
  scroll_scale per axis". B5 deletes the node it describes, so the header is stale, but it is
  not the justification of any value B1 or B2 re-derives. Tracked in **#92**.
- `_frame_backdrop()`'s doc block (`simulator.gd:74-98`) **is** in scope, unlike the two
  above, because B1 names `_frame_backdrop()` as one of the two values to re-derive and this
  block is where its rationale lives.

### 4.2a Relationship to the official tutorial, stated honestly

The rates already match the tutorial's table exactly on x (`0.1 / 0.2 / 0.3 / 0.5 / 0.7`),
and the six `Parallax2D` siblings under a plain `Node2D` follow its recommendation to use
`Parallax2D` rather than `ParallaxBackground`. Two departures must be written down rather
than discovered later:

- **The tutorial's whole model is camera-relative.** Under "Scroll offset" it documents
  `scroll_offset` as a *starting point* -- "if you prefer it to start at a different point"
  -- not as a per-frame scroll driver, and it offers no camera-less alternative. #89 removes
  the camera, so this plan necessarily repurposes `scroll_offset` as the driver. That is a
  public documented property written every frame, which is a legitimate use, but it is not
  the use the tutorial illustrates and the code comments must say so.
- **The tutorial's "Previewing in the editor" section is about our exact situation** -- a
  parallax stack inside a `Control`-based editor -- and recommends neither route taken here.
  Its options are `ParallaxBackground` + `follow_viewport_enabled`, a `CanvasLayer`, or
  KoBeWi's Parallax2D Preview addon. See 3.2 -- where the first two are not merely declined
  but ticket-measured as non-functional.

### 4.2b B8 disposition -- #88's `half_viewport * scroll_scale` formula

B8 asks that the relationship "be re-measuring under the corrected canvas". It cannot be,
and saying so is the disposition rather than a deferral.

`half_viewport` is the camera's half-viewport displacement. It exists only because a
`Camera2D` centres the view, so the canvas transform is non-identity and every layer in that
canvas is shifted by the same amount. B7 makes the canvas transform the identity, so after
this fix no layer sees a half-viewport offset again. The formula's **input** is undefined,
not merely wrong -- there is no quantity left to re-measure.

The ambiguity B8 actually names is removed rather than measured away: the body says a
camera-induced offset and a parallax-induced one "would present identically", and deleting
the camera deletes the first of the two, so there is nothing left to be confused with.

After this fix each layer's x position is `-fposmod(-scroll_offset.x, repeat_size.x)` with
`scroll_offset.x = scrub * scroll_scale.x`, so a per-band stagger is an additive constant in
scrub units applied to a value `simulator.gd` already computes per layer -- a simpler
expression than the current one, but the constants are #88's work.

**Recorded where it is actionable:** posted to
[#88](https://github.com/HidekiAI/sidescroll-towerdefense/issues/88#issuecomment-5970931374),
including the two questions #88 must now answer that it could not before (whether the
stagger is expressed in scrub units or screen units -- they differ by `scroll_scale.x` per
layer -- and whether it is a constant or a fraction of the scrub, since a fraction must be
checked against each layer's repeat period). No values are proposed here.

### 4.2c What no ticket clause reaches -- #92

The boundary this plan draws, stated so a later reader can see it was drawn deliberately:

| Item | In a ticket clause? | Disposition |
|---|---|---|
| `_frame_backdrop()` and its doc block | B1 names the function | in scope, 4.2 |
| 240px clamp and its justification | B2 item 2 says "and its justification" | in scope, 4.2 |
| #68's runtime horizontal alignment | B2 item 1 | in scope -- it is the x half of the B1 re-derivation |
| generator-change conclusion | B2 item 3 | re-tested by M-e; no generator change, 4.3 |
| `backdrop_preview.tscn` header comment (12-14) | no | #92 |
| `backdrop_preview.tscn` `Backdrops` comment (32-44) | no | #92 |
| `backdrop_preview.tscn` sky-overshoot comment (38-42) | no | #92 |
| `simulator.gd` file header (2-11) | no | #92 |
| `docs/PLAN-2026-09-30-parallax-tutorial-stack.md` 3.1 + Phase 3 | no | #92 |

The rule behind the table: **a comment is in scope only when it justifies a value a ticket
clause re-derives.** Everything else is stale prose, which is a real defect but not this
ticket's -- and #89 is a bug fix, where widening the blast radius is the failure mode this
gate exists to catch.

### 4.3 `editor/tests/test_parallax_backdrop.gd`

This suite is the standing contract and it currently *requires* the camera
(`test_parallax_backdrop.gd:69-73` fails with "BackdropStrip has no Camera2D to drive"),
so deleting the node turns it red by construction. It must be rewritten to the new
contract, keeping the parts that are still load-bearing:

- **Keep** the delta-based drift measurement (its header comment's reasoning still holds
  under M4).
- **Keep** the literal `scroll_scale` and literal texture-filename assertions, and the
  header comment recording that mutation testing refuted two earlier drafts because the
  expectation read the value under test. That lesson is the reason those lines exist.
- **Keep** the per-axis isolation check ("the x bar must move x while y stays exactly at
  zero"), retargeted from `camera.position` to `scroll_offset`.
- **Replace** the expected drift form. Under M4 it is no longer
  `camera_step * (1 - scroll_scale)`; it becomes the per-layer rate implied by M3/M4.
  The exact literals are fixed by measurement, and the `Foreground` sign case (its rate
  exceeds 1.0, so it moves faster than the scrub) is retained.
- **Add** a falsifiable assertion that the two bars write DIFFERENT `scroll_offset`
  values to layers with different `scroll_scale`. Without it, a suite that asserted only
  "the art moved" would pass for an implementation where all six layers move 1:1, which
  is exactly the M3 failure mode.
- **Keep `scroll_x.max_value == 6400.0` unchanged** (`test_parallax_backdrop.gd:111-112`).
  Re-citation: the ticket names **no scrub range anywhere** -- `grep -n 6400` on the issue text
  returns nothing -- so this is a keep, not a change, and no clause authorises altering it. The
  range is a code fact, documented at `simulator.gd:40-41` ("Strip is 6400px wide; scrub spans
  the full band"), and 6400 is the strip width rather than a framing constant, so B1's
  re-derivation does not reach it. D3's "full 6400 px range" is the acceptance phrasing for
  this assertion, cited as such and not as authority for a change.
- **Keep `scroll_v.max_value == 240.0`; rewrite its justification comment.**
  `test_parallax_backdrop.gd:108-109` asserts the value, `:106-107` justifies it, and the
  justification is false. It reads "Vertical travel is clamped to the generator's 240px
  overscan: past it the 1320px sky band would reveal its own top edge." M-e measured the sky's
  top edge at **3,637.5** scrub units away -- the geometric headroom is 15x the clamp, so past
  240 nothing is revealed. B2 item 2 names "the 240px vertical travel clamp **and its
  justification**", so the comment is in scope and the value is not.
  The number is kept because it is the generator's `overscan_px` (`manifest.json`), a *design*
  limit, and it is safe at both resolutions with margin to spare -- which is now measured rather
  than assumed. 4.3's earlier disposition ("M-e decides the value") expected M-e might move it;
  it did not, and the plan records the outcome instead of the expectation.

### 4.4 `editor/tests/probe_parallax_scroll.gd` -- delete

It reads `BackdropStrip/Camera2D` at `:28`, a node B5, B6 and B7 require to be gone, so
once the camera is deleted this probe cannot run at all. It is already dead independent of
this change: `:29` reads `Backdrops/L2_near`, a node removed by the restack commit `ba2cbb4`
in favour of the six manifest layers. Deleting it is therefore the minimal way to satisfy B6
without rewriting a probe for a scene shape that no longer exists, and it is the only
disposition available -- the alternative is repairing a file that tests nothing live.

Its contract is **superseded** by `test_parallax_backdrop.gd` as rewritten in 4.3, which
asserts the same layer-rate properties under the new model and additionally covers the axis
isolation and ordering checks the probe never had. Superseded, not "fully subsumed": an
earlier draft claimed the latter, which is a claim about coverage of a moving target and
should not be made for a file being replaced.

### 4.5 `editor/tests/probe_render_visibility.gd` -- new, real window

The only new non-document file. B3's four clauses require exactly what it does, and D2 is the
acceptance item only rendered evidence can settle -- #89 is a defect that every transform
read and every `z_index` probe reported as fine.

The repo's precedent for frame capture is `editor/tests/capture_backdrop.gd:29`, which does
`root.get_texture().get_image().save_png(path)` in one statement, and
`editor/capture_all_tabs.gd`, which splits the same pattern across two. Neither sets a
window size. `probe_parallax_scroll.gd` is **not** that precedent -- it is a headless
node-graph probe with no `DisplayServer` call and no frame read.

Because B3 clause 1 rules out headless (`DisplayServer.window_get_size()` is (0,0) there),
the sizing has to come from the command line as `--resolution 1920x1080` and
`--resolution 1280x800`, and the probe asserts the size it actually got rather than assuming
it. A run whose reported size is (0,0) exits non-zero as an invalid experiment, not as a
zero-pixel result.

Requirements it encodes, each with the clause that requires it:

- **B3 clause 4** -- a positive control known to draw, so a 0-pixel result is distinguishable
  from a broken differ.
- **B3 clause 2** -- canvas-space rect intersected with `get_visible_rect()`, for both bars.
- **B3 clause 3** -- changed-pixel count for each bar against a baseline frame.
- **B3 clause 1** -- run with a real window, never headless.

**On the second resolution.** The ticket names one size, "a real window at 1920x1080", and
nothing else. 1280x800 is nonetheless required, and the reason is in the code rather than in
the prompt: `simulator.gd:78` states the existing correction "depends on the window size", and
`simulator.gd:125` records a figure measured "at both window sizes". So the code being
re-derived under B1 is **already** two-size behaviour, and re-deriving it at one size would
produce a constant that is wrong at the other by construction. B3 supplies the method; the code
supplies the reason to run it twice.

**D3's range-end evidence, with the metric corrected.** D3 needs "art across the full
6400 px range". The first attempt measured that as a whole-frame CHANGED-pixel percentage
and is recorded void in 5.1: exposing bare background is itself a change, so a gap scores
identically to art, and every configuration reported 0.00% bare including at scrub 6400 where
one 1920 px copy cannot cover a 1920 px frame.

The replacement is an **absolute** count, not a difference:

- Count pixels equal to the Simulator tab's background colour, rather than counting pixels
  that changed. A gap is a positive quantity that stands on its own; a diff cannot tell a gap
  from art because both are "changed".
- Sample at scrub x = 0, 1600, 3200, 4800 and 6400. The endpoints are the scrub's extent,
  documented at `simulator.gd:40-41`; the five-point grid is not new, it is the grid
  `simulator.gd:63` already used for its (void) measurement, so it needs no new
  authorisation -- only a metric that can actually fail.
- **Positive control:** hide one named layer, assert the bare count RISES by that layer's
  pixel count, then restore it. This is the control 5.1 says every metric must carry, and it
  is also the check that distinguishes "no gaps" from "the diff cannot see gaps".
- **Stated prediction before the run:** M-k established geometrically that the authored copy
  spans `(-1920, 0]` at rate `scroll_scale.x`, which covers the frame at every scrub, so the
  expected bare count is 0 throughout. A non-zero result falsifies that and means the
  geometry reasoning is wrong, not that the metric is wrong.

---

## 5. Measurement plan (runs BEFORE any framing constant is written)

Every number that ends up in the code comes from here. Headless is unusable for all of
it (B3 clause 1).

| # | Question | Method | Decides |
|---|---|---|---|
| M-0 | Are M1 and M5 true at runtime, not just in the source? | read `layer.screen_offset` and `layer.position` on all six layers at scrub (0,0), before and after the camera is deleted | whether M1/M5 are measured or assumed; `Backdrops.position` baseline |
| M-a | Is the real per-axis rate `scrub * scroll_scale` (M3/M4)? | real window; scrub one bar; measure each layer's `position` delta over two equal steps | the literal drift constants in 4.3 |
| M-b | Where does the art land at scrub (0,0), at 1920x1080 and 1280x800? | hide-and-diff per layer, plus canvas-space rects | `Backdrops.position` (4.1) |
| M-c | Does one parent translation align all six bands on both axes? | M-b repeated per band | whether `_frame_backdrop` corrects y (M5) |
| M-d | Is `repeat_times = 5` correct, over-provisioned, or insufficient across 0..6400? | bare-background **count** at scrub x = 0, 1600, 3200, 4800, 6400, both sizes, with a hide-one-layer control (4.5) | whether `_size_layer_repeats` changes (3.1) |
| M-e | How much vertical travel is safe before the sky's top edge shows? | scrub y upward at both sizes, hide-and-diff | `_scroll_v.max_value` (B2 item 2) and the `240.0` assertion (4.3). Also re-tests B2 item 3, the generator-change conclusion |
| M-f | Do both bars contribute real pixels? | hide-and-diff with positive control, both sizes | D2 |
| M-h | What is actually painted in the sky, and where is the celestial body? | read `sky.png` directly, no frame capture | filed as #90; not a prerequisite here |
| M-k | Does any `repeat_times` value change the frame, and can the diff see a gap at all? | rt 1/2/3/5 vs each other, plus a positive control displacing every layer | 3.1, the `_size_layer_repeats` disposition |

This table lists only what still has to be **run**. Two questions were asked and answered during
planning and are recorded in 5.2 rather than here, because no plan item depends on either:

- **M-g** -- why `Forest` and `Foreground` contribute so few pixels. Answered: they are sparse
  silhouettes. See 5.2.
- **M-i/M-h** -- what is in the sky, and where is the celestial body. Answered, filed as #90.
  D3 is about scrub range, not about a body at infinity, so no #89 item depends on it; an
  earlier draft listed it as a prerequisite for D3, which was wrong.

### 5.1 Voided measurements, kept so they are not repeated

Three coverage metrics were built and then falsified. They are recorded here because each
one looked correct and each one measured nothing.

| Attempt | Why it was vacuous |
|---|---|
| M-d, `_measure_whole_frame_coverage` | Counts CHANGED pixels. Exposing bare tab background is itself a change, so a gap scores identically to art. Every `repeat_times` reported 0.00% bare, including at scrub 6400 where a single 1920 px copy cannot cover a 1920 px frame. |
| M-d1, bare-`%` across the scrub | Same metric, re-run. Also sampled only `scrub = 0`, where every `position.x` is exactly 0 and the authored copy lands flush at the frame's left edge, so `repeat_times = 1` looks sufficient when it is not. |
| M-j, divergence from a `repeat_times = 5` reference | Its own negative control failed first. `repeat_times = 0` reported 0 px divergence from a five-copy reference, which is impossible. |

Two traps in the attempts above are worth naming because they cost a run each:

- `repeat_times = 0` does **not** draw nothing. `set_repeat_times()` clamps with
  `MAX(p_repeat_times, 1)`. The "1614339 px divergence" that looked like a passing control
  was in fact measuring a displaced Sky left over from the previous section.
- `Parallax2D` has **no `NOTIFICATION_DRAW`**. It draws nothing itself; `_update_repeat()`
  hands the repeat to `RenderingServer.canvas_set_item_repeat(...)`. So `queue_redraw()`
  is irrelevant to `repeat_times`, and adding it to "fix" the measurement was addressing a
  mechanism that does not exist.

The lesson applied to the surviving metrics: a measurement needs a positive control that
**must** produce a non-zero result, and it needs a prediction stated before the run. M-k
satisfies both -- the control displaces every layer by +3000 px, and the 0-px gap between
rt1 and rt5 is the prediction being confirmed.

M-0 runs first and exists because M1 and M5 are the two claims the whole design rests on,
and this ticket's own history is a defect that every property read reported as fine. They
are read from the live node, not inferred from the engine source.

### 5.1a Two more voided measurements, found while fixing M-e

Both were produced by rebuilding M-e at the second resolution, and both **reported success**.

**1. A sweep whose range was chosen by hand.** The original M-e sampled
`scrub_y` in `[0, 60, 120, 180, 240, 480]` and reported "no gap" at every point. The true first
bare pixel is at `-387.5`, so the entire sweep sat inside the safe region and its "no gap" was a
statement about where it stopped. It also printed the sky's canvas rect but never the frame rect
it was comparing against, so the reader could not have checked the claim even if it were wrong.
This is the same defect as the earlier `scrub = 0`-only attempt, one axis over. **Sample points
for a threshold are now derived from the geometry that sets the threshold**, then used to verify
the prediction rather than to find it.

**2. `min|gap|` over all six bands, when five of them are occluded.** The rebuilt sweep computed
a per-band gap for every layer and took the smallest magnitude. It reported `-27.0` for the
Foreground -- and the Foreground's rect leaving the frame reveals *the Sky*, which is still
there. The correct bound comes from the single backmost opaque band, whose rect the run confirms
encloses all five others' rects at scrub 0. **`min` over occluders is not a bound on anything**;
the arithmetic was correct and the model was wrong, which is the harder defect to see because
the output is a plausible small number rather than an obvious error.

The shared lesson, and the reason this subsection exists: **both defects produced a clean
result.** A measurement is only as good as the question it was in a position to answer, and the
question has to be stated before the run -- not reconstructed after it. The surviving M-e states
which band bounds it and why, predicts the boundary before sampling, and carries a positive
control that must rise.

Each measurement is recorded with its window size and its positive control, and the
number that ships is the one measured, not the one predicted.

### 5.2 Measurements confirmed at 1920x1080

All values below were read or diffed with a window; headless was not used.

| # | Result |
|---|---|
| M-0 | All six layers `position = (0,0)`, `screen_offset = (0,0)`, `scroll_offset = (0,0)` at neutral scrub. M1 and M5 hold at runtime. |
| M-a | `scroll_offset = scrub * scroll_scale` gives exactly rate `scroll_scale` per layer per axis. Sky `+0.1` moved `12.0` px per `120` scrub; Foreground `+1.3` moved `156.0`. All six matched, and both step windows agreed (no repeat boundary between them). |
| M-b/M-c | `Backdrops.position.y = -291` gives 0.00% bare at 1920x1080 with every band inside the frame. The whole sweep is predicted by sky height 1320 vs frame 1029 plus the `+31` Simulator tab offset: `-780 -> 44.51%`, `-600 -> 27.02%`, `-400 -> 7.58%`, `0 -> 3.01%`, all matching prediction to rounding. One parent translation does align all six bands on both axes. |
| M-f | Both bars contribute real pixels: positive control `30912`, `bar_x` `30442`, `bar_y` `15690`, and both rects intersect `get_visible_rect()`. D2 holds at this size. |
| M-g | `Forest` and `Foreground` are sparse silhouettes, not a bug: `forest.png` is 36% opaque with content in rows 187..320 of 320, `foreground.png` is 18% opaque with content in rows 254..320. Their small pixel counts are what the art actually contains. |
| M-h | `sky.png` is 1920x1320 and fully opaque. The celestial body is a 116x117 disc at `(1248, 259)`, luma 0.937, brightest pixel `(1258, 336)`, 1248 px from the left tile edge and 556 from the right. At `scroll_scale.x = 0.1` it travels 640 px over the 6400 scrub, wraps its tile edge every 19200 px, and its next tiled copy sits at x=3168 -- so a viewport wider than that shows two. **Filed as #90; no #89 item depends on this.** |
| M-k | `repeat_times` 1/2/3/5 render identically. See 3.1. |
| M-e | **Vertical travel is bounded by the Sky alone**, and the sky's rect contains all five other bands' rects at scrub 0 (`inside_sky=true` for every one), so no other band can expose background. Predicted first bare pixel: `scrub_y = 3637.5` (sky top, **resolution-independent**: region top `31` and sky top `-260` are both fixed) or `-387.5` at 1920x1029 / `-3250.0` at 1280x800 (sky bottom). The shipped `240.0` clamp is below the upward bound at both sizes and measured `0` background px at `scrub_y` 0, 240.0 and 241.0. **Keep 240.0; fix its justification** (4.3). |

M-e's counter is verified four ways, none of which is "the number looked fine":

| Check | Expected | Measured |
|---|---|---|
| Bare frame counted against itself | exactly the region area | `1,916,160` = 1920x998; `984,320` = 1280x769 |
| Positive control, hide the Sky | must RISE | 0 -> `1,423,644`; 0 -> `921,975` |
| Gap appears where predicted, not before | 0 at the boundary, non-zero one step past | 0 at `-387.5`/`-3250.0`, then `15,360`/`83,200` |
| Count matches the predicted gap height | `step * rate_y * width`, to rounding | `96.875 * 0.08 * 1920 = 14,880` -> `15,360` (8 rows); `812.5 * 0.08 * 1280 = 83,200` -> `83,200` exactly |

**`--resolution 1920x1080` yields a 1920x1029 window.** The window manager grants 1029, not
1080, and `DisplayServer.window_get_size()` reports the granted size. Every number in this plan
labelled "1920x1080" was measured at 1920x1029, with a `seen_region` of 1920x998 at canvas
`y = 31`. The labels stay because they name the request a reader will reproduce, but
`probe_render_visibility.gd` must assert the size it *received* against the window manager's
actual screen size rather than against the requested one, or it will fail on any machine that
clamps for a different reason.

M-e is now **measured** (see its row and the counter-verification table above), so
`_scroll_v.max_value` is decided: keep `240.0`, correct the justification at
`test_parallax_backdrop.gd:106-107`. One item still unmeasured, and it is a framing constant
rather than a number: the `Backdrops.position.y` confirmation at the second resolution, which
M-b/M-c covered only at the first.

---

## 6. Block map (proposal, for approval before scaffolding)

Coding-assistant Phase 1. Test files are the assistant's, not blocks. Numbering is global
and in dependency order.

| Block | file:line | What to write (not how) | Clause | Depends on |
|---|---|---|---|---|
| 1/6 | `editor/scripts/simulator.gd:64` | **re-derive** `_size_layer_repeats()` under B2's lead-in; M-k's answer is that the quantity is unobservable, so removal is the *conditional* follow-through (4.2) | B2 lead-in | M-k |
| 2/6 | `editor/scenes/backdrop_preview.tscn:29` | remove the `Camera2D` node | B5, B6, B7 | - |
| 3/6 | `editor/scripts/simulator.gd:25,32-39` | replace the camera writes with per-layer `scroll_offset` writes, each bar on its own axis | B6, B7 | 2/6, M-a |
| 4/6 | `editor/scenes/backdrop_preview.tscn:45` | re-derive `Backdrops.position` | B1 | 3/6, M-b, M-c |
| 5/6 | `editor/scripts/simulator.gd:99` | re-derive `_frame_backdrop()` on both axes, with its doc block | B1, B2 item 1 | 4/6, M-b, M-c |
| 6/6 | `editor/scripts/simulator.gd:45` | set the vertical clamp and its justification from the measured travel | B2 item 2 | 5/6, M-e |

Ordering is dependency order, and block 1 is first because it is the only one that can be
dropped: while `_size_layer_repeats()` exists it rewrites `repeat_times` on every resize, so
any measurement or test of blocks 3-6 taken alongside it observes a value something else is
still setting. M-k says that value cannot affect the frame, so the ordering is precaution
rather than necessity -- which is the same reason 4.2 gives for making it droppable.

Assistant-written, not blocks: `test_parallax_backdrop.gd` (rewrite), the deletion of
`probe_parallax_scroll.gd` (4.4), and `probe_render_visibility.gd` (4.5). Nothing else is
touched -- in particular no comment outside 4.2c's first three rows and no design document,
both of which are #92.

GDScript holes must be **real static errors**, because GDScript has no ahead-of-time
compiler and a hole that only fails at runtime shows no red squiggle. So each block is
built around a wrong-typed initializer (`var total: int = "text"`) or a non-void return
with an empty body ("Not all code paths return a value"). Never `pass`, never a bare
`return`, never a cast. Every variable, parameter, return type and `@onready` is
explicitly typed, 4-space indent, ASCII only.

---

## 7. Gates

No `tools/gen-backdrop` file is touched, so its own tests are irrelevant here; the Rust gate is
the workspace's stated one. B2 item 3 asks for the *conclusion* about taller art to be
re-tested, which M-e does, not for the generator to change.

```bash
cargo test -p sstd-core                           # 110 tests, the workspace Rust gate

$HOME/bin/godot4 --headless --path editor --script res://tests/test_parallax_backdrop.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_image_to_map.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_terrain_brush.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_override_merge.gd

$HOME/bin/godot4 --path editor --resolution 1920x1080 --script res://tests/probe_render_visibility.gd
$HOME/bin/godot4 --path editor --resolution 1280x800  --script res://tests/probe_render_visibility.gd
```

Two rules, and both have cost a run before:

1. **Gate on the exit code**, never on a printed `failures=0` line. `test_screen_store` is
   absent from the list because it exits 1 on Godot 4.7.2 for pre-existing #85 reasons.
2. **`git checkout -- editor/project.godot` after every Godot run.** The file *is* tracked
   (`git ls-files --error-unmatch` returns it) and drifts on the `--import` that 4.7.2 does on
   open. Never `git add -A` or `git add .` -- `editor/addons/` is 516 MB of vendored code.

The probe is a gate rather than a probe because D2 is the acceptance item only rendered
evidence can settle. Its contract, restated from 4.5 so a failure is diagnosable from here:
exit 0 requires both bars non-zero at that resolution, a positive control that contributes
non-zero, and a reported window size other than (0,0).

---

## 8. Possible follow-ups (NOT this ticket, NOT planned)

- **M0 -- the celestial body filed as #90 rather than fixed here.** `sky.png` carries a
  116x117 px disc at tile x=1248. At `scroll_scale.x = 0.1` it travels 640 px -- 33% of a
  screen width -- over the 6400 px scrub, and its next tiled copy sits at x=3168, so a
  viewport wider than that shows two. A body at infinity should not move: rate `0` is the
  `Z -> inf` limit of `scroll_scale = Z_ref / Z`, and is the tutorial's "complete stop". It
  is **independent of this ticket** -- neither caused by the camera nor by the
  `scroll_offset` mechanism -- so it is not fixed here. Measured detail (M-h) and the two
  options are in #90; #91 covers the depth model that would give infinity a representation
  rather than a bare `0`.

### 8.1 Deferred, with the reason each is out of this ticket's reach

These are the parts of the bug that are real but unreachable by any clause in 2.1. Each one
names the clause that *would* have to exist for it to be in scope.

- **N1 -- #88, per-band stagger implementation.** B8's clause is discharged in 4.2b, which
  reasons why the formula cannot be re-measured as written and posts that to #88. What is
  *not* done here is choosing the stagger constants, because B8 asks about the relationship,
  not the values, and because per-layer control only becomes *possible* after this ticket
  -- it does not become *chosen*. The relationship's new form is `scrub * scroll_scale.x`
  plus an additive per-band constant, which is strictly simpler than the current expression;
  what remains for #88 is the constants and whether they are in scrub or screen units.
- **N2 -- taller art from `tools/gen-backdrop`.** B2 item 3 asks for the conclusion to be
  re-tested, and M-e is that re-test. If a gap proves real it needs its own ticket; no
  generator change is planned and no generator file is touched.
- **N3 -- the scrollbars' anchors and `z_index`.** B4 says the current values are correct
  and worth keeping. No change planned.
- **N4 -- #87 and the retirement of the 13 old tracked PNGs.** Untouched; still blocked
  separately.
- **N5 -- #92, the stale `Camera2D` comments and the superseded design plan.** Filed during
  this plan's gate, because a gate run found four comment blocks and one design document
  that no ticket clause reaches (4.2c). The defect is real; it is not this bug's.
- **N0 -- #90, the sky's celestial body, and #91, depth-derived rates.** #91 is filed as a
  sub-issue of #90, so the relationship is a real one rather than a cross-mention. Neither
  is caused by the camera nor by the `scroll_offset` mechanism.

---

## 9. Risks

| Risk | Mitigation |
|---|---|
| M3/M4 wrong, so D4 fails | Section 5 M-a measures the rate before any constant is written; the delta idiom in `test_parallax_backdrop.gd` is retained so a wrong rate is a red test, not a wrong picture |
| Framing constants re-derived through a stale method | B3's two rendered checks are mandatory and neither is a transform read; the previous constants were wrong precisely because a transform read said otherwise |
| `repeat_times` under-provisioned after the model change | **Retired.** M-k measured rt 1/2/3/5 as pixel-identical against a ~1.9M px positive control, and the engine clamps `repeat_times` to a minimum of 1 while the renderer covers the viewport regardless. See 3.1. |
| A measurement is trusted because it looks sane rather than because it can fail | 5.1 and 5.1a record **five** vacuous metrics that all reported success. Every surviving metric carries a positive control that MUST produce non-zero, and states its prediction before the run. 4.5 replaces the one metric the gate found to be vacuous by construction. |
| Scope creep into #88, the generator, or stale comments | Section 8.1 lists all three; 4.2c draws the line explicitly with the clause test each side of it passes or fails; the gate's hard cap applies |
| `_size_layer_repeats` re-derivation rests on B2's lead-in, not its enumeration | Stated as an argued reading in 4.2, not claimed as a quote. The shipped default is **re-derive**, and removal is separable -- M-k shows the function cannot affect the frame either way, so a reviewer who rejects the deletion loses nothing and drops one block |
| `project.godot` or `addons/` accidentally committed | Section 7, rule 2 |

---

## 10. Plan gate

**Iterations:** 3. Bar is 90. Evaluator: independent agent, same model every iteration, given
the ticket text, this document, and the rubric -- never the planner's reasoning.

| iteration | score | outcome |
|---|---|---|
| 1 | 84 | FAIL. No CRITICAL, but two uncovered clauses and seven deficiencies. |
| 2 | 91 | PASS as scored. **Invalid pass** -- see below. |
| 3 | **88** | FAIL on the bar (90), **no CRITICAL** -- the two CRITICALs it opened with were both resolved, and it declined to re-raise either. Seven deficiencies, all citation-hygiene rather than design. Revision below is in flight. |

### The iteration-2 pass was invalid, and why

Iteration 2 was scored against the ticket text **plus the task prompt's Definition of done**,
with the D clauses presented as normative alongside the ticket's. That fed the evaluator a
premise this plan should have been tested against, and the plan sailed through on it: the
document cites `D5` ("comments ... are corrected") as authority for rewriting a *design
document*, and no such clause exists in the ticket.

Iteration 3 supplied the ticket alone. The evaluator immediately found what two prior rounds
had not:

- **CRITICAL** -- the plan edited `docs/PLAN-2026-09-30-parallax-tutorial-stack.md`, which no
  ticket clause mentions. Citing it to D5 meant citing it to a clause outside the ticket.
- **CRITICAL** -- the plan deleted `_size_layer_repeats()`, which B2's three enumerated items
  do not name.

The lesson is not "the evaluator was stricter in round 3". It is that **a gate given a
weaker authority than the artefact will pass the artefact**, because the premise was supplied
by the party being gated. Section 2.1 now separates ticket clauses from task-prompt clauses and
section 2.2 states the authority rule the plan is held to.

### What the iteration-3 findings changed

| Finding | Change |
|---|---|
| CRITICAL: design-doc edit unsupported | **Removed** from the plan. Filed as **#92** with all five locations and reasons. Section 4.2c draws the line: a comment is in scope only when it justifies a value a ticket clause re-derives. |
| CRITICAL: `_size_layer_repeats` deletion unsupported | **Kept, with its authorisation stated.** B2's lead-in is "Anything else calibrated through the same lens is suspect", and this helper computes `ceil(scrub / repeat_size.x) + 1`, which reasons about how far the copy travels -- the displaced canvas. 4.2 now quotes the lead-in, records that the *enumeration* alone does not reach it, and gives the fallback: leave the function, since M-k shows it cannot affect the frame either way. |
| B8 (#88) uncovered | **Discharged** in 4.2b by reasoning rather than measurement -- `half_viewport` is camera-defined, so after B7 the formula's input no longer exists and cannot be re-measured. Posted to #88. |
| 4.5's bare-background metric vacuous by construction | **Replaced.** The old metric counted CHANGED pixels, which cannot distinguish a gap from art. It now counts pixels *matching* the tab background -- an absolute quantity -- with a hide-one-layer positive control and a stated prediction. |
| M-i was out-of-scope work listed as a prerequisite | **Demoted** to provenance, relabelled M-h, marked as #90's question with no #89 item depending on it. M-g added so the Forest/Foreground result has the row it was missing. |
| 4.4 claimed its contract was "fully subsumed" | **Softened** to "superseded" -- subsumption is a claim about coverage of a moving target. |
| Block numbering ran `1/5..5/5` then `6/6` | **Renumbered** `1/6..6/6`, and the assistant-written list no longer names the removed doc edits. |
| Section 7 gated a generator this ticket does not touch | **Switched** to `cargo test -p sstd-core`, the workspace's stated Rust gate. |
| N1's deferral asserted, not reasoned | **Reasoned** in 4.2b and section 8.1: B8 asks about the relationship, not the values, and #89 makes per-layer control possible without choosing it. |
| 3.2's `CanvasLayer` rejection rested on speculation | **Replaced with the ticket's own measurement.** See below. |

### The CanvasLayer correction

Iteration 2 was preceded by an earlier draft's proposal to re-test the `CanvasLayer` route,
reasoning that three of this plan's own metrics had proven vacuous so an earlier recorded
failure was not to be trusted. The gate did not raise this, but it was wrong, and the reason is
worth keeping:

**#89's own correction comment measured candidate A as "does not work at all"** -- UI stays
displaced, both bars draw 0 pixels, 63% bare tab against 1% before -- and the ticket body
reports the same 63% figure independently. That is a different observer, different evidence,
and the strongest measurement in the issue. Vacuous metrics elsewhere in a plan are not a reason
to doubt a measurement recorded elsewhere; they are a reason to check which measurement a claim
actually rests on. Claim checked: it rests on the ticket. Re-testing was dropped.

Section 3.2 now carries the ticket's measurements for both candidates A (`CanvasLayer`) and B
(`camera.enabled = false`), so neither is presented as a candidate this plan weighed and
declined.

### Verdict on the load-bearing claim

An evaluator re-derived the mechanism independently from `godotengine/godot` `4.7`
`scene/2d/parallax_2d.cpp` and `.h` and confirmed M1-M8 line by line, including that
`follow_viewport` being `true` is what makes the camera-era form in M4b measurable, and that
writing `scroll_offset = scrub * scroll_scale` per layer moves every layer at its own rate on x
(tiled) and on y (linear). **D4 is therefore satisfiable as designed**, and M-a has since
confirmed it on rendered frames at runtime rather than on paper alone.

### Iteration 3's remaining deficiencies, and one claim of its own that was wrong

The 88/100 came with seven deficiencies, all citation hygiene rather than design, and all now
closed:

| Deficiency | Closed by |
|---|---|
| `_size_layer_repeats` deletion rested on an argued reading | 4.2 restates **re-derive** as the primary disposition and removal as the conditional follow-through, and says plainly that the reading is argued, not quoted |
| `scroll_x.max_value == 6400.0` cited only to D3 | 4.3 re-cites it: the ticket names no scrub range (`grep -n 6400` on the issue returns nothing), the range is a code fact at `simulator.gd:40-41`, and D3 is the acceptance phrasing, not the authority |
| 4.5's sampling grid came from D3 | Grounded in the code instead: `simulator.gd:63` already used that exact five-point grid, so it needs no new authorisation, only a metric that can fail |
| 1280x800 has no ticket line | Grounded in the code: `simulator.gd:78` says the existing correction "depends on the window size" and `simulator.gd:125` records a figure measured "at both window sizes", so the code being re-derived is already two-size behaviour |
| M-g listed as a pending measurement | Moved out of section 5's table into 5.2 as an answered diagnostic, with "no plan item depends on this" stated |
| Section 7 and section 10 were a large share of the mass with no traceability | Section 7 compressed to the commands plus the two rules that actually bite; section 10's per-iteration changelogs moved to [gate history](PLAN-2026-10-03-parallax-scroll-offset-gate-history) |
| Two line references off by one | One accepted, one rejected -- see below |

**The rejected claim.** The evaluator reported `test_parallax_backdrop.gd:175-178` as off by
one and `:174-178` correct. Checked against the file: `:173-174` is the tail of the *previous*
`_fail` call, and the sign block is the comment at `:175-176` plus the guard and failure at
`:177-178`. So `:175-178` was right and the correction was not applied. The other half of the
claim was correct: "drift is not reproducible" is the failure at `:171`, inside the guard at
`:170`, so that citation is now `:170-171`.

Recorded because a gate report is evidence, not instruction, and adopting half of it blindly
would have put a wrong line number into the plan.

### Two structural defects the iteration-3 gate did not find

Found while applying its findings, by listing the headings rather than reading for content:

1. **`### 5.2 Measurements confirmed at 1920x1080` was filed inside section 7.** It sat between
   the `project.godot` rule and the `---` before section 8, so the measurement results were
   reachable only by scrolling past the gates, and section 5 -- the section whose whole job is
   "what has been measured" -- had no results in it. Moved into section 5.
2. **`## 9. Risks` appeared twice.** The first was not a risk table; it was the six-item
   deferred-scope list (N1-N0), which belongs with section 8's follow-ups. Retitled
   `### 8.1 Deferred, with the reason each is out of this ticket's reach`, which also forced
   each item to be read against 2.1 rather than as a loose end.

Both are the kind of thing a content-reading pass walks past: the first is a correct section
that happens to be under the wrong parent, and the second is a correct list that happens to
carry a misleading title. **Reading headings is not the same as reading text**, and a gate that
scores arguments can miss both.

