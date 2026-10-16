use std::path::Path;

use godot::classes::{TabContainer, Texture2D};
use godot::prelude::*;
use serde::{Deserialize, Serialize};
use sstd_core::config::ConfigStore;
use sstd_core::parallax::{place_layer, Anchor, Fill, LayerArt, Plane, Stack, ViewState};
use sstd_core::storage::{EntityDefsFile, ScreenFile, TerrainTypesFile};
use sstd_core::terrain::GridConfig;
use sstd_grpc::{spawn_server, EditorCommand, ScreenshotResult, SwitchTabResult};
use tokio::sync::mpsc;

struct SstdEditorBridge;

#[gdextension]
unsafe impl ExtensionLibrary for SstdEditorBridge {}

/// Base64-encode raw bytes for JSON transport. Uses the standard (padded)
/// alphabet from the `base64` crate.
fn base64_encode(bytes: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    STANDARD.encode(bytes)
}

/// Editor control state shared between GDScript (`main.gd`) and the in-process
/// gRPC server (see #64). The gRPC background thread only holds an mpsc
/// `Sender`; the `Receiver` and Godot `TabContainer` lives here on the main
/// thread and are drained/consumed by the `#[func]` methods below.
#[derive(GodotClass)]
#[class(init, base=Node)]
struct SstdBridge {
    base: Base<Node>,
    config_store: Option<ConfigStore>,
    grpc_tx: Option<mpsc::Sender<EditorCommand>>,
    grpc_rx: Option<mpsc::Receiver<EditorCommand>>,
    tab_container: Option<Gd<TabContainer>>,
}

#[godot_api]
impl SstdBridge {
    #[func]
    fn validate_placement(
        &self,
        map_json: GString,
        entity_key: GString,
        tile_x: i32,
        tile_y: i32,
    ) -> GString {
        let result =
            validate_placement_impl(map_json.to_string(), entity_key.to_string(), tile_x, tile_y);
        GString::from(result.as_str())
    }

    #[func]
    fn import_terrain_types(&self, json: GString) -> GString {
        let result = match TerrainTypesFile::from_json(&json.to_string()) {
            Ok(file) => serde_json::to_string_pretty(&file)
                .unwrap_or_else(|e| format!("{{\"error\": \"serialize failed: {}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn import_entity_defs(&self, json: GString) -> GString {
        let result = match EntityDefsFile::from_json(&json.to_string()) {
            Ok(file) => serde_json::to_string_pretty(&file)
                .unwrap_or_else(|e| format!("{{\"error\": \"serialize failed: {}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn import_screen(&self, json: GString) -> GString {
        let result = match ScreenFile::from_json(&json.to_string()) {
            Ok(file) => serde_json::to_string_pretty(&file)
                .unwrap_or_else(|e| format!("{{\"error\": \"serialize failed: {}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn validate_screen(&self, json: GString) -> GString {
        let result = match ScreenFile::from_json(&json.to_string()) {
            Ok(file) => {
                let validation = file.validate();
                if validation.is_valid() {
                    "{\"valid\": true}".to_string()
                } else {
                    format!(
                        "{{\"valid\": false, \"errors\": {}}}",
                        serde_json::to_string(&validation.messages)
                            .unwrap_or_else(|_| { "[]".to_string() })
                    )
                }
            }
            Err(e) => format!("{{\"valid\": false, \"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    /// Place this frame's backdrop layers for a `(view, anchor, stack, art, plane)` request.
    ///
    /// Returns `{"placements": [...]}` on success, `{"error": "<detail>"}` for a malformed
    /// request, or `{"error": "stack_invalid", "errors": [...]}` when the stack fails
    /// validation. The math lives in `sstd-core::parallax`; this method only composes the
    /// request and the response.
    #[func]
    fn parallax_placements(&self, request_json: GString) -> GString {
        GString::from(parallax_placements_impl(&request_json.to_string()).as_str())
    }

    /// Record the editor's `TabContainer` so the gRPC `SwitchTab`/`CaptureScreenshot`
    /// handlers can reach the real tabs. Must be called from `main.gd._ready`.
    #[func]
    fn set_tab_container(&mut self, tab_container: Gd<TabContainer>) {
        self.tab_container = Some(tab_container);
        godot_print!("[sstd-bridge] grpc: tab_container registered");
    }

    /// Apply the resolved tab name to the editor UI. Runs on the Godot main thread.
    /// Returns `{"ok":true,"tab_index":N}` or `{"ok":false,"error":...}`.
    fn switch_tab_impl(&mut self, tab_name: &str) -> SwitchTabResult {
        let idx = match sstd_grpc::resolve_tab_index(tab_name) {
            Some(i) => i,
            None => {
                return SwitchTabResult {
                    ok: false,
                    tab_index: -1,
                    error: format!("unknown tab: {tab_name}"),
                }
            }
        };
        match self.tab_container.as_mut() {
            Some(tabs) => {
                tabs.set_current_tab(idx);
                SwitchTabResult {
                    ok: true,
                    tab_index: idx,
                    error: String::new(),
                }
            }
            None => SwitchTabResult {
                ok: false,
                tab_index: -1,
                error: "tab_container not set; call set_tab_container first".to_string(),
            },
        }
    }

    /// gRPC-exposed tab switch. Returns a JSON string.
    #[func]
    fn switch_tab(&mut self, tab_name: GString) -> GString {
        let r = self.switch_tab_impl(&tab_name.to_string());
        let json = if r.ok {
            format!("{{\"ok\": true, \"tab_index\": {}}}", r.tab_index)
        } else {
            format!("{{\"ok\": false, \"error\": \"{}\"}}", r.error)
        };
        GString::from(json.as_str())
    }

    /// Capture a PNG of the root viewport. Returns
    /// `{"ok":true,"png":<base64>,"width":W,"height":H}` or
    /// `{"ok":false,"error":...}`. Fails in headless (Dummy) mode because the
    /// viewport texture is null (see #64 Phase 1).
    fn capture_screenshot_impl(&mut self) -> Result<ScreenshotResult, String> {
        let tabs = match self.tab_container.as_ref() {
            Some(t) => t,
            None => return Err("tab_container not set; call set_tab_container first".to_string()),
        };
        let viewport = match tabs.get_viewport() {
            Some(v) => v,
            None => return Err("no viewport available from tab_container".to_string()),
        };
        let texture = match viewport.get_texture() {
            Some(t) => t.upcast::<Texture2D>(),
            None => {
                return Err(
                    "viewport texture is null (headless/Dummy renderer cannot capture)".to_string(),
                )
            }
        };
        let image = match texture.get_image() {
            Some(img) => img,
            None => return Err("could not read viewport texture as image".to_string()),
        };
        let w = image.get_width();
        let h = image.get_height();
        let png = image.save_png_to_buffer();
        if png.is_empty() {
            return Err("save_png_to_buffer returned empty (headless/Dummy renderer)".to_string());
        }
        Ok(ScreenshotResult {
            png_data: png.to_vec(),
            width: w,
            height: h,
        })
    }

    /// gRPC-exposed screenshot. Returns JSON with base64 PNG.
    #[func]
    fn capture_screenshot(&mut self) -> GString {
        let r = match self.capture_screenshot_impl() {
            Ok(r) => r,
            Err(e) => {
                return GString::from(format!("{{\"ok\": false, \"error\": \"{e}\"}}").as_str())
            }
        };
        let b64 = base64_encode(&r.png_data);
        let json = format!(
            "{{\"ok\": true, \"width\": {}, \"height\": {}, \"png_size\": {}, \"png\": \"{}\"}}",
            r.width,
            r.height,
            r.png_data.len(),
            b64
        );
        GString::from(json.as_str())
    }

    /// Start the tonic gRPC server on the given port (background thread).
    /// Returns `{"ok":true,"port":N}` or `{"ok":false,"error":...}`.
    #[func]
    fn start_grpc_server(&mut self, port: i32) -> GString {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port as u16);
        match spawn_server(addr) {
            Ok(rx) => {
                self.grpc_tx = None; // Sender is owned by the spawned server.
                self.grpc_rx = Some(rx);
                godot_print!("[sstd-bridge] grpc server listening on {}", addr);
                GString::from(format!("{{\"ok\": true, \"port\": {}}}", port).as_str())
            }
            Err(e) => GString::from(format!("{{\"ok\": false, \"error\": \"{}\"}}", e).as_str()),
        }
    }

    /// Drain pending gRPC commands and fulfil them on the Godot main thread.
    /// Call every frame from `main.gd._process`. Returns count of commands handled.
    #[func]
    fn poll_grpc_commands(&mut self) -> i32 {
        let mut handled = 0;
        loop {
            // Take the command out of the channel (owned) so the borrow of
            // `self.grpc_rx` ends before the `&mut self` impl calls below.
            let cmd = match self.grpc_rx.as_mut() {
                Some(rx) => match rx.try_recv() {
                    Ok(c) => c,
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        self.grpc_rx = None;
                        godot_print!("[sstd-bridge] grpc command channel disconnected");
                        break;
                    }
                },
                None => break,
            };
            match cmd {
                EditorCommand::SwitchTab { tab_name, reply } => {
                    let r = self.switch_tab_impl(&tab_name);
                    let _ = reply.send(Ok(r));
                }
                EditorCommand::CaptureScreenshot { reply } => {
                    let r = self.capture_screenshot_impl();
                    let _ = reply.send(r);
                }
            }
            handled += 1;
        }
        handled
    }

    #[func]
    fn init_config_db(&mut self, path: GString) -> GString {
        let result = match ConfigStore::open_or_create(Path::new(&path.to_string())) {
            Ok(store) => {
                self.config_store = Some(store);
                "{\"ok\": true}".to_string()
            }
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn set_tile_set_category(&self, tile_set_key: GString, terrain_key: GString) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from("{\"error\": \"config store not initialized\"}"),
        };
        match store.set_tile_set_category(&tile_set_key.to_string(), &terrain_key.to_string()) {
            Ok(_) => GString::from("{\"ok\": true}"),
            Err(e) => GString::from(format!("{{\"error\": \"{}\"}}", e).as_str()),
        }
    }

    #[func]
    fn remove_tile_set_category(&self, tile_set_key: GString, terrain_key: GString) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from("{\"error\": \"config store not initialized\"}"),
        };
        match store.remove_tile_set_category(&tile_set_key.to_string(), &terrain_key.to_string()) {
            Ok(_) => GString::from("{\"ok\": true}"),
            Err(e) => GString::from(format!("{{\"error\": \"{}\"}}", e).as_str()),
        }
    }

    #[func]
    fn get_tile_sets_for_terrain(&self, terrain_key: GString) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from("[]"),
        };
        let result = match store.get_tile_sets_for_terrain(&terrain_key.to_string()) {
            Ok(keys) => serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string()),
            Err(_) => "[]".to_string(),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn get_all_tile_set_categories(&self) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from("{}"),
        };
        let result = match store.get_all_tile_set_categories() {
            Ok(entries) => {
                let mut map: std::collections::BTreeMap<String, Vec<String>> =
                    std::collections::BTreeMap::new();
                for (ts_key, t_key) in &entries {
                    map.entry(ts_key.clone()).or_default().push(t_key.clone());
                }
                serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
            }
            Err(_) => "{}".to_string(),
        };
        GString::from(result.as_str())
    }

    #[func]
    fn set_config_value(&self, key: GString, value: GString) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from("{\"error\": \"config store not initialized\"}"),
        };
        match store.set_config(&key.to_string(), &value.to_string()) {
            Ok(_) => GString::from("{\"ok\": true}"),
            Err(e) => GString::from(format!("{{\"error\": \"{}\"}}", e).as_str()),
        }
    }

    #[func]
    fn get_config_value(&self, key: GString) -> GString {
        let store = match &self.config_store {
            Some(s) => s,
            None => return GString::from(""),
        };
        match store.get_str(&key.to_string()) {
            Ok(v) => GString::from(v.as_str()),
            Err(_) => GString::from(""),
        }
    }

    #[func]
    fn get_grid_config(&self) -> GString {
        let config = match &self.config_store {
            Some(store) => store.grid_config().ok().unwrap_or_default(),
            None => GridConfig::default(),
        };
        GString::from(
            serde_json::to_string(&config)
                .unwrap_or_else(|_| "{\"error\": \"serialize failed\"}".to_string())
                .as_str(),
        )
    }

    #[func]
    fn check_dimension_compatibility(&self, screen_json: GString) -> GString {
        let current = match &self.config_store {
            Some(store) => store.grid_config().ok().unwrap_or_default(),
            None => GridConfig::default(),
        };
        let result = match ScreenFile::from_json(&screen_json.to_string()) {
            Ok(file) => {
                let same_w = file.tile_width_px == current.tile_width_in_pixels;
                let same_h = file.tile_height_px == current.tile_height_in_pixels;
                if same_w && same_h {
                    format!(
                        "{{\"compatible\": true, \"saved_px\": {{\"w\": {}, \"h\": {}}}, \"current_px\": {{\"w\": {}, \"h\": {}}}}}",
                        file.tile_width_px, file.tile_height_px,
                        current.tile_width_in_pixels, current.tile_height_in_pixels,
                    )
                } else {
                    format!(
                        "{{\"compatible\": false, \"saved_px\": {{\"w\": {}, \"h\": {}}}, \"current_px\": {{\"w\": {}, \"h\": {}}}, \"message\": \"Tile dimensions changed from {}x{} to {}x{}. Use porting to rescale.\"}}",
                        file.tile_width_px, file.tile_height_px,
                        current.tile_width_in_pixels, current.tile_height_in_pixels,
                        file.tile_width_px, file.tile_height_px,
                        current.tile_width_in_pixels, current.tile_height_in_pixels,
                    )
                }
            }
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        };
        GString::from(result.as_str())
    }

    /// Native parallel exact-diff over gate-surviving duplicate pairs.
    ///
    /// `variants_flat` is the concatenation of every tile's 4 flip variants
    /// (N * 4 * 4096 bytes, variant order: identity, h-flip, v-flip, hv-flip).
    /// `bytes_flat` is the concatenation of every tile's canonical RGBA bytes
    /// (N * 4096). `pairs` is a flat [base, candidate] index stream. Returns a
    /// flat [base, candidate, flip_h, flip_v] match stream. The scan splits
    /// the pair range across `worker_count` native threads (no GDScript
    /// serialization of packed-array access, which the measured 0.72x result
    /// showed is the GDScript bottleneck).
    #[func]
    fn scan_exact_matches(
        &self,
        variants_flat: PackedByteArray,
        bytes_flat: PackedByteArray,
        pairs: PackedInt32Array,
        tolerance: f64,
        worker_count: i32,
    ) -> PackedInt32Array {
        let variants = variants_flat.to_vec();
        let bytes = bytes_flat.to_vec();
        let pair_stream = pairs.to_vec();
        let matches =
            scan_exact_matches_impl(&variants, &bytes, &pair_stream, tolerance, worker_count);
        PackedInt32Array::from(matches)
    }

    /// Native parallel construction of the 4 RGBA flip variants for every tile.
    /// `bytes_flat` is the concatenation of every tile's canonical RGBA bytes
    /// (N * 4096). Returns the concatenation of every tile's variants
    /// (N * 4 * 4096, order: identity, h-flip, v-flip, hv-flip) — the same
    /// layout GDScript's `_flip_variants` builds, but without the per-tile
    /// GDScript loop (measured ~5s on the real catalog).
    #[func]
    fn build_variants(&self, bytes_flat: PackedByteArray, worker_count: i32) -> PackedByteArray {
        let bytes = bytes_flat.to_vec();
        let variants = build_variants_impl(&bytes, worker_count);
        PackedByteArray::from(variants)
    }

    /// Native parallel projection-window scan + grayscale-signature gate.
    /// `sigs_flat` is the concatenation of every tile's 32-byte grayscale
    /// signature (N * 32). `ordered_projections` and `ordered_indices` are the
    /// catalog positions sorted by (projection, key): projections in ascending
    /// order with the matching tile index per slot. Returns a flat [base,
    /// candidate] stream of pairs that pass the `_grayscale_sig_near` gate
    /// (diff <= 96), skipping pairs outside the projection window (delta).
    #[func]
    fn scan_gate_pairs(
        &self,
        sigs_flat: PackedByteArray,
        ordered_projections: PackedInt32Array,
        ordered_indices: PackedInt32Array,
        max_delta: i32,
        worker_count: i32,
    ) -> PackedInt32Array {
        let sigs = sigs_flat.to_vec();
        let projections = ordered_projections.to_vec();
        let indices = ordered_indices.to_vec();
        let pairs = scan_gate_pairs_impl(&sigs, &projections, &indices, max_delta, worker_count);
        PackedInt32Array::from(pairs)
    }

    /// Native enumeration of the one-unit neighbour FNV hashes of a
    /// canonical-coarse signature. `canon` is the 256-byte canonical coarse
    /// signature of one tile. For each of the 256 bytes, the byte perturbed by
    /// -1 and +1 is hashed; the distinct hash values are returned as i64 to
    /// match GDScript's 64-bit integer dictionary keys (`_stamp_fp_index`).
    /// Ports the GDScript `_a3_neighbor_hashes` + `_fp_hash` hot loop (~90% of
    /// A3 wall time on the real catalog).
    #[func]
    fn a3_neighbor_hashes(&self, canon: PackedByteArray) -> PackedInt64Array {
        let canon_vec = canon.to_vec();
        let hashes = a3_neighbor_hashes_impl(&canon_vec);
        PackedInt64Array::from(hashes)
    }

    /// Native port of `_flip_of_bytes`: given the canonical 4096-byte RGBA
    /// buffers of a representative and a candidate, return the winning flip as
    /// 0 (no match), 1 (identity), 2 (h-flip), 4 (v-flip), or 8 (hv-flip).
    #[func]
    fn flip_of_bytes(&self, base: PackedByteArray, cand: PackedByteArray, tolerance: f64) -> i32 {
        let base_vec = base.to_vec();
        let cand_vec = cand.to_vec();
        if base_vec.len() != TILE_BYTES || cand_vec.len() != TILE_BYTES {
            return 0;
        }
        flip_of_bytes_impl(&base_vec, &cand_vec, tolerance) as i32
    }

    /// Native front-end for the prune scan (#53): given the concatenated
    /// 4096-byte RGBA buffers (N * TILE_BYTES, row-major 32x32), compute each
    /// tile's 32-byte grayscale signature in one pass. Replaces the GDScript
    /// `_grayscale_sig` hot loop (half of the ~2.2s prep on the real catalog).
    #[func]
    fn scan_signatures(&self, bytes_flat: PackedByteArray) -> PackedByteArray {
        let bytes = bytes_flat.to_vec();
        if bytes.is_empty() || bytes.len() % TILE_BYTES != 0 {
            return PackedByteArray::new();
        }
        let n = bytes.len() / TILE_BYTES;
        let mut sigs_flat = Vec::with_capacity(SIG_BYTES * n);
        for i in 0..n {
            sigs_flat.extend_from_slice(&grayscale_signature_impl(
                &bytes[i * TILE_BYTES..(i + 1) * TILE_BYTES],
            ));
        }
        PackedByteArray::from(sigs_flat)
    }

    /// Native front-end for the prune scan (#53): each tile's luminance
    /// projection (`_grayscale_luminance_projection`) over the same flat RGBA
    /// buffer, index-aligned with `scan_signatures`.
    #[func]
    fn scan_projections(&self, bytes_flat: PackedByteArray) -> PackedInt32Array {
        let bytes = bytes_flat.to_vec();
        if bytes.is_empty() || bytes.len() % TILE_BYTES != 0 {
            return PackedInt32Array::new();
        }
        let n = bytes.len() / TILE_BYTES;
        let mut projections = Vec::with_capacity(n);
        for i in 0..n {
            projections.push(luminance_projection_impl(
                &bytes[i * TILE_BYTES..(i + 1) * TILE_BYTES],
            ));
        }
        PackedInt32Array::from(projections)
    }

    /// Native bulk canonical-coarse (#59): given the concatenated 4096-byte RGBA
    /// buffers (N * TILE_BYTES), compute each tile's canonical coarse signature
    /// (N * COARSE_BYTES) in one pass. Ports `_coarse_bytes_rgba` + `_canonical_of_coarse`
    /// (min of identity / h-flip / v-flip / hv-flip under lexicographic `<`).
    #[func]
    fn scan_canonical_coarse(&self, bytes_flat: PackedByteArray) -> PackedByteArray {
        let bytes = bytes_flat.to_vec();
        if bytes.is_empty() || bytes.len() % TILE_BYTES != 0 {
            return PackedByteArray::new();
        }
        let n = bytes.len() / TILE_BYTES;
        let mut canon_flat = Vec::with_capacity(COARSE_BYTES * n);
        for i in 0..n {
            canon_flat.extend_from_slice(&canonical_coarse_impl(
                &bytes[i * TILE_BYTES..(i + 1) * TILE_BYTES],
            ));
        }
        PackedByteArray::from(canon_flat)
    }
}

/// Port of the GDScript `_a3_neighbor_hashes`: for each of the 256 canonical
/// coarse bytes, perturb by -1 and +1 (clamped) and FNV-hash the resulting
/// signature; return the distinct hash values.
fn a3_neighbor_hashes_impl(canon: &[u8]) -> Vec<i64> {
    if canon.len() != COARSE_BYTES {
        return Vec::new();
    }
    let mut seen: Vec<i64> = Vec::with_capacity(COARSE_BYTES * 2);
    // Reuse one scratch buffer instead of cloning a fresh 256-byte Vec per
    // perturbed byte (the GDScript version allocates 512 arrays per tile).
    let mut s: Vec<u8> = canon.to_vec();
    for i in 0..COARSE_BYTES {
        for delta in [-1i64, 1i64] {
            let original = s[i];
            s[i] = (canon[i] as i64 + delta).clamp(0, 255) as u8;
            let h = fp_hash(&s) as i64;
            if !seen.contains(&h) {
                seen.push(h);
            }
            s[i] = original;
        }
    }
    seen
}

fn validate_placement_impl(
    map_json: String,
    entity_key: String,
    tile_x: i32,
    tile_y: i32,
) -> String {
    let screen: ScreenFile = match serde_json::from_str(&map_json) {
        Ok(s) => s,
        Err(e) => return format!("{{\"valid\": false, \"error\": \"{}\"}}", e),
    };

    if tile_x < 0 || tile_x >= screen.width_tiles {
        return format!(
            "{{\"valid\": false, \"error\": \"tile_x {} out of bounds (0-{})\"}}",
            tile_x,
            screen.width_tiles - 1
        );
    }

    if tile_y < 0 || tile_y >= screen.height_tiles {
        return format!(
            "{{\"valid\": false, \"error\": \"tile_y {} out of bounds (0-{})\"}}",
            tile_y,
            screen.height_tiles - 1
        );
    }

    for placed in &screen.placed_entities {
        if placed.entity_key == entity_key
            && placed.world_tile_x == tile_x
            && placed.world_tile_y == tile_y
        {
            return format!(
                "{{\"valid\": false, \"error\": \"entity '{}' already placed at ({}, {})\"}}",
                entity_key, tile_x, tile_y
            );
        }
    }

    format!(
        "{{\"valid\": true, \"entity\": \"{}\", \"tile_x\": {}, \"tile_y\": {}}}",
        entity_key, tile_x, tile_y
    )
}

// ---------------------------------------------------------------------------
// parallax_placements — the `view` / `art` halves of the request.
//
// `Stack`, `Layer`, `Anchor` and `Plane` carry serde derives and deserialize straight out of
// the request. `ViewState` and `LayerArt` deliberately do not: `sstd-core` has no reason to
// know this wire form (`parallax.rs`: "The caller measures these ... this module must not gain
// an image dependency"), so the bridge owns the two input structs below and maps them.
// ---------------------------------------------------------------------------

/// The camera half of a `parallax_placements` request.
#[derive(Deserialize)]
struct ViewInput {
    cam_center_world_px_x: f64,
    cam_center_world_px_y: f64,
    zoom: f64,
    view_w_px: f64,
    view_h_px: f64,
}

/// One layer's measured PNG facts, index-aligned with `stack.layers`. Exactly `LayerArt`'s
/// fields; the values come from the PNG decoder the caller already has (step 6's loader).
#[derive(Deserialize)]
struct LayerArtInput {
    width_px: u32,
    height_px: u32,
    is_rgba: bool,
    fully_opaque: bool,
    top_row_opaque: bool,
    bottom_row_opaque: bool,
}

/// A `parallax_placements` request. Only `plane` defaults: the anchor is required, because
/// choosing a default anchor is the loader's job (step 6, `TDD_Parallax-Scrolling.Data`
/// section 3), not this seam's.
#[derive(Deserialize)]
struct PlacementsRequest {
    view: ViewInput,
    anchor: Anchor,
    stack: Stack,
    art: Vec<LayerArtInput>,
    #[serde(default)]
    plane: Option<Plane>,
}

/// One layer as GDScript draws it: the identity of the layer, then the four values
/// `place_layer` computed, then what the drawer needs and cannot re-derive (the repeat period,
/// the art height, the two edge fills).
#[derive(Serialize)]
struct PlacementEntry {
    name: String,
    file: String,
    plane: Plane,
    draw_order: i32,
    first_copy_left_px: i32,
    copy_count: u32,
    screen_top_px: i32,
    scale: f64,
    period_px: f64,
    art_h_px: f64,
    fill_above: Fill,
    fill_below: Fill,
}

#[derive(Serialize)]
struct PlacementsResponse {
    placements: Vec<PlacementEntry>,
}

/// Every back-plane layer draws before every front-plane layer; within a plane, ascending
/// `draw_order` (`TDD_Parallax-Scrolling.Math` section 7).
fn plane_rank(plane: Plane) -> u8 {
    match plane {
        Plane::Back => 0,
        Plane::Front => 1,
    }
}

/// `{"error": "<detail>"}` for a request-level failure, matching `import_screen`.
///
/// Built through the `json!` macro rather than `format!`: a detail carrying a quote would
/// otherwise produce malformed JSON, and a serde parse error routinely does.
fn request_error_json(detail: &str) -> String {
    serde_json::json!({ "error": detail }).to_string()
}

/// The request-level hazards `Stack::validate` cannot see. It never looks at the view, and it
/// has no width or height rule, so a zero period or a zero zoom would reach the repeat in
/// `place_layer`, produce NaN, and saturate `NaN as u32` to `copy_count = 0` — a silently
/// empty draw instead of a diagnosable failure.
fn parallax_request_error(request: &PlacementsRequest) -> Option<String> {
    let view = &request.view;
    // Reject non-finite and non-positive values (NaN and +Inf both fail this).
    if !view.zoom.is_finite() || view.zoom <= 0.0 {
        return Some(format!(
            "view.zoom must be > 0 and finite, got {}",
            view.zoom
        ));
    }
    if !view.view_w_px.is_finite() || view.view_w_px <= 0.0 {
        return Some(format!(
            "view.view_w_px must be > 0 and finite, got {}",
            view.view_w_px
        ));
    }
    if !view.view_h_px.is_finite() || view.view_h_px <= 0.0 {
        return Some(format!(
            "view.view_h_px must be > 0 and finite, got {}",
            view.view_h_px
        ));
    }
    for (layer_index, facts) in request.art.iter().enumerate() {
        if facts.width_px == 0 {
            return Some(format!("art[{layer_index}].width_px must be > 0"));
        }
        if facts.height_px == 0 {
            return Some(format!("art[{layer_index}].height_px must be > 0"));
        }
    }
    None
}

/// Place every requested layer, in draw order. See the `parallax_placements` `#[func]` for the
/// wire contract.
fn parallax_placements_impl(request_json: &str) -> String {
    let request: PlacementsRequest = match serde_json::from_str(request_json) {
        Ok(request) => request,
        Err(error) => return request_error_json(&format!("bad request: {error}")),
    };

    if let Some(detail) = parallax_request_error(&request) {
        return request_error_json(&detail);
    }

    let art: Vec<LayerArt> = request
        .art
        .iter()
        .map(|facts| LayerArt {
            width_px: facts.width_px,
            height_px: facts.height_px,
            is_rgba: facts.is_rgba,
            fully_opaque: facts.fully_opaque,
            top_row_opaque: facts.top_row_opaque,
            bottom_row_opaque: facts.bottom_row_opaque,
        })
        .collect();

    // Validation and derivation both live in core; a bad stack is reported, never worked
    // around. `resolve` also enforces `art.len() == layers.len()` (its `art_count` rule), so
    // the zip below is safe by construction.
    let resolved = match request.stack.resolve(&art) {
        Ok(resolved) => resolved,
        Err(validation) => {
            return serde_json::json!({
                "error": "stack_invalid",
                "errors": validation.messages,
            })
            .to_string()
        }
    };

    let view = ViewState {
        cam_center_world_px_x: request.view.cam_center_world_px_x,
        cam_center_world_px_y: request.view.cam_center_world_px_y,
        zoom: request.view.zoom,
        view_w_px: request.view.view_w_px,
        view_h_px: request.view.view_h_px,
    };

    // `stack.layers` and `resolved.layers` are index-aligned: `resolve` zips them in order and
    // filters nothing. Identity comes from the authored layer; everything derived comes from
    // the resolved one.
    let mut placements: Vec<PlacementEntry> = request
        .stack
        .layers
        .iter()
        .zip(resolved.layers.iter())
        .filter(|(_, resolved_layer)| {
            request
                .plane
                .is_none_or(|plane| resolved_layer.plane == plane)
        })
        .map(|(layer, resolved_layer)| {
            let placement = place_layer(&view, &request.anchor, resolved_layer);
            PlacementEntry {
                name: layer.name.clone(),
                file: layer.file.clone(),
                plane: resolved_layer.plane,
                draw_order: resolved_layer.draw_order,
                first_copy_left_px: placement.first_copy_left_px,
                copy_count: placement.copy_count,
                screen_top_px: placement.screen_top_px,
                scale: placement.scale,
                period_px: resolved_layer.period_px,
                art_h_px: resolved_layer.art_h_px,
                fill_above: resolved_layer.fill_above,
                fill_below: resolved_layer.fill_below,
            }
        })
        .collect();

    // The response carries draw order so the drawer never has to assume the stack authored its
    // layers in order.
    placements.sort_by_key(|entry| (plane_rank(entry.plane), entry.draw_order));

    serde_json::to_string(&PlacementsResponse { placements })
        .unwrap_or_else(|error| request_error_json(&format!("serialize failed: {error}")))
}

const TILE_BYTES: usize = 32 * 32 * 4; // RGBA8 stamp, 4096 bytes.
const STAMP_CELL: usize = 32;
const VARIANTS_PER_TILE: usize = 4; // identity, h-flip, v-flip, hv-flip.
const SIG_BYTES: usize = 32; // grayscale signature: 8x8 blocks, 2 blocks/byte.
const COARSE_BYTES: usize = 8 * 8 * 4; // canonical coarse: 8x8 blocks x 4 channels.
const MAX_WORKERS: usize = 32;

fn scan_exact_matches_impl(
    variants_flat: &[u8],
    bytes_flat: &[u8],
    pair_stream: &[i32],
    tolerance: f64,
    worker_count: i32,
) -> Vec<i32> {
    let n_pairs = pair_stream.len() / 2;
    if n_pairs == 0 {
        return Vec::new();
    }
    if pair_stream.len() % 2 != 0 || bytes_flat.len() % TILE_BYTES != 0 {
        return Vec::new();
    }
    let n_tiles = bytes_flat.len() / TILE_BYTES;
    if variants_flat.len() != n_tiles * VARIANTS_PER_TILE * TILE_BYTES {
        return Vec::new();
    }

    let workers = (worker_count as usize).clamp(1, MAX_WORKERS).min(n_pairs);
    let chunk = n_pairs.div_ceil(workers);

    // Bandwidth optimization: the window scan emits pairs in (previous, current)
    // order, so consecutive pairs share the candidate but not the base. That makes
    // each worker re-read a fresh 16KB variants block per pair (no cache reuse).
    // Grouping pairs by base lets a worker read a base's 4 variants once and reuse
    // them across every candidate of that base. Results are re-emitted in original
    // pair order so the match stream is identical to the un-grouped scan.
    let mut by_base: Vec<(usize, usize, usize)> = pair_stream
        .chunks_exact(2)
        .enumerate()
        .map(|(idx, c)| (idx, c[0] as usize, c[1] as usize))
        .collect();
    by_base.sort_unstable_by_key(|&(_, base, _)| base);

    let by_base = by_base;
    let by_base_ref = &by_base;

    // Scoped threads: each worker owns a disjoint slice of the grouped pair list
    // and shares read-only access to the byte buffers by reference.
    let grouped_matches: Vec<(usize, i32, i32, i32, i32)> = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for w in 0..workers {
            let start = w * chunk;
            let end = (start + chunk).min(n_pairs);
            handles.push(scope.spawn(move || {
                let mut local = Vec::new();
                let mut i = start;
                while i < end {
                    let (idx, base, cand) = by_base_ref[i];
                    if base < n_tiles && cand < n_tiles {
                        let match_flip = flip_of_variants(
                            &variants_flat[base * VARIANTS_PER_TILE * TILE_BYTES..],
                            &bytes_flat[cand * TILE_BYTES..],
                            tolerance,
                        );
                        if let Some((flip_h, flip_v)) = match_flip {
                            local.push((
                                idx,
                                base as i32,
                                cand as i32,
                                flip_h as i32,
                                flip_v as i32,
                            ));
                        }
                    }
                    i += 1;
                }
                local
            }));
        }
        let mut all = Vec::new();
        for handle in handles {
            all.extend(handle.join().unwrap_or_default());
        }
        all
    });

    // Re-emit in original pair order so the match stream is identical to the
    // un-grouped layout the caller (and tests) rely on.
    let mut ordered = grouped_matches;
    ordered.sort_unstable_by_key(|&(idx, _, _, _, _)| idx);
    ordered
        .into_iter()
        .flat_map(|(_, base, cand, fh, fv)| [base, cand, fh, fv])
        .collect()
}

/// Port of the GDScript `_diff_capped`: running mean of per-byte abs diff with
/// early exit once the running mean exceeds `limit`. Returns `limit + 1.0` on
/// early exit so callers can reject without a full pass.
fn diff_capped(a: &[u8], b: &[u8], limit: f64) -> f64 {
    let n = a.len();
    let mut sum: i64 = 0;
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        sum += (*x as i64 - *y as i64).abs();
        if (sum as f64) / ((i + 1) as f64) > limit {
            return limit + 1.0;
        }
    }
    (sum as f64) / (n as f64).max(1.0)
}

/// Port of the GDScript `_diff` (uncapped): full-pass mean of per-byte abs
/// diff. Unlike `diff_capped`, never early-exits — matches `_flip_of_bytes`
/// semantics exactly (GDScript's finalize path uses the uncapped `_diff`).
fn diff_full(a: &[u8], b: &[u8]) -> f64 {
    let n = a.len();
    let mut sum: i64 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        sum += (*x as i64 - *y as i64).abs();
    }
    (sum as f64) / (n as f64).max(1.0)
}

/// Port of the GDScript `_flip_of_variants`: try all 4 flip variants of `base`
/// against the candidate's canonical bytes and return the best flip on match.
/// `variants_base` must hold the 4 variants contiguously (4096 bytes each).
///
/// IMPORTANT: mirrors `_flip_of_bytes` — the GDScript `_flip_of_variants`
/// declares `var best_diff := 0x7fffffff` (int), so `best_diff = d` TRUNCATES
/// the float diff. A variant whose diff is in `[tol, tol+1)` therefore still
/// matches (int best_diff 4 <= tol 4.0). Ported verbatim (i32 best_diff).
fn flip_of_variants(variants_base: &[u8], cand: &[u8], tolerance: f64) -> Option<(bool, bool)> {
    let flip_list: [[bool; 2]; 4] = [[false, false], [true, false], [false, true], [true, true]];
    let mut best_diff: i32 = 0x7fffffff;
    let mut best: (bool, bool) = (false, false);
    for v in 0..VARIANTS_PER_TILE {
        let variant = &variants_base[v * TILE_BYTES..(v + 1) * TILE_BYTES];
        let d = diff_capped(cand, variant, tolerance);
        if d < best_diff as f64 {
            best_diff = d as i32;
            best = (flip_list[v][0], flip_list[v][1]);
        }
    }
    if best_diff as f64 <= tolerance {
        Some(best)
    } else {
        None
    }
}

/// Port of the GDScript `_flip_of_bytes`: build the 4 flip variants of `base`
/// on the fly (identity, h, v, hv) and return the winning flip as a packed
/// int: 0 = no match, 1 = identity, 2 = h-flip, 4 = v-flip, 8 = hv-flip.
/// Mirrors the GDScript best-diff tie-break order (first in flip list wins on
/// equal diff). Uses the uncapped `diff_full` to match `_flip_of_bytes`.
///
/// IMPORTANT: the GDScript `_flip_of_bytes` declares `var best_diff :=
/// 0x7fffffff` — an int — so assigning `best_diff = d` TRUNCATES the float
/// diff. `_flip_of` therefore matches any variant whose diff is in `[tol,
/// tol+1)`, not just `<= tol`. We replicate the truncation exactly so the
/// native port agrees pair-for-pair with the GDScript path.
fn flip_of_bytes_impl(base: &[u8], cand: &[u8], tolerance: f64) -> u8 {
    let mut best_diff: i32 = 0x7fffffff;
    let mut best: u8 = 0;
    let flips: [u8; 4] = [1, 2, 4, 8];
    for (v, &code) in flips.iter().enumerate() {
        let flip_h = v & 1 == 1;
        let flip_v = v & 2 == 2;
        let mut variant = Vec::with_capacity(TILE_BYTES);
        variant.resize(TILE_BYTES, 0u8);
        for y in 0..STAMP_CELL {
            for x in 0..STAMP_CELL {
                let src_x = if flip_h { STAMP_CELL - 1 - x } else { x };
                let src_y = if flip_v { STAMP_CELL - 1 - y } else { y };
                let si = (src_y * STAMP_CELL + src_x) * 4;
                let di = (y * STAMP_CELL + x) * 4;
                variant[di] = base[si];
                variant[di + 1] = base[si + 1];
                variant[di + 2] = base[si + 2];
                variant[di + 3] = base[si + 3];
            }
        }
        let d = diff_full(cand, &variant);
        if d < best_diff as f64 {
            best_diff = d as i32;
            best = code;
        }
    }
    if best_diff as f64 <= tolerance {
        best
    } else {
        0
    }
}

/// Port of the GDScript `_flip_variants`: build the 4 RGBA flip variants of
/// every tile (identity, h-flip, v-flip, hv-flip) into a flat N*4*4096 buffer.
/// `bytes_flat` is the concatenation of canonical RGBA bytes (N*4096).
fn build_variants_impl(bytes_flat: &[u8], worker_count: i32) -> Vec<u8> {
    let n_tiles = bytes_flat.len() / TILE_BYTES;
    if n_tiles == 0 || bytes_flat.len() % TILE_BYTES != 0 {
        return Vec::new();
    }
    let workers = (worker_count as usize).clamp(1, MAX_WORKERS).min(n_tiles);
    let chunk = n_tiles.div_ceil(workers);

    let variants: Vec<u8> = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for w in 0..workers {
            let start = w * chunk;
            let end = (start + chunk).min(n_tiles);
            handles.push(scope.spawn(move || {
                let mut local = Vec::with_capacity((end - start) * VARIANTS_PER_TILE * TILE_BYTES);
                for t in start..end {
                    let tile = &bytes_flat[t * TILE_BYTES..(t + 1) * TILE_BYTES];
                    for (flip_h, flip_v) in
                        [(false, false), (true, false), (false, true), (true, true)]
                    {
                        for y in 0..32 {
                            for x in 0..32 {
                                let src_x = if flip_h { 31 - x } else { x };
                                let src_y = if flip_v { 31 - y } else { y };
                                let source = (src_y * 32 + src_x) * 4;
                                local.extend_from_slice(&tile[source..source + 4]);
                            }
                        }
                    }
                }
                local
            }));
        }
        let mut all = Vec::with_capacity(n_tiles * VARIANTS_PER_TILE * TILE_BYTES);
        for handle in handles {
            all.extend(handle.join().unwrap_or_default());
        }
        all
    });
    variants
}

/// Port of the GDScript `_grayscale_sig_near`: sum the hi/lo nibble diffs of
/// two 32-byte grayscale signatures; a pair is "near" when the sum is <= 96.
fn grayscale_sig_near(a: &[u8], b: &[u8]) -> bool {
    let mut diff: i64 = 0;
    for i in 0..a.len() {
        let av_hi = a[i] >> 4;
        let av_lo = a[i] & 0x0f;
        let bv_hi = b[i] >> 4;
        let bv_lo = b[i] & 0x0f;
        diff += (av_hi as i64 - bv_hi as i64).abs() + (av_lo as i64 - bv_lo as i64).abs();
    }
    diff <= 96
}

/// Port of the GDScript `_projection_lower_bound`: binary search into the
/// projection-sorted list for the first entry whose projection is >= `target`.
fn projection_lower_bound(sorted: &[i32], target: i32) -> usize {
    let mut lo = 0usize;
    let mut hi = sorted.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if sorted[mid] < target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Port of the GDScript `_fp_hash`: FNV-1a 32-bit hash over the bytes,
/// returned as i64 to match GDScript's 64-bit integer dictionary keys.
fn fp_hash(bytes: &[u8]) -> u32 {
    let mut h: u32 = 2166136261;
    for &b in bytes {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    h
}

/// Luminance of one 4x4 RGBA block (`_grayscale_sig`'s inner 16 pixels): the
/// truncated `int(0.299r + 0.587g + 0.114b)` per pixel, summed as GDScript.
/// GDScript floats are 64-bit doubles (verified: `0.299*16+0.587*16+0.114*16`
/// is 15.99999999999999822 -> int 15), so the port uses f64 exactly; an f32
/// port rounds boundary sums up to the integer and over-counts luminance.
fn block_luminance(bytes: &[u8], block_y: usize, block_x: usize) -> i64 {
    let mut sum: i64 = 0;
    for y in 0..4 {
        for x in 0..4 {
            let px = ((block_y * 4 + y) * STAMP_CELL + block_x * 4 + x) * 4;
            let r = bytes[px] as f64;
            let g = bytes[px + 1] as f64;
            let b = bytes[px + 2] as f64;
            sum += (0.299 * r + 0.587 * g + 0.114 * b) as i64;
        }
    }
    sum
}

/// Port of GDScript `_grayscale_sig`: 32-byte signature, 2 nibbles per byte
/// (8x8 blocks). `level = mini(15, (sum/16) >> 4)`; even blocks hold the hi
/// nibble, odd blocks the lo nibble. Pure over the 4096 RGBA bytes.
fn grayscale_signature_impl(bytes: &[u8]) -> [u8; SIG_BYTES] {
    let mut sig = [0u8; SIG_BYTES];
    for by in 0..8 {
        for bx in 0..8 {
            let lum = block_luminance(bytes, by, bx);
            let level = ((lum / 16) >> 4).min(15) as u8;
            let block = by * 8 + bx;
            let byte_index = block / 2;
            if block % 2 == 0 {
                sig[byte_index] = level << 4;
            } else {
                sig[byte_index] |= level;
            }
        }
    }
    sig
}

/// Port of GDScript `_grayscale_luminance_projection`: sum of each block's
/// integer mean luminance `sum/16`; the scalar used to order the window scan.
fn luminance_projection_impl(bytes: &[u8]) -> i32 {
    let mut total: i64 = 0;
    for by in 0..8 {
        for bx in 0..8 {
            total += block_luminance(bytes, by, bx) / 16;
        }
    }
    total as i32
}

/// Port of GDScript `_coarse_bytes_rgba`: 4x4 RGBA block means -> 8x8x4 bytes
/// (256). GDScript accumulates `r += data[si]` (i64) then stores `r / 16`
/// (integer truncation) — port uses i64 exactly.
fn coarse_bytes_rgba_impl(bytes: &[u8]) -> [u8; COARSE_BYTES] {
    let mut out = [0u8; COARSE_BYTES];
    for by in 0..8usize {
        for bx in 0..8usize {
            let mut r: i64 = 0;
            let mut g: i64 = 0;
            let mut b: i64 = 0;
            let mut a: i64 = 0;
            for y in 0..4usize {
                for x in 0..4usize {
                    let si = ((by * 4 + y) * STAMP_CELL + (bx * 4 + x)) * 4;
                    r += bytes[si] as i64;
                    g += bytes[si + 1] as i64;
                    b += bytes[si + 2] as i64;
                    a += bytes[si + 3] as i64;
                }
            }
            let o = (by * 8 + bx) * 4;
            out[o] = (r / 16) as u8;
            out[o + 1] = (g / 16) as u8;
            out[o + 2] = (b / 16) as u8;
            out[o + 3] = (a / 16) as u8;
        }
    }
    out
}

/// Port of GDScript `_flip_coarse`: mirror the 8x8x4 coarse bytes horizontally
/// and/or vertically (channel layout preserved within each block).
fn flip_coarse_impl(c: &[u8], flip_h: bool, flip_v: bool) -> [u8; COARSE_BYTES] {
    let mut out = [0u8; COARSE_BYTES];
    for by in 0..8usize {
        for bx in 0..8usize {
            let sby = if flip_v { 7 - by } else { by };
            let sbx = if flip_h { 7 - bx } else { bx };
            let si = (sby * 8 + sbx) * 4;
            let di = (by * 8 + bx) * 4;
            for k in 0..4 {
                out[di + k] = c[si + k];
            }
        }
    }
    out
}

/// Port of GDScript `_canonical_coarse_bytes` for one tile: coarse block means,
/// then the lexicographically smallest of identity / h-flip / v-flip / hv-flip
/// (GDScript `_bytes_less` compares byte-wise, shorter-wins-if-prefix; Rust
/// slice `<` matches).
fn canonical_coarse_impl(bytes: &[u8]) -> [u8; COARSE_BYTES] {
    let c = coarse_bytes_rgba_impl(bytes);
    let mut best: [u8; COARSE_BYTES] = c;
    for flip_v in [false, true] {
        for flip_h in [false, true] {
            if flip_v || flip_h {
                let f = flip_coarse_impl(&c, flip_h, flip_v);
                if f[..] < best[..] {
                    best = f;
                }
            }
        }
    }
    best
}

/// Port of the GDScript window scan + `_grayscale_sig_near` gate: for every
/// position in the projection-sorted list, walk the window of previous entries
/// within `max_delta` and keep the [base, candidate] pair when the pair's
/// grayscale signatures are near. Returns a flat [base, candidate] stream.
fn scan_gate_pairs_impl(
    sigs_flat: &[u8],
    ordered_projections: &[i32],
    ordered_indices: &[i32],
    max_delta: i32,
    worker_count: i32,
) -> Vec<i32> {
    let n = ordered_indices.len();
    if n == 0
        || sigs_flat.len() % SIG_BYTES != 0
        || ordered_projections.len() != n
        || sigs_flat.len() / SIG_BYTES != n
    {
        return Vec::new();
    }
    let workers = (worker_count as usize).clamp(1, MAX_WORKERS).min(n);
    let chunk = n.div_ceil(workers);

    let pairs: Vec<i32> = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for w in 0..workers {
            let start = w * chunk;
            let end = (start + chunk).min(n);
            handles.push(scope.spawn(move || {
                let mut local = Vec::new();
                for position in start..end {
                    let current_projection = ordered_projections[position];
                    let target_projection = current_projection - max_delta;
                    let window_start =
                        projection_lower_bound(ordered_projections, target_projection);
                    for previous_position in window_start..position {
                        let base = ordered_indices[previous_position] as usize;
                        let cand = ordered_indices[position] as usize;
                        let base_sig = &sigs_flat[base * SIG_BYTES..(base + 1) * SIG_BYTES];
                        let cand_sig = &sigs_flat[cand * SIG_BYTES..(cand + 1) * SIG_BYTES];
                        if grayscale_sig_near(base_sig, cand_sig) {
                            local.extend_from_slice(&[base as i32, cand as i32]);
                        }
                    }
                }
                local
            }));
        }
        let mut all = Vec::new();
        for handle in handles {
            all.extend(handle.join().unwrap_or_default());
        }
        all
    });
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_capped_identical_is_zero() {
        let a: Vec<u8> = (0u8..64).collect();
        let b: Vec<u8> = a.clone();
        assert_eq!(diff_capped(&a, &b, 4.0), 0.0);
    }

    #[test]
    fn diff_capped_heavy_deviation_exceeds_limit() {
        let mut a: Vec<u8> = (0u8..64).collect();
        let b: Vec<u8> = a.clone();
        a[0] = 200;
        assert!(diff_capped(&a, &b, 4.0) > 4.0);
    }

    #[test]
    fn diff_capped_small_deviation_under_limit() {
        let mut a: Vec<u8> = (0u8..64).collect();
        let b: Vec<u8> = a.clone();
        a[0] = 1;
        assert!(diff_capped(&a, &b, 4.0) <= 4.0);
    }

    #[test]
    fn flip_of_variants_matches_identity() {
        let mut tile = vec![0u8; TILE_BYTES];
        for (i, byte) in tile.iter_mut().enumerate() {
            *byte = (i % 255) as u8;
        }
        let mut variants = Vec::new();
        variants.extend_from_slice(&tile); // identity
        variants.extend_from_slice(&tile); // placeholders for h/v/hv flips
        variants.extend_from_slice(&tile);
        variants.extend_from_slice(&tile);
        assert_eq!(
            flip_of_variants(&variants, &tile, 4.0),
            Some((false, false))
        );
    }

    #[test]
    fn flip_of_variants_truncates_best_diff_like_gdscript() {
        // Regression for issue #52: GDScript `_flip_of_variants` declares
        // `var best_diff := 0x7fffffff` (int), so `best_diff = d` TRUNCATES the
        // float diff. That [tol, tol+1) acceptance window is, however,
        // UNREACHABLE: `flip_of_variants` scores through `_diff_capped`, which
        // early-exits to `limit + 1.0`, so the returned diff is always <= tol
        // or exactly tol+1.0. Int and f64 accumulation are observationally
        // identical; the i32 port mirrors the GDScript verbatim (defense in
        // depth). This test pins the parity invariant for an uncapped diff in
        // (tol, tol+1): such a candidate must still REJECT, as in GDScript.
        let mut base = [0u8; TILE_BYTES];
        for i in 0..TILE_BYTES {
            base[i] = (i % 251) as u8;
        }
        let mut cand = [0u8; TILE_BYTES];
        for i in 0..TILE_BYTES {
            // +4 on every byte (mean 4.0), plus +1 on every 64th byte pushes
            // the full-mean diff just above 4.0 while staying below 5.0.
            cand[i] = (base[i] as i32 + 4 + if i % 64 == 0 { 1 } else { 0 }) as u8;
        }
        let full = diff_full(&base, &cand);
        assert!(
            full > 4.0 && full < 5.0,
            "boundary diff expected, got {}",
            full
        );

        // Build the 4 contiguous flip variants of `base` (identity, h, v, hv).
        let flips: [[bool; 2]; 4] = [[false, false], [true, false], [false, true], [true, true]];
        let mut variants = Vec::with_capacity(TILE_BYTES * 4);
        for flip in flips {
            let (flip_h, flip_v) = (flip[0], flip[1]);
            let mut variant = vec![0u8; TILE_BYTES];
            for y in 0..STAMP_CELL {
                for x in 0..STAMP_CELL {
                    let sx = if flip_h { STAMP_CELL - 1 - x } else { x };
                    let sy = if flip_v { STAMP_CELL - 1 - y } else { y };
                    let si = (sy * STAMP_CELL + sx) * 4;
                    let di = (y * STAMP_CELL + x) * 4;
                    variant[di..di + 4].copy_from_slice(&base[si..si + 4]);
                }
            }
            variants.extend_from_slice(&variant);
        }

        // Any flip is a permutation of the bytes, so every variant has the same
        // uncapped diff of 4.0156 -> `_diff_capped` pops to tol+1 = 5.0,
        // truncated 5 > 4 -> no match (GDScript-identical).
        assert_eq!(flip_of_variants(&variants, &cand, 4.0), None);
    }

    #[test]
    fn flip_of_variants_still_matches_below_tolerance() {
        // Guard that the i32 port did not tighten matching: a candidate whose
        // best diff is <= tol must still match identity (first in flip order).
        let mut base = [0u8; TILE_BYTES];
        for i in 0..TILE_BYTES {
            base[i] = (i % 251) as u8;
        }
        let mut tight = base.clone();
        tight[0] = (tight[0] as i32 + 1) as u8;
        assert!(diff_capped(&base, &tight, 4.0) <= 4.0);
        let mut variants = Vec::with_capacity(TILE_BYTES * 4);
        variants.extend_from_slice(&base);
        variants.extend_from_slice(&base);
        variants.extend_from_slice(&base);
        variants.extend_from_slice(&base);
        assert_eq!(
            flip_of_variants(&variants, &tight, 4.0),
            Some((false, false))
        );
    }

    #[test]
    fn scan_exact_matches_parallel_finds_flipped_pair() {
        // Three 32x32 tiles: canonical, its h+v flip, and a distinct tile.
        // Byte layout is row-major RGBA; flipping both axes maps (x,y) ->
        // (31-x, 31-y). Variant order is identity, h-flip, v-flip, hv-flip.
        let mut canonical = vec![0u8; TILE_BYTES];
        for y in 0..32 {
            for x in 0..32 {
                let idx = (y * 32 + x) * 4;
                canonical[idx] = (x + y) as u8;
                canonical[idx + 1] = 255;
                canonical[idx + 2] = 0;
                canonical[idx + 3] = 255;
            }
        }
        let flip = |tile: &[u8], flip_h: bool, flip_v: bool| -> Vec<u8> {
            let mut out = vec![0u8; TILE_BYTES];
            for y in 0..32 {
                for x in 0..32 {
                    let sx = if flip_h { 31 - x } else { x };
                    let sy = if flip_v { 31 - y } else { y };
                    let src = (sy * 32 + sx) * 4;
                    let dst = (y * 32 + x) * 4;
                    out[dst..dst + 4].copy_from_slice(&tile[src..src + 4]);
                }
            }
            out
        };
        let flipped = flip(&canonical, true, true);
        let mut distinct = vec![0u8; TILE_BYTES];
        for (i, byte) in distinct.iter_mut().enumerate() {
            *byte = (i as u8).wrapping_mul(7);
        }

        let bytes_flat: Vec<u8> = [&canonical[..], &flipped[..], &distinct[..]].concat();
        let mut variants_flat: Vec<u8> = Vec::new();
        for tile in [&canonical[..], &flipped[..], &distinct[..]] {
            variants_flat.extend_from_slice(&flip(tile, false, false));
            variants_flat.extend_from_slice(&flip(tile, true, false));
            variants_flat.extend_from_slice(&flip(tile, false, true));
            variants_flat.extend_from_slice(&flip(tile, true, true));
        }
        let pair_stream = vec![0i32, 1, 0, 2];
        let matches = scan_exact_matches_impl(&variants_flat, &bytes_flat, &pair_stream, 4.0, 4);
        assert_eq!(matches, vec![0, 1, 1, 1]); // base 0 matches cand 1 via h+v flip
    }

    #[test]
    fn scan_exact_matches_preserves_input_pair_order_after_base_grouping() {
        // Four tiles: 0 canonical, 1 h+v flip of 0, 2 identical to 0, 3 distinct.
        // Base-grouping reorders processing by base, so the match stream must be
        // re-emitted in the caller's pair order, not base order.
        let mut canonical = vec![0u8; TILE_BYTES];
        for y in 0..32 {
            for x in 0..32 {
                let idx = (y * 32 + x) * 4;
                canonical[idx] = (x + y) as u8;
                canonical[idx + 1] = 200;
                canonical[idx + 2] = 10;
                canonical[idx + 3] = 255;
            }
        }
        let flip = |tile: &[u8], flip_h: bool, flip_v: bool| -> Vec<u8> {
            let mut out = vec![0u8; TILE_BYTES];
            for y in 0..32 {
                for x in 0..32 {
                    let sx = if flip_h { 31 - x } else { x };
                    let sy = if flip_v { 31 - y } else { y };
                    let src = (sy * 32 + sx) * 4;
                    let dst = (y * 32 + x) * 4;
                    out[dst..dst + 4].copy_from_slice(&tile[src..src + 4]);
                }
            }
            out
        };
        let flipped = flip(&canonical, true, true);
        let mut distinct = vec![0u8; TILE_BYTES];
        for (i, byte) in distinct.iter_mut().enumerate() {
            *byte = (i as u8).wrapping_mul(13);
        }

        let bytes_flat: Vec<u8> =
            [&canonical[..], &flipped[..], &canonical[..], &distinct[..]].concat();
        let mut variants_flat: Vec<u8> = Vec::new();
        for tile in [&canonical[..], &flipped[..], &canonical[..], &distinct[..]] {
            variants_flat.extend_from_slice(&flip(tile, false, false));
            variants_flat.extend_from_slice(&flip(tile, true, false));
            variants_flat.extend_from_slice(&flip(tile, false, true));
            variants_flat.extend_from_slice(&flip(tile, true, true));
        }
        // Pairs deliberately out of base order: cand 2 (matches 0) first, then
        // cand 1 (matches 0 via flip), then cand 3 (distinct). Expect matches
        // emitted in exactly this input order.
        let pair_stream = vec![0i32, 2, 0, 1, 0, 3];
        let matches = scan_exact_matches_impl(&variants_flat, &bytes_flat, &pair_stream, 4.0, 4);
        assert_eq!(matches, vec![0, 2, 0, 0, 0, 1, 1, 1]);
    }

    #[test]
    fn build_variants_matches_gdscript_flip_order() {
        // A 32x32 tile with a known row/col pattern; verify the flat variant
        // layout matches the GDScript convention: identity, h-flip, v-flip,
        // hv-flip, each row-major RGBA.
        let mut tile = vec![0u8; TILE_BYTES];
        for y in 0..32 {
            for x in 0..32 {
                let idx = (y * 32 + x) * 4;
                tile[idx] = (x * 3) as u8;
                tile[idx + 1] = (y * 5) as u8;
                tile[idx + 2] = 100;
                tile[idx + 3] = 255;
            }
        }
        let variants = build_variants_impl(&tile, 4);
        assert_eq!(variants.len(), VARIANTS_PER_TILE * TILE_BYTES);
        let identity = &variants[0 * TILE_BYTES..1 * TILE_BYTES];
        assert_eq!(identity, tile.as_slice());
        let h_flip = &variants[1 * TILE_BYTES..2 * TILE_BYTES];
        let v_flip = &variants[2 * TILE_BYTES..3 * TILE_BYTES];
        let hv_flip = &variants[3 * TILE_BYTES..4 * TILE_BYTES];
        for y in 0..32 {
            for x in 0..32 {
                let src = (y * 32 + (31 - x)) * 4;
                let dst = (y * 32 + x) * 4;
                assert_eq!(&h_flip[dst..dst + 4], &tile[src..src + 4]);
                let src_v = ((31 - y) * 32 + x) * 4;
                assert_eq!(&v_flip[dst..dst + 4], &tile[src_v..src_v + 4]);
                let src_hv = ((31 - y) * 32 + (31 - x)) * 4;
                assert_eq!(&hv_flip[dst..dst + 4], &tile[src_hv..src_hv + 4]);
            }
        }
    }

    #[test]
    fn grayscale_signature_impl_golden() {
        // All zero RGBA -> every block luminance 0 -> level 0 -> sig all 0x00,
        // projection 0.
        let black = vec![0u8; TILE_BYTES];
        assert!(grayscale_signature_impl(&black).iter().all(|&b| b == 0));
        assert_eq!(luminance_projection_impl(&black), 0);

        // All-white RGBA (r=g=b=255): per-pixel f64 luminance rounds to 255.0 exactly
        // (GDScript-identical; verified alongside the parity probe) -> block
        // sum 4080, level 15 -> sig all 0xFF. Projection = 64 * (4080/16).
        let white = vec![255u8; TILE_BYTES];
        assert!(grayscale_signature_impl(&white).iter().all(|&b| b == 0xff));
        assert_eq!(luminance_projection_impl(&white), 64 * 255);
    }

    #[test]
    fn grayscale_signature_impl_gradient_layout() {
        // Brick pattern: a 4096-byte buffer where luminance differs per 4x4
        // block deterministically. Check nibble packing only (hi/lo positions)
        // rather than absolute luminance values: block 0 (byte 0 hi) vs
        // block 1 (byte 0 lo), block 2 (byte 1 hi), block 3 (byte 1 lo).
        let mut tile = vec![0u8; TILE_BYTES];
        // Fill each pixel with the same gray value per block column band:
        // block_x band values 0..8 -> bytes 0..255.
        for y in 0..STAMP_CELL {
            for x in 0..STAMP_CELL {
                let px = (y * STAMP_CELL + x) * 4;
                let gray = ((x / 4) * 36) as u8; // 0,36,72,...,252 per 4-col block
                tile[px] = gray;
                tile[px + 1] = gray;
                tile[px + 2] = gray;
                tile[px + 3] = 255;
            }
        }
        let sig = grayscale_signature_impl(&tile);
        let by = 0;
        let levels: Vec<i32> = (0..8)
            .map(|bx| ((block_luminance(&tile, by, bx) / 16) >> 4).min(15) as i32)
            .collect();
        // Need at least one differing pair of adjacent blocks to prove packing.
        assert_ne!(levels[0], levels[7], "block luminances must differ");
        for bx in 0..8 {
            let block = bx;
            let byte_index = block / 2;
            if block % 2 == 0 {
                assert_eq!((sig[byte_index] >> 4) as i32, levels[bx]);
            } else {
                assert_eq!((sig[byte_index] & 0x0f) as i32, levels[bx]);
            }
        }
    }

    #[test]
    fn grayscale_sig_near_matches_threshold() {
        let near_a = [0x00u8; SIG_BYTES];
        let near_b = [0x00u8; SIG_BYTES];
        assert!(grayscale_sig_near(&near_a, &near_b));
        let mut far_b = [0x00u8; SIG_BYTES];
        // 4 bytes at max nibble diff (15+15=30 each) => 120 > 96.
        far_b[0] = 0xff;
        far_b[1] = 0xff;
        far_b[2] = 0xff;
        far_b[3] = 0xff;
        assert!(!grayscale_sig_near(&near_a, &far_b));
    }

    #[test]
    fn coarse_bytes_rgba_impl_block_means() {
        // Fill each 4x4 block with a distinct constant color; block means must
        // equal that constant (integer division of a multiple of 16).
        let mut tile = vec![0u8; TILE_BYTES];
        for by in 0..8usize {
            for bx in 0..8usize {
                let r = (by * 32) as u8;
                let g = (bx * 32) as u8;
                let b = ((by + bx) * 8) as u8;
                for y in 0..4usize {
                    for x in 0..4usize {
                        let px = ((by * 4 + y) * STAMP_CELL + bx * 4 + x) * 4;
                        tile[px] = r;
                        tile[px + 1] = g;
                        tile[px + 2] = b;
                        tile[px + 3] = 255;
                    }
                }
            }
        }
        let c = coarse_bytes_rgba_impl(&tile);
        assert_eq!(c.len(), COARSE_BYTES);
        for by in 0..8usize {
            for bx in 0..8usize {
                let o = (by * 8 + bx) * 4;
                assert_eq!(c[o], (by * 32) as u8);
                assert_eq!(c[o + 1], (bx * 32) as u8);
                assert_eq!(c[o + 2], ((by + bx) * 8) as u8);
                assert_eq!(c[o + 3], 255);
            }
        }
    }

    #[test]
    fn canonical_coarse_picks_lexicographic_min_flip() {
        // Build a tile whose coarse signature has a v-flip (or h-flip) smaller
        // than identity: a single colored block in the top-left corner. Its
        // h/v/hv flips move that block to the other corners; the canonical
        // form is the lexicographically smallest of the 4.
        let mut tile = vec![0u8; TILE_BYTES];
        // Top-left 4x4 block fully red.
        for y in 0..4usize {
            for x in 0..4usize {
                let px = (y * STAMP_CELL + x) * 4;
                tile[px] = 255;
                tile[px + 3] = 255;
            }
        }
        let c = coarse_bytes_rgba_impl(&tile);
        // Identity coarse: block (0,0) = [255,0,0,255], rest zero.
        assert_eq!(c[0], 255);
        assert_eq!(c[4], 0); // block (0,1) red channel 0
                             // Candidate flips of the raw coarse, and the canonical must be the min.
        let identity = c;
        let h = flip_coarse_impl(&identity, true, false);
        let v = flip_coarse_impl(&identity, false, true);
        let hv = flip_coarse_impl(&identity, true, true);
        let mut expected: Vec<u8> = identity.to_vec();
        for cand in [&h, &v, &hv] {
            if cand[..] < expected[..] {
                expected = cand.to_vec();
            }
        }
        let canon = canonical_coarse_impl(&tile);
        assert_eq!(
            canon.to_vec(),
            expected,
            "canonical must be the lexicographic min flip"
        );
    }

    #[test]
    fn scan_gate_pairs_windows_and_filters() {
        // Two tiles both with projection 10 (sorted), and one with projection
        // 500 (out of any window). Signatures: tile0 and tile1 are identical
        // (near), tile2 is far. With max_delta=256, tile0 vs tile1 is the only
        // in-window pair and passes the gate.
        let tile0_sig = [0x10u8; SIG_BYTES];
        let tile1_sig = [0x10u8; SIG_BYTES];
        let tile2_sig = [0xffu8; SIG_BYTES];
        let sigs_flat: Vec<u8> = [&tile0_sig[..], &tile1_sig[..], &tile2_sig[..]].concat();
        let ordered_projections = vec![10i32, 10, 500];
        let ordered_indices = vec![0i32, 1, 2];
        let pairs =
            scan_gate_pairs_impl(&sigs_flat, &ordered_projections, &ordered_indices, 256, 4);
        assert_eq!(pairs, vec![0, 1]);
    }

    #[test]
    fn scan_gate_pairs_sorts_indices_into_windows() {
        // 4 tiles whose projections cluster: p=[0,10,20,300]. max_delta=15.
        // Windows: position1 (p10) covers p0 (delta10); position2 (p20) covers
        // p10 (delta10) but not p0 (delta20). position3 (p300) covers none.
        let sig_near = [0x20u8; SIG_BYTES];
        let sig_far = [0x0fu8; SIG_BYTES];
        // sigs_flat is ordered by TILE INDEX: tile0 far, tiles 1-3 near.
        // ordered_indices maps sorted position -> tile index: [3,2,1,0] so
        // position0=tile3, position1=tile2, position2=tile1, position3=tile0.
        let sigs_flat: Vec<u8> =
            [&sig_far[..], &sig_near[..], &sig_near[..], &sig_near[..]].concat();
        let ordered_projections = vec![0i32, 10, 20, 300];
        let ordered_indices = vec![3i32, 2, 1, 0];
        let pairs = scan_gate_pairs_impl(&sigs_flat, &ordered_projections, &ordered_indices, 15, 4);
        assert_eq!(pairs, vec![3, 2, 2, 1]);
    }

    #[test]
    fn fp_hash_matches_fnv1a() {
        assert_eq!(fp_hash(&[]), 2166136261);
        assert_eq!(fp_hash(&[0x00u8, 0x01, 0x02, 0x03]), 3282719153);
        assert_eq!(fp_hash(&[0xffu8; 8]), 1823345245);
    }

    #[test]
    fn a3_neighbor_hashes_distinct_and_clamped() {
        // All-zero canonical: each byte perturbed to +1 gives a distinct hash
        // (256), and every -1 delta clamps to 0 giving the canonical zero hash
        // (1) -> 257 distinct total.
        let zero = [0u8; COARSE_BYTES];
        let hashes = a3_neighbor_hashes_impl(&zero);
        let mut sorted = hashes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            COARSE_BYTES + 1,
            "zero: 256 +1-neighbors + clamped-zero"
        );

        // All-0xff canonical: -1 deltas each give distinct hashes, +1 clamps.
        let maxed = [0xffu8; COARSE_BYTES];
        let hashes_max = a3_neighbor_hashes_impl(&maxed);
        let mut sorted_max = hashes_max.clone();
        sorted_max.sort_unstable();
        sorted_max.dedup();
        assert_eq!(
            sorted_max.len(),
            COARSE_BYTES + 1,
            "ff: 256 -1-neighbors + clamped-ff"
        );
    }

    #[test]
    fn flip_of_bytes_identifies_each_variant() {
        // A canonical whose h-flipped (or v-flipped) bytes are distinct lets us
        // verify each flip code. Build a gradient base: byte at (x,y) = x*4 + y.
        let mut base = [0u8; TILE_BYTES];
        for y in 0..STAMP_CELL {
            for x in 0..STAMP_CELL {
                let di = (y * STAMP_CELL + x) * 4;
                base[di] = (x * 4 + y) as u8;
                base[di + 1] = (x * 4 + y + 1) as u8;
                base[di + 2] = (x * 4 + y + 2) as u8;
                base[di + 3] = 255;
            }
        }
        // Identity candidate: identical bytes.
        assert_eq!(flip_of_bytes_impl(&base, &base, 4.0), 1);
        // h-flip candidate: build by re-mapping x.
        let mut hcand = [0u8; TILE_BYTES];
        for y in 0..STAMP_CELL {
            for x in 0..STAMP_CELL {
                let sx = STAMP_CELL - 1 - x;
                let si = (y * STAMP_CELL + sx) * 4;
                let di = (y * STAMP_CELL + x) * 4;
                hcand[di..di + 4].copy_from_slice(&base[si..si + 4]);
            }
        }
        assert_eq!(flip_of_bytes_impl(&base, &hcand, 4.0), 2);
        // v-flip candidate.
        let mut vcand = [0u8; TILE_BYTES];
        for y in 0..STAMP_CELL {
            for x in 0..STAMP_CELL {
                let sy = STAMP_CELL - 1 - y;
                let si = (sy * STAMP_CELL + x) * 4;
                let di = (y * STAMP_CELL + x) * 4;
                vcand[di..di + 4].copy_from_slice(&base[si..si + 4]);
            }
        }
        assert_eq!(flip_of_bytes_impl(&base, &vcand, 4.0), 4);
        // A wholly different tile (random noise) exceeds tolerance.
        let mut noise = [0x7fu8; TILE_BYTES];
        noise[0] = 1;
        assert_eq!(flip_of_bytes_impl(&base, &noise, 4.0), 0);
    }

    #[test]
    fn flip_of_bytes_truncates_best_diff_like_gdscript() {
        // Regression for the 982-vs-490 finalize bug: GDScript `_flip_of_bytes`
        // declares `var best_diff := 0x7fffffff` (int), so `best_diff = d`
        // TRUNCATES the float diff. A candidate whose best variant diff is in
        // [tol, tol+1) (here 4.0-5.0) still matches because floor(d) <= tol.
        // Build a base and a candidate that are identical except every RGBA
        // byte differs by 4: full-mean diff = 4.0 exactly. With a diff of 4.0
        // the truncated best_diff is 4 == tolerance, so identity must match.
        let mut base = [0u8; TILE_BYTES];
        for i in 0..TILE_BYTES {
            base[i] = (i % 251) as u8;
        }
        let mut cand = [0u8; TILE_BYTES];
        for i in 0..TILE_BYTES {
            cand[i] = (base[i] as i32 + 4).clamp(0, 255) as u8;
        }
        // Every byte differs by exactly 4 -> full diff = 4.0, truncated to 4.
        assert!(diff_full(&base, &cand) >= 4.0 && diff_full(&base, &cand) < 5.0);
        assert_eq!(flip_of_bytes_impl(&base, &cand, 4.0), 1);
    }

    // -----------------------------------------------------------------------
    // parallax_placements (issue #106)
    //
    // The fixture is built in code, never read from a file: the emitted `stack.json` under
    // `editor/assets/backdrop_stacks/` is untracked, so a test reading it would be asserting
    // machine state and would fail on a clean clone.
    // -----------------------------------------------------------------------

    /// A valid three-layer request: two `back` layers authored OUT of `draw_order` (so the
    /// response's ordering is load-bearing) plus one `front` layer (so the plane filter is
    /// testable without a second fixture). The base layer is the back plane's lowest
    /// `draw_order`, so it carries the fully opaque art and both non-transparent fills that
    /// core's `base_opaque` rule requires.
    ///
    /// The art widths differ per layer (1500 / 1920 / 640) so `period_px` identifies which art
    /// entry a placement was built from: with equal widths, a mis-zip of `stack.layers` against
    /// `resolved.layers` would produce the same numbers and pass.
    fn base_request() -> serde_json::Value {
        serde_json::json!({
            "view": {
                "cam_center_world_px_x": 960.0,
                "cam_center_world_px_y": 540.0,
                "zoom": 1.0,
                "view_w_px": 1920.0,
                "view_h_px": 1080.0
            },
            "anchor": { "world_px_x": 960.0, "world_px_y": 528.0 },
            "stack": {
                "schema_version": 2,
                "stack_key": "fixture",
                "palette_key": "neutral",
                "generator_seed": 1,
                "ref_view_px": { "width": 1920.0, "height": 1080.0 },
                "layers": [
                    {
                        "name": "Near", "file": "near.png", "plane": "back", "draw_order": 1,
                        "parallax_depth": { "kind": "finite", "value": 1.0 },
                        "vertical_damping": 0.8, "repeat_x": true,
                        "ref_screen_left_px": 0.0, "ref_screen_top_px": 0.0,
                        "fill_above": "transparent", "fill_below": "transparent"
                    },
                    {
                        "name": "Base", "file": "base.png", "plane": "back", "draw_order": 0,
                        "parallax_depth": { "kind": "finite", "value": 9.0 },
                        "vertical_damping": 0.8, "repeat_x": true,
                        "ref_screen_left_px": 0.0, "ref_screen_top_px": 0.0,
                        "fill_above": "#8896AAFF", "fill_below": "#C2CAD0FF"
                    },
                    {
                        "name": "Fore", "file": "fore.png", "plane": "front", "draw_order": 0,
                        "parallax_depth": { "kind": "finite", "value": -0.25 },
                        "vertical_damping": 0.8, "repeat_x": true,
                        "ref_screen_left_px": 0.0, "ref_screen_top_px": 0.0,
                        "fill_above": "transparent", "fill_below": "transparent"
                    }
                ]
            },
            "art": [
                {
                    "width_px": 1500, "height_px": 1080, "is_rgba": true,
                    "fully_opaque": false, "top_row_opaque": true, "bottom_row_opaque": true
                },
                {
                    "width_px": 1920, "height_px": 1080, "is_rgba": true,
                    "fully_opaque": true, "top_row_opaque": true, "bottom_row_opaque": true
                },
                {
                    "width_px": 640, "height_px": 1080, "is_rgba": true,
                    "fully_opaque": false, "top_row_opaque": true, "bottom_row_opaque": true
                }
            ]
        })
    }

    /// The `LayerArt` set matching `base_request()`'s art entries, written out literally so
    /// this oracle stays independent of the mapping under test.
    fn fixture_art() -> Vec<LayerArt> {
        vec![
            LayerArt {
                width_px: 1500,
                height_px: 1080,
                is_rgba: true,
                fully_opaque: false,
                top_row_opaque: true,
                bottom_row_opaque: true,
            },
            LayerArt {
                width_px: 1920,
                height_px: 1080,
                is_rgba: true,
                fully_opaque: true,
                top_row_opaque: true,
                bottom_row_opaque: true,
            },
            LayerArt {
                width_px: 640,
                height_px: 1080,
                is_rgba: true,
                fully_opaque: false,
                top_row_opaque: true,
                bottom_row_opaque: true,
            },
        ]
    }

    /// Call the bridge and hand back its `placements`, asserting the request was accepted.
    fn placements_of(request: &serde_json::Value) -> Vec<serde_json::Value> {
        let response: serde_json::Value =
            serde_json::from_str(&parallax_placements_impl(&request.to_string()))
                .expect("the bridge always returns JSON");
        assert!(
            response.get("error").is_none(),
            "a valid request must not error: {response}"
        );
        response["placements"]
            .as_array()
            .expect("placements array")
            .clone()
    }

    /// Call the bridge and hand back its `error` detail, asserting there was one.
    fn placement_error(request: &serde_json::Value) -> String {
        let response: serde_json::Value =
            serde_json::from_str(&parallax_placements_impl(&request.to_string()))
                .expect("the bridge always returns JSON");
        response["error"]
            .as_str()
            .unwrap_or_else(|| panic!("expected a request error, got {response}"))
            .to_string()
    }

    #[test]
    fn placements_match_place_layer_values() {
        let request_json = base_request();
        let placements = placements_of(&request_json);
        assert_eq!(
            placements.len(),
            3,
            "all three layers when no plane is given"
        );

        // The oracle: the same core call, on the same inputs. Comparing against
        // `place_layer` rather than against literals keeps this test valid when step 1's math
        // is tuned.
        let request: PlacementsRequest =
            serde_json::from_value(request_json).expect("the fixture parses");
        let resolved = request
            .stack
            .resolve(&fixture_art())
            .expect("the fixture validates");
        let view = ViewState {
            cam_center_world_px_x: request.view.cam_center_world_px_x,
            cam_center_world_px_y: request.view.cam_center_world_px_y,
            zoom: request.view.zoom,
            view_w_px: request.view.view_w_px,
            view_h_px: request.view.view_h_px,
        };

        for placement in &placements {
            let name = placement["name"].as_str().expect("name");
            let resolved_layer = resolved
                .layers
                .iter()
                .find(|candidate| candidate.name == name)
                .expect("every placement names a resolved layer");
            let expected = place_layer(&view, &request.anchor, resolved_layer);
            assert_eq!(
                placement["first_copy_left_px"],
                serde_json::json!(expected.first_copy_left_px),
                "{name} first_copy_left_px"
            );
            assert_eq!(
                placement["copy_count"],
                serde_json::json!(expected.copy_count),
                "{name} copy_count"
            );
            assert_eq!(
                placement["screen_top_px"],
                serde_json::json!(expected.screen_top_px),
                "{name} screen_top_px"
            );
            assert_eq!(
                placement["scale"],
                serde_json::json!(expected.scale),
                "{name} scale"
            );
            assert_eq!(
                placement["period_px"],
                serde_json::json!(resolved_layer.period_px),
                "{name} period_px"
            );
            assert_eq!(
                placement["art_h_px"],
                serde_json::json!(resolved_layer.art_h_px),
                "{name} art_h_px"
            );
        }
    }

    #[test]
    fn plane_filter_returns_only_that_plane() {
        let mut front_only = base_request();
        front_only["plane"] = serde_json::json!("front");
        let front_placements = placements_of(&front_only);
        assert_eq!(front_placements.len(), 1, "the fixture has one front layer");
        assert_eq!(front_placements[0]["name"], serde_json::json!("Fore"));

        let mut back_only = base_request();
        back_only["plane"] = serde_json::json!("back");
        let back_placements = placements_of(&back_only);
        let names: Vec<&str> = back_placements
            .iter()
            .map(|placement| placement["name"].as_str().expect("name"))
            .collect();
        assert_eq!(
            names,
            vec!["Base", "Near"],
            "both back layers, front excluded"
        );
    }

    #[test]
    fn placements_are_sorted_by_plane_then_draw_order() {
        // `base_request()` authors Near (draw_order 1) BEFORE Base (draw_order 0), so this only
        // passes if the response is sorted rather than echoed.
        let placements = placements_of(&base_request());
        let names: Vec<&str> = placements
            .iter()
            .map(|placement| placement["name"].as_str().expect("name"))
            .collect();
        assert_eq!(
            names,
            vec!["Base", "Near", "Fore"],
            "back plane ascending by draw_order, then the front plane"
        );
        let draw_orders: Vec<i64> = placements
            .iter()
            .map(|placement| placement["draw_order"].as_i64().expect("draw_order"))
            .collect();
        assert_eq!(draw_orders, vec![0, 1, 0]);
    }

    #[test]
    fn response_carries_file_and_period_from_the_matching_layer() {
        let placements = placements_of(&base_request());
        // Each layer's art has a distinct width, so `period_px` names the art entry it was
        // built from and a misaligned zip cannot pass.
        for (name, file, period_px) in [
            ("Base", "base.png", 1920.0),
            ("Near", "near.png", 1500.0),
            ("Fore", "fore.png", 640.0),
        ] {
            let placement = placements
                .iter()
                .find(|candidate| candidate["name"] == serde_json::json!(name))
                .unwrap_or_else(|| panic!("no placement named {name}"));
            assert_eq!(placement["file"], serde_json::json!(file), "{name} file");
            assert_eq!(
                placement["period_px"],
                serde_json::json!(period_px),
                "{name} period_px"
            );
        }

        // The fills travel through core's single serialization of `Fill`.
        let base = placements
            .iter()
            .find(|candidate| candidate["name"] == serde_json::json!("Base"))
            .expect("the base layer is always placed");
        assert_eq!(base["fill_above"], serde_json::json!("#8896AAFF"));
        assert_eq!(base["fill_below"], serde_json::json!("#C2CAD0FF"));
    }

    #[test]
    fn invalid_stack_returns_stack_invalid_with_messages() {
        // The base layer's art must be fully opaque -- core's `base_opaque` rule.
        let mut not_opaque = base_request();
        not_opaque["art"][1]["fully_opaque"] = serde_json::json!(false);
        let response: serde_json::Value =
            serde_json::from_str(&parallax_placements_impl(&not_opaque.to_string()))
                .expect("the bridge always returns JSON");
        assert_eq!(response["error"], serde_json::json!("stack_invalid"));
        let codes: Vec<&str> = response["errors"]
            .as_array()
            .expect("errors array")
            .iter()
            .map(|message| message["code"].as_str().expect("code"))
            .collect();
        assert!(
            codes.contains(&"base_opaque"),
            "expected base_opaque, got {codes:?}"
        );

        // An art array one short is core's `art_count` rule, reported on the same path and in
        // the same vocabulary -- one failure keeps one wire spelling.
        let mut short_art = base_request();
        short_art["art"].as_array_mut().expect("art array").pop();
        let response: serde_json::Value =
            serde_json::from_str(&parallax_placements_impl(&short_art.to_string()))
                .expect("the bridge always returns JSON");
        assert_eq!(response["error"], serde_json::json!("stack_invalid"));
        let codes: Vec<&str> = response["errors"]
            .as_array()
            .expect("errors array")
            .iter()
            .map(|message| message["code"].as_str().expect("code"))
            .collect();
        assert!(
            codes.contains(&"art_count"),
            "expected art_count, got {codes:?}"
        );
    }

    #[test]
    fn bad_request_returns_error_never_panics() {
        // Malformed JSON never reaches the math.
        let response: serde_json::Value =
            serde_json::from_str(&parallax_placements_impl("{ not json")).expect("always JSON");
        assert!(
            response["error"]
                .as_str()
                .expect("an error string")
                .contains("bad request"),
            "got {response}"
        );

        // Each hazard is named in the message, so the test fails on the guard it targets rather
        // than on any error at all.
        for (field, bad_value) in [("zoom", 0.0), ("view_w_px", -1920.0), ("view_h_px", 0.0)] {
            let mut request = base_request();
            request["view"][field] = serde_json::json!(bad_value);
            assert!(
                placement_error(&request).contains(field),
                "{field} must be rejected by name"
            );
        }

        for art_field in ["width_px", "height_px"] {
            let mut request = base_request();
            request["art"][0][art_field] = serde_json::json!(0);
            assert!(
                placement_error(&request).contains(art_field),
                "{art_field} must be rejected by name"
            );
        }
    }
}
