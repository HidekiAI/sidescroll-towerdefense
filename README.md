# sidescroll-towerdefense

Side Scrolling Tower Defense — hybrid base-management and side-scrolling tactical combat game.

- [Design Document](docs/Tower%20Defense%20Side-Scroller%20-%20Design%20Doc.md)
- [Wiki](https://github.com/your-org/sidescroll-towerdefense/wiki) — Technical Design Docs (TDD), Game Design Docs (GDD)

## Project Structure

```
├── Cargo.toml                  # Rust workspace root
├── crates/
│   └── sstd-core/              # Shared data model + physics (Editor + Game)
│       ├── src/
│       │   ├── terrain.rs      # TerrainType, SurfaceType, GridConfig
│       │   ├── entity.rs       # EntityClass, EntityDef, modifier resolution
│       │   ├── map.rs           # Screen, TileGrid, coordinate conversions
│       │   ├── storage.rs      # JSON import/export, schema versioning
│       │   ├── error.rs        # ValidationResult, StorageError
│       │   └── lib.rs          # Re-exports
│       └── README.md
├── editor/                     # Godot 4 editor project
│   ├── project.godot
│   ├── scenes/                 # 5 editor tab scenes (+ screen minimap dialog)
│   ├── scripts/                # GDScript UI logic (+ ScreenStore registry)
│   ├── tests/                  # Headless GDScript regression tests
│   └── rust/                   # GDExtension bridge config
├── scripts/
│   ├── setup.sh                # Install Godot 4 + create editor skeleton
│   └── ...
├── dev-tools/                  # Developer tooling
├── assets/                     # Reference samples, artwork
├── docs/                       # Design documents
└── README.md
```

## Setup

```bash
# Install Godot 4 + create editor skeleton
./scripts/setup.sh

# Build sstd-core (pure Rust, no Godot dependency)
cargo build -p sstd-core

# Build bridge + copy to editor/rust/
./scripts/build-bridge.sh

# Run tests
cargo test -p sstd-core

# Run the editor's headless GDScript regression suite
godot4 --headless --path editor --script res://tests/test_screen_store.gd

# Launch editor
godot4 --path editor
```

## Rust Crates

| Crate | Description | Depends On |
|---|---|---|
| `sstd-core` | Shared data model, validation, JSON I/O, physics engine | serde, serde_json |
| `sstd-editor-bridge` | Godot GDExtension — exposes sstd-core to GDScript | sstd-core, godot (gdext) |

## License

MIT — see [LICENSE](LICENSE).

## Image Attribution

All artwork in this repository (character sheets, concept art, UI mockups) is AI-generated using **Gemini 3.5** and is provided under a **Creative Commons (CC) license**. See individual file metadata for details.

