# PLAN: Rust-ify Simulator walker physics (#122)

Objective: move walker stepper + hill mechanics from GDScript (`world_view.gd::step_walker`) into `sstd-core` Rust so all sim state and stepping live on the Rust side. Godot keeps only GUI/visual/UI; data flows Rust ↔ Godot at the boundary via duplicated data structures (Dictionary on Godot side, structs on Rust side). Persisted state owned by Rust.

Status: plan. Prototype (flat ground + hill climb + fall damage) already working in GDScript. This is the Rust port plan.

## Data structures — duplicated on both sides

### Rust (`crates/sstd-core/src/physics.rs`)
```rust
pub struct WalkerState {
    pub pos_x: f64,   // world px
    pub pos_y: f64,   // world px (feet)
    pub vel_x: f64,   // pps
    pub vel_y: f64,   // pps
    pub grounded: bool,
    pub hp: f64,      // placeholder health
    pub fall_start_y: f64, // px where current fall began
}
pub struct WalkerInput {
    pub move_x: f64,      // -1..1
    pub move_y: f64,      // -1..1
    pub move_to_world: Option<(f64,f64)>,
    pub scroll_vec: Option<(f64,f64)>,
    pub place: bool,
    pub alt_place: bool,
    pub interact: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
}
```
WalkerState gains `hp` + `fall_start_y` vs current `physics.rs`. `step_walker` gains hill climb + fall damage to match the GDScript prototype contract (already written in `world_view.gd`).

### Godot (`world_view.gd::_walker_state`)
Dictionary mirroring Rust `WalkerState` keys: `pos_x`, `pos_y`, `vel_x`, `vel_y`, `grounded`, `hp`, `fall_start_y`. Built by GDScript from local state; passed into `step_walker` via Dictionary JSON over the bridge, read back into the Dictionary.

## Migration steps

### Step A: Port `step_walker` + hill mechanics into `sstd-core`
1. Add `hp: f64` and `fall_start_y: f64` to Rust `WalkerState`; default `hp = 100.0`, `fall_start_y = pos_y`.
2. Rewrite Rust `step_walker(state, input, dt, grid, tiles, speed_pps)` to match GDScript prototype contract:
   - gravity pulls down; horizontal move clamped to world edges;
   - **hill climb**: if moving horizontally and target column's surface is *higher* than feet, step up onto it (land immediately, `grounded=true`, reset `fall_start_y = feet_now_y`);
   - **hole**: no walkable tile in current column → fall; if previously grounded, record `fall_start_y = pos_y`;
   - **landing from above**: snap feet to surface, `grounded=true`, `vel_y=0`;
   - **fall damage**: if not previously grounded and surface is below `fall_start_y` by > 32 px, deduct 1 hp per 200 px dropped (hp clamped at 0).
3. Add Rust unit tests mirroring `test_walker_step`: hill climb onto higher surface, fall damage > threshold, fall damage < 32 px (none), determinism, hole fall-through, edge clamp.
4. Run `cargo test -p sstd-core`. Gate: 141 existing + N new, all pass.

### Step B: Make GDScript delegate to Rust stepper (not FFI per frame)
`world_view.gd::_update_walker(dt)` calls `_step_walker(state, ...)` (the GDScript static). Two options:
- **Option 1 (preferred for this release):** Keep the GDScript static as the single source of truth in-editor, but *identical* logic to Rust `step_walker`. Rust tests gate correctness; GDScript tests verify GDScript matches. No cross-process hop yet (GDExtension bridge already carries `step_walker(request_json) -> GString`; use it for the sim step).
- **Option 2 (future):** Route `_update_walker` through `world_view.gd::call_bridge_walk_step(...)` which serializes `WalkerInput` + `WalkerState` as JSON, calls `SstdBridge.step_sim(...)`, deserializes response into the Dictionary. This is the gRPC-ready shape but adds a hop per frame — exactly the "chunky round-trip" the placeholder TDD recommends against for per-frame stepping.

Decision: **Option 1 now** (pure GDScript static, logic mirrors Rust exactly; gate both sides with tests). Bridge path deferred until Sim gRPC service (#121) is real.

### Step C: Add gRPC proto for SimService.Step
Extend `crates/sstd-grpc/proto/editor.proto` (single schema source, `build.rs` glob compiles every `.proto`):
```proto
message WalkerState { double pos_x = 1; double pos_y = 2; double vel_x = 3; double vel_y = 4; bool grounded = 5; double hp = 6; double fall_start_y = 7; }
message WalkerInput { double move_x = 1; ... }
message StepSimRequest { WalkerInput input = 1; WalkerState state = 2; double dt = 3; GridConfig grid = 4; repeated Tile tiles = 5; double speed_pps = 6; }
message StepSimResponse { WalkerState state = 7; }
service SimService { rpc StepSim(StepSimRequest) returns (StepSimResponse); }
```
Per the single-schema rule: ship `.proto`, `include_proto!()` row, `proto/README.md` row, and `TDD_gRPC-Service-Index` entry in ONE commit.

### Step D: Update TDD docs
- `TDD_Simulator-Ground-Walk.md`: record hill walkability (done) + fall-damage semantics + Rust-side ownership note.
- `TDD_Simulator-Service-Contract.md`: replace placeholder sections with the real `SimService.Step` proto shape and the Godot hop decision.

### Step E: Tests
- Rust: `cargo test -p sstd-core` — hill climb, fall damage, hole fall, determinism (all new).
- GDScript: `test_walker_step` hill_climb + hill_fall_damage guards pass; `test_walker_visibility` still green; all four standing suites green.

### Dependencies / nothing new
No new crates: `sstd-core` already has `physics.rs`, `WalkerState`, `WalkerInput`, `GridConfig`, `Tile`. Just extend them.

## Verification gate
`cargo test -p sstd-core` (141 + new) AND `$HOME/bin/godot4 --headless --path editor --script res://tests/test_walker_step.gd` AND `test_walker_visibility` all exit 0.
