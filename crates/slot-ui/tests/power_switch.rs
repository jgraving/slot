use slot_ui::{draw_power_switch, power_switch_faces, Draw, PowerSwitchColour, TexId};

fn frame(seconds: f32, restart: bool) -> Vec<Draw> {
    let mut draws = Vec::new();
    draw_power_switch(
        seconds / if restart { 1.1 } else { 0.7 },
        restart,
        [TexId::from_raw(0), TexId::from_raw(1)],
        &mut draws,
    );
    draws
}

fn grip_mark(draws: &[Draw]) -> f32 {
    let Draw::Tex { x, w, .. } = draws[1] else {
        panic!("missing slider");
    };
    // The grip's centre mark is 159.5 units into the 324-unit slider SVG.
    x + w * 159.5 / 324.0
}

#[test]
fn detents_align_with_the_moulded_dots_and_restart_returns_to_on() {
    let on = 60.0 + 539.5 * 0.6;
    let off = 60.0 + 445.5 * 0.6;
    for restart in [false, true] {
        assert!((grip_mark(&frame(0.0, restart)) - on).abs() < 0.001);
        assert!((grip_mark(&frame(0.34, restart)) - off).abs() < 0.001);
        assert!((grip_mark(&frame(0.55, restart)) - off).abs() < 0.001);
        let middle = grip_mark(&frame(0.20, restart));
        assert!(middle > off && middle < on);
    }
    assert!((grip_mark(&frame(0.86, true)) - on).abs() < 0.001);
}

#[test]
fn both_layers_fade_to_black_together_without_changing_the_shell_geometry() {
    let start = frame(0.0, false);
    for (seconds, restart) in [(0.35, false), (0.65, false), (0.7, false), (1.1, true)] {
        let draws = frame(seconds, restart);
        assert_eq!(draws.len(), 2);
        let Draw::Tex {
            x, y, w, h, alpha, ..
        } = draws[0]
        else {
            panic!("missing shell");
        };
        assert_eq!((x, y, w, h), (60.0, 120.0, 600.0, 240.0));
        assert!((0.0..=1.0).contains(&alpha));
        assert!(matches!(draws[1], Draw::Tex { alpha: a, .. } if a == alpha));
        if seconds == 0.35 {
            assert_eq!(draws[0], start[0]);
        }
        if seconds == 0.7 || restart {
            assert_eq!(alpha, 0.0);
        }
    }
}

#[test]
fn embedded_vectors_rasterise_as_red_shell_and_transparent_charcoal_slider() {
    let [shell, slider] = power_switch_faces(PowerSwitchColour::FlameRed);
    assert_eq!((shell.w, shell.h), (600, 240));
    assert_eq!((slider.w, slider.h), (324, 102));
    for face in [&shell, &slider] {
        assert_eq!(face.rgba.len(), (face.w * face.h * 4) as usize);
        assert!(face.rgba.chunks_exact(4).any(|p| p[3] == 255));
        assert_eq!(
            face.rgba[3], 0,
            "transparent corners must remain transparent"
        );
    }
    let red = &shell.rgba[((200 * shell.w + 100) * 4) as usize..][..4];
    assert!(red[0] > red[1] * 2 && red[3] == 255);
    let grey = &slider.rgba[((40 * slider.w + 40) * 4) as usize..][..4];
    assert!(grey[0].abs_diff(grey[1]) < 20 && grey[3] == 255);
}

#[test]
fn each_finish_changes_only_the_shell_colour_and_keeps_its_transparency() {
    let [red, slider] = power_switch_faces(PowerSwitchColour::FlameRed);
    let mut colours = Vec::new();
    for colour in PowerSwitchColour::ALL {
        let [shell, grip] = power_switch_faces(colour);
        assert_eq!(slider.rgba, grip.rgba, "the physical grip stays charcoal");
        assert_eq!((shell.w, shell.h), (red.w, red.h));
        assert!(shell
            .rgba
            .chunks_exact(4)
            .zip(red.rgba.chunks_exact(4))
            .all(|(a, b)| a[3] == b[3]));
        let offset = ((200 * shell.w + 100) * 4) as usize;
        let sample = shell.rgba[offset..offset + 3].to_vec();
        assert!(
            !colours.contains(&sample),
            "each finish must be visibly different"
        );
        colours.push(sample);
    }
}

#[test]
fn boot_switch_moves_from_off_to_on_below_the_logo_without_fading() {
    let mut marks = Vec::new();
    for progress in [0.0, 0.5, 1.0, 2.0] {
        let mut out = Vec::new();
        slot_ui::draw_power_on(
            progress,
            [TexId::from_raw(0), TexId::from_raw(1)],
            &mut out,
        );
        assert!(matches!(out[0], Draw::Tex {
            x: 210.0, y: 300.0, w: 300.0, h: 120.0, alpha: 1.0, ..
        }));
        assert!(matches!(out[1], Draw::Tex { alpha: 1.0, .. }));
        marks.push(grip_mark(&out));
    }
    assert!((marks[0] - (210.0 + 445.5 * 0.3)).abs() < 0.001);
    assert!(marks[1] > marks[0] && marks[1] < marks[2]);
    assert!((marks[2] - (210.0 + 539.5 * 0.3)).abs() < 0.001);
    assert_eq!(marks[2], marks[3]);
}
