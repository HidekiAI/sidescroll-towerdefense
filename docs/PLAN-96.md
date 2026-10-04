# PLAN-96: Fix inability to switch back from Simulator tab (#96)

**Ticket:** [#96](https://github.com/HidekiAI/sidescroll-towerdefense/issues/96) (OPEN)  
**Parent:** [#89](https://github.com/HidekiAI/sidescroll-towerdefense/issues/89) (related fix)  
**Branch:** `feat/fix-96-tab-switching` (create from `trunk`)  
**Status:** Pre-coding. Plan must pass `plan-ticket-gate` before implementation.

## 1. TL;DR

After switching to Simulator tab, cannot switch back to other tabs. Root cause is likely `simulator.gd::_frame_backdrop()` awaiting `get_tree().process_frame` inside `_ready()` (async `_ready`) and/or signal connections interfering with TabContainer tab switching. Fix: defer backdrop framing until the tab becomes visible (use `visibility_changed` or `tab_changed`/defer), avoid awaiting in a way that blocks tab activation, and ensure early exit if not visible before awaiting. Also ensure no input-consuming nodes block tab navigation.

## 2. Phase 0: Investigation & Evidence (GATE - must complete before implementation)

**This is a hard gate. Do not proceed to Phase 1 until all items are completed and evidence recorded.**

1. [ ] **Reproduce**: Run editor, switch to Simulator tab, attempt to switch back to other tabs. Record whether repro occurs.
2. [ ] **Capture console**: Capture full console output during repro (including warnings/errors). Save to `docs/evidence/PLAN-96/console-repro.txt`.
3. [ ] **Record baseline**: Document timing/state when stuck. Note any error messages, focus state, or TabContainer behavior.
4. [ ] **Create evidence dir**: `mkdir -p docs/evidence/PLAN-96`
5. [ ] **GATE PASSED**: Record evidence in `docs/SESSION-CHECKPOINT.md` under `## #96 investigation (gate)` and only then proceed to Phase 1.

## 3. Root cause (grounded in code)

**Relevant code:**
- `editor/scripts/simulator.gd:15-35` `_ready()` calls `_frame_backdrop(strip)` (line 22) which does `await get_tree().process_frame` (line 113 in `_frame_backdrop`). Godot 4 allows async `_ready()`; awaiting a frame on tab activation can delay readiness but usually doesn't block TabContainer. However, the tab is a `Control` under `TabContainer`; if something grabs focus or if there's an issue with signal connections firing during tab transition, it may appear stuck.
- `simulator.gd:26-27` connects `resized` and `get_viewport().size_changed` in `_ready()`; both call `_frame_backdrop` (which awaits). These connections persist; if viewport/resize fires during tab switch, it could trigger awaits.
- `main.tscn`: Simulator tab is `Control` with `layout_mode=2` (line ~810). BackdropStrip and scrollbars (HScrollBar/VScrollBar, `z_index=20`) are children; scrollbars are focusable Controls by default. Normally TabContainer switching isn't blocked by child Controls unless something captures input/focus improperly.

**Hypothesis:** Awaiting in `_ready()` on first tab selection can sometimes interact poorly with TabContainer's internal state; also calling `_frame_backdrop` immediately before connections may cause layout thrashing. Deferring to next frame via `call_deferred("_frame_backdrop", strip)` or handling `visibility_changed` when tab becomes visible is safer. Also scrollbars don’t block tab switching via TabContainer shortcuts, but focus could remain inside tab.

**Repro notes from #96:** Tab becomes stuck after entering Simulator; cannot return to other views. Need to test in editor and check console for errors (Godot prints warnings/errors; #96 mentions checking console).

## 3. Investigation (required)

1. Run editor, switch to Simulator, try to switch back; capture console output (any errors/warnings). Look for script errors, await-related issues, focus issues.
2. Check if `_ready()` is completing (coroutine). Async `_ready()` returns a GDScriptFunctionState in theory, but in practice runs; awaiting process_frame is fine.
3. Test with minimal change: replace immediate `_frame_backdrop(strip)` call in `_ready()` with `call_deferred("_frame_backdrop", strip)`. Also consider deferring connections or making `_frame_backdrop` non-blocking for first layout? Or skip await if not visible?
4. Check focus: when Simulator tab becomes active, does a child (scrollbars) take focus? TabContainer navigation should still work (Ctrl+Tab or clicking other tabs). If mouse_filter/processing interferes, unlikely.

## 4. Fix candidates (safest first)

**Candidate A (minimal, safe):** Defer initial framing to avoid awaiting during tab activation.
- In `_ready()`, replace `_frame_backdrop(strip)` with `call_deferred("_frame_backdrop", strip)`. The `resized`/viewport signals still call it (may await later, but after tab is active). Rationale: layout is more stable after tab is shown.

**Candidate B (visibility-aware):** Only frame when visible; connect `visibility_changed` to reframe when becomes visible. Prevents work while hidden.
- Add `visibility_changed.connect(func(): if is_visible_in_tree(): call_deferred("_frame_backdrop", strip))` and call deferred on first become-visible rather than in `_ready()` immediately. Use `is_visible_in_tree()` consistently for all visibility checks; avoid bind-style ambiguity.

**Candidate C (early-exit before await):** In `_frame_backdrop`, check `is_visible_in_tree()` **before** `await get_tree().process_frame()` and return early if not visible/in tree. Avoid unnecessary awaits when hidden.
- Reorder: check visibility first, then proceed to measure/layout; skip await when not needed.

**Candidate D (check input/focus):** Ensure scrollbars don’t interfere; set `mouse_filter` appropriately or ensure no `focus_mode` issue blocking. Unlikely root cause but defensive.

Prefer A+B+C combined: defer initial call, visibility-aware reframe using `is_visible_in_tree()`, and early-exit before await in `_frame_backdrop`.

## 5. Implementation

### 5.1 `editor/scripts/simulator.gd`
- In `_ready()`:
  - Get strip as before.
  - Replace immediate `_frame_backdrop(strip)` with `call_deferred("_frame_backdrop", strip)`.
  - Keep `resized` and `viewport.size_changed` connections (they fire on size changes; awaiting process_frame is fine post-activation).
  - Add `visibility_changed.connect(func(): if is_visible_in_tree(): call_deferred("_frame_backdrop", strip))` to reframe when tab becomes visible again. Use `is_visible_in_tree()` consistently; avoid bind-style ambiguity.
- In `_frame_backdrop(strip: Node2D)`:
  - **Early exit before await:** if not `is_visible_in_tree()`, return immediately (no `await get_tree().process_frame()`).
  - Otherwise proceed with layout measurement as before.
- No other changes (scrollbar logic unchanged).

Rationale: deferring moves the first `await get_tree().process_frame` to after tab activation completes, avoiding potential tab-switching race in TabContainer.

### 5.2 Alternative/defensive
- If still stuck, try removing await? But `_frame_backdrop` needs layout (rect size). Can restructure: read size immediately, if zero, schedule retry with `get_tree().create_timer(0)` or just rely on `resized` signal later. But current approach is deliberate per comments.

## 6. Verification

1. Open editor, switch to Simulator tab. Verify can switch back to other tabs immediately.
2. Switch back and forth multiple times; no stuck state.
3. Resize window while in Simulator; framing still correct.
4. Scrub scrollbars; parallax still works (regression check for #89).
5. Run GDScript tests (same as #89): `test_image_to_map`, `test_terrain_brush`, `test_override_merge`, `test_screen_store` — record results. Note `test_screen_store` baseline on 4.7.2 per workspace (unchanged).
6. Check console for new errors/warnings; attach evidence (console-repro.txt and any test logs) to `docs/evidence/PLAN-96/`.
7. Include before/after evidence from Phase 0 in verification artifacts.

## 7. Acceptance criteria

- [ ] Can switch from Simulator to any other tab and back without getting stuck (repro fixed)
- [ ] Framing still correct at 1920x1080 and 1280x800
- [ ] Scrollbars functional, parallax correct (no regression of #89)
- [ ] All existing GDScript tests still pass (exit 0)
- [ ] No new console errors introduced

## 8. Exit checks

```bash
cd /home/hidekiai/projects/SSTD/sidescroll-towerdefense
cargo test -p sstd-core
cargo build
$HOME/bin/godot4 --headless --path editor --script res://tests/test_image_to_map.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_terrain_brush.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_override_merge.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd
```

## 9. Out of scope
- Deep TabContainer focus management beyond the framing/await timing issue
- Scrollbar input behavior changes (z_index/focus) unless necessary
