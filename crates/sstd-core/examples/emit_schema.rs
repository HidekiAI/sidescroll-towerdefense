use std::{env, fs};

use sstd_core::storage::{EntityDefsFile, ScreenFile, TerrainTypesFile, TileSetsFile};

/// Emits JSON Schemas for the persisted JSON file families into `editor/schemas/`.
/// Run: `cargo run -p sstd-core --example emit_schema`
fn write_schema<T: schemars::JsonSchema>(path: &str) {
    let schema = schemars::schema_for!(T);
    let json = serde_json::to_string_pretty(&schema).expect("schema serialization");
    fs::write(path, json).expect("schema write");
    println!("wrote {path}");
}

fn main() {
    let dir = format!("{}/../../editor/schemas", env!("CARGO_MANIFEST_DIR"));
    fs::create_dir_all(&dir).expect("schemas dir");

    write_schema::<TerrainTypesFile>(&format!("{dir}/terrain_types.schema.json"));
    write_schema::<EntityDefsFile>(&format!("{dir}/entity_defs.schema.json"));
    write_schema::<ScreenFile>(&format!("{dir}/screen.schema.json"));
    write_schema::<TileSetsFile>(&format!("{dir}/tile_sets.schema.json"));
}
