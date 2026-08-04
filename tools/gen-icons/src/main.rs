use image::{Rgba, RgbaImage};

fn alpha(c: Rgba<u8>, a: u8) -> Rgba<u8> {
    Rgba([c.0[0], c.0[1], c.0[2], a])
}

const SZ: u32 = 32;

fn main() {
    let out_tiles = "editor/assets/tiles";
    let out_entities = "editor/assets/entities";
    std::fs::create_dir_all(out_tiles).ok();
    std::fs::create_dir_all(out_entities).ok();

    for (name, gen) in TILES {
        let mut img = RgbaImage::new(SZ, SZ);
        gen(&mut img);
        img.save(format!("{}/{}_{}x{}.png", out_tiles, name, SZ, SZ))
            .unwrap();
        println!(
            "  tile  {:<12} → {}/{}_{}x{}.png",
            name, out_tiles, name, SZ, SZ
        );
    }
    for (name, gen) in ENTITIES {
        let mut img = RgbaImage::new(SZ, SZ);
        gen(&mut img);
        img.save(format!("{}/{}_{}x{}.png", out_entities, name, SZ, SZ))
            .unwrap();
        println!(
            "  ent   {:<12} → {}/{}_{}x{}.png",
            name, out_entities, name, SZ, SZ
        );
    }
}

fn px(img: &mut RgbaImage, x: i32, y: i32, c: Rgba<u8>) {
    if x >= 0 && x < SZ as i32 && y >= 0 && y < SZ as i32 {
        img.put_pixel(x as u32, y as u32, c);
    }
}

fn fill(img: &mut RgbaImage, x: i32, y: i32, w: i32, h: i32, c: Rgba<u8>) {
    for dy in 0..h {
        for dx in 0..w {
            px(img, x + dx, y + dy, c);
        }
    }
}

fn hline(img: &mut RgbaImage, x: i32, y: i32, w: i32, c: Rgba<u8>) {
    fill(img, x, y, w, 1, c);
}

fn vline(img: &mut RgbaImage, x: i32, y: i32, h: i32, c: Rgba<u8>) {
    fill(img, x, y, 1, h, c);
}

fn circle(img: &mut RgbaImage, cx: i32, cy: i32, r: i32, c: Rgba<u8>) {
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                px(img, cx + dx, cy + dy, c);
            }
        }
    }
}

fn circle_outline(img: &mut RgbaImage, cx: i32, cy: i32, r: i32, c: Rgba<u8>) {
    for dy in -r..=r {
        for dx in -r..=r {
            let d = dx * dx + dy * dy;
            if d > (r - 1) * (r - 1) && d <= r * r {
                px(img, cx + dx, cy + dy, c);
            }
        }
    }
}

fn line(img: &mut RgbaImage, x0: i32, y0: i32, x1: i32, y1: i32, c: Rgba<u8>) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;
    loop {
        px(img, x, y, c);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn diamond(img: &mut RgbaImage, cx: i32, cy: i32, r: i32, c: Rgba<u8>) {
    for dy in -r..=r {
        let w = r - dy.abs();
        for dx in -w..=w {
            px(img, cx + dx, cy + dy, c);
        }
    }
}

fn rect_outline(img: &mut RgbaImage, x: i32, y: i32, w: i32, h: i32, c: Rgba<u8>) {
    hline(img, x, y, w, c);
    hline(img, x, y + h - 1, w, c);
    vline(img, x, y, h, c);
    vline(img, x + w - 1, y, h, c);
}

fn gear_simple(img: &mut RgbaImage, cx: i32, cy: i32, r: i32, c: Rgba<u8>) {
    circle(img, cx, cy, r, c);
    for i in 0..6 {
        let a = i as f64 * std::f64::consts::TAU / 6.0;
        let tx = (a.cos() * (r + 1) as f64) as i32 + cx;
        let ty = (a.sin() * (r + 1) as f64) as i32 + cy;
        px(img, tx, ty, c);
        let tx2 = (a.cos() * (r + 2) as f64) as i32 + cx;
        let ty2 = (a.sin() * (r + 2) as f64) as i32 + cy;
        px(img, tx2, ty2, c);
    }
}

// ===== PALETTE =====
const BRASS: Rgba<u8> = Rgba([196, 160, 80, 255]);
const BRASS_LIGHT: Rgba<u8> = Rgba([220, 190, 120, 255]);
const COPPER: Rgba<u8> = Rgba([184, 115, 51, 255]);
const COPPER_LIGHT: Rgba<u8> = Rgba([210, 140, 70, 255]);
const DARK_IRON: Rgba<u8> = Rgba([80, 75, 70, 255]);
const LIGHT_IRON: Rgba<u8> = Rgba([160, 155, 150, 255]);
const GOLD: Rgba<u8> = Rgba([220, 190, 60, 255]);
const STEAM: Rgba<u8> = Rgba([200, 210, 210, 255]);
const OIL: Rgba<u8> = Rgba([30, 30, 35, 255]);
const PIPE: Rgba<u8> = Rgba([200, 100, 50, 255]);
const GLOW: Rgba<u8> = Rgba([255, 160, 30, 255]);
const GEAR_SILVER: Rgba<u8> = Rgba([180, 180, 190, 255]);
const SHADOW: Rgba<u8> = Rgba([40, 38, 35, 255]);
const SKY_BLUE: Rgba<u8> = Rgba([100, 170, 220, 255]);
const GRASS_GREEN: Rgba<u8> = Rgba([60, 110, 50, 255]);
const GRASS_LIGHT: Rgba<u8> = Rgba([80, 140, 65, 255]);
const DIRT_BROWN: Rgba<u8> = Rgba([120, 80, 50, 255]);
const DIRT_DARK: Rgba<u8> = Rgba([80, 55, 35, 255]);
const STONE_GRAY: Rgba<u8> = Rgba([122, 122, 122, 255]);
const STONE_LIGHT: Rgba<u8> = Rgba([150, 150, 150, 255]);
const WALL_DARK: Rgba<u8> = Rgba([70, 65, 60, 255]);
const WATER_BLUE: Rgba<u8> = Rgba([40, 100, 180, 255]);
const WATER_LIGHT: Rgba<u8> = Rgba([60, 130, 210, 255]);
const LAVA_ORANGE: Rgba<u8> = Rgba([255, 69, 0, 255]);
const LAVA_YELLOW: Rgba<u8> = Rgba([255, 200, 50, 255]);
const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
const BROWN: Rgba<u8> = Rgba([100, 60, 30, 255]);
const RED: Rgba<u8> = Rgba([200, 40, 40, 255]);
const DARK_GREEN: Rgba<u8> = Rgba([30, 70, 25, 255]);
const DARK_WATER: Rgba<u8> = Rgba([20, 60, 120, 255]);
const METAL_DARK: Rgba<u8> = Rgba([55, 52, 48, 255]);

type Gen = fn(&mut RgbaImage);

const TILES: &[(&str, Gen)] = &[
    ("grass", gen_grass),
    ("dirt", gen_dirt),
    ("stone", gen_stone),
    ("wall", gen_wall_tile),
    ("water", gen_water),
    ("lava", gen_lava),
    ("air", gen_air),
];

const ENTITIES: &[(&str, Gen)] = &[
    ("arrow_tower", gen_arrow_tower),
    ("ballista", gen_ballista),
    ("catapult", gen_catapult),
    ("wall", gen_wall_entity),
    ("bridge", gen_bridge),
    ("tarpit", gen_tarpit),
    ("mine", gen_mine),
];

// =========================================================================
// TILES
// =========================================================================

fn gen_grass(img: &mut RgbaImage) {
    fill(img, 0, 0, 32, 32, GRASS_GREEN);
    // grass texture noise
    for y in 0..32 {
        for x in 0..32 {
            if (x + y * 3) % 7 == 0 {
                px(img, x, y, GRASS_LIGHT);
            }
            if (x * 2 + y) % 9 == 0 {
                px(img, x, y, DARK_GREEN);
            }
        }
    }
    // steam vent bottom-left
    circle(img, 5, 27, 3, DARK_IRON);
    circle(img, 5, 27, 2, METAL_DARK);
    fill(img, 4, 24, 3, 3, STEAM);
    circle(img, 3, 22, 2, alpha(STEAM, 150));
    circle(img, 7, 20, 2, alpha(STEAM, 120));
    // brass gear embedded top-right
    gear_simple(img, 24, 8, 4, BRASS);
    circle(img, 24, 8, 2, METAL_DARK);
}

fn gen_dirt(img: &mut RgbaImage) {
    fill(img, 0, 0, 32, 32, DIRT_BROWN);
    // dirt texture
    for y in 0..32 {
        for x in 0..32 {
            if (x + y * 5) % 11 == 0 {
                px(img, x, y, DIRT_DARK);
            }
            if (x * 3 + y * 2) % 13 == 0 {
                px(img, x, y, Rgba([140, 100, 60, 255]));
            }
        }
    }
    // horizontal pipe
    fill(img, 0, 10, 32, 4, PIPE);
    hline(img, 0, 10, 32, COPPER);
    hline(img, 0, 13, 32, COPPER);
    // rivets on pipe
    for x in (3..32).step_by(7) {
        rx(img, x, 11);
        rx(img, x, 12);
    }
    // small pipe intersection
    fill(img, 14, 0, 4, 14, PIPE);
    vline(img, 14, 0, 14, COPPER);
    vline(img, 17, 0, 14, COPPER);
    // valve wheel
    circle(img, 16, 5, 3, BRASS);
    circle(img, 16, 5, 1, GOLD);
}

fn rx(img: &mut RgbaImage, x: i32, y: i32) {
    px(img, x, y, GEAR_SILVER);
    px(img, x, y, GOLD);
}

fn gen_stone(img: &mut RgbaImage) {
    fill(img, 0, 0, 32, 32, STONE_GRAY);
    // cobblestone pattern
    for by in 0..4 {
        for bx in 0..4 {
            let ox = bx * 8;
            let oy = by * 8;
            let shade = if (bx + by) % 2 == 0 {
                STONE_LIGHT
            } else {
                STONE_GRAY
            };
            fill(img, ox + 1, oy + 1, 6, 6, shade);
            rect_outline(img, ox, oy, 8, 8, DARK_IRON);
        }
    }
    // metal band across middle
    fill(img, 0, 13, 32, 6, METAL_DARK);
    hline(img, 0, 13, 32, BRASS);
    hline(img, 0, 18, 32, BRASS);
    // rivets on band
    for x in (2..32).step_by(7) {
        px(img, x, 14, GEAR_SILVER);
        px(img, x, 14, GOLD);
        px(img, x, 17, GEAR_SILVER);
        px(img, x, 17, GOLD);
    }
}

fn gen_wall_tile(img: &mut RgbaImage) {
    fill(img, 0, 0, 32, 32, WALL_DARK);
    // brick pattern
    for by in 0..8 {
        for bx in 0..4 {
            let ox = bx * 8 + (by % 2) * 4;
            let oy = by * 4;
            let c = if (bx + by) % 2 == 0 {
                METAL_DARK
            } else {
                Rgba([90, 85, 80, 255])
            };
            fill(img, ox + 1, oy + 1, 6, 2, c);
        }
    }
    // brass gear center
    gear_simple(img, 16, 16, 6, BRASS);
    circle(img, 16, 16, 3, METAL_DARK);
    // corner rivets
    for x in [2, 26] {
        for y in [2, 26] {
            px(img, x, y, GEAR_SILVER);
            px(img, x, y, GOLD);
        }
    }
}

fn gen_water(img: &mut RgbaImage) {
    fill(img, 0, 0, 32, 32, WATER_BLUE);
    // wave bands
    for y in [4, 10, 16, 22, 28] {
        for x in 0..32 {
            if (x + y / 3) % 4 < 2 {
                px(img, x, y, WATER_LIGHT);
            }
        }
    }
    // steam rising
    circle(img, 6, 6, 3, alpha(STEAM, 160));
    circle(img, 10, 3, 2, alpha(STEAM, 120));
    circle(img, 24, 8, 3, alpha(STEAM, 140));
    circle(img, 28, 4, 2, alpha(STEAM, 100));
    // submerged pipe
    fill(img, 8, 22, 16, 3, PIPE);
    hline(img, 8, 22, 16, COPPER);
    hline(img, 8, 24, 16, COPPER);
    // deep water bottom
    fill(img, 0, 28, 32, 4, DARK_WATER);
}

fn gen_lava(img: &mut RgbaImage) {
    fill(img, 0, 0, 32, 32, LAVA_ORANGE);
    // glow pattern
    for y in 0..32 {
        for x in 0..32 {
            if (x * 3 + y * 7) % 5 == 0 {
                px(img, x, y, LAVA_YELLOW);
            }
            if (x * 5 + y * 2) % 8 == 0 {
                px(img, x, y, GLOW);
            }
        }
    }
    // dark crust edges
    hline(img, 0, 0, 32, Rgba([60, 20, 0, 255]));
    hline(img, 0, 31, 32, Rgba([60, 20, 0, 255]));
    vline(img, 0, 0, 32, Rgba([60, 20, 0, 255]));
    vline(img, 31, 0, 32, Rgba([60, 20, 0, 255]));
    // metal grate submerged
    for x in (2..30).step_by(4) {
        vline(img, x, 12, 8, METAL_DARK);
    }
    hline(img, 2, 15, 28, METAL_DARK);
    // glow highlights on grate
    px(img, 8, 13, GLOW);
    px(img, 16, 14, GLOW);
    px(img, 24, 13, GLOW);
}

fn gen_air(img: &mut RgbaImage) {
    fill(img, 0, 0, 32, 32, SKY_BLUE);
    // clouds
    circle(img, 8, 10, 5, alpha(WHITE, 180));
    circle(img, 16, 8, 4, alpha(WHITE, 160));
    circle(img, 24, 14, 5, alpha(WHITE, 140));
    circle(img, 10, 20, 3, alpha(WHITE, 120));
    circle(img, 22, 24, 4, alpha(WHITE, 100));
    // small airship silhouette
    fill(img, 10, 18, 12, 3, METAL_DARK);
    circle(img, 16, 16, 3, DARK_IRON);
    fill(img, 15, 19, 2, 4, METAL_DARK);
    // gondola
    fill(img, 13, 21, 6, 2, COPPER);
}

// =========================================================================
// ENTITIES
// =========================================================================

fn gen_arrow_tower(img: &mut RgbaImage) {
    // Main tower body
    fill(img, 8, 4, 16, 24, BRASS);
    // Darker side edges
    fill(img, 8, 4, 3, 24, COPPER);
    fill(img, 21, 4, 3, 24, COPPER);
    // Top crenellations (battlements)
    fill(img, 6, 0, 4, 5, BRASS);
    fill(img, 12, 0, 4, 5, BRASS);
    fill(img, 18, 0, 4, 5, BRASS);
    fill(img, 24, 0, 4, 5, BRASS);
    // Arrow slit
    fill(img, 14, 8, 4, 10, SHADOW);
    fill(img, 15, 9, 2, 8, DARK_IRON);
    // Gear mechanism at top
    gear_simple(img, 16, 6, 3, GOLD);
    // Stone base
    fill(img, 6, 26, 20, 6, STONE_GRAY);
    hline(img, 6, 26, 20, DARK_IRON);
    fill(img, 6, 27, 20, 5, STONE_LIGHT);
    // Base rivets
    for x in [9, 15, 21] {
        px(img, x, 28, GEAR_SILVER);
        px(img, x, 28, GOLD);
    }
    // Platform edge highlight
    hline(img, 6, 31, 20, DARK_IRON);
}

fn gen_ballista(img: &mut RgbaImage) {
    // Frame body
    fill(img, 2, 12, 28, 8, BRASS);
    fill(img, 2, 12, 3, 8, COPPER);
    fill(img, 27, 12, 3, 8, COPPER);
    // Bow arms (angled)
    line(img, 2, 6, 12, 12, DARK_IRON);
    line(img, 30, 6, 20, 12, DARK_IRON);
    line(img, 2, 26, 12, 20, DARK_IRON);
    line(img, 30, 26, 20, 20, DARK_IRON);
    // Bowstring
    vline(img, 16, 4, 24, alpha(WHITE, 200));
    // Winding gear
    gear_simple(img, 16, 16, 5, GOLD);
    circle(img, 16, 16, 3, METAL_DARK);
    // Wheels
    circle(img, 7, 29, 3, DARK_IRON);
    circle(img, 25, 29, 3, DARK_IRON);
    circle(img, 7, 29, 1, GEAR_SILVER);
    circle(img, 25, 29, 1, GEAR_SILVER);
    // Bolt on top
    fill(img, 14, 5, 4, 2, SHADOW);
    px(img, 15, 5, GEAR_SILVER);
    px(img, 16, 5, GEAR_SILVER);
}

fn gen_catapult(img: &mut RgbaImage) {
    // Boiler drum
    circle(img, 16, 20, 10, COPPER);
    circle_outline(img, 16, 20, 10, BRASS);
    // Gauge face
    circle(img, 16, 20, 5, DARK_IRON);
    circle(img, 16, 20, 3, LIGHT_IRON);
    // Needle
    line(img, 16, 20, 18, 17, GLOW);
    // Throwing arm
    fill(img, 14, 3, 4, 15, DARK_IRON);
    fill(img, 14, 3, 4, 2, BRASS);
    // Bucket at top
    fill(img, 9, 0, 14, 4, DARK_IRON);
    fill(img, 9, 0, 3, 4, BRASS);
    fill(img, 20, 0, 3, 4, BRASS);
    fill(img, 11, 0, 10, 1, BRASS);
    // Steam
    circle(img, 4, 14, 3, alpha(STEAM, 180));
    circle(img, 3, 8, 2, alpha(STEAM, 140));
    circle(img, 28, 12, 3, alpha(STEAM, 160));
    // Wheels
    circle(img, 9, 29, 3, DARK_IRON);
    circle(img, 23, 29, 3, DARK_IRON);
    circle(img, 9, 29, 1, GEAR_SILVER);
    circle(img, 23, 29, 1, GEAR_SILVER);
    // Fire
    px(img, 14, 30, GLOW);
    px(img, 16, 30, GLOW);
    px(img, 18, 30, GLOW);
    px(img, 15, 31, LAVA_ORANGE);
    px(img, 17, 31, LAVA_ORANGE);
}

fn gen_wall_entity(img: &mut RgbaImage) {
    // Iron barricade body
    fill(img, 2, 0, 28, 32, DARK_IRON);
    // Top rail
    fill(img, 2, 0, 28, 3, BRASS);
    // Horizontal bands
    for y in [8, 19, 28] {
        hline(img, 2, y, 28, SHADOW);
        hline(img, 2, y + 1, 28, LIGHT_IRON);
    }
    // Vertical bars
    for x in (3..29).step_by(6) {
        vline(img, x, 3, 26, LIGHT_IRON);
        vline(img, x + 1, 3, 26, SHADOW);
    }
    // Rivets
    for y in [5, 14, 24] {
        for x in (4..29).step_by(6) {
            px(img, x, y, GEAR_SILVER);
            px(img, x, y, GOLD);
        }
    }
    // Bottom spikes
    for x in [5, 11, 17, 23] {
        px(img, x, 31, LIGHT_IRON);
        px(img, x + 1, 30, LIGHT_IRON);
    }
}

fn gen_bridge(img: &mut RgbaImage) {
    // Arch
    for r in [14, 13, 12, 11] {
        for dx in -r..=r {
            let dy = -(r as f64 * r as f64 - dx as f64 * dx as f64).sqrt().round() as i32;
            if dy < 0 {
                px(
                    img,
                    16 + dx,
                    28 + dy,
                    if r == 14 || r == 11 { BRASS } else { COPPER },
                );
            }
        }
    }
    // Deck
    fill(img, 1, 19, 30, 4, DARK_IRON);
    hline(img, 1, 19, 30, BRASS);
    hline(img, 1, 22, 30, BRASS);
    // Deck planks
    for x in (2..30).step_by(4) {
        vline(img, x, 20, 2, SHADOW);
    }
    // Rail pipes
    hline(img, 4, 10, 24, PIPE);
    hline(img, 4, 11, 24, GOLD);
    // Rail posts
    for x in (4..29).step_by(8) {
        vline(img, x, 6, 14, DARK_IRON);
        circle(img, x, 6, 2, BRASS);
    }
    // Gear center
    gear_simple(img, 16, 15, 4, GOLD);
}

fn gen_tarpit(img: &mut RgbaImage) {
    // Oil pool
    circle(img, 16, 18, 14, OIL);
    circle_outline(img, 16, 18, 14, Rgba([50, 48, 45, 255]));
    // Sheen rings
    circle_outline(img, 16, 18, 11, Rgba([80, 60, 50, 100]));
    circle_outline(img, 16, 18, 7, Rgba([60, 50, 40, 80]));
    // Bubbles
    circle(img, 8, 12, 2, Rgba([60, 55, 50, 200]));
    circle(img, 22, 14, 1, Rgba([60, 55, 50, 200]));
    circle(img, 14, 26, 2, Rgba([60, 55, 50, 200]));
    circle(img, 24, 24, 1, Rgba([60, 55, 50, 200]));
    // Highlight
    px(img, 10, 10, Rgba([80, 70, 60, 100]));
    // Submerged gear
    gear_simple(img, 8, 10, 3, alpha(BRASS, 180));
    // Steam
    circle(img, 20, 4, 3, alpha(STEAM, 140));
    circle(img, 10, 3, 2, alpha(STEAM, 100));
}

fn gen_mine(img: &mut RgbaImage) {
    // Outer ring
    circle(img, 16, 16, 14, BRASS);
    circle(img, 16, 16, 12, COPPER);
    circle_outline(img, 16, 16, 14, SHADOW);
    // Gauge face
    circle(img, 16, 16, 10, DARK_IRON);
    circle(img, 16, 16, 8, LIGHT_IRON);
    // Markings
    for i in 0..12 {
        let a = i as f64 * std::f64::consts::TAU / 12.0 - std::f64::consts::FRAC_PI_2;
        let r1 = 6.5;
        let r2 = 8.0;
        let x1 = (a.cos() * r1) as i32 + 16;
        let y1 = (a.sin() * r1) as i32 + 16;
        let x2 = (a.cos() * r2) as i32 + 16;
        let y2 = (a.sin() * r2) as i32 + 16;
        px(img, x1, y1, WHITE);
        px(img, x2, y2, WHITE);
    }
    // Needle
    for r in 0..6 {
        let x = (std::f64::consts::FRAC_PI_4.cos() * r as f64) as i32 + 16;
        let y = (std::f64::consts::FRAC_PI_4.sin() * r as f64) as i32 + 16;
        px(img, x, y, GLOW);
    }
    // Center axle
    circle(img, 16, 16, 2, GOLD);
    px(img, 16, 16, SHADOW);
    // Red danger zone
    for i in 8..12 {
        let a = i as f64 * std::f64::consts::TAU / 12.0 - std::f64::consts::FRAC_PI_2;
        let x = (a.cos() * 7.0) as i32 + 16;
        let y = (a.sin() * 7.0) as i32 + 16;
        px(img, x, y, LAVA_ORANGE);
    }
    // Outer rivets
    for i in 0..8 {
        let a = i as f64 * std::f64::consts::TAU / 8.0;
        let x = (a.cos() * 12.0) as i32 + 16;
        let y = (a.sin() * 12.0) as i32 + 16;
        px(img, x, y, GEAR_SILVER);
        px(img, x, y, GOLD);
    }
    // Pressure plate
    fill(img, 10, 28, 12, 4, DARK_IRON);
    fill(img, 12, 28, 8, 1, BRASS);
    px(img, 14, 30, GOLD);
    px(img, 16, 30, GOLD);
    px(img, 18, 30, GOLD);
}
