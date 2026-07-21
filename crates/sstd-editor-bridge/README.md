# sstd-editor-bridge

Godot 4 GDExtension bridge that exposes `sstd-core` to GDScript. Compiled as a shared library (`.so`/`.dll`) that Godot loads via `editor/rust/editor_bridge.gdextension`.

## Build

```bash
./scripts/build-bridge.sh          # debug
./scripts/build-bridge.sh --release # release
```

## Architecture

```
GDScript                              sstd-editor-bridge (gdext)        sstd-core
┌────────────────────────┐            ┌────────────────────────┐        ┌──────────────┐
│ var bridge = load(...)  │            │ SstdBridge (Node)       │        │              │
│ bridge.import_terrain…()│ ── JSON ──▶│   #[func] fn methods()  │──▶──│  data model   │
│ bridge.validate_screen()│ ◀── JSON ──│   serde_json parse/ser  │◀────│  validation   │
│ bridge.validate_placem…│            └────────────────────────┘        └──────────────┘
└────────────────────────┘
```

All data crosses the boundary as JSON strings. The bridge never exposes complex types to GDScript — serialize, process, deserialize.

## Exposed Methods

| Method | Input | Output | Purpose |
|--------|-------|--------|---------|
| `validate_placement` | map_json, entity_key, tile_x, tile_y | `{"valid": bool}` | Check if entity can be placed at tile |
| `import_terrain_types` | json string | parsed JSON | Validate + parse terrain_types.json |
| `import_entity_defs` | json string | parsed JSON | Validate + parse entity_defs.json |
| `import_screen` | json string | parsed JSON | Validate + parse screen_{id}.json |
| `validate_screen` | json string | `{"valid": bool}` | Validate screen data |

