use slot_ui::{icon_face, Icon, HUD_ICON_PX, HUD_INK};

#[test]
fn render_sound_icons() {
    let Ok(path) = std::env::var("SOUND_PREVIEW") else {
        return;
    };
    let lines = [
        "Sound",
        "",
        "COMPACT",
        "ROW ACTIVE DEVICE Bluetooth headphones",
        "ROW USB_DEVICE USB headphones",
        "ROW DISCONNECT 0%",
        "SELECTED BLUETOOTH Off",
        "B Back   A Select   X Save report",
    ]
    .map(str::to_owned);
    let mut face = slot::bluetooth::screen_face(&lines);
    for (i, icon) in [
        Icon::Bluetooth,
        Icon::Headphones,
        Icon::BluetoothMuted,
        Icon::HeadphonesMuted,
        Icon::VolumeMuted,
    ]
    .into_iter()
    .enumerate()
    {
        let glyph = icon_face(icon, HUD_ICON_PX, HUD_INK);
        let y0 = 315 + (i / 2) as u32 * 44;
        let x0 = 64 + (i % 2) as u32 * 320;
        for y in 0..glyph.h {
            for x in 0..glyph.w {
                let src = ((y * glyph.w + x) * 4) as usize;
                let dst = (((y0 + y) * face.w + x0 + x) * 4) as usize;
                let a = glyph.rgba[src + 3] as u32;
                for c in 0..3 {
                    face.rgba[dst + c] = ((glyph.rgba[src + c] as u32 * a
                        + face.rgba[dst + c] as u32 * (255 - a))
                        / 255) as u8;
                }
            }
        }
        for y in y0 + glyph.h / 2 - 3..y0 + glyph.h / 2 + 3 {
            for x in x0 + glyph.w + 10..x0 + 220 {
                let dst = ((y * face.w + x) * 4) as usize;
                face.rgba[dst..dst + 3].copy_from_slice(if x < x0 + 130 {
                    &HUD_INK
                } else {
                    &[65, 65, 65]
                });
            }
        }
    }
    let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), face.w, face.h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&face.rgba)
        .unwrap();
}
