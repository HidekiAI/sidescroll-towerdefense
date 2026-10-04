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

## Iteration 5 -- 86/100, no CRITICAL, one UNCOVERED clause

Scored against the ticket alone. First round that improved on its predecessor's findings being
applied; +10 over iteration 4. **Still below the bar of 90**, and iteration 5 is past the
`MAX_ITERATIONS` of 3, so the decision to run a sixth round is @me's, not the loop's.

Prompt discipline differed from iteration 4 in one respect, and it is worth recording: the
evaluator was given the ticket in full and the plan path, and was told explicitly **not** to
read the gate history, the session checkpoint, `.opencode/sessions/`, the wiki or the source
tree -- the plan must stand alone -- and that section 10 and the header's working-tree
disposition are the planner's self-assessment and score no points for it. Whether that
produced a stricter pass than iteration 4 got is unknowable from one data point.

### The UNCOVERED finding, verbatim

> "#88 (per-band stagger) was also derived from a half_viewport * scroll_scale formula; that
> relationship needs re-measuring under the corrected canvas" -- 4.2b discharges B8 by arguing
> the formula's input is undefined once the camera is deleted and posting two questions to #88;
> no plan item performs the re-measurement the clause asks for, so B8 is reasoned away rather
> than executed.

This is the same objection iteration 3 raised and the plan declined to accept (see the
iteration-3 table, row "B8 (#88) uncovered"). **Iteration 5 sides with iteration 3.** The
clause says *re-measured*, and the plan offered *moot*. The correct disposition is a plan item:
the corrected canvas only exists after blocks `2/6` and `3/6`, so the measurement is a
post-implementation gate, not a precondition.

### Applied

- **Section 10** -- iteration-5 row added; new subsection recording that B8 was reasoned away
  and that stating an argument more firmly is not answering the objection.

### Pending @me's go-ahead for iteration 6 -- the eight findings, unresolved

Nothing below is applied. It is recorded here so the decision is made against the full list
rather than a summary of it.

| # | Finding | Resolution if a sixth round runs |
|---|---|---|
| 1 | B8 / #88 re-measurement UNCOVERED | plan item + section 7 exit condition, measured after `3/6` lands |
| 2 | Block 1/6 (`_size_layer_repeats` deletion) cites only "suspect" | out of the block map; follow-up ticket |
| 3 | `-291` ships on a six-way tie; M-b1 outstanding | run M-b1, or state the equivalence class and per-size headroom in 4.1 |
| 4 | 1280x800 gate condition unconfirmed for the `-291` check | add the exit condition, so the second resolution is gated not noted |
| 5 | ledger row attributes M-e at 1280x800 to the five-point sweep, which is D3's | split the row; condition 4 gets its own |
| 6 | D5's "comments corrected" met only in-scope; `backdrop_preview.tscn:12-14`, `:32-44`, `simulator.gd:2-11` and one design doc ship knowingly false | state that list in 4.2c as #92's, so the gap is declared rather than silent |
| 7 | B2 item 1 ("the runtime horizontal alignment added in #68's work") discharged by naming `_frame_backdrop()` without saying where that alignment lives | name the lines, or state it is inside `_frame_backdrop()` |
| 8 | 1280x800 is ticket-unspecified | already grounded in `simulator.gd:78`/`:125` at iteration 3; the gate wants it labelled a gate choice rather than ticket authority |
| 9 | 4.2c's `simulator.gd:40-41` row | drop the row, or replace the B6 claim with why the comment survives unchanged scrub range |
| 10 | section 10 is ~180 lines of self-assessment | reduce to a pointer at this file |

**Refinements not adopted verbatim:** two of the eight restate findings already in the table
(the M-b1 run-or-declare choice; citing B2 item 1 by name in block 5/6's Clause cell); the
block-1/6 move is finding 2 and the section-10 reduction is finding 10.

## Iteration 4 -- 76/100, no CRITICAL

Ten deficiencies. Scored against the ticket alone, as iteration 3 was. The full narrative is
in the plan's section 10; this is the changelog.

**Verified before adopting, not after.** Iteration 3's report contained an off-by-one line
citation that was wrong, and applying it blindly would have put a bad line number in the plan.
So the load-bearing finding here was checked against the source before any of it was written
down: `renderer_canvas_render_rd.cpp:2239-2252` does use an inclusive bound
(`for (int rx = 0; rx <= repeat_times_x; rx++)`), and `parallax_2d.cpp` does clamp
(`repeat_times = MAX(p_repeat_times, 1)`). Both confirmed. Adopted.

### Applied

- **3.1** -- "the renderer already covers the viewport at the minimum" replaced with the actual
  mechanism and the source lines: `repeat_times = N` draws N+1 centred copies; at 1 that is two
  copies at offsets 0 and +1920 spanning `[px, px+3840)` with `px in (-1920, 0)`, which always
  covers a 1920-wide viewport. 4.2's restatement of the same reasoning corrected to match.
- **5.1** -- rewritten. The two mis-diagnosed readings are now withdrawn by name, each with the
  corrected fact beside it. New closing lesson: a control reporting a startling value is a claim
  about the engine, not a verdict on the metric. M-e's counter-verification is cited here too.
- **5 header** -- reframed from "only what still has to be run" to an explicit **ledger**, with
  a named three-row outstanding list (M-b1, M-d, M-e at 1280x800) and a cross-reference to 5.2
  for everything answered. Resolves M-h appearing as both pending and answered.
- **5.2 / new 5.3** -- the M-b row now says its bare-% column does not discriminate `-291`, and
  5.3 records the closed form the gate re-derived, the five further offsets it predicts at 0.00 %,
  M-b1 as the measurement that would settle it, and the disposition if M-b1 confirms the tie.
- **4.5** -- positive control now names `Sky` specifically, with the reason forced (only the
  bottom-most opaque layer can raise a background count) and M-e's measured 0 -> 1,423,644 /
  0 -> 921,975 as evidence. The stale sentence repeating the false "0.00 % is impossible"
  inference was removed.
- **4.2** -- "leaving it in place costs nothing" withdrawn. It is not free: keeping the function
  preserves `simulator.gd:49-63`, whose `:53` and `:63` figures came from the void M-d metric.
- **4.2c** -- the rule gained a second trigger, "or the change itself makes it false", and a row
  was added for `simulator.gd:40-41`. Also merged the duplicated `backdrop_preview.tscn` rows:
  38-42 sits inside 32-44, so #92's four comments are three blocks across two files.
- **4.3** -- new **sign assertion**, with the camera-era and new-model `position.x` formulas side
  by side. Also amended the `6400.0` bullet: the old draft cited `simulator.gd:40-41` as the
  reason to keep the value, which is circular when B6 falsifies that comment.
- **7** -- the probe's exit-0 contract is now four conditions in a table, adding the one that
  was missing: the five-point scrub sweep must be zero at every point (D3's range acceptance).
  4.5 computed it; nothing gated it.
- **header** -- added "Working-tree disposition": `measure_no_camera.gd` (765 lines, untracked)
  is deleted before merge and never `git add`ed; `editor/addons/` likewise. The old "no code
  written yet" line contradicted its own section 5.
- **6** -- block map reordered script-then-scene. `simulator.gd:25` is a hard `get_node`, so the
  old order left a window where `_ready()` raised "Node not found". The new order leaves an
  inert camera: wrong frame, but nothing errors.
- **9** -- risk table gained three rows: the probe being swept into a commit, `-291` being
  mistaken for a derived optimum, and a later reader "correcting" the inverted x direction.

### Not applied, with reasons

- None. Every one of the ten is reflected above. The gate's refinements section (eight items)
  was read and none was adopted verbatim: most restate deficiencies already applied, and the
  sign assertion it suggested independently is now in 4.3.
