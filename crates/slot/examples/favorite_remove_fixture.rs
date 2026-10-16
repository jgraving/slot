//! Synthetic carousel-only fixture for the existing headless GL recorder.
use slot_store::{write_slot_state, Platform, Recent, SlotState};
fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("fixture root"));
    slot::root::ensure(&root);
    let names = ["Emerald", "Fusion", "Minish Cap"];
    for name in names {
        let mut header = vec![0u8; 256];
        let title = name.to_uppercase();
        header[0xa0..0xa0 + title.len()].copy_from_slice(title.as_bytes());
        std::fs::write(root.join("Games/GBA").join(format!("{name}.gba")), header).unwrap();
    }
    write_slot_state(&root, &SlotState {
        clock_set: true,
        shelf: Some("favorites".into()),
        favorites: names.into_iter().map(|name| Recent { platform: Platform::Gba, stem: name.into() }).collect(),
        ..SlotState::default()
    }).unwrap();
}
