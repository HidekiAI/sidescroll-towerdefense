# Session handoff — 2026-09-07 (reboot interrupt; self-contained resume)

## Objective
Design + docs pass on the gRPC/protobuf surface, then debt-clearing on entity/terrain defs.
Session covered: #70 (gRPC index), #71 (living registry), #72 (single source of truth),
#67 (def overrides docs — in flight), #65 (type hierarchy design — in flight), plus
recorded prior work #68 (parallax) and #69 (OpenRouter gateway).

## Executed & committed (safe to ignore)
- **#70 CLOSED** — `TDD_gRPC-Service-Index` (wiki inventory: 2/113 RPCs implemented).
  Wiki `42e0d49`, main `72fc243`.
- **#71 CLOSED** — living registry: `crates/sstd-grpc/proto/README.md` + index purpose
  column + 3-layer model (Service Contract > gRPC surface > protobuf; gRPC != protobuf).
  Wiki `e63884a`, main `ba81f61`.
- **#72 CLOSED** — `build.rs` now compiles every `*.proto` in `proto/` via glob
  (sorted, empty => hard error, `rerun-if-changed`); one-lib rule recorded. Verified:
  `cargo check -p sstd-grpc` + `cargo test -p sstd-grpc` (6 tests) green.
  Main `c3af481`, wiki `366d136`.
- **#68 / #69** — designed + committed earlier this session (TDD/GDD wiki pages), issues
  OPEN pending implementation.

## IN FLIGHT (reboot interrupted here)
User decision (2026-09-07): "Close #67 + #65 docs" = write wiki page(s), then close.

- **#67** code was ALREADY shipped (main commit `9d78881`):
  - `crates/sstd-core/src/entity.rs:214` `EntityDefOverride` (all-optional) +
    `merge(:252)` + `into_entity_def(:281)` + `opt_entity_class_serde(:304)`.
  - `crates/sstd-core/src/terrain.rs:661` `TerrainTypeOverride` + `merge(:682)`.
  - `crates/sstd-core/src/storage.rs:111/196` `TerrainOverridesFile`/`EntityOverridesFile`.
  - `editor/scripts/world_archive.gd` `TERRAIN_OVERRIDES_PATH`/`WORLD_ENTITY_DEFS_PATH`,
    `save_world` writes them when non-empty; loader reads back.
  - `editor/scripts/entity_editor.gd` — removed `_add_defaults()`; added
    `set_framework_entities()`, `apply_world_entity_defs()`, `collect_world_entity_defs()`.
  - Verified: `cargo test -p sstd-core` = 110 passed.
  - **Wiki page `TechnicalDesign/TDD_Def-Overrides.md` WAS WRITTEN (complete)** but
    **NOT YET COMMITTED** — first thing to commit once resuming.
- **#65 terrain type hierarchy** — NO implementation exists anywhere (no `parent_type`/
  inheritance fields), and its referenced wiki page `TDD_Terrain-Type-Hierarchy` does NOT
  exist. It therefore CANNOT be closed (Issue-State Hygiene). Still to do:
  - Write `TechnicalDesign/TDD_Terrain-Type-Hierarchy.md` as DESIGN-ONLY (root
    ta/ground base types, `parent_type` field, merge parent-then-child, cyclic/reject
    invalid, editor greys inherited fields), note its composition order with #67 overrides
    (hierarchy resolves from framework prototype, then world override applies).
  - Keep #65 OPEN, comment "designed, no code".

## Next move (runnable order after reboot)
1. `cd /home/hidekiai/projects/SSTD/sidescroll-towerdefense.wiki`
2. `git status` — expect uncommitted `TechnicalDesign/TDD_Def-Overrides.md` (new).
3. Author `TechnicalDesign/TDD_Terrain-Type-Hierarchy.md` (design-only).
4. Update wiki `Home.md` (Technical Design table: rows for `TDD_Def-Overrides`, `TDD_Terrain-Type-Hierarchy`), `TODO.md` (new TS rows + footer date), `docs/SESSION-CHECKPOINT.md` (SHIPPED/IN-FLIGHT block for #67/#65).
5. Commit wiki (reference #67 and #65 — each commit must cite the issue number).
6. On main repo branch `trunk`: `gh issue close 67 --comment "<cite wiki page + commit 9d78881>"`.
   Comment on #65: designed-only, stays OPEN.
7. AGENTS.md has the durable rules already (gRPC colocation #72, etc.) — no change needed.

## Repo locations / rules reminders
- main repo `/home/hidekiai/projects/SSTD/sidescroll-towerdefense` branch `trunk`; wiki
  repo `...sidescroll-towerdefense.wiki` branch `master`. `gh` works from main dir.
- GitHub wiki only renders FLAT-slug links (`/wiki/TDD_Def-Overrides`); never path-qualify.
- Commit convention: Conventional Commits; every commit cites its issue number.
- Never push without explicit permission.
- Open issues after this session closes: #67, #65 (above), #68, #69 (designs awaiting implementation), and future/deferred items (#40, #44, #49, #51, #54, #56, #57, #58, #41, #63, #62).