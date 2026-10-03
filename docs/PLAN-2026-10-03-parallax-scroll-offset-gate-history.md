# Plan gate history: #89 parallax scroll-offset

Iteration-by-iteration record for
[PLAN-2026-10-03-parallax-scroll-offset](PLAN-2026-10-03-parallax-scroll-offset) section 10.
Kept out of the plan body so that body is plan items only; the scores, the invalid-pass
finding and the load-bearing conclusions stay in the plan, since a reviewer needs those.

---

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
- **Moved the `PLAN-2026-09-30` doc correction out of section 8** into 4.3a. **Iteration 3
  reverted this in the opposite direction** -- out of the plan entirely, into #92 -- which is
  the same finding arriving twice by opposite routes: iteration 1 saw a self-contradiction
  ("planned and not planned"), iteration 3 saw an unsupported item. The contradiction was the
  symptom; the missing clause was the cause.
- **Corrected 4.5's precedent**: `capture_backdrop.gd:29` / `capture_all_tabs.gd`, not
  `probe_parallax_scroll.gd` (which is a headless node-graph probe with no frame read).
- **Corrected section 7**: `editor/project.godot` IS tracked; the reason to restore it is
  that it drifts on `--import`, not that it is uncommitted.
- **Added M-0**: read `layer.screen_offset` and `layer.position` on the live nodes, so M1
  and M5 -- the two claims the whole design rests on -- are measured rather than inferred
  from source.

### Post-iteration-2 additions, retained as history

Iteration 2 scored as a pass, but surfaced defects worth closing before any code is written.
Recorded rather than silently dropped. **The fourth item below was later found to be the
iteration-2 mis-citation itself** -- it counts the `simulator.gd` file header among the
comments D5 requires be corrected, which is exactly the reasoning iteration 3 overturned:

- **M10** -- the x wrap is invisible in rendered pixels but NOT in `layer.position`, which
  is what the test reads. A wrap inside the suite's two-step window would make
  `test_parallax_backdrop.gd:170-171`'s reproducibility check report a false red on correct
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
- **4.2** enumerated `simulator.gd:49-63` and `simulator.gd:74-98` among the comments D5
  requires be corrected. Superseded in part: `74-98` is still in scope under B1 (it justifies
  a re-derived value), while `49-63` went with the `_size_layer_repeats` disposition and the
  file header moved to #92. It also corrected two attributions that stand: the
  `(2784, 419.32)` figures are comment prose rather than code constants, and the
