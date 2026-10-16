mod common;
use slot::{app::App, audio::Sfx};
use slot_input::Action;

#[test]
fn click_occurs_once_at_the_off_detent_without_extending_shutdown() {
    let d = common::tmp_root_with_carts(&["Alpha"]);
    common::clocked(d.path());
    let mut a = App::boot(d.path());
    a.apply(Action::PowerHold);
    let start = a.now();
    a.tick_ms(start + slot_ui::POWER_SWITCH_SLIDE_MS - 1);
    assert!(a.take_sfx().is_none());
    a.tick_ms(start + slot_ui::POWER_SWITCH_SLIDE_MS);
    assert_eq!(a.take_sfx(), Some(Sfx::PowerSwitch));
    a.tick_ms(start + slot_ui::POWER_SWITCH_SLIDE_MS + 1);
    assert!(a.take_sfx().is_none());
    a.tick_ms(start + slot_ui::POWER_SWITCH_MS);
    assert!(a.ready_to_power_off());
}

#[test]
fn switch_click_is_short_stereo_and_has_no_clipped_samples() {
    for rate in [32768, 44100, 48000] {
        let samples = Sfx::PowerSwitch.render(rate);
        assert_eq!(samples.len(), (rate as f32 * 0.028) as usize * 2);
        assert!(samples.chunks_exact(2).all(|p| p[0] == p[1]));
        let peak = samples.iter().map(|v| v.unsigned_abs()).max().unwrap();
        assert!((1000..24000).contains(&peak));
        assert!(samples.first().unwrap().abs() < 10);
        assert!(samples.last().unwrap().abs() < 10);
    }
    assert!(Sfx::PowerSwitch.render(0).is_empty());
    if let Ok(path) = std::env::var("SLOT_POWER_CLICK_PREVIEW") {
        let pcm: Vec<u8> = Sfx::PowerSwitch
            .render(48000)
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect();
        std::fs::write(path, pcm).unwrap();
    }
}
