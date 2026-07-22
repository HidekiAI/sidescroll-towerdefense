use godot::prelude::*;
use sstd_core::storage::{EntityDefsFile, ScreenFile, TerrainTypesFile};
use sstd_core::terrain::GridConfig;

struct SstdEditorBridge;

#[gdextension]
unsafe impl ExtensionLibrary for SstdEditorBridge {}

#[derive(GodotClass)]
#[class(init, base=Node)]
struct SstdBridge {
    base: Base<Node>,
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

    #[func]
    fn get_grid_config(&self) -> GString {
        let config = GridConfig::default();
        GString::from(
            serde_json::to_string(&config)
                .unwrap_or_else(|_| "{\"error\": \"serialize failed\"}".to_string())
                .as_str(),
        )
    }

    #[func]
    fn check_dimension_compatibility(&self, screen_json: GString) -> GString {
        let current = GridConfig::default();
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
