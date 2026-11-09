# PLAN 2026-11-09 — #122 slope set: regression guard + wiki correction

Ref: [Issue #122](https://github.com/HidekiAI/sidescroll-towerdefense/issues/122) (hill terrain, 4:7 slope set).
Code repo branch: `test/122-slope-set`. Wiki repo branch: `docs/122-slope-set`.
Wiki page of record: `TechnicalDesign/TDD_Simulator-Ground-Walk.md`.

This file IS the state. A step reads this file, executes its own section, and appends its
result before stopping. Never rely on conversation history: the previous agent may have been a
different model, on a different provider, five hours ago.

## Why this plan exists

The #122 hill-terrain code is committed on `trunk` (7x4 slope set, `slope_up_0`..`slope_up_6`,
per-column injection in `main.gd`). Two things are missing:

1. **No test anywhere covers the slope set.** `grep -n slope_up editor/tests/*.gd` returns
   nothing. The injection formula and the ground profile it produces are unguarded.
2. **The wiki page is wrong or silent.** Its one draft section (uncommitted, 22 lines) lists the
   wrong terrain keys (`hill_up` instead of `slope_up_<col>`), duplicates heading `## 7`, and
   places a scope line below `## 8 Related`.

Three facts measured on 2026-11-09 that a later step must not re-derive:

- The Simulator ground layer **never loads a PNG**. `editor/scripts/world_view_ground.gd` draws
  one polygon per contiguous slope run plus rects for flat walkable tiles. The
  `{key}_32x32.png` lookup exists only in `tile_grid_display.gd`, `map_editor.gd` and
  `placement_editor.gd` (palette and editor grid views). A resume prompt claiming a PNG
  fallback path describes code that does not exist.
- Walkability in the Simulator is a heuristic, not a terrain-table lookup:
  `editor/scripts/world_view.gd:633-635` marks everything walkable except `air`, `wall`, `lava`.
  Slope keys are therefore walkable by not being one of those three.
- `hill_up` and `hill_down` are legacy keys: still in `CANONICAL_TERRAIN`
  (`editor/scripts/world_archive.gd:13`) and used by `editor/tests/test_walker_visibility.gd`,
  but no PNG exists for them and `inject_simulator_hills` no longer emits them.

## Standing constraints (obeyed by every step)

- Godot: `$HOME/bin/godot4` (4.7.2.stable.mono). Rust: `cargo`.
- Gate on an **exit code**, never on a printed `failures=0` line.
- Known-red, claimed but NOT reproduced: `test_screen_store` (#85, `world-shared tile stored
  exactly once`) was recorded as red on 4.7.2. Measured 2026-11-09 on this host it exits **0**;
  see `Step 1 result`. Treat the exit code as the fact, not the label.
- No Python on this host. `jq`, Rust or GDScript instead.
- No emoji, emoticons or decorative icons in any file, commit message or comment.
- `git add` by **explicit path** only. Never `git add -A` / `git add .`: `editor/addons/` is
  516 MB of vendored addon.
- Never commit on `trunk` or `master`. Every change rides a branch, merged with
  `git merge --ff-only`; if ff refuses, STOP and ask.
- Push by explicit remote name, to every remote that already has the branch.
- If a Godot run rewrites `editor/project.godot`, `git checkout -- editor/project.godot`.
- Wiki pages cite **issue numbers, never git hashes**.
- Numbers are recorded as measured, never predicted.
- A step that finds something outside its scope records it here and continues. It never widens
  scope by fixing it.

## STEP LEDGER

| Step | Objective | Status | Evidence | Fingerprints |
|---|---|---|---|---|
| S0 | Create this ledger, branch, checkpoint pointer | done | commit below, `Step 0 result` | branch `test/122-slope-set` |
| S1 | Measure the gate and issue #122 state | done | all six suites rc=0, boot rc=0, 141 Rust tests pass, #122 OPEN | plan `Step 1 result` |
| S2 | Write + mutation-verify `test_slope_set.gd` | done | green rc=0; 3 mutants each rc=1 | `Step 2 result` |
| S3 | Commit the code repo | todo | | |
| S4 | Rewrite the wiki page section | todo | | |
| S5 | Index the page in Home.md + TODO.md | todo | | |
| S6 | Commit the wiki repo | todo | | |
| S7 | Publish both repos, settle issue #122 | todo | | |
| S8 | Final checkpoint state | todo | | |

## Step specifications

### S1 — Measure the gate and the issue

**Read first:** this file, `## Why this plan exists`; `editor/tests/README.md` (exit-code rule).
**Do**, inside `sidescroll-towerdefense/`:

```bash
$HOME/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_image_to_map.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_terrain_brush.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_override_merge.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_walker_step.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_walker_visibility.gd
$HOME/bin/godot4 --headless --path editor --quit-after 120
cargo test -p sstd-core
gh issue view 122 --json number,title,state,body
git checkout -- editor/project.godot   # only if the Godot run rewrote it
```

**Expected:** `test_screen_store` exit 1 (#85), everything else exit 0, `cargo` green.
The checkpoint previously claimed all six suites exit 0 on this host, which contradicts #85 —
record which claim the measurement supports and correct the checkpoint in S3.
**Done when:** the S1 result block lists nine measured outcomes (seven exit codes, the cargo
summary line, the issue state).
**STOP if:** any suite other than `test_screen_store` is red. Record the failure verbatim and
stop; a regression is a different ticket, not a silent fix here.

### S2 — Write and mutation-verify the slope test

**Read first:** S1 result; `editor/tests/test_walker_visibility.gd` lines 1-20 (harness shape:
`extends SceneTree`, `_initialize` defers `_run`, `_check`, `quit(0 if _failures == 0 else 1)`);
`editor/scripts/main.gd` lines 217-254; `editor/scripts/world_view.gd` lines 610-641.
**Do:** create `editor/tests/test_slope_set.gd` with three groups:

1. **Injection contract.** `load("res://scripts/main.gd").new()`, set `_grid_config` to
   `{"max_tiles_per_screen_x":60, "max_tiles_per_screen_y":33, "tile_width_in_pixels":32,
   "tile_height_in_pixels":32}`, fresh `ScreenStore.new()`, call `inject_simulator_hills`.
   - Screen `"1,0"`: 60 tiles. Columns 0-6 carry `slope_up_0`..`slope_up_6` at rows
     `31,31,30,30,29,29,28`. Columns 7-59 are `dirt` at row 31.
   - Screen `"2,0"`: columns 14-20 carry the same seven keys at the same rows. Every other
     column is `dirt` at row 31.
2. **Back-to-back repeat.** Overwrite one cached tile, call the function again, assert the cache
   is unchanged (the `is_occupied` early return must not clobber, and must not half-apply).
3. **Ground profile.** Instantiate `res://scenes/world_view.tscn`, wire the same store and grid
   config, assert `_ground_y_by_col` over the seven slope columns is
   `992, 992, 960, 960, 928, 928, 896` (row * 32).

Header comment must state that `_draw()` polygon pixels are NOT covered, because
`draw_polygon` cannot be intercepted from GDScript and intercepting it would mean refactoring
production code for a test.

**Then mutation-verify**, reverting each mutant before the next and re-running green at the end:
- divisor `7` -> `8` in `main.gd`'s `int(4 * col / 7)`;
- column index -> constant `0` in the injected key (`"slope_up_" + str(i)`);
- `slope_up_` -> `hill_up` in the injected key.

**Done when:** the suite exits 0 unmutated, and the S2 result block names each mutant, the
assertion it turned red, and the revert command used.
**STOP if:** any mutant leaves the suite green (the assertion is vacuous) or the mutation itself
does not parse (a void experiment, not a result).

### S3 — Commit the code repo

**Read first:** S1 and S2 results.
**Do:** `git add` by explicit path: `editor/tests/test_slope_set.gd`, its generated `.uid`,
`editor/tests/test_walker_step.gd.uid`, `editor/tests/test_walker_visibility.gd.uid` (both were
untracked; 50 `.uid` files are already tracked and nothing ignores them), plus this plan file and
`docs/SESSION-CHECKPOINT.md`. Commit
`test(sim): pin the 4:7 slope set injection and ground profile (#122)`.
**Done when:** `git status --short --branch` is clean on `test/122-slope-set`, and S3 records the
short hash.

### S4 — Rewrite the wiki page section

**Read first:** S3 result; `TechnicalDesign/TDD_Simulator-Ground-Walk.md` in full. Create the
wiki branch `docs/122-slope-set` first.
**Do:** one file, these edits only:
- Line 5: remove `Commits 09f3db5 and 54f0670 on local/fix/walker-framing`; keep the #122 link.
- Line 9: replace "no slopes" with whole-tile slopes drawn as one ramp polygon per contiguous run.
- Replace the drafted `## 7. Slope Sets` block; renumber `Out of Scope` to 8, `Related` to 9.
- The section states: 7x4 set, run 7 columns, rise 4 rows, indivisible, flush on flat ground at
  both ends; the key/offset table (col 0 `slope_up_0` offset 0, col 1 `slope_up_1` offset 0,
  col 2 `slope_up_2` offset 1, col 3 `slope_up_3` offset 1, col 4 `slope_up_4` offset 2,
  col 5 `slope_up_5` offset 2, col 6 `slope_up_6` offset 3, col 7+ `dirt` at ground);
  formula `y = ground_row - int(4 * col / 7)` with `ground_row = max_tiles_per_screen_y - 2 = 31`
  of 33; injection sites screen `(1,0)` cols 0-6 and screen `(2,0)` cols 14-20; rendering as the
  code is (polygon per run in `world_view_ground.gd`; `{key}_32x32.png` only in palette and
  editor grids; walkability = everything except `air`/`wall`/`lava`); collision stays whole-tile
  so the walker climbs in 1-row steps over a continuous ramp; `hill_up`/`hill_down` are legacy
  keys still in `CANONICAL_TERRAIN`, no PNG, not emitted.
- Keep the 30°-approximation and adjacency paragraphs. Move the orphaned
  "In scope (added 2026-11-08)" line into section 1.
**Done when:** `git diff --stat` shows exactly one file, and S4 records `grep -n '^## '` output.
**STOP if:** the drafted 22 lines disappear without a replacement section (stale read; redo).

### S5 — Index the page

**Read first:** S4 result; `Home.md` line 109; `TODO.md` rows TS75-TS76 for format.
**Do:** extend the existing Home.md row description with the 4:7 slope set. Add one `TS77` row
for #122 (what landed, status, links to the page and the issue). Update the `*Last updated*` line.
**Done when:** `grep -n "TS77" TODO.md` and `grep -n "Slope" Home.md` both hit, and S5 records
both lines.

### S6 — Commit the wiki repo

**Read first:** S5 result.
**Do:** `git add` the three files by explicit path; commit
`docs(sim): record the 4:7 slope set and how the ground layer draws it (#122)`.
**Done when:** `git status --short --branch` clean on `docs/122-slope-set`, and S6 records the
short hash.

### S7 — Publish both repos and settle the issue

**Read first:** S3 and S6 results (both hashes), S1's issue state.
**Do:** ask once for push permission, then per repo: push the branch by explicit remote name,
`git merge --ff-only` into `trunk` / `master`, push the base branch to every remote. Post the
#122 closing comment citing `TDD_Simulator-Ground-Walk` section 7 and `test_slope_set.gd`, then
close the issue only if its acceptance criteria match what shipped; otherwise leave it open and
name the gap in the comment.
**Done when:** S7 records both merged ref ranges and the issue's final state.

### S8 — Final checkpoint state

**Read first:** S7 result.
**Do:** on a one-commit branch `docs/122-final`, set the checkpoint's current-state block to
"nothing in flight", naming the merged wiki page and the issue state. Merge it the same way.
**Done when:** the checkpoint's first block says nothing is in flight and S8 is `done` in the
ledger.

## Step 0 result

- Branch created: `test/122-slope-set` (from `trunk` at `075b88e`, clean).
- This ledger created.
- `docs/SESSION-CHECKPOINT.md` current-state block rewritten to point here with
  `Current step: S1`.
- Untouched on purpose: the two untracked `editor/tests/*.gd.uid` files (S3 commits them), and
  `editor/project.godot` (only touched if a Godot run rewrites it).
- Commit subject: `docs(sim): open the #122 slope-set docs-and-guard plan (#122)`.
- Commit short hash, recorded here by S1 because S0 cannot know its own: `63214fa`.

## Step 1 result

Measured 2026-11-09 on `$HOME/bin/godot4` 4.7.2.stable.mono, `--path editor`, exit codes captured
per run (no pipeline, so `$?` is the suite's own status).

| Run | Exit | Last line |
|---|---|---|
| `test_screen_store.gd` | 0 | `6 resources still in use at exit` (ERROR, not a failure) |
| `test_image_to_map.gd` | 0 | draw trace of `tile_grid_display.gd` |
| `test_terrain_brush.gd` | 0 | `PASS (failures=0) (14 assertions)` |
| `test_override_merge.gd` | 0 | `override merge done, failures=0` |
| `test_walker_step.gd` | 0 | `0 failures` |
| `test_walker_visibility.gd` | 0 | `1 ObjectDB instance was leaked at exit` (WARNING) |
| headless boot `--quit-after 120` | 0 | 0 lines matching `SCRIPT ERROR` / `Parse Error` / `ERROR:` |
| `cargo test -p sstd-core` | 0 | `141 passed; 0 failed` (lib target; doc-tests 0) |
| `gh issue view 122` | 0 | `#122 OPEN - feat(sim): ground layer + avatar walk in Simulator` |

Corrections this step establishes:

1. **`test_screen_store` is GREEN here**, so the checkpoint's "all six suites exit 0" was right and
   the "#85 known red on 4.7.2" expectation is stale for this host. Did not investigate why: out of
   scope for #122. The standing constraint above was corrected to match the measurement.
2. **Rust test count is 141, not 110.** The canonical commands in the workspace AGENTS.md and the
   old resume prompt both say 110; the gate prints 141. Recorded as measured; the stale counts in
   those two documents are a separate doc fix, not part of this plan.
3. `editor/project.godot` was NOT rewritten by any run (`git status` clean for it).
4. The two untracked `editor/tests/*.gd.uid` files remain untracked, as S0 recorded; S3 commits them.

Next step: S2.

## Step 2 result

Created `editor/tests/test_slope_set.gd` (extends `SceneTree`, same harness shape as
`test_walker_visibility.gd`). Three groups:

1. **Injection contract** — `inject_simulator_hills` on a fresh store: screen `"1,0"` cols 0-6 carry
   `slope_up_0`..`slope_up_6` at rows 31,31,30,30,29,29,28 and col 7 is `dirt` at row 31; screen
   `"2,0"` cols 14-20 carry the same seven keys at the same rows, cols 13 and 21 `dirt` at row 31.
2. **Back-to-back repeat** — one cached tile overwritten with a sentinel (`dirt`/ground row), second
   call must leave it alone (the `is_occupied` early return must not clobber or half-apply).
3. **Ground profile** — `world_view.tscn` with the same store: `_ground_y_by_col` over world columns
   60-66 is 992, 992, 960, 960, 928, 928, 896, and column 67 is flat ground 992.

Run, unmutated: `slope_set failures=0`, **rc=0**.

Mutation verification, each mutant applied alone to `editor/scripts/main.gd`, run, then reverted with
`git checkout -- editor/scripts/main.gd`:

| Mutant | Edit | Assertion that went red | Run |
|---|---|---|---|
| 1 | `int(4 * i / 7)` -> `int(5 * i / 7)` (screen 2) | `1,0 col 3: row 29, expected 30` and the same for cols 5,6; ground profile too | rc=1 |
| 2 | `"slope_up_" + str(i)` -> `"slope_up_" + str(0)` (screen 2) | `1,0 col 1: terrain slope_up_0, expected slope_up_1` (cols 1-6) | rc=1 |
| 3 | `"slope_up_"` -> `"hill_up"` (both screens) | `1,0 col 0: terrain hill_up0, expected slope_up_0` (cols 0-6) | rc=1 |

After the third revert: `slope_set failures=0`, rc=0, and `git status --short
editor/scripts/main.gd` is empty, so the production file is byte-identical to `trunk`.

Not covered on purpose (stated in the test's header): the ramp polygon pixels inside
`world_view_ground.gd::_draw()`. `draw_polygon` cannot be intercepted from GDScript, so covering it
would mean reshaping production code for the test. The surface table asserted here is what the
polygon's top edge is drawn from, so a wrong slope still surfaces as a wrong surface.

Note for S3: `test_slope_set.gd` did not get a `.uid` sibling from a `--script` run (only editor
imports generate one), so S3 stages the two pre-existing walker `.uid` files and this `.gd` only.

Next step: S3.
