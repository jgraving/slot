use slot_ui::{icon_box, icon_face, Icon, HUD_ICON_PX, HUD_INK};

#[test]
fn audio_icons_keep_the_hud_box_and_clean_antialiased_ink() {
    for icon in [
        Icon::Bluetooth,
        Icon::BluetoothMuted,
        Icon::Usb,
        Icon::UsbMuted,
    ] {
        let face = icon_face(icon, HUD_ICON_PX, HUD_INK);
        assert_eq!((face.w, face.h), icon_box(HUD_ICON_PX));
        assert!(face.rgba.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
        for p in face.rgba.chunks_exact(4).filter(|p| p[3] > 0) {
            assert_eq!(&p[..3], &HUD_INK, "no dark outline between strokes");
        }
        let points: Vec<_> = face
            .rgba
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, p)| p[3] > 127)
            .map(|(i, _)| (i as u32 % face.w, i as u32 / face.w))
            .collect();
        let min_y = points.iter().map(|p| p.1).min().unwrap();
        let max_y = points.iter().map(|p| p.1).max().unwrap();
        assert!((min_y as i32 - (face.h - 1 - max_y) as i32).abs() <= 2);
        if matches!(icon, Icon::Bluetooth | Icon::Usb) {
            let width = points.iter().map(|p| p.0).max().unwrap()
                - points.iter().map(|p| p.0).min().unwrap()
                + 1;
            assert!(
                width < max_y - min_y + 1,
                "upright symbol must be taller than wide"
            );
        }
    }
}
