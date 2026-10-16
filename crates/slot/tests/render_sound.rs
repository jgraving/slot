#[test]
fn render_sound_menu() {
    let Ok(path) = std::env::var("SOUND_PREVIEW") else {
        return;
    };
    for (suffix, message) in [("", ""), ("-confirm", "CONFIRM_FORGET")] {
        let lines = [
            "",
            message,
            "COMPACT",
            "ROW SPEAKER_DEVICE Internal speaker",
            "SELECTED ACTIVE DEVICE WF-C510",
            "ROW USB_DEVICE USB headphones",
            "ROW Pair audio device",
            "ROW DISCONNECT 0%",
            "ROW BLUETOOTH On",
            "B Back · A Connect · Y Forget",
        ]
        .map(str::to_owned);
        let face = slot::bluetooth::screen_face(&lines);
        let path = format!("{path}{suffix}.png");
        let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), face.w, face.h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&face.rgba)
            .unwrap();
    }
}
