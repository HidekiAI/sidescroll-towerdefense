# Parallax Step 4 — Drawer + scene seam (#107)

**Epic:** #68. **Ticket:** #107. **Predecessor:** step 3 (#106). **Scope:** add `ParallaxDraw` (GDScript) that consumes `SstdBridge.parallax_placements` to render the layered parallax, wire it into the editor scene, and delete `editor/tests/probe_parallax_scroll.gd`.

## 1. Goal

Deliver the draw seam: from a stack + camera + view, get placements from the bridge and draw the layers (back-plane first, front last, in `draw_order`). Delete the stale probe script. The scene must not be left broken.

## 2. Scope

**In:**
- `editor/scripts/parallax_draw.gd`: a new `@tool` or runtime node that manages a `Sprite2D` per placement (a tilemap-like strip, using `TextureRect`/`Sprite2D` with `region_rect` and `repeat`), draws copies left-to-right starting at `first_copy_left_px`, and applies `screen_top_px` for vertical offset. 
- `editor/scenes/main.tscn`: wire `ParallaxDraw` into the Simulator/Backdrop area so placements are actually consumed. 
- Delete `editor/tests/probe_parallax_scroll.gd` (step 4 owns it; per checkpoint this is the right time). 

**Out:** anything that would re-enable the deleted PNG paths (`tile_grid_display.gd`'s `_BACKDROP_PATHS`) — leave that to step 5 if needed. No core changes. No gen-backdrop changes.

## 3. References
- `TDD_Parallax-Scrolling.Math.md` (drawing rules, back/front, draw_order)
- Step 3: `crates/sstd-editor-bridge/src/parallax_placements` contract
- `editor/tests/test_parallax_bridge.gd` for the wire shape

## 4. Acceptance criteria
1. `ParallaxDraw` calls `SstdBridge.parallax_placements` with the stack JSON + view state + art facts; renders the returned placements correctly.
2. Draw order: back-plane layers first, front last; within each, by `draw_order`.
3. `probe_parallax_scroll.gd` removed.
4. Existing tests still pass; no regressions.
5. Scene wiring doesn’t break headless parse/check-only.
6. **G1 (design-of-record, step 3→step 4):** `ParallaxDraw.last_placements()` equals the bridge result for the same `ViewState` (uses bridge results only, no recomputation). Verified by `test_parallax_draw.gd`.

## 5. Exit conditions
- `parallax_draw.gd` added and parse-clean (`--check-only`).
- `main.tscn` updated without syntax errors.
- `probe_parallax_scroll.gd` deleted.
- `editor/tests/test_parallax_draw.gd` added and exits 0 under headless (verifies G1 and basic draw behavior).
- All gates pass (Rust + GDScript: four existing suites + `test_parallax_backdrop` + `test_parallax_bridge` + `test_parallax_draw`).

## 6. Implementation notes
- For each placement, create sprites at `x = first_copy_left_px + i*period_px` for i in [0..copy_count). 
- Set texture from the art path given (must exist in the project; use `load()` and handle null defensively).
- Vertical: `global_position.y` += `screen_top_px` or set `offset.y`. 
- Plane/back-to-front: render back before front; draw_order inside plane controls order among same plane.
- Clear previous sprites on re-draw (on camera/view change).
- Keep headless-safe (no input). 

## 7. Completion record
_To be filled when landed._

