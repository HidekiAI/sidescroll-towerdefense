# Task prompt — replace camera-driven parallax scrolling with per-layer `scroll_offset`

> Start a NEW session in `sidescroll-towerdefense/` and send it this file's contents as
> the opening message. Everything below is written to be self-contained: a session that has
> never seen this conversation should be able to work from it alone.

---

You are continuing work on the SSTD Godot editor's parallax backdrop. Load the
**`coding-assistant`** skill and work through it with me. I write the logic; you scaffold,
advise and review. Nothing is implemented live by you unless I say so for a specific block.

Start at **Phase 1 (new work)** — no code for this task exists yet. Do its **Plan Gate**
and **Scope Check** before scaffolding anything.

## Where things are

- Workspace: `/home/hidekiai/projects/SSTD` (holds two repos, this is a workspace, not a git
  repo)
- Code repo: `/home/hidekiai/projects/SSTD/sidescroll-towerdefense`, default branch
  `trunk`
- **Work on the branch `feat/parallax-restack`**, which is already checked out. Never commit
  to `trunk`.
- Wiki repo `sidescroll-towerdefense.wiki` is **FROZEN** — do not touch it.
- Resume point of record: `docs/SESSION-CHECKPOINT.md`, section
  `## CURRENT STATE / NEXT MOVE`. Read it first; it is current as of 2026-10-03.
- Tickets: **#89** (the bug, full diagnosis), #68 (the feature this belongs to), #88
  (per-band stagger, related but **separate scope** — do not fold it in unless #89's
  Expected Result requires it).

## The files involved

```
editor/scenes/main.tscn               both bars, declared; z_index = 20
editor/scenes/backdrop_preview.tscn  6 Parallax2D layers + Camera2D + Backdrops
editor/scripts/simulator.gd           wires scrollbar value -> camera.position
editor/tests/test_parallax_backdrop.gd   existing parallax contract
editor/assets/backdrop_layers/manifest.json   single source of truth for the layers
```

## What is broken (measured, 2026-10-03)

`BackdropStrip`'s `Camera2D` sits in the **same canvas layer as the entire editor UI**. In
Godot a `Camera2D` transforms its whole canvas layer, so it was displacing every `Control`
— tab bar, all editors, both scrollbars — by half the viewport.

Measured with a real window at 1920x1080 (frame 1920x1029):

```
canvas_transform      = translate(960, 483.5)
UI pixel bbox         = (960, 483) size (960, 546)     <- UI crammed into one quadrant
bar_x draws at y      = 1496.5    (frame ends at 1029 -> off-screen)
bar_y draws at x      = 2864      (frame ends at 1920 -> off-screen)
bar_x pixels drawn    = 0
bar_y pixels drawn    = 0
```

The bars are anchored correctly — `bar_x` at `(0,1013)` size `1904x16`, `bar_y` at
`(1904,31)` size `16x982`. They render off-screen and draw nothing. **They have never been
visible.** They draw zero pixels at every `z_index` from 0 to 20, so layering is not the
issue: a control created in code at the same parent and same z draws 2968 px.

## Both simple fixes were measured and both fail

| candidate | UI bbox | bar_x px | bar_y px | art scrolls |
|---|---|---|---|---|
| baseline: camera on, strip inline | (960, 483) size (960, 546) | 0 | 0 | yes |
| `camera.enabled = false` | (0, 0) size (1920, 1029) | 30442 | 15690 | **no** |
| strip moved onto its own `CanvasLayer` | (960, 514) size (960, 515) | 0 | 0 | yes |

- `camera.enabled = false` restores the UI and both bars, but **kills scrolling**:
  `simulator.gd:32-39` drives the bars by writing `camera.position`, and a disabled camera
  applies no transform.
- The `CanvasLayer` move does not work at all: the camera keeps transforming the default
  canvas even when it sits on its own layer.

## The decision, already made

**Delete the camera.** No camera means no canvas transform, so the UI is never displaced.
Scroll the strip by driving each `Parallax2D.scroll_offset` directly from the scrollbar
value times that layer's own `scroll_scale`. Per-layer control is also the mechanism #88
will need, but **#88's stagger fix is not this task** — do not implement it here unless the
#89 Expected Result demands it.

"Delete" means: remove the `Camera2D` node from `backdrop_preview.tscn` and remove every
reference to it in `simulator.gd`. Do not leave it disabled as dead weight. Say so in the
commit message.

## The measurement trap that hid this — do not repeat it

`get_global_rect()` reports **anchor math only** and excludes the camera's canvas
transform. That is why the bars read as correctly placed at every window size for days
while being drawn nowhere, and why every geometry check passed.

Two checks are mandatory for anything expected to be visible, and **neither is a transform
read**:

1. intersect the **canvas-space** rect with `get_visible_rect()`;
2. hide the node, diff the rendered frame against a baseline, and count changed pixels.
   Include a positive control that is known to draw, so a zero result is distinguishable
   from a broken differ.

Never conclude a Control is visible from its rect, its anchors, or its `z_index`.

## Framing work that is VOID — re-derive it, do not trust it

All of the following were calibrated through the displaced canvas and are **wrong or
unverified**:

- `Backdrops.position = Vector2(-960, -780)` in `backdrop_preview.tscn`
- `_frame_backdrop()` runtime horizontal alignment in `simulator.gd`
- the 240px vertical travel clamp and its justification
- the conclusion that vertical travel opens a gap needing **taller art from the generator**.
  That was measured through the wrong canvas and must be re-tested before acting on it.

**This is your first job after the Scope Check:** with the camera gone, re-measure where
the art lands at two window sizes (1920x1080 and 1280x800) using hide-and-diff, and derive
the framing from those numbers. Do not carry the old constants forward on the assumption
that they were only slightly off.

## Other hard-won facts, so they are not rediscovered

- **Headless has no window.** `DisplayServer.window_get_size()` returns `(0,0)` there and
  every headless size reading is meaningless. Use `--resolution WxH` with a real window for
  any layout or visibility question.
- **Render differencing is the only ground truth for visibility.** `Parallax2D` renders
  tiled repeat copies that the authored child sprite's transform does not describe.
- Layers are authored at exactly 1920 px wide with `repeat_size = (1920, 0)`; `repeat_size_y`
  is hard-coded 0. Scrub range is 6400 px.
- `_size_layer_repeats()` in `simulator.gd` derives `repeat_times` as
  `ceil(scrub / repeat_size.x) + 1`. Left at Godot's default of 1, the horizontal scrub
  showed no art at all (100% bare at camera x=1600). The defect is real; the numbers in that
  comment were measured through the displaced canvas, so re-measure and correct the comment
  rather than leaving a measurement you know is stale.
- `editor/project.godot` is **not** committed. It drifts whenever a run does `--import`.
  Revert it after every Godot run: `git checkout -- editor/project.godot`.
- `.import` sidecars are gitignored; the six PNGs ship without theirs.
- `editor/addons/` must stay gitignored and untracked.
- **Never `git add -A` or `git add .`** — explicit paths only.
- `editor/tests/test_screen_store.gd` **exits 1 on Godot 4.7.2** for a pre-existing reason
  (#85, `ZIPPacker` now emits directory entries). Not yours. Gate on the **exit code**,
  never on a printed `failures=0` line.
- Godot is `$HOME/bin/godot4`, currently 4.7.2.stable.mono.

## Gates to run before any commit

```bash
cargo test -p gen-backdrop                       # 29 tests, generator

$HOME/bin/godot4 --headless --path editor --script res://tests/test_parallax_backdrop.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_image_to_map.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_terrain_brush.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_override_merge.gd
```

## Constraints specific to this task

- **Tests are your job, not mine.** Write them. Run them **red against the unfixed code
  first**, so we know they can fail, then again after each block. A test that passes before
  the fix proves nothing — say so plainly if one does.
- **Prove each SAMPLE compiles before handing it to me.** Uncomment it in a throwaway copy
  under `/tmp/user/1000/opencode/` and run it there. A build error means the experiment was
  void, not that the idea is wrong. Never leave a broken sample in my tree.
- **GDScript holes must be real static errors.** GDScript has no ahead-of-time compiler, so
  a hole that only fails at runtime will not show a red squiggle. Use a wrong-typed
  initializer (`var total: int = "text"`) or a non-void return with an empty body
  ("Not all code paths return a value") — both are parser errors the language server
  reports. Never `pass` with a bare `return`, never a cast.
- **Explicit types everywhere** in GDScript: no bare `var`, typed parameters, typed return
  types, typed `@onready`. Keep 4-space indent.
- **Never reference code by an opaque label.** Name the function. "Block 2 of 9" and
  "same as 6 of 9" are banned.
- **When you hand me a block, state its unit-test count**, measured, and separately state
  how many of N tests are falsifiable (can actually go red). Derive it by running, never by
  estimating.
- **I cannot see images.** Do not describe a screenshot. Measure it numerically.
- Commit in Conventional Commits form, referencing the issue number. Small commits, each one
  a coherent step, message saying what changed and why.

## Definition of done

1. The `Camera2D` node and every reference to it are gone.
2. Both scrollbars render real pixels, proven by hide-and-diff with a positive control, at
   1920x1080 and at 1280x800.
3. Horizontal scrub shows art across the full 6400 px range, at both window sizes.
4. Each layer moves at its own `scroll_scale` — the two-axis parallax is still observable.
5. Framing constants are re-derived from measurement, and the comments stating them are
   corrected to match what was actually measured.
6. All gates above green, `test_screen_store` aside.
7. Compressed on `feat/parallax-restack`, merged `--ff-only` into `trunk` only after I
   approve, then pushed. Offer to delete the branch afterwards; never delete it on your own
   initiative.