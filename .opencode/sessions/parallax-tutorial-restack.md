# Session: #68 parallax restack (research + design, 2026-09-30)

Handoff scratch. **The record is `docs/SESSION-CHECKPOINT.md`** (`## CURRENT STATE /
NEXT MOVE`) and the plan is `docs/PLAN-2026-09-30-parallax-tutorial-stack.md`. Do not
duplicate either here; this file only points at them and records the few things that are
not in either.

## State

Design merged and pushed 2026-09-30. **Phase 1 code has since started** — see the
checkpoint; this file's "zero code written" below is now stale for that reason only.

- code repo `feat/parallax-tutorial-stack` -> `trunk`: plan file,
  `tools/slice-layers/README.md`, checkpoint + AGENTS updated
- wiki `docs/parallax-tutorial-model` -> `master`: TDD revised, GDD Parallax
  section, `TDD_Parallax-Restack-2026-09-30` decision record, `Home.md` rows,
  `TODO.md` TS71 + DD7
- code repo `feat/gen-backdrop-crate` (off `trunk`, LOCAL, UNPUSHED): the `gen-backdrop`
  crate with its blocks still open

Issue #68 stays **OPEN**.

## Not recorded elsewhere

- The **`coding-assistant` skill can actually run here**, which was worth verifying
  before committing to it. `$NVIM` is set and a non-terminal editor window sits beside
  the opencode TUI terminal, which is exactly the shape the skill's `luaeval` opener
  expects. `code` is on `PATH` but no VS Code process is running, so nvim remote is the
  path in use. If a future session cannot reach `nvim --server "$NVIM"`, fall back to
  plain implementation and say so rather than silently substituting.
- **The nvim remote channel has sharp edges, measured.** `--remote-expr` evaluates Lua
  here and works. `--remote-silent` combined with `|` mis-parses and **creates junk
  buffers named after the command text** — always check `getbufinfo()` and clean up
  (`mod=0` is the tell). `vim.*`, `string`, `table`, `map`, `range` are **not** in the
  sandbox; only bare vimscript functions (`bufnr`, `bufname`, `execute`, `normal!`,
  `feedkeys`) work. `line("$")` reads the *current* window regardless of what you mapped
  over, which silently misreports every window in a loop — use `len(getbufline(winbufnr(v),
  1, "$"))` instead. `normal!` fails with "Can't re-enter normal mode from terminal mode"
  while the terminal window is focused; `wincmd p` (not `wincmd 1`, which raises E471) is
  what moves focus back to the editor window. **When the file is rewritten on disk,
  `checktime` does not visibly update the user's view** — tell them to refresh.
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
- **Lesson from the block-format correction:** I ran the skill's own verification grep
  during the original scaffold, got zero, and labelled the output with the conclusion I
  wanted ("not the convention here"). A check I had just decided to bypass produced the
  evidence against my own deviation, and I read it as confirmation. When a verification
  returns something inconvenient, the first move is to check whether the check is wrong,
  not to explain away the result.
- Rewording an **older** commit's subject with `git commit --amend` amends HEAD, not
  the commit you named. It silently gave a commit the wrong subject. The safe path on an
  unpushed branch is a scripted reword: `GIT_SEQUENCE_EDITOR` to mark the line
  `reword` plus `GIT_EDITOR` to sed the message. Verify the diffstat afterwards is
  byte-identical to before the reword.
- **`git checkout -- <file>` to undo a bad edit is a trap when the file is on an
  unpushed branch with uncommitted work.** It discarded a full rewrite (the block-format
  restoration) that had to be redone from the transcript. If an edit goes wrong, fix it
  forward; if a revert is genuinely wanted, commit first so the content is recoverable.

## Resume in one line

Open the checkpoint's CURRENT STATE block and work the numbered "Next move" list from
step 5 (Phase 1, `tools/gen-backdrop` via the `coding-assistant` skill). Steps 1-4 are
done: designed, reviewed, committed, pushed and merged.

