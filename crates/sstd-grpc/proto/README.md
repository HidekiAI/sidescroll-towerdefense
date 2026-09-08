# SSTD gRPC / Protobuf — Living Registry

> **Living document.** Every proto or gRPC change MUST update this table (new/edited
> row: purpose + status) AND the wiki index `TDD_gRPC-Service-Index` in the SAME commit.
> This file is the wire-level registry; the wiki index is the design-level map; the
> TDDs are the semantics. See the index page for the full 3-layer model
> (Service Contract > gRPC surface > Protobuf messages) and why gRPC != protobuf.

## Registry

| File | Package | Service(s) | RPCs | Status | What it does |
|------|---------|------------|------|--------|--------------|
| `editor.proto` | `sstd.editor` | `EditorService` | 2 (`SwitchTab`, `CaptureScreenshot`) | **implemented** | Headless editor control: switch tool tabs, capture tab screenshots (CI / agent workflows) |
| `agent_actions.proto` | `sstd.actions` | Entity/Behavior/Perception/Meta/Config/Camera/Admin | ~37 | designed (build with consumer) | Gameplay action surface: create/modify entities, behavior rules, LoS perception, meta/config/camera/admin |
| `spatial.proto` | `sstd.spatial` | `CoordinateService` + spatial queries | ~26 | designed | World<->screen<->tile coordinate conversion and grid queries (contiguous runs, clearance, stitching, adjacency, corridor) |
| `simulator.proto` | `sstd.sim` | `SimService` | ~4 (`Step`, `InjectEntity`, `QueryWorld`, `QueryCombatLog`) | designed (proto TBD) | Headless deterministic simulation steering for batch eval / ML training |
| `authoring.proto` | `sstd.authoring` | `AuthoringService` | 7 | designed (issue #69) | Agent-driven content generation via OpenRouter: tile + screen/world authoring, validation, budget, approval |
| `multiplayer.proto` | `sstd.multiplayer` | `GameAgent` | 1 (`Play` bidi stream) | designed | Bidirectional streaming transport for remote game agents |
| `common.proto` | `sstd.common` | — (message-only) | — | designed | Shared messages (`Action`, `ActionResult`, `ActionLog`, `Empty`, `StatusCode`, refs) reused by the services above |

## Conventions

- **Single schema source + single generated lib (enforced).** `proto/` is the ONLY
  schema directory in the repo; `sstd-grpc/build.rs` compiles every `*.proto` here
  via a glob (add a file -> it is built automatically, no Cargo.toml/protoc wiring,
  `cargo:rerun-if-changed` covers new/edited files). Consumers that need protobuf or
  gRPC types MUST depend on the `sstd-grpc` crate. NEVER hand-roll wire structs and
  NEVER re-generate `.rs` from `.proto` outside this crate. The GDExtension `#[func]`
  bridge + `EditorCommand` mpsc are a separate non-protobuf contract (JSON) and do
  not re-define the schema.
- **Packages** are dotted and lowercase: `sstd.<domain>` (matches existing `sstd.editor`).
- **Changes are additive only.** Never rename/renumber existing fields in a published
  proto; new fields use fresh field numbers; deprecated fields retire via `reserved`
  after a grace period. (No wire-breaking renames once a proto is in a released build.)
- **Codegen:** one `include_proto!` per package in `sstd-grpc`; `tonic-build` 0.12 +
  `prost-build` 0.13; protoc via `protobuf-src`. See `crates/sstd-grpc/build.rs`.
- **Server pattern:** every gRPC service ships as a thin tonic impl + a `*Command`
  mpsc bridge if it touches Godot (Godot APIs only on the main thread) —
  mirroring `EditorCommand`/`spawn_server`. Pure-Rust work (validation, storage,
  HTTP) runs on the background tokio thread.
- **Validation (CCMV):** every op validates against `sstd_config.sqlite3` constants
  first, then the Rust data model; responses are structured data, never raw display
  state. See `TDD_gRPC-Architecture`.
- **Protobuf != gRPC.** Messages (`message` blocks) are the serialization unit and
  may be reused outside gRPC (logs, config snapshots, state transfer). The `service`
  + `rpc` blocks are the gRPC surface. The Service Contract (auth/LoS/rate limits/
  engine dispatch) is a deeper, transport-agnostic layer that the GDExtension
  `#[func]` bridge and the `EditorCommand` mpsc channel also serve — without
  protobuf.
- **Test parity per service:** conversion test + mpsc round-trip + an e2e via
  `tonic::transport::Channel`; every transport action also writes an `action_log`
  row (journal-grade).