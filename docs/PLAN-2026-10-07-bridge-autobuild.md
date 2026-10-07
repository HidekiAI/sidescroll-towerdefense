# PLAN 2026-10-07 — pre-Run Rust bridge auto-build (issue #103)

Ticket: https://github.com/HidekiAI/sidescroll-towerdefense/issues/103 (no comments; body of
2026-10-07 is the whole ticket). The issue body carries the design rationale and grounding; this
plan is the file-level implementation plan gated against that ticket by the plan-ticket-gate
skill (result recorded in "Plan gate" at the bottom).

## Items — each cites the ticket clause it serves

1. **`.gitignore` rule split, including the comment that describes it** — ticket *Grounding*
   bullet 5: `.gitignore:46` is `editor/addons/` and "the rule must become
   `editor/addons/*` + `!editor/addons/bridge_autobuild/`, otherwise the plugin stays
   untracked". The comment block directly above that rule currently states the addons are
   third-party and "never authored here", which the mandated exception makes false; the
   comment changes together with the rule it describes (the 516 MB vendored rationale stays).
2. **`editor/addons/bridge_autobuild/plugin.cfg`** — ticket *What*: plugin lives at that path;
   format (name/description/author/version/script keys) mirrors the vendored sibling
   `editor/addons/godot_mcp_toolkit/plugin.cfg` (preplanning: existing pattern, not invented).
3. **`editor/addons/bridge_autobuild/plugin.gd`** — ticket *Design* pseudo-code: `@tool`,
   `extends EditorPlugin`, `static func build_bridge() -> bool` holding the command
   construction exactly as written there (`PATH` export, `&&` chain, `cd` to
   `globalize_path("res://").path_join("..")`, `exec scripts/build-bridge.sh 2>&1`,
   synchronous `OS.execute(..., read_stderr=true)`, `[bridge-autobuild]`-prefixed line
   echo, `push_error` + `false` on non-zero exit, `build OK (N ms)` on success), and the
   one-line `_build()` delegation the same section specifies.
4. **`editor/project.godot` enable entry** — ticket *Grounding* bullet 4 (the
   `[editor_plugins] enabled` array is empty) + *Verify* step 1 ("listed and enabled"): set
   `enabled=PackedStringArray("res://addons/bridge_autobuild/plugin.cfg")`.
5. **`editor/tests/test_bridge_autobuild.gd`** — ticket *Test* paragraph: runnable headless,
   preloads `plugin.gd`, calls the static `build_bridge()` twice with a >1 s sleep between,
   asserts both return `true` and that the `.so` mtime strictly advanced from the first
   build's baseline. Harness shape (4-space indent, `check()`, `_initialize` ->
   `_run.call_deferred()`, completion guard, `quit(0 if _failures == 0 else 1)`) is taken from
   `editor/tests/test_override_merge.gd` (preplanning: existing pattern).
6. **Headless verification of the implemented plugin** — ticket *Verify* steps enumerated:
   (a) step 1: plugin.cfg + project.godot enable entry make "Bridge Auto-Build" appear
   enabled in Project Settings -> Plugins (checked from the files, plus @me's editor run);
   (b) step 2: headless test proves `build_bridge()` returns `true`; the Output-prefixed
   build lines are observed on the headless run's stdout (@me's manual counterpart: build
   lines visible in the editor Output before the game window opens);
   (c) step 3: the test's second build proves a rebuild happens on demand and the `.so` mtime
   updates (`Compiling` line visibility in the editor is @me's visual check);
   (d) step 4 failure path: @me breaks the build manually; the headless test covers the
   complementary direction via the ticket *Test* paragraph's non-vacuity claim — mutation
   checks that the test goes red when `build_bridge()` is no-op'd (mtime assertion) and when
   it returns `false` (return-value assertion).
7. **`.opencode/AGENTS.md` repo-hygiene bullet update** — ticket *Grounding* bullet 5 changes
   the exact `.gitignore:46` line the AGENTS bullet documents ("Verified by
   `git check-ignore -v editor/addons/`, which resolves to `.gitignore:46`", "is therefore
   clean with respect to addons"); the bullet's cited rule form and line number shift with
   item 1 (exception path + rule now `editor/addons/*` at `.gitignore:47`), and doc
   consistency with that mandated change is the item's support.

## Grounding record (already executed, no work remains)

- Ticket *Grounding* bullet "Headless-testability probe": throwaway
  `editor/tests/tmp_probe_editorclass.gd` ran and informed the static-func design; it is
  never committed.

## Possible follow-ups (not this ticket)

- Register `test_bridge_autobuild.gd` in the AGENTS.md four-suite gate list — the ticket asks
  for the test file, not for expanding the standing gate.
- `EditorProgress` + `Thread` non-blocking build — ticket *Design* names it only as the
  upgrade path behind the deliberate `OS.execute` ceiling.
- Windows support for the bash-based command — ticket is Linux-targeted (repo OS bias).

## Addendum — found by executing the plan, after the gate passed

8. **`scripts/build-bridge.sh`: atomic replace of the installed `.so`** — running item 6's
   headless guard (the ticket's *Test* scenario with two real builds) crashed Godot with
   exit 139 (SIGSEGV), reproducibly. Root cause: `cp` over `editor/rust/libsstd_editor_bridge.so`
   truncates-and-rewrites the inode that the current process has mmap'd — the headless
   guard here, and **the editor itself in production**, since the editor loads the bridge at
   startup — so teardown/hot-reload reads replaced pages. Fix: `cp` to a `.tmp` file then
   `mv` (rename swaps the directory entry; the old inode stays valid for mapped consumers,
   new processes open the fresh inode). Verified: exit 0 after the fix, mtime still strictly
   advances across builds. Serves the ticket's *Test* paragraph (the guard must be
   headless-runnable, therefore the build must not kill the process that runs it) and
   *Verify* steps 2-3 (the editor must survive every Run's rebuild).

## Repo process (applies to every change here; outside the ticket's scope)

- Re-run the four gated GDScript suites after the editor change (repo rule: "all four must
  pass after any editor change"), against the documented known-red baseline
  (`test_screen_store` exits 1 on Godot 4.7.2, issue #85; the other three exit 0). The
  ticket's *Out of scope* names these suites as not-to-be-worked-on; running them is a
  push gate, not ticket work.
- Branch `feat/bridge-autobuild`, Conventional Commits subject with `ref #103`, local commit
  first; push only after @me's permission (branch-then-merge).

## Plan gate

- Date: 2026-10-07, plan-ticket-gate, MIN_SCORE 90, MAX_ITERATIONS 3.
- Iteration 1: **59/100** — CRITICAL (hard cap): item 9 "branch/commit mechanics" is repo
  process, no ticket clause supports it. Also: four-suite re-run inside item 8 justified only
  by repo rule; item 1's comment amendment weakly cited; probe record padded the item list;
  Verify section referenced wholesale instead of per-clause.
- Iteration 2: **98/100** — passed. Changes: item 9 + four-suite re-run moved to the "Repo
  process" footnote; probe moved to "Grounding record (already executed)"; item 1 cites the
  comment it invalidates; Verify enumerated (a)-(d). Refinements applied: 6(b) softened to
  observed-stdout wording, manual @me counterpart appended.
- Iteration 3: **98/100** — passed, after executing the plan surfaced a defect (exit 139:
  `cp` over the mapped bridge `.so`) that became Addendum item 8 and the ticket's "Shipped
  change" sub-bullet. Same-score re-gate: no CRITICAL, no UNCOVERED. Refinement applied:
  item 7 quotes the AGENTS bullet it keeps true.
- Items removed from ticket-traced scope: branch/commit mechanics, four-suite re-run (both
  retained as repo process, still executed). No ticket scope dropped.
