mod common;
use slot::app::{App, Phase};
use slot::comfort::Comfort;
use slot_input::{Action, Btn, Gestures, RawEvent};
use slot_store::SlotState;
use slot_ui::QuickRow;

const FIRST: &str = "AA:BB:CC:DD:EE:01";
const SECOND: &str = "AA:BB:CC:DD:EE:02";
fn tap(a: &mut App, b: Btn) {
    a.apply(Action::GbaDown(b));
    a.apply(Action::GbaUp(b));
}

#[test]
fn browsing_restores_each_shelf_and_current_cart_without_resuming_a_game() {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta"]);
    common::write_gb_cart(&d, "Tetris", "TETRIS");
    common::write_gb_cart(&d, "Zelda", "ZELDA");
    common::clocked(d.path());
    let mut a = App::boot(d.path());
    tap(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Beta"));
    tap(&mut a, Btn::R1);
    tap(&mut a, Btn::Right);
    a.settle_saves();
    let mut b = App::boot(d.path());
    assert!(matches!(b.phase(), Phase::Shelf));
    assert_eq!(b.selected_stem(), Some("Zelda"));
    tap(&mut b, Btn::L1);
    assert_eq!(b.selected_stem(), Some("Beta"));
}
#[test]
fn missing_browsed_game_falls_back_to_an_existing_cartridge() {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta", "Gamma"]);
    common::clocked(d.path());
    let mut a = App::boot(d.path());
    tap(&mut a, Btn::Right);
    a.settle_saves();
    std::fs::remove_file(d.path().join("Games/GBA/Beta.gba")).unwrap();
    let b = App::boot(d.path());
    assert_eq!(b.selected_stem(), Some("Alpha"));
}
#[test]
fn day_and_night_keep_separate_adjustments_and_survive_reboot() {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta"]);
    common::clocked(d.path());
    let mut a = App::boot(d.path());
    let day = (a.brightness(), a.blue_light());
    a.apply(Action::NightToggle);
    assert!(a.night_mode());
    assert_eq!((a.brightness(), a.blue_light()), (2, 9));
    a.apply(Action::BrightnessUp);
    a.apply(Action::BlueLightDown);
    a.apply(Action::NightToggle);
    assert_eq!((a.brightness(), a.blue_light()), day);
    a.apply(Action::NightToggle);
    assert_eq!((a.brightness(), a.blue_light()), (3, 8));
    a.settle_saves();
    a.flush_state();
    let mut b = App::boot(d.path());
    assert!(b.night_mode());
    assert_eq!((b.brightness(), b.blue_light()), (3, 8));
    b.apply(Action::NightToggle);
    assert_eq!((b.brightness(), b.blue_light()), day);
    assert_eq!(
        b.quick_value(QuickRow::Night),
        Some(slot_ui::QuickValue::Off)
    );
}
#[test]
fn night_chord_does_not_emit_select_or_x_to_the_game() {
    let mut g = Gestures::new();
    assert!(g.feed(RawEvent::Down(Btn::Select), 0).is_empty());
    assert_eq!(
        g.feed(RawEvent::Down(Btn::X), 50),
        vec![Action::NightToggle]
    );
    assert!(g.feed(RawEvent::Up(Btn::X), 100).is_empty());
    assert!(g.feed(RawEvent::Up(Btn::Select), 150).is_empty());
}
#[test]
fn each_bluetooth_headset_keeps_its_volume_and_mute_without_changing_speaker() {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta"]);
    common::clocked(d.path());
    let mut a = App::boot(d.path());
    let speaker = a.volume();
    a.bluetooth_state(Some(FIRST.into()), true);
    a.apply(Action::VolumeDown);
    let first = a.volume();
    a.bluetooth_state(Some(SECOND.into()), true);
    a.apply(Action::VolumeUp);
    let second = a.volume();
    a.apply_at(Action::MuteToggle, 500);
    a.bluetooth_state(None, false);
    assert_eq!(a.volume(), speaker);
    assert!(!a.muted());
    a.bluetooth_state(Some(FIRST.into()), true);
    assert_eq!(a.volume(), first);
    assert!(!a.muted());
    a.settle_saves();
    let mut b = App::boot(d.path());
    b.bluetooth_state(Some(SECOND.to_ascii_lowercase()), true);
    assert_eq!(b.volume(), second);
    assert!(b.muted());
    b.bluetooth_state(Some(FIRST.into()), true);
    assert_eq!(b.volume(), first);
    assert!(!b.muted());
}
#[test]
fn quiet_disconnect_is_opt_in_and_requires_explicit_output_choice() {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta"]);
    let mut a = common::app_playing_in(d.path(), "Alpha");
    a.bluetooth_state(Some(FIRST.into()), true);
    a.bluetooth_state(Some(FIRST.into()), false);
    assert!(!a.audio_paused());
    a.set_quiet_disconnect(true);
    a.bluetooth_state(Some(FIRST.into()), true);
    a.bluetooth_state(Some(FIRST.into()), false);
    assert!(a.audio_paused());
    assert_eq!(a.output_volume(), 0);
    assert!(matches!(a.phase(), Phase::Playing { .. }));
    tap(&mut a, Btn::B);
    assert!(a.audio_paused());
    assert!(a.bluetooth.is_some());
    a.bluetooth_state(Some(FIRST.into()), true);
    assert!(
        a.audio_paused(),
        "reconnection must not resume gameplay by itself"
    );
    a.bluetooth.as_mut().unwrap().route_choice = Some("speaker".into());
    a.update(0.016);
    assert!(!a.audio_paused());
    assert!(a.bluetooth.is_none());
    assert!(a.output_volume() > 0);
    let c = Comfort::read(d.path(), &SlotState::default()).unwrap();
    assert!(c.quiet);
}
#[test]
fn intended_output_switch_and_initial_unavailable_headset_do_not_pause() {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta"]);
    let mut a = common::app_playing_in(d.path(), "Alpha");
    a.set_quiet_disconnect(true);
    a.bluetooth_state(Some(FIRST.into()), false);
    assert!(!a.audio_paused());
    assert_eq!(a.output_volume(), 0);
    a.bluetooth_state(Some(FIRST.into()), true);
    a.bluetooth_state(None, false);
    assert!(!a.audio_paused());
    assert!(a.output_volume() > 0);
}
#[test]
fn comfort_file_clamps_settings_and_never_modifies_cartridge_or_save_files() {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta"]);
    common::clocked(d.path());
    let path = d.path().join("Config/comfort.txt");
    std::fs::write(&path,"night=1\nday=250,200\nbedtime=3,8\nheadset.AA:BB:CC:DD:EE:01=200,1\nheadset.bad=90,0\nshelf=90\n").unwrap();
    let c = Comfort::read(d.path(), &SlotState::default()).unwrap();
    assert_eq!(c.day.brightness, 9);
    assert_eq!(c.headsets[FIRST].volume, 100);
    assert_eq!(c.shelf, 0);
    assert_eq!(c.headsets.len(), 1);
    let rom = d.path().join("Games/GBA/Alpha.gba");
    let before = std::fs::read(&rom).unwrap();
    c.save(d.path()).unwrap();
    assert_eq!(std::fs::read(&rom).unwrap(), before);
    assert!(!d.path().join("Saves/GBA/Alpha.sav").exists());
}

#[test]
fn disconnect_overlay_actually_pauses_the_core_and_resume_choice_releases_it() {
    use slot::emu::Speed;
    use std::time::{Duration, Instant};
    let d = common::tmp_root_with_carts(&["Alpha", "Beta"]);
    common::clocked(d.path());
    let (mut s, _) = common::session_with_platform(d.path());
    s.feed([RawEvent::Down(Btn::A)], 16);
    s.feed([RawEvent::Up(Btn::A)], 32);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !matches!(s.app().phase(), Phase::Playing { .. }) {
        assert!(Instant::now() < deadline);
        s.update(0.016);
        std::thread::sleep(Duration::from_millis(1));
    }
    s.app_mut().set_quiet_disconnect(true);
    s.app_mut().bluetooth_state(Some(FIRST.into()), true);
    s.app_mut().bluetooth_state(Some(FIRST.into()), false);
    s.update(0.016);
    let deadline = Instant::now() + Duration::from_secs(2);
    while s.observed_speed() != Some(Speed::Paused) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    s.app_mut().bluetooth.as_mut().unwrap().route_choice = Some("speaker".into());
    s.update(0.016);
    let deadline = Instant::now() + Duration::from_secs(2);
    while s.observed_speed() != Some(Speed::Normal) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}
