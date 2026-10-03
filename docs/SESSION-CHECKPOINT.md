# Session Checkpoint — Parallax Restack (#68)

## CURRENT STATE / NEXT MOVE

> This block is the single authoritative resume point. It **supersedes** every
> `## Active step`, `## Next move` and `## SUPERSEDED` heading further down, which are
> retained as the historical record only. A cold-start session should read this block
> and stop.

_Last updated: 2026-10-03 (gate iteration 4 applied, same day)_

**Objective:** restack the #68 parallax background on the official
[2D Parallax tutorial](https://docs.godotengine.org/en/stable/tutorials/2d/2d_parallax.html)
— in-repo generated layers, a visible and correctly framed backdrop, provable two-axis
scroll.

**State: #89 is DESIGNED, GATED and MEASURED. No production code written yet. The plan of
record has cleared four gate iterations; the last scored 76/100 with all ten deficiencies
applied. Phase 2 still blocked on #87. Phase 6 pending.**

### The fix, in one line

Delete the `Camera2D`, and write `layer.scroll_offset = scrub * layer.scroll_scale` per
layer, per axis. From `Parallax2D::_update_scroll()`: the engine scales `screen_offset`,
never `scroll_offset`, so a per-layer write is what actually drives each band. No camera
means no canvas transform, so the editor UI is never displaced.

### The plan of record

`docs/PLAN-2026-10-03-parallax-scroll-offset.md`, with per-iteration changelogs in
`docs/PLAN-2026-10-03-parallax-scroll-offset-gate-history.md`. **Read section 10 first** —
it carries the gate verdict and what each iteration changed.

Gate: bar is 90, ticket alone, evaluator never sees the planner's reasoning.

| iter | score | outcome |
|---|---|---|
| 1 | 84 | FAIL, no CRITICAL, 2 uncovered clauses |
| 2 | 91 | **invalid pass** — evaluator was fed the task prompt's D clauses as normative |
| 3 | 88 | FAIL on bar, no CRITICAL, 7 citation deficiencies, all applied |
| 4 | 76 | FAIL on bar, no CRITICAL, **10 deficiencies, all applied** (`2c4e140`) |

### The correction that matters — read before repeating it

Iteration 4 found a **false engine claim** in the plan's central section, and it was
verified against source before being adopted (iteration 3's report had a wrong line
citation, so the gate's word is evidence, not instruction):

```cpp
// servers/rendering/renderer_rd/renderer_canvas_render_rd.cpp:2239-2252
Point2 start_pos = ci->repeat_size * -(ci->repeat_times / 2);
for (int rx = 0; rx <= repeat_times_x; rx++) {   // INCLUSIVE: N+1 copies, not N
```

`repeat_times = 1` draws **two** centred copies, spanning `[px, px+3840)` with
`px = position.x ∈ (-1920, 0)`, so a 1920-wide viewport is *always* covered. Separately
`parallax_2d.cpp` clamps `repeat_times = MAX(p_repeat_times, 1)`, so 0 ≡ 1 ≡ 5.

**Two of the plan's three "falsified" metrics had been declared impossible for the wrong
reason, and both readings were actually correct.** 0.00% bare at scrub 6400 is exactly what
rt=1 must produce; rt=0 matching a 5-copy reference is possible and was measured. The lesson
now recorded in plan 5.1: *a control reporting a startling value is a claim about the
engine, not a verdict on the metric.* No design changed — the clamp, the framing constant
and the per-layer writes are unaffected.

### Measured and confirmed (plan section 5.2)

M-0, M-a, M-b/M-c, M-e, M-f, M-g, M-h, M-k, plus M-e's four-way counter-verification.
Key outputs: `scroll_v.max_value` stays **240.0** (sky top is 3637.5 scrub units away, 15×
the clamp; its justification comment is false and gets rewritten); `scroll_x.max_value` stays
**6400.0**; `Backdrops.position.y = -291` measured at 0.00% bare with all six bands
contained.

`--resolution 1920x1080` yields a **1920x1029** window (the WM grants 1029). Every number
the plan labels "1920x1080" was measured there, `seen_region` 1920x998 at canvas y=31.

### Still open in the plan

- **M-b1** — `Backdrops.position.y = -291` is **not a derived optimum**. The bare-% column
  reproduces exactly from `bare(offset) = max(0, 31+offset) + max(0, -322-offset)`, which also
  predicts 0.00% at `-320, -240, -180, -120, -60` — a six-way tie. Recorded as an open gap,
  not a settled number.
- **M-e's five-point scrub sweep at 1280x800** — the upward bound is resolution-independent
  and confirmed; this is the one open end.
- **M-d** superseded by M-k.

### The bug (#89) — read this before touching the backdrop

`BackdropStrip`'s `Camera2D` sits in the **same canvas layer as the entire editor UI**. A
`Camera2D` transforms its whole canvas layer, so it was displacing every `Control` —
tab bar, all editors, both scrollbars — by half the viewport.

Measured with a real window at 1920x1080 (frame 1920x1029):

```
canvas_transform       = translate(960, 483.5)
ui pixels bbox         = (960, 483) size (960, 546)   <- UI in one quadrant
bar_x renders at y     = 1496.5   (frame ends at 1029 -> off-screen)
bar_y renders at x     = 2864     (frame ends at 1920 -> off-screen)
bar_x contributed px   = 0
bar_y contributed px   = 0
```

**Both scrollbars have been invisible this whole time.** They are anchored correctly
(`bar_x` at `(0,1013) 1904x16`, `bar_y` at `(1904,31) 16x982`) and render off-screen.

### Two candidates measured on rendered frames — both fail

| candidate                        | ui_bbox                      | bar_x px | bar_y px | art scrolls |
|----------------------------------|------------------------------|----------|----------|-------------|
| baseline (camera on, inline)     | (960, 483) size (960, 546)   | 0        | 0        | yes         |
| B: `camera.enabled = false`      | (0, 0) size (1920, 1029)     | 30442    | 15690    | **NO**      |
| A: strip onto its own CanvasLayer | (960, 514) size (960, 515) | 0        | 0        | yes         |

- **B is a diagnostic, not a fix.** `simulator.gd:32-39` drives both bars by writing
  `camera.position`, so a disabled camera leaves the strip immobile.
- **A does not work at all.** Moving the strip — and the camera — onto its own
  `CanvasLayer` should confine the camera to the art's canvas. It does not: the UI stays
  displaced and both bars still draw 0 px.

### The measurement trap that hid this — do not repeat it

`get_global_rect()` reports **anchor math only** and excludes the camera's canvas
transform. The bars therefore read as correctly placed at every window size while being
drawn off-screen entirely. `z_index` cannot detect it either: a control created in code at
the same parent and at `z_index = 20` draws 2968 px, while the declared bar draws 0 px at
**every** z from 0 to 20.

Two checks are required for anything expected to be visible, and neither is a transform
read:

1. intersect the **canvas-space** rect with `get_visible_rect()`;
2. hide the node, diff the rendered frame, and count changed pixels. Include a positive
   control that is known to draw, so a zero result is distinguishable from a broken differ.

**Corollary: every framing number measured before this finding is suspect**, because it was
calibrated through a displaced canvas. Specifically void —

- `Backdrops.position = Vector2(-960, -780)` in `backdrop_preview.tscn`
- `_frame_backdrop()` runtime horizontal alignment in `simulator.gd`
- the 240px vertical travel clamp and its justification
- the conclusion that vertical travel needs a **generator change** for taller art. That was
  measured through the wrong canvas and must be re-tested before acting on it.
- #88's `half_viewport * scroll_scale` stagger formula — a camera-induced half-viewport
  offset and a parallax-induced one present identically, so it must be re-measured.

### Next move

1. **Push, then ff-only merge** `feat/parallax-restack` to `trunk`. Branch-then-merge; ask
   before pushing. Never `--force`. Offer branch deletion after merge; never do it
   unilaterally.
2. **Delete `editor/tests/measure_no_camera.gd`** before the branch merges. It is a 765-line
   throwaway probe with no assertions; it produced the numbers in 5.2 and several of its
   metrics are the voided ones in plan 5.1. Untracked, never `git add`ed. `editor/addons/`
   likewise — 516 MB vendored, never `git add -A`.
3. **Rewrite `editor/tests/test_parallax_backdrop.gd`** (assistant's job, not a block): new
   `scroll_offset` contract; an ordering assertion replacing the negative-drift check (M11 —
   "Foreground moves backwards" was true only of the camera model); a **sign** assertion,
   because the x scrub inverts on screen under B6; keep `240.0` and `6400.0` assertions,
   rewrite the false `:106-107` justification; assert the wrap-free base of 640 (M10).
   **Run RED against the unfixed tree first.**
4. **Write `editor/tests/probe_render_visibility.gd`** — corrected absolute-bare-pixel metric,
   positive control must hide **Sky** specifically (only the bottom-most opaque layer can
   raise a background count). Exit 0 needs **four** conditions, including the five-point
   scrub sweep that 4.5 computed but section 7 previously did not gate. Assert received
   window size against the **WM** screen size, not the requested `--resolution`.
5. **Scaffold blocks 1/6–6/6** with the measured numbers, via the `coding-assistant` skill.
   Order is script-then-scene (`simulator.gd:25` is a hard `get_node`; deleting the node
   first leaves a window where `_ready()` raises "Node not found").
6. Re-test #88's stagger formula under the corrected canvas (already commented to #88).
7. **#87** (editor backdrop repoint + persisted-map migration + dead `_BACKDROP_PATHS`)
   before Phase 2's `git rm` of the 13 old tracked PNGs.
8. Phase 6: final gates, close #68, `--ff-only` merge, offer branch deletion.

### Parallel thread — #94 minimap (GDD merged, zoom ladder settled)

Wiki `master` at `ace6738`. `GDD_World-Layout.md` carries a *Minimap (Side-Scrolling Defense
Zones)* section plus `GameDesign/assets/minimap_schematic.{svg,png}` (a labelled **MOCKUP** —
no minimap exists to screenshot; drawn in screen units so it survives any scale).

Settled: scrolls rather than scales; the map's **full vertical extent** across full map width;
entities as dots (blue ally, red enemy, grey neutral); **M** key and a Minimap button; overlay
with the game live; **exactly three zoom levels, 2x apart**.

**The zoom ladder, settled 2026-10-03.** One screen occupies `1920/d` x `1056/d` panel px, so
`screens_shown = (panel_w * d) / 1920`. Recommended panel **480 x 264** gives a **1 / 2 / 4**
screens-wide ladder at `d = 4 / 8 / 16`. Two consequences, both verified by computation:

- The **scroll-unreachable problem is solved**. A 3-screen map is 720 panel px against a
  480 px window at normal zoom, so the window really scrolls. That was the question the GDD
  carried as open and could not answer; it is deleted from the wiki page, not left dangling.
- **Scaling is uniform on both axes**, and the panel *height* is what enforces it:
  `panel_h` must be a multiple of `1056 / d_zoom_in` = 264. Levels then divide evenly at
  264/132/66 px. Panel height is the one dimension that is not free.

Extent changed from "exactly one `screen_y` row" to the map's full height, because
`GDD_Combat-Mechanics` Blueprint 2 is `4 Screens Wide: 1 High ---> 2 High ---> 1 High`. One
`min(map, window)` rule per axis covers both. The EvE/ZvZ rule from `TDD_World-Editor` is
carried into the section: minimap contents must come from the owning client's own viewport
coverage, never server-global truth.

**Three items still open in the wiki, deliberately:**

- **Panel size** — 480 x 264 is a *recommendation*, not a decision. Widening it changes only
  the window width, never the ladder's ratio or the uniform-scale rule.
- **Zoom input binding** — wheel risks the live game behind the overlay consuming it; keys
  risk colliding with combat hotkeys. Neither chosen.
- **The Alarm tower conflict.** `GDD_Gameplay.Towers` gives Alarm its *entire* effect:
  "Reveals enemy position on minimap". Always-visible red enemy dots nullify it. A
  resolution is proposed and **marked unapproved**: reveal extends from the view rectangle to
  the whole window, so Alarm keeps a real effect. Changing that promise is a change to
  `GDD_Gameplay.Towers`, not to `GDD_World-Layout`.

Also: the viewport is 1080 px tall and a screen is 1056 px, so the view **straddles the next
`screen_y` row by 24 px** — 2.3%, invisible at minimap scale. The straddled row is defined by
the top edge of the viewport, stated as a rule rather than drawn as a cue.

Name collision to resolve before implementation: the editor already has a `class_name
ScreenMinimap` (`editor/scripts/screen_minimap.gd`, used by `map_editor.gd`,
`placement_editor.gd`, `test_screen_store.gd`) — renaming it is a breaking change across four
files.

### Landed and pushed to `trunk` (kept, but see the caveat above)

- Both bars at `z_index = 20`. Still correct: PlacementEditor's grid sits at `z_index = 5`,
  so a bar at the default 0 would be painted over.
- `_size_layer_repeats()` in `simulator.gd` — `repeat_times` derived as
  `ceil(scrub / repeat_size.x) + 1`.

  > **CORRECTED 2026-10-03.** The original justification here was wrong twice over. The
  > measurement was taken through the displaced canvas, so the numbers never transferred;
  > and the "100% bare at camera x=1600" figure came from the **voided M-d metric** (plan
  > 5.1) — it counted *changed* pixels, so a gap and art scored identically and the
  > metric could not report a gap. M-k measured it properly: `repeat_times` 1/2/3/5 render
  > **pixel-identical** at scrub 0, 1600 and 6400 against a positive control diverging
  > ~1.9M px. The renderer draws N+1 centred copies (inclusive bound), so two copies already
  > cover any viewport up to 3840 px wide. The function is unobservable and is slated for
  > deletion in block 1/6. **Do not cite the bare-percentages in its doc block
  > (`simulator.gd:53`, `:63`) — they are void.**

### Other open state

- **Phase 2** (retire 13 old tracked PNGs, ~15 MB) **blocked on #87**.
- **`chore/godot-4-7-upgrade`** (1 commit, local, unpushed) awaiting a landing decision; it
  carries the gitignore keeping `editor/addons/` out.
- **Wiki freeze**: lifted for `TDD_Parallax-Depth.md` only, then for the minimap section of
  `GDD_World-Layout.md` + its `Home.md` index row, then for the minimap zoom ladder in the
  same section + `minimap_schematic.{svg,png}`. **Nothing else in the wiki may be edited.**
  The first two are merged to `master` (`7e461f8`) and their branches deleted. One wiki branch
  is open, unmerged, awaiting push permission:
  - `docs/minimap-zoom-ladder` — `ace6738`, the three-level zoom ladder (#94).
- Wiki's six recorded discrepancies untouched (FROZEN pending @me's agreement).
- Issues filed today: **#90** celestial body, **#91** signed depth (real sub-issue of #90),
  **#92** four stale `Camera2D` comments + one superseded design plan, **#93**
  `TDD_Parallax-Background` names the camera as the parallax system's only input,
  **#94** the minimap feature request.

### Out of scope, deliberately

#88 stagger constants, taller art from `gen-backdrop`, bar anchors and `z_index`, #87, PNG
retirement, the celestial body (#90), the #91 depth migration, #92's stale comments, #93's
design-page correction. Each declined for a stated reason in the plan.

### Gates

`cargo test -p sstd-core` **110 tests**, the workspace Rust gate. `cargo test -p gen-backdrop`
29/29. `test_parallax_backdrop`, `test_image_to_map`, `test_terrain_brush`,
`test_override_merge` all green. `test_screen_store` exits 1 on Godot 4.7.2 (pre-existing
#85, `ZIPPacker` directory entry) — **not a gate.**

**Gate on the exit code, never on a printed `failures=0` line.**

Issues open: #68, #82, #84, #85, #86, #87, #88, **#89**, **#90**, **#91**, **#92**, **#93**,
**#94**.

---
---

## SUPERSEDED (2026-09-27) — agent-config consolidation + wiki link-rot sweep

_Last updated: 2026-09-27_

**Both repos clean, merged, and pushed, 0 unpushed.** #83 is closed.

| repo | default | head | this pass |
|---|---|---|---|
| `sidescroll-towerdefense` | `trunk` | `72857e2` | consolidated AGENTS + `opencode.json` wiring (#83) |
| `sidescroll-towerdefense.wiki` | `master` | `a61a7ed` | indexed 19 unlinked wiki pages (#83) |

Worked on `fix/agents-consolidate` / `docs/home-md-index`, both merged back with
`git merge --ff-only` and pushed (fast-forwards `47d0552..72857e2` and `8059ddd..a61a7ed`).
**No PR** — @me's standing decision, now recorded in
`~/.cache/opencode/branch-then-merge-repos.md` for both repos. The wiki remote has no usable
API repo at all, so a PR is impossible there regardless.

### Why the `opencode.json` files exist

`.opencode/AGENTS.md` is **not** auto-discovered. The only supported mechanism is the
`instructions` array in `opencode.json`, and neither repo had one — so the project's
knowledge file was never loaded by a session started inside the repo. Config lookup stops
at the nearest git directory, so two files are needed: the repo-root one for repo launches
and `SSTD/opencode.json` for workspace launches. Both verified with `opencode debug config`;
each resolves to a real file and both concatenate with the global
`~/.config/opencode/opencode.jsonc` entries rather than replacing them. **A new session is
required to observe the effect** — resolution is not loading.

### Do NOT redo these

- **The LUCK section of the project AGENTS is wrong** (filed as #84, deliberately left
  undecided). It claims deterministic, no-dice-roll behaviour with `luck.bonus_cap`,
  `crit_interval`, and `rarity_floor` — **none of those three exist in the code**, which
  has `crit_chance`, `WeightedTable::pick`, and `luck_weighted_rarity`. The wiki matches the
  code, not the AGENTS file. Same file also contradicts itself: the adjacent "Replay &
  seeded rolls" subsection correctly describes `seeded_roll` + `RollLog`. Do not "fix" the
  docs here without the owner deciding whether the code or the 2026-08-07 decision is wrong.
- **Coverage counts must be verified against `HEAD`, not trusted from a loop's output.** An
  earlier pass reported 11 unlinked wiki pages; the true figure was 19 (8
  `GDD_Gameplay.*` sub-pages were missed). Check with `git show HEAD:Home.md` before quoting
  a number.
- **Validate a mutation before believing its result.** Two void experiments in one pass: a
  tab injected into a space-indented file, and a "missing page" probe that was also linked
  in `Home.md`. Both produced a clean result that looked like a real finding.
- **`opencode debug config` prints the Anthropic API key in plaintext** (it is in the global
  `opencode.jsonc` under `provider.anthropic.options.apiKey`). Never paste that output into
  an issue, PR, commit message, or log.
- **`godot4` IS on `PATH`** via `~/bin` (`command -v godot4` -> `/home/hidekiai/bin/godot4`).
  The earlier claim that it was "not on the default PATH" was wrong and is corrected in
  `SSTD/AGENTS.md`. The `$HOME/bin/` prefix is kept for portability only.
- **`CONTRIBUTING.md` and `docs/guidelines.md` are in the global instructions list but
  exist in neither root.** They match nothing and are silently skipped. Pre-existing, from
  the global config, not introduced here.

### Shipped last cycle (2026-09-25/26)

| commit | what |
|---|---|
| `2c08989` | hermetic, falsifiable `test_override_merge` guard (#79) |
| `6c0732b` | corrected the withdrawn data-loss claim in this file (#79) |
| `c955cec` | inverted exit ternary in two test runners (#80) |
| `7ff696a` | override-delta write rule + guard requirements, `TDD_World-Archive` §3b / §8a |

Plans: `PLAN-2026-09-25-override-merge-guard-hermetic.md`, `PLAN-2026-09-25-test-exit-codes.md`.

### Do NOT redo these (last cycle)

- **A missing `terrain_overrides.json` is not data loss.** It is correct delta semantics
  and the normal representation of a world at framework defaults. #79's original claim
  was disproved; the measurements behind it were right, the inference was wrong.
- **The pre-#76 world backup in `/tmp` is skipped by user decision** (2026-09-25). Do not
  restore, chase, or re-raise it. `/tmp` does not survive a reboot; that is accepted.
- **Do not bundle the `entity_overrides` dead-channel removal** into another fix — it
  removes a `save_world` positional parameter, a breaking contract change.
- **Indentation is mixed**: `editor/scripts/entity_editor.gd` uses spaces,
  `editor/scripts/terrain_editor.gd` uses tabs. Injecting a line with the wrong style is
  a parse error, not a behavioural mutation; this invalidated a whole experiment once.

### Open threads

| # | item | why still open |
|---|---|---|
| #62 | save/load dialog default filter | code-fixed via filter-**ordering** (`current_filter` does not exist in Godot 4.4); needs a real-display click-through. A display **is** available here (`DISPLAY=:0.0`) |
| #81 | `test_screen_store` silent-truncation exposure | 86 `await` sites, no completion tracking. **Unproven** — the issue carries the experiment that would settle it |
| #68 | parallax runtime + legacy single-screen `.json` round-trip | parked; probe GREEN, runtime rendering unverified |
| #82 | `entity_overrides` dead channel | proven: written to the archive, read into the load result, **zero consumers**; only producer is a stub returning `{}`. Deleting it removes the 5th positional `save_world` param — breaking, own review |
| #84 | LUCK AGENTS section contradicts code, wiki, and itself | filed from #83; needs an owner decision, deliberately not resolved here |

Also still open from earlier cycles: #44, #65, #69.

### Next move

1. **#84 is the only thing this pass left open.** Once decided, correct either the AGENTS
   LUCK section or the luck code/wiki, and reconcile the "Replay & seeded rolls" subsection
   either way. Note that `seeded_roll`/`RollLog` are also load-bearing for the replay/CI
   checksum invariant, so a "make it deterministic" fix cannot simply delete them.
2. Start a **new session** to confirm the `opencode.json` wiring actually loads. The config
   resolves, but resolution is not loading — only a fresh session proves the AGENTS file is
   in context.

### Gate

`cargo test -p sstd-core` (**110** passed) plus all four GDScript suites, which must
each exit 0 — the exit code is authoritative, not the printed `failures=0` line:

```bash
~/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd    # 187 ok
~/bin/godot4 --headless --path editor --script res://tests/test_image_to_map.gd    # 9 ok
~/bin/godot4 --headless --path editor --script res://tests/test_terrain_brush.gd  # 14 ok
~/bin/godot4 --headless --path editor --script res://tests/test_override_merge.gd  # 26 ok
```

---

## SHIPPED (2026-09-25) — stale-issue sweep: #38 #49 #51 #63 closed; #76 re-cited; #79 filed

> Bookkeeping pass, no code changed. Four issues were OPEN but had shipped weeks
> ago; each was verified against the code before closing. Also filed a real bug
> found while gating (see #79 below).

**Closed, with evidence comments.** Each closing comment names the wiki page as
the design of record, the shipping commits, the acceptance criteria point by
point, and the test that guards it. Commit hashes live on the issues, never on
the wiki.

| Issue | Feature | Wiki page (design of record) | Guard |
|---|---|---|---|
| #38 | image-stamp importer | `TDD_Tile-Reuse-and-Stamp-Groups` §2 | `test_screen_file_json_roundtrip` (carries a `stamp_maps` entry) |
| #49 | visual tile palette | `TDD_Tile-Reuse-and-Stamp-Groups` §10 | `_test_tile_palette` |
| #51 | Prune Duplicates | `TDD_Tile-Deduplication` + §11 | 6 prune tests (`_test_prune_duplicates` … `_test_bridge_exact_scan`) |
| #63 | Collision Map button | `TDD_Collision-Map-Tooling` | **none — §9 is manual-only** (see caveat) |

**#63 verification caveat, recorded deliberately:** there is no headless
regression test for the Collision Map button, so the suite's `failures=0` does
*not* cover it. The only evidence of a live run is the user report at
`docs/SESSION-CHECKPOINT.md` line ~472 ("4 tile flicker / quadrant toggle"),
which confirms direct quadrant-paint ran on a real display. Treat
`TDD_Collision-Map-Tooling` §9 as the spec if it regresses.

**#44 stays open.** #49 shipped the palette that #44's region-select grouping
needs as its picker, but the region-select tool itself is still unbuilt
(`TDD_Tile-Reuse-and-Stamp-Groups` §4, "the gap").

**Wiki commit (local, wiki repo `master`):** `TDD_Tile-Reuse-and-Stamp-Groups`
header now cites #38/#46/#49/#51, each pointing at the section documenting it,
plus the guard test per feature — previously the header cited only #46, so the
importer's design linked to no issue and the traceability record was broken at
the join. `TDD_Parallax-Background` §4.2 gained the missing #76 art-to-tiles
import step (`ImgMapBtn` -> `_on_img_map_import` -> `_import_image_to_tiles` ->
`_nearest_terrain_key`), including the point that `air` is never returned so sky
becomes *no cell* rather than an air tile — that is what feeds the §2 hole model.
`TODO.md` TS41 -> `done`, TS62/63/64 record the closes, TS65 the sweep itself.
No commit hashes were added to the wiki (the two pre-existing ones in TS47/TS49
violate the rule and were left alone as out of scope).

**#76 doc hygiene: DONE.** Its closing comment cited this checkpoint instead of a
wiki page, breaking the Documentation Reference Architecture rule. There was no
wiki page to cite, so the §4.2 addition above was written first, then the comment
was re-cited in place with `gh issue comment 76 --edit-last` (verified: 1 comment
on the issue, replaced not appended). This resolves the "Outstanding doc hygiene
(not yet done)" note in the 2026-09-24 block below.

**#62 deliberately LEFT OPEN.** Code-fixed via filter-ORDERING, not a property:
`FileDialog.current_filter` does not exist in Godot 4.4, so instead the filter
list is ordered by world format — package-backed world puts `*.zip` first
(`map_editor.gd:1098-1104` save, `:1167-1173` load), legacy manifest puts
`*.json` first. Unverified because it needs a real display. A display *is*
available on this host (`DISPLAY=:0.0`, `/tmp/.X11-unix/X0`, seat0 session), so
this is closeable on request: open a package-backed world, confirm the filter
dropdown defaults to "SSTD World Package", then a legacy world and confirm
"Screen JSON (legacy)".

**#79 SHIPPED and CLOSED — the "data loss" reading was WRONG. Withdrawn.** The section
this replaces claimed "real data loss, cause not yet pinned", and flagged an
unproven `map_editor` / `placement_editor` asymmetry. Both claims were investigated
and both are false. **The measurements below are correct; the interpretation was not.**

What the archive diff actually shows (still valid, and it is what pinned the cause):

| | pre-#76 backup (2026-09-14) | current `editor/world.zip` (2026-09-19 13:22) |
|---|---|---|
| `terrain_overrides.json` | `{"air":{"sub_tile_mask":15},"dirt":{"sub_tile_mask":0}}` | **absent** |
| archive size | 1,259,491 B | 3,873 B |
| `screens/1.json` | 230,849 B | 6,764 B |
| tile bank | ~90 `stamp_N` tiles | `dirt.png`, `grass.png` |

No data was lost. Two things had been conflated:

1. **Boot never reads `res://world.json`.** `main.gd:4` uses
   `user://data/world.json`, and on this host that file is an empty legacy
   manifest, `{"screens": {}, "version": "0.2.0"}` — zero screens, no `.world`
   key. So `on_world_loaded` receives no overrides, `apply_terrain_overrides` is
   never called, the Terrain Editor sits at framework defaults, and
   `collect_terrain_overrides()` correctly returns `{}` — a zero delta against a
   framework it exactly matches. `world_archive.gd:117` writing only on a
   non-empty delta is **correct** delta semantics, symmetric with the read at
   `:181-184`. There is no defect there.
2. **The file was absent because the world was replaced, not corrupted.** A
   1.26 MB / ~90-tile world and a 3.8 KB / 2-tile world are different worlds, not
   one damaged world. The overrides belonged to the world that was swapped out.
   What caused the 2026-09-19 reset is still not established, but it is
   environment rot, not a code defect, and it is not this issue.

The `map_editor` / `placement_editor` asymmetry was also wrong:
`_collect_entity_overrides()` at `placement_editor.gd:598` is a **stub that returns
`{}`**, so both call sites pass `{}`. What is real is that `entity_overrides` is a
dead channel end to end — a separate follow-up, not folded into #79.

The actual defect was the test, and it was worse than "not reproducible":

- **Non-hermetic.** The guard read its input from gitignored, untracked
  `editor/world.zip`, so it asserted machine state, not tracked code behaviour.
- **Two vacuous assertions.** It checked `arrow_tower` `attack_power=100` /
  `max_hp=500` and `mine` `class=trap` — which are the framework prototype's own
  values in `default_package/entity_defs.json`. Those assertions passed whether
  or not the merge ran. That is why the entity half stayed green while the terrain
  half went red: the entity half was never testing anything.
- **Silent truncation.** A guard is a coroutine; a runtime script error aborts it
  but lets `_run` continue, so the suite printed `failures=0` and exited 0 with
  the merge never called. Proven by injecting a tab into space-indented
  `entity_editor.gd`.
- **Coverage hole.** The archive codec for overrides had no test with a non-empty
  payload, so a save/load asymmetry or a rename of `TERRAIN_OVERRIDES_PATH` would
  go unnoticed.

Fixed in `2c08989`; guard now has three falsifiable parts (terrain merge, entity
merge, archive codec round-trip) with vacuity tripwires and a completion check.
Verified by mutation, all files restored: no-op `apply_terrain_overrides` → 2
failures; no-op `apply_world_entity_defs` → 4 failures; renamed
`TERRAIN_OVERRIDES_PATH` → 1 failure; `world.zip` moved aside → 0; tab-parse abort
→ truncation check fires. Plan and full evidence in
[PLAN-2026-09-25-override-merge-guard-hermetic](PLAN-2026-09-25-override-merge-guard-hermetic.md).

**#80 filed** while verifying: `test_image_to_map.gd:63` and
`test_terrain_brush.gd:70` both have an inverted ternary,
`quit(1 if failures == 0 else 2)`, so they exit non-zero when they **pass**. They
survived because a failing test also exits non-zero, which satisfies an
exit-code check for the wrong reason. Not touched by #79.

`editor/world.zip` and the backup were never modified. The backup is still at
`/tmp/user/1000/opencode/sstd-world-pre-image-2026-09-19.zip`; `/tmp` does not
survive a reboot, so copy it somewhere durable if the pre-#76 world still matters.

**Legacy single-screen `.json` layer round-trip stays parked in #68.** It is
in-flight #68 work, not a repo regression, so no bug was filed. `_on_import_file`
-> `_load_screen_file` -> `_apply_screen` goes through `_serialize()`, which has
no layer payload; only the `.zip` path round-trips (`_serializable_layers` ->
`WorldArchive.save_world` -> `_load_world_package`). Recorded as a known gap in
`TDD_Parallax-Background` §4.2.

**Active step (next move): pick one, in order.**
1. #79 — pin the save-time cause, then fix. Requires reading the
   `map_editor.gd:1124` / `placement_editor.gd:574` call sites against the
   `on_world_loaded` path, and deciding the fixture question for the gitignored
   `world.zip`.
2. #62 — needs only a display click-through; see above.
3. #68 — the runtime `biome_backdrop_layers` DB binding is the one real open
   feature on the editor side. `grep biome_backdrop_layers` hits only docs, zero
   code or DB, so nothing populates those rows yet. `TDD_Parallax-Background` §2
   is the runtime model, §4.2 the editor tooling already shipped.

## SHIPPED (2026-09-24) — #68 parallax scroll probe GREEN; #74 and #76 closed

> Catch-up block. The previous newest entry was 2026-09-14, so the 2026-09-19 to
> 2026-09-24 work was unrecorded; restored here from `git log` + issue history.
>
> **#74 slice-layers (CLOSED 2026-09-13; commits d27f845, e94895a).**
> `tools/slice-layers` derives the L0/L1/L2 parallax layers from a rendered preview MP4
> by flow-not-ML depth recovery. Source MP4s stay local-only (`.gitignore`, user
> directive 2026-09-13); the derived layers are the artifact of record.
>
> **#76 image -> terrain map import (CLOSED 2026-09-19/20; commits 03e3272, 15ea539,
> d225363).** `map_editor.gd` imports any PNG and converts colour to terrain by
> quantisation. The resulting near band (`layer_2.png`) is rendered as a collidable
> backdrop strip in BOTH the Simulator stack and the MapEditor grid; the x5 collision
> strips are committed under `editor/assets/backdrop_layers/`. Outstanding doc hygiene
> (not yet done): the #76 closing comment cites this checkpoint instead of a wiki page.
>
> **#68 parallax scroll (still OPEN; probe GREEN 2026-09-24, commits 6f1f9da, 3dff990).**
> The editor scrollbar now drives a `Camera2D` at 1:1 and the near band sits at a static
> 0.9x factor. Probe: `editor/tests/probe_parallax_scroll.gd`. Wiki section 4.2 (Map
> Editor layer management) was committed 2026-09-25. Still to land for #68: the runtime
> `biome_backdrop_layers` DB binding (v2) and the wiki acceptance-criteria tick-off.
>
> **Active step (next move): stale-issue sweep.** OPEN but already shipped — #38
> (image-stamp importer), #49 (visual tile palette), #51 (Prune Duplicates button), #63
> (Collision Map button). #62 (load/save dialog default filter) is code-fixed via
> filter-ORDERING, because `FileDialog.current_filter` does not exist in Godot 4.4, and
> needs a real-display GUI re-verify before it can be closed.
>
> **Housekeeping.** Godot 4.4 `.uid` sidecars are now tracked for all scripts
> (20 were already tracked, 16 more added this pass). The pre-#76 world package
> `editor/world.zip.bak-2026-09-19` (1.26 MB) is parked at
> `/tmp/user/1000/opencode/sstd-world-pre-image-2026-09-19.zip` for the older-data-load
> and serde-versioning check — the `*.bak` rule in `.gitignore` does not match
> `*.bak-<date>`, which is why it kept appearing as untracked.

## SHIPPED (2026-09-06) — #7 rarity FK seed + doc integrity (impl from PLAN below)

> Implemented per the PLAN block below and landed this pass.
> - `crates/sstd-core/src/config.rs`: population `005` creates + seeds
>   `gacha_rarities` (R=1/SR=2/SSR=3/UR=4, display names per GDD_Progression-Gacha)
>   and `guild_revive_cooldowns(rarity_id INTEGER PRIMARY KEY REFERENCES
>   gacha_rarities(id), cooldown_scenarios)` (R=5/SR=3/SSR=1/UR=0); new public
>   structs `GachaRarity` + `GuildReviveCooldown`, accessors `gacha_rarities()` /
>   `guild_revive_cooldowns()`; schema_version now `0.0.5`. Re-exported in lib.rs.
> - Tests added: `test_gacha_rarities_seeded`, `test_guild_revive_cooldowns_seeded`,
>   `test_rarity_fk_integrity` (PRAGMA foreign_key_check -> 0 violations),
>   `test_gacha_seed_idempotent` (re-run keeps 4/4). Updated
>   `test_schema_version_after_population`/`test_population_applied`. cargo: 110 core,
>   full workspace 136 pass.
> - Wiki: TDD_Training-Center Revive Mechanics now FK lookup via
>   `guild_revive_cooldowns`; the 4 loose `training_guildReviveCooldown{R,SR,SSR,UR}`
>   config keys deleted from the GM-config table; TDD_GM-Config Gacha Rarity Enum gains
>   consumer note; TDD_Enum-Tables gains `gacha_rarities` section + cross-ref matrix row.
> - Bridge: no change (no GDScript consumer of gacha tables yet — deferred).

## PLAN (pre-implementation, 2026-09-06) — #7 rarity FK seed + doc integrity

> Grounding (read before planning): `crates/sstd-core/src/config.rs` (Population
> mechanism `Population{id,description,func:`}, `POPULATIONS` array, `pop_id_to_version`
> id->version, `run_populations` gate, `INSERT OR IGNORE` seeding, `all_config`/loaders/
> tests at lines 555-730), `crates/sstd-editor-bridge/src/lib.rs` init_config_db,
> wiki `TDD_GM-Config.md` ("Gacha Rarity Enum" `gacha_rarities` table, line 75),
> `TDD_Training-Center.md` (Revive Mechanics + GM config rows 98-111),
> `TDD_Enum-Tables.md` (enum inventory + cross-reference matrix, lines 351-364).
>
> **Defect (issue #7):** rarity is referenced as loose TEXT config keys
> (`training_guildReviveCooldown{R,SR,SSR,UR}`) and an inline comment, NOT FK-encoded;
> the `gacha_rarities` enum table is absent from TDD_Enum-Tables inventory, and has no
> SQLite presence. Design decisions D6/D7 require INTEGER FK references for every enum.
>
> **Design (module ownership: `crates/sstd-core/src/config.rs` owns schema + accessors;**
> **bridge exposure deferred — no GDScript consumer exists yet):**
> 1. Add `Population { id: "005", description: "Gacha rarity enum + guild revive cooldowns", func: populate_005 }`.
>    `pop_id_to_version("005")=5` -> `schema_version` becomes `0.0.5`. Idempotent on
>    existing DBs (CREATE TABLE IF NOT EXISTS + INSERT OR IGNORE), matching populations 001-004.
> 2. `populate_005` (execute_batch):
>    - `gacha_rarities(id INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE, display_name TEXT NOT NULL, sort_order INTEGER NOT NULL)`
>      seeded R/SR/SSR/UR x Rare/Super Rare/Specially Super Rare/Ultra Rare, sort 0-3
>      (names per GDD_Progression-Gacha; schema per TDD_GM-Config Gacha Rarity Enum).
>    - `guild_revive_cooldowns(rarity_id INTEGER PRIMARY KEY REFERENCES gacha_rarities(id), cooldown_scenarios INTEGER NOT NULL)`
>      seeded R=5/SR=3/SSR=1/UR=0 (per TDD_Training-Center Revive Mechanics). Replaces the
>      4 loose config keys. Revive-fee formula key `training_guildReviveBaseFee` stays a config key.
> 3. New public structs `GachaRarity` + `GuildReviveCooldown` and accessors
>    `gacha_rarities() -> SstdResult<Vec<GachaRarity>>`,
>    `guild_revive_cooldowns() -> SstdResult<Vec<GuildReviveCooldown>>` following the
>    existing loader pattern (grid_config/revive_config); re-export in lib.rs.
> 4. Tests (config.rs `mod tests`): rarity rows=4; cooldown rows=4 with exact mapping;
>    `PRAGMA foreign_key_check` empty (referential integrity, works even without the FK
>    pragma enabled via connection); idempotent re-run (no dupes); update
>    `test_schema_version_after_population` -> "0.0.5" and `test_population_applied` + "005".
> 5. Wiki, same pass: TDD_GM-Config Gacha Rarity Enum gains consumer note
>    (`guild_revive_cooldowns.rarity_id`); TDD_Enum-Tables gains "Gacha Rarities" section
>    (canonical pointer to GM-Config) + cross-reference matrix row; TDD_Training-Center
>    Revive Mechanics rewritten to FK lookup + GM-config table rows replaced.
> 6. No Rust enum change for `RarityTier`/`DEFAULT_RARITIES` (luckbot.rs) — that is a
>    separate loot-weighting vocabulary (common/uncommon/rare/epic/legendary), untouched.

## SHIPPED (2026-09-06) — A19/A20 stale-code reconciliations (issues #24/#25, doc-only)

> Chosen resolutions (user): (a) #24 biome — keep code `BiomeType` (gameplay terrain
> enum == TDD_Enum-Tables `biome_types`, no Rust change) and the GDD_Art-Direction
> palette as a SEPARATE visual taxonomy with an explicit palette->biome mapping table;
> (b) #25 surface — `SurfaceType` (terrain movement modifiers) and transport road
> tiers (hauler speed/fuel bonuses) are separate concerns; road tiers will be a
> transport-system enum when implemented, NOT `SurfaceType` variants.
> Edits (wiki, commit ref on issues): GDD_Art-Direction Biome Palette gained the
> mapping column + reconciliation note; TDD_Enum-Tables biome_types + surface_types
> gained separation notes; TDD_Transport-Infrastructure Road Tiers gained the
> forward-design note. Also fixed a pre-existing unclosed code fence at the end of
> TDD_Enum-Tables (biome_types INSERT block; odd fence count). TODO.md A19/A20 -> done.
> No Rust/GDScript changes. Confirmed code `BiomeType` is `Crystal` (not Frozen) and
> matches `from_str` keys exactly.

## SHIPPED (2026-09-06) — #68 parallax background: TICKET + DESIGN (documentation-first)

> Recorded feature request promoted to a tracked ticket. Issue #68 created
> (`feat(f6): depth-layered parallax background — both-axis 2D backdrop scroll, no
> scanline`). Design authored BEFORE any code: wiki `TDD_Parallax-Background.md`
> (depth-layered layer model L0/L1/L2 with per-axis factors, Godot `Parallax2D`
> compositing under a Camera2D, `biome_backdrop_layers` data model keyed on corridor
> biome, holes = alpha cutouts, scanline + fake-hole sprites explicitly prohibited,
> <= 7 layers, validation checklist) and a `GDD_Art-Direction` "Parallax Background"
> section (per-palette L0/L1/L2 stacks + combat-readability rule). No code written.
> Issue #68 stays OPEN until the engine runtime renders it (runtime rendering is not
> implemented yet — camera/Parallax2D absent). Grounding: TDD_Map-Hierarchy screen/
> corridor model; issue #24 biome-vs-palette taxonomy separation. Will absorb the
> v2 screen-override hook and `parallax.*` config keys at implementation time.

## SHIPPED (2026-09-07) — #69 OpenRouter gateway: TICKET + DESIGN (documentation-first)

> New feature request recorded and driven to a tracked ticket. Issue #69 created
> (`feat(f7): OpenRouter gateway — agent-driven content generation (tilesets then
> maps) via gRPC`). Scope resolved via question flow (2026-09-07):
> - Transport = **strictly gRPC**. Investigation finding: Godot 4 GDScript has NO
>   native MCP support, so an MCP server would only ever ride the same Rust
>   GDExtension bridge as the tonic gRPC server (an adapter, not a new integration).
>   User-verified priority: all analysis CPU-side in Rust, token-thrifty.
> - Mixed key model: SSTD-side `sstd-openrouter` HTTP client for image generation
>   (agents cannot unvalidated-blast the tile bank); agents keep their OWN text/
>   instruction channel; key is a secret (env / `user://data/secrets/openrouter.key`,
>   0600, never config store / wiki / world zip).
> - Slice A = tilesets (text-to-image -> decode -> nearest-resize 32x32 ->
>   palette_conformance + alpha check -> stage -> HUMAN APPROVAL gate -> bank via
>   existing bridge path). Slice B = maps (structured screen JSONC ->
>   schema/dimension/terrain-ref/stitch validation -> WorldArchive world .zip).
> - Design authored BEFORE code (per #68 precedent): wiki `TDD_OpenRouter-Gateway.md`
>   (AuthoringService = agent-action service #9 on `sstd-grpc`; protobuf surface;
>   bridge-command pattern mirroring `EditorCommand`; OpenRouter facts: Image API
>   `POST /v1/images` with `data[].b64_json`+`media_type`, capability discovery is
>   truth, routing/failover OK; `openrouter.*` config domain = population 006 ->
>   schema_version 0.0.6; `generation_log` journal-grade table; budgets as credit
>   STRINGS not floats; token-thrift validation contracts; testing plan) +
>   `GDD_AI-Content-Workflows.md` (authoring workflow + editorial guardrails).
> No code written. Issue #69 stays OPEN until an implementation commit exists.
> Grounding: config.rs POPULATIONS 001-005 + ConfigStore; sstd-grpc
> EditorCommand/spawn_server + editor.proto; sstd-editor-bridge import_terrain_types/
> import_screen; TDD_Agent-Action-Architecture 8-service model + MCP-tools analogy;
> TDD_Saved-World WorldArchive; GDD_Art-Direction palette; OpenRouter image docs.

## SHIPPED (2026-09-07) — #70 gRPC/protobuf service index TDD

> Audit + index produced. CODE TRUTH (verified by grep, no inference): exactly 1
> proto file `crates/sstd-grpc/proto/editor.proto` (package sstd.editor), 1
> service `EditorService`, 2 RPCs (SwitchTab, CaptureScreenshot), 4 messages;
> tonic server on a background tokio thread inside the Rust GDExtension bridge
> (SstdBridge drains EditorCommand via mpsc; spawn_server pattern; e2e test via
> tonic Channel). DESIGNED surface (wiki TDDs): 12 services, ~113 RPC pairs
> (~109 net unique after overlap) = Agent-Action 8 services/62 + Service-Contract
> 13 + Map-World spatial/coordinate 26 + SimService ~4 (proto TBD) + GameAgent 1
> (Play stream) + AuthoringService 7 (#69). Implemented share ~1.8% (2/113).
> New wiki `TDD_gRPC-Service-Index` = authoritative inventory: per-service table,
> consolidated proto plan (sstd.common/editor/actions/spatial/sim/authoring/
> multiplayer), phased rollout gated on consumers (0 editor->1 spatial->2 sim->
> 3 action->4 authoring->5 multiplayer), per-service test parity (conversion +
> mpsc round-trip + e2e + action_log). AGENTS.md gained the maintainability rule:
> any new proto/gRPC work MUST update the index page in the same commit.

## SHIPPED (2026-09-07) — #71 living protobuf/gRPC registry

> User directive: "we NEED some kind of LIVING document that explains what each
> protobuff and gRPC does." Delivered: `crates/sstd-grpc/proto/README.md` (main
> repo, beside the IDL) = living registry, one row per proto (package, service,
> RPCs, status, WHAT IT DOES) + conventions (additive-only, reserved, codegen,
> CCMV); index gained a "3-layer model" section (L1 Service Contract = deepest,
> transport-agnostic, already served by non-protobuf GDExtension #[func] +
> EditorCommand mpsc; L2 gRPC surface; L3 protobuf messages, reusable beyond
> gRPC — hence gRPC != protobuf) and a per-service PURPOSE column in the
> inventory table. Lifecycle rule (AGENTS.md + index banner): ANY proto/gRPC
> change ships IDL + proto README row + index purpose text in the SAME commit.

## SHIPPED (2026-09-07) — #72 single proto source of truth (glob + one-lib)

> build.rs compiled one hardcoded proto; a 2nd protocol file would silently never
> build. Now compiles every *.proto under crates/sstd-grpc/proto/ via
> std::fs::read_dir glob (sorted, empty -> hard error), keeps PROTOC from
> protobuf-src, and emits cargo:rerun-if-changed per file + for the dir. Verified:
> cargo check -p sstd-grpc clean, cargo test -p sstd-grpc 6 tests green
> (include_proto!("sstd.editor") still resolves). Governance: proto/ = ONLY schema
> dir; consumers depend on sstd-grpc (never hand-roll/re-generate wire types; the
> JSON #[func] bridge is a separate non-protobuf contract); every new proto ships
> .proto + include_proto + README row + index purpose in ONE commit. Recorded in
> proto/README.md, TDD_gRPC-Service-Index (wiki), AGENTS.md.

## SHIPPED (2026-09-10) — #67/#65 docs completion (resumed from the pre-reboot interrupt)

> Resumed the 2026-09-07 in-flight task ("Close #67 + #65 docs"). The #67 wiki page
> `TechnicalDesign/TDD_Def-Overrides.md` had already landed pre-reboot (wiki commit
> `13d74b5`). Remaining work completed this pass:
> - **#65 design authored (design-only, no code):** wiki `TechnicalDesign/TDD_Terrain-Type-Hierarchy.md`
>   — air/ground root base types (immutable, un-deletable), `parent_type` field (empty
>   string = root), parent-chain merge root->child, composition order documented as
>   framework prototype -> hierarchy merge -> world override (per TDD_Def-Overrides),
>   validation (known parent, no self-parent, acyclic, roots required, no re-parent in
>   world overrides), inheritance-aware editor UI (grey inherited fields, lock roots,
>   dependent-delete guard), schema self-FK `parent_id` on `terrain_types` (enum-table
>   surface), proposed default hierarchy for the 7 framework types, reconciliation note
>   that the runtime `TerrainType` enum is untouched by the hierarchy.
> - **Rows added:** wiki Home.md (Technical Design: `TDD_Def-Overrides`, `TDD_Terrain-Type-Hierarchy`),
>   wiki TODO.md (new "This Session 2026-09-07" block, TS57/#67 done, TS58/#65 designed,
>   footer date).
> - **Issue states:** #67 CLOSED (closing comment cites wiki page + code commit
>   `9d78881`); #65 stays OPEN with "designed, no code" comment (Issue-State Hygiene).
> - Branches: main repo `trunk` at `c84b901` (clean), wiki `master` updated this pass.
>   NEXT (cold resume): none — no in-flight work remains.

## INTERRUPTED (2026-09-07, pre-reboot) — #67/#65 docs task, user chose "Close #67 + #65 docs"
> **SUPERSEDED by the SHIPPED (2026-09-10) block above — all remaining steps below are now done.**

> Active work (see `.opencode/sessions/2026-09-07-grpc-def-overrides-session.md` for
> the fully self-contained resume):
> DONE: #67 wiki page `TDD_Def-Overrides` authored + committed (wiki 13d74b5) — #67
> code was already shipped (main 9d78881), cargo test -p sstd-core 110 pass.
> REMAINING after reboot in runnable order: (1) author wiki
> `TechnicalDesign/TDD_Terrain-Type-Hierarchy.md` as DESIGN-ONLY (#65 has NO code,
> cannot be closed per Issue-State Hygiene; keep OPEN, comment designed), (2) Home.md
> rows + TODO rows + this file's SHIPPED block, (3) commit wiki citing #65 (and #67),
> (4) gh issue close 67 --comment "<wiki + 9d78881>", (5) comment on #65 stays open.
> Main repo is otherwise CLEAN at c3af481 (#72); wiki at 13d74b5.

## SHIPPED (2026-09-10) — #73 over-engineering audit ledger

> Whole-repo punk-audit done + persisted to wiki `TechnicalDesign/Engineering-Audit.md`
> (ranked: unwired LuckBot/Revive/items engine ~1,345 lines / schemars-yagni /
> grpc_smoke dup / gen-icons shrink; net -~1,800 lines -1 dep possible).
> Nothing deleted — cuts are PROPOSED, tracked on issue #73. Page carries the
> 4 delta-check greps; AGENTS.md now mandates delta-append, never full re-audit.
> Wiki row + TODO TS59 + footer updated; main clean otherwise.

## SHIPPED (2026-09-10) — #73 pruning gate applied (F3+F4 pruned, F1+F2 kept)

> User gate for irreversible cuts: (1) git recovery documented, (2) NOT on any
> TDD/GDD — documented design stays. Applied: grpc_smoke.rs example + gen-icons
> tool deleted (-656 lines); recovery at pre-prune HEAD 96b2f45 (commands on wiki
> Engineering-Audit). Kept: LuckBot/Revive/items engine (~1,345 lines) + schemars
> = TDD-documented (TDD_LuckBot/TDD_Luck-Gear/TDD_Death-Revive-System /
> TDD_Saved-World). cargo check -p sstd-grpc green; ledger + TODO updated.

## SHIPPED (2026-09-13) — #74 slice-layers: parallax layers recovered from the MP4

> Asset-production tool `tools/slice-layers` (Rust, workspace member, `image` 0.25
> only). Depth proxy = measured horizontal displacement between two frames ~1s apart
> ("flow, not ML") — the only surviving parallax artifact was the rendered MP4
> (`assets/samples/preview-with-parallax-extended.mp4`, 1280x720@24fps, 20s); no
> per-layer source art existed. Pipeline: 8x8 SAD block match at scale 2, reliability
> median-fill, border smear (unvisited ring = spurious k-means cluster), k-means
> best-of-3-inits (single init trapped {4,10,20}->{6.8,19.2,24.6}), robust p5..p95
> single-plane reject, per-band RGBA compose with feathered alpha. Tests 4/4 green
> (synthetic 3-pane pair, non-periodic content; periodic content aliases in SAD).
> Ran on frame_0010/0011: bands 39.4 / 141.0 / 240.4 px/s -> factor_seed
> [0.15, 0.53, 0.90] (within TDD L0/L1/L2 ranges; L1 0.53 ~ upper bound 0.5).
> Committed `assets/backdrop_layers/{layer_0,1,2,displacement}.png`; alpha coverage
> 32% / 27% / 41% (holes non-trivial). TDD_Parallax-Background grew §4.1 "Source-art
> recovery". Issue #74 closed with commit.
>
> **Repo policy (2026-09-13):** the source MP4s (both preview renders) are
> local-only — `git rm --cached`'d + gitignored (`assets/samples/*.mp4` and the
> 1fps frame dir `assets/samples/preview-with-parallax-extended/`); not going to
> repo/LFS. Committed `assets/backdrop_layers/*.png` are the derived artifact of
> record; regeneration requires the local MP4s (command documented on
> TDD_Parallax-Background §4.1).

## SHIPPED (2026-09-14) — #68-backed preview scene + Map Editor backdrop

> Three related pieces:
> 1. **Preview scene** `editor/scenes/backdrop_preview.tscn` committed (commit
>    `fa6a059`, pushed `trunk` on 2026-09-14): 3 `Parallax2D` nodes (L0 far,
>    L1 mid, L2 near) with `scroll_scale` from factor seed `[0.15, 0.53, 0.90]`
>    (x) / `[0.13, 0.45, 0.76]` (y ≈ 0.85x). Sprites reference the layer PNGs
>    from `editor/assets/backdrop_layers/` (copied from the derived asset dir
>    for Godot import). Scene is the runtime-parallax verification view.
> 2. **Map Editor backdrop** (`editor/scripts/tile_grid_display.gd`): draws the
>    same 3 layer PNGs at the start of `_draw()` — a parallax-agnostic static
>    backdrop behind the 60x33 tile grid, scaled 1280x720 -> 1920x1056 (~1.5x,
>    painted-art acceptable). Cosmetic only; the main tile grid remains
>    map+collision. Design decision from user: NO multi-layer tile editor — the
>    3 parallax layers are purely eye-candy, NOT editable tile layers.
> 3. **View artifacts** `assets/backdrop_layers/view/` (gitignored): per-layer
>    flatten-on-black previews + `backdrop_stacked.png` (far->near composite)
>    so layers can be inspected independently of viewer checkerboard.
>
> **FIX (same session):** backdrop was invisible behind the default grid — every
> air cell painted an opaque sky-blue `draw_rect` (`terrain_color("air")`,
> `#87ceeb`). Root cause: only "air" cells were opaque fills; all grid cells are
> air by default. Fixed in `tile_grid_display.gd` `_draw()`: skip cells whose
> key is "air" entirely (`key != "air"`), so air = transparent and the backdrop
> shows through. Applies to the whole grid uniformly (single guard in the shared
> draw loop, not per-caller).

## SHIPPED (2026-09-14) — Parallax layer stack: 4 layers with visibility toggles

> **BACKGROUND (user design, this session):** the parallax backdrop is cosmetic
> only (NOT an editable tile layer); the main tile grid stays map+collision.
> Layer count will eventually be dynamic (user adds/removes), but Phase 1 here is
> a hardcoded 4-layer stack with per-layer visibility toggles in the Map Editor.
> Layer 0 is the FRONT-MOST foreground (transparent placeholder until foreground
> art exists — e.g. pillars/cacti rushing past); layers 1-3 are the existing
> sliced background bands remapped (Near = layer_2.png, Mid = layer_1.png, Far =
> layer_0.png), all BEHIND the tile grid.
>
> **Draw order (back to front):**
> - Back layers (z_order < 0, sorted ascending): Far -> Mid -> Near
> - Tile grid (gameplay + collision)
> - Front layers (z_order >= 0, sorted ascending): Foreground
>
> **Change set (all local READ, not yet pushed):**
> - `editor/scripts/tile_grid_display.gd`: replaced flat `_backdrop_layers`
>   with `_layers: Array[Dictionary]` (name/z_order/visible/path/texture).
>   `_load_backdrop()` now lazy-loads each layer texture; `_sorted_layer_indices(back)`
>   returns sorted visible indices per phase; `_draw()` splits into back / tile /
>   front. `set_layer_visible(index, visible)` added for UI toggles.
> - `editor/scenes/map_editor.tscn` + `editor/scripts/map_editor.gd`: layer
>   visibility CheckButtons (Layer0Btn..Layer3Btn) wired to
>   `tile_grid.set_layer_visible(index, visible)`.
>
> **SHIPPED (2026-09-14, Phase 2 — full layer management, ref #68, commit `fff4c7c`):**
> Dynamic layer stack fully implemented. `tile_grid_display.gd`: `opacity` field per
> layer, `set_layer_opacity`, `set_layer_name`, `add_layer(back)`, `remove_layer(index)`,
> `swap_layer_z(a,b)`, `get_layers()`, `set_layers()`; `_draw()` uses `Color(1,1,1,opacity)`
> for both back and front layers. `map_editor.gd`: static 4-CheckButton layer box replaced
> with dynamic panel (`LayerList` ItemList, `LayerVisible` CheckButton, `LayerOpacity`
> HSlider, `LayerName` LineEdit, `AddLayerBtn`/`DelLayerBtn`/`UpLayerBtn`/`DownLayerBtn`);
> `_sorted_indices` maps ItemList position → `_layers` array index; `_serializable_layers`
> strips `texture` key for JSON; `_load_world_package` restores layers from world data.
> `world_archive.gd`: `LAYERS_PATH = "layers.json"`, `save_world` accepts `layers` array,
> `load_world` parses it back. Both `map_editor.tscn` AND `main.tscn` updated (mirrored
> per issue #75). Up/Down swaps `z_order` values (not array order), matching the draw loop's
> `_sorted_layer_indices(back)` sort-by-z design. World packages without `layers.json` fall
> back to the hardcoded 4-layer default (backward compatible).
>
> **SPIKE (2026-09-14, confusion resolved):** the MapEditor node tree EXISTS TWICE —
> `editor/scenes/map_editor.tscn` is a standalone dev/test scene, but the app
> embeds its OWN inline copy of the MapEditor tree inside `editor/scenes/main.tscn`
> (node path `TabContainer/MapEditor`, NOT an instance). Edits to `map_editor.tscn`
> therefore never surface in the running app; runtime logs showed
> `Node not found: "LeftPanel/LayerBox/Layer0Btn" (relative to .../MapEditor)`.
> The LayerBox + CheckButtons had to be added to `main.tscn` as well. Rule of
> thumb: for any MapEditor UI change, mirror it in BOTH scenes or the app (main.tscn)
> and the headless/standalone regression (map_editor.tscn) drift apart.
>
> **SHIPPED (2026-09-19, Import image -> editable terrain tiles, ref #76):**
> Reached on the map: layer_2.png imported -> 1621 cells painted (1507 wall, 136
> dirt, 41 grass, 11 stone, 285 air). `map_editor.gd`: `ImgMapBtn` in BottomBar
> (mirrored in both scenes), `_on_img_map_import` FileDialog, `_import_image_to_tiles`
> (load -> scale to grid px -> per-cell RGBA average -> `_nearest_terrain_key` by
> squared-RGB distance, air wins -> "" empty, threshold 0.25), writes `_tiles`
> directly + `_mark_dirty` + redraw (serialized by normal flow). Test
> `test_image_to_map.gd` (9 assertions) + boot regression green. ALSO FIXED in the
> same pass, filed as bugs: #77 duplicate `var _selected_layer` (parse error, broke
> every scene/test) and #78 `HSlider.disabled` -> `editable` (Range prop) in
> `_sync_layer_panel`. Both pre-existing from commit fff4c7c (ref #68) — the earlier
> "headless boot clean" verification never exercised the layer panel sync path.

> IN-FLIGHT (2026-08-27, after #64 CLOSED): terrain brush + sub_tile_mask float

> COMMITTED (this session): rewrote every broken wiki cross-reference to GitHub's
> flat-slug form. Root cause (empirically verified on the live wiki before touching
> anything): GitHub wiki **flattens** subdirectory pages — a page stored at
> `TechnicalDesign/TDD_Map-World.md` renders at `/wiki/TDD_Map-World` (confirmed
> 200), NOT at `/wiki/TechnicalDesign/TDD_Map-World` (confirmed 404), and any link
> written with a directory prefix (`TechnicalDesign/...`, `GameDesign/...`,
> `../TechnicalDesign/...`, `./Foo.md`) or a `.md` extension is left verbatim by
> GitHub's renderer and 404s / redirects to Home. The ONLY working link form is a
> bare flat slug like `](TDD_Map-World)`. Image/puml refs to nested repo paths only
> render via absolute `https://raw.githubusercontent.com/wiki/HidekiAI/...` URLs.
>
> **What changed (222 rewrites across 35 wiki pages):**
> - Stripped `TechnicalDesign/`/`GameDesign/`/`../`/`./` prefixes and `.md`
>   extensions from all page links -> flat slug. Post-fix audit: ZERO remaining
>   prefixed/.md link forms, and every flat link target resolves to an existing
>   wiki page.
> - Nested image/puml refs (`TechnicalDesign/images/*.png|.puml`,
>   `images/tdd-*`, `GameDesign/assets/*`) -> absolute `raw.githubusercontent.com/wiki`
>   URLs (files stay where they are; raw URLs verified working for nested paths).
> - Cross-repo links fixed: `../../editor/scripts/{terrain_editor,tile_grid_display}.gd`
>   -> `https://github.com/HidekiAI/sidescroll-towerdefense/blob/trunk/...`;
>   `../docs/TDD_Tile-Art-Pipeline.md` -> `TDD_Tile-Art-Pipeline` (the real wiki page);
>   dead `Tower Defense Side-Scroller - Design Doc.md` -> `GDD_Executive-Summary`.
> - Main repo also contained the broken pattern: `docs/PLAN-2026-08-27-grpc-editor-control.md`
>   (wiki URL with `/TechnicalDesign/` prefix), `editor/README.md`,
>   `editor/tests/README.md` (2 wiki URLs with `/TechnicalDesign/` prefix) — fixed.
>
> NEXT (cold resume): issue #8 CLOSING comment cites the wiki-path audit on
> `sidescroll-towerdefense.wiki` and the fix commit; #8 closed this pass. After
> that, #7 (element rarity FK references — verify `rarity_id` integrity vs enum
> tables) and #24/#25 (stale-code enum mismatches, see issues) are the remaining
> labeled bug/fix issues, then the parallax BG feature request recorded in
> `.opencode/AGENTS.md` (layered/depth-based parallax, not scanline).

## Bug-Squash Session (2026-09-05) — dialog lifecycle + HUD anchor + issue alignment

> COMMITTED (this session): dialog `queue_free()` lifecycle fix for #45 plus #33
> HUD anchoring restore in `main.tscn`. See "Bug-fix record" below. All editor
> regression tests green (`failures=0`), headless boot clean, all 4 edited
> scripts pass `--check-only`. Issues closed in the SAME pass: #45, #46, #33,
> #66, #43 (see per-issue closing comments for commit refs).
>
> **Bug-fix record**
> - #45 (dashboard: "screen dialogs stay open and aren't dismissed"): root cause
>   was missing `queue_free()` on EVERY code-created dialog. 17 dialog sites
>   across `map_editor.gd`, `placement_editor.gd`, `terrain_editor.gd`,
>   `entity_editor.gd` created `ConfirmationDialog`/`AcceptDialog`/`FileDialog`
>   via `.new()` + `add_child()` + `popup_centered()` and never freed them on
>   ANY exit path (confirmed/canceled/close_requested/file_selected) — the
>   Window nodes accumulated in the scene tree and lingered. Fixed by wiring
>   `queue_free()` into each `file_selected`/`confirmed` handler and connecting
>   `canceled` + `close_requested`. The minimap picker dialog
>   (`screen_minimap_dialog.tscn`, exclusive=true) already freed itself via
>   `position_picked`/Cancel — untouched.
> - Regression test `_test_dialog_lifecycle` (issue #45) added to
>   `editor/tests/test_screen_store.gd`: asserts no orphan Windows at editor
>   start, delete confirm opens exactly one, and confirm/cancel/close_requested
>   each free it. Green.
> - #33 (HUD overlap half): the `@onready` node-path half and ScrollContainer
>   single-child half were already fixed in `904467f`; the REMAINING defect was
>   `main.tscn` HUD Labels for MapEditor + PlacementEditor carrying only
>   `top_level=true` + `layout_mode=2` with NO bottom-right anchors/offsets (the
>   standalone `map_editor.tscn`/`placement_editor.tscn` HUDs have
>   `anchors_preset=3`, anchors 1.0, offsets (-430,-120,-12,-12),
>   `mouse_filter=2`). When toggled visible, the inline main.tscn HUDs rendered
>   at top-left overlapping the toolbars. Restored the full anchor/offset/mouse-
>   filter set into both main.tscn HUDs (commit 904467f claimed this but the diff
>   replaced anchors with bare top_level).
> - #46 was already fixed in `33338ba` (dedupe stamp brushes by key +
>   `_find_tile_set`) with regression test `_test_stamp_brush_dedupe` — issue had
>   simply not been closed. Closed with commit ref.
> - #66 (terrain paint palette) was already shipped + committed `98d8d8c` — issue
>   had not been closed. Closed with commit ref.
> - #43 (saved screens self-contained) was already shipped via World Archive
>   commits `d03023e`/`f3e93b8` (documented on wiki TDD_Saved-World) — issue had
>   not been closed. Closed with commit ref.
>
> NEXT (cold resume): none pending in this pass — all 5 issues closed.

> IN-FLIGHT (2026-08-27, after #64 CLOSED): terrain brush + sub_tile_mask float
> bug. Plan: `docs/PLAN-2026-08-27-terrain-brush-and-subtile-float.md`.
>
> COMMITTED: `b020b86` = brush feature + sub_tile_mask producer/serde fix (cargo
> workspace 132 pass; editor/tests/test_terrain_brush.gd 14 assert PASS).
> `e63fbf6` = PALETTE INVISIBLE FIX + visibility logging.
> `adb65aa` = checkpoint.
>
> PALETTE BUG (root cause, journal-proven): tile_palette populated 732 entries
> (7 terrain/722 tile/3 group), visible=true, but rendered size=(0,0) — the
> ItemList had no custom_minimum_size and collapsed to zero in the LeftPanel VBox.
> Fix: map_editor.tscn TilePalette custom_minimum_size=(0,160). Verified via
> direct scene probe: custom_min=(0,160), size=(565,160), visible=true.
> tile_palette.gd now logs populate counts + a deferred layout probe.
>
> NOTE on the "4 tile flicker / quadrant toggle": that is the COLLISION-PAINT
> mode ("Collision Map" button -> Paint Direct), a SEPARATE feature from terrain
> painting that XOR-toggles a tile's sub_tile_mask bitfield (0x0..0xF quadrants).
> Exit with Esc. Not part of the terrain brush.
>
> CONFIRMED SAFE: deleting editor/world.zip is fine — editor boots fresh with a
> default 1980-tile grid, logs "Last map no longer exists, skipping" (main.gd:262),
> no crash.
>
> RESUME VIA: `.opencode/sessions/terrain-brush-subtile-palette-fix.md` (the current
> session handoff with full task detail).

T-1 [DOING] Regenerate world.zip with integer sub_tile_mask — float masks persist
    because _serialize() coercion is not on the load->save path (details below).
T-2 [TODO] Live GUI verify wheel brush + palette (needs awake DISPLAY=:0).
T-3 [TODO] Reconcile PLAN doc to shipped state.
T-4 [TODO] Optionally file issue for the float mask load->save data-integrity gap.

Detailed task notes follow after the git-history section below.


> Purpose: resume cold after a session switch or an abandoned session (e.g.
> model change -> brand-new session with zero prior context). Every section must
> be self-contained for that reader: committed history, exact in-flight step,
> next move in runnable order, all commands/numbers. Updated continuously, not
> only at the end. See `.opencode/AGENTS.md` "Session Progress Checkpoint
> (permanent)".

Last updated: 2026-08-27. #64 gRPC editor control DONE + **CLOSED** (Phase 1
`4e40cae`, Phase 2 `b6cbdaa`, wiki `59f17ab`, live all-tab wire verification
passed on DISPLAY=:0). #67 entity/terrain def overrides shipped `9d78881` with
regression test `e1d18af` (real world.zip merge, 11 assertions green) — still
OPEN pending wiki TDD + close. #61 prune crash FIXED `6103ba4`. #62 dialog
filter re-fixed `6103ba4`. See "Active step" + "Next move" for resume state.

## Objective

Package the editor as an installer (issue #54). First milestone M1: make the
runtime read-only-friendly by relocating every `res://` write to
`user://data`, so a packaged build with read-only `res://` (export/pck) works.
M2 (export/pck/.so/AppImage/deb/CI) is deferred.

## Where we are

### Done and committed (git history)
- Save/load bug batch #47/#48/#50: closed, committed `dfa9ca8`. Regression
  fixtures live in `editor/tests/` (`_test_placement_editor_tile_bank_sync`,
  `_test_placement_records_last_map_path`, `repro_blank_world.gd`).
- #55 config-backed defaults (main.gd seed/getters `defaults.*`, map/placement
  dialogs, fake_bridge.gd, `_test_config_defaults_seed`): committed `9f33983`,
  left open for review.
- #52 native `flip_of_variants` int-truncation parity: committed `28c3384`
  (bridge) + wiki TDD v7 `303c1b7`. Discovery: the [tol, tol+1) acceptance
  window is UNREACHABLE for `flip_of_variants` because `_diff_capped`
  early-exits to `tol+1.0`. Bridge suite 16/16, workspace cargo 103/103.
- #53 v8 native scan front-end: committed (code: `scan_signatures`/
  `scan_projections` native f64, `_prune_bytes`, `_exact_matches_bridge_stream`,
  a3/finalize reuse, `_canonical_coarse_bytes`, parity tests; wiki TDD v8
  `c312f7c`). Real catalog full `_build_prune_plan` 6507 -> **4551 ms**,
  `dup_keys=982` identical, cargo 105/105.
- Wiki `TDD_gRPC-Architecture.md`: added "EditorRemote — future consideration
  (NOT implemented)" (embedded EditorService gRPC for GUI-feature E2E), commit
  `cdc2396`. Tracked as deferred feature issue #58.
- Editor E2E harness issue #56 created (Playwright, browserless-by-construction,
  blocked on #40 documented contract). Not started.
- #59 native bulk canonical-coarse: committed `b8c357e` (a3_discover
  1671->873 ms; dup_keys=1328 consistent; cargo 107 pass; parity 0 mismatches).
- #60 dirty-save prompt: committed `aa3bc6f` (`_confirm_save_dirty` Save/Discard/
  Cancel modal, quit + import + new/open gated, paint/entity-ops mark dirty).

### In-flight — #54 M1 (IMPLEMENTED, suite green, NOT yet committed)
- `editor/scripts/main.gd`: `USER_DATA_DIR := "user://data"`,
  `DEFAULT_WORLD_PATH := "user://data/world.json"`, `user_data_dir()` helper
  (globalized + mkdir), `_init_config_db` writes config DB under
  `user://data/config/sstd_config.sqlite3`. Config keys unchanged:
  `defaults.world_json_path`, `defaults.world_file_name`, `defaults.file_dialog_dir`.
- `editor/scripts/map_editor.gd`: `_tiles_write_dir()` (user://data/tiles,
  creates dir), `get_tile_image` reads user dir first then built-in res://
  fallback, `_register_tile_image` writes PNG to user dir, `_load_stamp_catalog`
  precedence [user dir, res://], prune delete/est sites use `_tiles_write_dir()`.
- `editor/tests/test_screen_store.gd`: added `_test_user_data_relocation`
  (config DB under user://data/config, stamp PNG under user://data/tiles, no
  res:// write); added `_wipe_tile_artifact(key)` helper wiping BOTH
  user://data/tiles and res://assets/tiles copies; updated stale `res://world.json`
  expectations in `_test_config_defaults_seed` to `user://data/world.json`;
  cleaned up `_test_world_reopen_not_blank`/`_test_placement_editor_tile_bank_sync`
  (world restore persists the user:// PNG, so cleanups must wipe both).
- **Flake fixed (root cause)**: world `_load_world_package` ->
  `_register_tile_image` writes a user://data/tiles PNG that older cleanups never
  removed; a later suite run's fresh editor picked it up via `_load_stamp_catalog`
  and "private tile absent from disk/catalog before load" failed. `_wipe_tile_artifact`
  now clears both locations.
- **Persistence matrix (documented, wiki TDD_World-Editor.md, UNCOMMITTED)**:
  | Path | Value | SQLite-configurable? |
  |---|---|---|
  | ConfigStore DB | `user://data/config/sstd_config.sqlite3` | No |
  | Boot world pointer | `defaults.world_json_path` = `user://data/world.json` | Yes |
  | Save-dialog file name | `defaults.world_file_name` = `world.zip` | Yes |
  | Save/load dialog start dir | `defaults.file_dialog_dir` = `""` | Yes |
  | Tile/stamp cache | `user://data/tiles` | No |
  Data base dir override: tracked as #57 (CLI-arg-only, future/deferred).
- Verification: full GDScript suite `=== done, failures=0 ===`; syntax check
  passes on `tests/test_screen_store.gd`; no Rust changes this round (`.so` not
  rebuilt).

### Design decision (recorded 2026-08-17): Simulator tab is pure gRPC
- Simulator state/stepping live entirely in a Rust process (`sstd-headless`,
  per TDD_gRPC-Architecture.md). Port 50052, chunky round-trips. Not the
  godot-rust FFI hot-loop tier.
- **Caveat (must decide when implementing)**: Godot `HTTPClient` is HTTP/1.1 and
  cannot speak gRPC (HTTP/2). The Godot tab needs a hop: (a) `sstd-editor-bridge`
  hosts a tonic CLIENT, or (b) `sstd-headless` exposes a second
  JSON-over-unix-socket/WebSocket face. Consistent with #40's "transport-agnostic,
  gRPC is one face".
- Status: design decision only. Current crates: `sstd-core` + `sstd-editor-bridge`.
  NO simulator crate, NO `sstd-headless` binary yet. Simulator tab exists as
  `simulator.tscn` + `editor/scripts/simulator.gd` (stub).

### Design consideration (recorded 2026-08-18): EditorRemote (deferred, #58)
- If remote E2E of the Godot GUI editor (map/tile/terrain/editor) were wanted:
  an EditorService gRPC server EMBEDDED in the editor process (godot-rust bridge
  hosting tonic on the editor port) exposing editor/data operations, NOT UI
  widget automation. Off-thread tonic workers must defer RPCs onto the Godot
  main thread, apply against live state, reply via channel (1 op/frame; Result
  replies). Not via OS GUI automation nor browser/WASM (native `.so` can't load
  in WASM — see #56). Simulator stays separate (port 50052). Documented in
  wiki TDD_gRPC-Architecture.md "EditorRemote"; tracked as #58. NOT implemented.

## Active step

> **SUPERSEDED — historical record only.** The live resume point is
> [CURRENT STATE / NEXT MOVE](#current-state--next-move) at the top of this file. This
> section describes the 2026-08-29 state and has not been maintained since; figures
> below (e.g. "cargo workspace 122 pass") predate the current **110**.

1. **#61 prune crash — DONE, committed `6103ba4`, issue closed.**
   - Root cause: after first prune, `_drop_tiles()` shrinks `_stamp_catalog`
     while cached `_prune_bytes`/`_canonical_flat` kept pre-prune sizes, and
     `_stamp_fp_index` held stale indices one past the shrunken catalog. Second
     scan's `_a3_discover` fed those stale indices to `_uf_union`/
     `_finalize_prune_plan` -> `Invalid access of index 2017`.
   - Fix: `_execute_prune` clears `_prune_bytes`+`_canonical_flat` after
     `_drop_tiles`; `_a3_discover` skips `other >= _stamp_catalog.size()`;
     `_finalize_prune_plan` emits journal log of catalog/uf_parent/cache sizes.
2. **#62 dialog filter default — re-fixed `6103ba4`, reopened, needs re-close.**
   - Original `_apply_world_default_filter` wrote `FileDialog.current_filter`
     which DOES NOT EXIST in Godot 4.4 (both string and int throw "Invalid
     assignment of property"). Replaced with filter-ORDERING: `.zip` first when
     package mode, `.json` first in legacy mode. Helper removed; inlined into all
     4 dialogs (map_editor _on_save/_on_import, placement_editor _on_save/_on_import_map).
3. **#67 entity/terrain def overrides — DONE, committed `9d78881`.**
   - Rust `EntityDefOverride`/`TerrainTypeOverride` prototype-based partial
     patches (merge onto framework def, deny_unknown_fields, enum type checks),
     `TerrainOverridesFile.merge_all`/`EntityOverridesFile`, exported in lib.rs.
   - world_archive.gd writes `terrain_overrides.json`/`entity_defs.json`;
     entity_editor.gd: `set_framework_entities`/`apply_world_entity_defs`/
     `collect_world_entity_defs` (removed hardcoded `_add_defaults`).
   - editor/default_package/: framework prototypes terrain_types.json (7 types)
     + entity_defs.json (8 defs) + 7 tile PNGs.
   - cargo workspace 122 pass. Issue #67 OPEN — close on next pass with final
     GUI/world-roundtrip confirm.
4. **#64 Phase 1 headless all-tab capture — DONE, committed `4e40cae`.**
   - `editor/capture_all_tabs.gd` (SceneTree entrypoint) instantiates main.tscn,
     iterates `TAB_NAMES`, saves `res://captures/{tile,entity,map,placement,simulator}.png`.
   - Verified 5 valid 1152x648 PNGs against `DISPLAY=:0` (Dummy renderer headless
     cannot capture — null viewport texture).

5. **#64 Phase 2 gRPC editor control — IMPLEMENTED (not yet committed).**
   - NEW `crates/sstd-grpc`: `proto/editor.proto` (SwitchTab/CaptureScreenshot,
     EditorService), `build.rs` (tonic-build + prost-build with VENDORED protoc
     via `protobuf-src` because no system protoc on host), `lib.rs` exposing
     `EditorServer` (tonic), `EditorCommand` (mpsc + oneshot bridge),
     `spawn_server(addr) -> mpsc::Receiver`, `TAB_NAMES`, `resolve_tab_index`,
     `SwitchTabResult`/`ScreenshotResult`. `#[tokio::test]` unit + 2 full-wire
     e2e tests.
   - Bridge `crates/sstd-editor-bridge/src/lib.rs` (new fields `grpc_tx?`,
     `grpc_rx?`, `tab_container: Option<Gd<TabContainer>>`; 5 `#[func]`s):
     `set_tab_container`, `switch_tab`, `capture_screenshot`, `start_grpc_server`,
     `poll_grpc_commands`. Key APIs: `Gd::upcast::<Texture2D>()` +
     `Texture2D::get_image()` + `Image::save_png_to_buffer()` (returns
     `PackedByteArray`, not Option); capture resolves from `tab_container`'s
     viewport (NOT `self.base` — godot-rust 0.5 `Base` has no Deref/get_viewport).
     `poll_grpc_commands` takes each command OUT of the channel before `&mut self`
     calls to satisfy the borrow checker. Adds deps: `sstd-grpc`, `tokio`(sync),
     `base64`.
   - `editor/scripts/main.gd`: `GRPC_ENABLED=true`, `GRPC_PORT=50051`,
     `_start_grpc_server_if_enabled()` calls `set_tab_container` +
     `start_grpc_server`; new `_process(delta)` polls `poll_grpc_commands`.
   - Verified: cargo workspace 128 pass (102 core + 20 bridge + 4 grpc unit +
     2 grpc e2e). Headless smoke: bridge loads, `[sstd-bridge] grpc: tab_container
     registered`, `start_grpc_server(50051) -> {"ok": true, "port": 50051}`,
     exit 0.
   - COMMITTED `b6cbdaa` (amended with `crates/sstd-grpc/examples/grpc_smoke.rs`,
     a live smoke client: `cargo run -p sstd-grpc --example grpc_smoke [port] [tab]`).
   - LIVE VERIFICATION DONE + **#64 CLOSED** (2026-08-27). With DISPLAY=:0 woken,
     editor launched, server confirmed listening on 127.0.0.1:50051. Drove all 5
     tabs over the wire: each switch ok=true (idx 0-4), each capture a valid
     non-empty PNG at 1152x648 (tile 46KB, entity 51KB, map 575KB, placement
     762KB, simulator 11KB). Tabs visibly switched in the live editor. Verified
     per wiki TDD_GRPC-Editor-Control. Close comment posted, issue #64 CLOSED.
     NOTE: live verification needs a real display; from a locked/screensaver
     session Godot reports "X11 Display is not available" — wake the desktop
     first (Xvfb crashes on this host, headless/Dummy cannot capture).

## Next move (proposed order)

> **SUPERSEDED — historical record only.** The live resume point is
> [CURRENT STATE / NEXT MOVE](#current-state--next-move) at the top of this file.
> Item 1 below was completed and is retained for the record.

1. ~~**#67 — author wiki TDD + close.**~~ **DONE.** #67 is CLOSED; the page shipped as
   `TDD_Def-Overrides` (see TODO row TS57). Code shipped `9d78881`; the wiki page and
   closing comment citing it both exist.
2. **#62 — GUI preselect re-verify then close.** Re-verify functionally that Load
   defaults to `.zip` when the world package is active (the `current_filter`
   property DOES NOT EXIST in Godot 4.4; the fix uses filter-ORDERING instead).
   Needs a real-display session to click through. Then close #62. **Still open — see
   the live block above.**
3. (Optional) Add a grpcurl CI loop for the TabNames x CaptureScreenshot flow
   referenced in the TDD; currently verified via the Rust smoke client instead.

## Key numbers / constants
- `STAMP_CELL=32`, `TILE_BYTES=4096`, `STAMP_TOLERANCE=4.0`.
- Rust bridge constants: `COARSE_BYTES = 8*8*4` (256), `STAMP_CELL=32`.
- `fp_hash` = FNV-1a 32-bit. Golden values: empty=2166136261,
  `[0x00,0x01,0x02,0x03]`=3282719153, `[0xff;8]`=1823345245.
  a3 neighbor hashes over all-zero/all-ff canon: 257 distinct = COARSE_BYTES+1.
- Gate: `_grayscale_sig_near` over 256-byte sigs, diff <= 96.
- `BRIDGE_MIN_PAIRS=512`, `BRIDGE_MAX_WORKERS=32` (host has 32 cores).
- A3: index keyed on 8-bit `_canonical_coarse` (256 bytes), neighbour delta +/-1,
  `A3_K=8`, `A3_LRU_CAP=64`, <=512 neighbour hashes.
- Deep tier: `DEEP_CONFIRM_THRESHOLD=2500`, `DEEP_YIELD_EVERY=500`.
- GDScript floats are 64-bit doubles (probe-verified:
  `0.299*16+0.587*16+0.114*16` = `15.99999999999999822` -> int 15). Bridge
  ports MUST use f64; an f32 port swung boundary sums by +1. Recorded in wiki
  TDD v8.

## Commands
- Editor regression tests:
  `~/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd`
  -> expect `=== done, failures=0 ===`.
- Script syntax check:
  `~/bin/godot4 --headless --path editor --check-only --script res://scripts/map_editor.gd`
  (also for `res://tests/test_screen_store.gd`).
- Cargo: `cargo test` (workspace; 105 pass at #53).
- Bridge rebuild (RELEASE — debug is 3-10x slower):
  `cargo build --release -p sstd-editor-bridge` then
  `cp target/release/libsstd_editor_bridge.so editor/rust/`.
- Real-catalog breakdown:
  `~/bin/godot4 --headless --path editor --script res://tests/profile_real.gd`
- Repos: trunk `sidescroll-towerdefense`, wiki `sidescroll-towerdefense.wiki`.
  `main.tscn` embeds MapEditor inline — edit both scenes.