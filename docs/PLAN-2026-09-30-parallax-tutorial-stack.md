# PLAN: #68 parallax restack on the official Parallax2D tutorial

**Date:** 2026-09-30
**Issue:** [#68](https://github.com/HidekiAI/sidescroll-towerdefense/issues/68) (OPEN)
**Design of record:** wiki [TDD_Parallax-Background](../../sidescroll-towerdefense.wiki/TechnicalDesign/TDD_Parallax-Background.md) §2, §3.1, §3.2, §3.3, §4.1, §4.1.1, §6 and [GDD_Art-Direction](../../sidescroll-towerdefense.wiki/GameDesign/GDD_Art-Direction.md#parallax-background)
**Branch:** `feat/parallax-tutorial-stack` (code repo), `docs/parallax-tutorial-model` (wiki)
**Status at time of writing:** research and design complete, **no code written yet**

This document is the session-local plan. The TDD is the design of record; where the two
disagree, the TDD wins and this file is stale.

---

## 1. Objective

Replace the video-sliced parallax backdrop with a self-generated, tutorial-conformant
one, make the parallax actually visible in the running app, and prove the both-axis
behaviour with a test that cannot pass for the wrong reason.

---

## 2. Findings (the reason for the restack)

### 2.1 The tutorial ships no assets

The [2D Parallax tutorial](https://docs.godotengine.org/en/stable/tutorials/2d/2d_parallax.html)
cannot be used as an art source. Verified four ways on 2026-09-30:

1. The rendered page and the raw `tutorials/2d/2d_parallax.rst` reference only
   `img/2d_parallax_*.webp` screenshots and one `.webm` video. No zip, no sample
   project, no asset section.
2. The `tutorials/2d/img/` listing in `godot-docs` contains exactly those 15 images.
3. The page's original pull request (#9587, `Add 2D Parallax documentation page`) names
   no asset source.
4. `godotengine/godot-demo-projects` `2d/` has no parallax demo.

The page was added 2024-07-08; no revision has ever carried an asset bundle. Conclusion:
the tutorial is the **node/mechanism contract** (which is genuinely valuable, since its
pitfalls are documented failure modes), and the art is produced in-repo.

**Owner decision (2026-09-30):** generate the art. Do not use a CC0 or third-party
asset pack. Rationale: SSTD is a deliberate exercise in working with an LLM as the only
co-developer, and pulling in external art introduces dependencies and licence review.
Revisit only if @me asks for it.

### 2.2 The parallax is invisible in the running app, and a GREEN probe hid it

`editor/scenes/main.tscn:809` sets `BackdropStrip` to `visible = false`.
`editor/tests/probe_parallax_scroll.gd` reports GREEN and exits 0 (re-verified
2026-09-30) because it reads static node properties (`camera.position.x`,
`near.scroll_scale.x`) and never checks whether the strip is rendered. This is the
concrete cause behind the checkpoint's long-standing "runtime rendering unverified"
note.

Lesson applied to the new test: assert **visibility on the instantiated scene tree**,
and make every assertion falsifiable by mutation.

### 2.3 The current scene violates the tutorial's own positioning and sizing rules

`editor/scenes/backdrop_preview.tscn` parents plain `Sprite2D` children to each
`Parallax2D` with no `centered = false` and sets no `repeat_size`. The tutorial's
*Poor positioning* section calls centering textures on the `(0,0)` crossing the mistake
that leaves the infinite-repeat canvas only partly covered, and *Poor sizing* is the
pitfall the page opens with. This is a rebuild against the documented contract, not a
retune.

### 2.4 Screen geometry is derived, not chosen

60 x 33 tiles at 32 px = a **1920 x 1056** gameplay screen, and `project.godot` sets
`window/viewport_width/height = 1920/1080`. Authoring layers 1:1 at 1920 wide means no
scaling and no blur, so the tutorial's two escape hatches (scale the node, scale the
children) are unnecessary. Vertical coverage is overscan plus a clamp, not a vertical
`repeat_size` (a vertical repeat leaves empty blocks above and below a horizontal-only
stack).

### 2.5 Asset estate to remove

| What | Where | Why it goes |
|---|---|---|
| `layer_0/1/2.png`, `displacement.png` | `assets/backdrop_layers/` | sliced from a gitignored local-only MP4; **read by no code** (grep-verified 2026-09-30: the only non-doc hit outside `editor/` is `tools/slice-layers/src/main.rs:43`, which is its default *output* dir, not a read) |
| same 3 PNGs + 3 `.import` | `editor/assets/backdrop_layers/` | the engine-loaded copies; superseded by generated art |
| `layer_0/1/2_strip_x5.png` + 3 `.import` | `editor/assets/backdrop_layers/` | x5 collision strips for the tile-backed near band (#76); the new `forest` layer is decorative |
| `displacement.png` diagnostic | `assets/backdrop_layers/` | diagnostic for the retired flow-recovery path |

12 MB on disk: 10 tracked PNGs plus 3 `.import` files, 13 tracked files in all. `tools/slice-layers` is **kept** (owner
decision) with its caveat documented: its only inputs are `assets/samples/*.mp4` and
the 1 fps frame directory, both gitignored, so a clean clone can never re-run it.

---

## 3. Target design

Full specification in the TDD. The short version:

| Layer | `scroll_scale` | Texture | Recipe |
|---|---|---|---|
| Sky | `(0.10, 0.08)` | 1920 x 1320 | 3-stop gradient + fBm haze + sun disc, fully opaque |
| HighClouds | `(0.20, 0.17)` | 1920 x 320 + headroom | `alpha = smoothstep(t0, t1, fbm)` — the hole proof |
| LowClouds | `(0.30, 0.25)` | 1920 x 320 + headroom | same, lower frequency, different seed |
| Hills | `(0.50, 0.42)` | 1920 x 384 + headroom | periodic heightfield, opaque fill, darker rim |
| Forest | `(0.70, 0.60)` | 1920 x 320 + headroom | periodic canopy blobs over an opaque ground band |
| Foreground | `(1.30, 1.15)` | 1920 x 320 + headroom | z **above** the tile layer; out-runs the camera |

The horizontal factors are the tutorial's published values verbatim. SSTD scales the y
factor to ~0.85x so both axes move, which the issue requires.

### 3.1 Scene shape

```
BackdropPreview (Node2D)
├── Camera2D                       # the single parallax input
└── Backdrops (Node2D)
    ├── Sky        Parallax2D  z=0  repeat_size=(1920,0)
    │   └── Sprite2D  centered=false  position=(0,0)
    ├── HighClouds Parallax2D  z=1  repeat_size=(1920,0)
    │   └── Sprite2D  centered=false  position=(0,0)
    ├── LowClouds  Parallax2D  z=2  repeat_size=(1920,0)
    │   └── Sprite2D  centered=false  position=(0,0)
    ├── Hills      Parallax2D  z=3  repeat_size=(1920,0)
    │   └── Sprite2D  centered=false  position=(0,0)
    └── Forest     Parallax2D  z=4  repeat_size=(1920,0)
        └── Sprite2D  centered=false  position=(0,0)
```

Factors and `repeat_size` are emitted into `manifest.json` by the generator, which is
the single source of truth for both the scene and the tests, so a factor is never
pinned in two places and allowed to drift.

### 3.2 Where it is applied

Both the Simulator tab (the runtime verification view, which becomes visible for the
first time) and the Map Editor grid (its static backdrop stack, repointed at the new
art). The editor draws statically with no camera; the runtime adds scroll factors. The
Map Editor stack becomes 6 entries (the 5 layers + the `Foreground` front placeholder
that already exists at `z_order = 10`).

---

## 4. Phases

### Phase 0 - docs (DONE in this pass)

- Wiki `TDD_Parallax-Background.md` revised: §2, §3.1, §3.2, §3.3, §4.1, §4.1.1, §5, §6, §7
- Wiki `GDD_Art-Direction.md` Parallax Background section rewritten
- Wiki `Home.md` row and `TODO.md` row TS71
- This plan, plus the checkpoint and `tools/slice-layers/README.md`

### Phase 1 - `tools/gen-backdrop`

New Rust workspace member, mirroring `tools/import-tiles` and `tools/slice-layers`:
`image = "0.25"`, `serde` + `serde_json` config, journal-grade stdout carrying inputs,
dimensions and output paths, `run() -> Result<PathBuf, String>`, error via
`eprintln!("gen-backdrop: error: {}", e)`.

```
Usage: gen-backdrop [--out DIR] [--width 1920] [--overscan 240] [--seed N]
Outputs: sky.png, clouds_high.png, clouds_low.png, hills.png, forest.png
         + manifest.json
```

**Who writes this:** @me, via the `coding-assistant` skill, in 9 blocks over 10 functions.
The non-trivial choices (the wrapping noise lattice, the alpha threshold, the heightfield)
are the learning target; the scaffolding, the config plumbing, the assembly loop and all
tests are the assistant's. See §5.

**The one hard requirement:** seamlessness. A layer whose first and last column differ
shows a visible join under `repeat_size`. The technique is a value-noise lattice that
**wraps in x** at the texture width, so any texture sampled over `[0, width)` tiles
exactly; fBm **doubles** the period per octave so the wrap survives at every scale.
(Doubling, not halving: octave `o` samples at `2^o` frequency, so it must span `2^o` cells
to keep the same spatial period. Halving also stays seamless, so no test distinguishes
them — see the note in block 3/9.)

Required unit tests, each shown to fail by mutation before it is trusted:

| Test | Claim | Mutation that must redden it |
|---|---|---|
| `seam_is_invisible` | first and last column identical on every layer | remove the wrap |
| `sky_has_no_alpha` | sky alpha is 255 everywhere | introduce a hole |
| `cloud_layers_have_holes` | alpha histogram has populated and empty buckets | move the threshold out of range |
| `factors_in_unit_range` | every factor in `[0,1]`, `factor_x > factor_y` | set a factor to 1.5 |
| `deterministic` | same seed -> byte-identical PNGs, and a different seed -> different bytes | introduce RNG state |

Five are listed here as the Phase 1 minimum; nine shipped. The four added after
falsification are `value_noise_stays_in_unit_range`, `fbm_stays_in_unit_range_and_wraps`,
`validate_factors_rejects_bad_specs` and `relief_is_absolute_not_a_fraction`. The last one
exists because `relief_px` was originally authored as a fraction of `height`, which
collapses to zero at small sizes and silently makes the seed irrelevant — a defect only
`deterministic`'s "a different seed must change the image" arm could catch.

### Phase 2 - remove the old art

`git rm` the 10 tracked PNGs and the now-empty `assets/backdrop_layers/` directory.
Keep `tools/slice-layers` with a README recording the gitignored-input caveat.

Consequence to state plainly: the `*_strip_x5.png` collision strips die with the old
art. The already-imported near-band cells in a world package keep their collision,
because those cells live in the world data, not in the PNG. Re-authoring a collidable
near band later is a fresh image-to-tiles import from the new `forest.png`, not a
restoration.

### Phase 3 - runtime wiring

- Regenerate `editor/scenes/backdrop_preview.tscn` from `manifest.json`, with
  `centered = false` children on the `(0,0)` crossing and non-zero `repeat_size.x`.
- `main.tscn`: `visible = true` on `BackdropStrip`; add a `BackdropScrollV` VScrollBar
  with `max_value = 240`.
- `simulator.gd`: drive `camera.position = Vector2(h, v)` from both bars, with a
  journal line recording the camera move.

### Phase 4 - editor grid

Repoint `tile_grid_display.gd`'s `_layers` default stack (and the `_BACKDROP_PATHS`
const if it still has a consumer) at the new art, 6 entries. No change to the draw
sandwich, the persistence path, or the LayerBox UI.

### Phase 5 - tests

Replace `probe_parallax_scroll.gd` (its 0.9x single-layer assertion is obsolete the
moment the factors change) with `test_parallax_backdrop.gd`.

**Spike first, and throw the spike away if it does not hold:** step the camera by
`(+120, +40)`, `await process_frame` twice, read each `Parallax2D.get_screen_offset()`.
The existing probe's own comment says `Parallax2D.position` "only accumulates from
camera motion across render frames", so this needs a real frame loop. If the offsets
are observable headless, assert `offset ~= camera_delta * scroll_scale` per layer, both
axes, within 0.5 px (the TDD §6 camera-drift test). If not, the test degrades to a
static-contract check and the drift claim moves to the manual capture probe.

Assertions, each mutation-verified:

| Assertion | Mutation that must redden it |
|---|---|
| strip `visible` on the instantiated `main.tscn` | set it back to false |
| each layer's `scroll_scale` matches `manifest.json` | zero one factor |
| no `Sprite2D` centered on `(0,0)` | set `centered = true` |
| every layer has `repeat_size.x > 0` | set one to 0 |
| both-axis drift within 0.5 px (if the spike holds) | change one factor |
| first and last column identical on every generated layer | remove the noise wrap |

Extend `capture_backdrop.gd` to grab 3 scrub positions so the result is reviewable as
PNGs.

### Phase 6 - gates, docs, close out

```bash
cargo test -p sstd-core -p gen-backdrop     # 110 + new
$HOME/bin/godot4 --headless --path editor --script res://tests/test_screen_store.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_image_to_map.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_terrain_brush.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_override_merge.gd
$HOME/bin/godot4 --headless --path editor --script res://tests/test_parallax_backdrop.gd
```

Exit code is authoritative, never the printed `failures=0` line (issue #80).

**Expect one pre-existing red, unrelated to this work (issue #85):**
`test_screen_store` exits 1 on Godot 4.7.2 and 0 on 4.4.1. It counts every zip entry
matching `begins_with("tiles/")`, and 4.7's `ZIPPacker` emits a `tiles/` directory entry
beside the two PNGs. Measured 2026-09-30 — 4.4.1: 187 / 9 / 14 / 26 ok, all exit 0;
4.7.2: 186 ok + 1 fail on `world-shared tile stored exactly once`, then 9 / 14 / 26 ok.
Do not fold that fix into this change and do not read it as a regression from the parallax
work. The new `test_parallax_backdrop.gd` must still exit 0.

Then: wiki push on `master`, checkpoint `CURRENT STATE` block, **#68 closed** with the
wiki page cited, a follow-up issue for the pre-existing legacy `.json` layer round-trip
gap (TDD §4.2, deliberately not bundled in), `--ff-only` merge to `trunk`, push, then
offer to delete the branch.

---

## 5. The coding-assistant block map

Per the `coding-assistant` skill, non-trivial logic is @me's to write; the assistant
writes scaffolding, plumbing and all tests. Numbering is global, in dependency order.
`M = 9` blocks over **10** functions: block 9/9 spans `validate_factors` and
`build_manifest`, and gets one `begin`/`end` marker pair per location, same block number.

**Revised from `M = 7` on 2026-09-30.** The 7-block map was wrong because it counted only
the layer generators and omitted the two helpers the generators call. `smoothstep` and
`lerp_rgb` are non-trivial logic with their own intent (the u8-truncation trap, the
degenerate-interval NaN guard), so they are blocks, not scaffolding. The count that matters
for verification is **10 marker pairs / 10 E0308 holes / 10 function bodies**.

| Block | Placement in `tools/gen-backdrop/src/main.rs` | What to write (not how) | Depends on |
|---|---|---|---|
| 1/9 | after the module consts | deterministic integer lattice hash -> `[0,1)`, no RNG state | - |
| 2/9 | below 1/9 | value noise whose lattice **wraps** in x - the seam guarantee | 1/9 |
| 3/9 | below 2/9 | fBm over that noise, doubling period per octave so the wrap survives | 2/9 |
| 4/9 | below 3/9 | widen-then-mix-then-narrow RGB lerp; u8 math truncates and bands | - |
| 5/9 | below 4/9 | Hermite ramp `t*t*(3-2t)` with a degenerate-interval guard | - |
| 6/9 | below 5/9 | sky plane: opaque everywhere, vertical gradient, fBm haze, one sun disc | 2/9 3/9 4/9 5/9 |
| 7/9 | below 6/9 | cloud alpha = smoothstep over fBm, so the layer has genuine alpha holes | 2/9 3/9 4/9 5/9 |
| 8/9 | below 7/9 | periodic silhouette column (heightfield fill + rim), shared by hills and forest | 2/9 3/9 4/9 |
| 9/9 | after 8/9 | validate factors are in `(0,1]` and `factor_x > factor_y`, then emit `manifest.json` | 6/9 7/9 8/9 |

The assistant writes: `Cargo.toml`, the `use` block, the `main`/`run` signatures, the
serde config struct (mirroring `import-tiles`), the per-layer assembly loop that walks
the config and calls 6/7/8, the journal lines, the PNG writer call, and all 9 unit tests.

### 5.1 Block format and how it is verified

Each open block is `begin marker` / INTENT doc comment / signature with an empty body /
`end marker`, with the preferred implementation as commented-out code inside the body.

- **Hole mechanism:** real return type, empty body -> `E0308`. Never `todo!()`, which
  type-checks as `!` and would let the crate build with the logic missing.
- **Markers:** `// TODO(human): begin block N/9` / `end block N/9`. The skill's own check is
  that `grep -n "TODO(human)"` lists both ends of every open block; it must return **20**.
- **Sample notes vs sample code:** a note belonging to the SAMPLE is written `// // note`
  and uncomments to `// note`; a statement is `// code` and uncomments to `code`. The
  distinction is load bearing - see the double-strip failure in
  `docs/SESSION-CHECKPOINT.md`.
- **Verification:** strip markers, uncomment every sample line, and require the result to
  compile clean and pass all 9 tests. Run on a throwaway copy; the repo tree is never
  written to. Measured result: compiles clean, 9 passed / 0 failed.


**Environment prerequisite (verified 2026-09-30):** the editor-driving channel works, so
the skill can run as written. `$NVIM` is set, and a non-terminal editor window exists
beside the opencode terminal:

```sh
$ echo "$NVIM"
/run/user/1000/nvim.3165735.0
# window inventory via --remote-expr luaeval:
#   1002 buf=(unnamed)                     buftype=          <- real editor window
#   1000 buf=term://~/projects/SSTD/...     buftype=terminal   <- opencode TUI
```

The skill's opener skips terminal buffers, so opening a file does not steal focus from
the opencode terminal. `code` is also on `PATH` but no VS Code process is running, so
nvim remote is the path in use.

**GDScript adaptation:** the skill's fail-fast guarantee is LSP-based. GDScript has no
AOT compiler, so the closest static equivalent is a declared return type the body
cannot produce. The project's GDScript rule already requires explicit types on every
signature, so this costs nothing.

---

## 6. Risks and open points

| Risk | Mitigation |
|---|---|
| The drift assertion is unobservable headless | Spike before writing the test; degrade to static contract + capture probe, and record which one shipped |
| `Parallax2D` offsets depend on the camera being `enabled` on the active viewport | The spike answers this; if it bites, drive the camera through a `SubViewport` in the test |
| Removing the root `assets/backdrop_layers/` copy breaks the wiki replay command | The wiki §4.1.1 no longer instructs that replay; it points at the generator |
| The generated art is ugly and gets mistaken for final art | It is documented as a palette-agnostic placeholder whose job is to prove the mechanism; palette re-tinting is a separate later pass |
| `probe_parallax_scroll.gd` deletion loses the 1:1 camera assertion | Carried forward into `test_parallax_backdrop.gd`; the 0.9x factor assertion is dropped because the factors changed |
| Six layers is close to the 7 cap | The foreground is excluded from `biome_backdrop_layers` (§4 of the TDD), so the biome-backed rows stay at 5 |

---

## 7. References

- Issue #68, #74 (retired MP4 path), #76 (image-to-tiles import), #80 (exit codes)
- [2D Parallax tutorial](https://docs.godotengine.org/en/stable/tutorials/2d/2d_parallax.html)
- Wiki `TDD_Parallax-Background`, `GDD_Art-Direction`
- `tools/import-tiles/src/main.rs` and `tools/slice-layers/src/main.rs` (crate conventions)
- `docs/SESSION-CHECKPOINT.md` (checkpoint of record)
