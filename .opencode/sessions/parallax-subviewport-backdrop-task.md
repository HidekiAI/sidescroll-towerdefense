# Task prompt — #101: host the parallax stack in a SubViewport (start at B1)

> Start a NEW session inside `$HOME/projects/SSTD/sidescroll-towerdefense/` and send this
> file's contents as the opening message. It is written to be self-contained: a session with
> no memory of the prior conversation can work from it alone.

**TL;DR:** Implement ticket **#101** by executing the already-written, already-gated plan
`docs/PLAN-2026-10-06-subviewport-backdrop.md`, block **B1** first. Do not re-plan, do not
re-derive, do not re-gate: the plan passed the plan-ticket gate at 97/100 and the repo is
clean and pushed at `trunk b9e9d43`. Your first action is to read the plan's §8 (block map)
and §10 (exit conditions), then create a NEW branch and show B1's first hunk. Detail below.

---

## 0. What you are doing and why

The SSTD Godot editor's parallax backdrop lives in the editor's **root** viewport. That one
fact causes both open defects:

- **#88** — six bands stagger, because with no camera `screen_offset` never advances and
  each layer wraps at its own `repeat_size`.
- **#99** — ground bands fall below a windowed tab, because the framing is a hand-measured
  `-291` correction rather than a derived value.

Camera coupling is **per viewport** — `parallax_2d.cpp`'s `NOTIFICATION_ENTER_TREE` joins
`__cameras_<viewport_rid>` — so a `Camera2D` in the root viewport rewrites that viewport's
`canvas_transform` and displaces every `Control` (why #89 deleted it: that call was
correct), while a `Camera2D` inside a `SubViewport` cannot reach the root canvas at all. The
camera therefore returns, scoped inside a `SubViewport`. That is the whole structural idea.

Closing #88 and #99 at their shared root unblocks **#100** (zoom, deliberately **out of
scope** here).

## 1. Ground truth — read in this order, then stop reading

1. `gh issue view 101` — the ticket. Eight acceptance criteria A1–A8. It is the contract;
   anything neither it nor the plan calls for is out of scope.
2. `docs/PLAN-2026-10-06-subviewport-backdrop.md` — **the plan**. §6 technical design
   (6.1 node structure, 6.4 the measured sign, 6.5 framing, 6.7 ceiling, 6.8 what does NOT
   change), §8 block map, §9 measurement plan, §10 exit conditions, §11 stop conditions,
   §12 follow-ups.
3. `docs/SESSION-CHECKPOINT.md` → `## CURRENT STATE / NEXT MOVE` — resume point of record.
4. `.rsi_memory/sstd-parallax-subviewport-restructure.md` — playbook from the work so far.
   `.rsi_memory/plan-ticket-gate.md` — gate lessons. Both sit at the workspace root,
   deliberately outside both repos.

Ignore the older briefs in `.opencode/sessions/` (`parallax-scroll-offset-task.md`,
`parallax-tutorial-restack.md`). They describe #89, which is **closed**; their instructions
to work on `feat/parallax-restack` and to freeze the wiki are dead.

## 2. Repo state (verified 2026-10-06 at `trunk b9e9d43`)

```bash
cd $HOME/projects/SSTD/sidescroll-towerdefense
git branch --show-current          # -> trunk; trunk is the ONLY branch
git status --short                 # -> empty
git log --oneline -6               # plan, gate fixes, checkpoint, post-push refresh
```

Baseline, all verified green before any code:

| Gate | Result |
|---|---|
| `cargo test -p sstd-core` | exit 0 |
| `$HOME/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd` | exit 0 |
| same for `test_image_to_map` | exit 0 |
| same for `test_terrain_brush` | exit 0 |
| same for `test_override_merge` | exit 0 |

Gate on the **exit code**, never on a printed `failures=` line. Note: the workspace-level
`$HOME/projects/SSTD/AGENTS.md` still says `test_screen_store` is RED on Godot 4.7.2 — that
text predates `ecbf893`, which fixed #85. Do not chase it.

There is **no CI** and **no `rustfmt.toml`/`clippy.toml`** in either repo. Nothing gates a
push but you.

## 3. Non-negotiables

- **Branch-then-merge.** Every file change — including a one-line docs fix — lands on a NEW
  branch cut from `trunk`. Never commit to `trunk`. Sequence: branch -> commit -> ask once
  for push permission -> push -> `git merge --ff-only` -> push `trunk`. Delete the branch
  only on an explicit yes. Never `--force`.
- **Cadence: one hunk at a time.** Show the real diff (`git diff -- <file>`), walk it
  through, wait for explicit comprehension confirmation before the next hunk. Never batch a
  whole block. Which of us types the code was recorded ambiguously in the prior session —
  settle it in the first exchange, then hold that mode.
- **Measurements use a real window** on `DISPLAY=:0.0`, never a headless size read, and
  **render differencing** over geometry assertions — #88 recorded geometry reporting 0/6
  layers while screenshots plainly showed art.
- **`FORMULA-A/B/C`'s sign is measured by M-1, never re-derived.** §6.4 forbids it. M-1's
  result is picked once and consumed by B3.
- **Scope = #101's Expected Result only.** No zoom (#100), no `#92` re-scoping, no deletion
  of `_size_layer_repeats` unless M-3 proves it non-load-bearing (then a separate ticket).

## 4. The block sequence

**B1 -> B2 -> M-1 -> M-2 -> M-3 -> B3 -> M-4 -> M-6 -> A1 -> B4 -> B5**

B1/B2 encode no framing value, so they come first — and B1 *creates* the camera M-1 needs.
B3 is the first block that consumes M-1. Each B-block is one coherent commit. Full reasoning
and the file list per block: plan §8.

## 5. Start here — block B1

**File:** `editor/scenes/main.tscn` (only).

Add `BackdropView` (`SubViewportContainer`, stretch, fills the Simulator tab) → `Viewport`
(`SubViewport`) → `Camera2D`, and move the `BackdropStrip` instance inside. Both scrollbars
stay in `main.tscn` at `z_index = 20`. Exact structure: plan **§6.1**. B1 writes no framing
constant, so nothing needs measuring first.

Before touching it, read §8's ordering paragraph and §10's A1 row: A1 (root
`canvas_transform` byte-identical across a full two-axis scrub) is the criterion that
catches a camera escaping into the root viewport.

Then B2: `simulator.gd` gains the new node paths, scrollbar → `camera.position`, and
`_set_scroll_offset_x/_y` is deleted.

## 6. Known traps — do not spend tokens rediscovering these

- **`test_parallax_backdrop.gd` exits 1 on `trunk`** (`BackdropStrip has no Camera2D to
  drive`, line 69; assertions 114–198 never run). Pre-existing, **not** in the AGENTS.md
  gate list, which is why nothing caught it. B4 rewrites it.
- **`editor/tests/probe_parallax_scroll.gd` hangs** (exit 124 under `timeout 60`; it reads
  `L2_near`, a layer name retired by the restack). **B4 deletes it.** Do not run it.
- **`clip_contents` is set nowhere** in `main.tscn`/`backdrop_preview.tscn` — the Node2D
  strip currently paints unclipped, and `BackdropView` will be the first thing to bound it.
  Do not "fix" that as a separate change.
- **FORMULA-A's ceiling:** `half_viewport * scroll_scale < repeat_size`. Foreground's
  `k = 1.3` means a tab wider than ~2954 px breaks it. Documented in §6.7; recording it in
  code is a §12 follow-up, not this ticket.
- **A5 and A8 were amended after the gate found the ticket wrong.** A5 pins `scroll_scale`
  from a `manifest.json` read (the suite's `EXPECTED_LAYERS` is a hand-maintained copy and
  never reads the manifest); texture file names stay hardcoded literals. A8's target set is
  **three**, not the four #92 lists: `simulator.gd:2-14`, `backdrop_preview.tscn:31-34`,
  wiki `TDD_Parallax-Background` §3. **#92 still needs re-scoping — follow-up, not here.**
- **Stop conditions (plan §11):** if M-1 shows none of FORMULA-A/B/C satisfies A3 at both
  window sizes, stop and re-derive from the measured `screen_offset` — do not begin B3. If
  A1 fails, the camera reached the root viewport; the structure is wrong, not the value.

## 7. Definition of done

Plan §10's eight rows, each with its named proof: A1 probe, M-1/M-2/M-4/M-6 tables, B4's
suite exit 0 plus its three recorded non-zero mutation exits, and B5's doc greps (AGENTS.md
gate list + the three A8 locations + wiki §3). Wiki edits go to
`sidescroll-towerdefense.wiki` (branch `master`) — it is **not** frozen.

Then: update `docs/SESSION-CHECKPOINT.md`, close #101 with the wiki page citations, and
offer to delete the branch.
