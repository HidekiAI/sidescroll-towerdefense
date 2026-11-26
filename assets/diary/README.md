# assets/diary - retained review previews

A dated visual diary of what was actually eyeballed during the slope work. Kept deliberately so
progress reports and post-mortems can reference those pixels later. These are reference images,
not build output.

| File | Generated | What it shows |
| --- | --- | --- |
| `sstd-slope-grass-30deg-4x2-preview.png` | 2026-11-25 | Montage of the eight committed 32x32 tiles named by `editor/assets/tilesets/slope_grass_30deg_4x2.tres`, scaled 8x with 2 px spacing on `#404040`; 1040x520. Used to inspect the slope group before implementation. |
| `sstd-slope-cell-0-0.png` | 2026-11-25 | One 126x126 cell cropped from the source sheet `editor/assets/samples/tileset_2-2.png` at margin (104,65), pitch 127. Used to confirm the source grid geometry for issue #178 (grass slope conversion tool). |

The montage was rendered from tracked tile PNGs and the cell crop from the tracked source sheet,
so both can be regenerated without anything under `/tmp`.
