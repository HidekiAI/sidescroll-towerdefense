# PLAN: make the #67 override-merge guard hermetic and non-vacuous

> Date: 2026-09-25. Issue: [#79](https://github.com/HidekiAI/sidescroll-towerdefense/issues/79).
> Wiki page of record: [TDD_World-Archive](https://github.com/HidekiAI/sidescroll-towerdefense.wiki/wiki/TDD_World-Archive).
> Status: plan written before code, per the documentation-first workflow. No production
> code path changes in this plan — the only file edited is a test.

## TL;DR

`editor/tests/test_override_merge.gd` is a #67 regression guard with two independent
defects and it leaves one coverage hole open. It reads its input from `res://world.zip`,
which is gitignored and untracked, so it cannot pass on a fresh clone. Two of its five
merge assertions are **vacuous** — they compare against values the framework prototype
already has, so they pass whether or not the merge runs. And the archive codec for
overrides is exercised by no test with a non-empty payload. Fix all three in one file:
build the input in-test from the tracked framework prototypes, use patch values that
actually differ, and cover the codec with a tmp-zip round-trip in the same idiom as the
neighbouring `_test_world_archive_roundtrip`. Commits no binary.

## 1. Problem, as verified

### 1.1 Non-hermetic input (the cause of the current red)

`test_override_merge.gd:49` and `:105` both read `res://world.zip`:

```gdscript
var tf := FileAccess.open("res://world.zip", FileAccess.READ)
```

`editor/world.zip` is gitignored (`.gitignore:49`) and untracked. Consequences:

- the guard cannot pass on a fresh clone — the input does not exist there;
- it is not hermetic — it asserts machine state, not code behaviour;
- it flips red for reasons orthogonal to the merge it exists to protect.

Current state: 3 of 12 assertions fail, because the local `world.zip` was replaced with
a different minimal world carrying no `terrain_overrides.json`.

### 1.2 Vacuous entity assertions (found while diagnosing, not previously reported)

`apply_world_entity_defs` (`entity_editor.gd:299`) is *replace-per-key, append-if-new*:
for each world def it finds the framework def with the same `key`, overwrites every prop
present in the world def (`:305-306`), and leaves props the world def omits intact;
unknown keys are appended (`:310`).

The test asserted:

```gdscript
check(int(e.get("attack_power", 0)) == 100 and int(e.get("max_hp", 0)) == 500, "arrow_tower merged values applied")
check(e.get("class", "") == "trap", "mine class preserved as trap")
```

The tracked framework prototype `editor/default_package/entity_defs.json` already has
`arrow_tower.attack_power = 100`, `arrow_tower.max_hp = 500`, `mine.class = "trap"`. The
expected values and the framework values are **identical**, so both assertions pass
whether or not `apply_world_entity_defs` was called. They cannot fail. That is why the
entity half stayed green while the terrain half went red: the entity half was never
testing anything.

Note the terrain half is *not* vacuous — framework `air.sub_tile_mask = 0` and
`dirt.sub_tile_mask = 15`, and the patch sets 15 and 0, so those differ and are real
assertions. They simply had no input.

### 1.3 Silent truncation: a dying guard still reports a pass

Found by mutation testing, not by reading. A guard is a coroutine (it contains
`await process_frame`). A runtime script error inside it — a parse error, a call
to a missing method — **aborts that coroutine but lets `_run` continue to the next
guard**, and the suite still prints `failures=0` and exits 0.

Demonstrated by inserting a `return` with a tab into the space-indented
`entity_editor.gd`, which made the script fail to parse:

```
SCRIPT ERROR: Parse Error: Used tab character for indentation instead of space ...
ERROR: Failed to load script "res://scripts/entity_editor.gd" with error "Parse error".
SCRIPT ERROR: Invalid call. Nonexistent function 'set_framework_entities' in base 'HSplitContainer'.
=== override merge done, failures=0 ===          <-- false pass
```

`apply_world_entity_defs` was never called at all, and the suite reported success.
This is the same failure class as §1.2 — a guard that reports success without
testing anything — and it is worse, because it also swallows the parse error that
would have explained the red run. A suite whose guards can die silently is not a
gate.

Fix: each guard appends its name to `_completed` on its final line, and `_run`
asserts all three names are present. A truncated guard is now a failure. Proven in
§6 by re-injecting the same tab-parse error:

```
FAIL: guard 'entity_merge' ran to completion
=== override merge done, failures=1 ===
```

Not yet applied to the other test scripts; they share the guard/coroutine shape
but no truncation has been *proven* to occur in them, so that is follow-up work
rather than part of this fix.

### 1.4 Coverage hole in the archive codec

`test_screen_store.gd:905` `_test_world_archive_roundtrip` calls
`WorldArchive.save_world(tmp_zip, manifest, screens, tiles)` with four positional args, so
`terrain_overrides` defaults to `{}`, and it asserts nothing about overrides. These
branches therefore have no test with a non-empty payload:

| Branch | Code | Covered today |
|---|---|---|
| write delta when non-empty | `world_archive.gd:117-123` | no |
| read delta back | `world_archive.gd:181-184` | no |
| omit file when delta empty | `world_archive.gd:117` guard | no |

A save/load asymmetry in the override channel, or a rename of `TERRAIN_OVERRIDES_PATH`,
would not be caught.

## 2. Explicitly not in scope

- **The save path is correct and is not being changed.** `world_archive.gd:117` writing
  only on a non-empty delta is correct delta semantics, symmetric with the read at
  `:181-184`. An earlier draft of #79 claimed this was a data-loss defect; that claim was
  disproved and withdrawn. This plan only makes that correct behaviour *enforceable* by a
  test, not different.
- **`entity_overrides` dead channel.** `world_archive.gd:18` defines `ENTITY_OVERRIDE_DIR`,
  `:113-116` writes a loop over an always-empty dict, `:176-178` populates a load-result
  field nothing reads, and the only producer is a stub returning `{}` at
  `placement_editor.gd:598`. Removing a `save_world` positional parameter is a breaking
  contract change, so it gets its own ticket rather than riding along here.
- **Boot-time world resolution.** `user://data/world.json` is an empty legacy manifest on
  this host and `editor/world.json` points at a dead `/tmp` path. Environment rot;
  documented, not fixed here.

## 3. Design

Three guards, each answering one question, all inputs built in-test.

### Guard A — terrain merge semantics

Question: does `apply_terrain_overrides` patch the listed props and leave the rest?

- Input: the tracked framework `default_package/terrain_types.json` (7 types), loaded
  through the existing `_load_framework_terrains()`.
- Patch: a literal, deliberately inverted relative to the framework so every assertion
  can fail — `{"air": {"sub_tile_mask": 15}, "dirt": {"sub_tile_mask": 0}}`.
  Framework is `air=0`, `dirt=15`, so both differ.
- Assertions: `air` becomes 15, `dirt` becomes 0, **and** an unpatched key (`grass`)
  still reads 15 — that is the "leave unspecified framework fields intact" half of the
  test's stated purpose, which no existing assertion covers.
- Guard against vacuity: assert the patch dict is non-empty and that at least one patched
  value actually differs from the framework *before* applying. If a future framework
  change makes the patch a no-op, the test fails loudly instead of silently passing.

### Guard B — entity merge semantics

Question: does `apply_world_entity_defs` replace per key, leave unlisted props, and append
unknown keys?

- Input: the tracked framework `default_package/entity_defs.json` (8 defs).
- Patch: values chosen to **differ** from the framework, so each assertion is falsifiable:
  `arrow_tower` `attack_power 100 -> 175`; `mine` `attack_power 200 -> 275`; plus a new
  key `steam_tank` to drive the append branch at `entity_editor.gd:310`.
- Assertions: the two patched values took effect; count goes 8 -> 9 (one replaced in
  place, one appended, no duplicates); `ballista` (untouched) still reads
  `attack_power = 250`, proving unlisted props survive; `steam_tank` is present.

### Guard C — archive codec round-trip

Question: does a non-empty delta survive a real archive write and read?

- `save_world(tmp, manifest, {}, {}, {}, patch)` — the leading `{}` is
  `entity_overrides`, which stays empty; `patch` lands in `terrain_overrides`.
- `load_world(tmp)` and assert `terrain_overrides` equals the patch exactly.
- Then the inverse case, which pins the delta semantics the #79 correction relies on:
  save with an empty delta and assert `terrain_overrides.json` is **not** in the archive.
  This makes "empty delta writes no file" an enforced contract rather than a claim in an
  issue body.
- Temp path: `user://`, not a hardcoded `/tmp/user/1000/...` like the sibling test uses.
  A hardcoded absolute user path would reintroduce exactly the non-portability this plan
  exists to remove. Both temp archives are removed at the end.

### Why not a tracked fixture

A committed `.zip` would also cover Guard C, and would additionally catch a rename of
`TERRAIN_OVERRIDES_PATH`. Rejected: it puts a binary in git and, more importantly,
re-creates the failure mode this whole exercise is about — a guard that asserts against a
fixed stored artifact and can therefore go red because someone swapped the artifact. A
tmp-zip round-trip covers the same branches with no committed state. The archive layout
itself stays covered by `_test_world_archive_roundtrip`.

## 4. Algorithm

```
_run():
  fw_terrains = load(res://default_package/terrain_types.json).tiles      # tracked
  fw_entities = load(res://default_package/entity_defs.json).entities    # tracked
  assert counts (7, 8)                                                    # fail loudly if shape changes

  # Guard A
  patch_terrain = {air: {sub_tile_mask: 15}, dirt: {sub_tile_mask: 0}}
  assert patch_terrain non-empty
  assert patch differs from framework for at least one key                  # vacuity tripwire
  med = instantiate(terrain_editor.tscn); med.set_framework_terrains(fw)
  med.apply_terrain_overrides(patch_terrain)
  assert air == 15, dirt == 0, grass == 15 (unpatched survives)

  # Guard B
  patch_entities = [ {key: arrow_tower, attack_power: 175},
                     {key: mine,        attack_power: 275},
                     {key: steam_tank,  ...} ]
  assert every patched field differs from framework                         # vacuity tripwire
  eed = instantiate(entity_editor.tscn); eed.set_framework_entities(fw)
  eed.apply_world_entity_defs(patch_entities)
  assert arrow_tower.attack_power == 175, mine.attack_power == 275
  assert ballista.attack_power == 250 (untouched)
  assert count == 9 and steam_tank present
  free both editors

  # Guard C
  tmp = "user://test_override_roundtrip.zip"
  save_world(tmp, {0,0: manifest}, {}, {}, {}, patch_terrain)
  assert load_world(tmp).terrain_overrides == patch_terrain
  tmp2 = "user://test_override_empty.zip"
  save_world(tmp2, {0,0: manifest}, {}, {}, {}, {})
  assert "terrain_overrides.json" not in ZIPReader(tmp2).get_files()
  remove both
```

Complexity: unchanged from the original — the merge loops are O(n·m) over a 7-type and
9-def set, and the archive round-trip is O(archive size) on a few-KB file. No new hot
paths; this is a test that runs in well under a second.

## 5. Data structures

| Name | Type | Source | Notes |
|---|---|---|---|
| `fw_terrains` | `Array[Dictionary]` | tracked `default_package/terrain_types.json` | 7 entries |
| `fw_entities` | `Array[Dictionary]` | tracked `default_package/entity_defs.json` | 8 entries |
| `patch_terrain` | `Dictionary` | literal in-test | `air:15, dirt:0`, inverts framework |
| `patch_entities` | `Array[Dictionary]` | literal in-test | 2 replacements + 1 append |
| `tmp` / `tmp2` | `String` | `user://` paths | removed at end |

## 6. Acceptance criteria

1. `~/bin/godot4 --headless --path editor --script res://tests/test_override_merge.gd`
   prints `failures=0` on a machine with **no** `editor/world.zip` present.
2. Removing or corrupting `editor/world.zip` does not change the result — proven by
   running the suite with the file temporarily moved aside.
3. Deleting the body of `apply_terrain_overrides` makes Guard A fail.
4. Deleting the body of `apply_world_entity_defs` makes Guard B fail.
5. **Amended after implementation.** The original criterion read "changing
   `TERRAIN_OVERRIDES_PATH` in `world_archive.gd` makes Guard C fail". That is
   **unachievable by a round-trip test**, and the reason is structural: writer
   (`world_archive.gd:118`) and reader (`:181`) share the same constant, so a
   rename is self-consistent and the round-trip still passes. The on-disk name is
   nonetheless a real backward-compat contract, because worlds already on disk were
   written under that name. Guard C therefore additionally pins the **literal**
   filename `terrain_overrides.json` in the non-empty archive. That assertion does
   fail on a rename, which is what the criterion was reaching for.
6. The file no longer *reads* `editor/world.zip`. It still **mentions** it once, in
   the header comment that explains why the guard must not depend on a gitignored
   artifact — deliberately kept, to stop someone reintroducing the dependency.
   The test is `grep -cE '(FileAccess|ZIPReader|load)\(.*world\.zip'`, which must
   return 0.
7. `cargo test -p sstd-core` stays at 110 pass — no `.rs` is touched, so this is a
   no-change regression check, not a real gate for this work.

## 7. Risks

- **Criteria 3-5 are mutation checks, not something CI runs.** They are how I prove the
  guards are falsifiable during this change; they are verified by hand and then reverted,
  and are not left in the tree.
- **If the framework prototypes change**, the vacuity tripwires will fail rather than
  silently pass. That is the intended behaviour, but it means a routine framework bump
  may require updating the patch literals. The failure message must therefore name the
  key and both values, not just say "vacuous".
- **`user://` write access in headless mode.** Confirmed on this host:
  `user://` resolves to
  `/home/hidekiai/.local/share/godot/app_userdata/SSTD World Editor/`, writes and
  removes cleanly under `--headless`. Note `OS.get_temp_dir()` is `/tmp`, so the
  sibling test's hardcoded `/tmp/user/1000/opencode/` is a machine-specific path,
  not the temp dir — which is why `user://` is used here.

## 7. Verification results

All criteria executed on 2026-09-25. Mutating production code required matching
each file's real indentation style (spaces in `entity_editor.gd`, tabs in
`terrain_editor.gd`); a mismatched insert is itself a §1.3 parse error and is not
a valid mutation.

| # | Mutation | Expected | Actual | Verdict |
|---|---|---|---|---|
| — | none (baseline) | `failures=0` | `failures=0`, 26 ok, all 3 guards completed | pass |
| 2 | `editor/world.zip` moved aside | no change | `failures=0` | pass |
| 3 | `apply_terrain_overrides` no-op | Guard A fails | 2 failures (air 0, dirt 15) | pass |
| 4 | `apply_world_entity_defs` no-op | Guard B fails | 4 failures (arrow_tower 100, mine 200, steam_tank absent, count 8) | pass |
| 5 | `TERRAIN_OVERRIDES_PATH` renamed | compat-name assertion fails | 1 failure | pass |
| §1.3 | tab-parse abort in `entity_editor.gd` | truncation check fires | `FAIL: guard 'entity_merge' ran to completion`, `failures=1` | pass |

Every source file touched by a mutation was restored with `git checkout --`;
final `git status` shows only the test and this plan as changed. `editor/world.zip`
was moved aside and restored, and is intact.

Regression gate, no `.rs` touched:

- `cargo test -p sstd-core` — 110 passed, 0 failed.
- `test_screen_store.gd` — 187 ok, `failures=0`, exit 0.
- `test_image_to_map.gd` — 9 ok, `failures=0`, **exit 1** (pre-existing inverted
  ternary, filed as #80).
- `test_terrain_brush.gd` — 14 ok, `failures=0`, **exit 1** (same cause, #80).
- `test_override_merge.gd` — 26 ok, `failures=0`, exit 0.
