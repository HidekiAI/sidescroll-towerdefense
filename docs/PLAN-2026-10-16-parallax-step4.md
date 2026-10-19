# Parallax Step 4 — `WorldView` + `ParallaxDraw` replace `Parallax2D` (#107)

**Epic:** #68. **Ticket:** #107 (full body, not a summary). **Predecessor:** step 3 (#106).
**Rewritten:** 2026-10-18 against the whole #107 body — the 2026-10-16 draft gated at 94 because it
was scored against a summary of the ticket and covered only the drawer. **Gate:** plan-ticket-gate
>= 90 with #107's body in the prompt.

> **TL;DR:** Build one `WorldView` scene (`SubViewport` + its own `Camera2D`, a `ParallaxDraw` per
> plane, an empty `World`, an editor-only `Overlay`), let a loader turn `stack.json` + measured PNGs
> into bridge placements, and draw them per Math section 7 — then delete the six `Parallax2D` nodes,
> `backdrop_preview.tscn`, the 240 px clamp and `_size_layer_repeats`. Eleven gates, split into six
> headless and five windowed. The drawer keeps two written views: `_draw()` is the design of record,
> Sprite2D-per-placement is retained as the alternative (section 2). Detail below.

---

## 1. Ticket reconciliation

Every line of #107's **Scope** and **Acceptance criteria** and where this plan discharges it. A row
the ticket does not raise is not in this table.

| #107 item | Plan section | Gate |
|---|---|---|
| `WorldView` scene: `BackPlane`/`FrontPlane` `CanvasLayer` (−1/1), `World`, `Camera2D`, `Overlay` (layer 2, `follow_viewport_enabled`) | 4.1 | G1, G3 |
| `ParallaxDraw._draw()`: fills then art under `draw_set_transform`; redraw on `ViewState` change | 4.3, 4.4 | G5, G7, G12, S1 |
| A loader turns `stack.json` + measured PNGs into the bridge request (TL;DR: "two `ParallaxDraw` nodes that ask the bridge for placements"; Scope: the `ParallaxDraw` clause) | 4.2 | G1, G15 |
| Game camera controller: follow both axes, clamp, teleport = cut, `zoom` always 1 | 4.5 | G6, G13 |
| Runtime stretch in the game scene; `project.godot` stays `disabled` | 4.6 | G19 |
| Remove six `Parallax2D`, `backdrop_preview.tscn`, 240 px clamp, `_frame_backdrop` correction, `_size_layer_repeats` | 4.9 | "no `Parallax2D` remains" |
| Remove `BackdropStrip` and the old `BackdropView` wiring it superseded (TL;DR: "The Simulator uses it"; Scope: "`backdrop_preview.tscn` removed") | 4.9 | "no `Parallax2D` remains" |
| Rewrite `test_parallax_backdrop.gd`; delete `probe_parallax_scroll.gd` | 6 | reddens under mutation |
| `World` holds one `ScreenGridDisplay` per occupied screen | **deferred to #108** (4.1 keeps the node, empty) | G16 belongs to #108 |
| Spike G5 decides the tiling path | 4.3.1, 7 | G5 |
| G15 status HUD on fallback | 4.7 | G15 |

**Deferral, recorded on the ticket (#107 comment, 2026-10-18).** #107's Scope asks for `World`
(`Node2D`) with one `ScreenGridDisplay` per occupied screen, but its Acceptance criteria list G1, G3,
G5, G6, G7, G9, G12, G13, G15, G19 and S1 — **not G16**, the only gate that exercises per-screen
drawing and culling. Those gates belong to #108 (Map Editor live parallax on `WorldView`). This plan
therefore keeps the `World` and `Overlay` **nodes** in the scene so the tree matches the ticket, and
leaves them empty. The amendment is posted, so ticket and plan agree; without it this deferral would
be a plan item the ticket does not support.

---

## 2. Two views on the drawer (kept deliberately)

Recorded on **#107 (comment, 2026-10-18)** from @me's directive: *"rather than erasing removing what
was planned, write it up as 'another option is to do it …' way so we can observe it from 2 views."*
Both views are written down. **#107 and Math section 7 remain the design of record; only 2.1 is
implemented.** The alternative in 2.2 is documentation of a rejected design so its reasoning stays
visible — it is not an implementation option (2.2 closes on why).

### 2.1 Design of record — one `_draw()` per plane

`ParallaxDraw` is a `Node2D` that draws in `_draw()` and never instantiates children. It holds the
`placements` array the bridge returned and repaints when the `ViewState` changes.

- The bridge already returns everything the drawer cannot re-derive (`PlacementEntry`: `name`,
  `file`, `plane`, `draw_order`, `first_copy_left_px`, `copy_count`, `screen_top_px`, `scale`,
  `period_px`, `art_h_px`, `fill_above`, `fill_below`), so the drawer adds no state of its own.
- Geometry is one `draw_set_transform` plus one `draw_texture_rect` per layer (section 4.3).
- Cost: 6 repeating layers x 1 rect = 6 draw calls per redraw; redraws happen only on a `ViewState`
  change, never per frame.

### 2.2 Alternative option — one `Sprite2D` per placement

The 2026-10-16 draft's approach, retained as written: `ParallaxDraw` manages a `Sprite2D` per
placement, each with `region_enabled` and a `region_rect` covering one period, positioned from
`first_copy_left_px + copy_index * period_px`, with `Sprite2D`'s own `texture_repeat` and a
`scale` of `zoom`. Vertical position comes from `screen_top_px`.

What it buys:

- Engine-side culling per sprite, and no `_draw()` override to reason about.
- Instancing is trivial to inspect in the editor's remote scene tree — useful when a placement
  looks wrong, because each copy is a node you can select.
- `Sprite2D` handles texture filter and mipmaps by the same rules as the rest of the project.

What it costs, and why the ticket did not choose it:

- **The period must be recomputed per copy when `zoom != 1`.** `place_layer` returns
  `first_copy_left_px` already in zoomed screen space and `scale = zoom`, so a sprite copies at
  `period_px * zoom` — correct — but the count and the left edge are only right if the sprite's
  region is authored in unscaled art px and the node scale does the rest. Two places can disagree
  about which space a value is in, which is precisely the class of bug the coordinate-suffix rule
  exists to prevent.
- Node churn: entering/exiting zoom, resize or a stack swap creates and frees up to 30 nodes,
  where `_draw()` repaints in place.
- The fills (`fill_above` / `fill_below`) still need `_draw_rect()` calls, so the alternative does
  not remove the `_draw()` path — it gains a second one.

**Not a third tiling path.** #107's acceptance criterion 2 enumerates exactly two: a single
transformed `draw_texture_rect` with `tile = true`, or one call per copy. Its Fallback clause names
only the move to Bevy (O3a) / Fyrox (O5a), reached when "G5 [is] still red under the documented
per-copy fallback". Sprite-per-copy sits on neither ladder and is not implemented here; recorded on
#107 (comment, 2026-10-18) alongside the directive that keeps this view written down.

**Possible follow-up ticket (not this ticket):** adopt sprite-per-copy as the tiling path if spike G5
*and* the per-copy fallback both fail while a Godot draw path is still wanted — that is, before the
#107 Fallback clause's engine move would be reached. Out of scope for step 4; adopting it needs its
own ticket, and the placements and their geometry are identical either way.

---

## 3. Architecture

```plantuml
@startuml
skinparam componentStyle rectangle

package "Godot editor process" {
  [main.tscn / Simulator tab] as SIM
  [WorldView.tscn] as WV
  [ParallaxDraw (BackPlane)] as DRAW_B
  [ParallaxDraw (FrontPlane)] as DRAW_F
  [parallax_loader.gd] as LOADER
  [CameraController] as CAM
  [SstdBridge (GDExtension)] as BR
}

database "stack.json + PNGs" as STACK
database "sstd_config.sqlite3 / world" as WORLD

SIM --> WV : instantiate
WV --> LOADER : anchor + stack key + view
LOADER --> STACK : read manifest, measure PNGs
LOADER --> BR : parallax_placements(request)
BR --> [sstd-core::parallax] as CORE : place_layer per layer
CORE --> BR : PlacementEntry[]
BR --> LOADER : { placements }
LOADER --> DRAW_B : back-plane entries
LOADER --> DRAW_F : front-plane entries
DRAW_B --> DRAW_B : _draw() on ViewState change
CAM --> WV : cam_center / zoom (ViewState)
LOADER --> SIM : journal + status HUD
@enduml
```

**Ownership.** `sstd-core::parallax` owns all geometry (`place_layer`, `map_rectangle`,
`clamp_center`, `default_anchor`, `Stack::validate`, `journal_line`) — it never touches the
filesystem or an image. The bridge owns the wire form (`PlacementsRequest`, `PlacementEntry`) and
the stack's validation response. GDScript owns I/O: reading `stack.json`, measuring PNGs, resolving
the anchor, loading textures, and drawing. Nothing in GDScript re-derives a placement.

---

## 4. Technical design

### 4.1 Scene graph

```
WorldView (Control, script res://scripts/world_view.gd)     -- scene: scenes/world_view.tscn
├── SubViewportContainer (stretch = true, full-rect anchors)
│   └── Viewport    SubViewport   size 1920 x 1080, update_mode = ALWAYS
│       ├── BackPlane   CanvasLayer layer = -1
│       │   └── ParallaxDraw   (script res://scripts/parallax_draw.gd)
│       ├── World        Node2D          (empty in #107; ScreenGridDisplay per screen is #108)
│       ├── FrontPlane  CanvasLayer layer =  1
│       │   └── ParallaxDraw
│       └── Camera2D
└── Overlay   CanvasLayer layer = 2, follow_viewport_enabled = true   (editor-only, empty)
```

- **`Control`, not `Node2D` (deviation recorded while implementing, 2026-10-18).** This section's
  first sketch said `Node2D`. Godot resolves a Control's anchors against the nearest Control
  ancestor, and a `Node2D` in between breaks that chain back to the root viewport — so a `Node2D`
  WorldView under `TabContainer/Simulator` would make `SubViewportContainer` fill the whole editor
  window instead of the tab. WorldView is instanced into a Control and must fill it, so the
  Control chain stays intact. Everything else in this section is unchanged.

- The `SubViewport` keeps #101's isolation: its `Camera2D` cannot displace editor UI, which is what
  G3 asserts (every editor `Control` keeps its global rect after camera moves and zooms).
- `BackPlane`/`FrontPlane` split by `PlacementEntry.plane`; `plane_rank` already orders the bridge
  response back-before-front, so each node filters with `entry.plane == <mine>` and preserves order.
- `Overlay` lives outside the `SubViewportContainer`'s control of the canvas but is a `CanvasLayer`,
  so it draws over the viewport. It is present and empty; #108 fills it.
- **Anchors are mandatory** (Godot4-Learnings §1): `SubViewportContainer` gets `anchor_right =
  anchor_bottom = 1.0`, `grow_horizontal = grow_vertical = 2`. Without them the rect is `0x0` and
  the `SubViewport` is driven to `2x2` — nothing draws, nothing errors.

### 4.2 Loader — `editor/scripts/parallax_loader.gd`

The only component that reads the stack. It exists because `Stack::validate` explicitly cannot:
*"this module has no filename and no filesystem"*.

Responsibilities, in order, each logged:

1. **Read** `stack.json` (path from the caller; default `res://assets/backdrop_stacks/
   neutral_default/stack.json`). Keep the raw text — the request's `stack` key takes the parsed
   object, so the loader parses once and reuses it.
2. **Measure each layer PNG** into `LayerArtInput` (`width_px`, `height_px`, `is_rgba`,
   `fully_opaque`, `top_row_opaque`, `bottom_row_opaque`). The bridge documents these as *"the PNG
   decoder the caller already has"*. GDScript maps them without a hand-written loop:
   `Image.get_format() == Image.FORMAT_RGBA8` for `is_rgba`;
   `Image.detect_alpha() == Image.DETECT_ALPHA_NONE` for `fully_opaque` (a C++ scan, so a 1920 x
   ~1080 layer costs one call, not two million pixel reads); `top_row_opaque`/`bottom_row_opaque`
   walk one 1920-pixel row each — cheap enough to do at load.
3. **Resolve the anchor.** Required by the wire contract (`parallax.rs`: *"the anchor is required,
   because choosing a default anchor is the loader's job"*). The caller passes one. For a world with
   no anchor declaration, home = the view centre = `default_anchor`'s value for a one-screen map at
   1920 x 1080, i.e. `(960, 540)`. **No new bridge function** — `default_anchor` already exists in
   `sstd-core` and re-deriving it in GDScript would be a second source of truth for a value the
   caller supplies; YAGNI on a new `#[func]`.
4. **Call** `SstdBridge.parallax_placements(request_json)` and parse `{ placements: [...] }`.
5. **Fallback on failure.** On `{"error": "stack_invalid"}` or `{"error": "<detail>"}`: log one
   `parallax.validate FAIL …` line **per message** (section 4.8), log exactly one
   `parallax.load FALLBACK …` line, load `neutral_default`, set the status HUD, and continue. Never
   `assert()` — G15's mutation is *"return the rejected stack anyway; or `assert()` on failure"*,
   so crashing is a red, not an escape.

Complexity: O(layers) PNG measurement + O(layers) placements, all at load; nothing per frame.

### 4.3 `ParallaxDraw._draw()` — Math section 7, verbatim

Layers in ascending `draw_order`; per layer, fills first, then art. Let `view_w`/`view_h` be the
`SubViewport` size and `art_bottom = screen_top_px + art_h_px * scale`:

```gdscript
func _draw() -> void:
    for entry: Dictionary in _placements:            # already back-plane-then-front, ascending draw_order
        var art_bottom: float = entry.screen_top_px + entry.art_h_px * entry.scale
        var fill_above: Color = entry.fill_above
        if fill_above.a > 0.0 and entry.screen_top_px > 0.0:
            draw_rect(Rect2(0.0, 0.0, _view_w, entry.screen_top_px), fill_above)
        var fill_below: Color = entry.fill_below
        var below_height: float = _view_h - art_bottom
        if fill_below.a > 0.0 and below_height > 0.0:
            draw_rect(Rect2(0.0, art_bottom, _view_w, below_height), fill_below)
        draw_set_transform(
            Vector2(entry.first_copy_left_px, entry.screen_top_px),
            0.0,
            Vector2(entry.scale, entry.scale))
        draw_texture_rect(_texture_of(entry.file),
            Rect2(0.0, 0.0, entry.period_px * entry.copy_count, entry.art_h_px),
            true)                                     # tile = true; rect is UNSCALED art px
    draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)  # reset before the next caller
```

with `texture_repeat = TEXTURE_REPEAT_ENABLED` on the node. The rect is in unscaled art pixels and
the transform applies the zoom, so the on-screen period is `period_px * scale` (= `period_px * zoom`)
— the requirement G5 asserts at both `1` and `0.25`.

**A fill draws with its layer, never in a separate pass.** Intended so that a nearer fill can cover a
farther layer's art. A fill is skipped only when `transparent` or its height `<= 0`.

**CORRECTED 2026-10-19 by measurement (section 4.3.2).** This paragraph claimed G12 pins "fills draw
with their layer, never in a separate pass", with a fills-first pass putting "sky between the hills
and the ground". Measured: hoisting every fill into a first pass produces a **bit-identical frame**
at five cameras and three zooms (0 of 2,073,600 pixels differ, 10 frame comparisons), so the mutation
cannot be killed by any assertion at those cameras — the mutant is a rendering no-op, not a weak
guard. The same measurement shows the fill rects are drawn under the **previous** layer's transform
(`draw_set_transform` persists), which is why only each plane's first layer has a screen-space fill;
correcting that is likewise invisible with this stack. See 4.3.2 for the measurements and the
follow-up.

**G7 CORRECTED 2026-10-19 by measurement (section 4.3.2).** The 24 px overhang *was* derived from
source as `fill_below`, on the reasoning that `place_layer` repeats **x only**, so nothing tiles
downward past `art_bottom`. Measured, the bottom-24-row colour is **ground-coloured ART**, not a fill:
skipping the `fill_below` draw leaves G7's two checks green, and the mutation that kills the fill
claim is the fills-skipped one over G12's rows. G7 therefore pins the observed colour (`#2E3834`,
below the Sky fill `#C2CAD0`), and the drawer's own correctness on those rows is carried by G12.
The wiki's stale `#C4CCD2` correction still stands (TDD correction, section 7).

#### 4.3.1 Spike G5 — the one genuine unknown

**Unverified until the spike** (Math section 7 says so in as many words): whether
`CanvasItem.draw_set_transform` scales the `tile = true` repeat, i.e. whether the repeat is computed
in the item's local pre-transform space.

- **If yes (expected):** one `draw_texture_rect` per layer; 6 draw calls per redraw.
- **If no:** fall back to one `draw_texture_rect` per copy. `place_layer`'s own comment bounds the
  count — *"at most 5 copies per layer (section 3), 6 repeating layers, so at most 30 draw calls per
  redraw at zoom 0.25 instead of 6"*. Per copy: `Rect2(first_copy_left_px + copy_index * period_px,
  screen_top_px, period_px, art_h_px)`, each under the same transform, `tile = false`.

Either way the cost is negligible because redraws are event-driven (4.4). **The spike decides the
code path, not the design.**

Second G5 assertion, independent of the path: at zoom `1`, `texture_filter` must be `NEAREST`, or the
probe row above the Hills art top blends and the mutation `set ParallaxDraw.texture_filter = LINEAR`
turns it red.

**CORRECTED 2026-10-19 (section 4.3.2): that second assertion is unkillable as scoped.** At zoom `1`
the strips are drawn at their own 1:1 pixel scale, so `NEAREST` and `LINEAR` sample identically; the
mutation survives every check in both windowed suites. The filter is still asserted (it is a real
requirement at non-1 zooms) but it has no mutant behind it, and is recorded as such rather than
counted as falsified coverage.

#### 4.3.2 Measured corrections from the windowed suites (2026-10-19)

Both windowed suites are green headless-of-DISPLAY (`DISPLAY=:0.0`), exit code 0, and their mutation
battery is recorded here because three of the plan's own derived claims did not survive measurement.
Every number below is measured, with the command that produced it.

**G5's shape changed, for a measured reason.** A whole-period pan cannot kill the scale mutation at
all: `first_copy_left_px` is the only pan-dependent quantity the drawer reads, and a whole-period pan
leaves it at the same phase, so the frame is identical in the correct code *and* in the mutant. The
plan's "zoom 0.25 repeats the frame after a pan" form was therefore replaced by an **in-frame**
assertion of the plan's literal claim — the on-screen period is `period_px * zoom`, so at zoom 0.25
the frame must repeat every **480 px across a single probe row** (`ZOOM_QUARTER_PERIOD_PX = 480`,
row 620). Measured: 0 mismatches correct, **1205 mismatches under mutation (b)**
(`Vector2(scale_value, scale_value)` -> `Vector2.ONE`), so the check is falsifiable and the mutation
is killed by it. Zoom 1 keeps the whole-period pan (`ZOOM_ONE_PAN_PX = 19200`) with a MEASURED budget.

**The zoom-1 pan residual is 152 px, and its cause is data, not code.** `ZOOM_ONE_PAN_PX = 19200` is
the smallest pan that is whole-period for all six strips (rates k/10, k = 1,2,3,5,7,13), so all six
return to phase; the residual 152 px comes from `stack.json` carrying `parallax_depth` rounded to 4
decimal places (2.3333, 0.4286, -0.2308), which makes Foreground's rate 1.30000522 instead of 1.3 —
0.9984 px of drift over 19200, which `snap()` (`floor(v + 0.5)`) turns into a genuine 1 px shift, and
the stripe of rows 1015..1033 is that shift. `_pan_rounding_budget()` is therefore **160** (152
measured + 8 hair), never 0: a 0 budget would be a vacuous exact-equality assert. A node search over
P = 19200m ± 12 (m <= 400) found **no** pan where all six layers are whole-period, so an exact
reproduce assert is unachievable with the current data. Full-precision `parallax_depth` in the
generated stack is a `tools/gen-backdrop` (issue #105) matter, filed as a follow-up rather than fixed
here.

**A harness bug of my own, worth recording.** The probe first captured at the scene's default camera
(0,0) and then panned to 960 + 19200, giving a delta of 20160 — not whole periods (Sky drifts 2016 =
1 period + 96), so the diff was 1202952 px and looked like a rendering defect. Capturing after
`move_camera_to(960.0, 540.0)` restored the 152. The lesson: **pan invariance depends on the DELTA
being whole-period, not on the absolute position.** The same probe settled the startup question
separately — the backdrop does draw at startup (0 clear pixels of 2073600, 7 placements at (0,0),
zoom 1), so the old "runtime rendering unverified" note is closed as a non-defect for the new stack.

**Per-layer periods, measured at zoom 0.25 (anchor 960/540):** `sky` 1920, `clouds_high`,
`clouds_low`, `hills`, `forest`, `foreground` all 1920, and **`sun` 130** (a single non-tiling
sprite). So a full-frame 480-periodicity check does NOT hold at zoom 0.25: measured 1590 mismatches,
bbox col 591..1877, rows 455..661 — mostly the `sun` band (rows 455..485, up to 62 per row) plus
1-5-per-row copy-coverage edges in rows 595..661. Rows 0..454 are clean, but row 250 measured
**variation 0** (flat), which makes a period compare there vacuous, hence probe **row 620** (variation
204 correct, 78 under the mutation — still above the non-vacuity floor). Zoom 1's period-480 control
is 734081 mismatches on the anchor frame, which is the non-vacuity proof that at zoom 1 the period is
not 480.

**Mutants, honest count.** Killed: (a) skip the `fill_below` draw — killed by G12's first check (rows
400/1000/1070 lose the ground colour), NOT by G7; (b) drop the placement scale — killed by G5's
zoom-0.25 check (1205 mismatches). Unkillable as scoped, reported rather than papered over: (c) a
fills-first pass and (d) drawing the fills in screen space by resetting the transform first — both
measured **bit-identical** to the correct render at zoom 1/0.5/0.25 x cam y 540/1540/2000/3000
(0 differing pixels in every comparison), so no assertion at those cameras could kill them; the
`texture_filter = LINEAR` mutation is likewise unkillable at zoom 1 (above). The correct stack's fills
are all the same ground colour and the front-plane foreground art already covers the band the fills
would cover, which is why order and transform are unobservable in the render with this data.

**Candidate follow-up (not changed here; the drawer matches 4.3 verbatim and no gate fails):** the
fill rects are screen-space by construction yet are drawn under the previous layer's transform, so
only each plane's first layer (Sky on BackPlane — zero-height at these cameras; Foreground on
FrontPlane) actually paints in screen space. The one-line fix (reset the transform before the fills)
was measured as a no-op render with the current stack, so it is a latent-correctness change that wants
a stack whose mid-layer fill colour differs from the ground, plus its own ticket.

### 4.4 Redraw policy — on `ViewState` change, never per frame

`WorldView` polls and detects change: each frame it compares the current
`(cam_center_world_px_x, cam_center_world_px_y, zoom, view_w_px, view_h_px)` against the last values
handed to the bridge. On any difference it re-queries `parallax_placements` once, stores the result,
and calls `queue_redraw()` on both `ParallaxDraw` nodes. The two fields `place_layer` does not read
cannot trigger a query.

- **Resize:** setting the `SubViewport` to 1280 x 720 updates `view_w_px`/`view_h_px` and queues
  **exactly one** redraw — G9's mutation is *"read the size once at startup"*, so the size must be
  re-read each poll, not cached at `_ready()`.
- Polling rather than signal-driven is deliberate: `Camera2D` position and `SubViewport.size` are
  plain properties with no change signal, and a one-frame latency in a parallax background is
  invisible while the polling cost is a tuple compare.

### 4.5 Game camera controller — `editor/scripts/camera_controller.gd`

- **Follow on both axes.** Target position each frame, so G13's *"no lag"* holds: the mutation
  *"follow x only (y stays 1572)"* and *"follow the tick position instead of the interpolated one
  (lag of one tick)"* must both go red. That means following the **interpolated** render position,
  not the tick position.
- **Clamp in GDScript** with `clamp_center(center, view_extent, bound_low, bound_high)` — that
  function is the design of record for the small-bound case (`bound_high - bound_low < view_extent`
  returns `bound_low + half`, *not* the midpoint, per Math section 5's zoom-0.25 example). It lives
  in `sstd-core` and is already unit-tested; a re-derivation in GDScript would be a second source of
  truth for exactly the branch *"gets written wrong"*. A same-line comment points at it so no one
  re-derives it later.
- **Teleport = cut.** Set `cam_center` to the destination in one frame with no tween — G6's mutation
  is *"add a tween"*.
- **`zoom` always `1`.** G6's second mutation is *"allow zoom"*, so the controller refuses
  `zoom != 1` rather than ignoring it.

### 4.6 Runtime stretch — G19

In the game scene's `_ready()`, never in `project.godot`:

```gdscript
get_window().content_scale_size = Vector2i(GAME_LOGICAL_WIDTH_PX, GAME_LOGICAL_HEIGHT_PX)  # 1920 x 1080
get_window().content_scale_mode = Window.CONTENT_SCALE_MODE_CANVAS_ITEMS
get_window().content_scale_aspect = Window.CONTENT_SCALE_ASPECT_KEEP
```

`project.godot` stays `content_scale_mode = disabled`, so the editor panel letterboxes its own
`WorldView` at any size. The mutation *"put the stretch in `project.godot`: the editor panel
letterboxes"* keeps both halves honest. The logical size is named as a constant rather than written
twice.

### 4.7 Status HUD (G15)

A `Label` on the Simulator scene showing the last load outcome — `neutral_default` plus the failing
rule on fallback, the stack key otherwise. G15 asserts it *shows* the fallback, so the text is set
even when the load ultimately succeeds after a fallback.

### 4.8 Core + bridge: make the rule line correct (G15)

**The gap, verified 2026-10-18:** `journal_line(msg, stack_key, layer)` formats `layer={}` from its
third parameter, but it has **no production caller** — only two tests (`parallax.rs:1570`,
`:1735`), each hand-writing `"Sky"` / `"Foreground"`. The bridge returns
`{"error": "stack_invalid", "errors": [...]}` with raw messages and no journal lines. So there is no
path today that logs G15's *"logs the rule line"*, and even if one existed, nothing could supply a
correct layer: `ValidationMessage` has `severity/code/message/field/value` and no `layer`.

`Stack::validate`'s own doc comment already states the intent this violates: *"Every message carries
`.with_field(...)` and `.with_value(...)` so `journal_line` can render a complete line; put the
layer's name in the message for the per-layer rules."* The name currently reaches only the
human-readable `message` text, which is not something `journal_line` can parse back out.

**Change (three edits, all inside the existing design):**

1. `error.rs` — `ValidationMessage` gains `layer: Option<String>` and a builder
   `with_layer(...)`, matching the existing `with_field`/`with_value` idiom directly above it.
   **No `#[serde(default)]`** (this plan originally prescribed one; mutation on 2026-10-18 showed
   it changes nothing, because serde's missing-field path returns `None` for any `Option` — the
   attribute is only load-bearing on non-`Option` fields, which is why the rest of the codebase
   still uses it). The additive contract is pinned by
   `validation_message_layer_is_optional_and_spelled_layer`, whose killable half asserts the
   literal `"layer"` wire key.
2. `parallax.rs` `Stack::validate` — inside the existing
   `for (layer_index, layer) in self.layers.iter().enumerate()` loop, tag each **per-layer** message
   with `.with_layer(layer.name.as_str())`. Whole-stack messages (`schema_version`, `max_layers`,
   `art_count`) stay `None` and render `-`, consistent with how `field`/`value` render.
   **Two refinements made while implementing, both recorded here rather than silently:** `base_opaque`
   is also tagged (`base_layer.name` — one layer, unambiguous, and it is a per-layer rule that sits
   outside the loop only because finding the base needs the whole plane), while `draw_order_unique`
   and `draw_order_depth` are deliberately left `None` because each fires over a *pair* of layers and
   there is no single correct name; their message text names both and the raw `errors` array travels
   beside the journal lines.
3. `parallax.rs` `journal_line` — drop the `layer: &str` parameter and read
   `msg.layer.as_deref().unwrap_or("-")`. The format string is unchanged, so the greppable contract
   `parallax.validate FAIL stack=… layer=… field=… value=… rule=…` survives the change.

**Version bump, NOT part of #107 — follow-up ticket #114.** Dropping a parameter from the `pub`
`journal_line` is a breaking signature change, and this repository's standing policy marks such a
change with a minor version bump (`sstd-core` is at `0.1.0`). Nothing in #107 mentions crate
versions, so the bump is filed separately rather than planned here. It has no external reader to
inform: the crate is unpublished and its only consumer is the in-workspace `sstd-editor-bridge`
(`path = "../sstd-core"`). The two tests that call `journal_line` directly move with the change
either way.

**The bridge then returns `journal: [...]`** alongside `errors: [...]` on `stack_invalid`, one
formatted line per message, so GDScript logs them without re-implementing the format. The loader
adds its single `parallax.load FALLBACK …` line.

Scope justification: this is not a drive-by refactor. G15 is in #107's acceptance list and requires
that the rule line be logged with the layer identified; `journal_line`'s documented contract is
*"which stack, which layer, which field, what value, which rule"*. No other ticket owns it, because
no other ticket has a caller for `journal_line`.

### 4.9 Removals

| Remove | Where it lives today |
|---|---|
| Six `Parallax2D` nodes | `backdrop_preview.tscn` (also deletes the scene) |
| `_frame_backdrop` measured correction, `_size_layer_repeats`, the 240 px clamp | `simulator.gd` |
| `BackdropStrip` / the old `BackdropView` wiring superseded by `WorldView` | `main.tscn` |
| `probe_parallax_scroll.gd` (+ `.uid`) | `editor/tests/` — already deleted in the working tree |
| 240 px vertical overscan clamp + its comment | `simulator.gd:46-49` (`_scroll_v.max_value = 240.0`) |

**`project.godot` is left alone.** The MCP enable lines are committed, so `git checkout --
editor/project.godot` after an editor run is safe for the drift but must not drop them.

---

## 5. Gate plan

Split by whether a renderer exists — headless runs `RendererDummy`, so it cannot prove anything is
drawn (Godot4-Learnings §3).

**Headless (G1, G3, G6, G9, G13, G15)** — `godot4 --headless --path editor --script
res://tests/<name>.gd`, gate on the **exit code**, never on the printed `failures=` line.

**Windowed (G5, G7, G12, G19, S1)** — `DISPLAY=:0.0`. Pixel gates read
`SubViewport.get_texture().get_image()` and assert on numeric colours/rows, because that returns
values a test can compare. The MCP (`get_editor_screenshot`, `assert_visual_match`,
`set_visual_baseline`) is the **human-facing** check for these same frames: it writes a PNG, and a
model without image input cannot inspect it, so it can corroborate a gate but never be one.

| # | Section | Asserts | Mutation that must go red |
|---|---|---|---|
| G1 | 2.2 | instantiate `WorldView`; `BackPlane`/`FrontPlane` visible; `last_placements()` == bridge result | `BackPlane.visible = false`; skip one layer |
| G3 | 2.2 | after camera move + zoom every editor `Control` keeps its global rect | move the `Camera2D` out of the `SubViewport` |
| G6 | 2.2 | `CameraController` refuses `zoom != 1`; teleport moves `cam_center` in one frame | add a tween; allow zoom |
| G9 | 2.2 | `SubViewport` 1280x720 updates `view_w_px`/`view_h_px`, **exactly one** redraw queued | read the size once at startup |
| G13 | 2.2 | two-row map, `cam_center` == clamped player position each frame on **both** axes, no lag | follow x only; or follow tick position (one-tick lag) |
| G15 | 2.2 | rejected stack → `neutral_default`, rule line, one `parallax.load FALLBACK`, status HUD shows it, no crash | return the rejected stack; or `assert()` |
| G5 | 2.3 | Sky repeats every 1920 px at zoom 1 and 480 px at 0.25; edge probe row 695 shows cloud/sky, no Hills bottom-row component | remove the `draw_set_transform` scale (period reads 1920 at 0.25); set `texture_filter = LINEAR` |
| G7 | 2.3 | bottom 24 rows read Sky `fill_below` = `#C2CAD0FF`, not the clear colour | set the fill to `transparent` |
| G12 | 2.3 | camera 1000 px below anchor: rows 480..1080 read Forest `fill_below`, row 470 reads Forest art | draw all fills before all art → row 700 reads sky |
| G19 | 2.3 | game sets `content_scale_size (1920,1080)`, mode `canvas_items`, aspect `keep`; 1920x1200 window → 60 px bars; editor stays `disabled` | put the stretch in `project.godot` |
| S1 | 2.3 | golden frames at anchor, `+1000` x, top row; small per-pixel tolerance | change one `parallax_depth` → diff exceeds tolerance |

**S1 mechanism substitution, recorded on #107 (comment, 2026-10-18).** Verification §2.3 specifies
golden frames "via gRPC `capture_screenshot`", which needs grpcurl plus a running gRPC server;
neither exists here, so the gate as written cannot run. This step uses the scripted
`SubViewport.get_texture().get_image()` path the same section already names in its header
instruction, because it returns numeric pixel values a test can compare, and records the MCP
screenshot as the human-facing check. The gRPC path stays intended if a server is stood up later;
§2.3 needs correcting either way (section 7).

**Wiki step-row correction:** the step-4 row in the wiki omits G19 while #107's acceptance criteria
include it. **The ticket wins**; the row gains G19.

---

## 6. Test plan and falsifiability

Six GDScript suites, all gated on exit code:

| Suite | Covers |
|---|---|
| `test_parallax_backdrop.gd` (rewritten) | G1, G9, redraw policy, placements equality, and the D1-D5 drawer seam |
| `test_parallax_bridge.gd` (exists) | G15's `stack_invalid` + `journal` shape |
| new `test_world_view.gd` | G3, scene tree shape, anchors present |
| new `test_camera_controller.gd` | G6, G13 |
| windowed `test_parallax_pixels.gd` | G5, G7, G12, S1 |
| `test_parallax_stretch.gd` | G19 |

`probe_parallax_scroll.gd` is deleted, not rewritten — it exited 0 while reading static properties
and never instantiated the scene, which is how a hidden strip stayed green. `test_parallax_backdrop`
must instantiate `WorldView`, so it cannot pass by reading statics.

**`editor/scripts/parallax_draw.gd` and `editor/tests/test_parallax_draw.gd` are untracked working
copies (never committed) written against the Sprite2D alternative (2.2).** Both are rewritten to the `_draw()` design
of record: the drawer loses its `Sprite2D` children, and its assertions become `_draw()`-shaped.
Nothing carries across from the Sprite2D version except the harness setup — name, failure counter,
`quit()` on the exit code.

**As executed (2026-10-18), that paragraph's premise did not survive contact.** `parallax_draw.gd`
was committed as the drawer of record (section 4.3), and the table above names six suites — with
no `test_parallax_draw.gd` among them. Rather than leave a seventh suite the plan does not list,
its five guards were folded into the rewritten `test_parallax_backdrop.gd` (D1 identity of the
stored entries, D2 replace, D3 clear, D4 the two fill spellings, D5 the bare-placement draw path)
and the file was deleted. Absorbing them rather than dropping them keeps the drawer seam covered
by the suite that already owns "placements equality", which is the same claim one level down.

Folded suite, measured: 30 checks, rc=0 headless. Five mutants, all five KILLED, each turning
exactly the expected checks red:

| Mutant | Turns red |
|---|---|
| `BackPlane.visible = false` in `world_view.tscn` | `G1: BackPlane is visible` |
| the first placement routed to the front plane | the plane-membership check and the slice-concatenation check |
| `view_w_px` pinned to 64 in `_poll_view_state` | `G9: the new query carries the new view size`, `G9: the drawer stored the new view size` |
| change detection removed (`if false:`) | `G1: ONE bridge call`, `G9: a resize queues exactly one redraw`, `policy: an unchanged ViewState queues nothing` |
| `apply()` deep-copying its argument | `D1: the entries are the SAME dictionaries` |

One harness lesson from the first, green-looking run: assigning the recording loader BEFORE
`add_child` records nothing, because `WorldView._ready()` builds its own loader and overwrites the
assignment. The suite read `got 0` requests while the bridge had plainly been queried — a wrong
experiment reported as a vacuous guard, which is the failure mode the mutation rules exist to
catch. The swap now happens after `_ready()` and before the first `_process`.

**Every assertion is mutation-verified before it is trusted**, and each mutation is validated before
it is interpreted: confirm the substitution fired and the mutant still **parses** before reading its
result. A tab injected into a space-indented file is a parse error, not a mutation, and the
experiment is void. A build failure is also a void experiment, not a green one.

**Counts are measured, never carried over** — re-derive both before claiming one, and keep the
command beside the number:

```bash
cargo test -p sstd-core          -- --list | tail -1
cargo test -p sstd-editor-bridge -- --list | tail -1
```

Record N tests exist and M have a confirmed mutant; N and M differing is the interesting fact.

Both suites stay green — the change in 4.8 is additive except for the `journal_line` signature, and
its own tests (calling `journal_line` without the layer parameter, plus the back-compat
`ValidationMessage` deserialization test) go red-first.

**As executed (2026-10-18), only half of that was red-first.** The `journal_line` tests were: the
two-argument call does not compile until the parameter is dropped, so the signature change went
red as a compile error and green on implementation. The back-compat test was **not** red-first — it
was written after the field existed and passed on its first run, which is exactly the shape that
should be distrusted. Mutation then showed why: **removing the `#[serde(default)]` this section
prescribed changed no test result**, because serde's missing-field path already returns `None` for
any `Option`. The attribute was decoration that read as a requirement, so it was deleted from
`error.rs` and the test was re-cut into
`validation_message_layer_is_optional_and_spelled_layer`, whose killable half asserts the literal
`"layer"` wire key (a field rename moves writer and reader together and leaves the round-trip
green). Mutants, all four KILLED:

| Mutant | Kills |
|---|---|
| `#[serde(rename = "layer_name")]` on `layer` | `validation_message_layer_is_optional_and_spelled_layer` |
| drop `.with_layer(...)` from the `parallax_depth` rule | `u7_each_validation_rule_fires` |
| drop `.with_layer(...)` from `base_opaque` | bridge `invalid_stack_returns_stack_invalid_with_messages` |
| `journal_line` renders `layer` as a constant `-` | `u7`, `u15_front_plane_ceiling`, bridge test |

The non-kill is the useful record: a test that passed on arrival and could not fail was not
coverage, and only the mutation showed it.

**As executed (2026-10-19), the two windowed suites landed.** `test_parallax_pixels.gd` (G5, G7, G12,
S1) and `test_parallax_stretch.gd` (G19, 5 checks) both exit 0 under `DISPLAY=:0.0` — the suites set
`root.size = 1920x1080` themselves, because `editor/project.godot`'s viewport size is inert (it sits
under `[rendering]`, not `[display]`, so the window opens 1152x648). Their mutation results, including
three mutations that are **unkillable as scoped** and why, are in 4.3.2 — the plan's G5 pan form, its
G7 fill derivation and its G12 "fills-first" claim were all corrected there by measurement.

Three row-by-row notes on this section's own predictions, kept because they are the falsifiable part
of the plan:

- **G7's mutation as written here does not exist.** The row implies the overhang colour *is* the fill;
  measured, it is ground-coloured art, so skipping the fill leaves G7 green and it is G12's first
  check that kills the fill mutation. G7 now pins coverage rather than mechanism.
- **G12's second check cannot kill a fills-first pass**, measured bit-identical (see 4.3.2).
- **S1 asserts a full-frame coverage invariant**, not a golden PNG. Golden frames were deliberately
  NOT committed: the render is GPU-produced and not bit-reproducible across machines, and this model
  cannot view an image to approve one, so a stored PNG would be a baseline nobody can re-baseline
  honestly. The invariant (0 clear-colour pixels in the anchored frame) is re-derivable anywhere.

Two harness defects surfaced while proving the table above and are written up in
`$HOME/Documents/opencode/test-falsifiability-and-mutation-verification.md` §8: a build-failure
detector loose enough to match `test error::tests::...` (reported every mutant as void), and a
`cp`/`mv` restore that leaves the restored source **older** than the mutant cargo just built, so
cargo ships the stale mutant and reports a red for code that is not in the tree. The harness
script now anchors its detector and `touch`es after every restore; both runs above postdate
those fixes.

---

## 7. Corrections owed to the wiki (land with this plan, not separately)

Both found while reading #107 against the design of record; both pre-existing; neither caused by the
MCP.

1. **G7's expected colour is stale.** `TDD_Parallax-Scrolling.Verification.md:127` says `#C4CCD2`.
   `stack.json` emits `"fill_below": "#C2CAD0FF"` (measured 2026-10-18): 196,204,210 vs 194,202,208,
   and the stack value carries alpha. A gate written from the wiki fails on real data.
2. **S1's mechanism is unavailable.** `Verification.md:129` names gRPC `capture_screenshot`. Correct
   it to the scripted texture read as the gate, with the MCP screenshot as the human-facing check and
   the reason stated (a PNG cannot be inspected by an image-less model).
3. **The wiki step-4 row omits G19** while #107 includes it — add it. **Already discharged when
   checked 2026-10-19:** `TDD_Parallax-Scrolling.Verification.md`'s pixel table carries a full G19 row
   with its mutation column, so nothing was owed here; the check itself is the record. Corrections 1
   and 2 landed on the wiki's `docs/parallax-step4-corrections` branch on 2026-10-19, together with
   the measured corrections to G5/G7/G12 and the findings that no row can hold (the zoom-1 `152` px
   residual's data cause, `sun.png`'s `period_px` 130, the whole-period-DELTA rule for pans, and the
   drawer's fills riding the previous layer's transform).

**Separate — follow-up ticket #115, not part of #107.** The Home.md indexing self-check in
`.opencode/AGENTS.md` false-positives on sub-pages of an already-linked parent: it flags all four
`TDD_Parallax-Scrolling.*` pages, which are reachable through their parent. Correcting the loop is
filed as its own ticket so the finding is tracked rather than only recorded.

---

## 8. Risks and open items

| Risk | Handling |
|---|---|
| G5's `tile = true` transform space is unknown | Spike first; per-copy fallback specified in 4.3.1 with a bounded 30 draw calls |
| Letterbox scale filter at non-integer scale (open, @me) | Math section 7 assumes `LINEAR` until @me decides; `stretch/scale_mode = integer` is the alternative. Does not affect the function |
| `ValidationMessage.layer` is a shared type | Additive `Option` (no `#[serde(default)]` — mutation proved serde already yields `None` for a missing `Option`) plus a test pinning the literal `layer` wire key; no persisted payload changes shape |
| Editor `project.godot` drifts on every editor run | `git checkout -- editor/project.godot` after each MCP/editor call; MCP enable lines are committed |
| Six suites must pass after any editor change | Run all six; gate on exit codes |
| No CI, no `rustfmt.toml`/`clippy.toml` | Lint/format are whatever is invoked by hand; nothing gates a push |

**Fallback (from the ticket):** if the Godot draw path cannot produce correct output — G5 still red
under the per-copy fallback, or editor zoom filtering unacceptable — this step's game client moves to
Bevy (O3a) or Fyrox (O5a). Steps 1-3 and 5-7 are unchanged; the editor stays Godot.

---

## 9. Completion record

_Filled in as work ships; one row per landed step._

| Item | State |
|---|---|
| Plan rewritten to full #107 scope, both drawer views kept | 2026-10-18 |
| Three scope clarifications posted to #107 so the plan traces to the ticket | 2026-10-18, comment on #107 |
| **plan-ticket-gate >= 90** | **pass — 95/100**, iteration 4 (history below) |
| GDScript suites: `test_parallax_backdrop` (30), `test_world_view` (24), `test_camera_controller` (18), `test_parallax_pixels` (G5/G7/G12/S1), `test_parallax_stretch` (5) | **all exit 0**; pixel + stretch need `DISPLAY=:0.0` (headless `RendererDummy` has no framebuffer) |
| Implementation of #107 | **done and pushed 2026-10-19** — windowed suites + mutation battery landed, docs pass done, both repos pushed (`trunk` = `af0684b`, wiki `master` = `ff55a33`); gate re-run at `af0684b`: 138 Rust tests, seven GDScript suites, all exit 0 |

### Plan gate record (2026-10-18)

Scored by an independent evaluator that saw only the ticket and this plan — not the reasoning that
produced it.

| Run | Score | Outcome |
|---|---|---|
| 1 | 59/100 | Hard cap. The plan leaned on three scope clarifications that existed only in conversation; the evaluator could not see them. |
| 2 | 91/100 | Pass, after those clarifications were posted to #107 as a comment. |
| 3 | 59/100 | Hard cap, self-inflicted: this run added the `sstd-core` version bump, which discharges no clause of #107. |
| 4 | 95/100 | Pass, and the state of this document. |

Run 3 is the instructive one: the plan had already passed, and a confirming pass on a document I had
just edited found a real defect that a document I had not edited would not have had.

**Cut or reframed during the loop, each with its reason:**

- **`sstd-core` 0.1.0 -> 0.2.0 version bump** — removed from planned work. #107 never mentions crate
  versions; this repository's signature-change policy does, and a repository policy is not a ticket
  clause. Now a possible follow-up ticket (4.8).
- **Sprite2D-per-placement switch ladder** — "when to switch to 2.2" inserted a third tiling path
  ahead of the ticket's Bevy/Fyrox fallback. Deleted. The alternative view stays as documentation
  (2.2); adopting it needs its own ticket.
- **`Home.md` indexing self-check** — moved out of the corrections #107 owes and framed as its own
  follow-up ticket (7).

**Lesson for the next plan gate.** The hard cap fires on anything the *ticket* does not support, and
a directive that lives only in conversation is invisible to the evaluator. Posting the directive as
a ticket comment turns it into traceable scope — that is the Documentation-First rule doing its job,
not a way to game the score. The converse is also true: the version bump sat in the plan for one
round precisely because it was true and reasonable, and being true and reasonable is not the test.
