mod common;
use slot::app::App;
use slot_input::{Action, Btn, Gestures, RawEvent};
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
}
#[test]
fn night_chord_does_not_emit_select_or_x_to_the_game() {
    let mut g = Gestures::new();
    g.set_entries(slot::input::Table::new(
        &slot_store::Controls::default(), &slot_store::ShortcutKeys::default(),
        &slot_store::CarouselKeys::default(), false,
    ).entries());
    assert!(g.feed(RawEvent::Down(Btn::Select), 0).is_empty());
    assert_eq!(
        g.feed(RawEvent::Down(Btn::X), 50),
        vec![Action::NightToggle]
    );
    assert!(g.feed(RawEvent::Up(Btn::X), 100).is_empty());
    assert!(g.feed(RawEvent::Up(Btn::Select), 150).is_empty());
}

#[test]
fn migrates_only_display_fields_and_leaves_old_preferences_untouched() {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta"]);
    common::clocked(d.path());
    let old = "night=1\nday=7,2\nbedtime=1,8\nquiet=1\nlid_close=2\n";
    std::fs::write(d.path().join("Config/comfort.txt"), old).unwrap();
    let mut a = App::boot(d.path());
    assert!(a.night_mode());
    assert_eq!((a.brightness(), a.blue_light()), (1, 8));
    a.apply(Action::NightToggle);
    assert_eq!((a.brightness(), a.blue_light()), (7, 2));
    assert_eq!(
        std::fs::read_to_string(d.path().join("Config/comfort.txt")).unwrap(),
        old
    );
}
#[test]
fn invalid_display_values_leave_safe_defaults() {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta"]);
    common::clocked(d.path());
    std::fs::write(
        d.path().join("Config/display.txt"),
        "night=1\nbedtime=255,255\n",
    )
    .unwrap();
    let a = App::boot(d.path());
    assert_eq!((a.brightness(), a.blue_light()), (2, 9));
}
