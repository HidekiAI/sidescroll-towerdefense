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
sample it at `pos_x`. The collision surface is then the tile's own silhouette. The DRAW surface
is the tile sprite (its alpha, per amendment section 3), a second reading of the same art: the
two agree except on the anti-aliased edge columns, where they differ by at most 1 px (MEASURED
below), inside AC1's 1 px band. They are no longer literally ONE datum -- that claim belonged to
the superseded alpha-as-detector draft, where the alpha WAS the detector:

- `editor/scripts/world_view.gd::step_walker` -- the live path.
- `crates/sstd-core/src/physics.rs::step_walker` -- the model behind the bridge's `walk_step`.

Anti-jitter is in scope: on penetration, snap the entity out and zero the offending velocity
component (`dX = dV/dT`, `dV = 0` -> `dX = 0`), never zero gravity.

## Ticket acceptance criteria (as amended, amendment §5)

- **AC1:** feet Y equals the drawn surface at every column, within 1 px. The 2026-11-15 amendment
  first worded this "drawn alpha surface"; the 2026-11-16 supersession re-based it on the
  greyscale-derived mask, and the drawn surface (the sprite) and the mask agree to within 1 px.
- **AC2:** **DROPPED** (2026-11-15 amendment §5) -- per-frame rise need not be constant; the
  surface is art-derived and uneven.
- **AC3:** `slope_feet_y_matches_drawn_surface` passes against the mask surface.
- **AC4:** flat ground and a single-tile step up are unchanged; `cargo test -p sstd-core` stays
  green.

## The pattern: ART -> DATA -> TECH (directive 2026-11-16)

**The slope ANGLE is irrelevant and is never an input.** The artist authored the climb for how
it FEELS; the family's measured rises are 7 to 30 degrees (MEASURED, `magick <tile> -colorspace Gray -depth 8 hex:-` row positions at pixel columns x=0 and x=31, `atan2(rise,31)*180/pi`: 22.8 / 30.1 / 14.5 / 30.1 / 7.4 / 30.1; `slope_up_0` is flat). So the tool
must NOT infer geometry from a nominal angle, a formula, or a hand-authored ladder. It scans
the ART for air versus collidable, turns that into DATA (a per-pixel collision bit-mask), and
TECH consumes the mask. Redrawing the tile re-derives the mask with no code change.

1. **Art.** The tiles the runtime walks are `slope_up_{0..6}_32x32.png`, committed under
   `editor/assets/tiles/`. `inject_simulator_hills` (main.gd:236,250) places them at rows
   `31 - int(4*i/7)`. `slope_up_0` is a flat solid cap; `slope_up_1..6` are diagonals.
   `slope_30deg_*` is a second family whose air is drawn opaque grey; this ticket does NOT walk
   it. **There is no source sheet for `slope_up_*`** (`14e6fc1` adds the seven PNGs and no sheet;
   `tools/import-tiles` slices sheets, and the `tools/extract-tiles` that `AGENTS.md` records as
   deleted has NO path in any ref -- UNVERIFIED, so nothing load-bearing rests on it), so the
   tool must be runnable on the committed tile PNGs themselves, not only on a sheet.
2. **Data.** The tool emits `solid_mask` per tile, in the wiki's form
   (`TDD_Collision-Map-Tooling` line 20): **one bit per tile pixel**, a `PackedByteArray` of
   `(w*h + 7) / 8` = 128 bytes for a 32x32 tile, bit `(row * w + col)`. `sub_tile_mask` stays
   as the coarse 4-bit predecessor.
3. **Tech.** Both steppers build the world surface from `solid_mask` (topmost set bit per pixel
   column) and sample it at `pos_x`.

## The air-vs-collidable rule (what the tool implements)

The tile's **greyscale is the detector**; the mask is the product. Measured on the committed art
(`magick ... -alpha off -colorspace Gray`; an alpha-INCLUSIVE histogram counts 68 for
`slope_30deg_0_0`, because one grey carries many alpha values, so alpha must be off to count
greys):

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
6. **Resolve by fuzzy logic and best guess; the flag is the LAST resort** (@me, 2026-11-17).
   Before flagging, the tool tries every heuristic it has and takes the best guess: the
   bottom-row anchor, the uniform-patch region colours, 4-connectivity, and the majority reading
   across the tile's edge samples. A guess is accepted when the heuristics agree, or when one
   reading dominates. A tile is flagged ONLY when every heuristic is inconclusive (they
   contradict each other and none dominates), and then the tool bakes a red 1px border into the
   emitted PNG so the artist or level-designer hand-edits the collision.

**Why the flag must exist (@me, 2026-11-16).** No single anchor is universally right: the
upper-left corner reads air for a slope-up but not a slope-down; walking bottom-to-top reads air
for a slope but not a cave ceiling. The uniform-patch sampling makes the colour read robust, the
fuzzy/best-guess stage resolves the rest, and the flag remains for the tiles where every
heuristic is inconclusive.

**Flag criteria (@me, 2026-11-17, superseding the strict 2026-11-16 rule).** A tile is flagged
only when ALL heuristics are inconclusive: the greyscale does not split into two clusters at all,
or it does split but every candidate anchor is contradicted by the connectivity/majority reading.
A single ambiguous sample -- for example both clusters touching the bottom row -- is now RESOLVED
by the best guess, not flagged. `slope_up_*` all resolve on the bottom-row anchor alone (MEASURED:
the 7 tiles have exactly two greys with one cluster on the bottom row; `slope_30deg_0_0` has 53
greys, `magick ... -alpha off -colorspace Gray`).

**Divergence from the wiki, stated (not claimed as "the wiki's form").** `TDD_Collision-Map-Tooling`
section 1 carries two clauses this ticket diverges from:

- "every pixel below the first solid pixel in a column is solid" (the fill-below clause) -- NOT
  adopted (@me chose specks-only). For the slope art the two agree, because there are no interior
  pockets.
- "a tile pixel is solid iff its alpha is not `0x00`" (the alpha datum) -- NOT adopted. The
  detector is the tile's GREYSCALE; alpha is the emitted PRODUCT, not the detector (the
  2026-11-16 supersession, above).

**Ticket traceability.** The greyscale detector, the bottom-row ground anchor, the specks-only
clean and the flag criteria supersede the 2026-11-15 amendment posted to #126
(https://github.com/HidekiAI/sidescroll-towerdefense/issues/126#issuecomment-6900011032), whose
section 1 ("solid iff alpha != 0x00") and section 2 (Option 1a colour-key) this replaces. The
supersession is posted as
https://github.com/HidekiAI/sidescroll-towerdefense/issues/126#issuecomment-6924849581, so the
decision is ticket-traceable, not only session-traceable.

**Later refinement (@me, 2026-11-17).** The flag is the LAST resort, not the rule: resolution is
fuzzy/best-guess first (pipeline step 6). This supersedes the 2026-11-16 amendment's "FLAGGED,
never guessed" wording for the resolution ORDER only; the flag itself stays, and the flagged tile
is still never machine-guessed. Recorded here; to be posted as a third comment on #126 so the
order is ticket-traceable too.

The ticket BODY's profile clause -- "`build_ground_column_table`
becomes a run-based profile: a slope run is one line from its start to its end, not seven
plateaus" -- was displaced by the 2026-11-15 amendment's per-pixel mask datum (a mask surface
yields 32 samples per tile, not one line); the 2026-11-16 comment then replaced the alpha
detector with GREYSCALE. The body's second clause ("flat ground and single-tile steps stay
step functions") still holds, because the mask surface is constant across a flat tile.

## Provenance (gathered, not assumed)

`git blame` on every block this plan changes, `gh issue view` 122/124/125/126, `gh pr list
--state all` (only merged #95/#97/#98; no PR for #122/#124/#125/#126, so the issue threads are
the review record), and the three design pages.

| Block being changed | Introduced by (commit, date, issue) | Decision it carries |
|---|---|---|
| `build_ground_column_table` (world_view.gd:532) | `09f3db5`, 2026-11-08, #122 | one surface per column, absence = hole; **retired here** |
| `step_walker` (world_view.gd:594) | `09f3db5`, 2026-11-08, #122 | landing tested against the column's plateau; **replaced here** |
| the interpolation hack (world_view.gd:628) | `47716b9`, 2026-11-10, #124 | spreads current->next across the column; **deleted here** |
| `world_view_ground.gd::_draw` | `:12` (function entry) / `:37-77` (polygon block) | `075b88e`, 2026-11-09, #122 | the 4:7 polygon's top edge was the "drawn surface" AC1 measured; **replaced by rendering the tiles' alpha (their sprites); the 4:7 polygon removed** |
| `ground_y_for_column` / `step_walker` (physics.rs:238 / :266) | `8809c55`, 2026-11-11, #125 (`step_walker` created `9c4d897`, 2026-10-25, #122; last fixed `bd84440`, 2026-11-12, #125) | Rust model has no within-column interpolation; **ported to the mask sampler** |
| `slope_feet_y_matches_drawn_surface` (test_walker_step.gd:309-314) | `47716b9`, 2026-11-10, #124 | caps a single-frame rise at 16 px, never compares to the drawn line; **rewritten to compare to the mask surface** |
| `_check_ground_profile` (test_slope_set.gd:122) | `28f0439`, 2026-11-09, #122 | pins the injected slope set; **re-pointed at the mask surface** |
| `inject_simulator_hills` (main.gd:217) | `45085fa`, 2026-11-09, #122 | injects the 7 `slope_up_*` tiles; **unchanged** (placement is not this ticket) |
| `_rebuild_ground_tiles` (world_view.gd:675) | `5685a2d`, 2026-10-25, #122 | builds `_tiles_cache`, hands it to the ground layer, rebuilds the table, spawns; **builds the mask profile here** |
| `_spawn_walker_on_ground` / `_place_walker_on_col` (world_view.gd:714 / :759) | `09f3db5`, 2026-11-08, #122 | read the column table to seat the walker; **read the profile now** |
| `auto_detect_sub_tile_mask` (tools/import-tiles/src/main.rs:354) | `904467f`, 2026-08-03, #33 | coarse 4-bit mask; **`solid_mask` added alongside it** |
| `default_package/terrain_types.json` (7 framework types) | `9d78881`, 2026-08-27, #67 | framework terrain set; **gains the 7 `slope_up_*` entries with `solid_mask`** |

Every hash above is the introducing commit from `git log -1 -L <line>,<line>:<file>`; the author on
every row is @me (the repo's only author), so the author is not repeated per row. Every line
number in this plan was re-MEASURED on 2026-11-17 with `grep -n` against the working tree at
`feat/ground-profile-impl` tip `b05704a` (e.g. `grep -n "func step_walker" editor/scripts/world_view.gd`
-> `594`); a line number not so measured is marked UNVERIFIED at the point of use.

## What exists today (the structures this builds on)

| Thing | Where | What it does |
|---|---|---|
| `build_ground_column_table(tiles, tile_h_px)` | `world_view.gd:532` | `{world_col: surface_row * tile_h_px}`; absence = hole. Retired |
| `step_walker(state, move_x, dt, ground_y_by_col, tile_w_px, ...)` | `world_view.gd:594` | gravity, landing, step up, fall damage, world clamp. Retired |
| the per-column interpolation hack | `world_view.gd:628` | spreads current->next across the column; the #124 sawtooth. Deleted |
| `_rebuild_ground_tiles()` | `world_view.gd:675` | builds `_tiles_cache`, hands it to the ground layer, rebuilds the table, spawns. Now builds the mask profile |
| spawn paths | `world_view.gd:714-771` | read the column table; read the profile now |
| `world_view_ground.gd::_draw` | `:12` (function entry) / `:37-77` (polygon block) | flat tiles as rects + a 4:7 ramp polygon; no texture source. Replaced by drawing the tile sprites (their alpha); the polygon is removed |
| `ground_y_for_column` / `step_walker` | `physics.rs:238` / `:266` | per-column rule, no interpolation. Ported to the sampler |
| `auto_detect_sub_tile_mask` | `tools/import-tiles/src/main.rs:354` | coarse 4-bit mask. `solid_mask` added beside it |
| `TerrainTypeDef` | `crates/sstd-core/src/terrain.rs:188` | the terrain-type struct (it carries `sub_tile_mask` at `:201`); gains `solid_mask`. `TerrainType` at `:7` is the Ground/Wall/Air enum, a different item |
| `WalkTile` -> `phys::Tile` | `sstd-editor-bridge/src/lib.rs:2300` | carries `world_tile_x/y`, `is_walkable`; gains `solid_mask` |
| `slope_up_*` placement | `main.gd:236,250` | terrain-key strings; walkable via the default heuristic at `world_view.gd:699`. No terrain entry today |

## Data model and build rule (identical on both sides)

- **`solid_mask` is a property of the tile ART, so it lives on the terrain type.**
  `TerrainTypeDef` (`crates/sstd-core/src/terrain.rs:188`) gains `solid_mask`; the framework
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

Amendment section 4 (anti-jitter) fixes the penetration arm and names the two constants:

```
GROUND_SNAP_PX = 16.0       // amendment §4: half a 32 px tile -- a grounded walker within this above the surface stays glued
EMBED_TOLERANCE_PX = 8.0    // amendment §4: a quarter tile -- the deepest penetration treated as contact before the snap

if surface is None:                                       -> hole; a grounded walker starts a fall
elif vel_y >= 0 and feet_was_y <= surface <= feet_now_y:   -> landed from above (fall damage applies)
elif surface < feet_was_y and move_x != 0:                -> step UP onto higher ground (landed, vy = 0, no fall damage)
elif feet_y > surface + EMBED_TOLERANCE_PX:               -> penetration: snap feet_y to surface, vy = 0, grounded, zero the offending velocity component
elif grounded and feet_y <= surface and surface - feet_y <= GROUND_SNAP_PX:
                                                           -> in contact, INCLUDING step DOWN onto lower ground (surface below the feet): glued to the surface, vy = 0
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
| `tools/import-tiles/src/main.rs` | add `derive_solid_mask(pixels, w, h) -> (mask, resolved)` (greyscale pipeline: uniform-patch colour read, bottom-row ground anchor, speck removal, then fuzzy/best-guess resolution before the flag); emit per-pixel `solid_mask` beside `sub_tile_mask`; bake a red 1px border into the emitted PNG and set `resolved = false` only when every heuristic is inconclusive; add a tile-scan mode that runs the derivation over a directory of committed `{key}_32x32.png` and writes/updates the 7 `slope_up_*` entries in `default_package/terrain_types.json` | AC1, art->data |
| `editor/default_package/terrain_types.json` | add `slope_up_0..6` with `is_walkable: true` and `solid_mask` (base64 128 bytes) | AC1, data home |
| `crates/sstd-core/src/terrain.rs` | `TerrainTypeDef` gains `solid_mask` (serde default keeps old files parsing) | AC1 |
| `crates/sstd-core/src/physics.rs` | `build_ground_profile`, `ground_y_at`, `GROUND_SNAP_PX`, `EMBED_TOLERANCE_PX`, the anti-jitter snap; `step_walker` samples the profile | AC1, AC4, amendment §4 |
| `crates/sstd-editor-bridge/src/lib.rs` | `WalkTile` gains `solid_mask` | AC1 |
| `editor/scripts/world_view.gd` | add `set_terrain_masks(masks_by_key)`, `build_ground_profile`, `ground_y_at`; `_rebuild_ground_tiles` resolves each tile's mask by terrain key; `step_walker` takes the profile and the anti-jitter snap; delete `build_ground_column_table`, the hack, `tile_w_px` | AC1, AC3, AC4, amendment §4 |
| `editor/scripts/main.gd` | wire `set_terrain_masks` from `terrain_editor.get_terrain_types()` to the simulator's world view | AC1 |
| `editor/scripts/world_view_ground.gd` | `_draw` draws each walkable tile's SPRITE (`res://assets/tiles/{key}_32x32.png`, `load()`ed once per key and cached) at its cell, per amendment section 3; the flat-tile rects and the 4:7 `draw_polygon` block are removed. Fog/atmosphere NEVER lowers a tile's alpha (that alpha IS the collider datum): it goes on its own render layer, and the game has no fog-of-war | AC1 |
| `editor/tests/test_walker_step.gd` | ground table -> profile; `slope_feet_y_matches_drawn_surface` compares feet Y to the mask surface derived INDEPENDENTLY from the tile PNG in the test, within 1 px | AC1, AC3 |
| `editor/tests/test_slope_set.gd` | `_check_ground_profile` asserts the mask-derived surface of the shipped run (AC1's subject, not AC4's flat-ground clause) | AC1 |
| `crates/sstd-core/src/physics.rs` tests | profile + anti-jitter + back-to-back tests (below) | AC1, AC4 |
| `tools/import-tiles/src/main.rs` tests | in-source `#[cfg(test)]` mask tests on synthetic RGBA images (below) | AC1 |

Wiki/docs (executed per the Always-Update-All-Documents rule, not scored):
`TDD_Collision-Map-Tooling` (greyscale pipeline; the mask as the collision data form; note the
specks-only divergence), `TDD_Physics` (the mask datum), `TDD_Simulator-Ground-Walk` §5/§7
wording, `PostMortem/Slope-Hopping-2026-11-10` (dated supersession), `Home.md` (redact the
4:7-polygon description).

## Tests (falsifiable, each pinned to a number, each with its mutant)

**Layer.** Every case below is a UNIT test: in-source `#[cfg(test)]` for `physics.rs` and
`tools/import-tiles`, and headless GDScript script assertions for the two editor suites
(`test_walker_step.gd` drives `_test_walker_step()` headlessly; `test_slope_set.gd` asserts the
injected set). No E2E/UI test is added by this ticket -- the walker is driven headlessly, not
through a rendered scene.

**Race / repeat.** Race does not apply: both steppers are pure synchronous functions on one
thread and the tool is a one-shot CLI. Non-determinism does not apply either: deterministic pure
functions, no seed or clock input, so no pin is needed. The back-to-back repeat is pinned by
`same_input_twice_gives_the_same_state`.

### GDScript `test_walker_step.gd`

| Test | Assertion | AC | Mutant |
|---|---|---|---|
| `solid_mask_surface_rule` | given a tile mask, `build_ground_profile` returns `row*tile_h + top_row` per column; a fully air column is absent | AC4 | M1 |
| `interior_air_below_surface_is_solid` | a column with solid at rows 31 and 35 answers 31, never 35 | AC4 | M1 |
| `slope_feet_y_matches_drawn_surface` | walker seeded at the run's first column centre; **sweeps EVERY pixel column of the injected run** (7 tiles x 32 px) and asserts feet Y equals the surface the test derives INDEPENDENTLY from the tile PNG, within 1 px at each column. The PNG-derived sweep is the AC1 oracle; the 7 hand-measured centres (992/996/978/960/941/928/904) and 7 seams (992/1005/987/968/950/932/914) are a SECOND, hand-derived pin, so a bug shared by the tool and the test's derivation cannot pass unseen. The 13 px notch at col 61 is asserted, not tolerated. Never the 4:7 ladder | AC1, AC3 | M2, M5 |
| `slope_descent_matches_drawn_surface` | walked LEFT, grounded every ramp frame, feet Y equals the mask surface within 1 px, hp unchanged | AC1 | M2 |
| `penetration_snaps_out_and_zeroes_velocity` | walker forced 5 px into a surface has feet snapped to the surface and `vel_y == 0` next frame; no oscillation over 10 frames | amendment §4 | M5 |
| `ground_snap_threshold_is_under_one_tile` | surface 16 px below glues; 32 px below becomes airborne | AC1, AC4 | M4 |
| `blocked_by_wall_retries` | a grounded walker refused at an air segment for 120 frames does not creep | AC4 | M7 |
| `ground_layer_draws_sprites_not_a_polygon` | the `world_view_ground.gd` source contains no `draw_polygon` and does load a `_32x32.png` sprite, so the 4:7 ladder cannot silently return | amendment §3 | M8 |

Existing `falls_then_lands`, `stays_on_ground`, `falls_through_void`, `clamps_left`,
`clamps_right`, `blocked_by_wall`, `deterministic`, `hill_climb`, `hill_fall_damage` stay
green; fixtures move from a column dictionary to tile arrays with mask-derived surfaces.

`editor/tests/test_walker_visibility.gd` is an EXISTING suite (MEASURED present, 2026-11-17) that
instantiates `world_view.tscn` and checks layout framing at two viewport sizes. It is not changed
by this ticket, but `world_view.gd` is, so it is a stays-green regression guard and stays in the
Gate. `test_walker_visibility.gd` also still uses the legacy `hill_up`/`hill_down` keys, which is
why those keys must not be deleted here.

### GDScript `test_slope_set.gd`

| Test | Assertion | AC | Mutant |
|---|---|---|---|
| `_check_ground_profile` | the injected 7-tile run's seam surface equals the mask-derived seam table (992/1005/987/968/950/932/914), so the placement and the mask agree | AC1 | M1 |

### Rust `physics.rs`

| Test | Assertion | AC | Mutant |
|---|---|---|---|
| `solid_mask_surface_rule` | topmost solid row per column; fully air column => `None` | AC4 | M1 |
| `interior_air_below_surface_is_solid` | rows 31 and 35 => 31, never 35 | AC4 | M1 |
| `profile_coalesces_adjacent_holes` | three adjacent air columns => one Air run | AC4 | M3 |
| `walking_up_a_monotonic_ramp_tracks_the_surface` | a SYNTHETIC monotonic ramp, `move_x = 1.0` at 200 pps: `pos_y` equals the surface sample every ramp frame (within 1e-6), strictly decreasing | AC1 | M2 |
| `shipped_run_surface_is_reproduced_including_the_notch` | the shipped 7-tile run: feet Y equals the surface at every column, and the 992 -> 1005 px notch at world col 61 is reproduced. NO monotonicity claim | AC1, AC3 | M2 |
| `walking_down_the_ramp_grounded_without_damage` | mirrored walk leftward: grounded every frame, hp unchanged | AC1 | M2 |
| `penetration_snaps_out_and_zeroes_velocity` | 5 px penetration snaps to surface, `vel_y == 0`, stable over 10 ticks | amendment §4 | M5 |
| `snap_threshold_is_under_one_tile` | 16 px glues, 32 px does not | AC1, AC4 | M4 |
| `same_input_twice_gives_the_same_state` | back-to-back repeat pin | AC4 | M6 |

Existing `stepping_up_onto_a_higher_surface_does_not_start_a_fall` (physics.rs:593) pins AC4's
single-tile step-up; the GDScript equivalent is the `hill_up` step inside `_test_walker_step()`
(test_walker_step.gd:251). Both stay green.

### Tool `tools/import-tiles` (in-source `#[cfg(test)]`, synthetic RGBA images)

| Test | Assertion | AC | Mutant |
|---|---|---|---|
| `mask_two_cluster_bottom_anchor` | a synthetic RGBA tile with air above and ground at the bottom: the mask sets exactly the ground pixels | AC1 | T1 |
| `mask_all_grey_is_all_solid` | a single-grey tile: every bit set, `resolved = true` | AC1 | T2 |
| `mask_removes_floating_solid_specks` | a solid speck surrounded by air becomes air; the ground component survives | AC1 | T3 |
| `mask_best_guess_resolves_ambiguous_sample` | a tile whose bottom row carries both clusters is RESOLVED by the best guess (connectivity/majority), `resolved = true`, no red border | AC1 | T6 |
| `mask_flags_only_when_all_heuristics_inconclusive` | a tile whose greyscale does not split into two clusters at all returns `resolved = false`, and its emitted PNG has a red 1px border | AC1 | T4 |
| `mask_uniform_patch_ignores_edge_pixel` | a 2x2 uniform patch reads the region colour where a lone anti-aliased edge pixel would not | AC1 | T5 |

### Mutation battery

| Mutant | Expected red |
|---|---|
| M1 surface reads bottommost instead of topmost row | `solid_mask_surface_rule`, `interior_air_below_surface_is_solid`, `_check_ground_profile` |
| M2 lookup returns the column value with no interpolation | `slope_feet_y_matches_drawn_surface`, `walking_up_a_monotonic_ramp_tracks_the_surface`, `shipped_run_surface_is_reproduced_including_the_notch` |
| M3 air column returns 0.0 instead of None | `falls_through_void`, `profile_coalesces_adjacent_holes` |
| M4 GROUND_SNAP_PX raised to 32 | `ground_snap_threshold_is_under_one_tile`, `snap_threshold_is_under_one_tile` |
| M5 anti-jitter arm deleted | `penetration_snaps_out_and_zeroes_velocity` (both sides) |
| M6 profile rebuilt non-deterministically (iteration order) | `same_input_twice_gives_the_same_state` |
| M7 refusal sets grounded = false | `blocked_by_wall_retries` |
| M8 the 4:7 `draw_polygon` restored in `world_view_ground.gd` | `ground_layer_draws_sprites_not_a_polygon` |
| T1 bottom anchor replaced by the top row | `mask_two_cluster_bottom_anchor` |
| T2 all-grey tile treated as all air | `mask_all_grey_is_all_solid` |
| T3 speck removal deleted | `mask_removes_floating_solid_specks` |
| T4 ambiguity flag never set | `mask_flags_only_when_all_heuristics_inconclusive` |
| T5 colour read from a single pixel | `mask_uniform_patch_ignores_edge_pixel` |
| T6 best-guess stage deleted (an ambiguous sample is flagged instead of resolved) | `mask_best_guess_resolves_ambiguous_sample` |

Every test above has a named mutant that reddens it, and every mutant reddens at least one
test. A mutation is validated before it is interpreted (it must fire and parse, else the
experiment is void).

## Gate (run before any commit)

1. `cargo test -p sstd-core` (full suite, not a filtered name) and `cargo test -p import-tiles`
   (the tool's in-source mask tests), each gated on its EXIT CODE.
2. `test_walker_step`, `test_slope_set`, `test_walker_visibility`, each gated on its EXIT CODE
   (MEASURED 2026-11-17: `test_walker_step` exits 0 on Godot 4.7.2). `test_screen_store` is
   known red on 4.7.2 (#85) and is NOT in this gate.
3. **Run the mutation battery** (table above): apply each mutant, confirm it PARSES and FIRES,
   then confirm the named test goes RED. A mutant that does not fire, or a test that stays green,
   is a void experiment, not a pass. This step is what makes the suite a guard rather than a claim.
4. The `slope_up_*` entries in `default_package/terrain_types.json` are verified by the tool's own
   tile-scan run, not by a `slope_30deg_*` colour-key (that family is not walked; it DOES have a
   source sheet, `editor/assets/samples/tileset_2-2.png`, whereas `slope_up_*` has none).

Run the FULL suite, not only the new tests: this repo has repo-wide guard tests (the magic-number
scanner and the terrain-count tests), and a filtered run hides them.

## Risks

- The GDScript `step_walker` signature changes (table removed, profile added). Callers are
  `world_view.gd::_update_walker` and `test_walker_step.gd`, both in the Files table.
- `ground_y_at` past the last column clamps to the last value, keeping `clamps_right` green.
- The tool's air heuristic has the two named failure modes (slope-down, cave ceiling). The
  fuzzy/best-guess stage (step 6) resolves most such tiles; the red-border flag is the guard for
  the rest, and it is a visual signal only: a flagged tile is not machine-detectable at runtime,
  by @me's choice.
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
   solid row under the greyscale rule, placed at rows `31 - int(4*i/7)`. A slope tile's surface
   moves across its 32 pixels, so the tile's **left-edge pixel column (x=0, the seam)** and its
   **centre (x=16)** are both tabulated:

   | world col | 60 | 61 | 62 | 63 | 64 | 65 | 66 |
   |---|---|---|---|---|---|---|---|
   | tile | `slope_up_0` | `_1` | `_2` | `_3` | `_4` | `_5` | `_6` |
   | surface px at x=0 (seam) | 992 | 1005 | 987 | 968 | 950 | 932 | 914 |
   | surface px at x=16 (centre) | 992 | 996 | 978 | 960 | 941 | 928 | 904 |

   Both rows are jagged and non-monotonic: a 13 px DOWNWARD notch at col 61 on the seam (a 4 px
   one at the centre), then boundary jumps. **AC1 is the bar and is accepted as written** -- the
   walker reproduces the drawn surface including the notch. Fixing the art/placement is a
   separate ticket. Test expectations are the measured numbers above, never the `4:7` ladder.
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
6. **Resolution is fuzzy/best-guess FIRST; a red 1px border baked into the emitted PNG is the
   LAST resort** (@me, 2026-11-17), so the artist or level-designer hand-edits the collision only
   when every heuristic is inconclusive. A single ambiguous sample is resolved, not flagged. The
   flag stays in this ticket.
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
3. **`world_view_ground.gd` has no texture source.** RESOLVED: `_draw` renders the tiles
   themselves (their alpha, i.e. the tile sprites), per amendment section 3; the 4:7 polygon
   is removed; the AC1 expected surface is derived independently from the tile PNG in the test.
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

## Third skeptical pass (2026-11-16) -- plan-gate findings

Run-by-run scores are in the Plan gate section below; each finding, and its resolution:

1. **Decision 1's table was labelled "topmost solid row per tile" but held the tile's LEFT-EDGE
   (x=0, seam) values, and the test cross-checked it at column centres.** RESOLVED: the table now
   carries both the seam (x=0) and centre (x=16) rows, labelled; the test cross-checks the centre
   row at the centres and pins the seam row at x=0.
2. **The ticket body's "run-based profile ... one line from its start to its end" was never
   cited.** RESOLVED: the traceability paragraph now quotes it and states the 2026-11-16
   supersession.
3. **Provenance omitted `_rebuild_ground_tiles` and the spawn functions.** RESOLVED: two rows
   added (`5685a2d` 2026-10-25; `09f3db5` 2026-11-08).
4. **`_check_ground_profile` had no Tests row and no mutant, and was cited to AC4.** RESOLVED: it
   has its own `test_slope_set.gd` row with AC1 and mutant M1.
5. **The `world_view_ground.gd` draw source.** RESOLVED: `_draw` renders the tiles' alpha
   (the sprites), per amendment section 3; the 4:7 polygon is removed, stated in the Files row.
6. **The per-tick block and the two constants had no citation, and the third arm was labelled
   "step up / step down" while conditioned on step-up only.** RESOLVED: the constants cite
   amendment §4; the arm is labelled step-UP, and the in-contact arm is labelled as also serving
   step-down.
7. **`step_walker` was cited at `physics.rs:134-264`.** RESOLVED: the signature is `:153`, and the
   row names the introducing (`9c4d897`) and last-fixing (`bd84440`) commits.
8. **`tools/extract-tiles` "was deleted" is unverifiable.** RESOLVED: marked UNVERIFIED, with
   `14e6fc1` cited for the load-bearing half (no source sheet).
9. **`slope_30deg_0_0` "53 distinct greys" was called wrong (68).** REJECTED as a false alarm:
   53 is the count with alpha IGNORED, which is the plan's own rule; 68 counts alpha variants of
   one grey. The table caption now states the alpha-off basis so it cannot be re-flagged.

## Plan gate

**STATUS: NOT READY — bar 90 (awaiting the next evaluator run).** Evaluations so far: iter-1
(2026-11-15) 100/100 (plan-only, false positive); iter-2 (2026-11-16) 52/100 (alpha-datum
contradiction); iter-3 (2026-11-16) 55/100 (draw contradiction, stale refs, unasserted polygon
removal). The 2026-11-17 revision clears the iter-3 findings (list below) and folds in the
2026-11-17 fuzzy/best-guess directive. The next evaluator run scores this revision.

**Amendments cited:**
- 2026-11-15T19:29:17Z (alpha datum, Option 1a colour-key, AC1/AC3/anti-jitter/bar-90)
- 2026-11-16T18:08:38Z (greyscale detector supersedes alpha for walked family; Option 1a
  NOT used; AC1/AC3/anti-jitter/bar-90/4:7-polygon-removed unchanged)
- 2026-11-17 (session directive, to be posted as a third #126 comment): resolution is
  fuzzy/best-guess FIRST and the red-border flag is the LAST resort; the flag stays in this
  ticket. Supersedes the "never guessed" ORDER only, not the flag itself.

**Findings from iteration 3 (2026-11-16), each now resolved in this revision:**
- AC1 "every column" not tested -> RESOLVED: `slope_feet_y_matches_drawn_surface` sweeps every
  pixel column against the PNG-derived oracle, with the hand-measured centres+seams as a second pin.
- Amendment section 3 polygon removal unasserted -> RESOLVED:
  `ground_layer_draws_sprites_not_a_polygon` (mutant M8), and the draw source is named.
- Provenance line numbers unlabelled -> RESOLVED: every number re-MEASURED 2026-11-17 (`grep -n`),
  stated in the provenance note; stale refs corrected (world_view.gd 532/594/628/675/714-771/699,
  physics.rs 238/266/593, terrain.rs 188/201, main.gd 217, test_walker_step.gd 251/309-314).
- Decision 6 flag "belongs in a follow-up" -> REJECTED: the 2026-11-16 amendment §1 names the flag,
  so it is in-scope; the 2026-11-17 directive keeps it as the last resort. Cited, not deferred.
- "`test_walker_step.gd` exits 1 on 4.7.2" -> STALE: MEASURED exit 0 on 4.7.2 (2026-11-17).

Append above after evaluator passes (date, score, iterations, removed items with reasons).
