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
fn menu_exposes_saved_network_and_closes_without_revealing_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(dir.path().to_owned());
    assert!(manager.rows().contains(&"Connect saved network".into()));
    wait(&mut manager);
    manager.input(Action::GbaDown(Btn::B));
    assert!(!manager.visible);
}

fn wait(manager: &mut Manager) {
    for _ in 0..5000 {
        manager.poll();
        if !manager.busy() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("worker did not finish");
}
fn tap(manager: &mut Manager, button: Btn) {
    manager.input(Action::GbaDown(button));
}
fn type_text(manager: &mut Manager, text: &str) {
    let (mut bank, mut cursor) = (0, 0);
    for byte in text.bytes() {
        let pos = (byte - 32) as usize;
        while bank != pos / 40 {
            tap(manager, Btn::R1);
            bank = (bank + 1) % 3;
        }
        while cursor != pos % 40 {
            tap(manager, Btn::Right);
            cursor = (cursor + 1) % 40;
        }
        tap(manager, Btn::A);
    }
    tap(manager, Btn::Start);
    wait(manager);
}

#[test]
fn session_connection_never_writes_a_credential_and_keyboard_stays_masked() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("System")).unwrap();
    std::fs::write(
        dir.path().join("System/network.sh"),
        r#"#!/bin/sh
case "$1" in
scan) printf '00:11\t2412\t-40\t[WPA2-PSK-CCMP][ESS]\tHome\n';;
connect) cat >/dev/null; echo 'IP 192.168.1.2/24';;
esac
"#,
    )
    .unwrap();
    let mut manager = Manager::new(dir.path().to_owned());
    wait(&mut manager);
    tap(&mut manager, Btn::A);
    wait(&mut manager);
    tap(&mut manager, Btn::Down);
    tap(&mut manager, Btn::A);
    // The first key is space and must not expose typed text.
    tap(&mut manager, Btn::A);
    assert!(manager.lines().iter().any(|l| l.contains('•')));
    tap(&mut manager, Btn::X);
    type_text(&mut manager, "secret-password");
    assert!(manager.rows().contains(&"Connect for this session".into()));
    tap(&mut manager, Btn::A);
    assert!(!dir.path().join("Config/wifi.vault").exists());
    assert!(!manager
        .lines()
        .iter()
        .any(|l| l.contains("secret-password")));
    assert!(manager.rows().contains(&"Connect saved network".into()));
}

#[test]
fn wifi_has_no_redundant_heading_and_scrolls_networks_with_an_indicator() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("System")).unwrap();
    std::fs::write(
        dir.path().join("System/network.sh"),
        "case \"$1\" in scan) i=0; while [ $i -lt 10 ]; do printf '00:11\\t2412\\t-40\\t[WPA2-PSK-CCMP][ESS]\\tHome%s\\n' \"$i\"; i=$((i+1)); done;; esac\n",
    )
    .unwrap();
    let mut manager = Manager::new(dir.path().to_owned());
    wait(&mut manager);
    assert!(manager.lines()[0].is_empty());
    assert!(manager.lines()[1].is_empty());
    assert_eq!(manager.lines()[2], "COMPACT");
    tap(&mut manager, Btn::A);
    wait(&mut manager);
    assert!(manager.status.is_empty());
    let first = manager.screen_face();
    for _ in 0..10 {
        tap(&mut manager, Btn::Down);
    }
    let lines = manager.lines();
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.starts_with("ROW ") || l.starts_with("SELECTED "))
            .count(),
        6
    );
    assert!(lines.iter().any(|l| l == "SELECTED DEVICE Home9"));
    let last = manager.screen_face();
    let pixel = |face: &slot_ui::UndoFace| {
        let at = (90 * face.w as usize + face.w as usize - 11) * 4;
        face.rgba[at..at + 4].to_vec()
    };
    assert_ne!(pixel(&first), pixel(&last));
}
