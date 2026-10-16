//! Export the production slot layers for visual review.
use std::{fs::File, path::PathBuf};
fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, pixels) in [
        ("dark", slot_ui::slot_back()),
        ("light", slot_ui::slot_back_light()),
    ] {
        let mut enc = png::Encoder::new(
            File::create(dir.join(format!("slot-{name}.png"))).unwrap(),
            slot_ui::SLOT_W,
            slot_ui::SLOT_H,
        );
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()
            .unwrap()
            .write_image_data(pixels)
            .unwrap();
    }
}
