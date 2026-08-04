use image::GenericImageView;
fn main() {
    let img = image::open("editor/assets/samples/tileset_1-0.png").unwrap();
    let gray = image::imageops::grayscale(&img.to_rgba8());
    let (w, h) = gray.dimensions();
    // Check first 20 columns
    for x in 0..20 {
        let (mut mn, mut mx) = (255u8, 0u8);
        for y in 0..h {
            let l = gray.get_pixel(x, y).0[0];
            mn = mn.min(l); mx = mx.max(l);
        }
        println!("col {x}: range={}", mx as i32 - mn as i32);
    }
    // Check first 10 rows
    for y in 0..10 {
        let (mut mn, mut mx) = (255u8, 0u8);
        for x in 0..w {
            let l = gray.get_pixel(x, y).0[0];
            mn = mn.min(l); mx = mx.max(l);
        }
        println!("row {y}: range={}", mx as i32 - mn as i32);
    }
    // Check column 224
    let (mut mn, mut mx) = (255u8, 0u8);
    for y in 0..h {
        let l = gray.get_pixel(224, y).0[0];
        mn = mn.min(l); mx = mx.max(l);
    }
    println!("col 224: range={}", mx as i32 - mn as i32);
}
