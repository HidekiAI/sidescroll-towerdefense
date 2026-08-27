# PLAN — #64 Phase 2: gRPC Editor Control (tonic server + bridge wiring)

> Tracker: [#64](https://github.com/HidekiAI/sidescroll-towerdefense/issues/64)
> Wiki design truth: [TDD_GRPC-Editor-Control](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TechnicalDesign/TDD_GRPC-Editor-Control) (authoritative)
> Status: preplan (documented BEFORE coding, per AGENTS.md documentation-first workflow).

## Objective

Let external gRPC clients (CI scripts, LLM agents) **switch editor tabs** and
**capture viewport screenshots** against the running Godot editor. Godot cannot
host gRPC (its stack is HTTP/1.1), so the tonic gRPC server lives inside the
Rust GDExtension bridge (`sstd-editor-bridge`, the `.so` already in-process),
dispatching commands to the Godot main thread via Rust channels.

## Why a tonic server (the design burden)

- gRPC = HTTP/2. Godot's `HTTPClient`/`WebSocketPeer` are HTTP/1.1 -> Godot can
  neither host nor dial a gRPC endpoint natively.
- The bridge `.so` is compiled Rust already linked into the Godot process -> can
  host a tokio/tonic server on a background thread with full access to channels.
- Tab switch + viewport capture must run on the Godot main thread (Godot is not
  thread-safe). So the architecture is a **command bridge**:
  `tonic(bg) --mpsc--> SstdBridge._process() --oneshot--> tonic response`.

## Existing structures this builds on (grounding)

- `SstdBridge` (crates/sstd-editor-bridge/src/lib.rs): godot-rust `#[derive(GodotClass)]
  #[class(init, base=Node)] struct` + `#[godot_api] impl` with existing `#[func]`
  methods (validate/import/screen). New `#[func]`s live in the same impl.
  `SstdBridge` currently holds only `base: Base<Node>` + `config_store: Option<ConfigStore>`.
- `main.gd`: `_load_bridge()` instantiates `SstdBridge` and `add_child`s it;
  `_ready()` is where bridge wiring happens. `tabs: TabContainer` is `$TabContainer`
  with `TAB_NAMES` const and `current_tab`. `get_viewport()` is the root viewport.
- Workspace `Cargo.toml`: members `sstd-core`, `sstd-editor-bridge`, `tools/import-tiles`.
- godot-rust version: `godot = "0.5"` (api-4-4).

## Design decisions (resolved here to avoid a probe)

1. **New crate `sstd-grpc`** owns proto + server + command channel types
   (`EditorCommand`, `SwitchTabResult`, `ScreenshotResult`). Bridge depends on it.
   Keeps tonic dep out of per-build bridge compile; bridge is a `cdylib`.
2. **Native bridge methods `switch_tab`/`capture_screenshot`**: synchronous
   `#[func]`s that run directly on the Godot main thread and return a JSON string
   (matching the existing bridge convention of strings/PackedByteArray over serde).
   These are ALSO the body Phase 2 split-off option — usable by headless tests and
   by `poll_grpc_commands`. They do NOT depend on tonic.
3. **`start_grpc_server(port)`**: spawns a tokio runtime on a background thread
   hosting tonic; stores an `mpsc::Sender<EditorCommand>`. Returns `{ok, error}`.
   Command receiver is drained by `poll_grpc_commands()` on the main thread.
4. **`poll_grpc_commands()`**: `#[func]` called every frame from `main.gd._process`;
   drains the mpsc receiver (try_recv), executes each command via the native
   methods above, sends results back via oneshot. Never blocks the main thread.
5. **Command channel / Godot-safety invariant**: tonic bg thread only ever touches
   channels + its own state; every Godot API call happens inside poll on main thread.
   `SstdBridge` mutability: `poll_grpc_commands` mutates bridge state -> stored
   fields for `Base<Node>` need interior mutability or `GodotClass` requires the
   receiver to live in a `RefCell`/`Mutex`. Needed fields:
   `tab_container: Option<Gd<TabContainer>>`, `grpc_rx: Option<mpsc::Receiver<EditorCommand>>`,
   `grpc_tx: Option<mpsc::Sender<EditorCommand>>`. godot-rust `#[derive(GodotClass)]`
   requires all fields safe to move; `tokio::sync::mpsc` is Send, `Gd<TabContainer>`
   is not Sync -> wrap channel pieces as needed. Pending confirmation of the exact
   godot-rust 0.5 field rules during implementation.
6. **Proto location**: `crates/sstd-grpc/proto/editor.proto` per TDD. Build via
   `tonic-build` + `prost-build` with **vendored protoc** (`features=["vendored"]`)
   since this host has no system `protoc`.

## Async-thread reality (constraint)

Godot main thread cannot sleep/yield to the bridge on a long RPC. Implementation
is strictly non-blocking: tonic handler pushes command + awaits oneshot (off main
thread); main thread fulfils within a frame. A disconnect/timeout in the client
drops the oneshot sender; the receiver half resolves Err -> the server returns an
error gRPC status. No deadlock because mpsc is bounded and the server never touches
Godot.

## Display constraint (carried from Phase 1)

`get_viewport().get_texture()` is null under `--headless` (Dummy renderer). So
`capture_screenshot` returns an explicit error in headless mode. Valid in a real
display session (DISPLAY=:0). `switch_tab` works regardless of display.

## Files to create/change

- NEW `crates/sstd-grpc/Cargo.toml`, `build.rs`, `proto/editor.proto`,
  `src/lib.rs` (service trait, generated types, `EditorServer`, command types).
- `crates/sstd-editor-bridge/Cargo.toml`: add `sstd-grpc`, `tokio` (sync feature),
  `crossbeam-channel` or `std::sync::mpsc` receiver drain (prefer std/channel to
  avoid extra dep if godot-rust allows).
- `crates/sstd-editor-bridge/src/lib.rs`: `set_tab_container`, `switch_tab`,
  `capture_screenshot`, `start_grpc_server`, `poll_grpc_commands`.
- `editor/scripts/main.gd`: `_bridge.set_tab_container(tabs)`,
  `var r = _bridge.start_grpc_server(50051)`, new `_process(delta)` drains poll.
- `Cargo.toml` (workspace): add `crates/sstd-grpc`.
- Wiki `TDD_GRPC-Editor-Control.md`: update Phase 2 section with concrete
  implementation details + vendored protoc note. TODO TS45.

## Verification

- `cargo build -p sstd-editor-bridge` green; `cargo test` workspace green.
- Headless smoke: `godot4 --headless --path editor --quit-after 120`
  -- verify bridge loads, gRPC server starts `{ok:true}`, no crash.
- Real-display run: start editor, `grpcurl -plaintext -d '{"tab_name":"simulator"}'`
  returns ok+tab_index; `CaptureScreenshot` returns PNG bytes (file -> "PNG image data").
- Unit tests: sstd-grpc server route mapping test (tab_name -> index) + a
  command-channel round trip test independent of Godot.

## Done criteria (close #64 when all pass)

- Phase 1 capture (done) + Phase 2 gRPC switch/capture both working.
- `switch_tab` maps the 5 TAB_NAMES; unknown name -> `{ok:false,error}`.
- `capture_screenshot` returns valid PNG on a real display; explicit error headless.
- Wiki TDD reconciled with shipped implementation; issue #64 closed.
