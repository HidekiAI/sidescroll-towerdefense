# Session: #68 parallax restack (research + design, 2026-09-30)

Handoff scratch. **The record is `docs/SESSION-CHECKPOINT.md`** (`## CURRENT STATE /
NEXT MOVE`) and the plan is `docs/PLAN-2026-09-30-parallax-tutorial-stack.md`. Do not
duplicate either here; this file only points at them and records the few things that are
not in either.

## State

Research and design complete, **zero code written**. Both branches were merged `--ff-only`
onto their base branches and pushed on 2026-09-30:

- code repo `feat/parallax-tutorial-stack` -> `trunk`: plan file,
  `tools/slice-layers/README.md`, checkpoint + AGENTS updated
- wiki `docs/parallax-tutorial-model` -> `master`: TDD revised, GDD Parallax
  section, `TDD_Parallax-Restack-2026-09-30` decision record, `Home.md` rows,
  `TODO.md` TS71 + DD7

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
- The pre-push self-review found **two claims in my own docs were false**: the Godot
  runtime (both AGENTS files said 4.4.1; the `godot4` symlink had moved to 4.7.2 on
  2026-09-29) and the asset size. Re-measuring instead of trusting the memory turned up
  a real red — issue #85 — that had nothing to do with this work. **Lesson: a baseline
  number written from memory is a claim, not a measurement.** Re-run the gate before
  recording its result, and always state which runtime it came from.
- Rewording an **older** commit's subject with `git commit --amend` amends HEAD, not
  the commit you named. It silently gave a commit the wrong subject. The safe path on an
  unpushed branch is a scripted reword: `GIT_SEQUENCE_EDITOR` to mark the line
  `reword` plus `GIT_EDITOR` to sed the message. Verify the diffstat afterwards is
  byte-identical to before the reword.

## Resume in one line

Open the checkpoint's CURRENT STATE block, work the numbered "Next move" list from
step 5 (Phase 1, `tools/gen-backdrop` via the `coding-assistant` skill). Steps 1-4 are
done: designed, reviewed, committed, pushed and merged.
