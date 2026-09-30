# Project: SSTD (Sidescroll Tower Defense)

## How to Read This File

This is the **canonical, self-contained** project knowledge file for SSTD. It is wired into
opencode by `opencode.json` in this directory (`"instructions": [".opencode/AGENTS.md"]`) —
without that config entry opencode would never read this file, because `.opencode/` is
reserved for `agents/`, `commands/`, and `plugins/`, not `AGENTS.md`.

It is deliberately self-contained: opencode loads the project config by looking in the
current directory and traversing up to the nearest **git** directory, so a session launched
inside this repo does **not** see anything at the `SSTD/` parent level. Do not move content
out to a parent file or "de-duplicate" the permanent rules below against a global rules
file — a session started here must still get the full process contract.

Workspace-level orientation (the two-repo layout, branch topology) lives one level up in
`SSTD/AGENTS.md`. Session state of record lives in `docs/SESSION-CHECKPOINT.md`.

## Session Progress Checkpoint (permanent)

The user may switch sessions mid-task at any time, and may also **abandon a
session (e.g. change of model) and start a brand-new session with zero memory
of this conversation**. For multi-step work (`#51` prune optimization and any
long task), **keep `docs/SESSION-CHECKPOINT.md` continuously updated as work
proceeds** so a fresh session can resume cold:

`docs/SESSION-CHECKPOINT.md` is the **single checkpoint of record** for SSTD. Its
`## CURRENT STATE / NEXT MOVE` block is the authoritative resume point and supersedes every
other heading further down that file. Do not keep a second progress table in this file, in a
wiki page, or in `.opencode/sessions/` — a stale duplicate is what the
Always-Update-All-Documents rule exists to prevent. Per-task handoff notes in
`.opencode/sessions/` are working scratch, not the record.

- Record what shipped/committed, the exact current active step, and the next
  move after each meaningful milestone — not only at the end.
- Record any measured numbers, decisions, and commands so the successor does
  not re-derive them.
- Commit the checkpoint update together with (or immediately after) the code
  change it describes. (Rule recorded 2026-08-16.)
- **Write the checkpoint as if the next reader is a different model on a
  brand-new session that never saw this conversation**: it must be fully
  self-contained — state the objective, the committed history, the exact
  in-flight step, the next move in runnable order, and every command/finding
  needed to resume, without relying on prior chat context. A checkpoint that
  assumes the reader "knows the project" from memory is not sufficient; if a
  crash/abandonment means the in-flight step is lost, the checkpoint must be
  enough to restart that step from scratch. (Rule recorded 2026-08-17.)

## Wiki Maintenance Rule (permanent)

Whenever a **new wiki page** is added (GDD or TDD), **update the wiki Home page** (`sidescroll-towerdefense.wiki/Home.md`) in the same commit — add a row in the relevant section (Game Design / Technical Design) linking the new page. Do the same in `TODO.md` (tracked-work session table). A newly shipped wiki page with no Home.md/TODO.md entry is an oversight. (Rule recorded 2026-08-09, see #40.)

Self-check — run this from the wiki repo root; every hit is an unindexed page. `-F` is
required: the page names contain `.`, which `grep` would otherwise read as a regex wildcard.

```bash
cd ../sidescroll-towerdefense.wiki
for f in GameDesign/*.md TechnicalDesign/*.md; do
  b=$(basename "$f" .md); [ "$b" = "README" ] && continue
  grep -qF "$b" Home.md || echo "NOT in Home.md: $b"
done
```

19 pages were found unindexed on 2026-09-26 (11 TechnicalDesign + 8 `GDD_Gameplay.*`
sub-pages) and fixed in the same pass that added this check; the loop is how a future
session confirms it stayed fixed.

## TDD/GDD Documentation Location (permanent, user directive 2026-08-26)

ALL TDD and GDD documents live in the **wiki repo** (`sidescroll-towerdefense.wiki/TechnicalDesign/`), NEVER in the main repo's `docs/` directory. The main repo's `docs/` is for session-local artifacts only (PLAN-*, SESSION-CHECKPOINT.md, temporary design docs). If a TDD or GDD file is found in `docs/`, migrate it to the wiki and delete it from the main repo. (Rule recorded 2026-08-26, after migrating 9 misplaced TDD files.)

## Ticket-First Rule (permanent)

Do NOT wire up / implement anything (code changes, wiring, edits) until the
corresponding GitHub issue is created AND documented/planned in the issue body
(design, approach, scope). Feature requests and bugs alike: create the ticket,
document the plan in it, then implement — and reference the issue number in the
commit that implements it. (User directive 2026-08-17, enforced twice on the
#55 config-defaults work.)

## Always-Update-All-Documents Rule (permanent, user directive 2026-09-05)

Upon completing ANY small task (commit, fix, feature, refactor), you MUST
immediately update ALL relevant documents in the same pass — GitHub issues
(close/comment), wiki TODO tables, AGENTS.md, SESSION-CHECKPOINT, and any
other tracking docs. Do NOT defer document updates to "later" — deferred
updates are forgotten, causing stale issue states (e.g. bugs marked open
that are already closed), duplicate work, and broken traceability. The
document update IS part of the task, not a follow-up. (User directive
2026-09-05, after discovering bug #61 was closed but TODO.md still listed
it as tracked.)

## Idle-time Checkpointing (permanent, user directive 2026-09-13)

Always checkpoint and update documents — and the IDEAL time to do it is
when idling (no task in flight, waiting on the user, or between tasks).
Any pause is an opportunity to persist state: write/refresh the
SESSION-CHECKPOINT current-state block, mark wiki TODO rows, inline
findings, and push any pending doc commits. Do not sit idle when docs are
dirty or the checkpoint lags the true state — that is precisely the moment
the next crash/interrupt would erase progress.

## Issue-State Hygiene (permanent, user directive 2026-08-19)

GitHub issue states are part of the persisted status progression and MUST be
kept in sync with commits, alongside the checkpoint/wiki/AGENTS updates. This
applies to ALL issue types — features, bugs, and chores alike.

- **Close** an issue the moment its work is addressed and committed (a "DONE
  (committed ...)" comment alone is NOT enough — the issue stays open).
  - For FEATURES: the closing comment must also confirm the feature is
    implemented as stated on the wiki, citing the wiki page(s) that document
    it (e.g. `TDD_World-Editor`, `TDD_Tile-Deduplication`).
  - For BUGS: reference the commit that fixes it AND the wiki page(s)
    describing the intended/correct behavior.
  - For CHORES/DOCS: reference the commit (and wiki page if applicable).
  - Traceability invariant: every path — commit, closing comment, wiki TODO
    table — must lead (directly or indirectly) to a wiki page. Commit ->
    issue -> wiki; the closing comment is the issue->wiki link, so cite the
    wiki page in it.

## Documentation Reference Architecture (permanent, user directive 2026-08-19)

Rule of thumb for where commit hashes vs. wiki belong:

- **Commit ref-hashes live ONLY in GitHub Issues.** Any document that needs an
  association to a git ref-hash (commit hash) must anchor that association on a
  GitHub issue (issue body/comment), never embed the hash itself in the wiki.
  Example: a closing comment references the commit; the wiki never does.
- **Everything else is Wiki-based.** Documents that do not need a commit-hash
  association are grounded on wiki pages (TDD/GDD). Wiki pages cite ISSUE
  numbers, not commit hashes.
- **Wiki never contains git ref-hashes.** A commit hash in a wiki page breaks
  if the wiki moves off GitHub (no link to maintain) or history is rewritten.
  An issue number is a stable, movable, resolvable anchor; a raw hash is not.
- **"Document"** = any persisted history of text: commit messages, PR
  descriptions/comments, GitHub issues (feature/bug/chore), README, wiki pages.
- **Invariant**: scanning these documents across the GitHub repos must yield a
  complete record of WHY and WHAT for any change — wiki for design, issues for
  commit associations, and the issue link between them.
- **Reopen** it if a follow-up proves the fix incomplete (with the new
  repro/evidence in the body).
- **Keep open** when the issue is deliberately deferred in-body (e.g. #54 M2)
  or is a filed-but-unfixed bug (#61).
- Do this in the SAME pass as the code commit and doc updates, not as a
  separate "later" step. (Rule recorded 2026-08-19, after #55/#52/#53/#59/#60/
  #62 shipped but were left open.)

## Feature Request: OpenRouter gateway (issue #69, OPEN)

Recorded 2026-09-07; ticketed and designed the same day.

- **Integrate OpenRouter** so that TEXT can instruct the game's **LLM agents** to use
  text-to-image and other relevant OpenRouter models.
- Intended uses: agents generate **maps**, generate **new tilesets**, etc.
- Integration surface: user chose via the question flow (2026-09-07): gRPC is the
  single authoritative surface (**strictly gRPC**). Investigation resolved that MCP
  cannot attach *directly* to Godot 4 GDScript (no native MCP support; an MCP server
  would ride the same Rust GDExtension bridge as tonic anyway = adapter, not a new
  integration). User also directed: data analysis/geometry/validation must happen in
  pure Rust on CPU ("almost all the work done in CPU without LLM host") to be
  **token-thrifty** — the LLM never computes, it only instructs/proposes. Key model:
  **mixed** — SSTD-side HTTP client for image generation, agents keep their own text
  channel. Artifacts: **tilesets first, then maps**.
- Status: **TICKETED + DESIGNED (2026-09-07).** GitHub issue #69
  (`feat(f7): OpenRouter gateway — agent-driven content generation (tilesets then
  maps) via gRPC`), OPEN — implementation pending. Design authored BEFORE any code:
  wiki `TDD_OpenRouter-Gateway.md` (AuthoringService gRPC service on the 8-service
  agent-action model; `sstd-openrouter` HTTP client for `POST /v1/images`; pure-Rust
  validation contracts for the tile pipeline + screen/world pipeline; `openrouter.*`
  config domain population 006; `generation_log` journal; human approval gate;
  budgets as credit-strings; testing plan) + `GDD_AI-Content-Workflows.md`
  (Slice A/B designer workflows, editorial guardrails). This entry stays as the
  durable handoff lever for that implementation.

## Feature Request: depth-layered parallax background (issue #68, OPEN)

Recorded 2026-09-06; ticketed and designed the same day.

- **Parallaxing on 2D background scroll, BOTH axes (vertical + horizontal).**
- User's requirement reasoning: use **depth-based (layered) parallaxing** (multiple
  layers per depth/parallax factor), NOT the old **scanline-based** technique —
  scanline cannot handle "holes" in the BG. If a BG "hole" is needed, current-gen
  layered parallax is preferred; the scanline-era workaround (adding sprites on
  either side of a scanline to fake a hole) is explicitly the OLD technique and
  not the target. Even non-consumer GPUs handle depth-layered parallax fine today.
- Status: **TICKETED + DESIGNED (2026-09-06); RESTACKED (2026-09-30).** GitHub issue
  #68 (`feat(f6): depth-layered parallax background`), OPEN — implementation pending.
  Design authored BEFORE any code: wiki `TDD_Parallax-Background.md` + the
  `GDD_Art-Direction` Parallax Background section. This entry stays as the durable
  handoff lever for that implementation.
- **RESTACK (2026-09-30, still OPEN, no code yet).** The node contract now comes from
  the official [2D Parallax tutorial](https://docs.godotengine.org/en/stable/tutorials/2d/2d_parallax.html)
  (TDD §3.1). Findings that drove it:
  - **The tutorial ships NO downloadable assets** — verified four ways (page + raw
    `2d_parallax.rst` reference only screenshots and one `.webm`; the `tutorials/2d/img/`
    listing holds only those images; the original pull request #9587 names no asset
    source; `godot-demo-projects/2d/` has no parallax demo). **Do not re-search for
    tutorial assets.** It is the mechanism contract only.
  - **Art is self-generated**, not from an external/CC0 pack (@me's decision:
    SSTD is a deliberate LLM-co-developer exercise, so external art dependencies and
    licence review stay out unless asked). New crate `tools/gen-backdrop`, wrapping
    value-noise lattice for seamlessness; first pass is a neutral, palette-agnostic
    placeholder whose job is to prove the mechanism.
  - **Model 3 -> 5 layers + foreground:** `Sky (0.10,0.08)` / `HighClouds (0.20,0.17)`
    / `LowClouds (0.30,0.25)` / `Hills (0.50,0.42)` / `Forest (0.70,0.60)`, x factors
    the tutorial's own published values, y ~0.85x for both-axis motion, plus a
    `Foreground (1.30,1.15)` plane drawn IN FRONT of the tile layer. Six of the
    7-layer cap. The foreground is excluded from `biome_backdrop_layers` because its
    factor exceeds that schema's `[0,1]` CHECK by design.
  - **Screen geometry is derived:** 60x33 tiles @ 32px = a 1920x1056 screen
    (`project.godot` viewport 1920x1080), so layers author 1:1 at 1920 wide and nothing
    scales. Vertical coverage is overscan + a 240px camera-y clamp, NOT a vertical
    `repeat_size` (which leaves empty blocks above/below a horizontal-only stack).
- **Two defects a GREEN probe could not see** (both still unfixed, code phase pending):
  (1) `main.tscn` sets `BackdropStrip` to `visible = false`, so the parallax has never
  rendered in the app, while `probe_parallax_scroll.gd` exits 0 because it reads static
  properties and never instantiates the scene — the concrete cause behind the old
  "runtime rendering unverified" note; (2) `backdrop_preview.tscn` centers every
  `Sprite2D` on the `(0,0)` crossing with no `repeat_size`, which is exactly the
  positioning/sizing mistake the tutorial documents.
- **Retiring:** the 10 tracked sliced PNGs (~13 MB) in `assets/backdrop_layers/` (root
  copy is read by NO code) and `editor/assets/backdrop_layers/`, including the
  `*_strip_x5.png` collision strips. `tools/slice-layers` is KEPT as a documented
  one-off (its MP4 inputs are gitignored/local-only, so a clean clone can never re-run
  it — that is why it is not the normal path).
- **Plan + evidence:** `docs/PLAN-2026-09-30-parallax-tutorial-stack.md` (code repo),
  `docs/SESSION-CHECKPOINT.md` CURRENT STATE, `tools/slice-layers/README.md`, wiki
  `TODO.md` TS71, GDScript test `test_parallax_backdrop.gd` to replace the probe.

## gRPC Integration

**Surface inventory (2026-09-07):** implemented = 1 proto file (`crates/sstd-grpc/proto/editor.proto`), 1 service (`EditorService`: SwitchTab + CaptureScreenshot), 4 messages; designed = 12 services / ~113 RPC pairs across the wiki TDDs (~109 net unique). **Single schema source + single generated lib (enforced, #72):** `crates/sstd-grpc/proto/` is the ONLY schema dir; `sstd-grpc/build.rs` compiles every `*.proto` there via glob (new proto auto-builds, `rerun-if-changed` covers edits); every new proto ships 4 pieces in ONE commit — the `.proto`, its `include_proto!` in `sstd-grpc/src/lib.rs`, a row in `proto/README.md`, and the `TDD_gRPC-Service-Index` purpose text. Consumers MUST depend on `sstd-grpc` for proto/gRPC types — never hand-roll wire structs or re-generate `.rs` elsewhere (the JSON GDExtension `#[func]` bridge is a separate non-protobuf contract, not a second schema). gRPC != protobuf: the Service Contract is the deepest layer and already runs over a non-protobuf transport (`#[func]` bridge + `EditorCommand` mpsc). Services are built only when a consumer needs them (gRPC is an AI accommodation cost), not the full designed surface.

Three gRPC service contexts are designed, each on a separate port (localhost-only by
default, no auth). Only the Editor context has shipped — see the per-context status.

### 1. Editor gRPC — port 50051 (SHIPPED)

Bridges external editors (custom tile editors, Aseprite→SSTD pipelines, CI scripts) to the editor backend. All operations go through the shared `EditorState` — same validation, same CCMV (Config-Control-Model-View) constraints as the Godot editor UI.

- Import/validate terrain types, entity defs, screen files
- Query grid config, dimension compatibility
- List / export current editor state
- Follows the same bridge API shape as `SstdBridge` (terrain → import_terrain_types, etc.)

### 2. Simulator gRPC — port *TBD* (e.g. 50052) — DESIGNED, NOT BUILT

Controls the deterministic simulator for batch evaluation, AI training, headless testing.

- Step simulation N ticks
- Inject entity at tile position
- Query entity state, world state, combat log
- No display/rendering — pure data in/out

### 3. Game gRPC — port *TBD* (e.g. 50053) — DESIGNED, NOT BUILT

Runtime game interaction — multiplayer coordination, external AI opponents, spectator clients.

- Query game state
- Submit action (place entity, trigger ability, etc.)
- Stream game events

### Shared architecture

- The Editor server runs on a background tokio thread inside the Godot editor process
  (`editor/scripts/main.gd` holds `const GRPC_PORT := 50051`).
- Each context has its own proto service definition, combined in a shared `sstd-grpc` crate.
- Editor gRPC shares `Arc<RwLock<EditorState>>` with the Godot bridge.
- Simulator/game gRPC have their own state (world state, sim state).
- No auth/API keys — localhost-only by default; expose to network only via explicit config.
- **There is no `sstd-headless` binary.** Earlier notes in this file claimed one existed and
  could start all three servers without the Godot UI; it was never built and is not a
  workspace member (see `Cargo.toml` `members`). A headless server for CI / batch sim /
  AI training remains unbuilt design — tracked by the wiki `TDD_Simulator-Service-Contract`
  PLACEHOLDER page. Do not write code or docs assuming it exists.

### CCMV Constraint

All gRPC services must honor the **Config-Control-Model-View** (CCMV) architecture: every operation is validated against `sstd_config.sqlite3` config constants first, then processed through the Rust data model, and finally returned as structured data (never as raw display state). External tools bypassing CCMV (e.g., writing config values directly without going through the population system) must be rejected at the gRPC layer.

## Over-Engineering Audit Rule (2026-09-10)

The whole-repo over-engineering audit lives in the wiki
`TechnicalDesign/Engineering-Audit.md` (issue #73). It is a **ledger, not a
one-shot**: future sessions must run the page's 4 Delta-check greps and append a
delta row instead of re-auditing the whole tree. Baseline findings are ranked
there (biggest cut first); nothing was deleted. Any accepted/rejected cut is
recorded inline on that page.

## LUCK Stat

SSTD is strategic / no-RNG, so LUCK is a **deterministic, bounded modifier** — never a dice roll.

- **Deterministic, not random** (Decision, 2026-08-07): LuckBot companions apply a predictable,
  capped boost; no 2D20, no flat random, no bell-curve rolls for luck. Rolls stay out of the
  placement-strategy loop.
- **Behavior** (implemented @ `crates/sstd-core/src/luckbot.rs`):
  - `final_damage × (1 + %LUCK)` — additive-on-base, capped at `luck.bonus_cap` (default +25%)
  - Crit is a fixed cadence (`crit_interval`: every N-th hit), max luck → crit every hit
  - Drops use a guaranteed `rarity_floor` (luck raises the minimum tier), not weighted rolls
  - Neutral luck `0` → no effect (`×1.0`), so investing in a LuckBot is always a deliberate choice
  - All constants config-driven (`luck.*` keys via `populate_003`)
- Supersedes the earlier "Phase 2 Consideration" ±LUCK% to damage note; the deterministic stance is kept, now implemented as a bounded modifier with explicit crit cadence + rarity floor.

### Replay & seeded rolls

- **Persist the SEED, not rolls.** One `u64` seeds `sstd-core`'s dependency-free splitmix64
  (`seeded_roll(seed, counter)`); everything else (tiers, crits, damage) is derived data.
- `RollLog` (luckbot.rs) folds every `roll()` into a rotating `checksum()`; **same seed + same
  roll count ⇒ identical checksum** is the replay/CI invariant. Any call-order change, missing
  roll, PRNG swap, or config change breaks it.
- Rule: never persist a rolled tier directly as a replay source of truth; recompute it from
  the seed on replay.

## Aura & Movement Geometry

- AURA / radius and patrol/follow movement are core geometry (`within_aura`); actual
  ally-selection and target-following remain the engine's job.

## Pathfinding

- **No A*** — SSTD is a side-scroller; corridors are linear with occasional 2-3 way forks
- At fork joints, AI picks by priority rule (nearest enemy, weakest enemy, waypoint-route priority)
- Linear corridor segments: entities just walk forward, no pathfinding needed
- Waypoint-route system: entities follow waypoint chains; at junctions, fork-priority AI decides which branch to take

## Naming Conventions

### Coordinate & Unit — explicit suffixes required everywhere

- `world_tile_x/y` — tile grid positions
- `world_pixel_x/y` — continuous pixel positions (used for entity positions, physics)
- `world_pixel_offset_x/y` — sub-tile pixel offset (0..63)
- `local_tile_x/y` — screen-local tile coordinates (0..max-1)
- `screen_id` / `screen_id_y` — screen index within a corridor
- `*_pps` — velocity in pixels per second (e.g. `speed_pps: 640`)
- `*_px` — distance/size in pixels (e.g. `offset_px`, `width_px`)
- `*_tiles` — measurement in tile units (e.g. `radius_tiles`, `width_tiles`, `height_tiles`)

Schema exception: self-contained context (e.g. `tiles.x`, `tiles.y` in the `tiles` table)
needs no suffix.

Physics & timing:

- All velocity/timing values are authored **per-second** (pps, seconds) and converted to
  per-tick once at load via the `time_scale` multiplier.
- `time_scale` (GM Config, default `1.0`) scales all systems uniformly — movement,
  animations, cooldowns, DOTs stay in sync.
- Conversion: `speed_per_tick = speed_pps * time_scale / tick_rate`

Range/radius stays in tiles: `radius_tiles` is a grid query (spatial index AABB over tile
cells), not physics, so it needs no conversion to pixels. See `TDD_Tiles` for rationale.

### SQL column names

All SQL column names follow `snake_case` (code-enforced). Fixed `descriptionID` → `description_id` across wiki 2026-08-01 (TDD_Localization-System, TDD_Skill-Dependency-Graph, TDD_Skill-Designer-Templating).

### Config keys

All config keys follow `domain.snake_case` format (code-enforced). The wiki's `TDD_GM-Config.md` had bare `camelCase` keys in presets/cheatsheet (e.g. `baseDamage`, `mineSpawnRate`). Fixed 2026-08-01:
- Preset tables: `difficultyMultiplier` → `difficulty.multiplier`, `enemyHpMultiplier` → `difficulty.enemy_hp_multiplier`, etc.
- SQL examples: per-domain tables (`inventory_config`) → single `config` table per code
- Cheatsheet: all keys prefixed with domain + snake_cased
- Attribute keys in `TDD_Entity-Instance-System` (e.g. `attackRange`, `elementAffinity`) remain camelCase — those are entity attribute keys, not config keys.

## Status Badges

As of 2026-08-01 audit, most TDD/GDD pages describe planned systems with no code implementation (~80% design target). Recommended badge format for page headers: `**Status:** ✅ Implemented` / `⬜ Design Target` / `🔧 In Progress`. Not yet applied.

## Element Enums

Code has 6 (`Physical`/`Fire`/`Ice`/`Lightning`/`Holy`/`Dark`). Wiki documents 14 elements total (6 implemented ✅ + 8 planned ⬜) with 2 extra combo-only elements (Wind, Oil). Wiki header clarified 2026-08-01: dropped "12-element" claim, added explicit "8 planned" count.

## Rule of Thumb: Config-Based Constants

All tunable gameplay constants must live in `sstd_config.sqlite3` via the population system (`config.rs` POPULATIONS / `populate_NNN`). Code structs expose a `Default` ONLY as a fallback for an unseeded store — never as the runtime source of truth. The game always loads from `ConfigStore`. New systems: write the population script + keys + a `ConfigStore::<x>_config()` loader + loader test alongside the logic.

## Config Population Versioning

- Schema version bumps happen **only** when the editor has a "save configs" feature that can persist changes.
- Until then, default values can be changed in-place in `populate_001` (or a subsequent script) without bumping the version. Fresh databases get the latest defaults; existing databases retain whatever was seeded.
- Once the editor can save config overrides, any new population script must bump the version and old scripts must not be modified retroactively.

## TileSet (Multi-Tile Grouping) System

- `TileSetGrouping` (`scripts/resources/tile_set_grouping.gd`) is the primary authoring format — a `.tres` Resource with `key`, `display_name`, `width_tiles`, `height_tiles`, `tiles: Array`, `tags`, `source_image`.
- Each entry in `tiles` is a Dictionary with `local_x`, `local_y`, `terrain_key`, `elevation_tiles`, `z_depth`, `sub_tile_mask`, `source_col`, `source_row`.
- `terrain_key` doubles as the individual tile PNG filename: `{terrain_key}_32x32.png` in `editor/assets/tiles/`.
- `source_image` is the original spritesheet path; `source_col`/`source_row` are the grid position within it. Currently unused at render time (PNG extraction is done offline), but stored for provenance.
- `.tres` files in `editor/assets/tilesets/` are auto-discovered on Placement tab switch.
- The `validate()` method checks for duplicate positions, empty keys, and bounds.
- `resolve(base_x, base_y)` expands the group into flat terrain tiles with metadata.

### tileset_2-2.png (30° Grass Slope)
- Located at `editor/assets/samples/tileset_2-2.png`.
- Parameters (from Godot TileSet auto-detect): margins=(104,65), texture_region_size=(126,126), separation=(1,1). Pitch = 127px.
- Grid: 4 columns × 2 rows = 8 tiles, all with content.
- Each tile is 126×126 px in source, extracted and scaled to 32×32.
- Extraction tool was at `tools/extract-tiles/` (deleted after use). The Rust code used `image::imageops::resize(..., 32, 32, Nearest)`.
- Naming convention: `slope_30deg_{col}_{row}_32x32.png`.

## Tile Art Workflow

- Tile/entity pixel art is stored as **PNG files** on disk (`assets/tiles/{key}_{W}x{H}.png`, `assets/entities/{key}_{W}x{H}.png`), not in JSON/Dict.
- The native editor includes a built-in **PixelCanvas** for quick paint/touch-up and a **3×3 tiled preview** for seam checking.
- **Hybrid workflow**: PNGs can be created in external tools (Godot IDE, Aseprite, Photoshop) and imported via the "Import PNG" button. The two-way PNG roundtrip means no lock-in.
- PNG resolution matches the config tile size (default 32×32). External images are resampled with nearest-neighbor on import.
- **Parallax backdrop layers** (issue #68): **retired 2026-09-30.** The old path was `tools/slice-layers` (issue #74), which recovered 3 RGBA layer PNGs from a rendered parallax video by measuring horizontal displacement ("flow, not ML"). Its input MP4s are gitignored and local-only, so a clean clone can never re-run it. The tool is kept with `tools/slice-layers/README.md` documenting that caveat, but it is **not** the path to layers: the replacement is a planned `tools/gen-backdrop` crate that generates the 5-layer stack in-repo with a wrapping value-noise lattice (seamless under `repeat_size`). No external/CC0 art pack is used. See the #68 restack entry above and `docs/PLAN-2026-09-30-parallax-tutorial-stack.md`.
- **Preview MP4s are local-only** (user directive 2026-09-13): source renders `assets/samples/preview-with-parallax*.mp4` and the 1 fps frame dir are `git rm --cached`'d + gitignored — they will NOT go to the repo even with LFS. The derived `assets/backdrop_layers/*.png` are the committed artifact; regeneration needs the local files (replay command on TDD_Parallax-Background §4.1).

## Editor Tooling & Validation Commands

- Godot binary: `$HOME/bin/godot4` — **4.7.2.stable.mono** as of 2026-09-29 09:24
  (the symlink was repointed that day). `$HOME/bin/godot4.4` is still installed
  (4.4.1.stable) if a 4.4 comparison is needed. **Every baseline number recorded before
  2026-09-29 was measured on 4.4.1**, so they are stale for the canonical command — say
  which runtime a number came from, or re-measure.
- **Editor GDScript regression tests** (must pass after any editor change):
  ```bash
  $HOME/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd
  ```
  Expect `=== done, failures=0 ===`. Clean up any `editor/world.json` the tests write (gitignored).
- **Syntax check** a single script:
  ```bash
  $HOME/bin/godot4 --headless --path editor --check-only --script res://scripts/<file>.gd
  ```
- **Headless app boot** (surfaces node/scene errors): `$HOME/bin/godot4 --headless --path editor --quit-after 120`.
- After adding a new `class_name` global (e.g. `ScreenStore`, `ScreenMinimap`), run `godot4 --headless --path editor --import` once so other scripts resolve the type.
- `main.tscn` embeds the map/placement editors **inline** — the standalone `map_editor.tscn`/`placement_editor.tscn` are not what runs at runtime; edit `main.tscn` too.

## Editor Persistence Paths (#54 M1, 2026-08-18)

All runtime writes go under `user://data` (never `res://`, which is read-only in a packaged build):

| Path | Value | SQLite-configurable? |
|---|---|---|
| ConfigStore DB | `user://data/config/sstd_config.sqlite3` | No |
| Boot world pointer | `defaults.world_json_path` = `user://data/world.json` | Yes |
| Save-dialog file name | `defaults.world_file_name` = `world.zip` | Yes |
| Save/load dialog start dir | `defaults.file_dialog_dir` = `""` | Yes |
| Tile/stamp cache | `user://data/tiles` | No |

- `main.gd`: `USER_DATA_DIR := "user://data"`, `DEFAULT_WORLD_PATH := "user://data/world.json"`, `user_data_dir()` (globalized + mkdir). `_init_config_db` writes the config DB under `user://data/config/`.
- `map_editor.gd`: `_tiles_write_dir()` = `user://data/tiles`; `get_tile_image`/`_load_stamp_catalog` read user dir first then built-in `res://` fallback; `_register_tile_image` persists PNGs to the user dir.
- Data base dir override: deferred, issue #57 (CLI-arg-only, future).
- Test hygiene: world-restore tests persist the user:// PNG, so cleanups MUST wipe both `user://data/tiles` and `res://assets/tiles` copies (helper `_wipe_tile_artifact(key)` in `tests/test_screen_store.gd`).

## Dirty-State Save Prompt (#60, 2026-08-18)

- `main.gd` owns `is_dirty`. `mark_dirty()`/`clear_dirty()`; `_confirm_save_dirty()`
  returns 2=Save / 1=Discard / 0=Cancel (awaited modal; non-dirty skips -> 1);
  `_confirm_dirty_or_save()` is the guard for interactive loads.
- **Godot gotcha**: an `await`ed handler in `_notification(NOTIFICATION_WM_CLOSE_REQUEST)`
  does NOT block the engine — `SceneTree.auto_accept_quit` defaults to `true`, so the
  app quits right after the notification fires, even while the dialog awaits. Must set
  `get_tree().auto_accept_quit = false` in `_ready()` and call `get_tree().quit()`
  explicitly from the handler (dirty: after the prompt; clean: immediately).
- **Godot gotcha #2**: you CANNOT `await` inside a `_notification()` handler at all —
  Godot calls the handler and discards the returned coroutine, so code after the
  `await` never runs (the quit prompt "returned" but quit never fired). Use
  callback-driven dialogs there (connect button signals), not `await`.
- **World formats**: `.zip` = whole world package (primary, since #43). `.json` in the
  load/save dialogs = a single legacy screen. `user://data/world.json` = boot pointer
  file recording the active `.zip` path — never picked in a dialog. Dialog filter
  default follows the current world format via `_apply_world_default_filter` (#62):
  `.zip` when `world_package_path` is set, `.json` in legacy manifest mode.
- Dirty is set by map paint (`_paint_tile`), placement entity place/remove/clear,
  and cleared by `_save_world()` in either editor. Guards: quit
  (NOTIFICATION_WM_CLOSE_REQUEST), import dialogs (via `_on_import_file_guarded`),
  `new_project`/`open_project`.
- Test seam: tests call load/import methods directly (bypass the guarded dialog
  path) and never reach the modal; `_test_dirty_prompt` covers the flag mechanics.

## Prune Scan Hot Loops (#51/#53/#59)

Native bridge (`sstd-editor-bridge`) owns the scan hot loops; GDScript is the
oracle. Current front-end: `scan_signatures`+`scan_projections` (f64),
`scan_gate_pairs`, `build_variants`, `scan_exact_matches`, `a3_neighbor_hashes`,
`flip_of_bytes`, and `scan_canonical_coarse` (N*256, #59). `_a3_discover` slices
per-rep canon from `_canonical_flat` when the bridge produced it. Real catalog
(3346 tiles): full `_build_prune_plan` 5030 ms, dup_keys=1328.

## Screen Registry (ScreenStore)

- `editor/scripts/screen_store.gd` is the on-disk registry (`world.json`): maps `screen_id → (x, y) → file`, assigns `next_id`, and provides a per-screen cache.
- An **unregistered screen id is represented by position `(-1, -1)`** (sentinel) — never fall back to `Vector2i.ZERO`, which collides with the legitimate screen at `(0,0)` and caused test regressions.
- Guard screen ops with `_is_current_placed()` (`_screen_pos.x >= 0 and _store.is_occupied(...)`); cache/restore only for placed screens.
- On save/import of an unplaced screen, assign `_store.next_free_position()` before registering.
- Map editor writes `placed_entities: []`; `_merge_placed_entities()` re-attaches the cached entity array on save/clone so entity work isn't wiped.
- World coordinates: `world_x = screen_pos.x * grid_w + local_x` (wiki [TDD_Map-World § World Coordinates](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TechnicalDesign/TDD_Map-World)).

## Tech Stack

- **Game/Editor Engine**: Godot 4 with Rust GDExtension (gdext)
- **Rust handles**: data persistence (SQLite3 editor data files), deterministic physics
  simulation (kinematic engine), entity validation
- **GDScript handles**: UI layout, input, node management, visual rendering in the editor
- **Data format for editor**: JSON files initially (prototyping speed), SQLite3 for production
- **Editor is a standalone tool** — not embedded in the game binary. Game and editor share
  Rust crates (physics, entity defs, terrain types) via a common `sstd-core` library.

## Build & Test Commands

There is **no CI and no `rustfmt.toml` / `clippy.toml`** in this repo — lint and format
checks are whatever you invoke by hand. Nothing gates a push.

```bash
# Rust (workspace)
cargo build -p sstd-core                                   # pure Rust, no Godot dep
cargo test  -p sstd-core                                   # the Rust gate
cargo build                                                 # whole workspace
scripts/build-bridge.sh                                    # GDExtension bridge -> editor/rust/

# GDScript regression suites (all four must pass after any editor change)
$HOME/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_image_to_map.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_terrain_brush.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_override_merge.gd
```

Gate on the **exit code**, not the printed `failures=0` line — see
[Test Harness Conventions](#test-harness-conventions).

## Alive-in-Field Canon (MEMO, #39)

- **Nothing organic in the field** is absolute: village NPCs (incl. ゲママ) never walk the
  dungeon/corridor. The only organic field presence is the hero/魔王 and the **冒険者
  exception** — and 冒険者 fight surrounded by their own bot retinue that takes the
  sacrificial hits; the organic hero only risks life in rare moments.
- **Gemama = enchant-drone proxy.** She programs an **enchant drone** at her forge; the
  drone patrols the corridor and emits the stacking-weapon-buff aura. No more
  "Walk-By Enchantment" with her on the field.
- **"Apprentice" is banned prose** (and as an identifier). Support units = **support
  bots / drones** (animated 魔石 objects, no life, HP = structural integrity). Code
  identifiers: `repairBotCount` / `repair_bot_count`. Config keys stay `repair.<domain>.*`.
- **Training Center = programming/training line for bots**, NOT an organic-staff academy.
  It derives bot role levels from supporting-sponsor characters (university model, parallel
  disciplines).
- The old `GDD_Apprentice-Repair` page was renamed -> **GDD_Repair-Bots** (both wiki and
  code-repo doc mirrors must stay in sync).
- Mirror files live in the wiki repo (`sidescroll-towerdefense.wiki`, branch `master`) and
  the code repo (`sidescroll-towerdefense`, branch `trunk`, `docs/`).

## Entity Scale & Height — Change Checklist (MEMO)

When **entity `width_tiles` / `height_tiles` values change**, all of the following must be
updated in sync — they drift independently and are easy to miss:

1. `editor/scripts/placement_editor.gd` — `_load_defaults()` fallback palette (`arrow_tower`,
   `ballista`, `catapult`, `stone_golem`, etc.).
2. `editor/scripts/entity_editor.gd` — `_add_defaults()` full-def palette (contains more
   stats: `max_hp`, `attack_power`, etc.).
3. `GDD_Gameplay.Bots-Drones.md` (in `sidescroll-towerdefense.wiki` repo, `GameDesign/`) —
   `## Scale & Height Reference` table + `> **Rationale**` note.
4. Re-render the height chart asset: the wiki embeds the PNG
   (`GameDesign/assets/scale_height_chart.png`) not the SVG (SVG renders unreliably on the
   GitHub wiki). Regenerate via:
   `rsvg-convert -x 2 -y 2 GameDesign/assets/scale_height_chart.svg -o GameDesign/assets/scale_height_chart.png`
   and commit PNG + SVG together.
5. Grid pixel size / screen tile-count changes (NOT entity heights) require updating the
   `SQLite3` config DB `editor/config/sstd_config.sqlite3` `config` table keys:
   `grid.tile_width_in_pixels`, `grid.tile_height_in_pixels`,
   `grid.max_tiles_per_screen_x`, `grid.max_tiles_per_screen_y`. Entity heights are **not**
   stored in SQLite.

Verify: `cargo test -p sstd-core storage` (entity_defs JSON round-trip) and headless export
via `scripts/build.sh`.

## Image-Stamp Importer (MEMO)

### What it does

`editor/scripts/map_editor.gd` **Stamp Image...** button: imports any-size PNG, the user
drags on the grid to pick the origin cell (defines slicing alignment), commit slices it into
32x32 cells, dedups, and paints each cell as `_tiles[x,y] = tileID`.

### Storage model (do NOT change lightly)

- **Screen JSON stays a flat sparse list**: `ScreenFile.tiles: Vec<ScreenTileEntry>` each
  carrying its own `{x, y, terrain}` (`crates/sstd-core/src/storage.rs`). It is NOT
  `tiles[x][y] = tileID`. Local position is stored; world position is computed at runtime
  (`world = screen_pos * grid + local`). The validator rejects duplicate `(x,y)`.
- **tileID->location heritage** = new optional `ScreenFile.stamp_maps: Vec<StampMap>`
  (Rust-validated via `StampMap::validate`): each `StampMap` has `stamp_key`,
  `grid_cols/rows`, `cell_size_px`, and
  `cells: [{tile_id, local_x, local_y, src_col, src_row, flip_h, flip_v}]`. A missing key
  defaults to an empty vec — old saves still parse.
- **Dedup catalog is GLOBAL + PERSISTED**: `_stamp_catalog` is seeded at `_ready()` from
  `assets/tiles/stamp_*_32x32.png`; NOT cleared per import (cross-stamp dedup). `_stamp_seq`
  resumes from the highest existing `stamp_N`.
- Each unique tile = one cropped PNG at `assets/tiles/stamp_N_32x32.png`, registered in the
  terrain palette; flipped duplicates collapse to the same PNG with per-cell `flip_h`/`flip_v`
  in `tile_data`.
- The composition brush is a runtime-only `TileSetGrouping` appended to
  `_tile_set_groupings` (local_x/local_y + terrain + flips); NOT persisted as `.tres`.

### Shared tileID concept (user requirement)

The concept: **position (x,y) has the value of the tile ID; the ID is shared across
placements; each placement may be X- and/or Y-flipped.** `tile_data.flip_h`/`flip_v` (both
supported; 180 deg = both) carry the per-cell flip flags; the same `stamp_N` key appears at
many cells.

### Verification

- Rust: `cargo test -p sstd-core` (110 tests, incl. `test_screen_file_json_roundtrip` with a
  `stamp_maps` entry).
- GDScript: `$HOME/bin/godot4 --headless --check-only --script scripts/map_editor.gd --quit`.

## World Files & Override Deltas (MEMO, #79)

### Which world file the editor actually loads

- Boot uses **`user://data/world.json`**, set by `DEFAULT_WORLD_PATH` in
  `editor/scripts/main.gd`. It does **not** read `res://world.json`.
- `user://` resolves to `~/.local/share/godot/app_userdata/SSTD World Editor/`. On this
  host that file is an **empty legacy manifest**, `{"screens": {}, "version": "0.2.0"}` —
  zero screens, no `.world` key. So the Terrain Editor loads at framework defaults, which is
  normal, not a fault.
- `editor/world.json` is gitignored and points at a dead `/tmp` path. **Ignore it** —
  nothing reads it.
- `editor/world.zip` is gitignored and untracked. It is local state, gets replaced
  wholesale, and must never be treated as a source of truth or a test fixture.

### A missing `terrain_overrides.json` is NOT data loss

`world_archive.gd:117` writes `terrain_overrides.json` **only when the delta is non-empty**,
and the read at `:181-184` is symmetric. That is correct delta semantics: a world matching
the framework exactly has nothing to record. A world with no overrides file is the **normal**
representation of a world at framework defaults.

This was misread as a data-loss bug and filed as #79. The overrides had belonged to a
1.26 MB / ~90-tile world that had been replaced by a different 3.8 KB / 2-tile world — two
different worlds, not one damaged world. Before claiming loss, check whether the world
actually diverged from `default_package/` in the first place.

- The pre-#76 world backup sits under `/tmp` and does **not** survive a reboot. The user
  decided on 2026-09-25 to **skip** it: do not restore it, do not chase it, and do not
  raise it again unless asked.

### `entity_overrides` is a dead channel

`world_archive.gd:18` defines `ENTITY_OVERRIDE_DIR`; `:113-116` writes a loop over an
always-empty dict; `:176-178` populates a load-result field **nothing reads**; the only
producer is a stub returning `{}` at `placement_editor.gd:598`. `world_entity_defs` is the
channel that is actually live. Removing a `save_world` positional parameter is a **breaking
contract change**, so it needs its own ticket — do not bundle it with a neighbouring fix.

## Test Harness Conventions (MEMO, #79/#80)

- Gate on the **exit code**, not the printed `failures=0` line. All four suites gate on exit
  code; verified 2026-09-26 at `test_screen_store.gd:51`, `test_image_to_map.gd:63`,
  `test_terrain_brush.gd:70`, `test_override_merge.gd:243` (the counter is spelled
  `_failures` in two of them and `failures` in the other two — the shape is what matters:
  `quit(0 if <counter> == 0 else 1)`). The exit code is authoritative — see #80, where an
  inverted ternary made a *passing* suite exit 1 and survived review because failing runs
  also exit non-zero.
- Write temp files to **`user://`**, not a hardcoded `/tmp` path. `OS.get_temp_dir()` is
  `/tmp`; a hardcoded deeper path is machine-specific and will not port.
- **Indentation is mixed across these files**: `editor/scripts/entity_editor.gd` uses
  **spaces**, `editor/scripts/terrain_editor.gd` uses **tabs**. Inserting a line with the
  wrong style is a **parse error**, not a behavioural mutation — this invalidated a whole
  experiment during #79. Grep the file's real indentation before injecting anything.
- `test_screen_store.gd` (86 `await` sites) has **no** completion tracking, so it is
  exposed to the silent-truncation failure mode. Tracked as #81, unproven — do not "fix"
  it blind.
- **KNOWN RED on 4.7.2 (issue #85), green on 4.4.1:** `test_screen_store.gd:947`
  `world-shared tile stored exactly once` fails because it counts every zip entry matching
  `begins_with("tiles/")`, and 4.7's `ZIPPacker` now emits a `tiles/` **directory entry**
  beside the two PNGs — count 3, not 2. `screens/` gained one too. The behaviour under
  test is correct on both runtimes (4 references across 2 screens -> exactly 2 stored PNGs;
  `load_world` returns both), so this is a **test bug surfaced by the toolchain change**,
  not a product regression. Do not read this red as a regression from whatever you just
  changed, and do not "fix" it by loosening an assertion that is not the one at fault.
- Assertions that count **zip entries** are coupled to `ZIPPacker`'s directory-entry
  behaviour, which changed between 4.4 and 4.7. Prefer counting file entries (skip names
  ending in `/`), and mutation-verify any such count: the current form cannot distinguish
  "2 PNGs" from "2 PNGs + 1 directory entry".
- Gate: `cargo test -p sstd-core` (110) plus `test_screen_store`, `test_image_to_map`,
  `test_terrain_brush`, `test_override_merge`, all under
  `$HOME/bin/godot4 --headless --path editor --script res://tests/<name>.gd`.

## Provider Switching

The user switches between OpenCode and OpenRouter providers for LLM assistance. These
providers differ significantly in how input gets processed, and the user gets blocked at
times. **Always persist local data changes to disk immediately** — do not rely solely on
in-memory state or uncommitted git history. Write edits to files as you go. Commit is
separate from persistence; local file writes are the source of truth across provider
switches.

## Python is banned on this machine

Never use Python for any purpose — not scripts, one-liners, data inspection, or JSON
parsing. Use Rust, `jq`, or GDScript instead.
