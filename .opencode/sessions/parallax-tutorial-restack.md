# Session: #68 parallax restack (research + design, 2026-09-30)

Handoff scratch. **The record is `docs/SESSION-CHECKPOINT.md`** (`## CURRENT STATE /
NEXT MOVE`) and the plan is `docs/PLAN-2026-09-30-parallax-tutorial-stack.md`. Do not
duplicate either here; this file only points at them and records the few things that are
not in either.

## State

Research and design complete, **zero code written**. Two branches open, documentation
committed, nothing pushed:

- code repo `feat/parallax-tutorial-stack` (off `trunk`): plan file,
  `tools/slice-layers/README.md`, checkpoint + AGENTS updated
- wiki `docs/parallax-tutorial-model` (off `master`): TDD revised, GDD Parallax
  section, `Home.md` row, `TODO.md` TS71

Issue #68 stays **OPEN** — design revised, implementation pending.

## Not recorded elsewhere

- The **`coding-assistant` skill can actually run here**, which was worth verifying
  before committing to it. `$NVIM` is set and the window inventory has a non-terminal
  editor window (1002) beside the opencode TUI terminal (1000), which is exactly the
  shape the skill's `luaeval` opener expects. `code` is on `PATH` but no VS Code process
  is running, so nvim remote is the path in use. If a future session cannot reach
  `nvim --server "$NVIM"`, fall back to plain implementation and say so rather than
  silently substituting.
- **GDScript has no LSP fail-fast equivalent** to what the skill's fail-fast rule
  assumes. The adaptation used: an explicit return type the body cannot produce. The
  project rule already requires explicit types on every signature, so it is free.
- The two defects found (§ "Two defects a GREEN probe could not see" in the checkpoint)
  were found by *reading the scene file*, not by running anything. The probe had been
  GREEN for weeks. Reading the scene against the tutorial's own documented rules was
  what surfaced both.

## Resume in one line

Open the checkpoint's CURRENT STATE block, work the numbered "Next move" list from
step 3 (push permission), and start at Phase 1 of the plan file.
