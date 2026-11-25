# Verification evidence: GDScript DAO and View delegation (2026-11-25)

Scope: issue #161 (Rust-owned physics and terrain models), code branch
`local/feat/model-gateway-s4`. This is the tracked record of the falsifiability run. The scratch
harness that produced it was `/tmp/user/1000/opencode/sstd-s7-mutations.sh`; its captured output is
reproduced verbatim below, so no claim here rests on an untracked artifact.

## Method

- Runtime: Godot `4.7.2.stable.mono.official.ed1daf0bf`.
- Green baseline before any mutant: `test_view_model_delegation` exit 0, `test_model_gateway` exit 0.
- Each mutant is one string substitution. The harness aborts when the substitution does not fire,
  reads an unchanged target as VOID, and reads an exit 0 as REFUTED.
- After each mutant the target is copied back from a byte-for-byte backup and checked with `cmp`.
- A postflight reruns both suites after the final restore.

## Mutants and the named assertion each reddened

| Mutant | Target | Named assertion |
|---|---|---|
| `view-profile-call-dropped` | `world_view.gd` | profile: the View makes exactly one gateway call |
| `view-profile-null-zero` | `world_view.gd` | profile: the View unpacks bounds and keeps air columns as NAN |
| `view-sample-delegation-dropped` | `world_view.gd` | sample: the View delegates to the gateway |
| `view-walker-cached-tiles-dropped` | `world_view.gd` | walker: the View sends state, intent and cached tiles |
| `view-walker-unwrap-dropped` | `world_view.gd` | walker: the View unwraps the gateway's walker result |
| `view-plateau-fallback-removed` | `world_view.gd` | real bridge: the masked surface at column 6 is 39 px |
| `view-malformed-fallback-removed` | `world_view.gd` | malformed: an unparsable walker response leaves the caller's state unchanged |
| `dao-tile-coercion-dropped` | `model_gateway.gd` | coercion: world_tile_x is emitted as an integer |
| `dao-null-columns-dropped` | `model_gateway.gd` | null: an air column stays null, not NAN |
| `dao-null-to-nan-dropped` | `model_gateway.gd` | sample: a null sample becomes NAN |
| `dao-walker-unwrap-dropped` | `model_gateway.gd` | step: the walker result is unwrapped from the response |
| `dao-terrain-coercion-dropped` | `model_gateway.gd` | import: every listed integer field loses its decimal |
| `dao-repeat-nonce` | `model_gateway.gd` | repeat: the same input serializes identically |

Two earlier mutant forms were retargeted, not counted: replacing the DAO's whole decoded dictionary
made the guard abort before its assertion, and `OS.get_ticks_usec()` does not exist (the correct call
is `Time.get_ticks_usec()`). Both were void experiments and were replaced before the final run.

## Captured run (verbatim)

```text
2026-11-25T20:12:58Z baseline: test_view_model_delegation exit=0, test_model_gateway exit=0
2026-11-25T20:13:00Z view-profile-call-dropped FIRED: test_view_model_delegation exit=1 on 'profile: the View makes exactly one gateway call'; restored byte-identical
2026-11-25T20:13:01Z view-profile-null-zero FIRED: test_view_model_delegation exit=1 on 'profile: the View unpacks bounds and keeps air columns as NAN'; restored byte-identical
2026-11-25T20:13:03Z view-sample-delegation-dropped FIRED: test_view_model_delegation exit=1 on 'sample: the View delegates to the gateway'; restored byte-identical
2026-11-25T20:13:05Z view-walker-cached-tiles-dropped FIRED: test_view_model_delegation exit=1 on 'walker: the View sends state, intent and cached tiles'; restored byte-identical
2026-11-25T20:13:07Z view-walker-unwrap-dropped FIRED: test_view_model_delegation exit=1 on 'walker: the View unwraps the gateway's walker result'; restored byte-identical
2026-11-25T20:13:08Z view-plateau-fallback-removed FIRED: test_view_model_delegation exit=1 on 'real bridge: the masked surface at column 6 is 39 px'; restored byte-identical
2026-11-25T20:13:10Z view-malformed-fallback-removed FIRED: test_view_model_delegation exit=1 on 'malformed: an unparsable walker response leaves the caller's state unchanged'; restored byte-identical
2026-11-25T20:13:11Z dao-tile-coercion-dropped FIRED: test_model_gateway exit=1 on 'coercion: world_tile_x is emitted as an integer'; restored byte-identical
2026-11-25T20:13:13Z dao-null-columns-dropped FIRED: test_model_gateway exit=1 on 'null: an air column stays null, not NAN'; restored byte-identical
2026-11-25T20:13:14Z dao-null-to-nan-dropped FIRED: test_model_gateway exit=1 on 'sample: a null sample becomes NAN'; restored byte-identical
2026-11-25T20:13:16Z dao-walker-unwrap-dropped FIRED: test_model_gateway exit=1 on 'step: the walker result is unwrapped from the response'; restored byte-identical
2026-11-25T20:13:17Z dao-terrain-coercion-dropped FIRED: test_model_gateway exit=1 on 'import: every listed integer field loses its decimal'; restored byte-identical
2026-11-25T20:13:19Z dao-repeat-nonce FIRED: test_model_gateway exit=1 on 'repeat: the same input serializes identically'; restored byte-identical
2026-11-25T20:13:19Z RESTORED_BYTE_IDENTICAL=yes
2026-11-25T20:13:22Z postflight: test_view_model_delegation exit=0, test_model_gateway exit=0
2026-11-25T20:13:22Z ALL_MUTANTS_FIRED=yes
```

## Gate measurements on the same date

- `cargo test -p sstd-core`: 160 passed / 0 failed, exit 0.
- `cargo test -p sstd-editor-bridge`: 35 passed / 0 failed, exit 0.
- `cargo test -p import-tiles`: 8 passed / 0 failed, exit 0.
- Full headless sweep of all 19 `editor/tests/test_*.gd` suites: 18 exit 0, one exit 1.
- `test_parallax_pixels` ran twice consecutively and exited 1 both times with
  `SCRIPT ERROR: Cannot call method 'get_width' on a null value.` and the matching `get_height`
  error. This is the pre-existing null-texture condition already recorded in wiki
  `TDD_Model-Migration-Rust` section 10; it is outside this change and is not claimed as flaky.
- `test_screen_store` ran twice consecutively and exited 0 both times. The issue #85 known-red was
  not reproduced on this host, so this suite is included in the 18 green suites rather than excluded.
- Logs: `/tmp/user/1000/opencode/sstd-s7-full-sweep.log` and
  `/tmp/user/1000/opencode/sstd-s7-{test_parallax_pixels,test_screen_store}-run{1,2}.log` (scratch).
  The counters above are re-derived from those logs, not asserted from a printed `failures=0` line.
