# Tile Assets

This directory holds individual 32×32 px tile PNGs with their Godot 4 `.import`
files. Each terrain type gets one base tile; variant tiles (used for different
visual styles within the same terrain) live alongside.

## Automated Import (CLI)

Use the `import-tiles` Rust tool to slice a spritesheet, auto-detect collision
masks, and generate `terrain_types.json` + `tile_sets.json` in one pass:

```bash
# inside tools/import-tiles/
cargo run -- --config path/to/config.json
```

The config maps grid positions to terrain keys, defines tile sets, and controls
auto-detection parameters (luminance threshold for mask detection, terrain
classification by brightness).

Output goes to `editor/assets/imported/`:
- `tiles/*.png` — individual cropped tiles
- `terrain_types.json` — terrain definitions with auto-detected masks
- `tile_sets.json` — tile set groupings (e.g. 2×1 cliff)

Move or copy the results into this directory as needed.

## Manual Import (Godot 4 IDE)

1. Drag a 32×32 px PNG into this folder.
2. In the **Import** dock, set **Detect 3D** → off, **Compress/Mode** →
   **Lossless**, **Filter** → **Nearest** (pixel-art).
3. Click **Reimport**.
4. The `.import` file is auto-generated.
5. Register the tile in the editor (terrain editor script → terrain type list).

## Convention

- File name: `<terrain_key>_<variant>_32x32.png`
  - e.g. `grass_0_12_32x32.png`, `stone_2x7_32x32.png`
- Variant-less defaults keep the terrain key only: `grass_32x32.png`
- Godot import settings: Lossless + Nearest filter always.
