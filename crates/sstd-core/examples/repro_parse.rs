use sstd_core::storage::ScreenFile;
fn main() {
    let path = std::env::args().nth(1).expect("path");
    let json = std::fs::read_to_string(&path).unwrap();
    match ScreenFile::from_json(&json) {
        Ok(f) => println!("OK tiles={} ents={} stamps={}", f.tiles.len(), f.placed_entities.len(), f.stamp_maps.len()),
        Err(e) => println!("ERR: {e}"),
    }
}
