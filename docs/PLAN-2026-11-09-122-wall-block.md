# PLAN 2026-11-09 — #122 wall blocking in the walker stepper

Ref: [Issue #122](https://github.com/HidekiAI/sidescroll-towerdefense/issues/122) (ground layer +
avatar walk). Acceptance line not yet met: **"blocked by non-walkable"**.
Branch: `fix/122-wall-block`, off `trunk` at `e65cd11`.

## Why

Both steppers move horizontally with no obstruction check:

- `editor/scripts/world_view.gd:548` `step_walker` (the LIVE per-frame path, called at `:721`)
  computes `next_pos_x = clampf(pos_x + move_x * speed * dt, world_left_px, world_right_px)`.
  Only the world edges stop it.
- `crates/sstd-core/src/physics.rs:84` `step_walker` does the same, clamping to the min/max tile
  pixel. It is the documented contract ("wall block against non-walkable" in the issue body) and is
  reachable through the `walk_step` bridge, but the Simulator no longer calls it per frame.

A non-walkable column produces no entry in the ground table (`build_ground_column_table` skips
non-walkable tiles; `ground_y_for_column` returns `None`). So today walking into a wall column is
indistinguishable from walking into a pit: the walker keeps moving and then falls.

## Rule to implement (one rule, both languages)

**A grounded walker cannot walk into a column that has no walkable surface.** It stops at the last
pixel before that column.

Chosen over a terrain-aware rule (`wall` blocks, `air` falls) because the ground table already
throws the terrain distinction away, and re-introducing it would mean a second table threaded into
the hot path for a difference nothing in the world exercises. The rule is expressible in both
languages with the data each already has: GDScript has `ground_y_by_col`, Rust has
`ground_y_for_column(tiles, grid, col)`.

Consequences, stated so they are not discovered later:

- A pit edge blocks instead of swallowing the avatar. The avatar has no jump, so falling in would
  strand it; blocking is the better prototype behaviour.
- Falling is still possible when the walker is not grounded (spawn drop, mid-air) or when
  `move_x == 0`, which is exactly the case the existing `falls_through_void` guard covers.

## Algorithm

Inserted after the destination column is known, before the landing logic:

```text
if move_x != 0 and state.grounded and ground_table has no entry for destination_col:
    pos_x = the previous frame's pos_x          # refuse the step
    destination_col = floor(pos_x / tile_w)
```

Refusing the step rather than clamping to the column edge: a clamp is direction-blind (clamping to
`blocked_col * tile_w - 1` would teleport a LEFT-moving walker across the gap it is walking into),
and the walker is at most one frame's travel from the boundary, so refusing leaves no visible gap.

Stability: the previous pixel is inside a column that has a surface, so the next frame proposes the
same crossing and refuses again. Idempotent, no drift, no new state.

Complexity: one dictionary lookup and one comparison per frame, on top of the two lookups the frame
already does. No new allocation, no signature change, no new field on the walker state.

## Tests

- GDScript, `editor/tests/test_walker_step.gd`: new guard `blocked_by_wall` — ground at columns 0-2
  and 4-5 at 992 px, nothing at column 3; walk right from x=16 for 600 frames; assert `pos_x` stops
  at `3 * 32 - 1 = 95`, stays grounded at 992, and a further 120 frames do not move it. Second
  assertion walks left into the same gap from column 4 to prove the clamp is direction-agnostic.
- Rust, `crates/sstd-core/src/physics.rs`: `walker_blocked_by_missing_surface` — walkable tiles at
  columns 0-2 and 4-5 (row 1, 64 px tiles), `move_x = 1`, 300 steps; assert `pos_x == 3.0 * 64.0 -
  1.0` and `grounded`.
- Mutation verification: deleting the guard must turn both new tests red, and only those.

## Out of scope

Jumping, sub-tile collision, lava damage, and separating a wall from a pit.

## Result (2026-11-09)

Implemented as planned, with one design change found during implementation: the first draft clamped
`pos_x` to "one pixel before the blocked column", which is direction-blind and would have carried a
LEFT-moving walker across the gap onto the far side. The shipped rule instead refuses the step
(`pos_x = previous pos_x`), which is direction-agnostic and leaves at most one frame of travel as the
gap. The new left-direction assertion is what catches the clamp version.

Files: `editor/scripts/world_view.gd` (guard + corrected function comment),
`crates/sstd-core/src/physics.rs` (same guard, `feet_col` folded into `world_col`),
`editor/tests/test_walker_step.gd` (guard `blocked_by_wall`), `crates/sstd-core/src/physics.rs`
test `walker_blocked_by_missing_surface`.

Measured:

| Run | Result |
|---|---|
| `test_walker_step.gd` | rc=0, `blocked_by_wall` prints 5 `ok` lines |
| `test_slope_set.gd`, `test_walker_visibility.gd` | rc=0 |
| `test_screen_store`, `test_image_to_map`, `test_terrain_brush`, `test_override_merge` | rc=0 |
| headless boot `--quit-after 120` | rc=0, no parse errors |
| `cargo test -p sstd-core` | rc=0, **142 passed** (was 141) |
| `rustfmt --edition 2021 --check crates/sstd-core/src/physics.rs` | clean |

Mutation verification (each mutant alone, then reverted and re-run green):

| Mutant | Effect |
|---|---|
| GDScript guard replaced with `if false:` | 4 assertions red; right walk reached x=2016 and fell, left walk reached x=0 |
| Rust guard replaced with `if false {` | `walker_blocked_by_missing_surface` panicked at the `pos_x < 192` assertion |

Note: reverting a mutation with `git checkout -- <file>` on a file whose fix is still uncommitted
removes the fix too. Both files were re-edited afterwards and re-verified green, which is why the
numbers above come from the final, unmutated tree.

Published: commit `a4085da` on `fix/122-wall-block`, merged ff-only into `trunk`
(`e65cd11..a4085da`) and pushed to `origin`. Follow-up comment posted on #122
(id 6755391632). Issue left OPEN by @me's choice.
