# Project: SSTD (Sidescroll Tower Defense)

## gRPC Integration

- gRPC server (sstd-grpc crate) runs on a background tokio thread
- Shared `Arc<RwLock<EditorState>>` between bridge and gRPC handler
- State lock is `EditorState` — single shared struct for reads/writes from both GDScript and gRPC
- No separate authorization/API key layer — gRPC port is localhost-only by default
- For headless CI: `sstd-headless` binary starts gRPC server without Godot UI

## LUCK Stat (Phase 2 Consideration)

- If LUCK is added as a stat, use a **bounded modifier** (`±LUCK%` to damage, no roll) to preserve deterministic feel
- Decided against dice (2D20 or flat random) — SSTD is strategic/no-RNG; even a bell-curve roll undermines placement strategy
- LUCK acts as a predictable fudge factor: `final_damage × (1 ± LUCK%)`, capped at reasonable bounds (e.g., ±25%)

## Pathfinding

- **No A*** — SSTD is a side-scroller; corridors are linear with occasional 2-3 way forks
- At fork joints, AI picks by priority rule (nearest enemy, weakest enemy, waypoint-route priority)
- Linear corridor segments: entities just walk forward, no pathfinding needed
- Waypoint-route system: entities follow waypoint chains; at junctions, fork-priority AI decides which branch to take
