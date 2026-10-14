use slot::kid_policy::{Policy, Rule};
use slot_store::Platform;
const NOW: i64 = 1_800_000_000;
fn ready(rule: Rule) -> (tempfile::TempDir, Policy) {
    let d = tempfile::tempdir().unwrap();
    let mut p = Policy::default();
    p.rule = rule;
    p.minutes = 5;
    p.approved.push((Platform::Gba, "Example".into()));
    std::fs::create_dir_all(Policy::directory(d.path())).unwrap();
    std::fs::write(Policy::directory(d.path()).join("code"), b"test").unwrap();
    (d, p)
}
#[test]
fn daily_budget_survives_restart_and_pauses_do_not_consume_time() {
    let (d, mut p) = ready(Rule::Daily);
    p.begin(NOW).unwrap();
    p.tick(d.path(), NOW, 0, true).unwrap();
    p.tick(d.path(), NOW + 10, 10_000, true).unwrap();
    let before = p.remaining(NOW + 10);
    p.tick(d.path(), NOW + 10, 10_000, false).unwrap();
    p.tick(d.path(), NOW + 110, 110_000, false).unwrap();
    p.tick(d.path(), NOW + 110, 110_000, true).unwrap();
    assert_eq!(p.remaining(NOW + 110), before);
    let mut rebooted = Policy::read(d.path()).unwrap();
    assert_eq!(rebooted.remaining(NOW + 110), 240);
    assert!(rebooted.tick(d.path(), NOW + 110, 0, true).unwrap());
    assert!(rebooted.tick(d.path(), NOW + 170, 60_000, true).unwrap());
    assert!(rebooted.tick(d.path(), NOW + 230, 120_000, true).unwrap());
    assert!(rebooted.tick(d.path(), NOW + 290, 180_000, true).unwrap());
    assert!(!rebooted.tick(d.path(), NOW + 350, 240_000, true).unwrap());
}
#[test]
fn one_off_deadline_counts_sleep_and_cannot_be_refunded_by_reboot() {
    let (d, mut p) = ready(Rule::Session);
    p.begin(NOW).unwrap();
    p.save(d.path()).unwrap();
    let mut p = Policy::read(d.path()).unwrap();
    assert_eq!(p.remaining(NOW + 120), 180);
    assert!(!p.tick(d.path(), NOW + 300, 0, false).unwrap());
}
#[test]
fn bedtime_is_today_and_past_times_are_rejected() {
    let (_, mut p) = ready(Rule::Until);
    let midnight = NOW.div_euclid(86_400) * 86_400;
    p.bedtime = 19 * 60 + 30;
    p.begin(midnight + 19 * 3600).unwrap();
    assert_eq!(p.remaining(midnight + 19 * 3600), 1800);
    assert!(p.begin(midnight + 20 * 3600).is_err());
}
#[test]
fn clock_rollback_blocks_until_a_parent_authorizes_a_new_session() {
    let (d, mut p) = ready(Rule::Session);
    p.begin(NOW).unwrap();
    assert!(!p.tick(d.path(), NOW - 60, 0, true).unwrap());
    let p = Policy::read(d.path()).unwrap();
    assert_eq!(p.remaining(NOW), 0);
}
#[test]
fn failed_budget_write_does_not_grant_play() {
    let (d, mut p) = ready(Rule::Daily);
    p.begin(NOW).unwrap();
    let invalid = d.path().join("file");
    std::fs::write(&invalid, b"not a directory").unwrap();
    assert!(p.tick(&invalid, NOW, 0, true).is_err());
}
#[test]
fn malformed_or_missing_active_policy_is_not_treated_as_unrestricted() {
    let (d, _) = ready(Rule::Daily);
    assert!(Policy::read(d.path()).is_err());
    std::fs::write(Policy::directory(d.path()).join("settings"), b"broken").unwrap();
    assert!(Policy::read(d.path()).is_err());
}
