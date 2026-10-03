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

#89 carries no `Expected Result` heading; its normative clauses are these.

From the issue body:

- B1 (Consequence): "`simulator.gd _frame_backdrop()` and `Backdrops.position =
  Vector2(-960, -780)` were both tuned against this displaced canvas ... So those two
  values must be re-derived."
- B2: "the 240px vertical travel clamp and its justification" are among the values that
  "must be re-derived".
- B3 (Verification method): "Real window, not headless"; "Assert a control's canvas-space
  rect intersects `get_visible_rect()`"; "hide it and diff the frame"; "Include a positive
  control that is known to draw".
- B4 (Notes): "Both bars being `z_index = 20` remains correct and is worth keeping".

From the correction comment ("Where that leaves the fix"):

- B5: "The camera cannot stay in the default canvas, and it cannot simply be switched
  off."
- B6: "stop using a camera to scroll the strip at all, and drive each
  `Parallax2D.scroll_offset` directly from the scrollbar value times that layer's own
  `scroll_scale`."
- B7: "No camera means no canvas transform, so the UI is never displaced".

From the task prompt's Definition of done (the acceptance contract for this pass):

- D1: "The `Camera2D` node and every reference to it are gone."
- D2: "Both scrollbars render real pixels, proven by hide-and-diff with a positive
  control, at 1920x1080 and at 1280x800."
- D3: "Horizontal scrub shows art across the full 6400 px range, at both window sizes."
- D4: "Each layer moves at its own `scroll_scale` -- the two-axis parallax is still
  observable."
- D5: "Framing constants are re-derived from measurement, and the comments stating them
  are corrected to match what was actually measured."

Explicitly NOT required, and therefore not planned (section 8):

- N1: #88's per-band stagger.
- N2: A `tools/gen-backdrop` change for taller art.
- N3: Any change to the scrollbars' anchors or `z_index` (B4 says keep).
- N4: #87, or the retirement of the 13 old tracked PNGs.

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
jumps by +1920 at each period boundary. `test_parallax_backdrop.gd:170` compares two
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

`_size_layer_repeats()` (`simulator.gd:64`) derives
`repeat_times = ceil(scrub / repeat_size.x) + 1 = ceil(6400/1920) + 1 = 5`. That formula
was derived for the camera model, where (per M4b) the authored copy sat at
`S.x - fposmod(S.x * scale.x, 1920)` and travelled with the camera across the whole scrub,
so repeats had to span `[S.x - 1920, S.x + 1920]` for the whole 6400 range. Under M4 the
copy is stationary in `(-1920, 0]`, so the number of repeats needed is a function of the
VIEWPORT WIDTH alone and not of the scrub. **This helper is therefore re-measured, not
assumed dead or assumed right**: it stays unchanged unless a measurement shows it wrong
or wasteful. D3 ("art across the full 6400 px range") is the acceptance test for it
either way.

`_frame_backdrop()` (`simulator.gd:99`) becomes correctable on both axes (M5), so its
"Only x" comment and its measured constants are replaced.

---

## 4. Target design

### 4.1 `editor/scenes/backdrop_preview.tscn`

- **Delete** the `[node name="Camera2D" type="Camera2D" parent="."]` block. B5, D1.
- **Re-derive** `Backdrops.position`. The authored `(-960, -780)` encodes the camera's
  centred view (`540 - 1320 = -780`, per its own comment) and a `-960` half-viewport that
  has no meaning once there is no centring. B1 voids it; the replacement is whatever M5's
  measurement says.
- The `Camera2D`-dependent comment above `Backdrops` (lines 32-44) is rewritten to state
  the no-camera geometry. D5.
- Untouched: the six `Parallax2D` layers, their `scroll_scale`, `repeat_size`, `z_index`,
  and their `Sprite2D` children. Those are transcribed from `manifest.json`, which
  `backdrop_preview.tscn:4-6` names as the single source of truth and which this ticket
  does not change.

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
- `_size_layer_repeats()`'s doc block at `simulator.gd:49-63`, whose first line reads
  "Sizes every layer's horizontal repeat run to cover the full scrub." Under M4 the copy
  is stationary, so the run's length is a function of the VIEWPORT (plus one wrap), not of
  the scrub -- which is precisely the premise M4b invalidates. Reworded to the measured
  derivation, and the `100% bare at x=1600` / `89-92% bare` figures restated from M-d
  rather than carried over, since they were taken through the displaced canvas. D5.
- `_scroll_v.max_value`: the 240px clamp stays until measurement says otherwise, and its
  justification comment at `simulator.gd:42-44` is replaced by the M-e measurement. B2,
  D5. The `(2784, 419.32)` / `(2816, 199.48)` figures at `simulator.gd:79-81` are
  comment prose, not code constants, and are removed with that block.
- `_size_layer_repeats()`: re-measured per M-d; changed only if the measurement requires
  it. Section 3.1.
- File header (`simulator.gd:2-11`): currently explains the `Camera2D` contract in the
  first person -- "the tutorial's Parallax2D multiplies the camera offset by scroll_scale
  per axis". Rewritten to the no-camera contract. D5.

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
- **Keep, and pin to the measurement, `scroll_x.max_value == 6400.0`**
  (`test_parallax_backdrop.gd:111-112`). This assertion is the only in-suite pin of the
  scrub RANGE END, and it is what makes D3 ("art across the full 6400 px range") a
  statement about the scrub's extent rather than an unbounded one. The bar's range is not
  changed by this ticket -- 6400 is the strip width, not a framing constant -- so the
  assertion stays; what M-d supplies is the evidence that art actually renders at and
  beyond that end, and that evidence goes into `probe_render_visibility.gd` (4.5), which
  can measure bare background where a node-graph suite cannot.
- **Re-pin, or replace, `scroll_v.max_value == 240.0`**
  (`test_parallax_backdrop.gd:108-109`) and rewrite its justification comment at
  `test_parallax_backdrop.gd:106-107`, which today reads "Vertical travel is clamped to
  the generator's 240px overscan: past it the 1320px sky band would reveal its own top
  edge." That sentence states the 240px constant B2 calls suspect, so it is one of the
  "comments stating them" D5 requires be corrected. M-e decides the value; the assertion
  follows it, and its comment is rewritten to the measured travel. If M-e returns 240.0,
  the assertion is kept unchanged and only the comment is corrected -- the point is that
  the number is measured, not inherited.

### 4.3a `docs/PLAN-2026-09-30-parallax-tutorial-stack.md` -- correct the model it documents

That plan's section 3.1 and its Phase 3 describe the `Camera2D`-driven node contract this
ticket deletes. D5 requires the stated comments to match reality, and a plan document
that describes a deleted node as the design is the same defect in prose form. This is a
correction of that file, not new design work, and it is why it is an item here rather than
in section 8.

### 4.4 `editor/tests/probe_parallax_scroll.gd` -- delete

It reads `BackdropStrip/Camera2D` (`:28`), which D1 requires to be gone. It is already
dead independent of this change: `:29` reads `Backdrops/L2_near`, a node removed by the
restack commit `ba2cbb4` in favour of the six manifest layers. Its contract is fully
subsumed by `test_parallax_backdrop.gd`. Deleting it is the minimal way to satisfy D1
without rewriting a probe that tests a scene shape which no longer exists.

### 4.5 `editor/tests/probe_render_visibility.gd` -- new, real window

The only new non-document file. D2 and B3 require hide-and-diff proof with a positive
control at two window sizes, and #89's whole history is a defect that every transform
read and every `z_index` probe reported as fine.

The repo's precedent for frame capture is `editor/tests/capture_backdrop.gd:29`, which
does `root.get_texture().get_image().save_png(path)` in one statement, and
`editor/capture_all_tabs.gd`, which splits the same pattern across two. Neither sets a
window size. `probe_parallax_scroll.gd` is **not** that precedent -- it is a headless
node-graph probe with no `DisplayServer` call and no frame read.

Neither capture script sets a window size, and B3 clause 1 rules out running them
headless (`DisplayServer.window_get_size()` is (0,0) there), so this probe is also where
the sizing has to come from: the resolution is supplied per run on the command line as
`--resolution 1920x1080` and `--resolution 1280x800`, and the probe asserts the size it
actually got rather than assuming it. A run whose reported size is (0,0) exits non-zero
as an invalid experiment, not as a zero-pixel result.

Requirements it encodes:

- A positive control that is known to draw, so a 0-pixel result is distinguishable from a
  broken differ.
- Canvas-space rect intersected with `get_visible_rect()` (B3 clause 2).
- Changed-pixel count for each bar against a baseline frame (B3 clause 3).
- Run at both 1920x1080 and 1280x800.
- D3's range-end evidence: bare-background percentage at scrub x = 0 and x = 6400, so
  the assertion `test_parallax_backdrop.gd:111-112` makes about the scrub's extent has
  rendered-frame backing.

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
| M-d | Is `repeat_times = 5` correct, over-provisioned, or insufficient across 0..6400? | bare-background percentage at scrub x = 0, 1600, 3200, 4800, 6400, both sizes | whether `_size_layer_repeats` changes (3.1, D3) |
| M-e | How much vertical travel is safe before the sky's top edge shows? | scrub y upward at both sizes, hide-and-diff | `_scroll_v.max_value` (B2) and the `240.0` assertion (4.3) |
| M-f | Do both bars contribute real pixels? | hide-and-diff with positive control, both sizes | D2 |

M-0 runs first and exists because M1 and M5 are the two claims the whole design rests on,
and this ticket's own history is a defect that every property read reported as fine. They
are read from the live node, not inferred from the engine source.

Each measurement is recorded with its window size and its positive control, and the
number that ships is the one measured, not the one predicted.

---

## 6. Block map (proposal, for approval before scaffolding)

Coding-assistant Phase 1. Test files are the assistant's, not blocks. Numbering is global
and in dependency order.

| Block | file:line | What to write (not how) | Depends on |
|---|---|---|---|
| 1/5 | `editor/scenes/backdrop_preview.tscn:29` | remove the `Camera2D` node | - |
| 2/5 | `editor/scripts/simulator.gd:25,32-39` | replace the camera writes with per-layer `scroll_offset` writes, each bar on its own axis | 1/5, M-a |
| 3/5 | `editor/scenes/backdrop_preview.tscn:45` | re-derive `Backdrops.position` | 2/5, M-b, M-c |
| 4/5 | `editor/scripts/simulator.gd:99` | re-derive `_frame_backdrop()` on both axes | 3/5, M-b, M-c |
| 5/5 | `editor/scripts/simulator.gd:45` | set the vertical clamp from the measured travel | 4/5, M-e |

Assistant-written, not blocks: `test_parallax_backdrop.gd` (rewrite), the deletion of
`probe_parallax_scroll.gd`, `probe_render_visibility.gd` (new), and the correction to
`docs/PLAN-2026-09-30-parallax-tutorial-stack.md` (4.3a).

GDScript holes must be **real static errors**, because GDScript has no ahead-of-time
compiler and a hole that only fails at runtime shows no red squiggle. So each block is
built around a wrong-typed initializer (`var total: int = "text"`) or a non-void return
with an empty body ("Not all code paths return a value"). Never `pass`, never a bare
`return`, never a cast. Every variable, parameter, return type and `@onready` is
explicitly typed, 4-space indent, ASCII only.

---

## 7. Gates

```bash
cargo test -p gen-backdrop                       # 29 tests, generator (unchanged by this work)

$HOME/bin/godot4 --headless --path editor --script res://tests/test_parallax_backdrop.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_image_to_map.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_terrain_brush.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_override_merge.gd
```

Gate on the **exit code**, never on a printed `failures=0` line.
`test_screen_store` exits 1 on Godot 4.7.2 for pre-existing #85 reasons and is not a gate
for this work.

The new probe is a gate too, run twice, because D2 is the acceptance item that only
rendered evidence can settle and a probe nothing runs is not a gate:

```bash
$HOME/bin/godot4 --path editor --resolution 1920x1080 --script res://tests/probe_render_visibility.gd
$HOME/bin/godot4 --path editor --resolution 1280x800  --script res://tests/probe_render_visibility.gd
```

Both must exit 0, and the probe exits non-zero on a zero-pixel result for either bar, on a
positive control that contributes nothing (a broken differ), or on a reported window size
of (0,0) (an invalid experiment).

`editor/project.godot` IS tracked (`git ls-files --error-unmatch` returns it) and drifts
on every `--import` run, which Godot 4.7.2 does on open. Restore it after every Godot
run: `git checkout -- editor/project.godot`. Never `git add -A` or `git add .` --
`editor/addons/` is 516 MB of vendored third-party code.

---

## 8. Possible follow-ups (NOT this ticket, NOT planned)

- **N1 -- #88, per-band stagger.** The task prompt and the checkpoint both say its
  `half_viewport * scroll_scale` formula was measured through the displaced canvas and
  must be re-measured. That re-measurement is not planned here; this ticket only makes
  per-layer control *possible*.
- **N2 -- taller art from `tools/gen-backdrop`.** The conclusion that vertical travel
  needs a generator change was measured through the wrong canvas. M-e re-tests the
  premise. If a gap is real it needs its own ticket; no generator change is planned and
  no generator file is touched.
- **N3 -- the scrollbars' anchors and `z_index`.** B4 says the current values are correct.
  No change planned.
- **N4 -- #87 and the retirement of the 13 old tracked PNGs.** Untouched; still blocked
  separately.

---

## 9. Risks

| Risk | Mitigation |
|---|---|
| M3/M4 wrong, so D4 fails | Section 5 M-a measures the rate before any constant is written; the delta idiom in `test_parallax_backdrop.gd` is retained so a wrong rate is a red test, not a wrong picture |
| Framing constants re-derived through a stale method | B3's two checks are mandatory and neither is a transform read; the previous constants were wrong precisely because a transform read said otherwise |
| `repeat_times` under-provisioned after the model change | M-d measures bare-background percentage across the whole range at both sizes |
| Scope creep into #88 or the generator | Section 8 lists both as follow-ups; the hard cap in the plan gate applies |
| `project.godot` or `addons/` accidentally committed | Section 7's explicit-path rule |

---

## 10. Plan gate

**Iterations:** 2. **Final score: 91/100** (bar is 90). Evaluator: independent agent,
same model both iterations, given only the ticket text, the task prompt's Definition of
done, and this document -- not the planner's reasoning.

| iteration | score | outcome |
|---|---|---|
| 1 | 84 | FAIL. No CRITICAL, but two uncovered clauses and seven deficiencies. |
| 2 | 91 | PASS. No CRITICAL, no uncovered clause. |

### Items removed or moved at iteration 1

No item was removed as CRITICAL -- iteration 1 reported the cap as not triggered. The
scoping was already correct; what failed was coverage of two clauses and seven factual
claims. These are the changes iteration 1 forced:

- **Added to 4.3** the two range assertions `test_parallax_backdrop.gd:108-109`
  (`scroll_v.max_value == 240.0`, D5) and `:111-112` (`scroll_x.max_value == 6400.0`,
  D3), including the decision rule for each once M-d and M-e report.
- **Corrected M8**: `follow_viewport` defaults to `true`, not `false`
  (`parallax_2d.h`). The "no change needed" conclusion survives, but the reason is now
  "`screen_offset` is (0,0) so (4) would subtract zero", not the default value.
- **Corrected M4**: the x branch is `-fposmod(-scroll_offset.x, 1920)`, not
  `fposmod(scroll_offset.x, 1920)`. Range is `(-1920, 0]`, not `[0, 1920)`.
- **Moved the `PLAN-2026-09-30` doc correction out of section 8** into 4.3a, where it is a
  D5 item rather than a self-contradictory "planned and not planned".
- **Corrected 4.5's precedent**: `capture_backdrop.gd:29` / `capture_all_tabs.gd`, not
  `probe_parallax_scroll.gd` (which is a headless node-graph probe with no frame read).
- **Corrected section 7**: `editor/project.godot` IS tracked; the reason to restore it is
  that it drifts on `--import`, not that it is uncommitted.
- **Added M-0**: read `layer.screen_offset` and `layer.position` on the live nodes, so M1
  and M5 -- the two claims the whole design rests on -- are measured rather than inferred
  from source.

### Items added after the passing iteration 2

Iteration 2 passed, but surfaced defects worth closing before any code is written. They
are recorded here rather than silently dropped:

- **M10** -- the x wrap is invisible in rendered pixels but NOT in `layer.position`, which
  is what the test reads. A wrap inside the suite's two-step window would make
  `test_parallax_backdrop.gd:170`'s reproducibility check report a false red on correct
  code. The drift measurement must not start at `scroll_x = 0` (a period boundary), and
  the suite must assert the window is wrap-free rather than assume it.
- **M11** -- `Foreground`'s sign convention inverts. `test_parallax_backdrop.gd:19`'s "a
  layer at 1.3 (Foreground) moves backwards" was true only of the camera model
  (`step * (1 - 1.3)`). Under M3 every layer moves forward and Foreground merely moves
  faster, so that comment must change and an explicit ordering assertion replaces the
  negative-drift check.
- **M9** -- `get_scale()` is (1,1) for every layer, which is what makes the modulus exactly
  `repeat_size.x`. Recorded because a future scaled layer changes M10's arithmetic.
- **Section 7** now gates on the new probe at both resolutions, since D2 is the acceptance
  item only rendered evidence can settle.
- **4.2** now enumerates `simulator.gd:49-63` and `simulator.gd:74-98` among the comments
  D5 requires be corrected, and corrects two attributions: the `(2784, 419.32)` figures are
  comment prose rather than code constants, and the `manifest.json` single-source-of-truth
  claim belongs to `backdrop_preview.tscn:4-6` rather than to the task prompt.

### Verdict on the load-bearing claim

Iteration 2's evaluator re-derived the mechanism independently from
`godotengine/godot` `4.7` `scene/2d/parallax_2d.cpp` and `.h` and confirmed M1-M8 line by
line, including that `follow_viewport` being `true` is what makes the camera-era form in
M4b measurable, and that writing `scroll_offset = scrub * scroll_scale` per layer moves
every layer at its own rate on x (tiled) and on y (linear). **D4 is therefore satisfiable
as designed.**