# sstd-core

Shared data model and utility library for the SSTD (Side-Scrolling Tower Defense) project. Both the Editor and the Game link this crate, guaranteeing identical behavior across tooling and runtime.

## Modules

| Module | Contents | Phase |
|--------|----------|-------|
| `error.rs` | `ValidationResult`, `StorageError`, `SstdResult<T>` | 1 |
| `terrain.rs` | `TerrainType`, `SurfaceType`, `HazardType`, `ZoneType`, `TerrainTypeDef` (serde), `TerrainTile`, `GridConfig`, `TimeConfig`, `BiomeType`, `OwnershipType`, `EntityStateType` | 1 |
| `entity.rs` | `EntityClass` (4 subclasses), `EntityDef`, `EntityDefEditor` (serde), `EntityInstance`, `ModifierDef`, `ModifierGroup`, `resolve_attribute()`, `AttributeDef`, `InstanceModifier`, `InstanceStatus`, `EntityLimits` | 1 |
| `map.rs` | `Screen`, `TileEntry`, `PlacedEntity`, `TileGrid`, coordinate conversions (`tile_to_world`, `world_to_screen`, `pixel_to_tile`), `ClearanceResult` | 1 |
| `storage.rs` | `TerrainTypesFile`, `EntityDefsFile`, `ScreenFile` (serde JSON), schema versioning, validation | 1 |
| `physics.rs` | Kinematic engine (planned for Phase 2) | 2 |

## Build

```bash
cargo build -p sstd-core
cargo test -p sstd-core
```

## Design Docs

See `TDD_World-Editor.md` in the project wiki for architecture and integration details.
