# World Editor (Godot 4)

The standalone level-authoring tool for SSTD. Built in **Godot 4.4** with a Rust GDExtension bridge (`sstd-editor-bridge`). Provides five tabs: terrain tile definitions, entity definitions, map painting, entity placement, and (planned) simulation.

See the wiki for full specs: [TDD_World-Editor](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TechnicalDesign/TDD_World-Editor) and [TDD_Saved-World](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TechnicalDesign/TDD_Saved-World).

## Run

```bash
godot4 --path .          # open the editor project
godot4 --path . --editor # Godot editor mode (edit the .tscn files)
```

Note: `main.tscn` embeds the map and placement editors **inline** — edit that scene when the standalone `map_editor.tscn`/`placement_editor.tscn` changes don't show up at runtime.

## Headless Validation

```bash
# syntax-check a single script
godot4 --headless --path . --check-only --script res://scripts/map_editor.gd

# boot the app headless (surfaces node/scene errors) and exit after 120 frames
godot4 --headless --path . --quit-after 120
```

## Tests

```bash
godot4 --headless --path . --script res://tests/test_screen_store.gd
```

See [tests/README.md](tests/README.md). The suite covers the `ScreenStore` registry, map/placement screen operations, and the minimap dialog.

## Layout

```
editor/
├── project.godot
├── scenes/
│   ├── main.tscn               # Tab container — editors embedded inline
│   ├── map_editor.tscn         # Tab 3 (terrain + TileSet painting)
│   ├── placement_editor.tscn   # Tab 4 (entity placement)
│   ├── terrain_editor.tscn     # Tab 1
│   ├── entity_editor.tscn      # Tab 2
│   └── screen_minimap_dialog.tscn
├── scripts/
│   ├── main.gd                 # Tab orchestration, ScreenStore wiring, save/load
│   ├── map_editor.gd           # Paint logic, screen ops, status HUD
│   ├── placement_editor.gd     # Entity snap + validation, screen ops, status HUD
│   ├── screen_store.gd         # world.json registry + per-screen cache
│   ├── screen_minimap.gd       # 2D screen-grid picker Control
│   └── resources/              # TileSetGrouping etc.
├── tests/
│   └── test_screen_store.gd
├── rust/                       # editor_bridge.gdextension
└── assets/                     # tiles, entities, tilesets (.tres), samples
```

## Conventions

- Screen registry index: `world.json` (gitignored via `editor/*.json` — generated artifact).
- An unregistered screen id is represented by position `(-1, -1)` (sentinel), never `(0, 0)`.
- Map editor preserves `placed_entities` through the registry cache (`_merge_placed_entities`).
