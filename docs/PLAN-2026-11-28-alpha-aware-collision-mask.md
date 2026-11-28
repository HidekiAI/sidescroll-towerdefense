# Plan: alpha-aware coarse collision masks (2026-11-28)

## Problem and evidence

TL;DR: `auto_detect_sub_tile_mask` averages RGB luma while ignoring alpha, so transparent art pixels
change collision masks in both directions; fix the shared helper, not either call site. Detail below.

The hand-cleaned slope sheet `editor/assets/samples/tileset_2-2-cleaned.png` was run through the
generic importer with slope geometry (4x2 grid, offset `(104,65)`, pitch `127`, tile `32x32`,
threshold `0.45`) twice. The control run used an alpha-blind mutant of the helper; the fixed run
used the committed change. Both exited 0:

| run | eight emitted masks |
| --- | --- |
| alpha-blind control | `0x0` x5, `0x1`, `0x5`, `0xD` |
| alpha-aware fixed | `0x0` x3, `0x1`, `0x3`, `0x7`, `0xA`, `0xF` |

Raw stdout and the restore check are in `$HOME/Downloads/slope-clean-up/alpha-real-ab/compare.log`.
Four of eight cells differ: transparent pixels carrying stale RGB were counted as ground, and
transparent pixels with dark RGB diluted solid neighbours. The source file was restored
byte-identically after the control run.

A separate A/B on `tileset_1-0.png` with the repo's `sample_config.json` is **not** evidence for this
defect: the cleaned and opaque sheets both emitted 252 `0x0` masks after the fix because the cleaned
cells are genuinely air or have Rec.601 luma below `0.45`. Its logs remain in
`$HOME/Downloads/slope-clean-up/generic-proof{,-original}/stdout.log`.

## Root cause, grounded in existing code

`auto_detect_sub_tile_mask` in `tools/import-tiles/src/main.rs:536` averages Rec.601 luma over every
pixel of each quadrant and ignores alpha. Cleaned art keeps RGB under transparent pixels: bright
stale RGB there reads as ground, while dark RGB there pulls a mostly solid quadrant below
`luminance_threshold`. The sibling
`auto_detect_terrain_key` in the same file already reads alpha through `TRANSPARENT_ALPHA_CUTOFF`
(128), so the collision helper is the inconsistent one. This is the same alpha-blindness already
recorded for slope Pass I in `docs/PLAN-2026-11-25-slope-conversion-tool.md`; that mode is covered
by its optional reference pass and is out of scope here.

## Design of record

- Owner: `tools/import-tiles/src/main.rs`. The function `auto_detect_sub_tile_mask` is shared by
  the spritesheet-slice call at line 1907 and the tile-scan call at line 2046. One edit there fixes
  both modes; neither call site changes.
- Per quadrant, accumulate luma only from pixels with `alpha >= TRANSPARENT_ALPHA_CUTOFF` and
  divide by that solid-pixel count. A quadrant with no solid pixels reads air. Fully opaque
  sheets keep the current mean and therefore the current mask.
- Reuse the existing `TRANSPARENT_ALPHA_CUTOFF`; add no new constant and no new dependency.
  Complexity stays O(tile pixels), with two scalar counters per quadrant.
- This is a pure function with no clock, randomness, shared state or persisted side effect, so
  race and back-to-back repeat cases do not apply. No E2E is needed: the in-source unit guards call
  the same function both production call sites call.

## Guards, RED first

1. `sub_tile_mask_ignores_transparent_air_quadrants`: stale bright RGB under alpha 0 must not make
   the top two quadrants solid; opaque bottom quadrants must stay solid.
2. `sub_tile_mask_all_transparent_is_air`: a fully transparent tile with bright RGB reads `0x0`.
3. `sub_tile_mask_partial_transparency_keeps_solid_quadrant`: a quadrant half transparent and half
   opaque with the same bright RGB yet stays solid, so transparent rows cannot dilute the mean.
4. `sub_tile_mask_opaque_sheet_keeps_existing_verdict`: opaque bright tile `0xF`, opaque dark tile
   `0x0`, so the fix does not change the old sheets.

Guards 1 to 3 are verified RED against the current helper; guard 4 is a backward-compatibility
pin that is green before and after. A mutation battery
then removes the alpha guard, divides by all pixels instead of solid pixels, and inverts the alpha
cutoff; each mutant must redden its named guard and restore byte-identically. Gates are read from
exit codes, never printed pass lines.

## Verification and publication

- `cargo test -p import-tiles`, `cargo test -p sstd-core`, `cargo build`, `git diff --check`.
- A/B the hand-cleaned slope sheet against the alpha-blind control; confirm four of eight masks
  change and the source file restores byte-identically. Repeat the original opaque
  `tileset_1-0.png` config and confirm its histogram stays `49x0x0 / 7x0xA / 196x0xF`.
- Commit on the current feature branch with explicit paths, then push by explicit remote name;
  no merge and no issue close without approval.

## Mini-checkpoints

- [x] Write guards and see them RED. Resume: `bash /tmp/user/1000/opencode/sstd-alpha-gates.sh`.
  Evidence: guard run exit 101, then 26 passed / 0 failed after the fix. Recovery: guards and fix
  are in `tools/import-tiles/src/main.rs`; do not restyle other functions.
- [x] Mutate and restore. Resume: `bash /tmp/user/1000/opencode/sstd-alpha-mutations.sh`.
  Evidence: `failures=0`, all three mutants fired and restored byte-identically. Recovery: the
  script keeps a backup in `/tmp/user/1000/opencode/sstd-alpha-main.rs.orig` and traps restore.
- [x] Prove the defect on real art. Resume:
  `bash /tmp/user/1000/opencode/sstd-alpha-real-ab.sh`.
  Evidence: `alpha-real-ab/compare.log` shows four of eight masks changed and `source_restored=yes`.
  Recovery: the control is an alpha-blind mutant, not a checkout of another commit.
- [x] File the bug issue in @me's own wording. Resume: ask "How would you like to word this?",
  then post only the approved final text with `gh issue create --repo HidekiAI/sidescroll-towerdefense`.
  Evidence: issue #259 (alpha-blind collision masks), created from the approved draft; @me said LGTM.
  @me then removed the "I may be wrong" hedge from that wording: a bug filed by @me about @me's own
  code needs no humble hedge. The issue body was edited to match before the commit was published.
  Recovery: no issue exists until that approval.
- [ ] Commit and push. Resume: stage only `tools/import-tiles/src/main.rs`,
  `docs/PLAN-2026-11-28-alpha-aware-collision-mask.md`, `docs/SESSION-CHECKPOINT.md`; commit
  referencing the new issue; ask once before `git push origin fix/slope-mask-reference-pass`.
  Evidence: commit hash and remote tip in the checkpoint. Recovery: no merge and no issue close.
