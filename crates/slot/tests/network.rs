use slot::network::{parse_scan, Manager};
use slot_input::{Action, Btn};

#[test]
fn scan_accepts_wpa2_deduplicates_and_decodes_network_names_without_controls() {
    let rows = "bssid / frequency / signal level / flags / ssid\n\
00:11\t2412\t-40\t[WPA2-PSK-CCMP][ESS]\tMy\\x20network\n\
00:12\t5180\t-50\t[WPA2-PSK-CCMP][ESS]\tMy network\n\
00:13\t2412\t-30\t[ESS]\tOpen\n\
00:14\t2412\t-30\t[WPA2-PSK-CCMP][ESS]\tInjected\\x0amenu\n\
00:15\t2412\t-30\t[WPA2-EAP-CCMP][ESS]\tEnterprise\n";
    let found = parse_scan(rows);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].ssid, "My network");
}

#[test]
fn menu_exposes_locked_credentials_and_closes_without_revealing_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(dir.path().to_owned());
    assert!(manager.rows().contains(&"Unlock saved network".into()));
    manager.input(Action::GbaDown(Btn::B));
    assert!(!manager.visible);
}
