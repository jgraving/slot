use slot::{kid_menu::Menu, kid_policy::Policy};
use slot_input::{Action, Btn};
use slot_ui::UndoFace;
fn save(name: &str, face: UndoFace) {
    let Ok(dir) = std::env::var("KID_PREVIEWS") else {
        return;
    };
    let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut png = png::Encoder::new(file, face.w, face.h);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(&face.rgba)
        .unwrap();
}
#[test]
fn parent_screens_render_at_device_resolution() {
    let d = tempfile::tempdir().unwrap();
    let mut m = Menu::new(d.path().into(), Policy::default());
    assert_eq!(
        (m.screen_face().w, m.screen_face().h),
        (slot_ui::OUT_W, slot_ui::OUT_H)
    );
    save("choose-code", m.screen_face());
    m.input(Action::GbaDown(Btn::A), 1_800_000_000);
    m.input(Action::GbaDown(Btn::B), 1_800_000_000);
    save("enter-code", m.screen_face());
    for b in [
        Btn::X,
        Btn::Y,
        Btn::Start,
        Btn::A,
        Btn::B,
        Btn::X,
        Btn::Y,
        Btn::Start,
    ] {
        m.input(Action::GbaDown(b), 1_800_000_000);
    }
    save("parent-options", m.screen_face());
    save(
        "remove-favorite",
        slot::bluetooth::screen_face(&[
            "Remove favorite?".into(),
            "Example game".into(),
            "COMPACT".into(),
            "SELECTED Keep favorite".into(),
            "ROW Remove favorite".into(),
            "B Back   A Select".into(),
        ]),
    );
    save(
        "cartridge-back",
        slot_ui::cartridge_back_face(slot_store::Platform::Gba),
    );
}
