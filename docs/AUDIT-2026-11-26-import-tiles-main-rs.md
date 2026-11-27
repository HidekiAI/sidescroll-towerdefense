# Whole-file legibility audit: `tools/import-tiles/src/main.rs`

Date: 2026-11-26. Scope: every function in the file, 2,962 lines, 86 functions
(58 production, 28 in-module tests). Trigger: @me rejected two undocumented
contracts in one session (a GDScript sentinel return and a `void` "check"
helper) and required the whole Rust file audited before further edits.

Legend: CRITICAL = the caller cannot learn a failure mode from the function;
MEDIUM = fix when the block is next touched; LOW / no action = name or
surrounding code already carries the meaning.

## A. Return contracts and sentinels

- `778 remove_floating_specks(&mut [u8], u32, u32)` — CRITICAL. Its comment
  states the flood algorithm but not the precondition that seeds come from the
  bottom row, nor the failure mode when that row has no solid pixel: the
  entire ground mass is cleared. This is the exact defect that produced the
  transparent `r6`/`l5` tiles, and the caller cannot see it at the function.
- `320/348/393/421 fn_find_content_*_edge` — u32 coordinates, no sentinel; OK.
- `748 uniform_patch_grey`, `1249 fn_clear_grid_lines` — u8 grey / u32 count;
  no sentinel; OK.
- No `return -1` anywhere in Rust. `NO_SOLID_ROW` is GDScript-only and now has
  its contract stated at the returning function.

## B. Void functions (production)

- `778 remove_floating_specks` — see A.
- `931 flag_unresolved(&mut RgbaImage)` — comment states the red 1px border and
  that the interior is never repainted; adequate.
- `1464 print_usage()` — MEDIUM: no intent comment; state that it prints to
  stderr and does not exit.
- `1814 fn_write_terrain_types` — comment states sort-by-key and catalog write;
  adequate.
- `1924 fn_emit_tile_sets` — comment states the "only when authored" gate;
  adequate.
- `1959 main()` — body has block comments; adequate.
- Test helpers `2112 paint_grey`, `2468 fn_paint_unresolved_cell`,
  `2489 fn_paint_grid_lines` carry doc comments; `#[test]` names carry intent.

## C. Missing intent comments

- `300 col_has_content` — MEDIUM: siblings at 254 and 281 have doc comments;
  this one does not.
- `461 detect_content_bounds` — MEDIUM.
- `573 auto_detect_terrain_key` — MEDIUM: 20 inner comments but no
  function-level intent on a 160-line body.
- `1166 fn_merge_terrain_entries` — MEDIUM.
- `1479 parse_args` — MEDIUM.
- `130/133/136/139/142/162` serde defaults — LOW / no action: the function
  name is the intent.
- `2056 average_color` and the in-module `#[test]` functions — no action:
  test names carry intent.

## D. Oversized functions (one screen ≈ 90 lines)

- `573 auto_detect_terrain_key` — 160 lines, 20 inner comments.
- `1593 fn_build_grid` — 93 lines, 5 comments.
- `1697 fn_emit_tiles` — 117 lines, 8 comments.
- `1962 fn_run_convert_slope` — 94 lines, 16 comments; local to `main`.
- The earlier `162-253` hit was a false positive: struct declarations, not a
  function body.

## E. Banned short identifiers (all pre-existing, out of scope unless touched)

- 25 `|e|` closures at lines 987, 995, 1002, 1119, 1150, 1153, 1177, 1357,
  1360, 1376, 1429, 1431, 1444, 1451, 1548, 1550, 1568, 1583, 1825, 1951,
  1968, 1970, 1978, 2025, 2033 — rename to `error` / `io_error`.
- `1010 .sort_by(|a, b| ...)` — rename to `left, right`; the sibling at 1814
  already uses the named form.

## F. Magic numbers

- `140 default_luminance_threshold() -> 0.5` — MEDIUM: every other cutoff in
  this file is a named const; this one is a bare literal.
- `1505/1511/1521/2004 exit(0)` and `exit(1)` — flag: no named exit-status
  form exists in this crate. Per the no-magic-number rule, ask before adding
  one instead of silently choosing.
- `1985/1989/1995/2000 .nth(1..4)` — argv indices into a fixed layout;
  MEDIUM, pre-existing in `main`.
- `815/816/834 [u32; 256]`, `862/863 half-width and bottom row`, `1486 = 1`,
  `1302 [0,0,0,ALPHA_AIR]`, zero defaults at 965/969/971, 1782/1791/1793,
  1907/1908 — LOW / no action: byte range, geometry, identities.

## G. if/else producing one value (would read as `match`)

- `870 if bottom_is_dominant { (dominant, opposite) } else { ... }` — MEDIUM.
- `1074 if cell.flip_h { flip_horizontal(&resized) } else { resized }` —
  MEDIUM.

Both are pre-existing tail expressions; restyle only if their block is in
scope for this ticket.

## Gate status

Ledger complete. No further production edit to `main.rs` until @me chooses
scope. The pending converter fix is item A.
