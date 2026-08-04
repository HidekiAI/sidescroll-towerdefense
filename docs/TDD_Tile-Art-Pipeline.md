# TDD: Tile Art Pipeline — Real Image to T2I Iconization

> **Status**: Draft
> **Related**: `terrain_editor.gd`, `entity_editor.gd`, `pixel_canvas.gd`

## 1. Objective

Define a workflow for generating pixel-art tile icons from real-world reference images using text-to-image (T2I) AI. This bridges the gap between concept art (photos, sketches) and in-engine 32×32 tile sprites.

## 2. Workflow

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│ 1. Source Image  │ ──→ │ 2. T2I Generation │ ──→ │ 3. Import to     │
│ (photo / sketch) │     │ (pixel art style) │     │    PixelCanvas   │
└──────────────────┘     └──────────────────┘     └──────────────────┘
                                                           │
                                                           ↓
                                                    ┌──────────────────┐
                                                    │ 4. Manual Touchup│
                                                    │ (paint/erase)    │
                                                    └──────────────────┘
                                                           │
                                                           ↓
                                                    ┌──────────────────┐
                                                    │ 5. Export PNG    │
                                                    │ (32×32 to disk)  │
                                                    └──────────────────┘
```

### Step 1 — Source Image
- User captures or downloads a real-world reference (e.g. photo of grass, stone wall, metal grate).
- Image is cropped square and downscaled to 32×32 (or higher and T2I handles the downscale).

### Step 2 — T2I Generation
- User sends the source image to a T2I service with a prompt such as:
  > *"pixel art tile, 32x32, [description], top-down, game asset, no background"*
- The T2I model returns a 32×32 (or higher) pixel-art style image.
- **No in-editor integration** — this is an external tool step (Stable Diffusion, Midjourney, DALL-E, etc.).

### Step 3 — Import to PixelCanvas
- The generated PNG is imported into the terrain or entity editor via the **Import PNG** button (`_on_import_png`).
- If dimensions don't match `_tile_px` (32), `Image.resize(32, 32, INTERPOLATE_NEAREST)` is applied automatically.

### Step 4 — Manual Touchup
- PixelCanvas allows pixel-by-pixel correction of any artifacts from T2I output.
- Left-click paints, right-click erases, fill tool replaces entire canvas with the terrain's solid color.

### Step 5 — Export PNG
- User clicks **Export PNG** to save the final 32×32 tile to `assets/tiles/{key}_32x32.png`.
- The tile is immediately visible in the map/placement editor grid (texture cache picks it up on next redraw).

## 3. Editor Integration Points

| Component | Role |
|-----------|------|
| `terrain_editor.gd:_on_import_png` | Opens file dialog, loads PNG, auto-resizes to 32×32 |
| `terrain_editor.gd:_on_export_png` | Saves current PixelCanvas image to `assets/tiles/{key}_32x32.png` |
| `pixel_canvas.gd:load_png` | Reads PNG from disk into the editable Image buffer |
| `pixel_canvas.gd:save_png` | Writes Image buffer to disk as PNG |
| `tile_grid_display.gd:_tile_texture` | Loads cached `Texture2D` from the PNG path; fallback to solid color |
| `tiled_preview.gd:set_image` | Shows 3×3 tiled preview for seam checking |

## 4. Prompt Template

```
pixel art tile, 32x32, {terrain_description}, top-down, 
game asset tile, seamless, no background, 
color palette: {dominant_colors}
```

**Example — Grass**:
```
pixel art tile, 32x32, grass ground with scattered small leaves, 
top-down, game asset tile, seamless, no background,
color palette: #4a7c3f #6b8e23 #3a5f0b
```

## 5. Spritesheet Pipeline (Primary)

For TileSet authoring, the primary art source is a **spritesheet** (texture atlas) rather than individual PNGs per tile. The Godot TileSet editor handles automatic slicing; TileSet entries reference their source region via `source_spritesheet` + `source_rect`.

### 5.1 Spritesheet Source Formats

| Spritesheet | Content | Auto-Detect Params |
|---|---|---|
| `assets/samples/tileset_2-2.png` | 30° grass slope, 4×2 grid (8 tiles) | margins=(104,65), texture_region_size=126×126, separation=(1,1), pitch=127 |
| `assets/samples/tileset_0-0.png` | Grass terrain, multi-row (2812×1536px) | margins=(420,237), texture_region_size=492×508, separation=(3,3) |

### 5.2 Auto-Detect Slicing

When importing into Godot's TileSet editor, auto-detect scans the spritesheet and produces tile regions. Key parameters:

| Parameter | tileset_2-2.png | tileset_0-0.png | Description |
|---|---|---|---|
| `margins` | (104, 65) | (420, 237) | Offset from top-left before first tile |
| `texture_region_size` | 126×126 | 492×508 | Size of each tile region |
| `separation` | (1, 1) | (3, 3) | Gap between tile regions |
| `pitch` | 127 | derived | Step distance between tile starts (= region + separation) |

These parameters are baked into the `.tres` TileSetGrouping files as `source_rect` values:

```gdscript
# A single tile entry resolved from auto-detect
{
    "local_x": 0, "local_y": 0,
    "source_spritesheet": "res://assets/samples/tileset_2-2.png",
    "source_rect": Rect2(104, 65, 126, 126),  # margins + region_size
    "flip_h": false,
    "flip_v": false,
}
```

### 5.3 Individual PNG Extraction (Alternative)

For standalone use (e.g., 1×1 terrain tiles used outside TileSet placement), individual PNGs can be extracted from spritesheets:

```
source_region = Rect2(
    margins.x + col × pitch,
    margins.y + row × pitch,
    texture_region_size.x,
    texture_region_size.y
)
```

The Rust CLI tool `tools/extract-tiles/` performs this extraction with `--spritesheet`, `--margins`, `--cell-size`, `--grid`, `--pitch`, `--scale` arguments. Results are saved to `assets/tiles/{name}_32x32.png`.

**Extraction resolution:** Source tiles (e.g. 492×508 from `tileset_0-0.png`) are scaled down to 32×32 using `INTERPOLATE_NEAREST` to preserve pixel-art sharpness. Larger source regions produce cleaner downscaled results.

### 5.4 Selection Guideline

| Use Case | Path |
|---|---|
| Multi-tile TileSet (slope, rock formation) | **Spritesheet** — all tiles in one atlas, `source_rect` references |
| 1×1 terrain TileSet | **Either** — spritesheet with single rect, or extracted PNG |
| Custom/solo tile art | **Individual PNG** — generated via T2I or PixelCanvas |
| Runtime material-only (no art) | **Solid color fallback** — no texture file needed |

## 6. Limitations & Future

- **No in-editor T2I client** — the editor remains offline for this step. A CLI wrapper for Stable Diffusion could be added later as an optional `scripts/` tool.
- **No batch generation** — each tile is processed individually via the Import PNG button.
- **TypeScript script override** (Phase 5+): a `preprocess.ts` hook could automate T2I API calls from within the editor bridge.
