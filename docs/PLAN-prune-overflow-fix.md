# PLAN — Fix prune for near-duplicate tiles that straddle the coarse-bucket boundary

> **Status**: Implemented and tested 2026-08-16. A3 neighbour scan (fast tier)
> and the deep Approach-B tier are wired to a separate "Prune (deep)" button;
> synthetic suite green. See
> [Implementation status](#implementation-status-2026-08-16).
> **Tracks**: [issue #51](https://github.com/HidekiAI/sidescroll-towerdefense/issues/51)
> ("Prune Duplicates button"); a real-catalog pitfall found while manually
> verifying it.
>
> **Symptom (observed 2026-08-15)**: stamp_1539..1555 and stamp_1492..1495 look
> visually identical to the eye (SSIM ~0.978-0.981, raw per-byte mean diff
> ~0.8). The "Prune Duplicates" button ran twice and merged **nothing** — all
> PNGs remain on disk.

## Root cause

`_build_prune_plan()` (`editor/scripts/map_editor.gd:989-1015`) buckets every
tile by `_similarity_sig()`, then runs the exact diff check **only within a
bucket**:

1. `_similarity_sig(img)` -> `_coarse_bytes(img)` (mean of each 8x8 block,
   64*4 channels) then `>> 6` (keep top 2 bits), flip-canonicalized
   (`map_editor.gd:911-924`, `1362-1386`, `1388-1410`).
2. Group by `_fp_hash(sig)` (`_build_prune_plan`).
3. Within a group, pick lowest-id rep and confirm each candidate with
   `_flip_of` -> `_cell_bytes` + `_diff <= STAMP_TOLERANCE` (4.0)
   (`map_editor.gd:987-1015`, `1311-1360`, `36`).

The **exact** check passes for these tiles (raw mean diff ~0.8 < 4.0). The
failure is in step 1-2: a ~1-unit change to a block-mean can push any channel
across a `>>6` boundary, flipping the signature. Visually-near-identical tiles
then land in **different buckets**, so the exact diff is never even compared.
This is the same boundary weakness called out for the 8-bit fingerprint
(3025 tiles -> 3025 unique fingerprints) in
`docs/TDD_Tile-Reuse-and-Stamp-Groups.md:206-210`; the 2-bit quantization
collapses many, but boundary-straddlers still escape.

The existing flow is **user-triggered** (never at boot) because a full ~3k-tile
pass could feel like a hang (`map_editor.gd:954`, TDD §11).

---

## Two candidate approaches

Both fix the escape. They differ in where the "can't miss a straddler"
guarantee lives.

### Approach A — refine the coarse bucket (keep index-fast grouping)

Keep the fast bucket-and-verify structure, but make the *fingerprint* robust to
boundary straddling, then still confirm with the exact diff.

**A1 (largest change to the metric, strongest):** drop quantization per-channel
binning that straddles; use a *Euclidean/norm-based* or *threshold-tolerant*
signature lookup instead of an exact-equality `_fp_hash` group key:

- Replace the "identical 2-bit sig" assumption with a bucket that is a ring /
  LSH-style neighbourhood on the coarse vector, so tiles whose coarse vector
  differs slightly still share a bucket.
- The expensive exact `_flip_of` diff still runs as the final arbiter, so the
  coarse tier only needs to be "close-enough", not exact.

**A2 (minimal, keeps `_fp_hash`):** keep the exact-group `_fp_hash` bucketing,
but *repeat* the scan over K shifted quantizations and union the candidate
pairs. A tile straddling a boundary at `>>6` is caught under a neighboring
quantization offset (e.g. `>>6` plus `(x+16)>>6`). The additional reference
candidates add only small extra work vs. the single pass.

**A3 (bounded pairwise + LRU):** within a group's rep, also confirm a few
**neighbouring buckets** (those whose sig differs in one channel by one unit)
before giving up. Cheap and targeted, but only provable if we know the one-unit
neighbourhood was actually searched.

**Trade-offs (A):** cheap, keeps the index/flow, needs the sig/hash to be
boundary-robust in a way that must be reasoned about carefully. Or accepts a
bounded-but-not-formally-complete guarantee (A2/A3). Journal still gets an
accurate merge line. Most versions leave "came from a different bucket" as an
implicitly-tested edge (needs a regression test with a synthetic
boundary-straddler).

### Approach B — full pairwise exact-diff override on "no duplicates"

Add a **second, exhaustive tier** that only runs when the coarse pass reports
"no duplicates" (or optionally as a `--deep` / "Prune (deep)" second button). It
does a true O(N*M) exact-diff candidate search with no coarse pre-bucket:

- For each tile (in catalog rank order by rep), compute exact `_cell_bytes` and
  compare against every *kept* rep so far with `_flip_of` (all 4 flips).
- Keep the same merge/summary/journal logic (`merge_map`, `_rewrite_references`,
  `_drop_tiles`) — only the candidate *discovery* changes.
- Complexity: O(N^2) bytes in the worst case. For ~3k tiles at 32x32x4 bytes
  (4096 bytes each) that is ~12.5M combination, ~1 seconds-to-tens-of-seconds
  inside the awaited `_busy` pass. The flow is already user-triggered and shows
  a busy cursor — acceptable as an explicit deep prune.

**Correctness**: formally complete (no boundary is ever skipped) because it
ignores the fingerprint tier for discovery. Fully deterministic end-to-end.

**Trade-off (B)**: one-time worst-case cost; must stay a user-opted action (not
boot). No change to the strict exact-`_diff<=STAMP_TOLERANCE` semantics, no new
fingerprint to reason about.

---

## Recommended path

- **Adopt B (deep exact prune) as the primary fix** — guarantees the
  boundary-escape is closed with the *same* exact-diff semantic it already
  uses. Add it as a second, opt-in "deep" mode of the existing Prune button
  (ask user Q1).
- **Optionally harden the fast-tier with A2** (shifted-quantization + union of
  candidate buckets) so the already-fast path also catches straddlers, keeping
  the deep pass purely as a safety net. Decide scope in Q1/Q2.

> Resolution (2026-08-16): this section is superseded by the Decision below and
> the [Pinned-down technical design](#pinned-down-technical-design-2026-08-16).
> B is NOT the primary path; it is the separate "Prune (deep)" tier. The fast
> tier keeps A2 (as built) plus the A3 neighbour scan.

## Decision (2026-08-15)

Chosen: **approach A, via A2 + union-find**, as the primary fix.

> Superseding note (2026-08-16): this remains the fast A-tier. The deep tier
> (approach B) and the A3 neighbour scan are now pinned down in the
> [Pinned-down technical design](#pinned-down-technical-design-2026-08-16).

- Group candidates across **two quantization planes** (top-2-bit sig at shift 0
  and shift 32) so a coarse-block mean straddling a boundary in one plane stays
  inside one bucket in the other.
- Build a **Union-Find over catalog indices**: any member the exact `_flip_of`
  confirms against its group representative unions those two indexes; the
  lowest numeric stamp id becomes the root.
- After collapsing, each non-root member is **re-verified against the final
  root** with the exact `_flip_of` (`<= STAMP_TOLERANCE`) so transitivity never
  widens tolerance.
- This keeps the fast bucket-index flow (no full O(N^2) pass), fixes the
  boundary-straddler escape, and reuses `_rewrite_references` /
  `_execute_prune` untouched. Residual escape (multiple disjoint channels
  straddling on *different* planes at once) is rare and can be closed later by
  approach B if it ever shows up; B stays documented for that.
- **Implement now**: change `_build_prune_plan()` in `map_editor.gd` only
  (+ small Union-Find helper); extend the prune unit test in
  `editor/tests/test_screen_store.gd` with a synthetic boundary-straddler pair.

## Refined discovery design (planning only, after A-on-real-catalog trial)

The lowest-id-root union-find closes single-boundary straddles but the
1539..1555 family shares one 591-member coarse bucket whose root (`stamp_1`)
does not exactly match any far leaf (`passes_exact=0 of 590`), so the whole
bucket is rejected. Root cause: 2-bit coarse buckets are too coarse to give a
single representative that is exact-near every leaf, and lowest-id alone picks
a bad rep.

### Proposed image ordering

Use the image representation suggested during investigation:

1. Convert each RGBA tile to luminance using BT.601: `0.299R + 0.587G +
   0.114B`.
2. Divide the 32x32 tile into 64 non-overlapping 4x4 blocks (8 across, 8
   down), compute one mean luminance per block, and quantize each mean to 16
   levels (4 bits). The resulting fingerprint is 64 nibbles, packed into 32
   bytes. Keep alpha separate or require alpha to pass the existing exact diff;
   do not discard alpha in the final comparison.
3. Derive sortable projections from that fingerprint, initially:
   - total luminance / sum of all 64 nibbles;
   - optionally a second stable key such as the first few coarse cells or a
     luminance histogram.
4. Sort catalog entries by the projection. Equal projections retain a
   deterministic tie-break by `_catalog_rank`.
5. Scan candidates near each entry in sorted order and confirm every candidate
   with `_flip_of()` and the existing `_diff <= STAMP_TOLERANCE`. The exact
   diff remains the only merge authority.

### Important correctness constraint

A fixed **number-of-items** window is not itself a recall guarantee: a dense
brightness region can contain arbitrarily many unrelated tiles. The first
implementation must therefore use a **scalar-distance range** derived from the
maximum allowed exact diff, not silently rely on `W` alone. A bounded item
window may be added only as a performance optimization after measurement, with
a fallback to the full scalar range.

The scalar range must be derived conservatively and documented in code. If the
projection cannot provide a safe bound for all supported image formats, the
implementation must use multiple sorted projections or fall back to the
existing exhaustive candidate check for that bucket. A perceptual fingerprint
may reduce candidates, but it must never be treated as a cryptographic hash or
as proof of visual equality.

### Candidate discovery and merge semantics

- Retain the current coarse bucket pass for cheap exact matches.
- Use the luminance-sorted candidate pass to discover matches missed by the
  coarse pass, including 1539..1555 and 1492..1495.
- Choose the lowest numeric stamp ID among **exactly matching** candidates, not
  the lowest ID of a broad coarse bucket.
- Re-verify each proposed merge against its chosen representative so chained
  near-matches cannot widen the tolerance.
- Reuse `_rewrite_references`, `_drop_tiles`, `_execute_prune`, and the existing
  journal line; destructive deletion remains user-confirmed and opt-in.

### Verification plan

- Unit test: seed a grayscale boundary-straddler pair and a multi-leaf family;
  assert the ordering finds the pair/family, rejects a distinct same-brightness
  tile, chooses the lowest exact-match representative, and rewrites a grid
  reference.
- Read-only real-catalog smoke: assert stamp_1539..1555 and stamp_1492..1495
  are present in the proposed merge plan before any destructive merge.
- Performance smoke: record catalog size, candidate comparisons, elapsed scan
  time, and final duplicate count without printing binary data, raw hashes, or
  image bytes.
- Manual destructive run only after the preview count and representative list
  are reviewed.

### Prototype status (2026-08-15)

Implemented the read-only fingerprint primitive and regression coverage:

- `map_editor.gd` exposes `_grayscale_sig()` and `_grayscale_projection()` for
  a BT.601 grayscale, 8x8 block, 4-bit-per-cell representation packed into 32
  bytes.
- `_grayscale_luminance_projection()` and
  `_grayscale_projection_candidates()` now provide a read-only candidate pass:
  exact candidates are selected by a conservative scalar-distance range of
  256, then verified by `_flip_of()`.
- `test_screen_store.gd` verifies packed size, deterministic equality,
  dark-before-bright ordering, candidate generation, and exact rejection of a
  distinct tile.
- `map_editor.gd` still uses the existing A2 coarse-bucket prune path for
  actual merge planning; grayscale candidates do not yet delete or rewrite
  anything.
- Verification passed: Godot headless editor tests report zero failures;
  `cargo check --workspace` and `cargo test --workspace` pass.

Next implementation step: optimize the scalar-range candidate pass before
using it for the real catalog. The synthetic chain tests pass, but the first
real-catalog dry-run did not finish within the test timeout: a conservative
luminance range of 256 creates too many candidate comparisons in the dense
dark-tile region. Do not enable this path for destructive pruning until a
measured secondary projection, bounded bucket fallback, or another candidate
partition reduces that workload while preserving exact verification.

The candidate pass is now integrated into `_build_prune_plan()` in the working
tree, but this integration is not yet accepted for destructive real-catalog
use. A first performance correction changed the sorted backward scan from
`continue` to `break`, then added a cheap 4-bit grayscale-signature L1 gate
before `_flip_of()`.

Real-catalog dry-run result after that correction: 2718 catalog entries,
697 proposed duplicate keys, all 21 requested targets from stamp_1492..1495
and stamp_1539..1555 detected, elapsed time approximately 13.5 minutes. The
result is functionally promising but still too slow for an interactive prune;
optimize the candidate gate or use a lower-cost exact comparison before
exposing this plan to destructive confirmation. The synthetic suite remains at
zero failures.

## Optimization plan (2026-08-15)

Optimization happens before further destructive integration. The current
13.5-minute real-catalog preview is a functional proof only, not an acceptable
user workflow.

### Constraints

- Keep `_flip_of()` / `_diff <= STAMP_TOLERANCE` as the only merge authority.
- Keep `_build_prune_plan()` read-only until the benchmark target is met.
- Do not print raw image bytes, binary data, or hashes in diagnostics.
- Preserve deterministic representative selection and chain-safe re-verification.

### Staged work

1. Profile counts: catalog size, scalar-range candidates, grayscale-prefilter
   survivors, exact comparisons, exact matches, and elapsed time.
2. Cache immutable derived data once per tile: RGBA bytes, 64-cell grayscale
   signature, luminance projection, and cheap color/alpha summaries. Avoid
   repeated image conversion, data extraction, and allocation in pair loops.
3. Add a second conservative spatial projection. It may reject only when its
   bound proves the exact tolerance impossible; otherwise it passes through.
4. Add a color/alpha prefilter only when its rejection rule is conservative;
   color and alpha cannot be discarded because the final diff includes RGBA.
5. Benchmark each stage on the real catalog. Target preview time is under 10
   seconds; 30 seconds is the warning threshold.
6. If no conservative filter meets the target, expose a labelled Deep Prune
   mode with progress/cancel support rather than hiding a multi-minute pass.

### Acceptance criteria

- Synthetic duplicate, boundary, chain, and distinct-tile tests remain green.
- Real-catalog dry-run completes under 10 seconds, or explicitly reports that
  deep mode is required.
- All 21 known targets remain detected.
- Diagnostics contain numeric counts only; no binary data or raw hashes.
- No PNG deletion or reference rewrite occurs during optimization benchmarks.

Implementation starts with profiling and derived-data caching; destructive
behavior remains unchanged.

### Optimization attempt result

The first implementation added per-tile image/signature caching and numeric
counters, and the synthetic suite stayed green. A later invalid benchmark
invoked the full real-catalog scan twice and timed out; it was removed.

Next optimization pass: per-tile cached RGBA cell bytes plus the four prebuilt
flip variants, an early-exit `_diff_capped()`, and an instrumented single-pass
candidate scan. Synthetic suite stayed green. A scan-only real-catalog probe
completed in about 8.9 minutes (535 seconds) with 475k candidates in the
scalar range, about 403k surviving the 4-bit grayscale gate, and 403k exact
comparisons. This confirms the bottleneck is the dense near-black candidate
region and the cost of full RGBA flip comparisons over it, not image
extraction. The 10-second target is still not met, so this path remains
unsuitable for destructive confirmation. Destructive behavior is unchanged.

Additional work remains: strengthen the pre-verify gate with a conservative
per-region color or block-mean bound so it rejects most near-black candidates
without discarding color, or bucket the scan so dark tiles are compared to
fewer representatives. Any filter must only reject when the exact tolerance
is provably impossible; it must never alone be the merge authority.

## Implementation status (2026-08-16)

Implemented in `editor/scripts/map_editor.gd` and both `editor/scenes/map_editor.tscn`
and `editor/scenes/main.tscn`:

- Constants: `A3_K := 8`, `A3_LRU_CAP := 64`, `DEEP_CONFIRM_THRESHOLD := 2500`,
  `DEEP_YIELD_EVERY := 500`, `PRUNE_WARN_THRESHOLD := 500`,
  `PRUNE_EST_MS_PER_TILE := 197`.
- Fast tier refactor: `_build_prune_plan()` now runs grayscale scan -> union-find
  -> `_a3_discover()` -> union-find -> `_finalize_prune_plan()`. `_a3_discover()`
  probes the existing `_stamp_fp_index` one-unit neighbour buckets for singleton
  roots the grayscale pass left unmatched; `_a3_neighbor_hashes()` enumerates
  the <= 512 distinct neighbour hashes.
- Deep tier: `_on_prune_deep()` (budget gate + confirm) -> `_run_deep_scan()`
  -> `await _build_deep_prune_plan()` -> shared `_present_prune_plan()` ->
  `_execute_prune()`. `_build_deep_prune_plan()` is Approach B (rank-order kept
  reps, cached bytes + `_flip_of_variants`, `await` yield every
  `DEEP_YIELD_EVERY` diffs). `_on_prune_duplicates()` was refactored onto the
  same `_present_prune_plan()` presentation helper.
- A "Prune (deep)" button added after `PruneBtn` in the BottomBar of both
  scenes, wired via `deep_prune_btn`.
- **Scan-window binary search**: `_grayscale_projection_candidates()` now finds
  the lower bound of each tile's projection window via `_projection_lower_bound()`
  (classic binary search over the projection-sorted `ordered` array) instead of
  walking backward tile-by-tile. Sparse regions jump straight to the window
  start in `O(log N)`; the in-window walk is unchanged. Measured equivalent on
  dense synthetic data (window covers most tiles there).
- **Cost warning dialog**: pressing "Prune Duplicates" with more than
  `PRUNE_WARN_THRESHOLD` tiles first shows a `ConfirmationDialog` that states
  the estimated wall time (`tiles * PRUNE_EST_MS_PER_TILE`, the serial
  fast-tier rate measured on the real 2718-tile catalog: 535 s -> 197 ms/tile,
  so ~3000 tiles ~= 10 min) and that the UI pauses with a busy cursor while it
  scans. The scan runs only on confirm; below the threshold it runs immediately.

### Parallelism finding (measured 2026-08-16)

Attempted to speed the exact-comparison hot loop with GDScript `Thread`s over
`OS.get_processor_count()` (32 on this host). Measured results on Godot 4.4.1
headless:

- Pure integer loop: 8 threads -> 5.9x speedup (GDScript threads do run in
  parallel).
- The real workload (`_diff_capped` over shared `PackedByteArray` buffers):
  8 threads -> **0.72x** (slower than serial) with shared arrays, and only
  **1.77x** with private per-thread copies.

Conclusion: GDScript serializes packed-array element access across threads
(refcount/GC contention on the shared `Array[PackedByteArray]`), so spawning
`Thread`s in GDScript is *counterproductive* for this loop. The threaded path
was reverted; the scan stays serial. The honest paths to real 32-core
parallelism are (a) move the exact-diff loop into the existing
`crates/sstd-editor-bridge` godot-rust extension and parallelize with native
`std::thread`/rayon, or (b) keep the scalar gate so the serial rate stays the
only cost.

### Rust bridge parallel exact-diff (implemented 2026-08-16)

Path (a) is now implemented. A new `#[func] scan_exact_matches` on
`SstdBridge` (`crates/sstd-editor-bridge/src/lib.rs`) ports the exact-diff hot
loop to native Rust and runs it on `std::thread::scope` scoped threads (no new
crate dependency; rayon already exists in the lockfile as a transitive dep but
scoped threads need nothing extra). It receives three flat buffers from
GDScript and returns a flat match stream:

- `variants_flat`: `N * 4 * 4096` bytes (identity, h-flip, v-flip, hv-flip).
- `bytes_flat`: `N * 4096` canonical RGBA bytes.
- `pair_stream`: flat `[base, candidate]` index stream.
- Returns flat `[base, candidate, flip_h, flip_v]` matches.

The GDScript loop it replaces (`_diff_capped` running-mean early-exit,
`_flip_of_variants` best-of-4-flips, tolerance 4.0) is ported byte-for-byte as
`diff_capped`/`flip_of_variants`; each worker owns a disjoint slice of the pair
stream and shares the byte buffers read-only. `_exact_matches_parallel` in
`map_editor.gd` now calls the bridge when `_bridge.has_method(...)` and
`exact_pairs.size() >= BRIDGE_MIN_PAIRS` (512), marshalling the arrays into
flat buffers (`_exact_matches_bridge`), and falls back to the serial GDScript
loop otherwise (so headless test runs without a bridge stay correct).

Benchmark (19900 identical-ish pairs, 32 processors, Godot 4.4.1 headless):

- Serial GDScript: 89301 ms.
- Rust bridge: 683 ms -> **~131x speedup**; byte-identical match stream.

Projected on the real catalog (~403k gate-surviving pairs) this moves the exact
comparison from ~8.9 min to a couple of seconds; the projection/gate phase
stays GDScript and remains the dominant remaining cost.

Tests: 5 new cargo unit tests in the bridge crate (`diff_capped` port, flip
parity, parallel scan finds a flipped pair) plus `_test_bridge_exact_scan` in
the headless GDScript suite, which instantiates the real `SstdBridge` when the
extension is loaded, runs the same 48-tile synthetic input through both
`_exact_matches_slice` and `_exact_matches_bridge`, and asserts identical match
streams. Full suite: `failures=0`, cargo 92 tests pass.

Reconciliation with the pinned A3 text below: the pinned sketch described the
2-bit `_similarity_sig` space (`delta +/- 64`). The shipped index
`_stamp_fp_index` is keyed on the 8-bit `_canonical_coarse` vector (256 bytes),
so the implemented A3 perturbs one byte by `+/- 1` — one 8-bit block-mean unit,
the exact analog of one 2-bit unit in that space. Verified against the actual
key space (`map_editor.gd:1616` `_index_stamp_entry`).

Tests (all green in the headless suite): `_test_a3_neighbor_scan` (one-block
deviation duplicate merged, distinct tile kept, bounded neighbour enumeration),
`_test_deep_prune` (exact dup found, lowest-id rep, read-only plan, merge
drops dup + rewrites grid ref), and `_test_parallel_exact_scan` (48 identical
tiles -> 1128 gate-surviving pairs merge onto the lowest id; validates the
serial path stays correct). Full suite: `failures=0`, cargo 87 tests pass.
`main.tscn` smoke-loads with the new button attached.

Known limitation (updated 2026-08-16): the exact-comparison phase is now native
and ~131x faster in the Rust bridge (see above); the remaining fast-tier cost is
the GDScript projection/signature-gate phase, still ~8.9 min serial for ~2718
tiles. The cost warning dialog tells the user the estimated wall time before
the scan runs. The deep tier is budget-gated and opt-in, and destructive
confirmation is unchanged.

## Open questions to confirm before implementing

> Resolved 2026-08-16 — answers recorded in the Resolved decisions subsection
> and the Pinned-down technical design below. Kept for history.

1. Always-run deep as part of `_on_prune_duplicates`, or a separate "Prune
   (deep)" control? (Influences whether the no-dup dialog stays.)
2. Keep the fast A-tier path untouched, or extend it with A3? (Cost/benefit of
   the extra hardening.)
3. Maximum acceptable deep-pass budget (e.g. tiles > N skip without confirm).
4. Should deep also consider cross-flip variant that the coarse tier may have
   already merged (re-merge into same rep) — re-run or rely on existing
   `_rewrite_references` XOR logic?

### Resolved decisions (confirmed 2026-08-16)

1. Separate "Prune (deep)" control; the fast A-tier pass and its no-dup dialog
   stay as-is. Deep runs only when the user explicitly invokes it.
2. Extend the fast A-tier with the A3 hardening (bounded pairwise + LRU) rather
   than replacing it; keep a fast fallback for small catalogs.
3. Deep-pass budget: tiles above a threshold (default N = 2500) require the
   existing confirm dialog before the scan runs; below it, run immediately.
4. Cross-flip re-merge: rely on the existing `_rewrite_references` XOR logic;
   do not add a separate re-run for variants the coarse tier already merged.

## Pinned-down technical design (2026-08-16)

### Fast tier: existing A2 + A3 neighbour scan

The fast "Prune Duplicates" button keeps the current A2 flow (two
quantization planes at shift 0 and shift 32, union-find, final root
re-verify with `_flip_of`). A3 is bolted on only for tiles whose exact check
against their own bucket representative fails.

Data structures (all built once per `_build_prune_plan()` call):

- `_stamp_fp_index`: `_fp_hash(sig) -> [catalog idx ...]`, reused unchanged.
- `A3_LRU`: ordered map `rep_key -> [neighbour_hash ...]` (cap `A3_LRU_CAP = 64`)
  so a rep's neighbour buckets are enumerated once and shared by every member
  of its bucket.

A3 algorithm, per member `m` whose `_flip_of(rep, m)` returned empty:

1. `canon := _similarity_sig(m)`; for each of the 64 block cells x 4 channels,
   build the two one-unit neighbour signatures `canon[cell][ch] +/- 64`
   (clamped to 0..255). 512 candidates max.
2. Hash each neighbour with `_fp_hash` and look up its bucket. Deduplicate
   bucket lists via a per-call `seen_hash` set so one neighbour bucket is
   visited once.
3. For each distinct neighbour bucket, run `_flip_of(rep_neighbour, m)`.
   Stop early once `A3_K = 8` distinct reps have been exact-checked.
4. On a hit, union-find `m` with that rep exactly as A2 does; the final root
   re-verify in `_build_prune_plan()` stays authoritative.

Complexity: worst case `O(N * (512 hash lookups + A3_K exact diffs))`. The
hash lookups are dictionary probes; each exact diff is 4 flips x 4096 bytes.
A3 adds a bounded constant per tile, so it cannot reintroduce the dense-region
explosion.

Sample loop (GDScript sketch, not the final commit):

    for m in bucket:
        if not _flip_of(rep_img, _as_rgba8(m)).is_empty():
            _uf_union(parent, rep_idx, m)
            continue
        seen.clear()
        for cell in 64:
            for ch in 4:
                for delta in [-64, +64]:
                    sig2 = canon.duplicate()
                    sig2[cell * 4 + ch] = clampi(sig2[cell * 4 + ch] + delta, 0, 255)
                    h = _fp_hash(sig2)
                    if seen.has(h):
                        continue
                    seen[h] = true
                    for other in _stamp_fp_index.get(h, []):
                        if _flip_of(_as_rgba8(rep_of(other)), _as_rgba8(m)).is_empty():
                            continue
                        _uf_union(parent, idx(other), m)
                        break
                    if checked >= A3_K:
                        break

### Deep tier: Approach B full pairwise, budget-gated

The separate "Prune (deep)" control runs the exhaustive pairwise exact scan
from the Recommended-path section. It is formally complete: no coarse bucket
exists to escape, so every boundary-straddler is found.

Algorithm:

1. Sort catalog indices by `_catalog_rank` (lowest stamp id first).
2. Maintain a kept-rep array `kept = []`. For each tile `m` in rank order,
   exact-check `m` against every entry in `kept` with `_flip_of` until either a
   match is found or the LRU budget for this tile is exhausted.
3. On a match, union-find `m` to the matching kept rep and advance; on no
   match, append `m` to `kept`.
4. After the scan, run the same `_uf_union` root-collapse, per-root re-verify
   against the final root, and `merge_map` build as the fast tier, so both
   tiers share `_rewrite_references` / `_execute_prune` / `_drop_tiles`.

Complexity: `O(K * M)` exact diffs where `M` is kept reps; worst case `O(N^2)`
pairwise at 4 flips x 4096 bytes each. For ~3k tiles this is the 12.5M-pair
order estimated in the Recommended-path section: seconds to tens of seconds.

Budget gate wiring:

- If `_stamp_catalog.size() > DEEP_CONFIRM_THRESHOLD` (default 2500), open the
  existing `ConfirmationDialog` before scanning; the scan itself runs inside
  `_set_busy(true)`. Below the threshold, scan immediately.
- Progress: `_on_prune_deep` yields to the engine every `DEEP_YIELD_EVERY =
  500` exact diffs so the busy cursor stays responsive and a cancel flag can
  be polled (future work; first version runs to completion).
- The deep pass reuses the same read-only-plan -> confirm -> destructive flow
  as `_on_prune_duplicates`, with its own journal line
  (`prune deep: merged %d ...`). No new binary/hash output.

Correctness: exhaustive discovery guarantees no straddler escapes; the exact
`_diff <= STAMP_TOLERANCE` gate remains the only merge authority, and the
per-root re-verify prevents tolerance widening via chains.

## Findings recorded during native port (2026-08-17)

- **GDScript int-truncation quirk (root cause of the 982-vs-490 dup_keys
  mismatch)**: GDScript `_flip_of_bytes` (map_editor.gd:1204) declares
  `var best_diff := 0x7fffffff` (int). Assigning a float diff truncates it, so a
  variant with `diff in [tol, tol+1)` (e.g. 4.17 with `STAMP_TOLERANCE=4.0`)
  still matches. The Rust bridge must replicate exactly with `best_diff: i32` +
  `d as i32` + final check `best_diff as f64 <= tol`. The initial f64 port was
  stricter and dropped matches (dup_keys 982 -> 490). Verified pair-for-pair
  parity on the real catalog after the fix: checked=1228, native=982,
  gdscript=982, 0 mismatches.
- **`_flip_of_variants` (map_editor.gd:1167, serial scan path) shares the same
  int best_diff truncation**, but native `flip_of_variants` (used by
  `scan_exact_matches`) keeps f64 semantics. Consistent on real data
  (exact=133231) and the 600-tile probe; latent boundary divergence only.
- **Release build is required**: the debug `.so` was 3-10x slower across the
  board (scan 8.6s->4.2s, a3 neighbour hashes 4.3s->0.38s). Rebuild workflow is
  now `cargo build --release -p sstd-editor-bridge` + `cp target/release/...`.

## Verification plan

- Unit test (extend `editor/tests/test_screen_store.gd` `_test_prune_*`): seed
  a synthetic catalog with a fine boundary-straddler pair that the current
  `_similarity_sig` buckets apart; assert the deep pass merges them onto the
  lowest id, rewrites a `_tiles` / `_store.cache` / `_tile_set_groupings`
  reference, and drops the dup key.
- Headless smoke: boot editor, run plan builder on the real catalog with a dry
  flag, assert the journal registers the deep-merged count previously missed
  (1539..1555/1492..1495 appear), without deleting real PNGs in the dry mode.
- Full manual run (destructive, user-confirmed) then confirm that the
  previously-missed ids are removed from `assets/tiles/`.

## Anti-goals

- Do NOT merge merely-sub-threshold-but-distinct art; the exact
  `<= STAMP_TOLERANCE` gate stays the definition of "duplicate".
- Do NOT auto-run prune at boot — stays opt-in, busy cursor / confirm dialog.
- No new binary/hash output in journal that the OpenRouter filter could
  mis-flag; the prune journal line stays counts/ids only.