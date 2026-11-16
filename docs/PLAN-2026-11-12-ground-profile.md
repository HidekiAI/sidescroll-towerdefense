# Plan: continuous ground profile (#126)

Ticket: [#126](https://github.com/HidekiAI/sidescroll-towerdefense/issues/126) -- `feat(sim):
continuous ground_y(pos_x) so the collision surface matches the drawn slope`, plus the
2026-11-15 amendment comment on that issue.

Design of record: wiki `TDD_Collision-Map-Tooling` (#63, the per-pixel collision map),
`TDD_Physics` (ground contact), `TDD_Simulator-Ground-Walk` §7. Predecessor: #124
(closed "accept for now"), post-mortem `PostMortem/Slope-Hopping-2026-11-10`.

## Objective

Replace the per-column step table with a profile derived from the tiles' own **alpha**
(`alpha != 0x00` is solid). The drawn surface and the collision surface are then ONE datum
(the tile PNG's alpha), read by both steppers:

- `editor/scripts/world_view.gd::step_walker` -- the live path.
- `crates/sstd-core/src/physics.rs::step_walker` -- the model behind the bridge's `walk_step`.

Anti-jitter is in scope: on penetration, snap the entity out and zero the offending velocity
component (`dX = dV/dT`, `dV = 0` -> `dX = 0`), never zero gravity.

## Provenance (gathered, not assumed)

`git blame` on every block this plan changes, `gh issue view` 122/124/125/126, `gh pr list
--state all` (only merged #95/#97/#98; no PR for #122/#124/#125/#126, so the issue threads are
the review record), and the three design pages.

| Block being changed | Introduced by | Decision it carries |
|---|---|---|
| `build_ground_column_table` (world_view.gd:518) | #122 | one surface per column, absence = hole; **retired here** |
| `step_walker` (world_view.gd:553) + the interpolation hack (:586) | #122, reshaped #124 | landing tested against the column's plateau; **replaced here** |
| `world_view_ground.gd::_draw` (:37-77) | #122 | the 4:7 polygon's top edge was the "drawn surface" AC1 measured; **replaced by tile-alpha draw** |
| `ground_y_for_column` / `step_walker` (physics.rs:125 / :134-264) | #122, legibility #125 | Rust model has no within-column interpolation; **ported to the alpha sampler** |
| `slope_feet_y_matches_drawn_surface` (test_walker_step.gd:271) | #124 | caps a single-frame rise at 16 px, never compares to the drawn line; **rewritten to compare to alpha** |
| `_check_ground_profile` (test_slope_set.gd:112) | #122 | pins the injected slope set; **re-pointed at the alpha surface** |
| `inject_simulator_hills` (main.gd:227) | #122 | injects the 7 `slope_up_*` tiles; **unchanged** (placement is not this ticket) |
| `auto_detect_sub_tile_mask` (tools/import-tiles/src/main.rs:354) | #63 lineage | coarse 4-bit mask; **`solid_mask` added alongside it** |

## Art -> data -> tech (the pipeline this ticket consumes)

Art defines data, data defines tech.

1. **Art:** the slope is one X-by-Y tile set authored as a single PNG (`tileset_2-2.png`, 4x2
   grid, cell 126x126, margins (104,65), separation (1,1)). Its "air" is opaque grey
   `#D1D1D1`; the sheet has no alpha channel.
2. **Data:** `tools/import-tiles` colour-keys that grey to transparent (fuzz-tolerant) and
   emits a per-pixel `solid_mask` per tile (`alpha != 0`), alongside the existing sliced
   `{key}_32x32.png` and the coarse `sub_tile_mask`. `solid_mask` is the primary collision
   data form for maps, scenes and collisions.
3. **Tech:** both steppers build the world surface from `solid_mask` (topmost solid row per
   pixel column) and sample it at `pos_x`.

**Rule:** a tile's alpha IS its collider. Fog/atmosphere goes on its own render LAYER, never
by lowering a tile's alpha. The game has no fog-of-war concept.

## What exists today (the structures this builds on)

| Thing | Where | What it does |
|---|---|---|
| `build_ground_column_table(tiles, tile_h_px)` | `world_view.gd:518` | `{world_col: surface_row * tile_h_px}`; absence = hole. Retired |
| `step_walker(state, move_x, dt, ground_y_by_col, tile_w_px, ...)` | `world_view.gd:553` | gravity, landing, step up, fall damage, world clamp. Retired |
| the per-column interpolation hack | `world_view.gd:586` | spreads current->next across the column; the #124 sawtooth. Deleted |
| `_rebuild_ground_tiles()` | `world_view.gd:634` | builds `_tiles_cache`, hands it to the ground layer, rebuilds the table, spawns. Now builds the alpha profile |
| spawn paths | `world_view.gd:667-728` | read the column table; read the profile now |
| `world_view_ground.gd::_draw` | `:37-77` | flat tiles as rects + a 4:7 ramp polygon. Replaced by drawing the tiles |
| `ground_y_for_column` / `step_walker` | `physics.rs:125` / `:134-264` | per-column rule, no interpolation. Ported to the sampler |
| `auto_detect_sub_tile_mask` | `tools/import-tiles/src/main.rs:354` | coarse 4-bit mask. `solid_mask` added beside it |
| `WalkTile` -> `phys::Tile` | `sstd-editor-bridge/src/lib.rs:2300` | carries `world_tile_x/y`, `is_walkable`; gains `solid_mask` |

## Data model and build rule (identical on both sides)

- **Per-tile `solid_mask`:** for each tile key, the topmost solid row per pixel column
  (`-1` = fully air). One bit-row per column, derived from the PNG alpha.
- **World surface:** a mapping from world pixel column -> `surface_y = tile_row * tile_h_px +
  top_row`. An absent column is air.
- **Stitch:** the surface of a column is the smallest (topmost) row among the tiles in it.
- **Sample:** `surface_y(pos_x) = surface[floor(pos_x)]`, interpolated between the column and
  the next (this is a genuine alpha-derived ramp, not the old hack). Past the last column the
  last value answers, preserving the right-edge behaviour pinned by `clamps_right`.

```
ground_y_at(surface_by_col, pos_x) -> Option<f64>   // GDScript: NAN for air
    Air -> None
    column = floor(pos_x); interpolate between column and column+1
    -> Some(surface_y)
```

## Per-tick change in `step_walker`

```
if surface is None:                                       -> hole; a grounded walker starts a fall
elif vel_y >= 0 and feet_was_y <= surface <= feet_now_y:   -> landed from above (fall damage applies)
elif surface < feet_was_y and move_x != 0:                -> step up / step down (landed, vy = 0, no fall damage)
elif feet_y > surface + EMBED_TOLERANCE_PX:               -> penetration: snap feet_y to surface, vy = 0, grounded, zero the offending velocity component
elif grounded and feet_y <= surface and surface - feet_y <= GROUND_SNAP_PX:
                                                           -> in contact: glued to the surface, vy = 0
else:                                                     -> airborne, gravity takes over
```

The penetration/anti-jitter arm must come AFTER landing and step-up, or it swallows fall
damage. Anti-jitter zeroes the velocity component that drove the penetration (`vy` for a
vertical surface), not gravity.

The `feet_*` names stay: they are the existing `walk_step` wire contract's names. Renaming to
a bounds-based name is the separate collision-bounds ticket, not this one.

## Files

| File | Change | Serves |
|---|---|---|
| `tools/import-tiles/src/main.rs` | colour-key grey `#D1D1D1` to transparent; emit per-pixel `solid_mask` beside `sub_tile_mask`; keep sliced `{key}_32x32.png` | AC1 (alpha datum), art->data |
| `editor/scripts/world_view.gd` | add `build_ground_profile`, `ground_y_at`; `step_walker` takes the profile and the anti-jitter snap; delete `build_ground_column_table`, the hack, `tile_w_px`; `_rebuild_ground_tiles` builds the profile; spawn Y from the profile | AC1, AC3, AC4 |
| `editor/scripts/world_view_ground.gd` | `_draw` draws the tiles (their alpha) instead of the 4:7 polygon; flat tiles stay full rects | AC1 |
| `crates/sstd-core/src/physics.rs` | `build_ground_profile`, `ground_y_at`, `GROUND_SNAP_PX`, `EMBED_TOLERANCE_PX`, the anti-jitter snap; `step_walker` samples the profile | AC1, AC4 |
| `crates/sstd-editor-bridge/src/lib.rs` | `WalkTile` gains `solid_mask` | AC1 |
| `editor/tests/test_walker_step.gd` | ground table -> profile; `slope_feet_y_matches_drawn_surface` compares feet Y to the ALPHA surface (independently derived) within 1 px | AC1, AC3 |
| `editor/tests/test_slope_set.gd` | `_check_ground_profile` asserts the alpha-derived surface of the shipped run | AC4 |
| `crates/sstd-core/src/physics.rs` tests | profile + anti-jitter + back-to-back tests (below) | AC1, AC4 |

Wiki/docs (executed per the Always-Update-All-Documents rule, not scored):
`TDD_Collision-Map-Tooling` (alpha is the primary collision data form),
`TDD_Physics` (alpha datum; fog-on-a-layer errata), `TDD_Simulator-Ground-Walk` §5/§7 wording,
`PostMortem/Slope-Hopping-2026-11-10` (dated supersession), `Home.md` (redact the 4:7-polygon
description).

## Tests (falsifiable, each pinned to a number, each with its mutant)

### GDScript `test_walker_step.gd`

| Test | Assertion | AC | Mutant that reddens it |
|---|---|---|---|
| `profile_from_alpha_surface_rule` | given per-column top rows (e.g. slope_up_1 {13,13,12,...0}), `build_ground_profile` returns `row*tile_h + top_row` per column; a fully air column is absent | AC4 | M1 |
| `interior_air_below_surface_is_solid` | a column with tiles at row 31 and row 35 answers 31, never 35 | AC4 | M1 |
| `slope_feet_y_matches_drawn_surface` | walker seeded at the run's first column centre; every ramp frame, feet Y equals the alpha surface within 1 px. The alpha surface is **computed from the tile alpha in the test**, not from the drawer | AC1, AC3 | M2, M5 |
| `slope_descent_matches_drawn_surface` | walked LEFT, grounded every ramp frame, feet Y equals alpha surface within 1 px, hp unchanged | AC1 | M2 |
| `penetration_snaps_out_and_zeroes_velocity` | walker forced 5 px into a surface has feet snapped to the surface and `vel_y == 0` next frame; no oscillation over 10 frames | AC1 | M5 |
| `ground_snap_threshold_is_under_one_tile` | surface 16 px below glues; 32 px below becomes airborne | AC1, AC4 | M4 |
| `blocked_by_wall_retries` | a grounded walker refused at an air segment for 120 frames does not creep | AC4 | M7 |

Existing `falls_then_lands`, `stays_on_ground`, `falls_through_void`, `clamps_left`,
`clamps_right`, `blocked_by_wall`, `deterministic`, `hill_climb`, `hill_fall_damage` stay
green; fixtures move from a column dictionary to tile arrays with alpha-derived surfaces.

### Rust `physics.rs`

| Test | Assertion | AC | Mutant |
|---|---|---|---|
| `profile_from_alpha_surface_rule` | topmost solid row per column; fully air column => `None` | AC4 | M1 |
| `interior_air_below_surface_is_solid` | rows 31 and 35 => 31, never 35 | AC4 | M1 |
| `profile_coalesces_adjacent_holes` | three adjacent air columns => one Air run | AC4 | M3 |
| `walking_up_the_ramp_tracks_the_alpha_surface` | `move_x = 1.0` at 200 pps: `pos_y` equals the alpha-surface sample every ramp frame (within 1e-6), strictly decreasing | AC1 | M2 |
| `walking_down_the_ramp_grounded_without_damage` | mirrored walk leftward: grounded every frame, `pos_y` strictly increasing, hp unchanged | AC1 | M2 |
| `penetration_snaps_out_and_zeroes_velocity` | 5 px penetration snaps to surface, `vel_y == 0`, stable over 10 ticks | AC1 | M5 |
| `snap_threshold_is_under_one_tile` | 16 px glues, 32 px does not | AC1, AC4 | M4 |
| `same_input_twice_gives_the_same_state` | back-to-back repeat pin | AC4 | M6 |

### Mutation battery

| Mutant | Expected red |
|---|---|
| M1 surface reads bottommost instead of topmost row | `profile_from_alpha_surface_rule`, `interior_air_below_surface_is_solid` |
| M2 lookup returns the column value with no interpolation | `slope_feet_y_matches_drawn_surface`, `walking_up_the_ramp_tracks_the_alpha_surface` |
| M3 air column returns 0.0 instead of None | `falls_through_void`, `profile_coalesces_adjacent_holes` |
| M4 GROUND_SNAP_PX raised to 32 | `ground_snap_threshold_is_under_one_tile`, `snap_threshold_is_under_one_tile` |
| M5 anti-jitter arm deleted | `penetration_snaps_out_and_zeroes_velocity` (both sides) |
| M6 profile rebuilt non-deterministically (iteration order) | `same_input_twice_gives_the_same_state` |
| M7 refusal sets grounded = false | `blocked_by_wall_retries` |

Every test above has a named mutant that reddens it, and every mutant reddens at least one
test. A mutation is validated before it is interpreted (it must fire and parse, else the
experiment is void).

## Gate (run before any commit)

`cargo test -p sstd-core` plus `test_walker_step`, `test_slope_set`, `test_walker_visibility`,
each gated on its EXIT CODE. `tools/import-tiles` builds and its `solid_mask` is verified on
the 4x2 sheet (grey keyed out, art opaque).

## Risks

- The GDScript `step_walker` signature changes (table removed, profile added). Callers are
  `world_view.gd::_update_walker` and `test_walker_step.gd`, both in the Files table.
- `ground_y_at` past the last column clamps to the last value, keeping `clamps_right` green.
- Anti-aliased art leaves a fringe at the colour-key boundary; the alpha threshold is "any
  alpha > 0", so a fringe pixel counts as solid. This biases the surface a pixel low at worst,
  inside AC1's 1 px band.
- `test_screen_store.gd` is known red on Godot 4.7.2 (#85) and is not in this gate.
- Never `git add -A` (516 MB vendored `editor/addons/`); stage paths explicitly.

## Possible follow-ups (not this ticket)

- Collision bounds instead of the avatar's art (recorded directive, own ticket).
- Graduated fall damage from impact energy (recorded directive, own ticket).
- Relocating the injected slope run for whichever profile reads best on screen (art/placement
  choice, not required by this ticket's acceptance criteria).

## Plan gate

**SCORE: 100/100** — passed independent evaluation on 2026-11-15, iteration 1, by free model evaluator.

CRITICAL (unsupported by ticket): none.

UNCOVERED (ticket clause with no plan item, or `required test: <class>`): none.

OTHER DEFICIENCIES: none.

REFINEMENTS: none.

Append above after evaluator passes (date, score, iterations, removed items with reasons).
