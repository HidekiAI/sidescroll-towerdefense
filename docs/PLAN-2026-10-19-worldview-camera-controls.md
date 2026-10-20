# PLAN 2026-10-19 — Simulator camera controls (pan, zoom, jump)

**Ticket:** issue #119 — `feat(editor): Simulator camera pan, zoom and jump`
**Design of record:** wiki `TechnicalDesign/TDD_Parallax-Scrolling.md` §4 (Camera rules) and the
new §4.1 (editor camera input), `TDD_Parallax-Scrolling.Math.md` §5 (bounds, clamp), §6.1
(anchored zoom), §6.2 (pan step), `TDD_Parallax-Scrolling.Verification.md` G8, G14, G18, G20.
**Status:** design complete, no code yet.

---

## 1. Objective

Give the editor's Simulator tab the camera the design already specifies: free pan, a bounded
view, the zoom ladder, and a start/jump rule. Nothing about the game camera changes.

Three rows of the design table, one ticket:

| Row | Requirement |
|---|---|
| Follow | none (no target); free pan by middle drag and arrow keys |
| Bounds | the map rectangle grown by one screen on every side |
| Zoom | ladder `2 / 1 / 0.5 / 0.25`, Ctrl+wheel and `+` / `-`, anchored so the world point under the cursor does not move |
| Start / Jump | the screen chosen in the screen spinner or minimap; its centre becomes `cam_center` |

## 1.1 Acceptance criteria → plan item → gate

Each acceptance criterion in the ticket, the plan item that satisfies it, and the gate that
proves it. A criterion with no gate here is a criterion this plan does not actually build.

| A/C | Ticket wording (abridged) | Plan item | Gate |
|---|---|---|---|
| 1 | middle-drag, arrow keys, Ctrl+wheel and `+` / `-` move and zoom; nothing responds while another tab is showing | §4.3 input map, §4.4 pan, §4.6 zoom, `_input` gates (`Engine.is_editor_hint`, `is_visible_in_tree`, rect containment) | G18 (drag, arrows, visibility gate), G14 (both zoom inputs) |
| 2 | the view cannot be panned past the map plus one screen, either axis, at any rung | §4.5 bound derivation and clamp | G18, asserted at all four rungs |
| 3 | zoom keeps the world point under the cursor fixed, and only lands on `2 / 1 / 0.5 / 0.25` | §4.6 anchored move and ladder | G8 (anchored), G14 (ladder) |
| 4 | entering the Simulator tab centres on the active screen; picking another screen and returning re-centres | §4.7 one activation rule, including which editor owns the "active" id | G20, with a Placement-tab pick case |
| 5 | `cargo test -p sstd-core` and the five existing GDScript suites stay green, exit-code gated; the two new gates pass windowed and each is mutation-verified | §9 sequence, §9 baseline reconciliation for #85, §6 mutation battery | every command in §9 exits 0, except `test_screen_store.gd` which must exit exactly what it exited on the pre-branch baseline (#85); each new gate red under its listed mutation |
| 6 | the wiki design pages are updated to match what shipped (input contract, Start/Jump activation rule, gates) in the same pass | §10 | after the code lands: a read-through of the five named wiki pages against the shipped code, one page at a time, before the code branch is pushed. The pages that already exist describe the *design*; the post-code pass checks they still describe the *implementation* |
| Rule (ticket, "Rounding") | `cam_center` is rounded to whole world px so tiles and backdrops share one pixel grid | §4.5 (`roundf` before `move_camera_to`) | G18: pan by a fraction (arrow step at zoom 0.25 is 960 px, at zoom 2 it is 120 px; a drag of 1 px at zoom 0.5 gives 2 px, but `view_px / (8 * 0.25)` from an odd centre does not land on an integer) and assert `cam_center == cam_center.round()`. Mutation: drop `roundf` |



## 2. What exists, measured 2026-10-19

| Fact | Evidence |
|---|---|
| The Simulator has no camera input of any kind | no `_input` / `_gui_input` / `_unhandled_input` in `main.gd`, `world_view.gd`, `game_view.gd` |
| Nothing ever writes the camera zoom | `world_view.gd:113` only reads `_camera.zoom.x` |
| The pan entry point exists and is already wired to the whole backdrop | `WorldView.move_camera_to()` (`world_view.gd:84`) sets `_camera.position`; `_poll_view_state()` (`:107-122`) re-queries the bridge whenever `cam_center`, `zoom`, `view_w_px`, `view_h_px` or the anchor changes |
| Its only callers are tests | 8 call sites in `test_parallax_pixels.gd` (146, 149, 159, 166, 181, 205, 217, 233) and 1 in `test_world_view.gd` (206) |
| The scrollbars that used to scrub the view were deleted with #107 | `main.tscn:813-820`, whose comment defers the replacement to "its own ticket" — issue #119 is that ticket |
| The editor bound is not computed anywhere in GDScript | `camera_bound` / `map_rectangle` / `clamp_center` exist in `crates/sstd-core/src/parallax.rs` but are **not** `#[func]`-exposed |
| `CameraController` cannot be reused *as a node* (its `_clamp_center` function can) | it follows a target and refuses `zoom != 1` by design (G6), while the editor needs a ladder; §3 and §4.5 keep its clamp and drop its behaviour |
| The tab node is a plain `Control` wrapper | `main.gd:58` `simulator = tabs.get_child(4)`; the `WorldView` is its child, so `set_screen_store` / `set_grid_config` are never called on it today |

## 3. Reconciliation with the modules this touches
- **`WorldView`'s header contract** (`world_view.gd:1-17`) says its job is to *own* the viewport,
  the camera and the redraw policy. Camera input therefore belongs in `WorldView`, not in
  `main.gd`: the module already treats camera state as its own and the poll loop already reacts
  to exactly the two properties this plan writes.
- **Polling, not signals** (`world_view.gd:6-10`). This plan adds no signal. A pan writes
  `_camera.position`; the existing poll sees it on the next frame. Deliberately one frame of
  latency, which is what the module already accepts for the backdrop.
- **G3** (`world_view.gd:16-17`): the camera lives inside the `SubViewport`, so moving it cannot
  move editor UI. G18 re-asserts G3 after pan and zoom so a future relocation is caught.
- **`CameraController` keeps its refusals.** `zoom != 1` is still refused there (G6) and still
  enforced in `attach()`. The editor does not attach it; it writes `Camera2D.zoom` directly, and
  the comment says so.
- **`move_camera_to` keeps its raw semantics.** Existing tests (G3's `4000, -1200` move, the
  19 200 px pan of G5) call it with out-of-bound positions on purpose. The clamp is added as a
  *new* private setter, never inside `move_camera_to`.
- **No new bridge surface.** `camera_bound` is a 10-line function over data GDScript already
  holds (`ScreenStore.bounds()` plus the grid config). A `#[func]` would need JSON in and out for
  a value that changes only when the screen set changes, and it would not be cheaper than the
  derivation. The precedent is `CameraController._clamp_center`'s own comment (`:82-85`), which
  records the same decision for the game camera.

### 3.1 Provenance of every line this plan changes

Gathered with `git blame` / `git log` and `gh pr list --state all` in this repo, 2026-10-19.

| Line(s) this plan edits | Introduced by | Author / date | PR? | What the thread decided |
|---|---|---|---|---|
| `world_view.gd:84-90` (`move_camera_to`), `:107-134` (`_poll_view_state`, `_resolve_anchor`) | `37e87345` "feat(parallax): add WorldView, its loader, CameraController and the game view scene (#107)" | Hideki A. Ikeda, 2026-10-18 | none — #107 landed by ff-merging a branch, so there is no review thread to read | commit message and the file header (`:1-17`) are the only record: polling over signals, one-frame latency accepted |
| `camera_controller.gd:92-96` (`_clamp_center`) | same commit `37e87345` | same author / date | none | its own comment (`:82-91`) records why the clamp is transcribed from Rust instead of bridged, and that the small-bound case is the one written wrong |
| `main.tscn:813-820` (the `#107` scene comment) | `547d500c` "refactor(parallax): Simulator tab becomes a WorldView instance (#107)" | 2026-10-18 | none | says the scrollbars went with the `Parallax2D` bands and defers a WorldView scrub to "its own ticket" — issue #119 |
| `main.tscn:804-812` (the earlier `layout_mode` note, left untouched) | `3bbd60f3` (lines 804-809, 811, 2026-10-02), `646641e9` (line 810 `[node name="Simulator"]`, 2026-07-31), `d2253636` (line 812, 2026-09-20) | 2026-07-31 to 2026-10-02 | none | why the old backdrop framing and the scrollbar sizing were abandoned |
| `main.gd:252-253` `_on_tab_changed`, `:204-217` `_wire_editors` | `829e8fe6` (2026-07-21) and `1466a7af` (2026-08-01) | 2026-08-01 | none — predates this repo's PR use | the `set_bridge` / `set_grid_config` / `set_screen_store` distribution pattern this plan reuses, and the one-way Map -> Placement `_screen_id` copy §4.7 works around |

Related merged PRs that touched the same backdrop/camera area, read for what they decided:

- **#95** (`feat/fix-89-merge`, merged 2026-10-04) — removed the `Camera2D` from the old backdrop
  preview and drove `Parallax2D.scroll_offset` per layer. 0 comments, 0 reviews. This is the
  change that left the Simulator with no camera at all, and the reason #107 then deleted the
  `Parallax2D` wiring outright.
- **#97** (`feat/fix-96-tab-switching`, merged 2026-10-04) — defer framing, avoid `await` while
  the tab is hidden. 0 comments, 0 reviews. Its lesson (a hidden tab measures 0x0) is why G20 is
  asserted after the tab is selected, never from a hidden node.
- **#98** (`fix/backdrop-framing-after-camera-removal`, merged 2026-10-05) — re-derived
  `Backdrops.position.y = -291` against #89's criterion. One PR conversation comment, no formal
  reviews; that comment filed #99: the value is correct for the sky and still clips the ground
  bands, so the framing needed a maximised window. That whole approach was replaced by #107's
  `WorldView`.

Sources searched and found empty: **no PR exists for #107 or any #119-adjacent line** (checked
`gh pr list --state all`); **no Jira/Confluence** (this project tracks work in GitHub issues
only); no other wiki page states camera-input rules — `TDD_Parallax-Scrolling` §4, `.Math` §5/§6,
`.Verification` and `TDD_World-Editor` § Editor 5 are the whole set, all four read.

## 4. Technical design

### 4.1 Ownership map

```
main.tscn
  TabContainer/Simulator (Control, no script)          <- main.gd distributes config to
    WorldView (Control, @tool, world_view.gd)             editors that HAVE the setters,
      SubViewportContainer (stretch = true)               so this plan reaches the child
        Viewport (SubViewport)                            directly in _wire_editors
          BackPlane/ParallaxDraw
          FrontPlane/ParallaxDraw
          Camera2D                    <- position, zoom   (written here only)

  TabContainer/Map         (map_editor.gd)      screen_spin, minimap  -> _screen_id
  TabContainer/Placement   (placement_editor.gd) screen_spin, minimap  -> _screen_id

main.gd: _on_tab_changed(4) -> simulator_world_view.center_on_screen(map_editor._screen_id)
WorldView:  _input(event)  -> pan / zoom -> _apply_cam_center_clamped()
            _process       -> _poll_view_state() (unchanged) -> bridge -> ParallaxDraw
```

### 4.2 New state (all private to `WorldView`)

| Name | Type | Why |
|---|---|---|
| `_screen_store` | `ScreenStore` | source of the occupied screen cells |
| `_grid_config` | `Dictionary` | `max_tiles_per_screen_x/y`, `tile_width/height_in_pixels` |
| `_dragging_middle` | `bool` | middle-button state across motion events |

No new type is introduced. The bound is a `Rect2` computed on demand; a caching struct would
need invalidation on add / move / clone / delete of a screen, which is exactly the bug this
design avoids by deriving it.

### 4.3 Input entry point

`_input(event: InputEvent)`, subject to one guard hierarchy:

```gdscript
func _input(event: InputEvent) -> void:
    # @tool: in the Godot editor the same scene is live, and a middle drag over the Simulator
    # tab would write Camera2D.position into the edited scene (dirty) while Godot's own 2D
    # viewport uses that button to pan. The app run is the only place camera input belongs.
    if Engine.is_editor_hint():
        return
    if not is_visible_in_tree():
        return                       # another tab is showing
    ...
```

Why `_input` and not `_unhandled_input`: `SubViewportContainer` has `stretch = true` and
`mouse_filter` defaults to `STOP`, so it consumes GUI mouse events; unhandled input would never
see a middle press over the view.

Pointer events are additionally gated on `get_global_rect().has_point(event.position)`; keys are
gated on "no visible text-entry Control has focus":

```gdscript
func _text_entry_has_focus() -> bool:
    var focus_owner: Control = get_viewport().gui_get_focus_owner()
    if focus_owner == null or not focus_owner.is_visible_in_tree():
        return false                 # a SpinBox on a hidden tab must not eat arrow keys
    return focus_owner is LineEdit or focus_owner is SpinBox or focus_owner is TextEdit
```

Event table:

| Event | Handling |
|---|---|
| `InputEventMouseButton`, `MOUSE_BUTTON_MIDDLE`, `pressed` / `!pressed` | set / clear `_dragging_middle` |
| `InputEventMouseMotion` with `_dragging_middle` | `_pan_by_world_px(-event.relative / zoom)` |
| `InputEventKey`, `is_action_pressed(action, true)` for `ui_left/right/up/down` | `_pan_by_world_px(step)` where `step = view_px / (ARROW_PAN_DIVISOR * zoom)`. The second argument is `allow_echo = true`; the Godot 4 default is `false`, which would drop every held-key repeat and make G18's "press and echo" half untestable |
| `InputEventMouseButton`, wheel up/down, `is_command_or_control_pressed()` | `_step_zoom_rung(-1 / +1, event.position - get_global_rect().position)` |
| `InputEventKey` `KEY_EQUAL` / `KEY_PLUS` / `KEY_MINUS` | `_step_zoom_rung(±1, _viewport.size * 0.5)` |
| wheel without Ctrl | untouched — the Map tab's brush radius (G14) |

`event.position` is in root-viewport coordinates; `_step_zoom_rung` takes a position measured
from the WorldView's top-left (the `SubViewport`'s top-left, since the container stretches to
fill it), so the subtraction happens at the call site and the parameter is named for its space.

Nothing calls `set_input_as_handled()`. Neither the ticket's input table nor `TDD_Parallax-Scrolling`
§4.1 asks for it, and no other editor control claims the middle button, Ctrl+wheel or the ladder
keys, so consuming them would only risk changing wheel behaviour elsewhere.

### 4.4 Pan

```gdscript
const ARROW_PAN_DIVISOR: int = 8      # one arrow step = 1/8 of the visible world

# Screen-px delta in, world-px move out. Dividing by zoom is what makes the world follow the
# pointer at every rung; the negation is the grab-the-world convention (drag right -> world
# moves right -> the camera moves left).
func _pan_by_world_px(world_px_delta: Vector2) -> void:
    _apply_cam_center_clamped(_camera.position + world_px_delta)
```

Arrow step, per press and per echo: `view_px / (ARROW_PAN_DIVISOR * zoom)` world px on the
pressed axis. At zoom 1 in a 1920-wide panel that is 240 px; at 0.25 it is 960 px, i.e. the same
fraction of the visible world at every rung (Math §6.2).

### 4.5 Bound derivation and clamp

```gdscript
# The screen size in world px, from the grid config. One copy: the bound, the centre-on-screen
# move and (later) the Map Editor's culling all need it, and the config is the source of truth
# (Rule of Thumb: Config-Based Constants) so no literal fallback is written here.
func _screen_size_world_px() -> Vector2:
    return Vector2(
        float(_grid_config.get("max_tiles_per_screen_x", 0)) * float(_grid_config.get("tile_width_in_pixels", 0)),
        float(_grid_config.get("max_tiles_per_screen_y", 0)) * float(_grid_config.get("tile_height_in_pixels", 0)))

# The map rectangle grown by one screen on every side (Math section 5, `camera_bound(editor =
# true)`). Derived per call: ScreenStore.bounds() is a pass over the screens map, which the
# editor holds at single-digit size, and deriving it removes every invalidation path.
func _editor_bound_world_px() -> Rect2:
    if _screen_store == null or _grid_config.is_empty():
        return Rect2()                      # no world yet: nothing to clamp against
    var screen_size_px: Vector2 = _screen_size_world_px()
    if screen_size_px.x <= 0.0 or screen_size_px.y <= 0.0:
        return Rect2()                      # grid config not loaded yet: same case
    var cells: Rect2i = _screen_store.bounds()
    var map_world_px := Rect2(
        Vector2(cells.position) * screen_size_px,
        Vector2(cells.size) * screen_size_px)
    return map_world_px.grow_individual(
        screen_size_px.x, screen_size_px.y, screen_size_px.x, screen_size_px.y)
```

`Rect2.grow_individual(left, top, right, bottom)` grows by one screen on each side, matching
`camera_bound`'s `left - SCREEN_W_PX … right + SCREEN_W_PX`.

`ScreenStore.bounds()` seeds its min/max at `(0, 0)`, so `(0, 0)` is always inside `cells`.
That is the screen grid's own convention (a world is authored from the origin) and is recorded
in the wiki rather than "fixed" here.

Clamp, with the view extent converted to world px:

```gdscript
func _apply_cam_center_clamped(world_px_center: Vector2) -> void:
    var bound: Rect2 = _editor_bound_world_px()
    var zoom: float = _camera.zoom.x if _camera != null else 1.0
    var view_world_px: Vector2 = _viewport.size / zoom     # Camera2D zoom = magnification
    var target: Vector2 = world_px_center
    if bound.size.x > 0.0 and bound.size.y > 0.0:
        target = Vector2(
            CameraController.clamp_center(target.x, view_world_px.x, bound.position.x, bound.end.x),
            CameraController.clamp_center(target.y, view_world_px.y, bound.position.y, bound.end.y))
    # Whole world px, so tiles, backdrops and the parallax anchor share one pixel grid.
    move_camera_to(roundf(target.x), roundf(target.y))
```

`CameraController._clamp_center` is promoted to `static func clamp_center(...)`; its body and
its small-bound case (`bound_low + half view`, **not** the midpoint) are unchanged, and its one
internal call site becomes `clamp_center(...)`. One copy, two users.

### 4.6 Zoom ladder and the anchored move

```gdscript
const ZOOM_LADDER: Array[float] = [2.0, 1.0, 0.5, 0.25]

# cursor_from_view_topleft_px is measured from the WorldView's top-left (the SubViewport's
# top-left too, since the container stretches to fill it); the caller does the subtraction.
func _step_zoom_rung(direction: int, cursor_from_view_topleft_px: Vector2) -> void:
    var zoom_old: float = _camera.zoom.x
    # Nothing else in the project writes Camera2D.zoom (measured: only world_view.gd:113 reads
    # it), so the current value is always a rung and find() answers.
    var rung_index: int = ZOOM_LADDER.find(zoom_old)
    var next_index: int = clampi(rung_index + direction, 0, ZOOM_LADDER.size() - 1)
    var zoom_new: float = ZOOM_LADDER[next_index]
    if is_equal_approx(zoom_new, zoom_old):
        return                                   # already at an end of the ladder
    # Math section 6.1: keep the world point under the cursor fixed.
    var cursor_from_center_px: Vector2 = cursor_from_view_topleft_px - _viewport.size * 0.5
    var zoom_shift: float = 1.0 / zoom_old - 1.0 / zoom_new
    _camera.zoom = Vector2(zoom_new, zoom_new)
    _apply_cam_center_clamped(_camera.position + cursor_from_center_px * zoom_shift)
```

Order matters: the new zoom must be in place before the clamp runs, because the clamp's view
extent is `view_px / zoom`. Both writes land in one frame, so `_poll_view_state` sees a single
change (one bridge query, one redraw).

### 4.7 Start and Jump — one rule, one hook

The camera is only observable while the Simulator tab is showing, and a spinner or minimap pick
cannot be made while it is showing. Both rows therefore collapse into: *entering the Simulator
tab centres the camera on the active screen*.

`main.gd` reaches the `WorldView` once (the tab node is a scriptless `Control`, so the
`has_method` distribution loop never touched it):

```gdscript
# in _wire_editors(), after the existing distribution loop
simulator_world_view = simulator.get_node_or_null("WorldView") as WorldView
if simulator_world_view != null:
    simulator_world_view.set_screen_store(screen_store)
    simulator_world_view.set_grid_config(_grid_config)
```

```gdscript
# in _on_tab_changed(), beside the existing tab 2 / tab 3 branches
func _on_tab_changed(tab: int) -> void:
    var previous_tab: int = current_tab
    current_tab = tab
    ...
    if tab == 4 and simulator_world_view != null:
        # The two editors keep separate screen ids, and main.gd copies Map -> Placement on
        # entering the Placement tab, never back. Read the id from the tab the user left, so a
        # pick made inside Placement reaches the camera instead of the stale Map id (G20).
        var active_screen_id: int = map_editor._screen_id
        if previous_tab == 3:
            active_screen_id = placement_editor._screen_id
        simulator_world_view.center_on_screen(active_screen_id)
```

Read, not copy: writing Placement's id back into `map_editor` would change what the Map tab
shows, which is a behaviour neither the ticket nor the design asks for. The asymmetry is
pre-existing (`main.gd:270` copies one way only) and is recorded here rather than "fixed".

```gdscript
func center_on_screen(screen_id: int) -> void:
    if _screen_store == null:
        return
    var cell: Vector2i = _screen_store.position_of_id(screen_id)
    if cell.x < 0:
        return                                   # unregistered screen, (-1, -1) sentinel
    var screen_size_px: Vector2 = _screen_size_world_px()
    if screen_size_px.x <= 0.0 or screen_size_px.y <= 0.0:
        return
    _apply_cam_center_clamped((Vector2(cell) + Vector2(0.5, 0.5)) * screen_size_px)
```

No signal is added to `map_editor` / `placement_editor`: their picks end in
`set_value_no_signal` (6 call sites in `map_editor.gd` + `placement_editor.gd`, 7 with
`layer_opacity`), so `value_changed` alone would miss them, and emitting from each editor would
mean touching every one of those sites for a value `main.gd` already reads.

Remembering a pan across tab visits is explicitly **not** built: the tab is not a persistent
viewport and the Start row defines its subject as the screen chosen in the spinner or minimap.
If @me later wants "keep my pan", that is its own decision and its own row.

### 4.8 Complexity and frame cost

| Operation | Cost |
|---|---|
| pan per motion event | one `ScreenStore.bounds()` pass (O(screens), single digits) + two `clampf` |
| zoom per wheel tick | one ladder find (4 entries) + the same clamp |
| poll (unchanged) | one dictionary compare per frame; one bridge query only when the view changed |
| tab activation | one `position_of_id` pass (O(screens)) |

Nothing is per-frame except the existing poll.

## 5. Exact edits

| File | Change |
|---|---|
| `editor/scripts/camera_controller.gd` | `func _clamp_center` → `static func clamp_center`; update the one internal call and the comment that names it |
| `editor/scripts/world_view.gd` | new constants `ZOOM_LADDER`, `ARROW_PAN_DIVISOR`; new members `_screen_store`, `_grid_config`, `_dragging_middle`; `set_screen_store`, `set_grid_config`, `center_on_screen`, `_input`, `_pan_by_world_px`, `_step_zoom_rung`, `_apply_cam_center_clamped`, `_screen_size_world_px`, `_editor_bound_world_px`, `_text_entry_has_focus`; header comment gains the input contract |
| `editor/scripts/main.gd` | `var simulator_world_view: WorldView`; wire it in `_wire_editors`; tab 4 branch in `_on_tab_changed` |
| `editor/scenes/main.tscn` | comment at `:813-820`: the deferred "its own ticket" is #119, now covered by §4.1 |
| `editor/tests/test_world_view_controls.gd` | new windowed suite: G8, G14, G18, G20 |
| `docs/PLAN-2026-10-19-worldview-camera-controls.md` | this file, updated live as the code lands |

## 6. Gates

New file `editor/tests/test_world_view_controls.gd`, `extends SceneTree`, the #107 harness shape
(`GUARDS` completion tracking, `quit(0 if _failures == 0 else 1)`), run windowed because a
`Camera2D` needs a real viewport:

```bash
DISPLAY=:0.0 $HOME/bin/godot4 --path editor --script res://tests/test_world_view_controls.gd
echo "exit=$?"
```

| Gate | Assertion | Mutations (each must turn it red) |
|---|---|---|
| G18 pan and clamp | middle drag `(dx, dy)` at zoom 1 moves `cam_center` by exactly `(-dx, -dy)`; at zoom 0.5 by `(-dx/0.5, -dy/0.5)`; one arrow press and one echo move `view_px / (8 * zoom)`; a drag far past the bound stops at `bound ± view/(2*zoom)` on that axis **and does so at all four rungs** (2, 1, 0.5, 0.25 — the view extent doubles as the zoom halves, so one rung cannot stand in for another); every editor `Control` keeps its global rect; `is_processing_input()` is true; and every centre it lands on is whole world px (the ticket's rounding rule — one arrow step at zoom 0.25 from an even centre would otherwise land on a half px) | (a) pan `+` instead of `-`; (b) pan `event.relative` without `/ zoom`; (c) drop the one-screen margin from the bound; (d) drop the half-view inset; (e) `set_process_input(false)`; (f) remove the `is_visible_in_tree()` gate and assert a hidden view does not pan; (g) drop the `roundf` |
| G20 start and jump | entering the tab puts `cam_center` at the active screen's centre for `(0,0)` and `(1,0)`; picking another screen and re-entering re-centres; a pick made inside the **Placement** tab also re-centres (set `current_tab = 3`, set `placement_editor._screen_id` to the second screen, call `_on_tab_changed(4)`) | (a) always centre on `(0,0)`; (b) centre on the screen's top-left corner; (c) skip the clamp so an edge screen is centred off-bound — caught by asserting the clamped value for a far screen; (d) read `map_editor._screen_id` unconditionally, which the Placement case turns red |
| G8 (wiki, existing, previously unbuildable) | cursor `(1500, 300)`, view `1920x1080`, `cam (960,528)`, zoom `1 -> 0.5`: the world point under the cursor is unchanged within 0.5 px; expected `cam (420, 768)` per Math §6.1 | zoom about the view centre |
| G14 (wiki, existing, previously unbuildable) | the ladder only ever lands on `2 / 1 / 0.5 / 0.25`; the ends stop; plain wheel leaves zoom at 1 | allow `0.75`; let plain wheel zoom |

G14's other half — "plain wheel changes the Map tab's brush radius" — is deliberately **not**
asserted here: that behaviour lives in `map_editor.gd` and already has `test_terrain_brush.gd`.
What this suite owns is the negative: the Simulator hands a plain wheel on untouched, which is
what keeps the two from fighting when the tab changes. Recorded so the gap is a decision, not an
oversight.

Two behaviours in §4.3/§4.5 cannot be exercised by a windowed app run, and each is covered
differently:

- **The no-world skip is gated.** A `WorldView` with no store set is exactly the state the
  suite starts in, so G18's first action is a pan on an unconfigured view: it must move and must
  not be clamped to `(0,0)`. Mutation: make `_editor_bound_world_px()` return a one-screen rect
  instead of an empty one and the pan stops dead.
- **`Engine.is_editor_hint()` is a manual check, not a gate.** In an app run the hint is always
  false, so any assertion would be vacuous (the same trap as a probe that assigns the value it
  asserts on). The check is: open `main.tscn` in the Godot editor, middle-drag over the Simulator
  tab, confirm the camera does not move and the scene does not become dirty. Written down because
  an unexercised guard is otherwise indistinguishable from a wrong one.

The `CameraController` promotion is proven by the existing suite, not by a new gate:
`test_camera_controller.gd` already asserts the small-bound case (`bound_low + half view`, not
the midpoint). Its mutation is the obvious one — return the midpoint — and it must go red *after*
the rename, which is what shows the promotion is a rename and not a behaviour change.

Synthetic events are built as real `InputEventMouseButton` / `InputEventMouseMotion` /
`InputEventKey` objects and handed to `world_view._input(event)`. That bypasses the engine's
dispatch but not the node's own gates (editor hint, visibility, rect containment, focus,
`is_command_or_control_pressed`), and `is_processing_input()` pins the one thing a direct call
cannot prove: that the handler is registered at all. Recorded because it is the same class of
trap as "a probe that assigns the value it then asserts on".

Every gate above is mutation-verified before it is reported green: confirm the substitution
fired and the mutant parses, then read the exit code (never the printed counter).

## 7. Risks and open points

1. **`@tool` + editor hint.** Without `Engine.is_editor_hint()`, middle-dragging the Simulator
   tab inside the Godot editor would (a) fight Godot's own pan and (b) mark `main.tscn` dirty by
   writing `Camera2D.position`. Guard added; no other `@tool` script in the repo does input, so
   there is no precedent either way. Recorded on the ticket (issue #119, comment 1, section 1) so
   the behaviour is a requirement rather than a plan-only decision.
2. **Wheel conflicts.** Ctrl+wheel is unclaimed today; plain wheel stays the Map tab's brush
   radius and is untouched. If #108's Map Editor `WorldView` lands, it inherits this handler and
   both visible-gated views pan themselves — which is correct, not a conflict.
3. **Focus on a hidden `SpinBox`.** A `SpinBox` on a hidden tab can still own focus; the guard
   requires `focus_owner.is_visible_in_tree()`, so arrow keys pan the Simulator instead of
   silently changing a hidden spinner.
4. **Off-ladder zoom.** Nothing else writes `Camera2D.zoom` today (measured: `world_view.gd:113`
   is a read only), so the current value is always a rung and `ZOOM_LADDER.find()` answers. The
   `-1` branch exists only so a future writer degrades to the `1.0` rung instead of indexing the
   array with `-1`; it is two lines, not a helper.
5. **No world loaded.** `_editor_bound_world_px()` returns an empty `Rect2` and the clamp is
   skipped; pan and zoom still work, which is what a designer sees before any screen exists.
   Recorded on the ticket (issue #119, comment 1, section 2), which is also why A/C 2's bound
   applies only when a bound exists.
6. **`ScreenStore.bounds()` includes `(0,0)` by construction.** A world entirely at negative
   columns gets a bound grown to the origin. Accepted and documented, matching the screen grid's
   authored-from-origin convention.

## 8. Out of scope

- The game camera (`CameraController`, `GameView`): unchanged, still refuses `zoom != 1` (G6).
- **No scrollbars.** `TDD_Parallax-Scrolling` §4's Follow row used to list "middle drag, scrollbars,
  arrow keys". The scrollbars were deleted with #107 (they only drove the six `Parallax2D` bands)
  and this ticket does not bring them back: the design's own replacement list is the three input
  methods, and the wiki row was corrected to say so in the same pass. If @me wants a scrub
  control after using it, that is a new ticket with its own bound and step rules.
- Zoom RPC (`CameraService`), layer visibility/opacity prefs, per-screen pan memory.
- #108's Map Editor `WorldView`: it will reuse this code when it lands.
- `project.godot` viewport keys (#118) and the transform-reset follow-up (#117).

## 9. Verification sequence

```bash
cd sidescroll-towerdefense
cargo test -p sstd-core                       # exit code, not the printed count
DISPLAY=:0.0 $HOME/bin/godot4 --path editor --script res://tests/test_world_view_controls.gd
DISPLAY=:0.0 $HOME/bin/godot4 --path editor --script res://tests/test_world_view.gd
DISPLAY=:0.0 $HOME/bin/godot4 --path editor --script res://tests/test_parallax_pixels.gd
DISPLAY=:0.0 $HOME/bin/godot4 --path editor --script res://tests/test_parallax_stretch.gd
DISPLAY=:0.0 $HOME/bin/godot4 --path editor --script res://tests/test_camera_controller.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_parallax_bridge.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_parallax_backdrop.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_bridge_autobuild.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_image_to_map.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_terrain_brush.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_override_merge.gd
git checkout -- editor/project.godot           # Godot rewrites it on open
```

Every line is gated on `$?`, never on the printed `failures=` line (the counter is spelled
`failures` in two suites and `_failures` in the others, and #80 was an inverted ternary that
exited 1 on a pass).

**`test_screen_store.gd` is the one exception, and it is pre-existing.** A/C 5 says the suites
"stay green"; that suite already exits 1 on Godot 4.7.2 for issue #85 (`ZIPPacker` gained a
`tiles/` directory entry, so a zip-entry count reads 3 instead of 2) and exits 0 on 4.4.1. "Stay
green" is therefore read as **no new failure against the baseline measured immediately before
this branch** — run the whole set once on `trunk` before the first code edit and diff the exit
codes against it. Re-measuring the baseline is cheaper than arguing about it later.

`test_camera_controller.gd` is in the list because `clamp_center` is being promoted there.
`test_parallax_bridge.gd`, `test_parallax_backdrop.gd` and `test_bridge_autobuild.gd` touch the
viewport/bridge area the pan writes through, so they are cheap insurance rather than proof. A/C 5
names five suites; the extra three are run because they cost seconds, not because this change is
allowed to fail them.

Plus a human check on a real window: pan to each bound edge, both ends of the ladder, and enter
the tab from two different active screens.

## 9.1 Corrections carried from the ticket body

Three things in issue #119's body were wrong or incomplete and are corrected by its first
comment, so the plan follows the comment:

| Body said | Corrected to |
|---|---|
| formulas go in `TDD_Parallax-Scrolling.Math` section 7 | sections **6.1** (anchored zoom) and **6.2** (pan step); section 7 is the drawing/input/resize/headless page |
| G8 and G14 belong to the design's step 5 | step 5 is #108 (Map Editor); G8/G14 test the Simulator camera and are this ticket's. The design's step table gained a step 8 row for #119 and step 5's gate list dropped them |
| "x9, x1" call sites | 8 in `test_parallax_pixels.gd`, 1 in `test_world_view.gd` |

Both extra behaviours (editor-hint guard, no-world skip) are recorded in the same comment,
which is why §7 cites it.

## 10. Documents updated with this plan

- Wiki `TDD_Parallax-Scrolling.md` §4.1 — the input contract (done, same day as the ticket).
- Wiki `TDD_Parallax-Scrolling.md` §6 step table — a step 8 row for #119; step 5 (#108) no
  longer claims G8/G14 (done, same commit).
- Wiki `TDD_Parallax-Scrolling.Math.md` §6.1 / §6.2 — the anchored zoom derivation and the pan
  step (done).
- Wiki `TDD_Parallax-Scrolling.Verification.md` — G18 and G20 rows (done).
- Wiki `TDD_World-Editor.md` § Editor 5 — the status correction (done).
- Wiki `TODO.md` — TS76 (done).
- Issue #119 comment 1 — the editor-hint guard, the no-world behaviour, the Math §6.1/§6.2
  correction, the G8/G14 re-homing and the two measured corrections (done, before coding).
- This repo: `docs/SESSION-CHECKPOINT.md` after each step; this plan updated to what shipped.
