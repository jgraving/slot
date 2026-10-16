use slot::bluetooth::screen_face;
use slot_ui::{quick_pointer_face, OUT_W};

#[test]
fn sound_uses_the_shared_pointer_over_a_transparent_menu_background() {
    let lines = [
        "",
        "",
        "COMPACT",
        "ROW SPEAKER_DEVICE Internal speaker",
        "SELECTED DEVICE Headphones",
        "ROW BLUETOOTH On",
        "B Back · A Connect · Y Forget",
    ]
    .map(str::to_owned);
    let face = screen_face(&lines);
    let pointer = quick_pointer_face();
    for y in 0..pointer.h {
        for x in 0..pointer.w {
            let a = ((y * pointer.w + x) * 4) as usize;
            let b = (((140 + y) * OUT_W + 18 + x) * 4) as usize;
            if pointer.rgba[a + 3] > 0 {
                assert_eq!(&face.rgba[b..b + 4], &pointer.rgba[a..a + 4]);
            }
        }
    }
    // No old full-width highlight or opaque screen hiding the wallpaper/game.
    for y in 88..400 {
        assert_eq!(face.rgba[((y * OUT_W + 710) * 4 + 3) as usize], 0);
    }
    assert!(face.rgba.chunks_exact(4).any(|p| p == [154, 154, 164, 255]));
    // The central slot opening remains clear between the two footer groups.
    for y in 422..480 {
        for x in 300..400 {
            assert_eq!(face.rgba[((y * OUT_W + x) * 4 + 3) as usize], 0);
        }
    }
}

#[test]
fn long_device_names_and_statuses_fit_without_overflowing_the_screen() {
    let lines = [
        "",
        "Error: Bluetooth could not connect. Try again.",
        "COMPACT",
        "SELECTED ACTIVE DEVICE An extremely long Bluetooth headphone name that must fit",
        "ROW USB_DEVICE USB headphones",
        "ROW Pair audio device",
        "ROW DISCONNECT 100%",
        "ROW BLUETOOTH Off",
        "ROW More devices",
        "B Back · Change",
    ]
    .map(str::to_owned);
    let face = screen_face(&lines);
    assert_eq!(face.rgba.len(), (face.w * face.h * 4) as usize);
    assert!(face.rgba.chunks_exact(4).any(|p| p[3] > 0));
}

#[test]
fn device_icons_have_equal_visible_heights_and_the_check_uses_the_value_edge() {
    use slot_ui::{sound_device_icon, Icon};
    let bounds = |face: &slot_ui::UndoFace| {
        let points: Vec<_> = face
            .rgba
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, p)| p[3] > 32)
            .map(|(i, _)| i as u32 / face.w)
            .collect();
        (*points.iter().min().unwrap(), *points.iter().max().unwrap())
    };
    let hp = sound_device_icon(Icon::Headphones, [255; 3]);
    for icon in [Icon::Bluetooth, Icon::Volume] {
        let face = sound_device_icon(icon, [255; 3]);
        assert_eq!(face.h, hp.h);
        let (a, b) = bounds(&face);
        let (c, d) = bounds(&hp);
        assert!(((b - a) as i32 - (d - c) as i32).abs() <= 1);
    }
    let lines = [
        "",
        "",
        "COMPACT",
        "ROW ACTIVE SPEAKER_DEVICE Internal speaker",
        "ROW DISCONNECT 0%",
        "ROW BLUETOOTH On",
        "B Back · A Select",
    ]
    .map(str::to_owned);
    let face = screen_face(&lines);
    let right = |top| {
        (600..720)
            .filter(|&x| (top..top + 40).any(|y| face.rgba[((y * 720 + x) * 4 + 3) as usize] > 127))
            .max()
            .unwrap()
    };
    assert_eq!(right(92), 687);
    assert!((right(92) as i32 - right(144) as i32).abs() <= 1);
    assert!((right(92) as i32 - right(196) as i32).abs() <= 1);
}

#[test]
fn progress_ring_animates_inside_a_fixed_small_transparent_box() {
    let first = slot::bluetooth::progress_face(0);
    let next = slot::bluetooth::progress_face(1);
    assert_eq!((first.w, first.h), (32, 32));
    assert_ne!(first.rgba, next.rgba);
    assert_eq!(first.rgba, slot::bluetooth::progress_face(12).rgba);
    assert_eq!(first.rgba[((16 * 32 + 16) * 4 + 3) as usize], 0);
    assert!(first.rgba.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
}
