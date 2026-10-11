use slot::bluetooth::parse_devices;

#[test]
fn bluetooth_list_keeps_names_and_rejects_untrusted_addresses() {
    let devices = parse_devices("noise\nDEVICE AA:BB:CC:DD:EE:FF My headphones\nDEVICE invalid bad\nDEVICE AA:BB:CC:DD:EE:FF Duplicate\nDEVICE 00:11:22:33:44:55 Second pair\n");
    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].name, "My headphones");
    assert_eq!(devices[1].address, "00:11:22:33:44:55");
}

#[test]
fn bluetooth_is_reachable_from_settings_and_back_returns_to_settings() {
    use slot_input::{Action, Btn};
    use slot_ui::QuickRow;
    assert!(QuickRow::MAIN.contains(&QuickRow::Bluetooth));
    assert!(QuickRow::Bluetooth.opens());
    let mut manager = slot::bluetooth::Manager::new("/nonexistent".into());
    assert!(manager.input(Action::GbaDown(Btn::B)));
}

#[test]
fn mac_only_names_are_hidden_and_pairing_state_is_preserved() {
    let devices = parse_devices("DEVICE AA:BB:CC:DD:EE:FF WF-C510\nSTATE AA:BB:CC:DD:EE:FF yes no\nDEVICE 00:11:22:33:44:55 00-11-22-33-44-55\nDEVICE 00:11:22:33:44:56 001122334456\n");
    assert_eq!(devices.len(), 1);
    assert!(devices[0].paired);
    assert!(!devices[0].connected);
}

#[test]
fn highlighted_menu_and_wrapped_error_render_at_device_resolution() {
    let lines = vec![
        "Bluetooth audio".into(),
        "Error: Pairing failed: org.bluez.Error.AuthenticationFailed".into(),
        "Log: Bluetooth/latest-error.txt".into(),
        "SELECTED Pair headphones".into(),
        "ROW Saved headphones".into(),
        "ROW Audio output".into(),
        "ROW Diagnostics".into(),
        "B Back · A Select".into(),
    ];
    let face = slot::bluetooth::screen_face(&lines);
    assert_eq!((face.w, face.h), (slot_ui::OUT_W, slot_ui::OUT_H));
    assert_ne!(
        &face.rgba[(184 * slot_ui::OUT_W as usize * 4)..(184 * slot_ui::OUT_W as usize * 4 + 4)],
        &face.rgba[..4]
    );
    if let Ok(path) = std::env::var("SLOT_BT_PREVIEW") {
        std::fs::write(path, face.rgba).unwrap();
    }
}

#[test]
fn minimalist_menu_shows_native_hints_and_active_output_without_headers() {
    let lines = vec![
        String::new(),
        String::new(),
        String::new(),
        "ROW Pair headphones".into(),
        "ROW Speaker".into(),
        "SELECTED ACTIVE WF-C510".into(),
        "B Back · A Select · Y Manage".into(),
    ];
    let face = slot::bluetooth::screen_face(&lines);
    assert_eq!(
        face.rgba.len(),
        (slot_ui::OUT_W * slot_ui::OUT_H * 4) as usize
    );
    if let Ok(path) = std::env::var("SLOT_BT_PREVIEW") {
        std::fs::write(format!("{path}.minimal"), face.rgba).unwrap();
    }
}

#[test]
fn bluetooth_symbol_uses_a_small_transparent_footer_face() {
    let face = slot::bluetooth::indicator_face();
    assert_eq!((face.w, face.h), (16, 16));
    assert_eq!(face.rgba[3], 0);
    assert!(face.rgba.chunks_exact(4).any(|p| p[3] == 255));
    if let Ok(path) = std::env::var("SLOT_BT_PREVIEW") {
        std::fs::write(format!("{path}.indicator"), face.rgba).unwrap();
    }
}
