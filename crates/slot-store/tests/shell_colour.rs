use slot_store::{read_slot_state, write_slot_state, ShellColour, SlotState};

#[test]
fn each_finish_and_auto_survive_a_restart() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("Config")).unwrap();
    for colour in ShellColour::ALL {
        let state = SlotState { shell_colour: colour, ..SlotState::default() };
        write_slot_state(d.path(), &state).unwrap();
        assert_eq!(read_slot_state(d.path()).shell_colour, colour);
        assert_eq!(colour.step(true).step(false), colour);
    }
}

#[test]
fn old_or_invalid_preferences_follow_the_existing_theme() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("Config")).unwrap();
    let state = SlotState { appearance: slot_store::Appearance::Light, ..SlotState::default() };
    write_slot_state(d.path(), &state).unwrap();
    let path = d.path().join("Config/slot.state");
    let old: String = std::fs::read_to_string(&path).unwrap().lines()
        .filter(|line| !line.starts_with("shell_colour="))
        .map(|line| format!("{line}\n")).collect();
    for extra in ["", "shell_colour=leather\n"] {
        std::fs::write(&path, format!("{old}{extra}")).unwrap();
        let s = read_slot_state(d.path());
        assert_eq!(s.shell_colour, ShellColour::Theme);
        assert_eq!(s.shell_colour.resolved(s.appearance.light()), ShellColour::Platinum);
    }
    assert_eq!(ShellColour::Theme.resolved(false), ShellColour::Graphite);
}
