mod common;

use slot::app::App;
use slot_input::Action;
use slot_power::{Battery, Charge, LedState, Platform, Power};
use slot_store::{read_slot_state, write_slot_state};
use std::path::{Path, PathBuf};
use std::time::Duration;

struct Panel {
    root: PathBuf,
    supported: bool,
    succeeds: bool,
    enabled: bool,
}
impl Platform for Panel {
    fn root(&self) -> &Path {
        &self.root
    }
    fn now(&self) -> i64 {
        1_800_000_000
    }
    fn set_clock(&mut self, _: i64) {}
    fn set_backlight(&mut self, _: u8) {}
    fn battery(&self) -> Option<Battery> {
        None
    }
    fn charge(&self) -> Charge {
        Charge::Unknown
    }
    fn set_led(&mut self, _: LedState) {}
    fn set_rumble(&mut self, _: u16) {}
    fn poweroff(&mut self) -> ! {
        panic!("unexpected poweroff")
    }
    fn restart(&mut self) -> ! {
        panic!("unexpected restart")
    }
    fn supports_high_refresh(&self) -> bool {
        self.supported
    }
    fn set_high_refresh(&mut self, high: bool) -> bool {
        self.enabled = high && self.succeeds;
        self.succeeds
    }
    fn high_refresh_enabled(&self) -> bool {
        self.enabled
    }
    fn resume_display(&mut self) {
        self.enabled &= self.succeeds;
    }
}

#[test]
fn ui_uses_actual_panel_mode_and_retains_it_after_lid_wake() {
    for (supported, succeeds, preference, high) in [
        (true, true, true, true),
        (true, true, false, false),
        (true, false, true, false),
        (false, true, true, false),
    ] {
        let d = common::tmp_root_with_carts(&["Alpha"]);
        common::clocked(d.path());
        let mut state = read_slot_state(d.path());
        state.high_refresh = preference;
        write_slot_state(d.path(), &state).unwrap();
        assert_eq!(read_slot_state(d.path()).high_refresh, preference);
        let mut app = App::boot(d.path());
        app.set_power(Power::new(
            Box::new(Panel {
                root: d.path().to_owned(),
                supported,
                succeeds,
                enabled: false,
            }),
            Duration::from_secs(180),
        ));
        let expected = Duration::from_nanos(if high { 8_333_333 } else { 16_666_667 });
        assert_eq!(app.ui_frame_clock().period, expected);
        app.apply(Action::LidClose);
        app.apply(Action::LidOpen);
        assert_eq!(app.ui_frame_clock().period, expected);
    }
}
