mod common;
use slot::{
    app::{App, Phase},
    power_settings::PowerSettings,
};
use slot_input::{Action, Btn};
use slot_power::{Battery, Charge, LedState, Platform, Power};
use slot_ui::QuickRow;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

struct SleepPlatform {
    root: PathBuf,
    calls: Arc<Mutex<Vec<Option<Duration>>>>,
    expired: bool,
}
impl Platform for SleepPlatform {
    fn set_backlight(&mut self, _: u8) {}
    fn battery(&self) -> Option<Battery> {
        None
    }
    fn charge(&self) -> Charge {
        Charge::Unknown
    }
    fn set_led(&mut self, _: LedState) {}
    fn poweroff(&mut self) -> ! {
        panic!("unexpected poweroff")
    }
    fn restart(&mut self) -> ! {
        panic!("unexpected restart")
    }
    fn root(&self) -> &Path {
        &self.root
    }
    fn now(&self) -> i64 {
        1_800_000_000
    }
    fn set_clock(&mut self, _: i64) {}
    fn set_rumble(&mut self, _: u16) {}
    fn supports_deep_sleep(&self) -> bool {
        true
    }
    fn suspend(&mut self, timeout: Option<Duration>) -> std::io::Result<bool> {
        self.calls.lock().unwrap().push(timeout);
        Ok(self.expired)
    }
}
fn setup(
    expired: bool,
    timeout: u8,
) -> (tempfile::TempDir, App, Arc<Mutex<Vec<Option<Duration>>>>) {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta"]);
    common::clocked(d.path());
    let mut c = PowerSettings::default();
    c.lid_close = 1;
    c.sleep_timeout = timeout;
    c.save(d.path()).unwrap();
    let mut a = App::boot(d.path());
    let calls = Arc::new(Mutex::new(Vec::new()));
    a.set_power(Power::new(
        Box::new(SleepPlatform {
            root: d.path().into(),
            calls: calls.clone(),
            expired,
        }),
        Duration::from_secs(180),
    ));
    (d, a, calls)
}
#[test]
fn deep_sleep_cancels_if_lid_reopens_and_power_wake_does_not_sleep_again() {
    let (_d, mut a, calls) = setup(false, 0);
    a.apply(Action::LidClose);
    assert!(a.suspend_requested());
    a.apply(Action::LidOpen);
    a.suspend_if_requested();
    assert!(calls.lock().unwrap().is_empty());
    a.apply(Action::LidClose);
    a.suspend_if_requested();
    assert_eq!(*calls.lock().unwrap(), vec![Some(Duration::from_secs(180))]);
    assert!(matches!(a.phase(), Phase::Shelf));
    a.apply(Action::PowerPress);
    a.apply(Action::PowerTap);
    assert!(matches!(a.phase(), Phase::Shelf));
}
#[test]
fn timed_wake_shuts_down_and_never_has_no_alarm() {
    let (_d, mut a, calls) = setup(true, 1);
    a.apply(Action::LidClose);
    a.suspend_if_requested();
    assert!(a.powering_off());
    assert_eq!(*calls.lock().unwrap(), vec![Some(Duration::from_secs(900))]);
    let (_d, mut a, calls) = setup(false, 3);
    a.apply(Action::LidClose);
    a.suspend_if_requested();
    assert!(!a.powering_off());
    assert_eq!(*calls.lock().unwrap(), vec![None]);
}
#[test]
fn power_page_persists_lid_choice_and_sleep_timer() {
    let (d, mut a, _) = setup(false, 0);
    a.apply(Action::QuickMenu);
    for _ in 0..QuickRow::Power.position(&[]) {
        a.apply(Action::GbaDown(Btn::Down));
    }
    assert_eq!(a.quick_menu(), Some(QuickRow::Power));
    a.apply(Action::GbaDown(Btn::A));
    assert_eq!(a.quick_menu(), Some(QuickRow::LidClose));
    a.apply(Action::GbaDown(Btn::Right));
    a.apply(Action::GbaDown(Btn::Down));
    a.apply(Action::GbaDown(Btn::Right));
    let c = PowerSettings::read(d.path());
    assert_eq!((c.lid_close, c.sleep_timeout), (2, 1));
    a.apply(Action::LidClose);
    assert!(a.powering_off());
    assert!(!a.suspend_requested());
}

#[test]
fn menu_sleep_uses_deep_sleep_and_never_timer_without_shutting_down() {
    let (_d, mut a, calls) = setup(false, 3);
    a.apply(Action::QuickMenu);
    for _ in 0..QuickRow::Power.position(&[]) {
        a.apply(Action::GbaDown(Btn::Down));
    }
    a.apply(Action::GbaDown(Btn::A));
    a.apply(Action::GbaDown(Btn::Down));
    a.apply(Action::GbaDown(Btn::Down));
    assert_eq!(a.quick_menu(), Some(QuickRow::SleepNow));
    a.apply(Action::GbaDown(Btn::A));
    assert!(a.suspend_requested());
    a.suspend_if_requested();
    assert_eq!(*calls.lock().unwrap(), vec![None]);
    assert!(!a.powering_off());
}

#[test]
fn preferences_migrate_without_overwriting_other_comfort_settings() {
    let d = common::tmp_root_with_carts(&["Alpha"]);
    let path = d.path().join("Config/comfort.txt");
    let old = "sounds=0\nlid_close=1\nsleep_timeout=2\nnight=1\n";
    std::fs::write(&path, old).unwrap();
    let mut settings = PowerSettings::read(d.path());
    assert_eq!((settings.lid_close, settings.sleep_timeout), (1, 2));
    settings.sleep_timeout = 3;
    settings.save(d.path()).unwrap();
    assert_eq!(std::fs::read_to_string(path).unwrap(), old);
    assert_eq!(PowerSettings::read(d.path()).sleep_timeout, 3);
    std::fs::write(
        d.path().join("Config/power.txt"),
        "lid_close=99\nsleep_timeout=-1\n",
    )
    .unwrap();
    assert_eq!(PowerSettings::read(d.path()), PowerSettings::default());
}

#[test]
fn unavailable_suspend_wakes_safely_and_reports_the_failure() {
    let d = common::tmp_root_with_carts(&["Alpha"]);
    common::clocked(d.path());
    PowerSettings {
        lid_close: 1,
        sleep_timeout: 0,
    }
    .save(d.path())
    .unwrap();
    let mut a = App::boot(d.path());
    a.apply(Action::LidClose);
    assert!(a.suspend_requested());
    a.suspend_if_requested();
    assert!(!a.suspend_requested());
    assert!(matches!(a.phase(), Phase::Shelf));
    assert!(!a.powering_off());
    assert_eq!(a.toast(), Some(slot_ui::Toast::SleepUnavailable));
}

#[test]
fn shutdown_cancels_a_pending_deep_sleep() {
    let (_d, mut a, calls) = setup(false, 0);
    a.apply(Action::LidClose);
    assert!(a.suspend_requested());
    a.apply(Action::PowerHold);
    assert!(a.powering_off());
    assert!(!a.suspend_requested());
    a.suspend_if_requested();
    assert!(calls.lock().unwrap().is_empty());
}
