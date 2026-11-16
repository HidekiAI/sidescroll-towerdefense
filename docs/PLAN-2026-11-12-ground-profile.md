# Plan: continuous ground profile (#126)

Ticket: [#126](https://github.com/HidekiAI/sidescroll-towerdefense/issues/126) -- `feat(sim):
continuous ground_y(pos_x) so the collision surface matches the drawn slope`, plus the
2026-11-15 amendment comment on that issue.

Design of record: wiki `TDD_Collision-Map-Tooling` (#63, the per-pixel collision map),
`TDD_Physics` (ground contact), `TDD_Simulator-Ground-Walk` §7. Predecessor: #124
(closed "accept for now"), post-mortem `PostMortem/Slope-Hopping-2026-11-10`.

## Objective

Replace the per-column step table with a profile derived from the tiles' own **art**, through
the ART -> DATA -> TECH pipeline. The tool scans each tile, decides air versus collidable,
emits a per-pixel `solid_mask`, and both steppers build the world surface from that mask and
sample it at `pos_x`. The drawn surface and the collision surface are then ONE datum (the
tile's silhouette):

- `editor/scripts/world_view.gd::step_walker` -- the live path.
- `crates/sstd-core/src/physics.rs::step_walker` -- the model behind the bridge's `walk_step`.

Anti-jitter is in scope: on penetration, snap the entity out and zero the offending velocity
component (`dX = dV/dT`, `dV = 0` -> `dX = 0`), never zero gravity.

## The pattern: ART -> DATA -> TECH (directive 2026-11-16)

**The slope ANGLE is irrelevant and is never an input.** The artist authored the climb for how
it FEELS; the family's measured rises are 7 to 30 degrees and deliberately uneven. So the tool
must NOT infer geometry from a nominal angle, a formula, or a hand-authored ladder. It scans
the ART for air versus collidable, turns that into DATA (a per-pixel collision bit-mask), and
TECH consumes the mask. Redrawing the tile re-derives the mask with no code change.

1. **Art.** The tiles the runtime walks are `slope_up_{0..6}_32x32.png`, committed under
   `editor/assets/tiles/`. `inject_simulator_hills` (main.gd:236,250) places them at rows
   `31 - int(4*i/7)`. `slope_up_0` is a flat solid cap; `slope_up_1..6` are diagonals.
   `slope_30deg_*` is a second family whose air is drawn opaque grey; this ticket does NOT walk
   it. **There is no source sheet for `slope_up_*`** (`tools/import-tiles` slices sheets and
   `tools/extract-tiles` was deleted), so the tool must be runnable on the committed tile PNGs
   themselves, not only on a sheet.
2. **Data.** The tool emits `solid_mask` per tile, in the wiki's form
   (`TDD_Collision-Map-Tooling` line 20): **one bit per tile pixel**, a `PackedByteArray` of
   `(w*h + 7) / 8` = 128 bytes for a 32x32 tile, bit `(row * w + col)`. `sub_tile_mask` stays
   as the coarse 4-bit predecessor.
3. **Tech.** Both steppers build the world surface from `solid_mask` (topmost set bit per pixel
   column) and sample it at `pos_x`.

## The air-vs-collidable rule (what the tool implements)

The tile's **greyscale is the detector**; the mask is the product. Measured on the committed art
(ImageMagick grey, alpha ignored):

| tile | distinct greys | air | ground | bottom row |
|---|---|---|---|---|
| `slope_up_0` | 1 | none | 155 | all 155 |
| `slope_up_1..6` | 2 | 0 | 155 | all 155 |
| `slope_30deg_0_0` | 53 (2 modes) | ~209 | ~137 | mixed (4 greys) |
| `air` | 1 | - | 0 (opaque) | all 0 |

**Pipeline (@me, 2026-11-16): GREYSCALE -> THRESHOLD -> NO-DONUT CLEAN -> MASK.**

1. Acquire the NxN tile; greyscale it (alpha ignored).
2. Read a region's colour by sampling a **small uniform patch (2x2 to 4x4)**, not a single
   pixel: a patch whose pixels are a similar colour is safely INSIDE a region, never on the
   anti-aliased edge.
3. Anchor the GROUND at the tile's **bottom row** (@me, 2026-11-16): the greyscale value
   touching the bottom row is solid; the other value is air. A single-grey tile is all solid
   (`slope_up_0`).
4. Threshold to a 2-valued air(0)/solid(1) mask.
5. **No-donut clean: floating solid specks only** (@me, 2026-11-16). A solid pixel not
   4-connected to the ground component becomes air -- "pretend it was just dust". Enclosed air
   pockets are NOT filled.
6. **UNRESOLVED tiles are flagged, never guessed** (@me, 2026-11-16): the tool bakes a red 1px
   border into the emitted PNG so the artist or level-designer hand-edits the collision.

**Why the flag must exist (@me, 2026-11-16).** No single anchor is universally right: the
upper-left corner reads air for a slope-up but not a slope-down; walking bottom-to-top reads air
for a slope but not a cave ceiling. The uniform-patch sampling makes the colour read robust; the
flag catches the tiles where the anchors disagree.

**Flag criteria (@me, 2026-11-16).** BOTH greyscale clusters touch the tile's bottom row, OR the
greyscale does not split into exactly two clusters. `slope_up_*` all resolve;
`slope_30deg_0_0` would flag.

**Divergence from the wiki, stated (not claimed as "the wiki's form").**
`TDD_Collision-Map-Tooling` section 1 also says "every pixel below the first solid pixel in a
column is solid". This ticket does NOT adopt that fill-below clause (@me chose specks-only), so
the plan no longer claims the mask rule is the wiki's rule verbatim. For the slope art the two
agree, because there are no interior pockets.

## Provenance (gathered, not assumed)

`git blame` on every block this plan changes, `gh issue view` 122/124/125/126, `gh pr list
--state all` (only merged #95/#97/#98; no PR for #122/#124/#125/#126, so the issue threads are
the review record), and the three design pages.

| Block being changed | Introduced by | Decision it carries |
|---|---|---|
| `build_ground_column_table` (world_view.gd:518) | #122 | one surface per column, absence = hole; **retired here** |
| `step_walker` (world_view.gd:553) + the interpolation hack (:586) | #122, reshaped #124 | landing tested against the column's plateau; **replaced here** |
| `world_view_ground.gd::_draw` (:37-77) | #122 | the 4:7 polygon's top edge was the "drawn surface" AC1 measured; **replaced by a `solid_mask` surface draw** |
| `ground_y_for_column` / `step_walker` (physics.rs:125 / :134-264) | #122, legibility #125 | Rust model has no within-column interpolation; **ported to the mask sampler** |
| `slope_feet_y_matches_drawn_surface` (test_walker_step.gd:271) | #124 | caps a single-frame rise at 16 px, never compares to the drawn line; **rewritten to compare to the mask surface** |
| `_check_ground_profile` (test_slope_set.gd:112) | #122 | pins the injected slope set; **re-pointed at the mask surface** |
| `inject_simulator_hills` (main.gd:227) | #122 | injects the 7 `slope_up_*` tiles; **unchanged** (placement is not this ticket) |
| `auto_detect_sub_tile_mask` (tools/import-tiles/src/main.rs:354) | #63 lineage | coarse 4-bit mask; **`solid_mask` added alongside it** |
| `default_package/terrain_types.json` (7 framework types) | #54 lineage | framework terrain set; **gains the 7 `slope_up_*` entries with `solid_mask`** |

## What exists today (the structures this builds on)

| Thing | Where | What it does |
|---|---|---|
| `build_ground_column_table(tiles, tile_h_px)` | `world_view.gd:518` | `{world_col: surface_row * tile_h_px}`; absence = hole. Retired |
| `step_walker(state, move_x, dt, ground_y_by_col, tile_w_px, ...)` | `world_view.gd:553` | gravity, landing, step up, fall damage, world clamp. Retired |
| the per-column interpolation hack | `world_view.gd:586` | spreads current->next across the column; the #124 sawtooth. Deleted |
| `_rebuild_ground_tiles()` | `world_view.gd:634` | builds `_tiles_cache`, hands it to the ground layer, rebuilds the table, spawns. Now builds the mask profile |
| spawn paths | `world_view.gd:667-728` | read the column table; read the profile now |
| `world_view_ground.gd::_draw` | `:37-77` | flat tiles as rects + a 4:7 ramp polygon; no texture source. Replaced by a `solid_mask` surface draw |
| `ground_y_for_column` / `step_walker` | `physics.rs:125` / `:134-264` | per-column rule, no interpolation. Ported to the sampler |
| `auto_detect_sub_tile_mask` | `tools/import-tiles/src/main.rs:354` | coarse 4-bit mask. `solid_mask` added beside it |
| `TerrainType` | `crates/sstd-core/src/terrain.rs:162` | framework terrain type; gains `solid_mask` |
| `WalkTile` -> `phys::Tile` | `sstd-editor-bridge/src/lib.rs:2300` | carries `world_tile_x/y`, `is_walkable`; gains `solid_mask` |
| `slope_up_*` placement | `main.gd:236,250` | terrain-key strings; walkable via the default heuristic at `world_view.gd:656`. No terrain entry today |

## Data model and build rule (identical on both sides)

- **`solid_mask` is a property of the tile ART, so it lives on the terrain type.**
  `TerrainType` (`crates/sstd-core/src/terrain.rs:162`) gains `solid_mask`; the framework
  `editor/default_package/terrain_types.json` carries it for the 7 `slope_up_*` entries the tool
  writes (@me, 2026-11-16). `ScreenTileEntry` does NOT gain it: a placement references a terrain
  key, and duplicating the same art's mask per placement would be redundant. This corrects the
  earlier "carried by `ScreenStore` and `storage.rs`" wording, which put the mask on the
  placement.
- **World surface:** a mapping from world pixel column -> `surface_y = tile_row * tile_h_px +
  top_row`, where `top_row` is the topmost set bit of the tile's `solid_mask` in that pixel
  column (`-1` = fully air, so the column is absent).
- **Stitch:** the surface of a world pixel column is the smallest (topmost) `surface_y` among
  the tiles overlapping it.
- **Sample:** `surface_y(pos_x) = surface[floor(pos_x)]`, interpolated between the pixel column
  and the next (a genuine art-derived ramp, not the old hack). Past the last column the last
  value answers, preserving the right-edge behaviour pinned by `clamps_right`.

```
ground_y_at(surface_by_col, pos_x) -> Option<f64>   // GDScript: NAN for air
    Air -> None
    column = floor(pos_x); interpolate between column and column+1
    -> Some(surface_y)
```

## Per-tick change in `step_walker`

```
GROUND_SNAP_PX = 16.0       // half a 32 px tile: a grounded walker within this above the surface stays glued
EMBED_TOLERANCE_PX = 8.0    // a quarter tile: the deepest penetration treated as contact before the snap

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
| `tools/import-tiles/src/main.rs` | add `derive_solid_mask(pixels, w, h) -> (mask, resolved)` (greyscale pipeline: uniform-patch colour read, bottom-row ground anchor, speck removal, ambiguity flag); emit per-pixel `solid_mask` beside `sub_tile_mask`; bake a red 1px border into the emitted PNG and set `resolved = false` when UNRESOLVED; add a tile-scan mode that runs the derivation over a directory of committed `{key}_32x32.png` and writes/updates the 7 `slope_up_*` entries in `default_package/terrain_types.json` | AC1, art->data |
| `editor/default_package/terrain_types.json` | add `slope_up_0..6` with `is_walkable: true` and `solid_mask` (base64 128 bytes) | AC1, data home |
| `crates/sstd-core/src/terrain.rs` | `TerrainType` gains `solid_mask` (serde default keeps old files parsing) | AC1 |
| `crates/sstd-core/src/physics.rs` | `build_ground_profile`, `ground_y_at`, `GROUND_SNAP_PX`, `EMBED_TOLERANCE_PX`, the anti-jitter snap; `step_walker` samples the profile | AC1, AC4 |
| `crates/sstd-editor-bridge/src/lib.rs` | `WalkTile` gains `solid_mask` | AC1 |
| `editor/scripts/world_view.gd` | add `set_terrain_masks(masks_by_key)`, `build_ground_profile`, `ground_y_at`; `_rebuild_ground_tiles` resolves each tile's mask by terrain key; `step_walker` takes the profile and the anti-jitter snap; delete `build_ground_column_table`, the hack, `tile_w_px` | AC1, AC3, AC4 |
| `editor/scripts/main.gd` | wire `set_terrain_masks` from `terrain_editor.get_terrain_types()` to the simulator's world view | AC1 |
| `editor/scripts/world_view_ground.gd` | `_draw` draws the surface from `solid_mask` (per-column rects) instead of the 4:7 polygon; flat tiles stay full rects. No texture source is needed or added | AC1 |
| `editor/tests/test_walker_step.gd` | ground table -> profile; `slope_feet_y_matches_drawn_surface` compares feet Y to the mask surface derived INDEPENDENTLY from the tile PNG in the test, within 1 px | AC1, AC3 |
| `editor/tests/test_slope_set.gd` | `_check_ground_profile` asserts the mask-derived surface of the shipped run | AC4 |
| `crates/sstd-core/src/physics.rs` tests | profile + anti-jitter + back-to-back tests (below) | AC1, AC4 |
| `tools/import-tiles/src/main.rs` tests | in-source `#[cfg(test)]` mask tests on synthetic RGBA images (below) | AC1 |

Wiki/docs (executed per the Always-Update-All-Documents rule, not scored):
`TDD_Collision-Map-Tooling` (greyscale pipeline; the mask as the collision data form; note the
specks-only divergence), `TDD_Physics` (the mask datum), `TDD_Simulator-Ground-Walk` §5/§7
wording, `PostMortem/Slope-Hopping-2026-11-10` (dated supersession), `Home.md` (redact the
4:7-polygon description).

## Tests (falsifiable, each pinned to a number, each with its mutant)

### GDScript `test_walker_step.gd`

| Test | Assertion | AC | Mutant |
|---|---|---|---|
| `solid_mask_surface_rule` | given a tile mask, `build_ground_profile` returns `row*tile_h + top_row` per column; a fully air column is absent | AC4 | M1 |
| `interior_air_below_surface_is_solid` | a column with solid at rows 31 and 35 answers 31, never 35 | AC4 | M1 |
| `slope_feet_y_matches_drawn_surface` | walker seeded at the run's first column centre; every ramp frame, feet Y equals the surface derived INDEPENDENTLY from the tile PNG's alpha in the test, within 1 px. Expected values are the measured jagged table in the decisions section (992/1005/987/969/950/932/914), never the 4:7 ladder | AC1, AC3 | M2, M5 |
| `slope_descent_matches_drawn_surface` | walked LEFT, grounded every ramp frame, feet Y equals the mask surface within 1 px, hp unchanged | AC1 | M2 |
| `penetration_snaps_out_and_zeroes_velocity` | walker forced 5 px into a surface has feet snapped to the surface and `vel_y == 0` next frame; no oscillation over 10 frames | AC1 | M5 |
| `ground_snap_threshold_is_under_one_tile` | surface 16 px below glues; 32 px below becomes airborne | AC1, AC4 | M4 |
| `blocked_by_wall_retries` | a grounded walker refused at an air segment for 120 frames does not creep | AC4 | M7 |

Existing `falls_then_lands`, `stays_on_ground`, `falls_through_void`, `clamps_left`,
`clamps_right`, `blocked_by_wall`, `deterministic`, `hill_climb`, `hill_fall_damage` stay
green; fixtures move from a column dictionary to tile arrays with mask-derived surfaces.

### Rust `physics.rs`

| Test | Assertion | AC | Mutant |
|---|---|---|---|
| `solid_mask_surface_rule` | topmost solid row per column; fully air column => `None` | AC4 | M1 |
| `interior_air_below_surface_is_solid` | rows 31 and 35 => 31, never 35 | AC4 | M1 |
| `profile_coalesces_adjacent_holes` | three adjacent air columns => one Air run | AC4 | M3 |
| `walking_up_a_monotonic_ramp_tracks_the_surface` | a SYNTHETIC monotonic ramp, `move_x = 1.0` at 200 pps: `pos_y` equals the surface sample every ramp frame (within 1e-6), strictly decreasing | AC1 | M2 |
| `shipped_run_surface_is_reproduced_including_the_notch` | the shipped 7-tile run: feet Y equals the surface at every column, and the 992 -> 1005 px notch at world col 61 is reproduced. NO monotonicity claim | AC1, AC3 | M2 |
| `walking_down_the_ramp_grounded_without_damage` | mirrored walk leftward: grounded every frame, hp unchanged | AC1 | M2 |
| `penetration_snaps_out_and_zeroes_velocity` | 5 px penetration snaps to surface, `vel_y == 0`, stable over 10 ticks | AC1 | M5 |
| `snap_threshold_is_under_one_tile` | 16 px glues, 32 px does not | AC1, AC4 | M4 |
| `same_input_twice_gives_the_same_state` | back-to-back repeat pin | AC4 | M6 |

### Tool `tools/import-tiles` (in-source `#[cfg(test)]`, synthetic RGBA images)

| Test | Assertion | AC | Mutant |
|---|---|---|---|
| `mask_two_cluster_bottom_anchor` | a synthetic RGBA tile with air above and ground at the bottom: the mask sets exactly the ground pixels | AC1 | T1 |
| `mask_all_grey_is_all_solid` | a single-grey tile: every bit set, `resolved = true` | AC1 | T2 |
| `mask_removes_floating_solid_specks` | a solid speck surrounded by air becomes air; the ground component survives | AC1 | T3 |
| `mask_flags_both_clusters_on_bottom_row` | a tile whose bottom row carries both clusters returns `resolved = false`, and its emitted PNG has a red 1px border | AC1 | T4 |
| `mask_uniform_patch_ignores_edge_pixel` | a 2x2 uniform patch reads the region colour where a lone anti-aliased edge pixel would not | AC1 | T5 |

### Mutation battery

| Mutant | Expected red |
|---|---|
| M1 surface reads bottommost instead of topmost row | `solid_mask_surface_rule`, `interior_air_below_surface_is_solid` |
| M2 lookup returns the column value with no interpolation | `slope_feet_y_matches_drawn_surface`, `walking_up_a_monotonic_ramp_tracks_the_surface`, `shipped_run_surface_is_reproduced_including_the_notch` |
| M3 air column returns 0.0 instead of None | `falls_through_void`, `profile_coalesces_adjacent_holes` |
| M4 GROUND_SNAP_PX raised to 32 | `ground_snap_threshold_is_under_one_tile`, `snap_threshold_is_under_one_tile` |
| M5 anti-jitter arm deleted | `penetration_snaps_out_and_zeroes_velocity` (both sides) |
| M6 profile rebuilt non-deterministically (iteration order) | `same_input_twice_gives_the_same_state` |
| M7 refusal sets grounded = false | `blocked_by_wall_retries` |
| T1 bottom anchor replaced by the top row | `mask_two_cluster_bottom_anchor` |
| T2 all-grey tile treated as all air | `mask_all_grey_is_all_solid` |
| T3 speck removal deleted | `mask_removes_floating_solid_specks` |
| T4 ambiguity flag never set | `mask_flags_both_clusters_on_bottom_row` |
| T5 colour read from a single pixel | `mask_uniform_patch_ignores_edge_pixel` |

Every test above has a named mutant that reddens it, and every mutant reddens at least one
test. A mutation is validated before it is interpreted (it must fire and parse, else the
experiment is void).

## Gate (run before any commit)

`cargo test -p sstd-core`, `cargo test -p import-tiles` (the tool's in-source mask tests), and
`test_walker_step`, `test_slope_set`, `test_walker_visibility`, each gated on its EXIT CODE. The
`slope_up_*` entries in `default_package/terrain_types.json` are verified by the tool's own
tile-scan run, not by a `slope_30deg_*` colour-key (that family is not walked and has no source
sheet).

## Risks

- The GDScript `step_walker` signature changes (table removed, profile added). Callers are
  `world_view.gd::_update_walker` and `test_walker_step.gd`, both in the Files table.
- `ground_y_at` past the last column clamps to the last value, keeping `clamps_right` green.
- The tool's air heuristic has the two named failure modes (slope-down, cave ceiling). The
  red-border flag is the guard, and it is a visual signal only: a flagged tile is not
  machine-detectable at runtime, by @me's choice.
- Adding the 7 `slope_up_*` entries to `default_package/terrain_types.json` changes the
  framework terrain count. `test_override_merge.gd` and any count-sensitive test must be
  re-run; the 7 entries are additive.
- `world_view.gd` needs a key -> mask lookup, wired from `main.gd`; without it the profile
  cannot resolve a tile's mask.
- Anti-aliased art leaves a fringe at the threshold boundary. The threshold splits two clusters,
  so a fringe pixel lands on one side; this biases the surface a pixel at worst, inside AC1's
  1 px band.
- `test_screen_store.gd` is known red on Godot 4.7.2 (#85) and is not in this gate.
- Never `git add -A` (516 MB vendored `editor/addons/`); stage paths explicitly.

## Possible follow-ups (not this ticket)

- Collision bounds instead of the avatar's art (recorded directive, own ticket).
- Graduated fall damage from impact energy (recorded directive, own ticket).
- Relocating the injected slope run for whichever profile reads best on screen (art/placement
  choice, not required by this ticket's acceptance criteria).
- A per-pixel collision paint UI for the flagged tiles (the editor has only the coarse 4-bit
  quadrant mask today, `terrain_editor.gd`).
- Re-cutting the `slope_up_*` art so the run's seams line up (the accepted notch is art, not
  code).

## Decisions recorded 2026-11-16 (skeptical review against the repo)

1. **The shipped run's mask surface is jagged and non-monotonic -- ACCEPTED.** Measured topmost
   solid row per tile (`alpha != 0`), placed at rows `31 - int(4*i/7)`:

   | world col | 60 | 61 | 62 | 63 | 64 | 65 | 66 |
   |---|---|---|---|---|---|---|---|
   | tile | `slope_up_0` | `_1` | `_2` | `_3` | `_4` | `_5` | `_6` |
   | surface px | 992 | 1005 | 987 | 969 | 950 | 932 | 914 |

   A 13 px DOWNWARD notch at col 61, then boundary jumps of 0-14 px. **AC1 is the bar and is
   accepted as written** -- the walker reproduces the drawn surface including the notch. Fixing
   the art/placement is a separate ticket. Test expectations are the measured numbers above,
   never the `4:7` ladder.
2. **The angle is not an input; the pattern is ART -> DATA -> TECH (directive 2026-11-16).**
   The tool scans the art for air versus collidable and emits the mask; no formula, ladder or
   nominal angle survives into the data or the tech.
3. **The air rule is the GREYSCALE pipeline** (greyscale -> threshold -> no-donut clean ->
   mask), superseding the same day's alpha-first draft. Alpha is the PRODUCT, not the detector.
4. **Per-tile air is located by anchoring the ground at the tile's bottom row**, with region
   colours read from a uniform 2x2-4x4 patch (not a single pixel). A single-grey tile is all
   solid (`slope_up_0`).
5. **The no-donut clean removes floating solid specks only**; enclosed air pockets are NOT
   filled. The wiki's fill-below clause is deliberately not adopted (divergence stated above).
6. **Unresolvable tiles are flagged with a red 1px border baked into the emitted PNG**, so the
   artist or level-designer hand-edits the collision. Flag criteria: both clusters touch the
   bottom row, or the greyscale is not exactly two clusters.
7. **`slope_up_0..6` are added to `default_package/terrain_types.json`, each carrying
   `solid_mask`.** The mask lives on the terrain type, not the placement.
8. **`solid_mask` is the wiki bitfield** (`TDD_Collision-Map-Tooling` line 20): one bit per
   tile pixel, 128 bytes for 32x32. `sub_tile_mask` stays as the coarse predecessor.

## Second skeptical pass (2026-11-16) -- findings, now resolved

Every premise below was checked against the repo, not asserted. Each finding is resolved above.

1. **The Rust ramp test contradicted decision 1.** RESOLVED: split into
   `walking_up_a_monotonic_ramp_tracks_the_surface` (a synthetic monotonic ramp, where
   "strictly decreasing" is valid) and `shipped_run_surface_is_reproduced_including_the_notch`
   (the shipped run, no monotonicity claim).
2. **The Gate still tested the demoted colour-key, and the tool change had no end-to-end
   input.** RESOLVED: the Gate now runs the tool's in-source tests on synthetic RGBA images,
   and the tool gains a tile-scan mode over the committed `slope_up_*` PNGs (no source sheet
   exists), whose output is the committed `terrain_types.json` entries.
3. **`world_view_ground.gd` has no texture source.** RESOLVED: `_draw` draws the surface from
   `solid_mask` (per-column rects); it has no texture source and needs none, and the AC1
   expected surface is derived independently from the tile PNG in the test.
4. **Two constants had no stated value.** RESOLVED: `GROUND_SNAP_PX = 16.0`,
   `EMBED_TOLERANCE_PX = 8.0`, stated in the per-tick section.
5. **The mask rule was stated narrower than the wiki's while claiming "the wiki's form".**
   RESOLVED: the divergence is stated explicitly; the mask is the wiki's per-pixel bitfield,
   and the fill-below clause is deliberately not adopted.

**MEASURED: alpha-first vs greyscale-threshold.** On `slope_up_1..6`, `air = (alpha == 0)` and
`grey` thresholding differ in only 1-5 pixels per tile, all on the anti-aliased diagonal edge
(for `slope_up_1`: (22,0), (15,4), (8,8), (1,12)). Alpha calls the edge pixel solid, greyscale
calls it air -- a 1 px surface difference at those columns, inside AC1's 1 px band. So the two
rules are near-equivalent for this art and both satisfy AC1; the pipeline is greyscale by
@me's choice.

## Plan gate

**SCORE: PENDING re-score.** The earlier 100/100 (2026-11-15, iteration 1) was scored against
the plan text, not the repo, and is SUPERSEDED: it survived while the plan's alpha premise, the
two `solid_mask` definitions, the missing schema path and the missing `slope_up_*` producer were
all wrong. Re-score against #126, bar 90.

CRITICAL (unsupported by ticket): none.

UNCOVERED (ticket clause with no plan item, or `required test: <class>`): none.

OTHER DEFICIENCIES: none.

REFINEMENTS: none.

Append above after evaluator passes (date, score, iterations, removed items with reasons).
