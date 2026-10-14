use slot::wifi_vault::{Profile, Vault};
use zeroize::Zeroizing;
#[test]
fn device_key_encrypts_profile_and_detects_modification_without_saving_wifi_password() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Config/wifi.vault");
    let key = dir.path().join("private/vault.key");
    let vault = Vault::device(&key).unwrap();
    let profile = Profile::from_password(
        "My network".into(),
        Zeroizing::new("wifi sentinel password".into()),
    )
    .unwrap();
    vault.save(&path, &profile).unwrap();
    let original = std::fs::read(&path).unwrap();
    for secret in ["wifi sentinel password", "My network"] {
        assert!(!original
            .windows(secret.len())
            .any(|w| w == secret.as_bytes()));
    }
    let reopened = Vault::device(&key).unwrap();
    let loaded = reopened.read(&path).unwrap();
    assert_eq!(loaded.ssid, profile.ssid);
    assert_eq!(loaded.psk.as_ref(), profile.psk.as_ref());
    let other = Vault::device(&dir.path().join("other/key")).unwrap();
    assert!(other.read(&path).is_err());
    for at in [8, 20, original.len() - 1] {
        let mut damaged = original.clone();
        damaged[at] ^= 1;
        std::fs::write(&path, damaged).unwrap();
        assert!(reopened.read(&path).is_err());
    }
    vault.save(&path, &profile).unwrap();
    assert_ne!(original, std::fs::read(&path).unwrap());
    assert!(!profile.supplicant().contains("wifi sentinel password"));
}
#[test]
fn device_key_rejects_world_readable_files_and_symlinks() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/key");
    Vault::device(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(Vault::device(&path).is_err());
    let link = dir.path().join("private/link");
    symlink(&path, &link).unwrap();
    assert!(Vault::device(&link).is_err());
}
#[test]
fn invalid_wifi_credentials_are_rejected() {
    assert!(Profile::from_password("Network".into(), Zeroizing::new("short".into())).is_err());
    assert!(Profile::from_password("".into(), Zeroizing::new("password".into())).is_err());
}

#[test]
fn malformed_and_truncated_vaults_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let vault = Vault::device(&dir.path().join("private/key")).unwrap();
    let path = dir.path().join("profile");
    for data in [vec![], vec![0; 70], vec![0; 4097]] {
        std::fs::write(&path, data).unwrap();
        assert!(vault.read(&path).is_err());
    }
}

#[test]
fn psk_matches_wpa_reference_vector_and_ssid_cannot_inject_configuration() {
    let profile = Profile::from_password("IEEE".into(), Zeroizing::new("password".into())).unwrap();
    let hex: String = profile.psk.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        "f42c6fc52df0ebef9ebb4b90b38a5f902e83fe1b135a70e23aed762e9710a12e"
    );
    let profile = Profile::from_password(
        "quoted\"\nnetwork".into(),
        Zeroizing::new("password".into()),
    )
    .unwrap();
    assert!(profile
        .supplicant()
        .contains("ssid=71756f746564220a6e6574776f726b\n"));
}
