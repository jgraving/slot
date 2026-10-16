use slot::bluetooth::screen_face;
use slot_ui::{quick_pointer_face, OUT_W};

#[test]
fn sound_uses_the_shared_pointer_over_a_transparent_menu_background() {
    let lines = [
        "",
        "",
        "COMPACT",
        "ROW Internal speaker",
        "SELECTED DEVICE Headphones",
        "ROW BLUETOOTH On",
        "B Back · A Select · Y Manage · X Save report",
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
        for x in 300..420 {
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
        "B Back · Change · X Save report",
    ]
    .map(str::to_owned);
    let face = screen_face(&lines);
    assert_eq!(face.rgba.len(), (face.w * face.h * 4) as usize);
    assert!(face.rgba.chunks_exact(4).any(|p| p[3] > 0));
}
