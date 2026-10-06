# PLAN: #101 host the parallax stack in a SubViewport with its own Camera2D

**Date:** 2026-10-06
**Ticket:** [#101](https://github.com/HidekiAI/sidescroll-towerdefense/issues/101) (OPEN, refactor)
**Parent feature:** [#68](https://github.com/HidekiAI/sidescroll-towerdefense/issues/68) (OPEN)
**Work branch:** B1–B5 run on a **new** branch cut from `trunk` when B1 starts. The
pre-implementation branch `refactor/subviewport-backdrop` was pushed, ff-merged into `trunk`
at `f7e70bc`, and deleted — it carried only this plan and the checkpoint update, so `trunk`
is where both now live. Do not commit B1 on `trunk`; branch-then-merge has no small-change
carve-out.
**Design of record (read-only):** wiki `TDD_Parallax-Background`, §3 *Rendering* and §3.1 *Node contract*
**Status:** B1 committed as `2e3145b` on `refactor/backdrop-subviewport` — scene structure
only, no framing value authored, so nothing here needed re-measuring. B2 is next. Section 9
runs before any framing value is authored; section 10's exit conditions are the gates.

---

## 1. Objective

Move the Simulator's backdrop out of the editor's root viewport into a `SubViewport` with its
own `Camera2D`, so that:

- the camera can drive the parallax as the tutorial and the wiki TDD specify, **without**
  rewriting the root viewport's `canvas_transform` — the defect #89 fixed by deleting the
  camera it had;
- the six bands register against a real screen-space reference instead of a magic
  `Backdrops.position` measured per window size;
- #88's per-band stagger and #99's clipped ground bands close at their shared root.

---

## 2. Ticket clauses

#101's eight Acceptance Criteria are normative; they are reproduced verbatim in §10, one per
exit condition. The clauses this plan must not lose sight of:

- **A1** — root viewport `canvas_transform` byte-identical before and after a full two-axis scrub.
- **A3** — *every* layer: design x = 0 at the tab's left edge within ±0.5 px, and design baseline
  y = 1320 at the tab's bottom edge within ±0.5 px.
- **A6** — `test_parallax_backdrop.gd` exits 0, a mutation reddens it, `probe_parallax_scroll.gd`
  deleted, both named in `AGENTS.md`'s gate list.
- **A8** — the comments still asserting the camera is absent (`simulator.gd:2-14`,
  `backdrop_preview.tscn:31-34`) and wiki §3 say "camera inside a `SubViewport`".

---

## 3. Existing code this builds on

The plan relocates and re-wires existing structures; it invents none.

| Existing item | Location | Role here |
|---|---|---|
| `simulator.gd:_ready()` | `editor/scripts/simulator.gd:18-45` | Owns the wiring. Connects `_scroll_x.value_changed` → `_set_scroll_offset_x` and `_scroll_v.value_changed` → `_set_scroll_offset_y`. These two lambdas are what retarget to the camera. |
| `simulator.gd:_size_layer_repeats()` | `:62-70` | Writes `parallax.repeat_times = ceil(max / repeat_size.x) + 1`. **Retained unchanged.** |
| `simulator.gd:_frame_backdrop()` | `:97-141` | The function being replaced. Resets `backdrops.position` to the captured authored base, awaits a frame, measures `sky_sprite.get_global_transform_with_canvas().origin`, then writes `position.x = authored + (tab_rect.position.x - sky_rect.position.x)` and leaves y authored. |
| `_authored_backdrops_position`, `_authored_position_captured` | `:15-16` | The idempotency base `_frame_backdrop` depends on: the correction is re-derived from the authored base every time so repeated resize calls do not drift. Retained, re-based. |
| `simulator.gd:_set_scroll_offset_x/_y()` | `:143-161` | Writes `parallax.scroll_offset.{x,y} = value * parallax.scroll_scale`. **Replaced** by camera writes. |
| `backdrop_preview.tscn` root | `editor/scenes/backdrop_preview.tscn` | Root is `BackdropPreview` **type `Node2D`**; child `Backdrops` (Node2D) holds six `Parallax2D` layers, each with one `Sprite2D` named `Sprite`. The root type is what lets the instance drop straight into a `SubViewport`. |
| `Backdrops.position` | `backdrop_preview.tscn:34` | `Vector2(-960, -291)` — the magic y that is #99. Replaced by a derived value. |
| Layer literals | `editor/assets/backdrop_layers/manifest.json` | Source of truth for `scroll_scale`, `repeat_size`, `z_index`, `height_px`. The scene file's own header calls itself "a hand-maintained transcription of it". |
| `Main/TabContainer/Simulator` | `editor/scenes/main.tscn` | `Control`, `layout_mode = 2`, `script = ExtResource("10")`. Holds `BackdropStrip` (the instance), `BackdropScroll` (`HScrollBar`, `z_index = 20`), `BackdropScrollV` (`VScrollBar`, `z_index = 20`). |
| Scrollbar anchors | `main.tscn`, comment block above each | Both are `unique_name_in_owner = true`, so `%BackdropScroll` resolves. **Geometry is owned by `main.tscn`**, not by `simulator.gd` — the script's own header states this. |
| `test_parallax_backdrop.gd` | `editor/tests/test_parallax_backdrop.gd` | The suite to rewrite. `EXPECTED_LAYERS` literal table, the two-independent-sample drift check, and the "pin the literal *before* deriving" discipline are all **kept**; only the driver changes. |
| `probe_parallax_scroll.gd` | `editor/tests/probe_parallax_scroll.gd` | **Deleted.** Reads `BackdropStrip/Camera2D` (gone) and `Backdrops/L2_near` (a layer name that no longer exists), then hangs on null because nothing calls `quit()`. Verified: exit 124 under a 60 s timeout. |
| A8 targets (A8) | `simulator.gd:2-14`, `backdrop_preview.tscn:31-34`, wiki §3 `:87-99` — the three enumerated in #101 and restated in §8 B5 | Rewritten to describe the camera inside `BackdropView/Viewport`. The `tscn:31-34` block is deleted outright by B3 with the magic offset. `test_parallax_backdrop.gd`'s camera references are covered by B4's rewrite, not by A8. |
| Engine source of record | `godotengine/godot` 4.7 `scene/2d/parallax_2d.cpp` / `.h` | Ground truth for every formula in §6. Cited, not inferred. |

---

## 4. Module ownership

| Responsibility | Owner | Called by |
|---|---|---|
| Which tabs exist, their `layout_mode`, scrollbar anchors and `z_index` | `editor/scenes/main.tscn` | — |
| Backdrop runtime wiring: repeat sizing, registration, scrollbar → camera | `editor/scripts/simulator.gd` (`TabContainer/Simulator`) | scene signals only |
| Layer authored data: `scroll_scale`, `repeat_size`, `z_index`, sprite placement, `Backdrops.position` base | `editor/scenes/backdrop_preview.tscn`, transcribed from `manifest.json` | `simulator.gd` (reads/overrides at runtime) |
| Layer numeric contract | `editor/assets/backdrop_layers/manifest.json` | both scene files, by transcription |
| Contract test for the whole stack | `editor/tests/test_parallax_backdrop.gd` | the gate in `AGENTS.md` |
| Design intent | wiki `TDD_Parallax-Background` | all of the above |

**Consequence:** `simulator.gd` may write `scroll_offset`, `Backdrops.position` and
`camera.position` at runtime, but must never author them into `backdrop_preview.tscn` — that
file's header already declares itself a transcription of `manifest.json`, and #88's
measurement notes state that hand-editing `Backdrops.position.x` there is futile because
`_frame_backdrop()` overwrites it on the next layout.

---

## 5. Reconciliation with documented intent

Wiki `TDD_Parallax-Background` §3 states: *"The rendering runtime introduces a `Camera2D`
driven by the active screen + player position ... This camera is the single input to the
parallax system."* and *"No script ticks any layer per frame: the camera transform alone
moves them."*

- **Current code contradicts this**: #89 deleted the camera and `simulator.gd:143-161` ticks
  every layer's `scroll_offset` from a scrollbar. #93 already tracks the stale doc.
- **This plan restores it.** After the change the camera is the single input again, and the
  scrollbar writes only `camera.position`. §3's sentence becomes true, with the refinement
  that the camera lives inside a `SubViewport` rather than the root — a scoping detail the
  TDD did not have to state because it did not contemplate an editor tab.

§3.1's node-contract table is already satisfied by `backdrop_preview.tscn` and stays
untouched: one `Parallax2D` per depth, per-axis `scroll_scale`, `centered = false`,
`repeat_size = (1920, 0)` on a 1920-wide texture, `scroll_offset` as the phase knob,
`repeat_times` for coverage.

**The two departures from the tutorial are deliberate and recorded, not oversights:**
sprites are authored at design y 936/1000 rather than `(0,0)` so all six bands
bottom-align to the 1320 baseline; and the y factors are ~0.85× the x factors because #68
requires both axes while the tutorial's published ladder is `(x, 1)`.

---

## 6. Technical design

### 6.1 Node structure

```
TabContainer/Simulator            Control, script=simulator.gd          [main.tscn]
├── BackdropView                  SubViewportContainer, stretch = true  [new, main.tscn]
│   └── Viewport                  SubViewport, size follows container   [new, main.tscn]
│       ├── Camera2D              anchor mode DRAG_CENTER (default)     [new, main.tscn]
│       └── BackdropStrip         instance of backdrop_preview.tscn     [moved]
│           └── Backdrops         Node2D                                [unchanged]
│               ├── Sky .. Foreground   6 x Parallax2D > Sprite2D       [unchanged]
├── BackdropScroll                HScrollBar, z_index 20                [unchanged]
└── BackdropScrollV               VScrollBar, z_index 20                [unchanged]
```

`SubViewportContainer` is the native choice (ladder rung 4): with `stretch = true` it sizes
its `SubViewport` to the container's rect every frame, so the parallax viewport always equals
the tab content rect with no size bookkeeping in `simulator.gd`. The container draws the
viewport texture bounded to its own rect, which also stops the `Node2D` strip painting across
the whole window — **`clip_contents` is set nowhere in `main.tscn` or `backdrop_preview.tscn`
today**, so the strip is currently unclipped canvas content.

### 6.2 Engine relations used

From `scene/2d/parallax_2d.cpp` and `.h`, 4.7:

```cpp
// NOTIFICATION_ENTER_TREE — camera coupling is keyed by VIEWPORT
group_name = "__cameras_" + itos(get_viewport_rid().get_id());

// parallax_2d.h defaults
bool follow_viewport = true;

// _update_scroll(), with follow_viewport true and repeat_size.x != 0
scroll_ofs.x = screen_offset.x
             - fposmod(screen_offset.x * scroll_scale.x - scroll_offset.x,
                       repeat_size.x * get_scale().x);
set_position(scroll_ofs);
```

- `screen_offset` is **zero-initialised** (`Point2 screen_offset;`) and written only by
  `_camera_moved()`, which the camera calls for parallax nodes in its own viewport group.
  With no camera it never changes — that is the source-level statement of #88's problem.
- `follow_viewport` defaults to `true`, so no layer property change is needed to opt in.
- `NOTIFICATION_INTERNAL_PROCESS` wraps `scroll_offset` into `[0, repeat_size)` per axis, but
  `_update_process()` gates internal processing on `(autoscroll.x || autoscroll.y)`. Our
  `autoscroll` is `(0,0)` everywhere, so **the wrap does not run** and `scroll_offset` holds
  whatever is written. Verified, not assumed: this kills an earlier reading that the
  scrollbar's 0..6400 range was being silently collapsed.

### 6.3 The two things `scroll_offset` must do

With a camera at world position `C` and the default DRAG_CENTER anchor,
`screen_offset = C + half_viewport` (half_viewport of the SubViewport). Substituting into
`_update_scroll`:

```
pos = screen_offset - fposmod(screen_offset * k - r, R)     per axis
```

**Registration** (#88, exit condition A3): all six layers share one `pos` only when
`fposmod(screen_offset * k - r, R)` is constant across `k`. Holding `r` at 0 leaves
`pos = screen_offset - fposmod(screen_offset * k, R)`, whose layer-to-layer spread is
`half_viewport * k` at `C = 0` — **exactly the `half_viewport * scroll_scale` term #88
measured**, so the measurement and this derivation agree.

**Parallax** (exit condition A5): the empirically established law is
`delta(pos) = camera_step * (1 - k)`, reproduced by `test_parallax_backdrop.gd` at 0.5 px
tolerance against two independent samples. It holds for any `r` constant across the step.

Both at once are satisfied by a per-layer **registration offset** held constant while the
camera moves:

```
r_k = half_viewport * k                    (candidate FORMULA-A)
```

Check at `C = 0`: `screen_offset = half_viewport`, so `screen_offset*k - r_k = 0`,
`fposmod(0, R) = 0`, and `pos = half_viewport` **for every layer** — aligned.
Now step the camera by `Δ`: `screen_offset*k - r_k = Δ*k`, so
`pos = half_viewport + Δ - Δ*k`, i.e. `delta(pos) = Δ*(1-k)` — **the parallax law, intact.**

### 6.4 OPEN — the sign of FORMULA-A must be measured, not assumed

#88's own note proposes the mirror image: *"`scroll_offset` is the correct per-layer knob ...
Each layer needs `scroll_offset.x -= half_viewport * scroll_scale.x`"*. FORMULA-A above has a
**plus**. The two disagree, and #88's note was taken on a canvas whose `screen_offset`
included a camera at `camera.position = (0,0)`; deriving the sign a third time would be
re-deriving what already went wrong once.

**Resolved by block M-1, before any value is authored.** Candidates, in the convention where
"align the stack to Sky" and "align the stack to the viewport point `screen_offset`" are two
different targets:

| Id | Formula | Aligns the stack to |
|---|---|---|
| FORMULA-A | `scroll_offset = +half_viewport * scroll_scale` | the point `screen_offset` (viewport-consistent) |
| FORMULA-B | `scroll_offset = +half_viewport * (scroll_scale - sky_scale)` | Sky (relative to the layer #88 used as reference) |
| FORMULA-C | `scroll_offset = -half_viewport * scroll_scale` | #88's note as literally written |

M-1 measures each layer's screen x at `C = 0` for all three and picks the one satisfying A3
at both 1920x1080 and 1280x771. If none satisfies both, that is a finding, not a fallback
chain: record it and re-derive with the measured `screen_offset` in hand.

### 6.5 Vertical framing

`Backdrops.position` stays as the **whole-stack** translation (one parent value moves all six
bands equally, and after §6.4 they are already aligned to each other), but is **derived**
rather than measured:

```
backdrops.position.x = -scroll_offset_registration    # design x = 0 -> tab left
backdrops.position.y = baseline_world_y - tab_height   # design y = 1320 -> tab bottom
```

with `baseline_world_y = 1320` taken from `manifest.json` (`sky.height_px`, the tallest band)
and `tab_height` read from the SubViewport's size after layout. This replaces both `-960` and
`-291`. `_frame_backdrop()` keeps its deferred-read, reset-to-authored-base, idempotent shape
(the part that already works and that `:90-96` documents at length); only the correction it
computes changes, and the sky-rect measurement at `:137-138` goes away.

`_authored_backdrops_position` remains the base so repeated resize calls stay idempotent.

### 6.6 Scrolling

`_set_scroll_offset_x/_y` become camera writes:

```gdscript
func _scrub_camera_x(value: float) -> void: camera.position.x = value
func _scrub_camera_y(value: float) -> void: camera.position.y = value
```

Bars keep their ranges: `_scroll_x.max_value = 6400.0`, `_scroll_v.max_value = 240.0` /
`min_value = 0.0`, both already set at `simulator.gd:39-43`. The camera home position must be
chosen so that `C = 0` corresponds to the strip's origin; `camera.position` is then
`home + value`.

### 6.7 Ceiling on FORMULA-A

`half_viewport * scroll_scale < repeat_size` must hold per layer or the registration term
falls into the next repeat period. Worst case is Foreground (`k = 1.3`) at a wide tab:
`half_viewport < 1920 / 1.3 ≈ 1477`, i.e. a SubViewport wider than ~2954 px. The authored
design width is 1920 and `project.godot` sets `window/viewport_width = 1920`, so this holds
at every size A3 tests, with wide margin. It is an analysis of the formula A3 requires, not
a change: recording it in-code sits under *Possible follow-ups* below.

### 6.8 What is explicitly not changed

- `_size_layer_repeats()` — kept. Its doc comment records a real-window measurement where
  `repeat_times = 1` left the tab 100% bare at `camera x = 1600`. Whether it is still
  load-bearing under the SubViewport is measured in M-3; if it is not, that deletion is a
  **separate ticket**, because #101's criteria do not call for it.
- The six layers' `scroll_scale`, `repeat_size`, `z_index`, texture paths, `centered = false`,
  sprite local positions.
- Both scrollbars' anchors, `z_index = 20`, `unique_name_in_owner`.
- `manifest.json` and `tools/gen-backdrop`.

---

## 7. Risks and how they are discharged

| Risk | Why it could bite | Discharge |
|---|---|---|
| Headless reads are meaningless | `DisplayServer.window_get_size()` returns `(0,0)` headless; this already produced a bogus "189x33 tab" reading | All measurements in §9 run with a real window (`DISPLAY` is set on this host) |
| Geometry misreports coverage | #88 recorded geometric checks reporting 0/6 layers covering the tab while screenshots plainly showed art | Render differencing everywhere: hide the layer, diff the frame, take the changed-pixel fraction |
| Void experiments | A parse error or a space/tab mismatch aborts a run and any output read from it is worthless | Validate stderr first; on a surprising result, hypothesise "my experiment was invalid" before "the code is wrong" |
| A test that asserts only static properties | The wiki decision record §3 records the probe that stayed GREEN for weeks while the feature never rendered | The suite drives the scrollbars and re-measures layer displacement; part 3 additionally pins on-disk file names literally |
| Expectation derived from the value under test | Documented in the suite's own header: deriving drift as `step * (1 - layer.scroll_scale)` let a manifest scale edit move both sides together | Literal scale pin runs **before** the derived expectation, exactly as `:151-159` does today |

---

## 8. Block map (approval required before scaffolding)

| Block | What | Files |
|---|---|---|
| **M-1** | Measure `screen_offset` and per-layer screen x at `C = 0` for FORMULA-A/B/C, two window sizes. **Pick the formula.** | throwaway probe (never committed) |
| **M-2** | Step the camera by `(120, 40)` twice; confirm `delta(pos) = step * (1 - scroll_scale)` on both axes survives the registration offset (A5) | throwaway probe |
| **M-3** | Measure whether `_size_layer_repeats` still moves the bare-background percentage | throwaway probe |
| **M-4** | Hide-and-diff tables: A2's changed-pixel fractions + Foreground bottom-edge test, and A4's bare-background ≤ 2% at five x positions, at 1280x771 and 1920x1080 | throwaway probe |
| **M-6** | Hide-and-diff each scrollbar at A7's sizes (1920x1080 and 1280x800) with a positive control | throwaway probe |
| **A1 probe** | Snapshot root `canvas_transform`, run a full two-axis scrub, compare bytes (A1) | throwaway probe |
| **B1** | Scene: add `BackdropView` + `Viewport` + `Camera2D`, move the `BackdropStrip` instance inside, keep both scrollbars | `editor/scenes/main.tscn` |
| **B2** | `simulator.gd`: new node paths, scrollbar → `camera.position`, delete `_set_scroll_offset_x/_y` | `editor/scripts/simulator.gd` |
| **B3** | `simulator.gd`: replace `_frame_backdrop`'s measured correction with the derived §6.5 form; delete `-291`/`-960` from `backdrop_preview.tscn` | `editor/scripts/simulator.gd`, `editor/scenes/backdrop_preview.tscn` |
| **B4** | Rewrite `test_parallax_backdrop.gd` (§9.1, incl. all three mutation runs); delete `probe_parallax_scroll.gd` | `editor/tests/` |
| **B5** | Docs per A6 and A8: add the suite + both mutation checks to `AGENTS.md`'s gate list; update A8's three targets | `AGENTS.md`, files below, wiki |

**A8's four comment locations**, taken verbatim from #92's table so B5 is a checklist and
not a research task:

1. `editor/scenes/backdrop_preview.tscn:12-14` — justifies `repeat_size_x = 1920` with
   "the camera can pan forever".
2. `editor/scenes/backdrop_preview.tscn:32-44` — derives the `Backdrops` offset from
   `ANCHOR_MODE_DRAG_CENTER` and a camera view of `x -960..960, y -540..540`.
3. `editor/scenes/backdrop_preview.tscn:38-42` — sizes the sky's 240px overshoot against
   "the camera's 240px vertical travel".
4. `editor/scripts/simulator.gd:2-11` (file header) — states the `Camera2D` contract in the
   first person as the thing #89 deleted.

Plus wiki `TDD_Parallax-Background` §3 (`:87-99`), whose diagram at `:99` shows `Camera2D` as
a child of `World`.

**Ordering — and why B1/B2 come before the M-blocks that measure them.** B1 and B2 encode no
framing value: B1 is pure scene structure (a container, a viewport, a camera, a moved
instance) and B2 is a signal rewiring from `scroll_offset` to `camera.position`. Neither can
be wrong in a way M-1's answer would change. B3 is the first block that *consumes* M-1's
result — it writes the derived registration and framing — so B3 waits for M-1.

| Phase | Blocks | Why that side |
|---|---|---|
| **Structure first** | B1, B2 | No framing value is chosen here, so nothing needs measuring first. B1 also *creates* the camera M-1 needs. |
| **Formula** | M-1, M-2, M-3 | M-1 is blocking: FORMULA-A/B/C disagree on a sign, and §6.4 forbids settling it by re-derivation. Runs after B1 so `screen_offset` derives from the `SubViewport` (= the tab), not the root viewport. |
| **Consume it** | B3 | First block that writes a framing constant. |
| **Verify** | M-4, M-6, A1 probe | Measure the finished result: A2/A4 rendering, A7's rewired scrollbars, A1's root `canvas_transform`. Running these earlier would measure the defect being fixed. |
| **Test + docs** | B4, B5 | B4's three mutation runs come last. |

Sequence: **B1 → B2 → M-1 → M-2 → M-3 → B3 → M-4 → M-6 → A1 → B4 → B5.**
Each B-block is one coherent commit.

---

## 9. Measurement plan — runs BEFORE any framing value is written

Per #88's notes and #89's B3 verification method. All with a real window.

- **M-1 (blocking, A3):** with the B1 camera in place at `camera.position = Vector2(0,0)`,
  write each of FORMULA-A/B/C into every layer's `scroll_offset`, then read each layer's
  screen-space origin (`get_global_transform_with_canvas().origin` — the read
  `_frame_backdrop()` already uses; **not** `layer.position`, which excludes the parent
  translation A3 measures against).

  **Must run after B1.** Only there does `screen_offset` derive from the `SubViewport`'s
  size, which equals the tab. A pre-B1 probe camera would sit in the root viewport and yield
  a 1920x1080 `screen_offset`: it could rank the six layers against each other, but it could
  never satisfy A3's placement half, which is measured against the tab.

  **Pass requires both halves of A3**, at 1920x1080 **and** 1280x771:
  - registration — all six layers within ±0.5 px of each other;
  - placement — design x = 0 at the tab's left edge within ±0.5 px, and design baseline
    y = 1320 at the tab's bottom edge within ±0.5 px.

  Fail: record all three tables and re-derive from the measured `screen_offset` rather than
  falling through to the next candidate.
- **M-2:** with the winning formula, step the camera by `(120, 40)` twice and confirm
  `delta(pos) = step * (1 - scroll_scale)` for all six layers on both axes — this is the
  relationship §6.3 claims FORMULA-A preserves, and it must survive the registration offset.
- **M-3:** set `repeat_times = 1` vs the computed value and compare bare-background at
  x = 0/1600/3200/4800/6400. Decides whether `_size_layer_repeats` is load-bearing. **If it
  is not, do not delete it here** — file it.
- **M-4 (A2, A4):** at **1280x771 and 1920x1080**, hide/show each of Hills, Forest and
  Foreground and diff for the changed-pixel fraction (A2's non-zero test), assert
  Foreground's bottom edge is at or above the tab's bottom edge (A2's geometric half), and
  compute the **bare-background percentage, asserting ≤ 2%** at x = 0, 1600, 3200, 4800,
  6400 at both sizes (A4).
- **M-6 (A7):** at **1920x1080 and 1280x800** — note A7's sizes differ from A4's — hide
  `BackdropScroll` and diff, hide `BackdropScrollV` and diff, and assert each gives a
  **non-zero** changed-pixel fraction. Include a **positive control that is known to draw**
  (a Control already verified visible) so a zero result can be attributed to the bar rather
  than to a broken harness.
- **A1 probe:** snapshot `root.canvas_transform`, run a full two-axis scrub, compare bytes.

### 9.1 Test rewrite contract (B4)

Kept from the current suite: file-name literals, `centered`/`repeat_size` assertions,
the two-independent-sample drift check, `DRIFT_TOLERANCE_PX`, and the rule that the literal
pin runs before the derived expectation.

**Changed — the `scroll_scale` pin's source, required by A5.** A5 says the literal must be
*pinned from `manifest.json`* so that *mutating a manifest scale reddens the suite*. Today's
suite cannot satisfy that: `test_parallax_backdrop.gd` never reads `manifest.json`. Its
`EXPECTED_LAYERS` (`:46-53`) is a hand-maintained copy of the manifest's numbers, so editing
`manifest.json` reddens **nothing** — only editing `backdrop_preview.tscn` does. B4 therefore
loads `scroll_scale` from `manifest.json` at runtime and asserts the scene's value matches it:

| Mutated | Result after B4 |
|---|---|
| `manifest.json` scale only | **red** — pin moved, scene did not |
| `backdrop_preview.tscn` scale only | **red** — scene moved, pin did not |
| both, to the same new value | green — they genuinely agree (correct) |

File-name assertions stay as **hardcoded literals, not read from the manifest**, because the
suite's own header (`:37-41`) records why a shared constant cannot be its own guard: a rename
that moves manifest and scene together must still be caught. `scroll_scale` is the one value
where manifest-vs-scene disagreement *is* the contract, so it reads the manifest; the on-disk
file name remains asserted literally.

Changed: the driver is `Camera2D` under `BackdropView/Viewport` rather than a deleted node;
Part 1 asserts the two bars each move `camera.position` on their own axis and leave the other
held (the existing anti-coupling assertions at `:92-104` port directly, with `camera.position`
in place of the current field); a new Part asserts registration and placement (A3) using the
formula picked in M-1, with each layer's registration term pinned against its own manifest
scale.

**Three mutations are explicit steps, not expectations.** After the suite is green:

1. **A5, manifest side** — set one `scroll_scale_x` in `manifest.json` (Hills `0.5` →
   `0.42`, the exact edit the header at `:39-40` records as having fooled an earlier draft),
   run the suite, record the exit code being **non-zero**. Restore, re-run green.
2. **A5, scene side** — set the same `scroll_scale_x` in `backdrop_preview.tscn` alone, run,
   record **non-zero**. Restore, re-run green. This is the mutation the suite's header says
   refuted two early drafts, so it must be demonstrated, not asserted.
3. **A6** — break the scrollbar → `camera.position` wiring in `simulator.gd` (point one
   bar's handler at nothing, or drop the connection), run, record **non-zero**. Restore,
   re-run green.

All six exit codes (three red, three green) go in the commit message. A green suite that
cannot be reddened has not been shown to test anything.

`probe_parallax_scroll.gd` is deleted rather than repaired: it probes a scene structure that
no longer exists, references layer names retired by the restack, and has no `quit()` on its
error path.

---

## 10. Exit conditions

Reproduced verbatim from #101; each maps to a block.

| # | Acceptance criterion (verbatim) | Block | Proof |
|---|---|---|---|
| A1 | Root viewport `canvas_transform` is byte-identical before and after a full two-axis scrub across the whole 0..6400 / 0..240 range. | B1-B2 | A1 probe |
| A2 | At a windowed 1280x771, each of Hills, Forest and Foreground shows a non-zero changed-pixel fraction by render differencing (hide the layer, diff the frame), **and** Foreground's bottom edge is at or above the tab's bottom edge. Geometry alone is not sufficient: #88 recorded geometric coverage reporting 0/6 layers while screenshots plainly showed art. | B3, M-4 | M-4 tables |
| A3 | For **every** layer: design x = 0 lands at the tab's left edge within ±0.5 px, and the design baseline y = 1320 lands at the tab's bottom edge within ±0.5 px. | M-1, B3 | M-1's placement table (both window sizes) + the suite's registration Part |
| A4 | Bare-background ≤ 2% at x = 0, 1600, 3200, 4800, 6400, at **both** 1280x771 and 1920x1080, measured by render differencing. | B2, M-3, M-4 | M-4 bare-background table, sizes and ≤ 2% asserted in the block |
| A5 | Each layer displaces at its own `scroll_scale` on both axes across a camera step; the expectation is `camera_step * (1 - scroll_scale)`, with the literal scale pinned from `manifest.json` *before* the expectation is derived so that mutating a manifest scale reddens the suite rather than moving both sides together. *(A5's pin source changes — see §9.1; today's suite does not read the manifest.)* | M-2, B4 | M-2 table (drift law preserved), suite exit 0, and §9.1 mutation 1 recorded **non-zero** after editing `manifest.json`'s Hills `scroll_scale_x` |
| A6 | `test_parallax_backdrop.gd` exits 0; a mutation that breaks the scrollbar wiring makes it exit non-zero; `probe_parallax_scroll.gd` is deleted; both the suite and the mutation check are named in `AGENTS.md`'s gate list. | B4, B5 | green exit, the three recorded non-zero exits from §9.1's explicit mutation runs, and a gate-list grep |
| A7 | Both scrollbars show a non-zero changed-pixel fraction at 1920x1080 and 1280x800, by hide-and-diff, with a positive control that is known to draw. | B2, M-6 | M-6 tables (both of A7's sizes, with the positive control) |
| A8 | Every comment still asserting the camera is absent or that the stack lives in the root viewport is corrected, and wiki `TDD_Parallax-Background` §3 describes the camera inside `BackdropView/Viewport`. Targets as enumerated in #101: `simulator.gd:2-14`, `backdrop_preview.tscn:31-34`, wiki §3 `:87-99`. | B5 | grep those three locations for "camera is no longer used" / "camera removed" returning nothing, plus a read of wiki §3 |

**Not a gate here:** #100's zoom control, #90, #91, #87, #94 — all out of scope per #101.

---

## 11. Stop conditions

- If M-1 shows **none** of FORMULA-A/B/C satisfies A3 at both window sizes: STOP, record the
  three measured tables, re-derive from the measured `screen_offset`. Do not begin **B3** —
  it is the first block that writes a framing constant. (B1 and B2 have already run: they
  carry no framing value, which is why §8 puts them ahead of M-1.)
- If A1 fails after B1: the camera reached the root viewport. STOP; that is exactly the
  defect #89 fixed and the structure is wrong, not the value.

---

## 12. Possible follow-ups (not this ticket)

Listed so they are not silently folded into #101. None of these is required by any #101
Acceptance Criterion.

- **Record §6.7's ceiling in code.** A one-line comment at the registration formula naming
  `half_viewport * scroll_scale < repeat_size` and the FORMULA-B upgrade path. §6.7 already
  documents the bound in full for whoever needs it.
- **Delete `_size_layer_repeats()` if M-3 shows it is no longer load-bearing.** #101 keeps
  it; the deletion needs its own ticket and its own measurement.
- **#100** zoom/scale-to-fit, **#90** the sky's sun, **#91** depth-derived `scroll_scale`,
  **#87** the Map Editor's static stack — all already filed, all out of scope here.
