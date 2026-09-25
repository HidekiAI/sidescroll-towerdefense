# PLAN: fix the inverted exit code in two test runners (#80)

> Date: 2026-09-25. Issue: [#80](https://github.com/HidekiAI/sidescroll-towerdefense/issues/80).
> Follows [PLAN-2026-09-25-override-merge-guard-hermetic](PLAN-2026-09-25-override-merge-guard-hermetic.md),
> which surfaced this while verifying gates.
> Status: plan written before code.

## TL;DR

`test_image_to_map.gd:63` and `test_terrain_brush.gd:70` both end with
`quit(1 if failures == 0 else 2)` — the condition is inverted, so the runner exits
non-zero when the suite **passes**. Both files currently print `failures=0` and exit 1.
Correct the ternary to `quit(0 if failures == 0 else 1)` in both, and record the exit-code
convention in `editor/tests/README.md`, which currently documents only the older
"never calls `quit()`" form and so does not describe either of the three idioms now in
the tree. Two characters of code, one doc paragraph, no new tests — the exit code of the
run **is** the test for this fix.

## 1. The defect

```gdscript
# editor/tests/test_image_to_map.gd:63  and  editor/tests/test_terrain_brush.gd:70
quit(1 if failures == 0 else 2)
```

Truth table:

| `failures` | suite result | current exit | correct exit |
|---|---|---|---|
| 0 | PASS | **1** | 0 |
| >0 | FAIL | 2 | 1 |

Both outcomes are non-zero, so the exit code carries no information. This survived
because a genuinely failing test also exits non-zero, which satisfies an exit-code
check *for the wrong reason* — a CI gate keyed on exit status would have looked correct
throughout.

Observed 2026-09-25, both suites fully passing:

```
test_image_to_map : PASS (failures=0) (9 assertions)     exit=1
test_terrain_brush : PASS (failures=0) (14 assertions)   exit=1
```

## 2. Root cause: convention drift, not a typo in isolation

`editor/tests/README.md:13` documents the convention as:

> The runner is a `SceneTree` script (it does **not** call `quit()` on load), so the
> process exits itself when the suite finishes. A `failures=0` summary line means all
> checks passed.

So the documented design is: no `quit()`, and the **printed line** is the pass signal.
The two files below were later given a `quit()` call — a reasonable change, since
"always exit 0" is worse than "exit the right code" — but they were not covered by the
README, and the ternary was written inverted. An audit of every `quit()` in
`editor/tests/` finds three live idioms:

| file | form | meaning |
|---|---|---|
| `test_screen_store.gd:51` | `quit(0 if _failures == 0 else 1)` | correct |
| `test_override_merge.gd:243` | `quit(1 if _failures > 0 else 0)` | correct |
| `test_image_to_map.gd:63`, `test_terrain_brush.gd:70` | `quit(1 if failures == 0 else 2)` | **inverted** |
| `repro_blank_world.gd:16` | `quit(failures)` | exit code = failure count; fine for a repro script, not a suite |

Probes and other helpers (`probe_*.gd`, `profile_*.gd`, `capture_backdrop.gd`) exit 0 on
success and 1 on a hard abort, which is consistent enough to leave alone. The rest of
this plan does not touch them.

**Deliberately not doing:** extracting a shared test harness so the exit convention
lives in one place. There is no shared helper today — each runner carries its own
`check` / `_failures` / `_oks` — and introducing one means reworking four suites to fix
a two-character defect. That is a refactor with its own justification, not a bug fix.

## 3. The fix

1. `editor/tests/test_image_to_map.gd:63` — `quit(1 if failures == 0 else 2)` becomes
   `quit(0 if failures == 0 else 1)`.
2. `editor/tests/test_terrain_brush.gd:70` — same edit.
3. `editor/tests/README.md` — state that a suite runner calls `quit()` with 0 on success
   and non-zero on failure, that the **exit code is authoritative** and the printed
   `failures=0` line is a human-readable summary, and that the current correct form is
   `quit(0 if _failures == 0 else 1)`. Keep the existing note about the runner not
   calling `quit()` on *load*, which is a separate and still-true statement.

Using the `test_screen_store.gd` form verbatim keeps the suites textually identical at
the exit line, so the next reader greps one shape.

4. `editor/tests/test_override_merge.gd:243` was already semantically correct but
   written inverted, as `quit(1 if _failures > 0 else 0)`. Behaviour is identical, so
   this is cosmetic — but the point of documenting a convention is that it be greppable,
   and a second shape defeats that. Normalized to `quit(0 if _failures == 0 else 1)`.

## 4. Why no new test

The regression test for an exit-code bug is the exit code. The runnable check is:

```sh
~/bin/godot4 --headless --path editor --script res://tests/test_image_to_map.gd; echo "exit=$?"
~/bin/godot4 --headless --path editor --script res://tests/test_terrain_brush.gd; echo "exit=$?"
```

Both must print `failures=0` **and** `exit=0`. Adding a wrapper that shells out to
assert on an exit code would be a second test harness to maintain, to test a one-line
contract that `echo $?` already checks. That is the check, and it is runnable.

The negative direction is covered by the same command: if either suite's assertions
regress, `failures` becomes non-zero and the exit code must become non-zero. Verified
by mutation in §6 rather than trusted.

## 5. Acceptance criteria

1. `test_image_to_map.gd` prints `failures=0` and exits 0.
2. `test_terrain_brush.gd` prints `failures=0` and exits 0.
3. Forcing a failure in either suite makes the exit code non-zero. This is the criterion
   that distinguishes a real fix from flipping the constants; it is checked by mutation.
4. `test_screen_store.gd` and `test_override_merge.gd` still exit 0, unchanged.
5. `cargo test -p sstd-core` still reports 110 passed. No `.rs` is touched.
6. `editor/tests/README.md` documents the exit-code convention and no longer implies
   that `quit()` is unused in runners.

## 6. Verification plan

| # | Action | Expected |
|---|---|---|
| 1 | run both suites, unmutated | `failures=0`, exit 0 |
| 2 | break an assertion in each file, in turn | `failures>0`, exit non-zero |
| 3 | run the other two suites | unchanged, exit 0 |
| 4 | `cargo test -p sstd-core` | 110 passed |
| 5 | `rg -n "quit\(" editor/tests/test_*.gd` | all four suites on one shape |

Mutation 2 is the part that matters. Without it, "exit 0" and "always exit 0" are
indistinguishable, and the fix would be a guess in the same way the original defect was.

## 6a. Verification results

All executed 2026-09-25. The two fixed files were **backed up to `/tmp` before
mutating, not restored with `git checkout`** — at that point the fix was uncommitted, so
`git checkout --` would have reinstated the broken ternary.

| # | Action | Expected | Actual | Verdict |
|---|---|---|---|---|
| 1 | both suites, unmutated | `failures=0`, exit 0 | `test_image_to_map` 9 ok exit 0; `test_terrain_brush` 14 ok exit 0 | pass |
| 2 | force a failure in each | `failures>0`, exit non-zero | `test_terrain_brush` `FAIL (failures=1)` exit 1; `test_image_to_map` `FAIL (failures=1)` exit 1 | pass |
| 3 | other two suites | unchanged, exit 0 | `test_screen_store` exit 0; `test_override_merge` exit 0 | pass |
| 4 | `cargo test -p sstd-core` | 110 passed | 110 passed, 0 failed | pass |
| 5 | `rg "quit\(" editor/tests/test_*.gd` | one shape | 4 of 4 on `quit(0 if ... == 0 else 1)` | pass |
| — | restored files vs backup | identical | `diff -q` clean for both | pass |

Row 2 is the load-bearing one: it is what separates a real fix from swapping the
constants, and it is the check the original defect would have failed. Before the fix, a
failing run and a passing run both exited non-zero, so there was no input for which the
old and new code differed. Now they differ on every input.

## 7. Out of scope

- **Shared test harness.** See §2.
- **Silent-truncation exposure in `test_screen_store.gd`.** That suite has 86 `await`
  sites and none of the completion tracking added to `test_override_merge.gd`, so it
  shares the shape that produced a false pass there. No truncation has been *proven* to
  occur in it, so it does not belong in this fix; it is filed separately with the
  experiment that would settle it.
- **The `entity_overrides` dead channel** and the pre-#76 backup in `/tmp`, both
  unchanged from the #79 work.
