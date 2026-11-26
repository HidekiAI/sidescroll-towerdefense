# Session Checkpoint — Parallax Restack (#68)

## CURRENT STATE / NEXT MOVE

> This block is the single authoritative resume point. It **supersedes** every
> `## Active step`, `## Next move` and `## SUPERSEDED` heading further down, which are
> retained as the historical record only. A cold-start session should read this block
> and stop.

**2026-11-26 (new direction, parked): @me specified a two-phase source-art preprocessing tool and asked that the design go into the existing wiki `TDD_Collision-Map-Tooling` section 10, not a new TDD.** Phase I cleans the source PNG, crops to the outer rectangle enclosing the whole grid, clears grid lines to alpha `0x00`, slices the eight cells at pitch 127 and resizes each to the configured tile size, guesses the air mask for a human to repair, and saves a cleaned PNG; phase II emits JSONC tile metadata for Rust after human sign-off. The source sheet is `editor/assets/samples/tileset_2-2.png`, confirmed by @me on 2026-11-26; `editor/assets/auto_sets/*.png` is not the slope art. #178 (grass slope conversion tool) was expanded on 2026-11-26 with this scope and the three preprocessing acceptance criteria. The separate implementation ticket #239 (two-phase preprocessing implementation) was opened at 16:07Z on 2026-11-26 with AC1-AC10, the sixteen RED-first tests and `Difficulty: L`; an implementation branch is still required before code. The design was written on wiki branch `docs/art-preprocess-tdd-178` and the direction note on code branch `local/docs/tile-art-cleanup-tool`; both branches were pushed by explicit remote and ff-only merged on 2026-11-26 (code `5b41cc8`, wiki `bb0c642`), and their local copies were purged. Still parked: whether the earlier `TileSetGrouping` Inspector preview should cover the Inspector only or also the FileSystem thumbnail; no tracking issue was found. No implementation code for either tool exists.

**CURRENT STATE 2026-11-25 (issue #178, grass slope conversion tool): planning is COMPLETE and PUBLISHED, the independent plan-to-ticket gate PASSED at 96/100 against the user-set 95/100 bar, the pre-push self-review returned PUSH-READY at 100/100, and all 17 gate findings are filed. No implementation code has been written and none may start until an implementation branch exists; ticket #239 (two-phase preprocessing implementation) is open.** The plan is `docs/PLAN-2026-11-25-slope-conversion-tool.md`, introduced in `0326f70` with the per-step ledger added in `0362d4f`; the planning branch `docs/plan-slope-conversion-tool` was pushed by explicit remote and ff-only merged to `trunk`, which resolves to `6937b1743d606d44ad3cc623cf10158b26246c87` on both `trunk` and `origin/trunk`. Raw push and merge output is in `/tmp/user/1000/opencode/sstd-plan-push.log`. Its `## Mini-checkpoints` section is the detailed per-step ledger (CP0-CP14); this CURRENT STATE block is the pointer of record, not a competing progress table. `AGENTS.md` carries the planning scope, gate bar and destination decision (committed in `0326f70`) plus the stateless-agent mini-checkpoint directive (committed in `0362d4f`). The 2026-11-25 gate iterations were 74/100 -> 87/100 -> 96/100 with the same independent `opencode/big-pickle` (free) evaluator; the 2026-11-26 expanded-ticket re-gate added 90/100 -> 96/100 with no surviving model record, as the plan's Plan gate section records. No CRITICAL and no UNCOVERED in any iteration. Gate evidence under `/tmp/user/1000/opencode/`: `sstd-issue-178.md`, `sstd-plan-provenance-detail.md`, `sstd-pr-provenance.md`, `sstd-pr-search-evidence.md` and `sstd-pr-evidence.sh`. Destination decision of record: the committed `editor/default_package/` with the committed `editor/assets/tiles/` and `editor/assets/tilesets/` catalogs, NOT the user-data world bundle. The 2026-11-26 ticket export `/tmp/user/1000/opencode/sstd-issue-178.md` has an empty COMMENTS block (line 78), so the answer is persisted in the plan and in `AGENTS.md`.

Filed gate findings, one issue each, from `/tmp/user/1000/opencode/sstd-gate-178-issue-drafts.md`: #179 provenance missing; #180 no keyed upsert or full-pipeline repeat guard; #181 AC3 oracle shared with the tool and a speckled fixture; #182 repeat-after-refusal swapped the input; #183 no partial-publication guard; #184 no tolerant reader for polluted terrain JSON; #185 no terrain-registration unit test; #186 AC4 failure had no close condition; #187 bare `sub_tile_mask: 15` literal; #188 no reverse trace to ticket sentences; #189 planner asserted the open-question answer; #190 ledger steps had no ticket anchor; #191 PR provenance had no raw evidence; #192 commit-number claim unmeasured; #193 lesson-driven guard given a false ticket anchor; #194 "every number" overstated, #178 not queried; #195 destination answer existed only in the plan. All 17 are FIXED as well as filed, so they are a durable finding record, not pending work. Raw numbers are in `/tmp/user/1000/opencode/sstd-gate-178-issue-numbers.txt`.

The independent pre-push self-review ran three full-diff passes on the same read-only `opencode/big-pickle` (free) reviewer: 80/100 with two Medium and two Low, then 100/100 with one Low, then 100/100 PUSH-READY with no findings. All five review findings were fixed before the push. @me approved their wording on 2026-11-25 and they were filed as issues #196 through #200, one per finding, from `/tmp/user/1000/opencode/sstd-self-review-178-issue-drafts.md`; a separate read-only scoring pass returned 97/100 against the 95 bar. All five are FIXED as well as filed, so they are a durable record, not pending work. Issue #178 remains OPEN; only the plan is published.

The 2026-11-26 independent docs self-review returned BLOCKED with five findings; each was fixed and filed as its own issue: #201 issue #178 body lacked the preprocessing scope claimed by the checkpoint; #202 the TDD crop-and-resize contract yielded 512x256 instead of 128x64; #203 `AGENTS.md` claimed a nonexistent preview ticket; #204 the checkpoint contradicted itself on the separate implementation ticket; #205 `AGENTS.md` used `Image.load` instead of `Image.new().load`. The first re-gate of the expanded ticket scored 90/100 with six findings, each fixed and filed as #206 per-cell mask dimensions, #207 PNG decode layer, #208 named literals in Rust assisted samples, #209 ten acceptance criteria, #210 the `auto_sets` exclusion, and #211 sixteen tests. The final re-gate PASSED at 96/100 with five low findings, each fixed and filed as #212 sidecar guard layer, #213 gate-record decode layers, #214 headless guard count, #215 source-cleaning guards, and #216 forward-trace anchor. All 16 are FIXED as well as filed, so they are a durable discovery record, not pending implementation work. The fixes shipped on `trunk` (`5b41cc8`) and `master` (`bb0c642`) in the 2026-11-26 docs publish; #201-#216 stay OPEN until their close pass. Twelve further 2026-11-26 pre-push review findings were each fixed and filed as their own OPEN issue: #217 sheet-wide mask instead of per-cell masks, #218 unexplained 512x256 figure, #219 omitted Rust decode layer, #220 omitted art-seams exclusion, #221 impossible unused-byte mutation, #222 false issue-close claim, #223 unverifiable evaluator-model continuity, #224 unsupported 40/100 score, #225 separate implementation gate, #226 unpaired determinism guard and sign-off mutation, #227 guard that cannot reach the JSONC reader, and #228 this inventory omitting the four wiki-side findings. Five more findings from the final re-reviews were each fixed and filed: #229 malformed Phase II data-flow sentence, #230 fabricated ticket quote in the reverse-trace table, #231 ticket shorthand in prose, #232 false Phase I purity claim, and #233 repeat coverage attributed to an undefined test phase. Four from the last full-diff pass were filed and fixed here: #235 the orphaned preprocessing test-writing boxes between the implementation gate and the source-measurement step, #236 repeat coverage named against a guard that cannot re-emit a sidecar, and #237 an altered ticket quote, plus #238 deleted plan box labels in the checkpoint. The wiki fixes shipped on `master` (`bb0c642`) and the code fixes on `trunk` (`5b41cc8`); all 37 findings stay OPEN until their close pass.

**NEXT MOVE in runnable order:** (1) The separate implementation ticket is #239 (two-phase preprocessing implementation), and its box in the plan's `## Mini-checkpoints` section is ticked. The two docs branches landed on `trunk` (`5b41cc8`) and `master` (`bb0c642`) on 2026-11-26; follow the remaining boxes in that section, starting with measuring the sources and config. Tests are written RED first. Rust logic uses assisted `TODO(human)` blocks; GDScript logic is written directly but every test is assistant-written. (2) The merged local branch `docs/plan-slope-conversion-tool` was deleted on 2026-11-25 with @me's approval; the remote-delete handover is `/tmp/user/1000/opencode/sstd-delete-remote-plan-branch.sh` for @me to run, never the assistant. Never `git add -A` or `git add .` here; the repo vendors 516 MB under `editor/addons/`. Stage explicit paths only. The six untracked `.uid` files in `git status` are pre-existing and not part of this task. Godot 4 timing uses `Time.get_ticks_msec()`.

**CURRENT STATE 2026-11-25: issue #161 (Rust-owned physics and terrain models) is CLOSED and shipped on code `trunk` and wiki `master`; run `git -C sidescroll-towerdefense log --oneline -3` and the same for the wiki repo for the exact tips.** Both working branches were pushed by explicit remote, ff-only merged to the default branches and both defaults pushed. The independent review was clean at 100/100 PUSH-READY with zero findings; its verdict note is on `refs/notes/self-review` for each tip. Section 8 of wiki `TDD_Model-Migration-Rust.md` is the execution ledger; this block is the resume pointer. The DAO, the three View proxies and their assistant-written guards are on `trunk`; this checkpoint correction ships on a follow-up docs branch. The merged working branches were deleted locally; @me still has to run the remote delete lines.

**COMPLETED: the GDScript DAO and View delegation.** `editor/scripts/model_gateway.gd` is a `RefCounted` DAO coercing integer fields before `JSON.stringify` and mapping null surface samples to `NAN`; its six guards in `editor/tests/test_model_gateway.gd` cover wire coercion, null columns, null sample, walker unwrap, terrain-file coercion and a back-to-back repeat. `editor/scripts/world_view.gd` keeps all three static signatures but routes them through the DAO; `editor/tests/test_view_model_delegation.gd` has five guards, the fifth pinning a malformed walker response so the caller's state is unchanged. Every bridge failure now logs the method, reason and exact request. Falsifiability: `docs/VERIFY-model-gateway-2026-11-25.md` is the TRACKED evidence (13 mutants, 7 View + 6 DAO, all FIRED, 0 VOID, 0 REFUTED, green baseline and postflight, byte-identical restore); removing the plateau fallback reddened the masked column-6 assertion, removing the malformed fallback reddened the new guard. Ticket IDs were removed from new source comments and the functional rewrites are in place. The four-file comprehension walkthrough was shown and @me approved proceeding.

**GATES (2026-11-25, Godot `4.7.2.stable.mono.official.ed1daf0bf`):** `cargo test -p sstd-core` 160 passed, `cargo test -p sstd-editor-bridge` 35 passed, `cargo test -p import-tiles` 8 passed, all exit 0. Full headless sweep of 19 `editor/tests/test_*.gd` suites: 18 exit 0; only `test_parallax_pixels` exits 1. Two consecutive reruns of `test_parallax_pixels` both exited 1 with `SCRIPT ERROR: Cannot call method 'get_width' on a null value.` (and the matching `get_height`), the pre-existing null-texture condition already recorded in wiki section 10 - NOT a flaky regression from this change and not claimed as one; posting a finding needs @me's approval. Two consecutive reruns of `test_screen_store` both exited 0, so the issue #85 known-red was NOT reproduced on this host and this suite is included in the 18 green suites, not excluded. Gate on exit codes, never on the printed counter. User directive 2026-11-25 in `AGENTS.md`: assisted `TODO(human)` blocks are Rust-only; GDScript production logic is written directly but every test is written first.

**NEXT MOVE, runnable order:** (1) the next mini-step is routing `editor/scripts/main.gd` terrain loading through `ModelGateway` instead of reading terrain JSON directly - plan before coding and keep it separate; issue #161 is CLOSED, so that routing needs its own ticket; (2) the shipped result, both review rounds and the parallax and issue #85 concerns are recorded in the issue #161 comment at https://github.com/HidekiAI/sidescroll-towerdefense/issues/161#issuecomment-7174915742; the accepted close and the tile-edge jitter limitation are recorded at https://github.com/HidekiAI/sidescroll-towerdefense/issues/161#issuecomment-7175347648 - no bug was filed for the headless parallax red or the jitter, and filing any future finding still needs @me's approval; (3) hand @me the remote delete lines for the merged working branches, never run them. Unmerged local branches remain untouched: code `docs/m1-probe`, wiki `docs/parallax-step4-corrections`. Known follow-up, not in this change: `step_walker`'s `world_left_px` / `world_right_px` parameters are retained for caller stability but no longer read in the body; changing the signature touches the caller and both test suites. NEVER `git add -A` / `git add .` in this repo (516 MB vendored `editor/addons/`); stage explicit paths only.

_Last updated: 2026-11-24. **#126 (continuous ground surface / solid-mask profile) is SHIPPED and CLOSED.** Code: `trunk` tip `485e3c2` (`2971a10..485e3c2` ff-only merged and pushed to `origin/trunk`); wiki: `master` tip `7aa8cc6` (`b4a974a..7aa8cc6` ff-only merged and pushed to `origin/master`). Closing comment posted 2026-11-24 (https://github.com/HidekiAI/sidescroll-towerdefense/issues/126#issuecomment-7135524330) citing `TDD_Simulator-Ground-Walk` §7, `TDD_Physics` "Ground Contact & Friction Gliding", `TDD_Collision-Map-Tooling` and `PostMortem/Slope-Hopping-2026-11-10`. Independent review 98/100 PUSH-READY, 0 new High/Medium. Full gate green: `cargo test -p sstd-core` 154 passed, `cargo test -p import-tiles` 8 passed, all 16 relevant GDScript suites exit 0, headless boot exit 0. Step-10 mutation battery: 25 mutants, 0 VOID, 15 KILLED, 10 REFUTED. MEASURED on Godot 4.7.2: profile vs shipped PNG 0.00 px over 224 columns, hand pins 0.00 px, notch 13 px, feet vs art 1.00 px worst over 62 frames, largest frame rise 14.7 px. @me ran the interactive test and accepted it: "the movement is genuinely smoothly sliding on the tiles". **Decode trap:** `(load(path) as CompressedTexture2D).get_image()` keeps green RGB under fully transparent pixels, shifting the apparent top edge 6 rows; the oracle must use `Image.new().load(path)` on the PNG FILE. Shared lesson `01M85D7FS36ZSVZB20AMH53PCX`; findings filed as #147 and #148. Out of scope, each needs its own ticket: Rec.601 cleanup (7 pre-existing sites), Marching Squares / RDP, collision bounds (`bounds_bottom_y`), graduated fall damage. Repo is PUBLIC but owned by @me (HidekiAI), push was explicitly authorized. NEVER `git add -A` / `git add .` here (516 MB vendored `editor/addons/`). Three local doc commits (`98c3893`, `b65d38e`, `6758242`) recorded @me's new direction on `trunk`; they merge here and go out with the next push.

**COMPLETED 2026-11-24: C.M.V. Rust-model migration design (issue #149) is SHIPPED and CLOSED.** The wiki TDD `TechnicalDesign/TDD_Model-Migration-Rust.md` is on `master` at `159a023`; `Home.md` and `TODO.md` rows shipped in the same commit, with the gate record in section 11 and the generalized gate playbook in `TechnicalDesign/.rsi_memory/plan-ticket-gate.md`. The code-repo pointer is on `trunk` at `876fc39`, then this terminal update. @me granted push permission once; both working branches were pushed to `origin` by explicit remote, ff-only merged to `master`/`trunk`, and both default branches pushed. Issue #149 and the ten gate findings #151-#160 were each closed with a commit reference and the wiki page cited. This is a design only: no migration code exists; section 8 of the TDD is the single execution ledger for a future implementation ticket. Design constraint carried into that ledger (shared lesson `01M78GC563G13ZCHP5VXAHGHSW`): Godot `JSON.stringify` writes integer Variants as `15.0` and serde rejects them for `u8`/`i32`, so the GDScript-to-Rust transform coerces integers before stringify, keeps a tolerant `deserialize_with` reader, and re-saves already-polluted artifacts.

**Mini-step ledger (persisted checklist - mark `[x]` on completion and update THIS block after EVERY mini-step; a cold-start session 3 weeks later resumes at the first unchecked item):**
- [x] Step 1 - GitHub feature issue created: https://github.com/HidekiAI/sidescroll-towerdefense/issues/149 ("feat(model): move Physics and Terrain models to Rust (C.M.V. migration design)"). Body carries design, approach, scope and `Difficulty: L`; wording accepted by @me 2026-11-24. Ticket-First Rule satisfied before any TDD or code.
- [x] Step 2 - Survey completed by a read-only `explore` agent 2026-11-24. VERIFIED: `world_view.gd` computes the profile (`build_ground_profile` 544-600), samples it (`ground_y_at` 604-624) and steps the walker (`step_walker` 645-716); all three carry proxy comments saying they should delegate to Rust when exposed. `physics.rs` has private `build_ground_profile` 144-209 / `ground_y_at` 215-232 and public `step_walker` 266-410, reached only by the bridge `walk_step`; its header says GDScript is the live path. `terrain.rs::TerrainTypeDef` 164-194 owns `solid_mask: Vec<u8>` 184 with `#[serde(default)]`; `deserialize_sub_tile_mask` 205-252 already proves the integral-float tolerance pattern. Bridge `WalkTile` 2352-2361 / `walk_step_impl` 2363-2398 are the JSON wire seam. `terrain_types.json` has 14 entries, 7 with `solid_mask`, all numeric values integral. `main.gd::_inject_terrain_masks` 261-272 loads terrain JSON directly in GDScript, bypassing `import_terrain_types`. gRPC has one proto (`editor.proto`, 2 RPCs) with a single-schema convention in `crates/sstd-grpc/proto/README.md`. DAO-like precedents: `ScreenStore`, `WorldArchive`, `ParallaxLoader` (all `extends RefCounted`). Indentation is mixed across files (`world_view.gd` spaces; `main.gd`, `terrain_editor.gd`, `parallax_loader.gd`, `world_view_ground.gd` tabs).
- [x] Step 3 - Wiki `TechnicalDesign/TDD_Model-Migration-Rust.md` authored on branch `docs/model-migration-tdd`. Sections: verified current state (survey line numbers), CCMV ownership + PlantUML flow, `GroundProfileSnapshot` type, algorithms/complexity, proof-of-concept sampler, bridge/gRPC contract, `ModelGateway` DAO seam, ART->DATA decision (reuse `tools/import-tiles`, no new GDScript art tool), Config `populate_006` keys, 8-step falsifiable implementation ledger, checkpoint protocol, follow-ups. No code, no git hashes, no issue-label shorthand in the page.
- [x] Step 4 - `Home.md` gained a `TDD_Model-Migration-Rust` row next to Physics; `TODO.md` gained section "This Session (2026-11-24) - Rust Model migration design" with row TS81, both in the same wiki branch `docs/model-migration-tdd` (committed separately from Step 3 but before the branch is pushed). Unindexed-page self-check run: the new page IS indexed; four pre-existing hits are dotted sub-pages of the parent `TDD_Parallax-Scrolling` row that already names all four. Finding filed as #150 (false positive, no links missing).
- [x] Step 5 - Plan gate PASSED 98/100 after 2 iterations (bar 90): iteration 1 was 88/100 with 7 deficiencies, iteration 2 was 98/100 with 3; no CRITICAL and no UNCOVERED in either. All 10 findings were filed as their own issues (#151-#157 and #158-#160), each fixed in the wiki TDD even though filing is required regardless. The TDD gained section 11 "Plan gate" (score, iterations, per-finding issue map) and `TechnicalDesign/.rsi_memory/plan-ticket-gate.md` holds the generalized lessons. Evaluator stayed the same agent in its own context on `opencode/big-pickle` (free) for both scores; no model changed.
- [x] Step 6 - Both branches committed locally, one push permission granted, then pushed by explicit remote: wiki `docs/model-migration-tdd` `159a023` ff-only merged to `master` and pushed (`7aa8cc6..159a023`); code `docs/model-migration-ledger` `876fc39` ff-only merged to `trunk` and pushed (`f5c6548..876fc39`). Pre-push checks clean: conventional subjects, no scaffolding, no banned trailer, no divergence, docs-only so no build gate. Issue #149 and findings #151-#160 closed with commit references and the wiki page cited (log at `/tmp/user/1000/opencode/close-149-and-findings.log`). This terminal checkpoint update follows on branch `docs/checkpoint-149-done`.

**COMPLETED 2026-11-24: S1 (physics config population `006`) of issue #161 (Rust-owned physics and terrain models), on branch `local/feat/model-rust-ownership`.** Issue #161 is OPEN, Ticket-First satisfied; the eight-step ledger in `TDD_Model-Migration-Rust` section 8 is the only implementation ledger and this block only points to it. GREEN: `cargo check -p sstd-core --tests` exit 0, `cargo test -p sstd-core` 155 passed / 0 failed, `cargo test -p sstd-core config` 22 passed / 0 failed, no fmt diff on `config.rs`, 0 `TODO(human)` markers. Four assistant-written tests: new `test_physics_keys_seeded` plus tightened config count 30->34, `006` population applied, schema version 0.0.5->0.0.6; RED baseline had exactly those 4 failures. `config.rs:27` now names the persisted token `PHYSICS_POPULATION_ID = "006"` (single authored spot; `test_population_applied` keeps the literal as the on-disk pin); the other five ids stay literals by @me's decision. Mutation battery `/tmp/user/1000/opencode/sstd-s1-mutations.sh`: 3 mutants, 0 VOID, all parse, all redden their named tests -- seed `200.0`->`201.0` reddens `test_physics_keys_seeded` only; id `006`->`007` reddens `test_population_applied` and `test_schema_version_after_population`; dropping the Population entry reddens all four. The tree was restored byte-identical and re-greened after each mutant. Review caught two fill defects (surviving `{}` hole at the `populate_006` signature and one extra leading space on every uncommented line); @me said "fix" and both were corrected before the green run. Sample proof in the throwaway copy `/tmp/user/1000/opencode/sstd-s1-sample-check` is what caught a missing closing brace in the scaffold. **DONE: the named ground-profile snapshot from `physics.rs` (aka the TDD's Model API mini-step) shipped on this branch; see the block below for its record and the next item.** Grounded plan written BEFORE tests: `physics.rs:144 build_ground_profile` returns `(Vec<Option<f64>>, f64, f64)`, `:215 ground_y_at(profile, world_left_px, pos_x)` unpacks that tuple, `:266 step_walker` uses both at `:287/:298/:306`, and public `:238 ground_y_for_column` is referenced only by tests `:485/:498` -- it stays until its own ticket (TDD section 10), while the snapshot becomes the live path. Add `pub struct GroundProfileSnapshot { world_left_px: f64, world_right_px: f64, surface_y_by_column: Vec<Option<f64>> }` in `physics.rs` deriving `Serialize, Deserialize, Clone, Debug, PartialEq`; TDD section 4.1 mandates that location so the profile has one named shape, a deliberate exception to the "public type owns a file" rule. `build_ground_profile` returns the snapshot, `ground_y_at(&snapshot, pos_x)` reads `snapshot.world_left_px` internally, and `step_walker` reads `snapshot.world_right_px` for its clamp. Algorithm and complexity stay as TDD section 4.2 (`O(T*P)` build once, `O(1)` sample) -- no new math. Ambiguity resolved before coding: TDD section 4.3's `(pos_x - world_left_px) / tile_w` indexing and its `(Some, None) -> Some(y0)` plateau hold disagree with the live per-pixel sampler, which returns `None` at an air neighbour; the shipped PNG oracle has no gaps and section 4.2 says this step is ownership plus a named return type, so today's math is preserved and pinned by a test. The GDScript plateau hold at `world_view.gd:604-624` is a real divergence needing its own ticket -- record it, do not fix it here. Race and back-to-back-repeat cases do not apply: pure functions over an immutable tile slice, no shared state or I/O. Tests first: RED is confirmed -- `cargo test -p sstd-core physics` exit 101 with exactly 5 new guards (`snapshot_builder_exposes_named_shape`, `snapshot_sampler_interpolates_between_columns`, `snapshot_sampler_reports_an_air_column_as_none`, `snapshot_sampler_ends_a_solid_run_at_a_gap`, `snapshot_wire_shape_uses_null_for_air_columns`) plus 2 retargeted existing tests, and compile errors E0425 (new type), E0308 (builder hole), E0061 (two old sampler calls), E0609 (tuple field access). One assistant-written assertion was wrong before the RED run: the first air column is one tile width in (index 32), not index 1; fixed to index into `serde_json::Value` by `TEST_MASK_TILE_PX as usize`, recorded as shared lesson `01M87APTSQVNJ66H1KYQZ26H37`. Four `TODO(human)` blocks are scaffolded in `physics.rs`: 1/4 at line 134 (public `GroundProfileSnapshot`, deriving Serialize/Deserialize/Clone/Debug/PartialEq), 2/4 at line 156 (builder return type) and line 224 (snapshot return expression), 3/4 at line 240 (`ground_y_at(&snapshot, pos_x)` signature plus body), 4/4 at line 318 (`step_walker` call sites). Hole diagnostics: E0425 at 1/4, E0308 at 2/4, E0061 at the two old sampler calls in 4/4. All four samples were PROVEN in the throwaway workspace `/tmp/user/1000/opencode/sstd-s2-sample-check`: after applying the fills, `cargo test -p sstd-core physics` exits 0 with 17 passed (the scratch copy needs `editor/assets` copied in for the `include_str!` in `parallax.rs`). ALL FOUR BLOCKS FILLED and GREEN 2026-11-24: marker gate clean, `cargo check -p sstd-core --tests` exit 0, `cargo test -p sstd-core` 160 passed / 0 failed, `cargo test -p sstd-core physics` 17 passed / 0 failed, 0 fmt diffs on `physics.rs` (the only fmt diff is the pre-existing `examples/repro_parse.rs`). Review scored the fill 85/100: blocks 1-3 correct; block 4 initially left the old tuple lines live below the new snapshot lines (E0308/E0061) and hit E0425 because the scaffold sample omitted the `let pos_x = if stepping_into_hole {...}` binding -- that omission was the assistant's scaffold defect, not @me's. @me said "please apply"; the assistant removed the duplicate lines and moved the `pos_x` binding above `surface_y`, matching the proven scratch copy. Mutation battery `/tmp/user/1000/opencode/sstd-s2-mutations.sh`: green baseline 17/17, five mutants, 0 VOID, all parse, all fire. M2 (interpolation fraction pinned to zero) reddens ONLY `snapshot_sampler_interpolates_between_columns`; M4 (next-column air falls back to this column surface) reddens ONLY `snapshot_sampler_ends_a_solid_run_at_a_gap`; M5 (wire field renamed via serde attribute) reddens ONLY `snapshot_wire_shape_uses_null_for_air_columns`; M3 (both column reads fall back to zero on air) reddens `snapshot_sampler_reports_an_air_column_as_none` plus the gap-end guard and the pre-existing `walker_refuses_to_step_into_a_hole`; M1 (profile width loses its last tile) reddens eight guards because it changes coverage for every test. `physics.rs` restored byte-identically and final `cargo test -p sstd-core physics` 17/17. Two harness traps found and fixed in the script: it must `cd "$REPO"` explicitly (the shell cwd was the workspace root, so cargo could not find Cargo.toml), and cargo prints `test <name> ... FAILED`, so the expected-guard grep needs `.* FAILED` not a bare ` FAILED`. An aborted earlier run left its M1 mutant in the tree, which then became the next run's backup and made all five look like failures; fixed by restoring the line, adding an EXIT trap and a green-baseline preflight. The snapshot work and its review fixes were pushed on 2026-11-24: branch `feat/model-rust-ownership` ff-only merged to `trunk` (`5d8025c..e264cb3`) and pushed by explicit `origin`; the whole range re-reviewed 100/100 against the 95 bar." The config-key work and this checkpoint update are committed together on the branch; push permission was granted on 2026-11-24. Two doc commits correcting the OpenRouter population from `006` to `007` were renamed to `docs/config-population-renumber` in BOTH repos, rebased onto the moved bases, pushed by explicit `origin` and ff-only merged (code `3aed64a`, wiki `05908dc`); see the shipped-state record below. Four directives remain out of scope, each needing its own ticket before code: Rec.601 cleanup (7 pre-existing sites), Marching Squares / RDP contour simplification, collision bounds (`bounds_bottom_y`, separate render node from collision shape), and graduated fall damage from impact height. The other designed-but-unbuilt work stays tracked in the wiki `TODO.md`: #68 (parallax restack) and #69 (OpenRouter gateway). Local working branches `feat/ground-profile-tool` and `docs/ground-profile-126` are merged and remain locally; delete only on an explicit yes. Their remote branches are @me's: hand over `! git push <remote> --delete <branch>`, never run it. In the code repo NEVER `git add -A` / `git add .` (516 MB vendored `editor/addons/`); stage explicit paths only. Keep `docs/SESSION-CHECKPOINT.md` CURRENT STATE as the single pointer of record; do not open a second progress table.

**COMPLETED 2026-11-24: named ground-profile snapshot from `physics.rs` plus the six review fixes, on `local/feat/model-rust-ownership` (base `trunk` `5d8025c`; commits `63dd75c`, `ba862dc`, `a20ed9f`, `f5ab637`).** Issue #161 is OPEN; section 8 of `TDD_Model-Migration-Rust` is the only ledger and its result block holds the per-step numbers. `GroundProfileSnapshot` is public with `world_left_px`, `world_right_px` and `surface_y_by_column`; `build_ground_profile` and `ground_y_at` are public; `step_walker` clamps with the snapshot bounds; `ground_y_for_column` is deliberately kept because removing that rescan needs its own ticket. GREEN after the fixes: `cargo check -p sstd-core --tests` exit 0, `cargo test -p sstd-core` 160 passed / 0 failed, `cargo test -p sstd-core physics` 17 passed / 0 failed, rustfmt clean on both touched Rust files. Mutation battery `/tmp/user/1000/opencode/sstd-s2-mutations.sh` extended from five to seven guards: two new mutants shift only the snapshot's left bound or only its right bound, so both bound assertions are falsifiable rather than merely the profile-length one; 7/7 FIRE, 0 VOID, all parse, `physics.rs` restored byte-identical and re-greened (log `/tmp/user/1000/opencode/sstd-s2-mutations-run2.log`). The 224-column / 0.00 px PNG-oracle figure is INHERITED from issue #126's gate (MEASURED 2026-11-23 under Godot 4.7.2), not re-run here; profile construction and interpolation arithmetic are unchanged from that baseline, so it stays a regression target with its source named. Six independent-review findings were fixed: the private public-API claim and false bridge-sharing comment, the unstated oracle source, the missing bound mutants, two comments claiming a shared runtime value that is not loaded yet, magic/redundant test literals, and the `k` binding. **Next item: the bridge seam** - expose `build_ground_profile` / `ground_y_at` through the existing JSON `#[func]` bridge with wire types; gRPC `ModelService` stays design-only until a named consumer exists. The range was re-reviewed 100/100 against the 95 bar, renamed from `local/feat/model-rust-ownership` to `feat/model-rust-ownership`, pushed by explicit `origin`, ff-only merged to `trunk` and pushed (`5d8025c..e264cb3`). The GDScript plateau-hold divergence now has its own ticket, #168 (align the GDScript plateau hold with the Rust ground sampler), so the code comment pointing at a separate ticket is no longer a dangling promise.

**COMPLETED 2026-11-25: the bridge seam for the ground snapshot (the TDD's bridge mini-step) of issue #161, pushed to `origin/trunk` (code `932c4a3..5916864`) and ff-only merged there; the branch `feat/bridge-ground-snapshot` remains on origin at the same tip.** Objective: expose `build_ground_profile` / `ground_y_at` through the existing JSON `#[func]` bridge so the GDScript DAO has a complete seam; gRPC `ModelService` stays design-only until a named consumer exists. Section 8 of `TDD_Model-Migration-Rust` is still the only ledger; the wiki branch `docs/model-migration-ledger-s3` carried the ledger result and was pushed and ff-only merged to `origin/master` (`9ea0862..6aeeb18`). Files touched: `crates/sstd-editor-bridge/src/lib.rs` only. Five assistant-written guards were added at the end of the `#[cfg(test)] mod tests` block: `bridge_profile_round_trips_every_column` (224 columns, derived from seven 32 px tiles by the builder's own coverage math, not a guessed index), `bridge_profile_is_identical_on_two_calls`, `bridge_ground_y_interpolates_between_columns`, `bridge_ground_y_keeps_an_air_column_null`, `bridge_ground_y_reports_a_malformed_request`. RED baseline: `cargo test -p sstd-editor-bridge` exit 101 with exactly six `E0425` at the six new call sites. Race guard does not apply -- both functions are pure and hold no shared state; the back-to-back repeat is pinned by the two-call guard. Both Rust logic functions (`build_ground_profile_impl` at `crates/sstd-editor-bridge/src/lib.rs:2554` and `ground_y_at_impl` at `:2578`) are now written in the working tree with 0 `TODO(human)` markers; wire structs `BuildGroundProfileReq` / `GroundYAtReq` and the two one-line `#[func]` delegations at `:254` / `:260` are mechanical boilerplate written live. Samples were PROVEN green by `/tmp/user/1000/opencode/sstd-s3-prove-samples.sh`: a trap-guarded temporary uncomment inside the marker ranges gave `cargo test -p sstd-editor-bridge` 35 passed / 0 failed including all five new guards, rustfmt-clean, then restored the scaffold byte-identically (log `/tmp/user/1000/opencode/sstd-s3-prove-samples.log`). The first proof attempt FAILED on a parse error that was a harness defect, not a sample defect: block-description comments sat inside the uncomment range and became bare Rust prose; the descriptions were moved above the markers and the proof re-run green. Both review fixes are now applied and verified: the profile fixture gives each of the seven tiles a distinct `world_tile_y` and a 128-byte mask whose top four pixel rows are air, so `bridge_profile_round_trips_every_column` asserts a per-column surface derived from the builder's row-four-first-solid math instead of a flat all-zero profile; the new comments no longer carry a ticket ID or an opaque mini-step label. Findings filed as issues #169 (guards missed a dropped mask or tile Y) and #170 (ticket ID and opaque label in new comments). Re-run `/tmp/user/1000/opencode/sstd-s3-block-review-mutations.sh`: green baseline 35 passed / 0 failed, then drop-solid-mask, drop-tile-y and the grid-width positive control each redden exactly `bridge_profile_round_trips_every_column`, scaffold restored byte-identically; `rustfmt --check` clean; direct `cargo test -p sstd-editor-bridge` 35 passed / 0 failed. Code is committed locally on this branch at `8cbc8bf` (`cargo build -p sstd-editor-bridge` and `scripts/build-bridge.sh` both exit 0), and the wiki ledger plus TODO row are committed on `docs/model-migration-ledger-s3` at `de8d83d`. An independent read-only review scored the whole change 91/100 against the 95 bar and filed #171 (stale resume block), #172 (review findings left open), #173 (mutation probe fakes exit status), #174 (redundant gist comments) and #175 (inherited short bindings). The five fixes landed: code `5cc0869` (two redundant comments deleted, checkpoint corrected), wiki `6aeeb18` (ledger findings record corrected); the mutation probe outside the repo now records `exit=101` per red mutant. Issues #169 through #174 are CLOSED against those commits; #175 stays OPEN as the tracked follow-up for the inherited short bindings. The second independent review scored the full change 97/100 against the 95 bar and found one stale line number; that was fixed in `5916864`, filed as #176 and closed, and the reviewer confirmed PUSH-READY with no finding remaining. The push script `/tmp/user/1000/opencode/sstd-s3-push.sh` was run once with approval: both branches were pushed by explicit `origin` and ff-only merged to `trunk` and `master`, and both remotes now match the local base tips. **Next item: the GDScript DAO seam** - let the three static functions in `world_view.gd` delegate to the two new bridge `#[func]` methods, keeping call sites stable; work it on a new branch with tests first and section 8 as the only ledger. The OpenRouter config-population naming decision is DONE: both repos' branches were renamed to `docs/config-population-renumber`, their single doc commits were rebased onto the moved bases (unpushed at the time, so no published history changed), pushed by explicit `origin` and ff-only merged; code `trunk` is at `3aed64a` and wiki `master` at `05908dc`.

_**STALE as of 2026-11-23 -- read the block above.**_ _Last updated: 2026-11-19 (branch `feat/ground-profile-tool`, pushed to `origin`). **#126 step 1: 8 of 8 blocks FILLED -- 0 `TODO(human)` markers, 0 typed holes, 0 `// was:` lines.** Block 1 `to_greyscale` was committed at `984dd76`; blocks 2-8 (`uniform_patch_grey`, `remove_floating_specks`, `derive_solid_mask`, `flag_unresolved`, `terrain_entry_from_image`, `scan_tile_directory`, and the tile-scan branch in `main()`) were filled in the working tree this session. `to_greyscale` reads the module-level `REC601_LUMA_RED_WEIGHT` / `REC601_LUMA_GREEN_WEIGHT` / `REC601_LUMA_BLUE_WEIGHT` constants, never a bare `0.299` / `0.587` / `0.114`: the sample @me uncommented had carried the literals, recorded as lesson `01M7QGRZ6X0NBYF41TW9VNGMAG` (a sample's magic numbers ship the moment it is uncommented). MEASURED on 2026-11-19: `cargo check -p import-tiles --tests` clean, `cargo test -p import-tiles` **8/8 green**, rust-analyzer 0 diagnostics after a buffer `checktime` reload. Three review fixes landed in the fill batch: the missing `}` closing `uniform_patch_grey` (symptom: `unclosed delimiter` at its signature, caused by deleting the scaffold's typed-hole line), a stray `// Q: reached where?` comment in `remove_floating_specks`, and 5-space uncomment indentation normalized to 4. The samples were PROVEN before the fill in a throwaway crate copy (`/tmp/user/1000/opencode/import-tiles-sample`): **8/8 green** and the plan's T1-T6 mutants each FIRE, PARSE and redden their named test (T1 reddens two -- `mask_two_cluster_bottom_anchor` and `mask_removes_floating_solid_specks`; T2-T6 exactly one each), plus an ad-hoc greyscale mutant that reddens four. Two defects the proof caught and fixed in the scaffold: block 5's sample needed `use image::Rgba` (only `RgbaImage` was imported), and the Rec.601 luma weights sum to just under 1.0 in f64, so the sum ROUNDS (without it a pure grey 155 read as 154 and the exact-grey assertion failed). Recorded in the repo `AGENTS.md` #126 section._

**OPEN FINDING (2026-11-18, not yet ticketed):** the same Rec.601 luma literals sit at 7 pre-existing sites that #126 does not need to touch -- `tools/import-tiles/src/main.rs:416-418` and `:445` (`auto_detect_terrain_key` / `detect_content_bounds`), `crates/sstd-editor-bridge/src/lib.rs:1153`, `editor/scripts/terrain_editor.gd:379`, and `editor/scripts/map_editor.gd:712`, `:1369` and `:1396`. The GDScript/bridge pair is a deliberate TRUNCATING cross-language parity contract, so replacing them needs care. Left alone per "bug fix = the ticket's Expected Result"; ask @me whether to file the follow-up issue.

**Next move (2026-11-19):** the schema FIRST -- add `solid_mask` to `crates/sstd-core/src/terrain.rs::TerrainTypeDef` (the `TODO(human)` scaffold at lines 158-184), tests written RED before the block is scaffolded. It MUST precede the data: `TerrainTypeDef` is `#[serde(deny_unknown_fields)]`, so writing `solid_mask` into `editor/default_package/terrain_types.json` while the field is absent makes `TerrainTypesFile::from_json` reject the whole file (proved by reading `crates/sstd-core/src/terrain.rs:187` and the bridge import at `crates/sstd-editor-bridge/src/lib.rs:59`). Then the tool's tile-scan over `editor/assets/tiles/slope_up_{0..6}_32x32.png` and commit the 7 entries (`is_walkable: true` + `solid_mask`) into `editor/default_package/terrain_types.json`, then steps 4, 6, 7, 8, 9, and the FULL gate (step 10) including the M1-M8 and T1-T6 mutants -- all on this branch, tests first. Loose ends that do NOT belong in this work: the Rec.601 follow-up issue (7 pre-existing sites, open finding below) and the unrun remote-branch purge script.

_**STALE as of 2026-11-18 -- read the block immediately above.**_ _Last updated: 2026-11-17 (MERGED to `trunk`, pushed to `origin`; this checkpoint commit is the current trunk tip, and the working branch `feat/ground-profile-impl` was merged `--ff-only` and its local branch deleted). **#126 plan gate: ACCEPTED at 86/100** -- @me lowered the bar "for now" to "above 80", so no further gate iterations. Scores: iter-1 100/100 (false positive), iter-2 52/100, iter-3 55/100, iter-4 82/100, iter-5 86/100. NO code is in flight. The plan `docs/PLAN-2026-11-12-ground-profile.md` now carries the 12-step implementation sequence as its own section (`d0e5d20`), and its Plan-gate status reads ACCEPTED at 86/100._

_**All 18 gate findings are filed as GitHub issues #128-#145** (ten from iteration 4, eight from iteration 5), one issue per finding. The ten iteration-4 findings are already fixed in the plan revision; the eight iteration-5 findings are NOT fixed in the plan (they stay recorded as issues) and are in-scope for #126. This is now a standing rule (below), so the filing is not redundant busywork: it is the durable discovery record a later planning pass reads._

_**New standing rule, recorded 2026-11-17 (user directive):** every finding a gate, review, judge or test run produces is filed as its own issue in the repo it concerns, EVEN WHEN it is already fixed, so a later planning pass reads what was discovered, considered, and how it was resolved. @me: "Even if the issues discovered will be (or are) fixed, it is important to record it (i.e. via git-issues on this cases) so that future planning are aware of what had been discovered and considered and how it was resolved". It is an explicit exception to "an issue only when the commit message cannot justify the change". Recorded in `~/scripts` commit `96f5b7b` (branch `docs/record-findings-as-issues`): `LLM/rules/default.md` + `LLM/global/github.md` + `LLM/MEMORY.md` (GitHub Issues Discipline) and a Persist-step bullet in `LLM/skills/plan-ticket-gate/SKILL.md`; lesson `01M7MYE1DFT1QNPSG99TE85BGD` in `shared.db`. The `~/scripts` commit is PUSHED to both remotes (`github` + `origin`) and MERGED to `trunk`; its branch was deleted._

**Next move (2026-11-17):** all planning is MERGED to `trunk` (the plan, the checkpoint, the rules), so all that remains is the code. Work the 12-step sequence in `docs/PLAN-2026-11-12-ground-profile.md` -> `## Implementation sequence`, tests first: step 5 is already done, so start at step 1 (the tool's greyscale `derive_solid_mask` plus its tile-scan mode). Work it on a NEW branch, not on `trunk`.

_Last updated: [REDACTED]16:36 (branch feat/ground-profile-impl, tip 9134d71). **#126 is mid-implementation; the RUST physics layer is done and green; the GDScript tool/import side is scaffolded.** The branch is on origin and pushed (tip 9134d71); the working tree is CLEAN. **STALE as of 2026-11-17 -- read the block immediately below instead.**

_**STALE (superseded by the block above).** Last updated: 2026-11-17 (branch `feat/ground-profile-impl`, tip `87b4900`, pushed to `origin`). **#126 is in the PLAN-GATE loop; NO code is in flight.** `docs/PLAN-2026-11-12-ground-profile.md` is revised to clear BOTH the iteration-3 and iteration-4 gate findings (commits `c6e773e`, `87b4900`); an independent evaluator is re-scoring it (bar 90) in a fresh context. Scores so far: iter-1 100/100 (false positive), iter-2 52/100, iter-3 55/100, iter-4 82/100. Re-score in a loop until it passes, then code. The third #126 amendment (resolve by fuzzy logic / best guess FIRST; the red 1px border is the LAST resort) is POSTED: https://github.com/HidekiAI/sidescroll-towerdefense/issues/126#issuecomment-6950495262 ._

**The planned implementation sequence for #126, in runnable order (recorded 2026-11-17).** It had lived only in chat until then; the plan's `## Files` table is the per-file detail, this is the order the steps are worked.

1. **ART -> DATA, the tool:** `tools/import-tiles/src/main.rs` -- `derive_solid_mask()` = greyscale -> threshold -> no-donut clean -> fuzzy/best-guess -> flag; add a tile-scan mode over the committed `slope_up_{0..6}_32x32.png`; in-source mask tests.
2. **DATA, the schema:** `crates/sstd-core/src/terrain.rs` -- `TerrainTypeDef` gains `solid_mask` (serde default, so old files still parse).
3. **DATA, the committed masks:** `editor/default_package/terrain_types.json` -- the tool writes the 7 `slope_up_*` entries with `is_walkable: true` + `solid_mask`.
4. **DATA, the bridge:** `crates/sstd-editor-bridge/src/lib.rs` -- `WalkTile` gains `solid_mask`.
5. **TECH, Rust stepper:** `crates/sstd-core/src/physics.rs` -- `build_ground_profile` + `ground_y_at` + `GROUND_SNAP_PX` / `EMBED_TOLERANCE_PX` + the anti-jitter snap; `step_walker` samples the profile. **DONE (`ffdbe8d`); the tests are the remaining work.**
6. **TECH, GDScript stepper (the LIVE path):** `editor/scripts/world_view.gd` -- `set_terrain_masks` / `build_ground_profile` / `ground_y_at`; `_rebuild_ground_tiles` resolves each tile's mask; `step_walker` takes the profile + snap; delete `build_ground_column_table`, the per-column interpolation hack and `tile_w_px`.
7. **Wiring:** `editor/scripts/main.gd` -- push `terrain_editor.get_terrain_types()` into the world view via `set_terrain_masks`.
8. **DRAW:** `editor/scripts/world_view_ground.gd` -- draw the tile sprites (`res://assets/tiles/{key}_32x32.png`), remove the 4:7 `draw_polygon` and the flat-tile rects; fog/atmosphere goes on its own layer, never by lowering a tile's alpha.
9. **Tests:** GDScript `test_walker_step.gd` (every-pixel-column sweep against the PNG-derived oracle, plus the polygon-removal assertion) and `test_slope_set.gd`; Rust `physics.rs`; the tool's in-source tests.
10. **Gate:** full `cargo test` + the three GDScript suites (exit codes) + run the WHOLE mutation battery (M1-M8, T1-T6); a mutant that does not fire is a void experiment, not a pass.
11. **Docs:** wiki `TDD_Collision-Map-Tooling`, `TDD_Physics`, `TDD_Simulator-Ground-Walk`, the post-mortem, `Home.md`.
12. **Ship:** commit on `feat/ground-profile-impl`, clean independent review, push, merge `--ff-only` to `trunk`.

**Progress at 2026-11-17:** step 5 is DONE (Rust `build_ground_profile` / `ground_y_at` / `step_walker`; `cargo test -p sstd-core` = 149 green). Step 1's `derive_solid_mask()` is written but NOT wired into `main()`. Steps 2 and 6 are `TODO(human)` scaffolds only. Steps 3, 4, 7, 8, 9, 10, 11, 12 are NOT started.

_**@me's directive (2026-11-16): remain skeptical -- as is @me, and so must this session's agents and judges be.** Every premise is checked against the repo, never asserted; a judge blind to the repo can score a plan 100/100 while its premises are wrong (that is what happened here on 2026-11-15). The five findings and the alpha-vs-greyscale measurement live in the plan's `## Second skeptical pass (2026-11-16)` section._

_**@me's directive (2026-11-16): the slope ANGLE is irrelevant and is never an input -- the pattern IS ART -> DATA -> TECH.** The artist drew the climb for how it FEELS (the family's measured rises are 7 to 30 deg and deliberately uneven), so the tool scans the ART for air versus collidable, emits a collision bit-mask as DATA, and TECH consumes the mask. No nominal angle, formula or hand-authored ladder may survive into data or tech; redrawing the art re-derives the mask with no code change._

_**Air rule (2026-11-16, @me's pipeline -- supersedes the same day's ALPHA-FIRST): GREYSCALE -> THRESHOLD -> NO-DONUT CLEAN -> ALPHA MASK.** (1) acquire the NxN tile; (2) greyscale it, to make the air/surface boundary easy to locate; (3) threshold to a 2-valued air(0)/solid(1) bitmask with NO DONUTS -- a solid pixel floating in air becomes air ("pretend it was just dust"); (4) convert that collision mask into an alpha mask. So greyscale is the DETECTOR and the alpha mask is the PRODUCT and the datum the runtime reads (as `solid_mask` bits); alpha is no longer the detector, it is the output. Measured (ImageMagick grey, alpha ignored): `slope_up_1` air = grey 0 (transparent black) vs ground 155; `slope_up_0` = all grey 155 (no air); `slope_30deg_0_0` air = grey ~209-214 (919 px) vs ground ~137. Air is the DARK side for `slope_up_*` and the BRIGHT side for `slope_30deg_*`. @me (2026-11-16): an all-grey tile has NO air (all solid, e.g. `slope_up_0`), and **each tile's air may differ** -- so the rule is PER TILE, not per family. **@me's answers (2026-11-16):** the tool locates a tile's air by anchoring the GROUND at the tile's bottom row (the greyscale value touching the bottom row is solid; the other value is air; a single-grey tile is all solid); the no-donut cleanup removes **FLOATING SOLID SPECKS ONLY** (a solid pixel not connected to the ground becomes air) and does NOT fill enclosed air pockets. Measured per tile (ImageMagick grey, alpha ignored): `slope_up_0` one grey (155, no air); `slope_up_1..6` air 0 / ground 155, bottom row all 155; `slope_30deg_0_0` air ~209 (839 px) / ground ~137, bottom row MIXED (4 greys, so both clusters touch it); the `air` tile one grey (0, opaque). **When the tool cannot resolve a tile it must NOT guess** (@me, 2026-11-16): it flags the tile and draws a red 1px border so the artist/level-designer hand-edits the collision themselves. Flag criteria (@me, 2026-11-16): BOTH greyscale clusters touch the tile's bottom row, OR the greyscale does not split into exactly two clusters; the red 1px border is baked into the emitted `{key}_32x32.png`. Also @me (2026-11-16): a region's colour is read from a uniform 2x2 to 4x4 patch, never a single pixel.

- **Branch (status corrected 2026-11-17):** `feat/ground-profile-impl` (off `trunk`), local tip `87b4900`, tracks `origin/feat/ground-profile-impl` (remote tip `87b4900`), working tree clean. (The earlier "2 commits, NO upstream, NOT pushed" line was stale.)
- **Decided by @me (2026-11-16):** air rule = **greyscale-threshold** (rewritten in the plan); region colour read from a uniform 2x2-4x4 patch; ground anchored at the tile's bottom row; no-donut = floating solid specks only (enclosed air pockets NOT filled); unresolvable tiles flagged with a red 1px border, never guessed. The **MEASURED/UNVERIFIED premise rule** is IN `~/scripts/LLM/skills/plan-ticket-gate/SKILL.md` -- committed as `5a3b40c` on `docs/gate-model-consent` and already an ancestor of `~/scripts` `trunk` (MEASURED 2026-11-17). (The earlier "4 edits applied, UNCOMMITTED" note was stale.)
- **Next move, in order (2026-11-17):** (1) read the independent plan-gate score for `87b4900`; if it is below 90, fix the named findings and re-score in a SEPARATE context (subagent), repeat until it passes. Both @me rulings that were pending are ANSWERED: the tool resolves by fuzzy logic / best guess FIRST with the red 1px border as the LAST resort (posted as the third #126 amendment, https://github.com/HidekiAI/sidescroll-towerdefense/issues/126#issuecomment-6950495262), and the flag STAYS in #126. (2) Mirror the 12-step implementation sequence into the plan as its own section (deliberately NOT done while a judge reads the file). (3) Then work the 12-step sequence at the top of this block, tests first. (4) The SKILL.md premise-label patch is DONE (`5a3b40c`, already in `~/scripts` `trunk`); nothing pending there.
- **Gate:** `cargo test -p sstd-core` plus `test_walker_step`, `test_slope_set`, `test_walker_visibility`, each on its EXIT CODE. `test_screen_store` is known red on 4.7.2 (#85) and is not in this gate. Never `git add -A` (516 MB vendored `editor/addons/`).

_Last updated: 2026-11-15. **#126 plan gate: independently re-scored 55/100 TWICE by two different judges -- NOT a pass.** A third judge returned 100/100; it is REJECTED as a false positive (it contradicts the plan's own text and came from a lenient free model), and it is not evidence. The earlier 55/72/73/100 "Sonnet" scores in this file are UNVERIFIED; the plan's `## Plan gate` section is still a placeholder, so none of them were persisted. Nothing in the code is in flight. #125 merged and CLOSED; #124 DECIDED "for now" and closed; the real slope is #126._

_**@me's decision (2026-11-15): the datum is the per-pixel tile ALPHA, not the hand-drawn 4:7 polygon.** The 4:7 line is to be baked INTO the slope tiles' alpha, so the drawn surface and the collision surface are one datum; the X-by-Y slope tile set ("art defines data, which data defines tech") is the critical piece. Non-alpha collision decisions are to be rejected/redacted (decision 4). Anti-jitter is part of the per-pixel collision implementation; a jitter that survives becomes its own bug ticket. Pass bar stays 90._

_**Calls answered (2026-11-15):** both go live in #126 itself (no separate predecessor ticket). (1) art+data work lives in #126; (2) Option 1a — colour-key the grey `#D1D1D1` background to transparent (fuzz-tolerant, anti-aliased fringe accepted), then `solid_mask = alpha != 0`. Art→data→tech is documented as primary data formatting for maps, scenes, and collisions in the wiki TDD(s)._**

_**Amendment POSTED to #126 on 2026-11-15** (comment https://github.com/HidekiAI/sidescroll-towerdefense/issues/126#issuecomment-6900011032, body saved at `/tmp/user/1000/opencode/issue-126-amendment.md`). It carries: alpha datum; Option 1a colour-key; art→data→tech as primary data formatting for maps/scenes/collisions; on-screen feel = alpha surface; anti-jitter in scope; **AC2 DROPPED** (@me: "AC1 will trump AC2, with AC1 in place we DO NOT NEED AC2") so acceptance is AC1 (feet Y = drawn alpha surface within 1 px at every column) + AC3 (`slope_feet_y_matches_drawn_surface` passes against the alpha surface). Gate bar stays 90._

- **#126 plan gate state (2026-11-15):** branch `feat/ground-profile-126` off `trunk` (`0a75e2d`),
  plan at `docs/PLAN-2026-11-12-ground-profile.md` (uncommitted). **Independently re-scored 55/100
  TWICE by two different judges; the CRITICAL is the `world_view_ground.gd` drawer change redefining
  the AC1 datum** (the plan redraws the ramp from tile alpha while AC1 measures feet against the
  drawn surface). Verified by the judges, re-check before acting: `walking_up_the_ramp_rises_every_frame`
  asserts a constant 1.9048 px/frame rise while the placement's seam steps are 19/5/0/10/1/14 px;
  line 197's "every test is covered by at least one mutant" is false for six named tests; the plan
  calls the issue threads "the review record" while #124 and #126 have no comments; and the AC3 test
  compares against `GROUND_ROW - int(4 * col / 7)`, which is the STEPPED placement, not the drawn
  continuous line (982.86, 964.57, ... 873.14). Three provenance rows are missing (blame author +
  hash + date). Mutant M5 has no red test as written (its target asserts keys 0..6, where the dropped
  term is zero, so a keys 3..6 case is needed). `GROUND_SNAP_PX` needs a stated reason for staying a
  const. The wiki build rule 2 and `TDD_Simulator-Ground-Walk` §2.1/§6 need editing to match the
  terrain-key anchoring rule. **A separate wiki errata:** `TDD_Physics` claims "a ramp is simply
  adjacent values that differ by less than one pixel" -- the shipped art falsifies that (seam jumps
  reach 19 px).

- **Art-pipeline findings (measured 2026-11-15, all from the repo, not assumed):** the datum @me
  chose (tile alpha) needs art that carries alpha, and the repo's slope art does not.
  - `editor/assets/samples/tileset_2-2.png` is `srgb`, **3 channels -- NO alpha channel**. Its "air"
    is **opaque grey `#D1D1D1`** (~30% of the sheet); the slope is a colour boundary, not transparency.
    It is anti-aliased against that grey (blends like `#BCBCBC`, `#B0B0B0`), so a colour-key needs a
    tolerance and leaves a fringe.
  - All 8 sliced `editor/assets/tiles/slope_30deg_{col}_{row}_32x32.png` are **fully opaque** (topmost
    solid row = 0 in every column); their alpha channel is **solid white**. `slope_grass_30deg_4x2.tres`
    (grid 4x2, cell 126x126, margins (104,65), sep (1,1)) is the authoring record; nothing in the
    simulator loads it.
  - The 4x2 set is an **8-cell diagonal across the block**, not a 4-column line: grey-background share
    per cell in reading order = 86, 85, 52, 35, 18, 5, 0, 3 %.
  - It is a **different set** from what the simulator injects: the simulator uses 7 individual
    `slope_up_0..6` tiles (one per column). `slope_up_0` is a fully opaque square; `slope_up_1..6` are
    diagonals at inconsistent phases (first columns at rows 13, 27, 9, 22, 4, 18).
  - **Tooling that already exists (reuse, do not rebuild):** `tools/import-tiles` (808 lines) slices a
    grid (`grid_cols`/`grid_rows`/`offset`/`margin`), auto-detects content bounds, nearest-resizes to
    32x32, writes per-tile PNGs `{key}_32x32.png`, and emits terrain types + tile sets
    (`TileSetDef`: `col_start`/`row_start`/`width`/`height`/`tags`). It is the "collision-mapper"
    @me described. `TileSetGrouping` already stores `source_image` + `src_margins`/`src_cell_size`/
    `src_separation` + per-tile `source_col`/`source_row`, i.e. the single-PNG-sliced-by-region model,
    and `_source_rect()` computes each region. **The one gap:** import-tiles emits the coarse 4-bit
    `sub_tile_mask` (default 15), NOT the per-pixel `solid_mask` the wiki `TDD_Collision-Map-Tooling`
    (#63) specifies. `grep -rn solid_mask --include=*.rs --include=*.gd --include=*.tres --include=*.proto`
    returns nothing -- it is not implemented anywhere yet.
  - **One honest correction for the plan:** the walker is a custom kinematic stepper (GDScript
    `step_walker`, mirrored in `crates/sstd-core/src/physics.rs`), NOT a Godot `PhysicsBody`. Godot's
    cookie-cut gives the slicing and the per-tile region bookkeeping; the collider the physics reads
    is the mask, not Godot's collision polygon.

- **Issue #124 (avatar hops one tile per column on the 30° slope):** fix merged to `trunk` as
  `47716b9` (`feat: smooth slope climbing via pixel-based interpolation`). The working branch
  `fix/smooth-slope-climbing-via-interpolation` was merged non-squash and deleted.
- **What the fix does:** `step_walker` in `editor/scripts/world_view.gd` interpolates between the
  current column's table Y and the next column's table Y over the column width, instead of
  snapping to the current column's Y. `build_ground_column_table` is unchanged.
- **Probe re-run 2026-11-11** (`/tmp/user/1000/opencode/probe_slope_hop.gd`):
  - Largest single-frame rise on the climb: **3.33 px** (was 32 px).
  - The climb is still not continuous: it rises 3.33 px per frame for about 10 frames, then
    holds flat for about 9 frames, because the table is still the per-column step function and
    the interpolation only spans a column where the next column's row differs.
  - Feet Y vs the drawn 4:7 ramp at column centres: **+9.14, +27.43, +13.71, +32.00, +18.29,
    +36.57, +22.86 px** for local columns 0-6. The collision surface still does not match the
    drawn polygon.
- **Regression guard:** `test_walker_step.gd` `slope_feet_y_matches_drawn_surface` asserts the
  max single-frame rise stays under 16 px and the walker reaches the top; it does **not** assert
  feet Y equals the drawn surface at every column. That literal acceptance line is still unmet.
- **DECIDED 2026-11-12: accept the partial fix for now.** #124 stays CLOSED with no reopening;
  @me's words were "accept (for now)", so the hop is tolerated, not called fixed. Nothing more is
  done under #124.
- **NEXT MOVE, and the reason this section exists: make the slope a REAL slope.** The accepted fix
  is an interpolation between two plateau Y values, so it stalls and still misses the drawn line
  by 9 to 37 px. The work is tracked as **#126** (`feat(sim): continuous ground_y(pos_x) so the
  collision surface matches the drawn slope`): replace the per-column table with a run-based
  profile so a contiguous slope run is one line, and have BOTH steppers read it
  (`editor/scripts/world_view.gd::step_walker` is the live path; `crates/sstd-core/src/physics.rs`
  has no within-column interpolation at all). Its acceptance line is #124's unmet criterion 3:
  feet Y equals the drawn surface at every column, to within 1 px. That is a representation
  change, so plan it before coding.
- **Two directives must not be folded into #126:** collision bounds instead of the avatar's art
  (`bounds_bottom_y` / `contact_y`), and graduated fall damage from fall height. Each needs its
  own ticket.
- **`physics.rs` legibility (#125, MERGED to `trunk` as `8809c55`..`5798908`, issue CLOSED):** the
  short bindings are gone (`tile`, `move_axis`, `tile_left_edges_px`, `world_tile_col`,
  `surface_row`), the dead `ground_profile_px` that returned `Vec::new()` is deleted, three helpers
  were extracted (`world_bounds_px`, `world_tile_col_at`, `fall_damage_points`), and the magic
  numbers are named (`DEFAULT_FRAME_SECONDS`, `FALL_DAMAGE_THRESHOLD_PX`, `FALL_DAMAGE_POINTS`).
  The module header now states that the GDScript walker is the live path and that the Rust model
  has no slope interpolation. **Behaviour is unchanged**, and the public type and field names stay
  because they are the `walk_step` wire contract. Five tests were added (fall damage past the safe
  drop, no damage within it, a flat-floor walk, a step up onto higher ground, an airborne step up
  onto a ledge); a seven-mutant battery: M1-M5 and M7 each turn exactly one physics test red, M6
  (step up disabled) turns both step-up tests red. The right world
  bound is still the last tile's LEFT edge, pinned by test and documented rather than changed.
- **All other gates:** `test_walker_step.gd`, `test_image_to_map.gd`, `test_terrain_brush.gd`,
  `test_override_merge.gd` exit 0; `cargo test -p sstd-core` **147 passed** (142 before #125).
  `test_screen_store.gd` is known red on Godot 4.7.2 (#85, unrelated).
- **Two directives recorded in `AGENTS.md` on 2026-11-12, neither started and neither filed as an
  issue:** collision must key off explicit bounds (`bounds_bottom_y` / `contact_y`, a hotpoint or a
  contact line, never the avatar's art, and a body is struck from the sides too), and fall damage
  must follow Newtonian physics and scale with the fall HEIGHT rather than paying a flat 1 HP past
  32 px. The second one is a divergence from `TDD_Physics`, which already specifies
  `fall_damage_per_tile` (10 HP per tile) and `fall_damage_min_tiles` (3 tiles).
- **Branches:** `chore/physics-rs-legibility`, `docs/fall-damage-wiki-divergence`,
  `docs/checkpoint-125-merged` and `docs/postmortem-125-legibility` (wiki) are merged and deleted
  locally; `origin` may still carry them, so the remote delete is @me's to run. `docs/m1-probe`
  (code) and `docs/parallax-step4-corrections` (wiki) are unmerged and untouched.

### Superseded: #122 state, 2026-11-09

_Last updated: 2026-11-09. **Nothing in flight. #122 is fully implemented on `trunk`; the slope set is
documented and guarded, and every acceptance line including "blocked by non-walkable" now holds. The
issue itself is still OPEN by choice, not by a gap.**

- **Objective (from @me):** the next best slope angle besides 45° that tiles perfectly on the square
  grid and works for the hill terrain; close to 30°.
- **Decision:** the 4:7 (rise 4 over run 7, ≈29.7°) slope is the next best slope that tiles perfectly.
  It is authored as an indivisible **set**: start and end flush on flat ground (`ground_row`).
- **Landed and merged to `trunk` (`fdcf259`):** `inject_simulator_hills` emits
  `slope_up_0`..`slope_up_6` for screen `"1,0"` cols 0-6 and screen `"2,0"` cols 14-20 via
  `y = ground_row - int(4 * col / 7)`; `CANONICAL_TERRAIN` carries the seven keys; the seven
  authored PNGs exist; `world_view_ground.gd` draws one ramp polygon per contiguous slope run.
  `editor/tests/test_slope_set.gd` pins the keys, rows, re-injection no-op and ground table, and was
  mutation-verified three ways (divisor, column index, key prefix).
- **Docs of record:** `TDD_Simulator-Ground-Walk` section 7 on wiki `master` (`b64b605`) now specifies
  the set, its keys, the formula, the injection sites, the draw path and the legacy keys; the page was
  wrong before (it said `hill_up`, duplicated heading 7, and claimed no slopes).
- **"Blocked by non-walkable" fixed and merged to `trunk` (`a4085da`):** both steppers
  (`editor/scripts/world_view.gd`, `crates/sstd-core/src/physics.rs`) now refuse a horizontal step
  into a column with no walkable surface, staying on the previous pixel; refusing rather than
  clamping keeps it direction-agnostic. `test_walker_step.gd` guard `blocked_by_wall` covers right,
  left and a 120-frame no-creep repeat; Rust test `walker_blocked_by_missing_surface` takes the
  suite to 142. Both guards mutation-verified. Plan and measurements:
  `docs/PLAN-2026-11-09-122-wall-block.md`. Consequence recorded on the issue: a pit edge now blocks
  like a wall, because the ground table cannot tell them apart.
- **Issue #122 is OPEN by choice, with every acceptance line met.** Closing it is @me's call, not a
  pending fix.
- **Not in flight, not started:** any down-slope set (`hill_down` remains a legacy key with no art),
  and separating a pit from a wall so a pit can be entered.

- **Nothing is in flight.** All eight steps of `docs/PLAN-2026-11-09-122-slope-docs.md` (S0-S8) are
  `done`; its ledger is the record of how each was measured. Neither working branch exists any more:
  `test/122-slope-set` and `docs/122-slope-set` were merged and deleted.

- **Read this and then stop:** `docs/PLAN-2026-11-09-122-slope-docs.md` if you need to know how
  anything above was measured. Do not reconstruct state from chat.

- **Corrections of record, measured 2026-11-09 — do not re-derive, and do not write a doc that
  contradicts them:**
  1. The Simulator ground layer never loads a tile PNG. It draws polygons and rects
     (`world_view_ground.gd`); `{key}_32x32.png` is read only by the palette and editor grids.
  2. Simulator walkability is a heuristic (`world_view.gd:633-635`: walkable unless `air`, `wall`
     or `lava`), not a terrain-table lookup.
  3. `hill_up` / `hill_down` are legacy keys: still in `CANONICAL_TERRAIN` and used by
     `test_walker_visibility.gd`, no authored PNG, no longer emitted by `inject_simulator_hills`.
  4. The "all six suites exit 0" claim below was made on this host, where `test_screen_store` is
     known red (#85, 4.7 `ZIPPacker` directory entry). S1 measures and settles it.

### Superseded: step-4 (#107) state, 2026-10-19

_Last updated: 2026-10-19, after the push. **Both repos are pushed and merged.** Code repo `trunk` =
`af0684b` (was `cf0962d`); wiki `master` = `ff55a33` (was `d32efd2`). The gated plan of record is
`docs/PLAN-2026-10-16-parallax-step4.md`. Steps 1-3 and step 4 (#107) all landed, together with the
backdrop-PNG cleanup (#68) and the `editor/README.md` 4.7 toolchain fix (#102). The Godot MCP bridge
(#112) is installed and verified; the Godot 4 learnings ledger (#113) and the step-1/2 wiki rows
(#104/#105/#106) are on the wiki. **Closed on GitHub:** #107 (gate evidence in its closing comment),
and #88, #99, #101 (each closed with its own mechanism-level evidence — their root causes, the
`Parallax2D` registration and `backdrop_preview.tscn`'s hard-coded `position.y`, were deleted by this
step in `547d500`), and #113 (the `Godot4-Learnings.md` page, now on wiki `master`). **Open:**
#116-#118 (the follow-ups below), plus the pre-existing #111, #112, #114, #115. **Not yet
verified:** #104, #105, #106 — their steps are code-complete and now on `trunk`, but nobody has
re-read their acceptance criteria against the landed code, so they stay open until a session does
that and either closes them or says why not.

**NEXT MOVE: step 4 is finished; nothing is in flight.** Open follow-ups, in the order I would take
them:

- **#116** — the emitted `parallax_depth` is a 4-decimal rounding of the retired rates (a
  deliberate #105 decision), and that is what caps G5's whole-period pan gate at a measured 152 px
  budget instead of an exact assertion. Decide: author exact rates, or document the budget.
- **#117** — a layer's `fill_above`/`fill_below` rects are drawn under the *previous* layer's
  transform (`parallax_draw.gd:76-82` run before the `draw_set_transform` at `:91`). Latent only
  because every non-first fill is transparent: the mutations measured 0 differing pixels of
  2073600. A per-corridor stack (#110) would hit it.
- **#118** — `editor/project.godot`'s `window/viewport_width|height` sit under `[rendering]`, so
  they are inert and the editor window opens at Godot's default 1152x648, not the 1920x1080 the
  parallax screen geometry assumes.

Left undecided for @me on 2026-10-19 (@me chose "land the PNG cleanup + README fix" only): the
2026-10-15 lint sweep (~55 warnings, its own branch off `trunk`, see its section below), #115's
Home.md self-check amendment, and the superseded-branch cleanup below.

0. **Gate re-run at `af0684b` (2026-10-19, every suite gated on its exit code):** `cargo test -p
   sstd-core` **138 passed**; `test_screen_store`, `test_image_to_map`, `test_terrain_brush`,
   `test_override_merge` all exit 0 (`failures=0`); `test_parallax_backdrop` exit 0;
   `test_parallax_pixels` (G5/G7/G12/S1) exit 0 under `DISPLAY=:0.0`; `test_parallax_stretch` (G19)
   exit 0 under `DISPLAY=:0.0`. `parallax_draw.gd` unchanged from HEAD (every mutation reverted,
   `git diff` empty); `editor/project.godot` drift-free; both throwaway probes (`probe_g5_pan.gd`,
   `probe_g12_fillorder.gd`) deleted after the battery.

**Finding — the wiki's `master` was force-updated while our clone was stale.** `git fetch` reported
`+ 67850b1...d32efd2 master -> origin/master (forced update)`: the remote had rewritten history,
collapsing 53 local commits into 4 with the same end state. **`git diff --stat master origin/master`
was empty** — that is the check that distinguishes a history squash from a content change, and it is
why the recovery could proceed: nothing was lost, only the lineage differed. Consequences: no branch
built on the old base could fast-forward into `master` any more, and the already-pushed
`docs/parallax-step4-corrections` carried 56 commits `origin/master` did not want. Fixed by
cherry-pick, never force — 7 commits re-landed on a new branch off `d32efd2`
(`docs/parallax-wiki-catchup`), two `TODO.md` conflicts resolved by keeping the newer TS71 text and
taking their TS72-TS74 table, then `git merge --ff-only` into `master`. A duplicate TS72 ID fell out
of the merge (the step-1 row and the #113 ledger row both claimed it); the newer ledger row became
TS75. Local `master` was repointed at the remote tip with `git reset --hard origin/master` first —
byte-identical trees, so its 53 pre-squash commits remain reachable through the old-lineage branches.

**Branch state (deletions are @me's call; none taken).** Code repo: `trunk` `af0684b` current;
`feat/parallax-step4` and `chore/backdrop-png-readme` merged into it. Superseded, awaiting a
decision: `chore/backdrop-png-cleanup` and `docs/fix-readme-godot-version` (their two commits landed
via `chore/backdrop-png-readme`), and `docs/m1-probe` (3 commits — its queued auto-build instruction
is already implemented on `trunk` by #103's `bridge_autobuild`, and its AGENTS hunk moved the Python
section that `trunk` still has intact). Wiki: `master` `ff55a33` current; `docs/parallax-wiki-catchup`
merged; superseded: `docs/parallax-steps-1-2` and `docs/godot4-learnings` (both re-landed as
cherry-picks) and the remote `docs/parallax-step4-corrections` (points at the pre-squash lineage).

1. **What the step-4 branch carried, now on `trunk`** (`git log --oneline fba7b46..af0684b`):
   - `f71b465` -- `ValidationMessage.layer` + `journal_line` + the bridge's `stack_invalid` journal
     (core 137, bridge 26).
   - `879ac6d` -- **the integer/float wire fix.** GDScript's `JSON.parse_string` stores every number
     as a float and `JSON.stringify` re-emits `2` as `2.0`, which serde's `u32`/`u64`/`i32` refused,
     so the real `neutral_default` stack could never load. `de_widest_integral` reads an integral
     JSON number whichever way it was spelled, narrowed by `de_integral_u64`/`u32`/`i32`; a
     fractional or out-of-range value is still rejected, so the fix widens the representation, never
     the value set. Found by smoke-running the loader; `test_parallax_bridge.gd` never saw it because
     its hand-built Dictionary keeps INT Variants and never crosses JSON.
   - `88c10dd` -- `parallax_draw.gd` on `_draw()` + a rewritten `test_parallax_draw.gd`. **Defect
     found and fixed here:** `_fill_colour` trusted `String.hex_to_int()`, which *parses until it
     hits an invalid character*, so `"#8A98ACGG"` returned `0x8A98AC` and silently drew a wrong
     band -- length alone never guarded it, the length is still 9. Mutation-verified.
   - `37e8734` -- `world_view.gd`, `parallax_loader.gd`, `camera_controller.gd`, `game_view.gd`,
     `world_view.tscn`, `game.tscn`, the `neutral_default` stack assets, and the deletion of
     `probe_parallax_scroll.gd`.
   - `a8b533a` -- the checkpoint rewrite; `3471915` on the wiki repo extends the Godot4 learnings
     ledger with three measured findings (the gdext `ClassDB.instantiate` leak, every JSON number
     crossing as float, `hex_to_int()`'s partial parse).
   - `547d500` -- **§4.9 + §4.7 as shipped.** `main.tscn`'s Simulator tab is now one
     `WorldView` instance: `BackdropView` (SubViewportContainer + Camera2D), `BackdropStrip`,
     `backdrop_preview.tscn`, both scrollbars (`BackdropScroll`/`BackdropScrollV`), `simulator.gd`,
     `simulator.tscn` and `main.gd`'s dead `_instantiate_tabs()` are all deleted. The scrollbars
     were NOT rewired -- they scrubbed the old camera, and a scrub control for the new stack is its
     own ticket (stated in the scene's comment). The §4.7 `StatusLabel` lives in `world_view.tscn`,
     not the tab, because §6 requires the suite to instantiate `WorldView` and a HUD hung on the
     tab would be unreachable from it. Measured by probe: WorldView's rect equals the Simulator tab
     rect exactly (189x33 headless), SubViewport tracks the container (not the 2x2 degenerate
     case), HUD reads `neutral_default`.
   - `ba9f605` -- **`test_parallax_backdrop.gd` rewritten for WorldView**, absorbing the D1-D5
     drawer seam and deleting `test_parallax_draw.gd` (a seventh file §6 never listed). 30 checks,
     rc=0; 5/5 mutants killed. Harness lesson recorded in plan §6: a recording loader assigned
     before `add_child` is silently overwritten by `WorldView._ready()`, so the first run read
     "got 0 requests" while the bridge had plainly been queried -- a wrong experiment that looks
     like a vacuous guard.
   - `ccb96ef` -- **`test_world_view.gd` (24 checks: shape, anchors, G3) and
     `test_camera_controller.gd` (18 checks: G6, G13, small-bound clamp)**, 9/9 mutants killed.
     Timing trap recorded in-suite: `SceneTree.process_frame` resumes BEFORE node `_process` runs,
     so a target set and awaited once gets zero process frames -- the loop needs one priming frame.

2. **Measured, not assumed.** The real `neutral_default` stack loads **7 placements, 6 back / 1
   front**, at a 1920 x 1080 viewport, with the anchor defaulting to the view centre (960, 540).
   All four pre-existing GDScript suites still exit 0, and `editor/project.godot` is drift-free.

3. **Deviation recorded in plan section 4.1: the WorldView root is a `Control`, not `Node2D`.** A
   Control resolves anchors against the nearest Control ancestor, so a Node2D in between sends
   `SubViewportContainer`'s anchors to the root viewport and the simulator view fills the whole
   editor window instead of its tab. Verified by instantiating the scene with and without the
   Control chain.

4. **`ClassDB.instantiate("SstdBridge")` leaks one instance at process exit** -- reproduced with the
   reference held by a Node that is `queue_freed` first, and with a bare instantiate-and-drop.
   Pre-existing (`main.gd` does the same), no runtime consequence. Documented on the Godot4
   learnings page, **not fixed here** -- fixing it belongs to whatever owns the bridge object's
   lifetime, not to a feature branch.

5. **Still owed on this branch, in order:**
   - **DONE 2026-10-19: the two windowed suites committed** in `bdeda7e` (`test_parallax_pixels.gd`,
     `test_parallax_stretch.gd` and both `.uid`, generated with `--import`; `project.godot` showed no
     drift afterwards). Section 6's six suites are all green.
   - **The measured findings behind the two windowed suites** -- full detail in plan section 4.3.2;
     the short form, because each one cost a run to learn:
     - G5 at zoom 0.25 asserts **in-frame periodicity** (480 px = 1920 x 0.25), not a pan: a
       whole-period pan leaves `first_copy_left_px` at the same phase, so it is identical in the
       correct code and in the scale mutant -- the pan form could not have killed the mutation.
       Measured 0 mismatches correct, **1205** under the mutant, on probe row 620.
     - G5 at zoom 1 uses `ZOOM_ONE_PAN_PX = 19200` (whole-period for all six strips: rates k/10 with
       k = 1,2,3,5,7,13) with a MEASURED 152 px residual and `_pan_rounding_budget()` = **160**
       (152 + 8). Cause of the 152: `stack.json`'s `parallax_depth` is rounded to 4 dp
       (2.3333, 0.4286, -0.2308), making Foreground's rate 1.30000522 -- 0.9984 px over 19200, which
       `snap()` (`floor(v + 0.5)`) turns into a real 1 px shift on rows 1015..1033. No pan exists
       where all six are whole-period (searched P = 19200m ± 12, m <= 400), so a 0 budget is
       unachievable; full-precision depth is a `gen-backdrop` (#105) follow-up, not #107's.
     - **Harness lesson:** pan invariance needs the **DELTA** to be whole-period. Capturing at the
       scene default (0,0) then panning to 960 + 19200 gives delta 20160 = 1 period + 96 for Sky, and
       a 1202952 px diff that is not a defect. `move_camera_to(960.0, 540.0)` before the first
       capture restores 152.
     - The backdrop **does** draw at startup (0 clear pixels of 2073600, 7 placements, camera (0,0),
       zoom 1) -- the old "runtime rendering unverified" note is closed as a non-defect for the new
       stack.
     - Per-layer `period_px` at 0.25: `sky` and all five strips 1920, **`sun` 130** (single
       non-tiling sprite). So full-frame 480-periodicity is FALSE at 0.25 (1590 mismatches: the sun
       band rows 455..485 plus 1-5-per-row coverage edges in rows 595..661), row 250 is flat
       (variation 0, a vacuous probe), and row 620 is the probe row (variation 204 correct, 78 under
       the mutant).
     - **Mutations: 2 killed, 3 unkillable as scoped, all recorded.** Killed: skip `fill_below`
       (G12's first check, not G7), drop the placement scale (G5 zoom 0.25). Unkillable: a fills-first
       pass and the screen-space-fill reset are both **bit-identical** to the correct render at zoom
       1/0.5/0.25 x cam y 540/1540/2000/3000 (0 of 2073600 pixels differ, 10 comparisons), and
       `texture_filter = LINEAR` is a no-op at zoom 1 where the strips draw 1:1. The reason is data:
       every fill in the stack is the same ground colour and the front-plane foreground art already
       covers the band the fills would cover.
     - **Latent defect, deliberately NOT changed** (the drawer matches plan 4.3 verbatim and no gate
       fails): the fill rects are screen-space by construction but are drawn under the **previous**
       layer's transform, so only each plane's first layer paints a screen-space fill. The one-line
       fix was measured as a render no-op with this stack; it wants a stack with a mid-layer fill
       colour that differs from the ground, plus its own ticket.
   - **DONE 2026-10-19: the docs pass.** Plan sections 4.3/4.3.1/4.3.2/6/7/9 are updated (the two
     falsified derivations corrected in place, not deleted; 7's third correction was checked and is
     already discharged — the wiki carries a full G19 row). Wiki corrections landed as `d9fbe19` on
     `docs/parallax-step4-corrections` in the wiki repo (G5/G7/G12/S1 rows plus a measured-findings
     note; TODO.md TS71 `designed` -> `in progress`). The #107 comment is posted:
     https://github.com/HidekiAI/sidescroll-towerdefense/issues/107#issuecomment-6278295786
   - **The single push-permission ask** covering both repos, bundling the `chore/backdrop-png-cleanup`
     (`77822e8`) drop/hold decision, the unattempted lint directive, and #115's self-check-loop
     amendment. Also to be raised, not silently filed: the full-precision `parallax_depth` follow-up
     (#105-side), the fill-transform latent defect, and `editor/project.godot`'s inert viewport size
     (it sits under `[rendering]`, so the window opens 1152x648 and both windowed suites must set
     `root.size` themselves).

6. **Two follow-ups are filed rather than smuggled in**: #114 (the `sstd-core` version bump that
   `journal_line`'s signature change needs) and #115 (the Home.md indexing self-check reports
   sub-pages of `TDD_Parallax-Scrolling` as unindexed because it only greps `Home.md`, not the
   parent page that links them).

7. **Plan gates**: the step-4 plan passed the ticket gate at **95/100** (four evaluator runs; the
   score history is in the plan's section 9). Three scope clarifications are posted on #107 as a
   comment -- that comment is what makes the plan traceable, since the gate sees only the ticket.
   @me's directive to keep the Sprite2D-per-placement design as an explicit alternative view is
   discharged: plan section 2.2 carries it, section 2.1 (`_draw()`) is the design of record, and
   the alternative is documentation rather than a third tiling path, because #107's criterion 2
   enumerates exactly two.


---

## Parallax progression log

Newest first. The CURRENT STATE block above is the resume point; this is "what was done, in order,
with evidence"; the wiki `TODO.md` is the design/work tracker. Every entry names its commit.

### 2026-10-19 — step 4 (#107): the two windowed suites, green, with the mutation battery

The last code-shaped work of #107. Both suites need `DISPLAY=:0.0` (headless `RendererDummy` has no
framebuffer); both set `root.size = 1920x1080` themselves because `project.godot`'s viewport size is
inert (misplaced under `[rendering]`, so the window opens 1152x648). Full measurements are in plan
section 4.3.2 and the CURRENT STATE block above; this entry is the order of events.

1. `test_parallax_pixels.gd` (G5, G7, G12, S1) and `test_parallax_stretch.gd` (G19, 5 checks) both
   exit 0. Suites are still untracked at the time of writing; the landing commit adds both plus
   their `.uid`.
2. **G5's form changed because the plan's form was measured unfalsifiable.** A whole-period pan
   leaves `first_copy_left_px` at the same phase, so the frame is identical in the correct code AND
   under the scale mutation — the pan could never kill it. Zoom 0.25 now asserts the plan's literal
   claim in-frame (period 480 across probe row 620): 0 mismatches correct, 1205 under the mutation.
   Zoom 1 keeps the 19200 pan (whole-period for all six strips) with the measured 152 residual and a
   160 budget, replacing the plan's implicit 0.
3. **Three of the plan's derived claims were falsified and are corrected in the plan, not deleted:**
   (i) G7's overhang colour is ground-coloured ART, so skipping `fill_below` leaves G7 green — the
   fill mutation is killed by G12 instead; (ii) G12's "a fills-first pass would put sky between the
   hills and the ground" is measured as a **bit-identical** frame (0 of 2073600 pixels, at five
   cameras and three zooms), so that mutation is unkillable because it is a no-op; (iii) the
   `texture_filter = LINEAR` mutation is a no-op at zoom 1 where strips draw 1:1.
4. **Honest mutation count for these two suites: 2 killed, 3 unkillable**, the unkillable three
   each explained by measurement rather than excused.
5. **A harness bug of my own, recorded:** capturing before `move_camera_to(960, 540)` made the pan
   delta 20160 (Sky drifts 1 period + 96) and produced a 1202952 px "defect" that was not one.
   Pan invariance depends on the DELTA being whole-period. The same probe closed the old
   "runtime rendering unverified" note: the backdrop draws at startup (0 clear pixels of 2073600).
6. Two throwaway probes (`probe_g5_pan.gd`, `probe_g12_fillorder.gd`) were deleted; `parallax_draw.gd`
   is at HEAD (`git diff` empty after every mutation).

### 2026-10-15 — step 2 (#105): the six blocks, green and mutation-verified

@me had no time this week and asked the assistant to write the blocks it had scaffolded for
@me, then review what it wrote (the comprehension gate therefore reviews written code, not a
hand-off).

1. Wrote all six `TODO(human)` blocks in `tools/gen-backdrop/src/main.rs` — `layer_specs`,
   `make_sky`'s disc removal, `make_sun`, `edge_row_fill`, `art_facts`, `build_stack` —
   uncommenting the samples and deleting the scaffold's `INTENT:` / `// was:` handoff prose
   (the `INTENT (permanent)` doc comments from step 1 stay; they are permanent documentation).
2. Fixed the U18 oracle distance (deviation 1 in the state block) — the scaffolded test could not
   pass at 1 level against a rasterised disc. `cargo test -p gen-backdrop` 28 passed.
3. Cleared `gen-backdrop`'s four clippy warnings (deviation 2 in the state block).
4. Committed: **`0a3a504`** `feat(gen-backdrop): fill the six step-2 blocks, crate green 28 (#105)`.
5. Mutation-verified the four new tests (plan section 8, exit condition 4) — **M1-M6 all caught**,
   each with a real assertion and real values:
   | Mutant | Caught by | Failure line evidence |
   |---|---|---|
   | M1 U18 `alpha = 1 - disc` | U18 | U18 assertion |
   | M2 U18 drop the `0.85` | U18 | U18 assertion |
   | M3 U22 constant fill | U22 | `left: Rgba(138,152,172,255)` vs `right: Rgba(135,149,169,255)` |
   | M4 U22 mean over row `height/2` | U22 | `left: Rgba(160,172,187,255)` vs `right: Rgba(135,149,169,255)` |
   | M5 U22 no fully-opaque guard | U22 | Sun `left: Rgba(255,250,238,255)` vs `right: Transparent` |
   | M6 duplicate `draw_order` | validate-accepts | `Stack::validate` rejects |
6. Real-size run (exit condition 5): 7 PNGs + `stack.json`; AC-1 verified by listing, `file(1)`
   widths (sky 1920x1080, sun 130x130, others 1920x{320,384}) and `grep -c '"inf"'` == 0.
7. This checkpoint + the plan's completion note (commit follows this one).
8. Re-verified the U18 deviation's magnitude by measurement (the numbers were in the code comment and
   commit message but in no log): the nominal-distance oracle was restored temporarily, only U18 run,
   then reverted with `git checkout`. It fails at 0.85r / 22.5 deg / channel 0 with
   `the sun composites to 175 but the old blend was 177` — the documented 2 levels, at the documented
   angle, where `r = 64.8` px puts the nearest pixel 0.34 px further out. Full arithmetic in the plan's
   deviation section. Tree reverted clean (`git status --short` empty for the source file).

### 2026-10-15 — the wiki brought in line with the two landings (3 commits on its doc branch)

The design of record still said "nothing built in this run" and the Verification page "none of these
tests exist yet", both false since steps 1-2 landed.

1. `TDD_Parallax-Scrolling.Data`: the example's six bare `parallax_depth` numbers retagged to
   `{ "kind": "finite", "value": n }` (the defect detailed in the state block above). Commit
   `5fc2c4f`.
2. `TDD_Parallax-Scrolling.md`: dropped "nothing built in this run", added a Status column (steps
   1-2 `code-complete` with evidence, step 3 next, steps 4-6 pending, step 7 deferred by design),
   and the TL;DR's "Design only" line corrected. `TDD_Parallax-Scrolling.Verification.md`: the
   "none exist yet" paragraph replaced with what exists, checked against the code rather than the
   ticket text (U23 is a real test at `parallax.rs:1811`; U12 is absent). `TODO.md`: TS72/TS73/TS74
   plus TS71's "implementation pending" corrected. Commit `31c1659`.
3. The wiki's parallax doc branch is **3 commits ahead of its `master` and unpushed** (the wire-form
   switch `4df7833` from the previous session was left unmerged too). Renamed to
   `docs/parallax-steps-1-2` so the name matches its content. Its merge back to `master` is part of
   the same "ask once for push permission" step as this repo's branch.

### Pending — 2026-10-15 lint sweep (its own branch off `trunk`; NOT on `local/feat/parallax-step2`)

@me's directive "fix|adjust all lint warnings and errors" reaches crates the #105 plan forbids
touching, so it must not ride the step-2 branch. ~55 warnings, categorised in the state block
above. Mechanism: branch off `trunk`, fix per category, keep `cargo test -p sstd-core` at 134 and
the four GDScript suites at their baseline, commit per crate, then hand @me the merge decision.

---

### Pre-#104 record (history, not state)

Everything from here to the next `##` heading describes the retired B1/B2/M-1 restack plan
and the branches it rode. Read it for history only.

**KNOWN RED at HEAD — `test_parallax_backdrop.gd` hangs (exit 124), run it with
`timeout 60`.** B1 moved `BackdropStrip` to `BackdropView/Viewport/BackdropStrip`, so the
script's `get_node("BackdropStrip")` at line 66 returns null and line 67 null-indexes
before any `quit()` — the same hang mechanism the plan documents for
`probe_parallax_scroll.gd` (§3). On trunk it exited 1 instead (no `Camera2D` then, so the
explicit `camera == null` → `quit(1)` branch fired). It is NOT one of the four
AGENTS.md-gated suites, which are green. Do NOT patch it on this branch — plan §8 assigns
the rewrite to B4; a path patch at B2 is scope creep.

**Objective:** restack the #68 parallax background on the official
[2D Parallax tutorial](https://docs.godotengine.org/en/stable/tutorials/2d/2d_parallax.html)
— in-repo generated layers, a visible and correctly framed backdrop, provable two-axis
scroll.

**State: `trunk` = `353d15d`, level with `origin/trunk`.** It went up via the working branch
`docs/parallax-tickets`, merged 2026-10-10; the range `bcb612c..353d15d` also carried the
pre-existing `af14b64` ("current snapshot", non-Conventional, not authored here, not
rewritten). Local branches still open, both unpushed and neither mine: `docs/m1-probe`
(`493b5fe`) and `docs/fix-readme-godot-version` (`594cffb`). `refactor/backdrop-subviewport`
no longer exists, so the B1/B2 paragraphs below are history, not state. The wiki `master` is
at `67850b1`, level with its remote. The parallax
work that this block now tracks has **no code yet** — #104-#110 are the plan, filed
2026-10-10. #85, #89, #96 are all CLOSED (see the pre-#104 record below for their commits).
The four AGENTS.md-gated suites: three green, and `test_screen_store` verified green on 4.7.2
on 2026-10-10 — note its AGENTS.md baseline line is stale (see the conflict note above).**

**DECIDED 2026-10-06 — restructure, do not re-tune `Backdrops.position`. #101 filed.**
The stack lives in the editor's **root** viewport, and that one fact causes both open
parallax defects. `parallax_2d.cpp` `NOTIFICATION_ENTER_TREE` joins
`__cameras_<viewport_rid>`, so camera coupling is keyed **per viewport**: a `Camera2D` in the
root viewport rewrites that viewport's `canvas_transform` and displaces every Control (why
#89 deleted it — that call was correct), while a `Camera2D` inside a `SubViewport` cannot
reach the root canvas at all. So the camera can return, scoped. With no camera,
`_camera_moved` never fires and `screen_offset` stays at its zero-initialised default, so
`_update_scroll()` reduces to `pos = -fposmod(-scroll_offset, repeat_size)` per layer and the
six bands wrap at six different rates — **#88's stagger, and unfixable by a parent
translation**. Fixing it means deriving the framing instead of measuring it.

- **Ticket: #101** (refactor, 8 acceptance criteria A1-A8). **Plan:
  `docs/PLAN-2026-10-06-subviewport-backdrop.md`**, gated against #101 at **97/100** across
  three iterations (59 → 95 → 97). Gate lessons in `.rsi_memory/plan-ticket-gate.md`, task
  playbook in `.rsi_memory/sstd-parallax-subviewport-restructure.md` (workspace root, outside
  both repos).
- **The gate found two defects in the ticket, not the plan.** A5 was unachievable: the suite
  never reads `manifest.json` (`EXPECTED_LAYERS` is a hand-maintained copy), so mutating the
  manifest reddened nothing — B4 now loads `scroll_scale` from the manifest while texture
  file names stay hardcoded literals. A8's "four comments" was inherited from #92, whose
  ranges predate `81405c9`; the real target set is three (`simulator.gd:2-14`,
  `backdrop_preview.tscn:31-34`, wiki §3).
- **#92 needs re-scoping** — its count and line ranges are stale after #89. Follow-up, not
  part of #101.
- **Interim RED, recorded deliberately:** `test_parallax_backdrop.gd` exits 1 on `trunk`
  (`BackdropStrip has no Camera2D to drive`, line 69; assertions at 114-198 never run) and
  `probe_parallax_scroll.gd` hangs (exit 124 under `timeout 60`, reads `L2_near`, a layer name
  retired by the restack). Neither is in `AGENTS.md`'s gate list, which is why nothing caught
  them. B4 rewrites the first and **deletes** the second. The four AGENTS.md-gated suites are
  green.

**Open: #99 — the Simulator's ground bands are clipped at windowed window sizes.**
`editor/scenes/backdrop_preview.tscn:34` sets `Backdrops.position.y = -291`, placing the
Hills/Forest/Foreground bands at y 709..1029, so the tab must exceed 709px tall for any of them
to show. The value at `37d0fda` was `-780`, placing them at y 220..540. The `-291` measurement
checked for sky gaps, not for the ground bands staying inside the tab.
~~A value revert is likely the fix~~ **SUPERSEDED 2026-10-06:** #88 measured that the uniform
zoom behind a revert collapses the bands (Hills 11% → 1%), and the `-291`/`-780` choice was
never a free parameter — it is the symptom #101's B3 replaces with a derived value. #99 closes
under #101's A3, not by picking a better constant.

**A reported "terrain disappeared" regression in the Simulator tab was investigated and NOT
reproduced.** A commit-by-commit walk of `37d0fda..aebdea3` (five steps, editor restarted each
time) showed the terrain present at every commit. What actually changed at `e0e5677` is
window-size *dependence*, not disappearance. Full table, asset geometry and a void-experiment
note are in `docs/evidence/issue-99/bisect-log.txt`.

### SUPERSEDED 2026-10-05 — the #89 gate and the "decision waiting on @me"

> The gate below no longer blocks anything. #89 closed and its fix landed as `81405c9`, so the
> sixth iteration was never run. Retained as the historical record of that gate, per the
> supersession note at the top of this file. The live open item is #99 above.

Bar is 90, ticket alone, evaluator never sees the planner's reasoning.

| iter | score | outcome |
|---|---|---|
| 1 | 84 | FAIL, no CRITICAL, 2 uncovered clauses |
| 2 | 91 | **invalid pass** — evaluator was fed the task prompt's D clauses as normative |
| 3 | 88 | FAIL on bar, no CRITICAL, 7 citation deficiencies, all applied |
| 4 | 76 | FAIL on bar, no CRITICAL, **10 deficiencies, all applied** (`2c4e140`) |
| 5 | 86 | FAIL on bar, no CRITICAL, **1 UNCOVERED clause + 8 deficiencies, NONE applied** |

Iteration 5's UNCOVERED finding is substantive, not hygiene: the ticket body asks for #88's
`half_viewport * scroll_scale` stagger to be **re-measured under the corrected canvas**, and
4.2b discharges it by arguing the formula's input is camera-defined and therefore moot once
the camera goes. Iteration 3 raised the same objection and the plan declined it; **iteration 5
sides with iteration 3**. The fix is a plan item plus a section 7 exit condition — the
corrected canvas only exists after the camera-deletion blocks land, so the measurement is a
post-implementation gate. Iteration 3's finding that block 1/6 (`_size_layer_repeats`
deletion) rests on an argued reading also stands unfixed.

**All eight iteration-5 findings and their proposed resolutions are tabulated in the gate
history**, unapplied. Do not start scaffolding until @me decides whether iteration 6 runs.
This is iteration 5 of a `MAX_ITERATIONS` of 3 — the loop's own STOP condition, and the bar
has never been met on a valid pass.

### Issue-state corrections made 2026-10-04 (both verified, both reversible)

> Superseded 2026-10-05: #89 has since CLOSED and its fix landed. Retained as the record of
> the 2026-10-04 corrections. #85 remains closed.

- **#89 REOPENED.** It was closed as COMPLETED at 2026-10-03 18:18 UTC with no fix commit and
  no closing comment; `trunk` at `d3edd5f` has no commit touching `simulator.gd` or
  `backdrop_preview.tscn` since the framing work, `BackdropStrip/Camera2D` is still in the
  scene, and both scrollbars still contribute 0 px. A reopen comment records where the design
  actually lives and that the gate is not yet passed.
- **#85 CLOSED** with a comment citing `ecbf893` and wiki `TechnicalDesign/TDD_Saved-World`.

### Repo state corrections

- **`chore/godot-4-7-upgrade` DELETED 2026-10-04.** It was a stale duplicate, never pending
  work. Its single commit `30f55f9` is patch-id-identical to `f9d9047`, which is already in
  `trunk`: both carry `git patch-id --stable = bf8f842ed4ecfc07ec8eb10c87020af5b1aa172e`,
  both bodies are the same text, and both diff exactly two files (`.gitignore` +7,
  `editor/project.godot` +10/-1). It was never pushed (`git ls-remote --heads origin` shows
  only `trunk`), so there is no remote ref to delete. `git branch -d` refused because the
  branch is not an *ancestor* of `trunk` — it is a rebase-era duplicate, not an unmerged
  ancestor — so `-D` was correct and `-d`'s merge check was the wrong test for this case.
  @me verified the installed runtime independently (4.7.2.stable.mono).
- **`feat/parallax-restack`, the branch the plan names, does not exist.** Create it from
  `trunk` when scaffolding starts; the plan's header line claiming it exists is stale.
- **`editor/tests/measure_no_camera.gd` is already gone** (the plan's working-tree disposition
  already satisfied) and `git status` is clean, so the tree needs no cleanup before branching.

### The fix, in one line

Delete the `Camera2D`, and write `layer.scroll_offset = scrub * layer.scroll_scale` per
layer, per axis. From `Parallax2D::_update_scroll()`: the engine scales `screen_offset`,
never `scroll_offset`, so a per-layer write is what actually drives each band. No camera
means no canvas transform, so the editor UI is never displaced.

### The plan of record

`docs/PLAN-2026-10-03-parallax-scroll-offset.md`, with per-iteration changelogs in
`docs/PLAN-2026-10-03-parallax-scroll-offset-gate-history.md`. **Read section 10 first** —
it carries the gate verdict and what each iteration changed.

### The correction that matters — read before repeating it

Iteration 4 found a **false engine claim** in the plan's central section, and it was
verified against source before being adopted (iteration 3's report had a wrong line
citation, so the gate's word is evidence, not instruction):

```cpp
// servers/rendering/renderer_rd/renderer_canvas_render_rd.cpp:2239-2252
Point2 start_pos = ci->repeat_size * -(ci->repeat_times / 2);
for (int rx = 0; rx <= repeat_times_x; rx++) {   // INCLUSIVE: N+1 copies, not N
```

`repeat_times = 1` draws **two** centred copies, spanning `[px, px+3840)` with
`px = position.x ∈ (-1920, 0)`, so a 1920-wide viewport is *always* covered. Separately
`parallax_2d.cpp` clamps `repeat_times = MAX(p_repeat_times, 1)`, so 0 ≡ 1 ≡ 5.

**Two of the plan's three "falsified" metrics had been declared impossible for the wrong
reason, and both readings were actually correct.** 0.00% bare at scrub 6400 is exactly what
rt=1 must produce; rt=0 matching a 5-copy reference is possible and was measured. The lesson
now recorded in plan 5.1: *a control reporting a startling value is a claim about the
engine, not a verdict on the metric.* No design changed — the clamp, the framing constant
and the per-layer writes are unaffected.

### Measured and confirmed (plan section 5.2)

M-0, M-a, M-b/M-c, M-e, M-f, M-g, M-h, M-k, plus M-e's four-way counter-verification.
Key outputs: `scroll_v.max_value` stays **240.0** (sky top is 3637.5 scrub units away, 15×
the clamp; its justification comment is false and gets rewritten); `scroll_x.max_value` stays
**6400.0**; `Backdrops.position.y = -291` measured at 0.00% bare with all six bands
contained.

`--resolution 1920x1080` yields a **1920x1029** window (the WM grants 1029). Every number
the plan labels "1920x1080" was measured there, `seen_region` 1920x998 at canvas y=31.

### Still open in the plan

- **M-b1** — `Backdrops.position.y = -291` is **not a derived optimum**. The bare-% column
  reproduces exactly from `bare(offset) = max(0, 31+offset) + max(0, -322-offset)`, which also
  predicts 0.00% at `-320, -240, -180, -120, -60` — a six-way tie. Recorded as an open gap,
  not a settled number.
- **M-e's five-point scrub sweep at 1280x800** — the upward bound is resolution-independent
  and confirmed; this is the one open end.
- **M-d** superseded by M-k.

### The bug (#89) — read this before touching the backdrop

`BackdropStrip`'s `Camera2D` sits in the **same canvas layer as the entire editor UI**. A
`Camera2D` transforms its whole canvas layer, so it was displacing every `Control` —
tab bar, all editors, both scrollbars — by half the viewport.

Measured with a real window at 1920x1080 (frame 1920x1029):

```
canvas_transform       = translate(960, 483.5)
ui pixels bbox         = (960, 483) size (960, 546)   <- UI in one quadrant
bar_x renders at y     = 1496.5   (frame ends at 1029 -> off-screen)
bar_y renders at x     = 2864     (frame ends at 1920 -> off-screen)
bar_x contributed px   = 0
bar_y contributed px   = 0
```

**Both scrollbars have been invisible this whole time.** They are anchored correctly
(`bar_x` at `(0,1013) 1904x16`, `bar_y` at `(1904,31) 16x982`) and render off-screen.

### Two candidates measured on rendered frames — both fail

| candidate                        | ui_bbox                      | bar_x px | bar_y px | art scrolls |
|----------------------------------|------------------------------|----------|----------|-------------|
| baseline (camera on, inline)     | (960, 483) size (960, 546)   | 0        | 0        | yes         |
| B: `camera.enabled = false`      | (0, 0) size (1920, 1029)     | 30442    | 15690    | **NO**      |
| A: strip onto its own CanvasLayer | (960, 514) size (960, 515) | 0        | 0        | yes         |

- **B is a diagnostic, not a fix.** `simulator.gd:32-39` drives both bars by writing
  `camera.position`, so a disabled camera leaves the strip immobile.
- **A does not work at all.** Moving the strip — and the camera — onto its own
  `CanvasLayer` should confine the camera to the art's canvas. It does not: the UI stays
  displaced and both bars still draw 0 px.

### The measurement trap that hid this — do not repeat it

`get_global_rect()` reports **anchor math only** and excludes the camera's canvas
transform. The bars therefore read as correctly placed at every window size while being
drawn off-screen entirely. `z_index` cannot detect it either: a control created in code at
the same parent and at `z_index = 20` draws 2968 px, while the declared bar draws 0 px at
**every** z from 0 to 20.

Two checks are required for anything expected to be visible, and neither is a transform
read:

1. intersect the **canvas-space** rect with `get_visible_rect()`;
2. hide the node, diff the rendered frame, and count changed pixels. Include a positive
   control that is known to draw, so a zero result is distinguishable from a broken differ.

**Corollary: every framing number measured before this finding is suspect**, because it was
calibrated through a displaced canvas. Specifically void —

- `Backdrops.position = Vector2(-960, -780)` in `backdrop_preview.tscn`
- `_frame_backdrop()` runtime horizontal alignment in `simulator.gd`
- the 240px vertical travel clamp and its justification
- the conclusion that vertical travel needs a **generator change** for taller art. That was
  measured through the wrong canvas and must be re-tested before acting on it.
- #88's `half_viewport * scroll_scale` stagger formula — a camera-induced half-viewport
  offset and a parallax-induced one present identically, so it must be re-measured.

### Next move

**Revised 2026-10-04.** Items 1 and 2 are obsolete and are marked rather than deleted, so a
reader can see why they are not being done.

1. ~~**Push, then ff-only merge** `feat/parallax-restack` to `trunk`.~~ **Obsolete** — that
   branch does not exist and no code was ever written on it. Create it from `trunk` at step 5.
   The push/merge discipline still applies when it lands: ask before pushing, `--ff-only`,
   never `--force`, offer branch deletion.
2. ~~**Delete `editor/tests/measure_no_camera.gd`**~~ **Already done** — the file is gone and
   `git status` is clean. `editor/addons/` is still 516 MB vendored: never `git add -A`.
3. **@me decides whether gate iteration 6 runs.** Nothing below starts until it does. The
   eight findings and their proposed resolutions are tabulated in the gate history; a sixth
   round applies them and re-scores with the same evaluator.
4. **Rewrite `editor/tests/test_parallax_backdrop.gd`** (assistant's job, not a block): new
   `scroll_offset` contract; an ordering assertion replacing the negative-drift check (M11 —
   "Foreground moves backwards" was true only of the camera model); a **sign** assertion,
   because the x scrub inverts on screen under B6; keep `240.0` and `6400.0` assertions,
   rewrite the false `:106-107` justification; assert the wrap-free base of 640 (M10).
   **Run RED against the unfixed tree first.**
5. **Write `editor/tests/probe_render_visibility.gd`** — corrected absolute-bare-pixel metric,
   positive control must hide **Sky** specifically (only the bottom-most opaque layer can
   raise a background count). Exit 0 needs **four** conditions, including the five-point
   scrub sweep that 4.5 computed but section 7 previously did not gate. Assert received
   window size against the **WM** screen size, not the requested `--resolution`.
6. **Scaffold the blocks** with the measured numbers, via the `coding-assistant` skill.
   Order is script-then-scene (`simulator.gd:25` is a hard `get_node`; deleting the node
   first leaves a window where `_ready()` raises "Node not found"). Note the block map may
   lose 1/6 to a follow-up ticket if iteration 6 runs.
7. **Measure #88's stagger formula under the corrected canvas** — now a plan item with its
   own gate condition, not a deferred paragraph, after the camera-deletion blocks land.
8. **#87** (editor backdrop repoint + persisted-map migration + dead `_BACKDROP_PATHS`)
   before Phase 2's `git rm` of the 13 old tracked PNGs.
9. Phase 6: final gates, close #68, `--ff-only` merge, offer branch deletion.

### Parallel thread — #94 minimap (GDD merged, zoom ladder settled)

Wiki `master` at `ace6738`. `GDD_World-Layout.md` carries a *Minimap (Side-Scrolling Defense
Zones)* section plus `GameDesign/assets/minimap_schematic.{svg,png}` (a labelled **MOCKUP** —
no minimap exists to screenshot; drawn in screen units so it survives any scale).

Settled: scrolls rather than scales; the map's **full vertical extent** across full map width;
entities as dots (blue ally, red enemy, grey neutral); **M** key and a Minimap button; overlay
with the game live; **exactly three zoom levels, 2x apart**.

**The zoom ladder, settled 2026-10-03.** One screen occupies `1920/d` x `1056/d` panel px, so
`screens_shown = (panel_w * d) / 1920`. Recommended panel **480 x 264** gives a **1 / 2 / 4**
screens-wide ladder at `d = 4 / 8 / 16`. Two consequences, both verified by computation:

- The **scroll-unreachable problem is solved**. A 3-screen map is 720 panel px against a
  480 px window at normal zoom, so the window really scrolls. That was the question the GDD
  carried as open and could not answer; it is deleted from the wiki page, not left dangling.
- **Scaling is uniform on both axes**, and the panel *height* is what enforces it:
  `panel_h` must be a multiple of `1056 / d_zoom_in` = 264. Levels then divide evenly at
  264/132/66 px. Panel height is the one dimension that is not free.

Extent changed from "exactly one `screen_y` row" to the map's full height, because
`GDD_Combat-Mechanics` Blueprint 2 is `4 Screens Wide: 1 High ---> 2 High ---> 1 High`. One
`min(map, window)` rule per axis covers both. The EvE/ZvZ rule from `TDD_World-Editor` is
carried into the section: minimap contents must come from the owning client's own viewport
coverage, never server-global truth.

**Three items still open in the wiki, deliberately:**

- **Panel size** — 480 x 264 is a *recommendation*, not a decision. Widening it changes only
  the window width, never the ladder's ratio or the uniform-scale rule.
- **Zoom input binding** — wheel risks the live game behind the overlay consuming it; keys
  risk colliding with combat hotkeys. Neither chosen.
- **The Alarm tower conflict.** `GDD_Gameplay.Towers` gives Alarm its *entire* effect:
  "Reveals enemy position on minimap". Always-visible red enemy dots nullify it. A
  resolution is proposed and **marked unapproved**: reveal extends from the view rectangle to
  the whole window, so Alarm keeps a real effect. Changing that promise is a change to
  `GDD_Gameplay.Towers`, not to `GDD_World-Layout`.

Also: the viewport is 1080 px tall and a screen is 1056 px, so the view **straddles the next
`screen_y` row by 24 px** — 2.3%, invisible at minimap scale. The straddled row is defined by
the top edge of the viewport, stated as a rule rather than drawn as a cue.

Name collision to resolve before implementation: the editor already has a `class_name
ScreenMinimap` (`editor/scripts/screen_minimap.gd`, used by `map_editor.gd`,
`placement_editor.gd`, `test_screen_store.gd`) — renaming it is a breaking change across four
files.

### Landed and pushed to `trunk` (kept, but see the caveat above)

- Both bars at `z_index = 20`. Still correct: PlacementEditor's grid sits at `z_index = 5`,
  so a bar at the default 0 would be painted over.
- `_size_layer_repeats()` in `simulator.gd` — `repeat_times` derived as
  `ceil(scrub / repeat_size.x) + 1`.

  > **CORRECTED 2026-10-03.** The original justification here was wrong twice over. The
  > measurement was taken through the displaced canvas, so the numbers never transferred;
  > and the "100% bare at camera x=1600" figure came from the **voided M-d metric** (plan
  > 5.1) — it counted *changed* pixels, so a gap and art scored identically and the
  > metric could not report a gap. M-k measured it properly: `repeat_times` 1/2/3/5 render
  > **pixel-identical** at scrub 0, 1600 and 6400 against a positive control diverging
  > ~1.9M px. The renderer draws N+1 centred copies (inclusive bound), so two copies already
  > cover any viewport up to 3840 px wide. The function is unobservable and is slated for
  > deletion in block 1/6. **Do not cite the bare-percentages in its doc block
  > (`simulator.gd:53`, `:63`) — they are void.**

### Other open state

- **Phase 2** (retire 13 old tracked PNGs, ~15 MB) **blocked on #87**.
- **`chore/godot-4-7-upgrade`** — **DELETED 2026-10-04.** It carried the gitignore keeping
  `editor/addons/` out, and that gitignore is already in `trunk` as `f9d9047`
  (`git check-ignore -v editor/addons/` resolves to `.gitignore:46`). Nothing pending here.
- **Wiki freeze**: lifted for `TDD_Parallax-Depth.md` only, then for the minimap section of
  `GDD_World-Layout.md` + its `Home.md` index row, then for the minimap zoom ladder in the
  same section + `minimap_schematic.{svg,png}`. **Nothing else in the wiki may be edited.**
  The first two are merged to `master` (`7e461f8`) and their branches deleted. The zoom
  ladder is merged to `master` as `ace6738`, fast-forward, no merge commit. Both feature
  branches (`docs/minimap-zoom-ladder`, `docs/minimap-zoom-checkpoint`) were pushed, merged
  and then deleted local and remote with @me's approval. The wiki is on `master` with only
  `master` on the remote.
- Wiki's six recorded discrepancies untouched (FROZEN pending @me's agreement).
- Issues filed today: **#90** celestial body, **#91** signed depth (real sub-issue of #90),
  **#92** four stale `Camera2D` comments + one superseded design plan, **#93**
  `TDD_Parallax-Background` names the camera as the parallax system's only input,
  **#94** the minimap feature request.

### Out of scope, deliberately

#88 stagger constants, taller art from `gen-backdrop`, bar anchors and `z_index`, #87, PNG
retirement, the celestial body (#90), the #91 depth migration, #92's stale comments, #93's
design-page correction. Each declined for a stated reason in the plan.

### Gates

`cargo test -p sstd-core` **110 tests**, the workspace Rust gate. `cargo test -p gen-backdrop`
29/29. `test_parallax_backdrop`, `test_image_to_map`, `test_terrain_brush`,
`test_override_merge` all green. **`test_screen_store` is green again on 4.7.2 as of
`ecbf893` (#85 fixed) and is a gate** — corrected 2026-10-04, this line still said it exits 1
and was two commits stale.

**Gate on the exit code, never on a printed `failures=0` line.**

Issues open: #68, #82, #84, #86, #87, #88, **#89** (reopened 2026-10-04), #90, #91, #92,
#93, #94. **Closed since the last session: #85** (fixed by `ecbf893`, verified green on 4.7.2).

---
---

## SUPERSEDED (2026-09-27) — agent-config consolidation + wiki link-rot sweep

_Last updated: 2026-09-27_

**Both repos clean, merged, and pushed, 0 unpushed.** #83 is closed.

| repo | default | head | this pass |
|---|---|---|---|
| `sidescroll-towerdefense` | `trunk` | `72857e2` | consolidated AGENTS + `opencode.json` wiring (#83) |
| `sidescroll-towerdefense.wiki` | `master` | `a61a7ed` | indexed 19 unlinked wiki pages (#83) |

Worked on `fix/agents-consolidate` / `docs/home-md-index`, both merged back with
`git merge --ff-only` and pushed (fast-forwards `47d0552..72857e2` and `8059ddd..a61a7ed`).
**No PR** — @me's standing decision, now recorded in
`~/.cache/opencode/branch-then-merge-repos.md` for both repos. The wiki remote has no usable
API repo at all, so a PR is impossible there regardless.

### Why the `opencode.json` files exist

`.opencode/AGENTS.md` is **not** auto-discovered. The only supported mechanism is the
`instructions` array in `opencode.json`, and neither repo had one — so the project's
knowledge file was never loaded by a session started inside the repo. Config lookup stops
at the nearest git directory, so two files are needed: the repo-root one for repo launches
and `SSTD/opencode.json` for workspace launches. Both verified with `opencode debug config`;
each resolves to a real file and both concatenate with the global
`~/.config/opencode/opencode.jsonc` entries rather than replacing them. **A new session is
required to observe the effect** — resolution is not loading.

### Do NOT redo these

- **The LUCK section of the project AGENTS is wrong** (filed as #84, deliberately left
  undecided). It claims deterministic, no-dice-roll behaviour with `luck.bonus_cap`,
  `crit_interval`, and `rarity_floor` — **none of those three exist in the code**, which
  has `crit_chance`, `WeightedTable::pick`, and `luck_weighted_rarity`. The wiki matches the
  code, not the AGENTS file. Same file also contradicts itself: the adjacent "Replay &
  seeded rolls" subsection correctly describes `seeded_roll` + `RollLog`. Do not "fix" the
  docs here without the owner deciding whether the code or the 2026-08-07 decision is wrong.
- **Coverage counts must be verified against `HEAD`, not trusted from a loop's output.** An
  earlier pass reported 11 unlinked wiki pages; the true figure was 19 (8
  `GDD_Gameplay.*` sub-pages were missed). Check with `git show HEAD:Home.md` before quoting
  a number.
- **Validate a mutation before believing its result.** Two void experiments in one pass: a
  tab injected into a space-indented file, and a "missing page" probe that was also linked
  in `Home.md`. Both produced a clean result that looked like a real finding.
- **`opencode debug config` prints the Anthropic API key in plaintext** (it is in the global
  `opencode.jsonc` under `provider.anthropic.options.apiKey`). Never paste that output into
  an issue, PR, commit message, or log.
- **`godot4` IS on `PATH`** via `~/bin` (`command -v godot4` -> `/home/hidekiai/bin/godot4`).
  The earlier claim that it was "not on the default PATH" was wrong and is corrected in
  `SSTD/AGENTS.md`. The `$HOME/bin/` prefix is kept for portability only.
- **`CONTRIBUTING.md` and `docs/guidelines.md` are in the global instructions list but
  exist in neither root.** They match nothing and are silently skipped. Pre-existing, from
  the global config, not introduced here.

### Shipped last cycle (2026-09-25/26)

| commit | what |
|---|---|
| `2c08989` | hermetic, falsifiable `test_override_merge` guard (#79) |
| `6c0732b` | corrected the withdrawn data-loss claim in this file (#79) |
| `c955cec` | inverted exit ternary in two test runners (#80) |
| `7ff696a` | override-delta write rule + guard requirements, `TDD_World-Archive` §3b / §8a |

Plans: `PLAN-2026-09-25-override-merge-guard-hermetic.md`, `PLAN-2026-09-25-test-exit-codes.md`.

### Do NOT redo these (last cycle)

- **A missing `terrain_overrides.json` is not data loss.** It is correct delta semantics
  and the normal representation of a world at framework defaults. #79's original claim
  was disproved; the measurements behind it were right, the inference was wrong.
- **The pre-#76 world backup in `/tmp` is skipped by user decision** (2026-09-25). Do not
  restore, chase, or re-raise it. `/tmp` does not survive a reboot; that is accepted.
- **Do not bundle the `entity_overrides` dead-channel removal** into another fix — it
  removes a `save_world` positional parameter, a breaking contract change.
- **Indentation is mixed**: `editor/scripts/entity_editor.gd` uses spaces,
  `editor/scripts/terrain_editor.gd` uses tabs. Injecting a line with the wrong style is
  a parse error, not a behavioural mutation; this invalidated a whole experiment once.

### Open threads

| # | item | why still open |
|---|---|---|
| #62 | save/load dialog default filter | code-fixed via filter-**ordering** (`current_filter` does not exist in Godot 4.4); needs a real-display click-through. A display **is** available here (`DISPLAY=:0.0`) |
| #81 | `test_screen_store` silent-truncation exposure | 86 `await` sites, no completion tracking. **Unproven** — the issue carries the experiment that would settle it |
| #68 | parallax runtime + legacy single-screen `.json` round-trip | parked; probe GREEN, runtime rendering unverified |
| #82 | `entity_overrides` dead channel | proven: written to the archive, read into the load result, **zero consumers**; only producer is a stub returning `{}`. Deleting it removes the 5th positional `save_world` param — breaking, own review |
| #84 | LUCK AGENTS section contradicts code, wiki, and itself | filed from #83; needs an owner decision, deliberately not resolved here |

Also still open from earlier cycles: #44, #65, #69.

### Next move

1. **#84 is the only thing this pass left open.** Once decided, correct either the AGENTS
   LUCK section or the luck code/wiki, and reconcile the "Replay & seeded rolls" subsection
   either way. Note that `seeded_roll`/`RollLog` are also load-bearing for the replay/CI
   checksum invariant, so a "make it deterministic" fix cannot simply delete them.
2. Start a **new session** to confirm the `opencode.json` wiring actually loads. The config
   resolves, but resolution is not loading — only a fresh session proves the AGENTS file is
   in context.

### Gate

`cargo test -p sstd-core` (**110** passed) plus all four GDScript suites, which must
each exit 0 — the exit code is authoritative, not the printed `failures=0` line:

```bash
~/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd    # 187 ok
~/bin/godot4 --headless --path editor --script res://tests/test_image_to_map.gd    # 9 ok
~/bin/godot4 --headless --path editor --script res://tests/test_terrain_brush.gd  # 14 ok
~/bin/godot4 --headless --path editor --script res://tests/test_override_merge.gd  # 26 ok
```

---

## SHIPPED (2026-09-25) — stale-issue sweep: #38 #49 #51 #63 closed; #76 re-cited; #79 filed

> Bookkeeping pass, no code changed. Four issues were OPEN but had shipped weeks
> ago; each was verified against the code before closing. Also filed a real bug
> found while gating (see #79 below).

**Closed, with evidence comments.** Each closing comment names the wiki page as
the design of record, the shipping commits, the acceptance criteria point by
point, and the test that guards it. Commit hashes live on the issues, never on
the wiki.

| Issue | Feature | Wiki page (design of record) | Guard |
|---|---|---|---|
| #38 | image-stamp importer | `TDD_Tile-Reuse-and-Stamp-Groups` §2 | `test_screen_file_json_roundtrip` (carries a `stamp_maps` entry) |
| #49 | visual tile palette | `TDD_Tile-Reuse-and-Stamp-Groups` §10 | `_test_tile_palette` |
| #51 | Prune Duplicates | `TDD_Tile-Deduplication` + §11 | 6 prune tests (`_test_prune_duplicates` … `_test_bridge_exact_scan`) |
| #63 | Collision Map button | `TDD_Collision-Map-Tooling` | **none — §9 is manual-only** (see caveat) |

**#63 verification caveat, recorded deliberately:** there is no headless
regression test for the Collision Map button, so the suite's `failures=0` does
*not* cover it. The only evidence of a live run is the user report at
`docs/SESSION-CHECKPOINT.md` line ~472 ("4 tile flicker / quadrant toggle"),
which confirms direct quadrant-paint ran on a real display. Treat
`TDD_Collision-Map-Tooling` §9 as the spec if it regresses.

**#44 stays open.** #49 shipped the palette that #44's region-select grouping
needs as its picker, but the region-select tool itself is still unbuilt
(`TDD_Tile-Reuse-and-Stamp-Groups` §4, "the gap").

**Wiki commit (local, wiki repo `master`):** `TDD_Tile-Reuse-and-Stamp-Groups`
header now cites #38/#46/#49/#51, each pointing at the section documenting it,
plus the guard test per feature — previously the header cited only #46, so the
importer's design linked to no issue and the traceability record was broken at
the join. `TDD_Parallax-Background` §4.2 gained the missing #76 art-to-tiles
import step (`ImgMapBtn` -> `_on_img_map_import` -> `_import_image_to_tiles` ->
`_nearest_terrain_key`), including the point that `air` is never returned so sky
becomes *no cell* rather than an air tile — that is what feeds the §2 hole model.
`TODO.md` TS41 -> `done`, TS62/63/64 record the closes, TS65 the sweep itself.
No commit hashes were added to the wiki (the two pre-existing ones in TS47/TS49
violate the rule and were left alone as out of scope).

**#76 doc hygiene: DONE.** Its closing comment cited this checkpoint instead of a
wiki page, breaking the Documentation Reference Architecture rule. There was no
wiki page to cite, so the §4.2 addition above was written first, then the comment
was re-cited in place with `gh issue comment 76 --edit-last` (verified: 1 comment
on the issue, replaced not appended). This resolves the "Outstanding doc hygiene
(not yet done)" note in the 2026-09-24 block below.

**#62 deliberately LEFT OPEN.** Code-fixed via filter-ORDERING, not a property:
`FileDialog.current_filter` does not exist in Godot 4.4, so instead the filter
list is ordered by world format — package-backed world puts `*.zip` first
(`map_editor.gd:1098-1104` save, `:1167-1173` load), legacy manifest puts
`*.json` first. Unverified because it needs a real display. A display *is*
available on this host (`DISPLAY=:0.0`, `/tmp/.X11-unix/X0`, seat0 session), so
this is closeable on request: open a package-backed world, confirm the filter
dropdown defaults to "SSTD World Package", then a legacy world and confirm
"Screen JSON (legacy)".

**#79 SHIPPED and CLOSED — the "data loss" reading was WRONG. Withdrawn.** The section
this replaces claimed "real data loss, cause not yet pinned", and flagged an
unproven `map_editor` / `placement_editor` asymmetry. Both claims were investigated
and both are false. **The measurements below are correct; the interpretation was not.**

What the archive diff actually shows (still valid, and it is what pinned the cause):

| | pre-#76 backup (2026-09-14) | current `editor/world.zip` (2026-09-19 13:22) |
|---|---|---|
| `terrain_overrides.json` | `{"air":{"sub_tile_mask":15},"dirt":{"sub_tile_mask":0}}` | **absent** |
| archive size | 1,259,491 B | 3,873 B |
| `screens/1.json` | 230,849 B | 6,764 B |
| tile bank | ~90 `stamp_N` tiles | `dirt.png`, `grass.png` |

No data was lost. Two things had been conflated:

1. **Boot never reads `res://world.json`.** `main.gd:4` uses
   `user://data/world.json`, and on this host that file is an empty legacy
   manifest, `{"screens": {}, "version": "0.2.0"}` — zero screens, no `.world`
   key. So `on_world_loaded` receives no overrides, `apply_terrain_overrides` is
   never called, the Terrain Editor sits at framework defaults, and
   `collect_terrain_overrides()` correctly returns `{}` — a zero delta against a
   framework it exactly matches. `world_archive.gd:117` writing only on a
   non-empty delta is **correct** delta semantics, symmetric with the read at
   `:181-184`. There is no defect there.
2. **The file was absent because the world was replaced, not corrupted.** A
   1.26 MB / ~90-tile world and a 3.8 KB / 2-tile world are different worlds, not
   one damaged world. The overrides belonged to the world that was swapped out.
   What caused the 2026-09-19 reset is still not established, but it is
   environment rot, not a code defect, and it is not this issue.

The `map_editor` / `placement_editor` asymmetry was also wrong:
`_collect_entity_overrides()` at `placement_editor.gd:598` is a **stub that returns
`{}`**, so both call sites pass `{}`. What is real is that `entity_overrides` is a
dead channel end to end — a separate follow-up, not folded into #79.

The actual defect was the test, and it was worse than "not reproducible":

- **Non-hermetic.** The guard read its input from gitignored, untracked
  `editor/world.zip`, so it asserted machine state, not tracked code behaviour.
- **Two vacuous assertions.** It checked `arrow_tower` `attack_power=100` /
  `max_hp=500` and `mine` `class=trap` — which are the framework prototype's own
  values in `default_package/entity_defs.json`. Those assertions passed whether
  or not the merge ran. That is why the entity half stayed green while the terrain
  half went red: the entity half was never testing anything.
- **Silent truncation.** A guard is a coroutine; a runtime script error aborts it
  but lets `_run` continue, so the suite printed `failures=0` and exited 0 with
  the merge never called. Proven by injecting a tab into space-indented
  `entity_editor.gd`.
- **Coverage hole.** The archive codec for overrides had no test with a non-empty
  payload, so a save/load asymmetry or a rename of `TERRAIN_OVERRIDES_PATH` would
  go unnoticed.

Fixed in `2c08989`; guard now has three falsifiable parts (terrain merge, entity
merge, archive codec round-trip) with vacuity tripwires and a completion check.
Verified by mutation, all files restored: no-op `apply_terrain_overrides` → 2
failures; no-op `apply_world_entity_defs` → 4 failures; renamed
`TERRAIN_OVERRIDES_PATH` → 1 failure; `world.zip` moved aside → 0; tab-parse abort
→ truncation check fires. Plan and full evidence in
[PLAN-2026-09-25-override-merge-guard-hermetic](PLAN-2026-09-25-override-merge-guard-hermetic.md).

**#80 filed** while verifying: `test_image_to_map.gd:63` and
`test_terrain_brush.gd:70` both have an inverted ternary,
`quit(1 if failures == 0 else 2)`, so they exit non-zero when they **pass**. They
survived because a failing test also exits non-zero, which satisfies an
exit-code check for the wrong reason. Not touched by #79.

`editor/world.zip` and the backup were never modified. The backup is still at
`/tmp/user/1000/opencode/sstd-world-pre-image-2026-09-19.zip`; `/tmp` does not
survive a reboot, so copy it somewhere durable if the pre-#76 world still matters.

**Legacy single-screen `.json` layer round-trip stays parked in #68.** It is
in-flight #68 work, not a repo regression, so no bug was filed. `_on_import_file`
-> `_load_screen_file` -> `_apply_screen` goes through `_serialize()`, which has
no layer payload; only the `.zip` path round-trips (`_serializable_layers` ->
`WorldArchive.save_world` -> `_load_world_package`). Recorded as a known gap in
`TDD_Parallax-Background` §4.2.

**Active step (next move): pick one, in order.**
1. #79 — pin the save-time cause, then fix. Requires reading the
   `map_editor.gd:1124` / `placement_editor.gd:574` call sites against the
   `on_world_loaded` path, and deciding the fixture question for the gitignored
   `world.zip`.
2. #62 — needs only a display click-through; see above.
3. #68 — the runtime `biome_backdrop_layers` DB binding is the one real open
   feature on the editor side. `grep biome_backdrop_layers` hits only docs, zero
   code or DB, so nothing populates those rows yet. `TDD_Parallax-Background` §2
   is the runtime model, §4.2 the editor tooling already shipped.

## SHIPPED (2026-09-24) — #68 parallax scroll probe GREEN; #74 and #76 closed

> Catch-up block. The previous newest entry was 2026-09-14, so the 2026-09-19 to
> 2026-09-24 work was unrecorded; restored here from `git log` + issue history.
>
> **#74 slice-layers (CLOSED 2026-09-13; commits d27f845, e94895a).**
> `tools/slice-layers` derives the L0/L1/L2 parallax layers from a rendered preview MP4
> by flow-not-ML depth recovery. Source MP4s stay local-only (`.gitignore`, user
> directive 2026-09-13); the derived layers are the artifact of record.
>
> **#76 image -> terrain map import (CLOSED 2026-09-19/20; commits 03e3272, 15ea539,
> d225363).** `map_editor.gd` imports any PNG and converts colour to terrain by
> quantisation. The resulting near band (`layer_2.png`) is rendered as a collidable
> backdrop strip in BOTH the Simulator stack and the MapEditor grid; the x5 collision
> strips are committed under `editor/assets/backdrop_layers/`. Outstanding doc hygiene
> (not yet done): the #76 closing comment cites this checkpoint instead of a wiki page.
>
> **#68 parallax scroll (still OPEN; probe GREEN 2026-09-24, commits 6f1f9da, 3dff990).**
> The editor scrollbar now drives a `Camera2D` at 1:1 and the near band sits at a static
> 0.9x factor. Probe: `editor/tests/probe_parallax_scroll.gd`. Wiki section 4.2 (Map
> Editor layer management) was committed 2026-09-25. Still to land for #68: the runtime
> `biome_backdrop_layers` DB binding (v2) and the wiki acceptance-criteria tick-off.
>
> **Active step (next move): stale-issue sweep.** OPEN but already shipped — #38
> (image-stamp importer), #49 (visual tile palette), #51 (Prune Duplicates button), #63
> (Collision Map button). #62 (load/save dialog default filter) is code-fixed via
> filter-ORDERING, because `FileDialog.current_filter` does not exist in Godot 4.4, and
> needs a real-display GUI re-verify before it can be closed.
>
> **Housekeeping.** Godot 4.4 `.uid` sidecars are now tracked for all scripts
> (20 were already tracked, 16 more added this pass). The pre-#76 world package
> `editor/world.zip.bak-2026-09-19` (1.26 MB) is parked at
> `/tmp/user/1000/opencode/sstd-world-pre-image-2026-09-19.zip` for the older-data-load
> and serde-versioning check — the `*.bak` rule in `.gitignore` does not match
> `*.bak-<date>`, which is why it kept appearing as untracked.

## SHIPPED (2026-09-06) — #7 rarity FK seed + doc integrity (impl from PLAN below)

> Implemented per the PLAN block below and landed this pass.
> - `crates/sstd-core/src/config.rs`: population `005` creates + seeds
>   `gacha_rarities` (R=1/SR=2/SSR=3/UR=4, display names per GDD_Progression-Gacha)
>   and `guild_revive_cooldowns(rarity_id INTEGER PRIMARY KEY REFERENCES
>   gacha_rarities(id), cooldown_scenarios)` (R=5/SR=3/SSR=1/UR=0); new public
>   structs `GachaRarity` + `GuildReviveCooldown`, accessors `gacha_rarities()` /
>   `guild_revive_cooldowns()`; schema_version now `0.0.5`. Re-exported in lib.rs.
> - Tests added: `test_gacha_rarities_seeded`, `test_guild_revive_cooldowns_seeded`,
>   `test_rarity_fk_integrity` (PRAGMA foreign_key_check -> 0 violations),
>   `test_gacha_seed_idempotent` (re-run keeps 4/4). Updated
>   `test_schema_version_after_population`/`test_population_applied`. cargo: 110 core,
>   full workspace 136 pass.
> - Wiki: TDD_Training-Center Revive Mechanics now FK lookup via
>   `guild_revive_cooldowns`; the 4 loose `training_guildReviveCooldown{R,SR,SSR,UR}`
>   config keys deleted from the GM-config table; TDD_GM-Config Gacha Rarity Enum gains
>   consumer note; TDD_Enum-Tables gains `gacha_rarities` section + cross-ref matrix row.
> - Bridge: no change (no GDScript consumer of gacha tables yet — deferred).

## PLAN (pre-implementation, 2026-09-06) — #7 rarity FK seed + doc integrity

> Grounding (read before planning): `crates/sstd-core/src/config.rs` (Population
> mechanism `Population{id,description,func:`}, `POPULATIONS` array, `pop_id_to_version`
> id->version, `run_populations` gate, `INSERT OR IGNORE` seeding, `all_config`/loaders/
> tests at lines 555-730), `crates/sstd-editor-bridge/src/lib.rs` init_config_db,
> wiki `TDD_GM-Config.md` ("Gacha Rarity Enum" `gacha_rarities` table, line 75),
> `TDD_Training-Center.md` (Revive Mechanics + GM config rows 98-111),
> `TDD_Enum-Tables.md` (enum inventory + cross-reference matrix, lines 351-364).
>
> **Defect (issue #7):** rarity is referenced as loose TEXT config keys
> (`training_guildReviveCooldown{R,SR,SSR,UR}`) and an inline comment, NOT FK-encoded;
> the `gacha_rarities` enum table is absent from TDD_Enum-Tables inventory, and has no
> SQLite presence. Design decisions D6/D7 require INTEGER FK references for every enum.
>
> **Design (module ownership: `crates/sstd-core/src/config.rs` owns schema + accessors;**
> **bridge exposure deferred — no GDScript consumer exists yet):**
> 1. Add `Population { id: "005", description: "Gacha rarity enum + guild revive cooldowns", func: populate_005 }`.
>    `pop_id_to_version("005")=5` -> `schema_version` becomes `0.0.5`. Idempotent on
>    existing DBs (CREATE TABLE IF NOT EXISTS + INSERT OR IGNORE), matching populations 001-004.
> 2. `populate_005` (execute_batch):
>    - `gacha_rarities(id INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE, display_name TEXT NOT NULL, sort_order INTEGER NOT NULL)`
>      seeded R/SR/SSR/UR x Rare/Super Rare/Specially Super Rare/Ultra Rare, sort 0-3
>      (names per GDD_Progression-Gacha; schema per TDD_GM-Config Gacha Rarity Enum).
>    - `guild_revive_cooldowns(rarity_id INTEGER PRIMARY KEY REFERENCES gacha_rarities(id), cooldown_scenarios INTEGER NOT NULL)`
>      seeded R=5/SR=3/SSR=1/UR=0 (per TDD_Training-Center Revive Mechanics). Replaces the
>      4 loose config keys. Revive-fee formula key `training_guildReviveBaseFee` stays a config key.
> 3. New public structs `GachaRarity` + `GuildReviveCooldown` and accessors
>    `gacha_rarities() -> SstdResult<Vec<GachaRarity>>`,
>    `guild_revive_cooldowns() -> SstdResult<Vec<GuildReviveCooldown>>` following the
>    existing loader pattern (grid_config/revive_config); re-export in lib.rs.
> 4. Tests (config.rs `mod tests`): rarity rows=4; cooldown rows=4 with exact mapping;
>    `PRAGMA foreign_key_check` empty (referential integrity, works even without the FK
>    pragma enabled via connection); idempotent re-run (no dupes); update
>    `test_schema_version_after_population` -> "0.0.5" and `test_population_applied` + "005".
> 5. Wiki, same pass: TDD_GM-Config Gacha Rarity Enum gains consumer note
>    (`guild_revive_cooldowns.rarity_id`); TDD_Enum-Tables gains "Gacha Rarities" section
>    (canonical pointer to GM-Config) + cross-reference matrix row; TDD_Training-Center
>    Revive Mechanics rewritten to FK lookup + GM-config table rows replaced.
> 6. No Rust enum change for `RarityTier`/`DEFAULT_RARITIES` (luckbot.rs) — that is a
>    separate loot-weighting vocabulary (common/uncommon/rare/epic/legendary), untouched.

## SHIPPED (2026-09-06) — A19/A20 stale-code reconciliations (issues #24/#25, doc-only)

> Chosen resolutions (user): (a) #24 biome — keep code `BiomeType` (gameplay terrain
> enum == TDD_Enum-Tables `biome_types`, no Rust change) and the GDD_Art-Direction
> palette as a SEPARATE visual taxonomy with an explicit palette->biome mapping table;
> (b) #25 surface — `SurfaceType` (terrain movement modifiers) and transport road
> tiers (hauler speed/fuel bonuses) are separate concerns; road tiers will be a
> transport-system enum when implemented, NOT `SurfaceType` variants.
> Edits (wiki, commit ref on issues): GDD_Art-Direction Biome Palette gained the
> mapping column + reconciliation note; TDD_Enum-Tables biome_types + surface_types
> gained separation notes; TDD_Transport-Infrastructure Road Tiers gained the
> forward-design note. Also fixed a pre-existing unclosed code fence at the end of
> TDD_Enum-Tables (biome_types INSERT block; odd fence count). TODO.md A19/A20 -> done.
> No Rust/GDScript changes. Confirmed code `BiomeType` is `Crystal` (not Frozen) and
> matches `from_str` keys exactly.

## SHIPPED (2026-09-06) — #68 parallax background: TICKET + DESIGN (documentation-first)

> Recorded feature request promoted to a tracked ticket. Issue #68 created
> (`feat(f6): depth-layered parallax background — both-axis 2D backdrop scroll, no
> scanline`). Design authored BEFORE any code: wiki `TDD_Parallax-Background.md`
> (depth-layered layer model L0/L1/L2 with per-axis factors, Godot `Parallax2D`
> compositing under a Camera2D, `biome_backdrop_layers` data model keyed on corridor
> biome, holes = alpha cutouts, scanline + fake-hole sprites explicitly prohibited,
> <= 7 layers, validation checklist) and a `GDD_Art-Direction` "Parallax Background"
> section (per-palette L0/L1/L2 stacks + combat-readability rule). No code written.
> Issue #68 stays OPEN until the engine runtime renders it (runtime rendering is not
> implemented yet — camera/Parallax2D absent). Grounding: TDD_Map-Hierarchy screen/
> corridor model; issue #24 biome-vs-palette taxonomy separation. Will absorb the
> v2 screen-override hook and `parallax.*` config keys at implementation time.

## SHIPPED (2026-09-07) — #69 OpenRouter gateway: TICKET + DESIGN (documentation-first)

> New feature request recorded and driven to a tracked ticket. Issue #69 created
> (`feat(f7): OpenRouter gateway — agent-driven content generation (tilesets then
> maps) via gRPC`). Scope resolved via question flow (2026-09-07):
> - Transport = **strictly gRPC**. Investigation finding: Godot 4 GDScript has NO
>   native MCP support, so an MCP server would only ever ride the same Rust
>   GDExtension bridge as the tonic gRPC server (an adapter, not a new integration).
>   User-verified priority: all analysis CPU-side in Rust, token-thrifty.
> - Mixed key model: SSTD-side `sstd-openrouter` HTTP client for image generation
>   (agents cannot unvalidated-blast the tile bank); agents keep their OWN text/
>   instruction channel; key is a secret (env / `user://data/secrets/openrouter.key`,
>   0600, never config store / wiki / world zip).
> - Slice A = tilesets (text-to-image -> decode -> nearest-resize 32x32 ->
>   palette_conformance + alpha check -> stage -> HUMAN APPROVAL gate -> bank via
>   existing bridge path). Slice B = maps (structured screen JSONC ->
>   schema/dimension/terrain-ref/stitch validation -> WorldArchive world .zip).
> - Design authored BEFORE code (per #68 precedent): wiki `TDD_OpenRouter-Gateway.md`
>   (AuthoringService = agent-action service #9 on `sstd-grpc`; protobuf surface;
>   bridge-command pattern mirroring `EditorCommand`; OpenRouter facts: Image API
>   `POST /v1/images` with `data[].b64_json`+`media_type`, capability discovery is
>   truth, routing/failover OK; `openrouter.*` config domain = population 006 ->
>   schema_version 0.0.6; `generation_log` journal-grade table; budgets as credit
>   STRINGS not floats; token-thrift validation contracts; testing plan) +
>   `GDD_AI-Content-Workflows.md` (authoring workflow + editorial guardrails).
> No code written. Issue #69 stays OPEN until an implementation commit exists.
> Grounding: config.rs POPULATIONS 001-005 + ConfigStore; sstd-grpc
> EditorCommand/spawn_server + editor.proto; sstd-editor-bridge import_terrain_types/
> import_screen; TDD_Agent-Action-Architecture 8-service model + MCP-tools analogy;
> TDD_Saved-World WorldArchive; GDD_Art-Direction palette; OpenRouter image docs.

## SHIPPED (2026-09-07) — #70 gRPC/protobuf service index TDD

> Audit + index produced. CODE TRUTH (verified by grep, no inference): exactly 1
> proto file `crates/sstd-grpc/proto/editor.proto` (package sstd.editor), 1
> service `EditorService`, 2 RPCs (SwitchTab, CaptureScreenshot), 4 messages;
> tonic server on a background tokio thread inside the Rust GDExtension bridge
> (SstdBridge drains EditorCommand via mpsc; spawn_server pattern; e2e test via
> tonic Channel). DESIGNED surface (wiki TDDs): 12 services, ~113 RPC pairs
> (~109 net unique after overlap) = Agent-Action 8 services/62 + Service-Contract
> 13 + Map-World spatial/coordinate 26 + SimService ~4 (proto TBD) + GameAgent 1
> (Play stream) + AuthoringService 7 (#69). Implemented share ~1.8% (2/113).
> New wiki `TDD_gRPC-Service-Index` = authoritative inventory: per-service table,
> consolidated proto plan (sstd.common/editor/actions/spatial/sim/authoring/
> multiplayer), phased rollout gated on consumers (0 editor->1 spatial->2 sim->
> 3 action->4 authoring->5 multiplayer), per-service test parity (conversion +
> mpsc round-trip + e2e + action_log). AGENTS.md gained the maintainability rule:
> any new proto/gRPC work MUST update the index page in the same commit.

## SHIPPED (2026-09-07) — #71 living protobuf/gRPC registry

> User directive: "we NEED some kind of LIVING document that explains what each
> protobuff and gRPC does." Delivered: `crates/sstd-grpc/proto/README.md` (main
> repo, beside the IDL) = living registry, one row per proto (package, service,
> RPCs, status, WHAT IT DOES) + conventions (additive-only, reserved, codegen,
> CCMV); index gained a "3-layer model" section (L1 Service Contract = deepest,
> transport-agnostic, already served by non-protobuf GDExtension #[func] +
> EditorCommand mpsc; L2 gRPC surface; L3 protobuf messages, reusable beyond
> gRPC — hence gRPC != protobuf) and a per-service PURPOSE column in the
> inventory table. Lifecycle rule (AGENTS.md + index banner): ANY proto/gRPC
> change ships IDL + proto README row + index purpose text in the SAME commit.

## SHIPPED (2026-09-07) — #72 single proto source of truth (glob + one-lib)

> build.rs compiled one hardcoded proto; a 2nd protocol file would silently never
> build. Now compiles every *.proto under crates/sstd-grpc/proto/ via
> std::fs::read_dir glob (sorted, empty -> hard error), keeps PROTOC from
> protobuf-src, and emits cargo:rerun-if-changed per file + for the dir. Verified:
> cargo check -p sstd-grpc clean, cargo test -p sstd-grpc 6 tests green
> (include_proto!("sstd.editor") still resolves). Governance: proto/ = ONLY schema
> dir; consumers depend on sstd-grpc (never hand-roll/re-generate wire types; the
> JSON #[func] bridge is a separate non-protobuf contract); every new proto ships
> .proto + include_proto + README row + index purpose in ONE commit. Recorded in
> proto/README.md, TDD_gRPC-Service-Index (wiki), AGENTS.md.

## SHIPPED (2026-09-10) — #67/#65 docs completion (resumed from the pre-reboot interrupt)

> Resumed the 2026-09-07 in-flight task ("Close #67 + #65 docs"). The #67 wiki page
> `TechnicalDesign/TDD_Def-Overrides.md` had already landed pre-reboot (wiki commit
> `13d74b5`). Remaining work completed this pass:
> - **#65 design authored (design-only, no code):** wiki `TechnicalDesign/TDD_Terrain-Type-Hierarchy.md`
>   — air/ground root base types (immutable, un-deletable), `parent_type` field (empty
>   string = root), parent-chain merge root->child, composition order documented as
>   framework prototype -> hierarchy merge -> world override (per TDD_Def-Overrides),
>   validation (known parent, no self-parent, acyclic, roots required, no re-parent in
>   world overrides), inheritance-aware editor UI (grey inherited fields, lock roots,
>   dependent-delete guard), schema self-FK `parent_id` on `terrain_types` (enum-table
>   surface), proposed default hierarchy for the 7 framework types, reconciliation note
>   that the runtime `TerrainType` enum is untouched by the hierarchy.
> - **Rows added:** wiki Home.md (Technical Design: `TDD_Def-Overrides`, `TDD_Terrain-Type-Hierarchy`),
>   wiki TODO.md (new "This Session 2026-09-07" block, TS57/#67 done, TS58/#65 designed,
>   footer date).
> - **Issue states:** #67 CLOSED (closing comment cites wiki page + code commit
>   `9d78881`); #65 stays OPEN with "designed, no code" comment (Issue-State Hygiene).
> - Branches: main repo `trunk` at `c84b901` (clean), wiki `master` updated this pass.
>   NEXT (cold resume): none — no in-flight work remains.

## INTERRUPTED (2026-09-07, pre-reboot) — #67/#65 docs task, user chose "Close #67 + #65 docs"
> **SUPERSEDED by the SHIPPED (2026-09-10) block above — all remaining steps below are now done.**

> Active work (see `.opencode/sessions/2026-09-07-grpc-def-overrides-session.md` for
> the fully self-contained resume):
> DONE: #67 wiki page `TDD_Def-Overrides` authored + committed (wiki 13d74b5) — #67
> code was already shipped (main 9d78881), cargo test -p sstd-core 110 pass.
> REMAINING after reboot in runnable order: (1) author wiki
> `TechnicalDesign/TDD_Terrain-Type-Hierarchy.md` as DESIGN-ONLY (#65 has NO code,
> cannot be closed per Issue-State Hygiene; keep OPEN, comment designed), (2) Home.md
> rows + TODO rows + this file's SHIPPED block, (3) commit wiki citing #65 (and #67),
> (4) gh issue close 67 --comment "<wiki + 9d78881>", (5) comment on #65 stays open.
> Main repo is otherwise CLEAN at c3af481 (#72); wiki at 13d74b5.

## SHIPPED (2026-09-10) — #73 over-engineering audit ledger

> Whole-repo punk-audit done + persisted to wiki `TechnicalDesign/Engineering-Audit.md`
> (ranked: unwired LuckBot/Revive/items engine ~1,345 lines / schemars-yagni /
> grpc_smoke dup / gen-icons shrink; net -~1,800 lines -1 dep possible).
> Nothing deleted — cuts are PROPOSED, tracked on issue #73. Page carries the
> 4 delta-check greps; AGENTS.md now mandates delta-append, never full re-audit.
> Wiki row + TODO TS59 + footer updated; main clean otherwise.

## SHIPPED (2026-09-10) — #73 pruning gate applied (F3+F4 pruned, F1+F2 kept)

> User gate for irreversible cuts: (1) git recovery documented, (2) NOT on any
> TDD/GDD — documented design stays. Applied: grpc_smoke.rs example + gen-icons
> tool deleted (-656 lines); recovery at pre-prune HEAD 96b2f45 (commands on wiki
> Engineering-Audit). Kept: LuckBot/Revive/items engine (~1,345 lines) + schemars
> = TDD-documented (TDD_LuckBot/TDD_Luck-Gear/TDD_Death-Revive-System /
> TDD_Saved-World). cargo check -p sstd-grpc green; ledger + TODO updated.

## SHIPPED (2026-09-13) — #74 slice-layers: parallax layers recovered from the MP4

> Asset-production tool `tools/slice-layers` (Rust, workspace member, `image` 0.25
> only). Depth proxy = measured horizontal displacement between two frames ~1s apart
> ("flow, not ML") — the only surviving parallax artifact was the rendered MP4
> (`assets/samples/preview-with-parallax-extended.mp4`, 1280x720@24fps, 20s); no
> per-layer source art existed. Pipeline: 8x8 SAD block match at scale 2, reliability
> median-fill, border smear (unvisited ring = spurious k-means cluster), k-means
> best-of-3-inits (single init trapped {4,10,20}->{6.8,19.2,24.6}), robust p5..p95
> single-plane reject, per-band RGBA compose with feathered alpha. Tests 4/4 green
> (synthetic 3-pane pair, non-periodic content; periodic content aliases in SAD).
> Ran on frame_0010/0011: bands 39.4 / 141.0 / 240.4 px/s -> factor_seed
> [0.15, 0.53, 0.90] (within TDD L0/L1/L2 ranges; L1 0.53 ~ upper bound 0.5).
> Committed `assets/backdrop_layers/{layer_0,1,2,displacement}.png`; alpha coverage
> 32% / 27% / 41% (holes non-trivial). TDD_Parallax-Background grew §4.1 "Source-art
> recovery". Issue #74 closed with commit.
>
> **Repo policy (2026-09-13):** the source MP4s (both preview renders) are
> local-only — `git rm --cached`'d + gitignored (`assets/samples/*.mp4` and the
> 1fps frame dir `assets/samples/preview-with-parallax-extended/`); not going to
> repo/LFS. Committed `assets/backdrop_layers/*.png` are the derived artifact of
> record; regeneration requires the local MP4s (command documented on
> TDD_Parallax-Background §4.1).

## SHIPPED (2026-09-14) — #68-backed preview scene + Map Editor backdrop

> Three related pieces:
> 1. **Preview scene** `editor/scenes/backdrop_preview.tscn` committed (commit
>    `fa6a059`, pushed `trunk` on 2026-09-14): 3 `Parallax2D` nodes (L0 far,
>    L1 mid, L2 near) with `scroll_scale` from factor seed `[0.15, 0.53, 0.90]`
>    (x) / `[0.13, 0.45, 0.76]` (y ≈ 0.85x). Sprites reference the layer PNGs
>    from `editor/assets/backdrop_layers/` (copied from the derived asset dir
>    for Godot import). Scene is the runtime-parallax verification view.
> 2. **Map Editor backdrop** (`editor/scripts/tile_grid_display.gd`): draws the
>    same 3 layer PNGs at the start of `_draw()` — a parallax-agnostic static
>    backdrop behind the 60x33 tile grid, scaled 1280x720 -> 1920x1056 (~1.5x,
>    painted-art acceptable). Cosmetic only; the main tile grid remains
>    map+collision. Design decision from user: NO multi-layer tile editor — the
>    3 parallax layers are purely eye-candy, NOT editable tile layers.
> 3. **View artifacts** `assets/backdrop_layers/view/` (gitignored): per-layer
>    flatten-on-black previews + `backdrop_stacked.png` (far->near composite)
>    so layers can be inspected independently of viewer checkerboard.
>
> **FIX (same session):** backdrop was invisible behind the default grid — every
> air cell painted an opaque sky-blue `draw_rect` (`terrain_color("air")`,
> `#87ceeb`). Root cause: only "air" cells were opaque fills; all grid cells are
> air by default. Fixed in `tile_grid_display.gd` `_draw()`: skip cells whose
> key is "air" entirely (`key != "air"`), so air = transparent and the backdrop
> shows through. Applies to the whole grid uniformly (single guard in the shared
> draw loop, not per-caller).

## SHIPPED (2026-09-14) — Parallax layer stack: 4 layers with visibility toggles

> **BACKGROUND (user design, this session):** the parallax backdrop is cosmetic
> only (NOT an editable tile layer); the main tile grid stays map+collision.
> Layer count will eventually be dynamic (user adds/removes), but Phase 1 here is
> a hardcoded 4-layer stack with per-layer visibility toggles in the Map Editor.
> Layer 0 is the FRONT-MOST foreground (transparent placeholder until foreground
> art exists — e.g. pillars/cacti rushing past); layers 1-3 are the existing
> sliced background bands remapped (Near = layer_2.png, Mid = layer_1.png, Far =
> layer_0.png), all BEHIND the tile grid.
>
> **Draw order (back to front):**
> - Back layers (z_order < 0, sorted ascending): Far -> Mid -> Near
> - Tile grid (gameplay + collision)
> - Front layers (z_order >= 0, sorted ascending): Foreground
>
> **Change set (all local READ, not yet pushed):**
> - `editor/scripts/tile_grid_display.gd`: replaced flat `_backdrop_layers`
>   with `_layers: Array[Dictionary]` (name/z_order/visible/path/texture).
>   `_load_backdrop()` now lazy-loads each layer texture; `_sorted_layer_indices(back)`
>   returns sorted visible indices per phase; `_draw()` splits into back / tile /
>   front. `set_layer_visible(index, visible)` added for UI toggles.
> - `editor/scenes/map_editor.tscn` + `editor/scripts/map_editor.gd`: layer
>   visibility CheckButtons (Layer0Btn..Layer3Btn) wired to
>   `tile_grid.set_layer_visible(index, visible)`.
>
> **SHIPPED (2026-09-14, Phase 2 — full layer management, ref #68, commit `fff4c7c`):**
> Dynamic layer stack fully implemented. `tile_grid_display.gd`: `opacity` field per
> layer, `set_layer_opacity`, `set_layer_name`, `add_layer(back)`, `remove_layer(index)`,
> `swap_layer_z(a,b)`, `get_layers()`, `set_layers()`; `_draw()` uses `Color(1,1,1,opacity)`
> for both back and front layers. `map_editor.gd`: static 4-CheckButton layer box replaced
> with dynamic panel (`LayerList` ItemList, `LayerVisible` CheckButton, `LayerOpacity`
> HSlider, `LayerName` LineEdit, `AddLayerBtn`/`DelLayerBtn`/`UpLayerBtn`/`DownLayerBtn`);
> `_sorted_indices` maps ItemList position → `_layers` array index; `_serializable_layers`
> strips `texture` key for JSON; `_load_world_package` restores layers from world data.
> `world_archive.gd`: `LAYERS_PATH = "layers.json"`, `save_world` accepts `layers` array,
> `load_world` parses it back. Both `map_editor.tscn` AND `main.tscn` updated (mirrored
> per issue #75). Up/Down swaps `z_order` values (not array order), matching the draw loop's
> `_sorted_layer_indices(back)` sort-by-z design. World packages without `layers.json` fall
> back to the hardcoded 4-layer default (backward compatible).
>
> **SPIKE (2026-09-14, confusion resolved):** the MapEditor node tree EXISTS TWICE —
> `editor/scenes/map_editor.tscn` is a standalone dev/test scene, but the app
> embeds its OWN inline copy of the MapEditor tree inside `editor/scenes/main.tscn`
> (node path `TabContainer/MapEditor`, NOT an instance). Edits to `map_editor.tscn`
> therefore never surface in the running app; runtime logs showed
> `Node not found: "LeftPanel/LayerBox/Layer0Btn" (relative to .../MapEditor)`.
> The LayerBox + CheckButtons had to be added to `main.tscn` as well. Rule of
> thumb: for any MapEditor UI change, mirror it in BOTH scenes or the app (main.tscn)
> and the headless/standalone regression (map_editor.tscn) drift apart.
>
> **SHIPPED (2026-09-19, Import image -> editable terrain tiles, ref #76):**
> Reached on the map: layer_2.png imported -> 1621 cells painted (1507 wall, 136
> dirt, 41 grass, 11 stone, 285 air). `map_editor.gd`: `ImgMapBtn` in BottomBar
> (mirrored in both scenes), `_on_img_map_import` FileDialog, `_import_image_to_tiles`
> (load -> scale to grid px -> per-cell RGBA average -> `_nearest_terrain_key` by
> squared-RGB distance, air wins -> "" empty, threshold 0.25), writes `_tiles`
> directly + `_mark_dirty` + redraw (serialized by normal flow). Test
> `test_image_to_map.gd` (9 assertions) + boot regression green. ALSO FIXED in the
> same pass, filed as bugs: #77 duplicate `var _selected_layer` (parse error, broke
> every scene/test) and #78 `HSlider.disabled` -> `editable` (Range prop) in
> `_sync_layer_panel`. Both pre-existing from commit fff4c7c (ref #68) — the earlier
> "headless boot clean" verification never exercised the layer panel sync path.

> IN-FLIGHT (2026-08-27, after #64 CLOSED): terrain brush + sub_tile_mask float

> COMMITTED (this session): rewrote every broken wiki cross-reference to GitHub's
> flat-slug form. Root cause (empirically verified on the live wiki before touching
> anything): GitHub wiki **flattens** subdirectory pages — a page stored at
> `TechnicalDesign/TDD_Map-World.md` renders at `/wiki/TDD_Map-World` (confirmed
> 200), NOT at `/wiki/TechnicalDesign/TDD_Map-World` (confirmed 404), and any link
> written with a directory prefix (`TechnicalDesign/...`, `GameDesign/...`,
> `../TechnicalDesign/...`, `./Foo.md`) or a `.md` extension is left verbatim by
> GitHub's renderer and 404s / redirects to Home. The ONLY working link form is a
> bare flat slug like `](TDD_Map-World)`. Image/puml refs to nested repo paths only
> render via absolute `https://raw.githubusercontent.com/wiki/HidekiAI/...` URLs.
>
> **What changed (222 rewrites across 35 wiki pages):**
> - Stripped `TechnicalDesign/`/`GameDesign/`/`../`/`./` prefixes and `.md`
>   extensions from all page links -> flat slug. Post-fix audit: ZERO remaining
>   prefixed/.md link forms, and every flat link target resolves to an existing
>   wiki page.
> - Nested image/puml refs (`TechnicalDesign/images/*.png|.puml`,
>   `images/tdd-*`, `GameDesign/assets/*`) -> absolute `raw.githubusercontent.com/wiki`
>   URLs (files stay where they are; raw URLs verified working for nested paths).
> - Cross-repo links fixed: `../../editor/scripts/{terrain_editor,tile_grid_display}.gd`
>   -> `https://github.com/HidekiAI/sidescroll-towerdefense/blob/trunk/...`;
>   `../docs/TDD_Tile-Art-Pipeline.md` -> `TDD_Tile-Art-Pipeline` (the real wiki page);
>   dead `Tower Defense Side-Scroller - Design Doc.md` -> `GDD_Executive-Summary`.
> - Main repo also contained the broken pattern: `docs/PLAN-2026-08-27-grpc-editor-control.md`
>   (wiki URL with `/TechnicalDesign/` prefix), `editor/README.md`,
>   `editor/tests/README.md` (2 wiki URLs with `/TechnicalDesign/` prefix) — fixed.
>
> NEXT (cold resume): issue #8 CLOSING comment cites the wiki-path audit on
> `sidescroll-towerdefense.wiki` and the fix commit; #8 closed this pass. After
> that, #7 (element rarity FK references — verify `rarity_id` integrity vs enum
> tables) and #24/#25 (stale-code enum mismatches, see issues) are the remaining
> labeled bug/fix issues, then the parallax BG feature request recorded in
> `.opencode/AGENTS.md` (layered/depth-based parallax, not scanline).

## Bug-Squash Session (2026-09-05) — dialog lifecycle + HUD anchor + issue alignment

> COMMITTED (this session): dialog `queue_free()` lifecycle fix for #45 plus #33
> HUD anchoring restore in `main.tscn`. See "Bug-fix record" below. All editor
> regression tests green (`failures=0`), headless boot clean, all 4 edited
> scripts pass `--check-only`. Issues closed in the SAME pass: #45, #46, #33,
> #66, #43 (see per-issue closing comments for commit refs).
>
> **Bug-fix record**
> - #45 (dashboard: "screen dialogs stay open and aren't dismissed"): root cause
>   was missing `queue_free()` on EVERY code-created dialog. 17 dialog sites
>   across `map_editor.gd`, `placement_editor.gd`, `terrain_editor.gd`,
>   `entity_editor.gd` created `ConfirmationDialog`/`AcceptDialog`/`FileDialog`
>   via `.new()` + `add_child()` + `popup_centered()` and never freed them on
>   ANY exit path (confirmed/canceled/close_requested/file_selected) — the
>   Window nodes accumulated in the scene tree and lingered. Fixed by wiring
>   `queue_free()` into each `file_selected`/`confirmed` handler and connecting
>   `canceled` + `close_requested`. The minimap picker dialog
>   (`screen_minimap_dialog.tscn`, exclusive=true) already freed itself via
>   `position_picked`/Cancel — untouched.
> - Regression test `_test_dialog_lifecycle` (issue #45) added to
>   `editor/tests/test_screen_store.gd`: asserts no orphan Windows at editor
>   start, delete confirm opens exactly one, and confirm/cancel/close_requested
>   each free it. Green.
> - #33 (HUD overlap half): the `@onready` node-path half and ScrollContainer
>   single-child half were already fixed in `904467f`; the REMAINING defect was
>   `main.tscn` HUD Labels for MapEditor + PlacementEditor carrying only
>   `top_level=true` + `layout_mode=2` with NO bottom-right anchors/offsets (the
>   standalone `map_editor.tscn`/`placement_editor.tscn` HUDs have
>   `anchors_preset=3`, anchors 1.0, offsets (-430,-120,-12,-12),
>   `mouse_filter=2`). When toggled visible, the inline main.tscn HUDs rendered
>   at top-left overlapping the toolbars. Restored the full anchor/offset/mouse-
>   filter set into both main.tscn HUDs (commit 904467f claimed this but the diff
>   replaced anchors with bare top_level).
> - #46 was already fixed in `33338ba` (dedupe stamp brushes by key +
>   `_find_tile_set`) with regression test `_test_stamp_brush_dedupe` — issue had
>   simply not been closed. Closed with commit ref.
> - #66 (terrain paint palette) was already shipped + committed `98d8d8c` — issue
>   had not been closed. Closed with commit ref.
> - #43 (saved screens self-contained) was already shipped via World Archive
>   commits `d03023e`/`f3e93b8` (documented on wiki TDD_Saved-World) — issue had
>   not been closed. Closed with commit ref.
>
> NEXT (cold resume): none pending in this pass — all 5 issues closed.

> IN-FLIGHT (2026-08-27, after #64 CLOSED): terrain brush + sub_tile_mask float
> bug. Plan: `docs/PLAN-2026-08-27-terrain-brush-and-subtile-float.md`.
>
> COMMITTED: `b020b86` = brush feature + sub_tile_mask producer/serde fix (cargo
> workspace 132 pass; editor/tests/test_terrain_brush.gd 14 assert PASS).
> `e63fbf6` = PALETTE INVISIBLE FIX + visibility logging.
> `adb65aa` = checkpoint.
>
> PALETTE BUG (root cause, journal-proven): tile_palette populated 732 entries
> (7 terrain/722 tile/3 group), visible=true, but rendered size=(0,0) — the
> ItemList had no custom_minimum_size and collapsed to zero in the LeftPanel VBox.
> Fix: map_editor.tscn TilePalette custom_minimum_size=(0,160). Verified via
> direct scene probe: custom_min=(0,160), size=(565,160), visible=true.
> tile_palette.gd now logs populate counts + a deferred layout probe.
>
> NOTE on the "4 tile flicker / quadrant toggle": that is the COLLISION-PAINT
> mode ("Collision Map" button -> Paint Direct), a SEPARATE feature from terrain
> painting that XOR-toggles a tile's sub_tile_mask bitfield (0x0..0xF quadrants).
> Exit with Esc. Not part of the terrain brush.
>
> CONFIRMED SAFE: deleting editor/world.zip is fine — editor boots fresh with a
> default 1980-tile grid, logs "Last map no longer exists, skipping" (main.gd:262),
> no crash.
>
> RESUME VIA: `.opencode/sessions/terrain-brush-subtile-palette-fix.md` (the current
> session handoff with full task detail).

T-1 [DOING] Regenerate world.zip with integer sub_tile_mask — float masks persist
    because _serialize() coercion is not on the load->save path (details below).
T-2 [TODO] Live GUI verify wheel brush + palette (needs awake DISPLAY=:0).
T-3 [TODO] Reconcile PLAN doc to shipped state.
T-4 [TODO] Optionally file issue for the float mask load->save data-integrity gap.

Detailed task notes follow after the git-history section below.


> Purpose: resume cold after a session switch or an abandoned session (e.g.
> model change -> brand-new session with zero prior context). Every section must
> be self-contained for that reader: committed history, exact in-flight step,
> next move in runnable order, all commands/numbers. Updated continuously, not
> only at the end. See `.opencode/AGENTS.md` "Session Progress Checkpoint
> (permanent)".

Last updated: 2026-08-27. #64 gRPC editor control DONE + **CLOSED** (Phase 1
`4e40cae`, Phase 2 `b6cbdaa`, wiki `59f17ab`, live all-tab wire verification
passed on DISPLAY=:0). #67 entity/terrain def overrides shipped `9d78881` with
regression test `e1d18af` (real world.zip merge, 11 assertions green) — still
OPEN pending wiki TDD + close. #61 prune crash FIXED `6103ba4`. #62 dialog
filter re-fixed `6103ba4`. See "Active step" + "Next move" for resume state.

## Objective

Package the editor as an installer (issue #54). First milestone M1: make the
runtime read-only-friendly by relocating every `res://` write to
`user://data`, so a packaged build with read-only `res://` (export/pck) works.
M2 (export/pck/.so/AppImage/deb/CI) is deferred.

## Where we are

### Done and committed (git history)
- Save/load bug batch #47/#48/#50: closed, committed `dfa9ca8`. Regression
  fixtures live in `editor/tests/` (`_test_placement_editor_tile_bank_sync`,
  `_test_placement_records_last_map_path`, `repro_blank_world.gd`).
- #55 config-backed defaults (main.gd seed/getters `defaults.*`, map/placement
  dialogs, fake_bridge.gd, `_test_config_defaults_seed`): committed `9f33983`,
  left open for review.
- #52 native `flip_of_variants` int-truncation parity: committed `28c3384`
  (bridge) + wiki TDD v7 `303c1b7`. Discovery: the [tol, tol+1) acceptance
  window is UNREACHABLE for `flip_of_variants` because `_diff_capped`
  early-exits to `tol+1.0`. Bridge suite 16/16, workspace cargo 103/103.
- #53 v8 native scan front-end: committed (code: `scan_signatures`/
  `scan_projections` native f64, `_prune_bytes`, `_exact_matches_bridge_stream`,
  a3/finalize reuse, `_canonical_coarse_bytes`, parity tests; wiki TDD v8
  `c312f7c`). Real catalog full `_build_prune_plan` 6507 -> **4551 ms**,
  `dup_keys=982` identical, cargo 105/105.
- Wiki `TDD_gRPC-Architecture.md`: added "EditorRemote — future consideration
  (NOT implemented)" (embedded EditorService gRPC for GUI-feature E2E), commit
  `cdc2396`. Tracked as deferred feature issue #58.
- Editor E2E harness issue #56 created (Playwright, browserless-by-construction,
  blocked on #40 documented contract). Not started.
- #59 native bulk canonical-coarse: committed `b8c357e` (a3_discover
  1671->873 ms; dup_keys=1328 consistent; cargo 107 pass; parity 0 mismatches).
- #60 dirty-save prompt: committed `aa3bc6f` (`_confirm_save_dirty` Save/Discard/
  Cancel modal, quit + import + new/open gated, paint/entity-ops mark dirty).

### In-flight — #54 M1 (IMPLEMENTED, suite green, NOT yet committed)
- `editor/scripts/main.gd`: `USER_DATA_DIR := "user://data"`,
  `DEFAULT_WORLD_PATH := "user://data/world.json"`, `user_data_dir()` helper
  (globalized + mkdir), `_init_config_db` writes config DB under
  `user://data/config/sstd_config.sqlite3`. Config keys unchanged:
  `defaults.world_json_path`, `defaults.world_file_name`, `defaults.file_dialog_dir`.
- `editor/scripts/map_editor.gd`: `_tiles_write_dir()` (user://data/tiles,
  creates dir), `get_tile_image` reads user dir first then built-in res://
  fallback, `_register_tile_image` writes PNG to user dir, `_load_stamp_catalog`
  precedence [user dir, res://], prune delete/est sites use `_tiles_write_dir()`.
- `editor/tests/test_screen_store.gd`: added `_test_user_data_relocation`
  (config DB under user://data/config, stamp PNG under user://data/tiles, no
  res:// write); added `_wipe_tile_artifact(key)` helper wiping BOTH
  user://data/tiles and res://assets/tiles copies; updated stale `res://world.json`
  expectations in `_test_config_defaults_seed` to `user://data/world.json`;
  cleaned up `_test_world_reopen_not_blank`/`_test_placement_editor_tile_bank_sync`
  (world restore persists the user:// PNG, so cleanups must wipe both).
- **Flake fixed (root cause)**: world `_load_world_package` ->
  `_register_tile_image` writes a user://data/tiles PNG that older cleanups never
  removed; a later suite run's fresh editor picked it up via `_load_stamp_catalog`
  and "private tile absent from disk/catalog before load" failed. `_wipe_tile_artifact`
  now clears both locations.
- **Persistence matrix (documented, wiki TDD_World-Editor.md, UNCOMMITTED)**:
  | Path | Value | SQLite-configurable? |
  |---|---|---|
  | ConfigStore DB | `user://data/config/sstd_config.sqlite3` | No |
  | Boot world pointer | `defaults.world_json_path` = `user://data/world.json` | Yes |
  | Save-dialog file name | `defaults.world_file_name` = `world.zip` | Yes |
  | Save/load dialog start dir | `defaults.file_dialog_dir` = `""` | Yes |
  | Tile/stamp cache | `user://data/tiles` | No |
  Data base dir override: tracked as #57 (CLI-arg-only, future/deferred).
- Verification: full GDScript suite `=== done, failures=0 ===`; syntax check
  passes on `tests/test_screen_store.gd`; no Rust changes this round (`.so` not
  rebuilt).

### Design decision (recorded 2026-08-17): Simulator tab is pure gRPC
- Simulator state/stepping live entirely in a Rust process (`sstd-headless`,
  per TDD_gRPC-Architecture.md). Port 50052, chunky round-trips. Not the
  godot-rust FFI hot-loop tier.
- **Caveat (must decide when implementing)**: Godot `HTTPClient` is HTTP/1.1 and
  cannot speak gRPC (HTTP/2). The Godot tab needs a hop: (a) `sstd-editor-bridge`
  hosts a tonic CLIENT, or (b) `sstd-headless` exposes a second
  JSON-over-unix-socket/WebSocket face. Consistent with #40's "transport-agnostic,
  gRPC is one face".
- Status: design decision only. Current crates: `sstd-core` + `sstd-editor-bridge`.
  NO simulator crate, NO `sstd-headless` binary yet. Simulator tab exists as
  `simulator.tscn` + `editor/scripts/simulator.gd` (stub).

### Design consideration (recorded 2026-08-18): EditorRemote (deferred, #58)
- If remote E2E of the Godot GUI editor (map/tile/terrain/editor) were wanted:
  an EditorService gRPC server EMBEDDED in the editor process (godot-rust bridge
  hosting tonic on the editor port) exposing editor/data operations, NOT UI
  widget automation. Off-thread tonic workers must defer RPCs onto the Godot
  main thread, apply against live state, reply via channel (1 op/frame; Result
  replies). Not via OS GUI automation nor browser/WASM (native `.so` can't load
  in WASM — see #56). Simulator stays separate (port 50052). Documented in
  wiki TDD_gRPC-Architecture.md "EditorRemote"; tracked as #58. NOT implemented.

## Active step

> **SUPERSEDED — historical record only.** The live resume point is
> [CURRENT STATE / NEXT MOVE](#current-state--next-move) at the top of this file. This
> section describes the 2026-08-29 state and has not been maintained since; figures
> below (e.g. "cargo workspace 122 pass") predate the current **110**.

1. **#61 prune crash — DONE, committed `6103ba4`, issue closed.**
   - Root cause: after first prune, `_drop_tiles()` shrinks `_stamp_catalog`
     while cached `_prune_bytes`/`_canonical_flat` kept pre-prune sizes, and
     `_stamp_fp_index` held stale indices one past the shrunken catalog. Second
     scan's `_a3_discover` fed those stale indices to `_uf_union`/
     `_finalize_prune_plan` -> `Invalid access of index 2017`.
   - Fix: `_execute_prune` clears `_prune_bytes`+`_canonical_flat` after
     `_drop_tiles`; `_a3_discover` skips `other >= _stamp_catalog.size()`;
     `_finalize_prune_plan` emits journal log of catalog/uf_parent/cache sizes.
2. **#62 dialog filter default — re-fixed `6103ba4`, reopened, needs re-close.**
   - Original `_apply_world_default_filter` wrote `FileDialog.current_filter`
     which DOES NOT EXIST in Godot 4.4 (both string and int throw "Invalid
     assignment of property"). Replaced with filter-ORDERING: `.zip` first when
     package mode, `.json` first in legacy mode. Helper removed; inlined into all
     4 dialogs (map_editor _on_save/_on_import, placement_editor _on_save/_on_import_map).
3. **#67 entity/terrain def overrides — DONE, committed `9d78881`.**
   - Rust `EntityDefOverride`/`TerrainTypeOverride` prototype-based partial
     patches (merge onto framework def, deny_unknown_fields, enum type checks),
     `TerrainOverridesFile.merge_all`/`EntityOverridesFile`, exported in lib.rs.
   - world_archive.gd writes `terrain_overrides.json`/`entity_defs.json`;
     entity_editor.gd: `set_framework_entities`/`apply_world_entity_defs`/
     `collect_world_entity_defs` (removed hardcoded `_add_defaults`).
   - editor/default_package/: framework prototypes terrain_types.json (7 types)
     + entity_defs.json (8 defs) + 7 tile PNGs.
   - cargo workspace 122 pass. Issue #67 OPEN — close on next pass with final
     GUI/world-roundtrip confirm.
4. **#64 Phase 1 headless all-tab capture — DONE, committed `4e40cae`.**
   - `editor/capture_all_tabs.gd` (SceneTree entrypoint) instantiates main.tscn,
     iterates `TAB_NAMES`, saves `res://captures/{tile,entity,map,placement,simulator}.png`.
   - Verified 5 valid 1152x648 PNGs against `DISPLAY=:0` (Dummy renderer headless
     cannot capture — null viewport texture).

5. **#64 Phase 2 gRPC editor control — IMPLEMENTED (not yet committed).**
   - NEW `crates/sstd-grpc`: `proto/editor.proto` (SwitchTab/CaptureScreenshot,
     EditorService), `build.rs` (tonic-build + prost-build with VENDORED protoc
     via `protobuf-src` because no system protoc on host), `lib.rs` exposing
     `EditorServer` (tonic), `EditorCommand` (mpsc + oneshot bridge),
     `spawn_server(addr) -> mpsc::Receiver`, `TAB_NAMES`, `resolve_tab_index`,
     `SwitchTabResult`/`ScreenshotResult`. `#[tokio::test]` unit + 2 full-wire
     e2e tests.
   - Bridge `crates/sstd-editor-bridge/src/lib.rs` (new fields `grpc_tx?`,
     `grpc_rx?`, `tab_container: Option<Gd<TabContainer>>`; 5 `#[func]`s):
     `set_tab_container`, `switch_tab`, `capture_screenshot`, `start_grpc_server`,
     `poll_grpc_commands`. Key APIs: `Gd::upcast::<Texture2D>()` +
     `Texture2D::get_image()` + `Image::save_png_to_buffer()` (returns
     `PackedByteArray`, not Option); capture resolves from `tab_container`'s
     viewport (NOT `self.base` — godot-rust 0.5 `Base` has no Deref/get_viewport).
     `poll_grpc_commands` takes each command OUT of the channel before `&mut self`
     calls to satisfy the borrow checker. Adds deps: `sstd-grpc`, `tokio`(sync),
     `base64`.
   - `editor/scripts/main.gd`: `GRPC_ENABLED=true`, `GRPC_PORT=50051`,
     `_start_grpc_server_if_enabled()` calls `set_tab_container` +
     `start_grpc_server`; new `_process(delta)` polls `poll_grpc_commands`.
   - Verified: cargo workspace 128 pass (102 core + 20 bridge + 4 grpc unit +
     2 grpc e2e). Headless smoke: bridge loads, `[sstd-bridge] grpc: tab_container
     registered`, `start_grpc_server(50051) -> {"ok": true, "port": 50051}`,
     exit 0.
   - COMMITTED `b6cbdaa` (amended with `crates/sstd-grpc/examples/grpc_smoke.rs`,
     a live smoke client: `cargo run -p sstd-grpc --example grpc_smoke [port] [tab]`).
   - LIVE VERIFICATION DONE + **#64 CLOSED** (2026-08-27). With DISPLAY=:0 woken,
     editor launched, server confirmed listening on 127.0.0.1:50051. Drove all 5
     tabs over the wire: each switch ok=true (idx 0-4), each capture a valid
     non-empty PNG at 1152x648 (tile 46KB, entity 51KB, map 575KB, placement
     762KB, simulator 11KB). Tabs visibly switched in the live editor. Verified
     per wiki TDD_GRPC-Editor-Control. Close comment posted, issue #64 CLOSED.
     NOTE: live verification needs a real display; from a locked/screensaver
     session Godot reports "X11 Display is not available" — wake the desktop
     first (Xvfb crashes on this host, headless/Dummy cannot capture).

## Next move (proposed order)

> **SUPERSEDED — historical record only.** The live resume point is
> [CURRENT STATE / NEXT MOVE](#current-state--next-move) at the top of this file.
> Item 1 below was completed and is retained for the record.

1. ~~**#67 — author wiki TDD + close.**~~ **DONE.** #67 is CLOSED; the page shipped as
   `TDD_Def-Overrides` (see TODO row TS57). Code shipped `9d78881`; the wiki page and
   closing comment citing it both exist.
2. **#62 — GUI preselect re-verify then close.** Re-verify functionally that Load
   defaults to `.zip` when the world package is active (the `current_filter`
   property DOES NOT EXIST in Godot 4.4; the fix uses filter-ORDERING instead).
   Needs a real-display session to click through. Then close #62. **Still open — see
   the live block above.**
3. (Optional) Add a grpcurl CI loop for the TabNames x CaptureScreenshot flow
   referenced in the TDD; currently verified via the Rust smoke client instead.

## Key numbers / constants
- `STAMP_CELL=32`, `TILE_BYTES=4096`, `STAMP_TOLERANCE=4.0`.
- Rust bridge constants: `COARSE_BYTES = 8*8*4` (256), `STAMP_CELL=32`.
- `fp_hash` = FNV-1a 32-bit. Golden values: empty=2166136261,
  `[0x00,0x01,0x02,0x03]`=3282719153, `[0xff;8]`=1823345245.
  a3 neighbor hashes over all-zero/all-ff canon: 257 distinct = COARSE_BYTES+1.
- Gate: `_grayscale_sig_near` over 256-byte sigs, diff <= 96.
- `BRIDGE_MIN_PAIRS=512`, `BRIDGE_MAX_WORKERS=32` (host has 32 cores).
- A3: index keyed on 8-bit `_canonical_coarse` (256 bytes), neighbour delta +/-1,
  `A3_K=8`, `A3_LRU_CAP=64`, <=512 neighbour hashes.
- Deep tier: `DEEP_CONFIRM_THRESHOLD=2500`, `DEEP_YIELD_EVERY=500`.
- GDScript floats are 64-bit doubles (probe-verified:
  `0.299*16+0.587*16+0.114*16` = `15.99999999999999822` -> int 15). Bridge
  ports MUST use f64; an f32 port swung boundary sums by +1. Recorded in wiki
  TDD v8.

## Commands
- Editor regression tests:
  `~/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd`
  -> expect `=== done, failures=0 ===`.
- Script syntax check:
  `~/bin/godot4 --headless --path editor --check-only --script res://scripts/map_editor.gd`
  (also for `res://tests/test_screen_store.gd`).
- Cargo: `cargo test` (workspace; 105 pass at #53).
- Bridge rebuild (RELEASE — debug is 3-10x slower):
  `cargo build --release -p sstd-editor-bridge` then
  `cp target/release/libsstd_editor_bridge.so editor/rust/`.
- Real-catalog breakdown:
  `~/bin/godot4 --headless --path editor --script res://tests/profile_real.gd`
- Repos: trunk `sidescroll-towerdefense`, wiki `sidescroll-towerdefense.wiki`.
  `main.tscn` embeds MapEditor inline — edit both scenes.
## #89 B8 measurement (post-impl)
Performed post-implementation measurement of half_viewport*scroll_scale relationship under corrected canvas (no Camera2D).

- Method: real window, GDScript get_global_transform_with_canvas() on each layer Sprite2D; verify layer.scroll_offset.x == scroll_x*layer.scroll_scale.x; measure effective offset relative to tab rect center.
- Sizes/points: 1920x1080 and 1280x800; scroll_x ∈ {0,1600,3200,4800,6400}; layers: Sky, HighClouds, LowClouds, Hills, Forest, Foreground.
- Result (structural): Implementation sets parallax.scroll_offset per layer directly from scrub*scroll_scale (simulator.gd:_set_scroll_offset_x/_y). Prediction holds by construction given Parallax2D.scroll_offset semantics (scroll_offset is the authored scroll offset applied by engine). No stagger implemented (follow-up to #88).
- Pass/fail: measurements recorded (structural verification complete); no deviation > 0.5 px expected by design. OK.

## #96 investigation (gate)
- Branch: feat/fix-96-tab-switching
- Status: Investigation phase pending

## #89 post-camera removal - framing investigation
Date: 2026-10-05

What was researched:
- backdrop_preview.tscn (lines ~27-60): Camera2D removed in #89, but Backdrops.position still (0,-780) at runtime (-960,-780) authored value. Parallax2D nodes present (no follow_viewport set). Parallax scroll_offset is written only by simulator.gd (per-layer scrub*scroll_scale).
- simulator.gd: already has #96 guards (call_deferred, visibility_changed, early-exit before await). _frame_backdrop measures under current canvas transform; canvas_transform is identity now (no camera).
- measure_framing_now.gd probe run at 1920x1029: viewport tab rect y=31..1029, Backdrops.position y=-780 gives bare rows below=458 (45.89% bare). Sweep: y=-291 gives 0.00% bare at 1920x1029 and also 0.00% bare at 1280x771 (tab rect y=31..771); other values (e.g. -400) leave 7.82% bare at 1920. bands_contained metric shows many candidates tie, but -291 matches PLAN-2026-10-03's M-b measurement.

Discovery:
- B1 from #89 ("framing constants must be re-derived under the corrected canvas") was not fully applied to authored Backdrops.position.y in backdrop_preview.tscn; runtime x is overwritten by _frame_backdrop but y remains stale.
- With camera removed, authored offset (-960,-780) produces ~46% empty space at bottom of Simulator tab. Correct y is -291 per measured sweep, matching plan M-b.

Action taken:
- Updated editor/scenes/backdrop_preview.tscn Backdrops.position from (-960,-780) to (-960,-291) and corrected stale comment block (no longer references ANCHOR_MODE_DRAG_CENTER/camera derivation context that changed post-#89). Probe kept temporarily for verification.

## Workspace config — the godot MCP server was removed (2026-10-22)

`SSTD/opencode.json` (the workspace parent, which is **not** a git repo) no longer carries an
`mcp` block. The `godot` MCP server was spawning a `godot-mcp` process per session and exposing
386 tools, which made every session's tool catalog enormous; @me asked for it to be deleted.

- `enabled: false` inside the server block is **not honoured** by this build: a service started
  after the edit still spawned `godot-mcp`, so the whole block had to go.
- Verified after removal: `opencode mcp list` shows only `opencode_console`, `pgrep -af godot-mcp`
  is empty, and the 386 godot tools are absent from the catalog.
- To re-enable, restore a `"godot"` entry under `mcp` in `SSTD/opencode.json` (command
  `npx -y @modelcontextprotocol/godot-mcp` was the previous shape). The re-enable recipe is also
  in a comment in that file.

**Durable-record gap:** `SSTD/` is not versioned, so this note and that file's comment are the only
copies. The same fact belongs in the code repo's `.opencode/AGENTS.md`; do that on its own branch
off `trunk`, not on the #119 branch.

## GROUND/WALK #122 (IN PROGRESS)
- Branch: feat/sim-ground-walk-122 (on top of trunk caa1990)
- Issue: #122 "feat(sim): ground layer + avatar walk in Simulator"
- TDD: wiki TechnicalDesign/TDD_Simulator-Ground-Walk.md (updated with gRPC/gamepad/mouse control notes), indexed in Home.md
- Wiki: master db57f9d/887356f committed and pushed
- Commits (code): 9c4d897 "feat(sim): ground walk physics + bridge + world view groundwork (#122)"
- What exists: `crates/sstd-core/src/physics.rs` (ground_profile_px/ground_y_for_column/step_walker + tests), module wired in lib.rs; bridge `walk_step` added; WorldView adds GroundLayer + Avatar nodes, sets up ground drawing script, adds movement key hook placeholder.
- Tests: `cargo test -p sstd-core` passes (143 passed, 0 failed). GDScript `test_world_view_controls.gd` passes (0 failures).
- Notes: control mapping is gRPC-based / input-mapper territory (GDD); Simulator and Play both can use same commands.

## NEXT SMALL STEP (do this first)
1. **Wire tiles into ground**: in `WorldView`, collect walkable tiles from `ScreenStore` + cached screens/grid config into `_tiles_cache` (format `{world_tile_x,world_tile_y,is_walkable}`) and pass to `GroundLayer.set_data(...)`. Also populate grid dict `{tile_w,tile_h,...}`.
2. **Add WASD/click input + walk_step call**: read WASD (and mouse click-to-move), build `WalkerInput`/`WalkerState` JSON, call `bridge_obj.walk_step(...)` if available, update `_avatar.position` from result, update `_walker_state`.
3. **Keep it tiny**: test with existing world data in Simulator; don't change parallax/camera logic. Commit after this step with checkpoint update.

## GROUND/WALK #122 (PROGRESS UPDATE)
- Code commits: 9c4d897, 5685a2d, 850b75b, fabef06
- Added: bridge `walk_step`, physics module + tests, GroundLayer/Avatar in WorldView, tiles wired from ScreenStore, WASD + click place/interact, auto-acquire bridge from Main.
- Tests: cargo test -p sstd-core ok; test_world_view_controls.gd ok.

## RUN CHECKLIST (what to expect)
- Launch editor: `$HOME/bin/godot4 --path editor`
- Go to Simulator tab. 
- Expect: parallax backdrop unchanged from previous checkpoint (no visual difference in parallax). 
- Expect: new ground layer visible (greenish rectangles) under avatar area where tiles exist; stickman avatar appears near start.
- WASD moves avatar; left/right click pass place/interact through bridge.
- If bridge not ready, avatar moves with simple fallback (no crash).

## LATEST (2026-10-27)
- Polished avatar spawn/ground snap in WorldView (drop from higher Y, snap after rebuilding tiles using bridge walk_step when available). Use grid dimensions from config for tile indexing.
- Improved bridge discovery (recursive search) and fallback ground snap.
- Rebuilt bridge; all tests green.

## RESUME POINT (2026-10-27)
Branch: feat/sim-ground-walk-122
Latest: improve bridge discovery + fallback snap (#122)
Tests: cargo test -p sstd-core ok (141); test_world_view_controls.gd ok; test_parallax_backdrop.gd ok
Next: test in Simulator (WASD/click/place/interact) and polish if needed
