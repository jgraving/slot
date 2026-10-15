mod common;
use common::{app_booting_at, tmp_root_with_carts, StubSnapshot};
use slot::{
    app::{App, Phase},
    kid_policy::{Policy, Rule},
    parent_code::Gate,
};
use slot_input::{Action, Btn};
use slot_store::{write_slot_state, Platform, SlotState};
const NOW: i64 = 1_800_000_000;
fn card(rule: Rule) -> tempfile::TempDir {
    let d = tmp_root_with_carts(&["Allowed", "Other"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            cart: Some("Other".into()),
            ..Default::default()
        },
    )
    .unwrap();
    Gate::new(Policy::directory(d.path()).join("code"))
        .set(&[0, 1, 2, 3])
        .unwrap();
    let mut p = Policy::default();
    p.rule = rule;
    p.minutes = 5;
    p.approved.push((Platform::Gba, "Allowed".into()));
    p.begin(NOW).unwrap();
    p.save(d.path()).unwrap();
    d
}
fn tap(a: &mut App, b: Btn) {
    a.apply(Action::GbaDown(b));
    a.apply(Action::GbaUp(b));
}
fn launch(a: &mut App) {
    a.set_snapshot(StubSnapshot::boxed());
    a.apply(Action::Insert);
    a.on_core_ready();
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    assert!(matches!(a.phase(), Phase::Playing { .. }));
}
#[test]
fn boot_ignores_an_unapproved_resume_and_locks_shelves_and_marks() {
    let d = card(Rule::Daily);
    let (mut a, _) = app_booting_at(d.path(), NOW);
    assert!(matches!(a.phase(), Phase::Shelf));
    assert_eq!(a.selected_stem(), Some("Allowed"));
    for b in [Btn::R1, Btn::L1, Btn::X, Btn::Y, Btn::Right] {
        tap(&mut a, b);
    }
    assert_eq!(a.selected_stem(), Some("Allowed"));
    assert_eq!(a.kids.approved.len(), 1);
    assert!(!a.selected_is_favorite());
    tap(&mut a, Btn::Start);
    assert!(a.kid_menu.is_some());
    assert!(a.core_picker().is_none());
}
#[test]
fn wrong_code_does_not_unlock_and_correct_code_can_exit() {
    let d = card(Rule::Daily);
    let (mut a, _) = app_booting_at(d.path(), NOW);
    a.apply(Action::QuickMenu);
    for _ in 0..4 {
        tap(&mut a, Btn::A);
    }
    tap(&mut a, Btn::Start);
    assert!(a
        .kid_menu
        .as_ref()
        .unwrap()
        .lines()
        .iter()
        .any(|s| s == "Code not recognized"));
    for b in [Btn::A, Btn::B, Btn::X, Btn::Y, Btn::Start] {
        tap(&mut a, b);
    }
    for _ in 0..3 {
        tap(&mut a, Btn::Down);
    }
    tap(&mut a, Btn::A);
    assert!(!a.kids.active);
    assert!(a.kid_menu.is_none());
    assert!(!Policy::read(d.path()).unwrap().active);
}
#[test]
fn elapsed_session_stops_play_and_cannot_restart_without_parent() {
    let d = card(Rule::Session);
    let (mut a, clock) = app_booting_at(d.path(), NOW);
    launch(&mut a);
    clock.advance(301);
    a.update(0.016);
    assert!(matches!(a.phase(), Phase::Shelf));
    assert!(a.kid_menu.is_some());
    tap(&mut a, Btn::B);
    a.apply(Action::Insert);
    assert!(matches!(a.phase(), Phase::Shelf));
    assert!(a.kid_menu.is_some());
}
#[test]
fn waking_after_bedtime_never_resumes_play() {
    let d = card(Rule::Session);
    let (mut a, clock) = app_booting_at(d.path(), NOW);
    launch(&mut a);
    a.apply(Action::LidClose);
    clock.advance(301);
    a.apply(Action::LidOpen);
    assert!(matches!(a.phase(), Phase::Shelf));
    assert!(a.kid_menu.is_some());
}
#[test]
fn daily_budget_stops_a_game_and_survives_reboot() {
    let d = card(Rule::Daily);
    let (mut a, clock) = app_booting_at(d.path(), NOW);
    launch(&mut a);
    clock.advance(301);
    a.update(301.0);
    assert!(matches!(a.phase(), Phase::Shelf));
    let (mut a, _) = app_booting_at(d.path(), NOW + 301);
    a.apply(Action::Insert);
    assert!(matches!(a.phase(), Phase::Shelf));
    assert!(a.kid_menu.is_some());
}
#[test]
fn opening_parent_controls_pauses_game_and_mutes_audio() {
    let d = card(Rule::Daily);
    let (mut a, _) = app_booting_at(d.path(), NOW);
    launch(&mut a);
    a.apply(Action::QuickMenu);
    assert!(a.audio_paused());
    assert_eq!(a.output_volume(), 0);
}
#[test]
fn missing_code_in_active_mode_does_not_offer_a_replacement_code() {
    let d = card(Rule::Daily);
    std::fs::remove_file(Policy::directory(d.path()).join("code")).unwrap();
    let (mut a, _) = app_booting_at(d.path(), NOW);
    a.apply(Action::QuickMenu);
    assert_eq!(a.kid_menu.as_ref().unwrap().lines()[0], "Parent code");
}
