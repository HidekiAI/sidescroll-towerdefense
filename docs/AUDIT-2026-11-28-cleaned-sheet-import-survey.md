# Audit: generic importer on the cleaned sheets (2026-11-28)

## TL;DR

The generic importer cannot produce production-quality output from the 17 cleaned sheets in
`editor/assets/samples/fixed/` in auto-bounding mode. Its pitch detector reads the first blank
column or row as a cell pitch, which yields pitches of 1-3 px and up to 90,005 cells on some
sheets. With explicit geometry, the two documented sheets are workable, but only the slope sheet
has a hand-edited collision reference that proves the masks. The grass sheet additionally needs a
lower luminance threshold than the sample config uses, and its documented cell size is not
representable by the importer's single scalar `margin`. Detail below.

## Scope and method

Survey command: `/tmp/user/1000/opencode/sstd-auto-detect-survey.sh`. It ran
`target/debug/import-tiles` once per cleaned sheet with `tile_size_px: 32`, `auto_bounding: true`
and no authored grid. Raw stdout, stderr and config files are in
`$HOME/Downloads/slope-clean-up/production-survey-auto/`. Every run exited 0; the exit codes were
checked from the script, not from printed pass lines.

The baseline for the hand-edited reference is the final slope run recorded in
`docs/PLAN-2026-11-25-slope-conversion-tool.md`: slope geometry is a 4x2 grid, offset `(104, 65)`,
region `126x126`, separation `(1, 1)`, pitch `127`, tile `32x32`, threshold `0.45`.
The cleaned slope sheet is `editor/assets/samples/tileset_2-2-cleaned.png`.

## Measured auto-bounding results

| Sheet | Detected pitch | Detected grid | Emitted tiles |
|---|---:|---:|---:|
| `tileset_0-0.png` | 256x44 | 9x25 | 225 |
| `tileset_0-1.png` | 256x256 | 2x2 | 4 |
| `tileset_0-2.png` | 256x214 | 3x2 | 6 |
| `tileset_0-3.png` | 203x133 | 2x2 | 4 |
| `tileset_1-0.png` | 249x1 | 2x295 | 590 |
| `tileset_1-1.png` | 256x4 | 3x85 | 255 |
| `tileset_1-2.png` | 256x256 | 3x2 | 6 |
| `tileset_1-3.png` | 2x256 | 351x2 | 702 |
| `tileset_2-0.png` | 2x256 | 350x2 | 700 |
| `tileset_2-2.png` | 256x218 | 3x2 | 6 |
| `tileset_3-0.png` | 256x1 | 3x322 | 966 |
| `tileset_3-1_2x2.png` | 256x256 | 3x2 | 6 |
| `tileset_3-1_misaligned.png` | 3x1 | 235x383 | 90,005 |
| `tileset_3-1.png` | 256x256 | 3x2 | 6 |
| `tileset_3-2.png` | 256x256 | 3x2 | 6 |
| `tileset_3-3.png` | 256x256 | 3x2 | 6 |
| `tileset-dirt-combined.png` | 256x1 | 2x257 | 514 |

Nine sheets land on a 3x2 or 2x2 grid, but their detected pitch varies from 203 to 256 px and no
authored metadata confirms those grids. Eight sheets emit a pitch of 1, 2, 3, 4 or 44 px in one
axis; the 3x1 case emits 90,005 tiles from a 704x383 image. `detect_content_bounds`
in `tools/import-tiles/src/main.rs:489-518` picks the first column or row with no greyscale
variation inside the content rectangle. On art with transparent or flat-colour seams that is a
blank pixel inside one cell, not a cell boundary, so the pitch collapses. This is a wrong-output
defect in the shared detector, not a bad config on one sheet.

## Explicit-geometry results

The slope sheet with the documented grid and the alpha-aware helper emitted eight masks with
0 of 8,192 differing bits against the hand-edited reference; the run is in
`$HOME/Downloads/slope-clean-up/fixed-source-final/out`. No PNG contains a `tIME` chunk; the
`date:create` values reported by ImageMagick are filesystem timestamps, not embedded art data.

The grass sheet `tileset_1-0.png` was run with `tools/import-tiles/sample_config.json` geometry
(21x12 grid, offset `(3, 3)`, tile `32x32`) at five thresholds. Raw stdout is in
`$HOME/Downloads/slope-clean-up/grass-threshold-sweep/`.

| Threshold | Mask histogram |
|---:|---|
| 0.45 | 252 `0x0` |
| 0.35 | 239 `0x0`, 2 `0x1`, 1 `0x2`, 10 `0x3` |
| 0.25 | 221 `0x0`, 2 `0x1`, 1 `0x2`, 13 `0x3`, 2 `0x7`, 3 `0xB`, 5 `0xC`, 2 `0xD`, 3 `0xF` |
| 0.15 | 188 `0x0`, 8 `0x3`, 56 `0xF` |
| 0.10 | same as 0.15 |

An alpha-thresholded occupancy map of the cleaned sheet shows one 8-column by 8-row block of
opaque cells starting at grid column 7, row 2; all other cells are fully transparent. The opaque
cells have per-cell Rec.601 luma from 0.180645 to 0.387526 when transparent pixels are excluded.
The sample threshold 0.45 therefore labels every cleaned cell as air. A threshold of 0.15 emits
56 solid and 8 partial masks, which is plausible for that block but has no hand-edited reference
and is not a default the tool documents.

## Two-sheet production run

@me chose the two runnable documented sheets. The script
`/tmp/user/1000/opencode/sstd-production-two-sheets.sh` ran them with explicit geometry and
recorded exit codes from the process, not from printed lines. Raw stdout and stderr are in
`$HOME/Downloads/slope-clean-up/production-two-sheets/`.

| Run | Exit | Masks |
|---|---:|---|
| slope, threshold 0.45 | 0 | 3 `0x0`, 1 `0x1`, 1 `0x3`, 1 `0x7`, 1 `0xA`, 1 `0xF` |
| `tileset_1-0.png`, threshold 0.45 | 0 | 252 `0x0` |
| `tileset_1-0.png`, threshold 0.15 | 0 | 188 `0x0`, 8 `0x3`, 56 `0xF` |

No emitted PNG contains a `tIME` chunk. The slope masks match the hand-edited reference set, but
the PNGs are not byte-identical to the two-phase conversion-tool output: four of eight cells differ
by 30-32 alpha pixels out of 1,024 when compared cell by cell. The other four are alpha-identical.
All eight differ in RGB because the generic importer crops the full 127 px pitch, while the
conversion tool crops the authored 126 px region before resizing. The alpha difference is a second
effect of that crop, not a mask-verdict difference.

## Limitation found in the config model

The wiki's documented `tileset_0-0.png` geometry is margin `(420, 237)`, region `492x508` and
separation `(3, 3)`, so its source pitch is `495x511`. `ImportConfig` has one scalar `margin`,
and the authored-grid branch of `import_spritesheet` sets both source pitches to
`tile_size_px + margin`. The importer cannot represent that rectangular cell without changing its
config contract. No run was attempted against that sheet with an incorrect square pitch.

The same branch crops `source_pitch_w x source_pitch_h`, not the authored region. For the slope,
that is `127x127` instead of `126x126`, which accounts for the RGB and partial-alpha differences
in the two-sheet run. A config can express the pitch but not the region size and separation
independently.

## Verdict

- Auto-bounding mode: not production-quality on these cleaned sheets. Do not consume its output.
- Slope sheet with explicit geometry: acceptable at the coarse-mask level for the eight cells;
  four cells differ from the conversion-tool alpha by 30-32 pixels and all eight differ in RGB, so
  it is not a drop-in replacement for the two-phase artifacts.
- Grass `tileset_1-0.png` with explicit geometry: not production-quality at the sample threshold;
  the threshold needs a documented per-sheet decision and a reference mask before its output can
  be trusted.
- The other 14 sheets: no authored grid exists in the code repo or the design wiki. They need a
  per-sheet geometry decision before any production run.

## Tracked findings

- Issue #260 (auto-bounding detects 1-4 px cell pitches) was filed with @me's approved wording and
  is OPEN. It covers the pitch-collapsing defect measured above.
- The region-versus-pitch crop difference is recorded in this audit but is not a separate issue:
  @me approved only the pitch-detector issue in this pass.

## Open decision

Choose the target set for the production run: all 17 cleaned sheets with per-sheet geometry
supplied by @me, only the two runnable sheets with existing metadata (the slope and
`tileset_1-0.png`), or stop at this survey and file follow-up issues. Do not merge
`fix/slope-mask-reference-pass` or close issue #239 (two-phase slope source-art preprocessing)
without explicit approval.
