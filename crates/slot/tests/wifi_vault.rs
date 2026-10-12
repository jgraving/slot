use slot::wifi_vault::{Profile, Vault};
use zeroize::Zeroizing;

#[test]
fn vault_encrypts_and_authenticates_every_part_without_persisting_passwords() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wifi.vault");
    let password = "wifi password sentinel";
    let unlock = "separate unlock passphrase";
    let profile = Profile::from_password("My network".into(), Zeroizing::new(password.into())).unwrap();
    let vault = Vault::create(Zeroizing::new(unlock.into())).unwrap();
    vault.save(&path, &profile).unwrap();
    let original = std::fs::read(&path).unwrap();
    for secret in [password, unlock, "My network"] {
        assert!(!original.windows(secret.len()).any(|w| w == secret.as_bytes()));
    }
    let (_, loaded) = Vault::unlock(&path, Zeroizing::new(unlock.into())).unwrap();
    assert_eq!(loaded.ssid, profile.ssid);
    assert_eq!(loaded.psk.as_ref(), profile.psk.as_ref());
    assert!(Vault::unlock(&path, Zeroizing::new("wrong passphrase".into())).is_err());
    for at in [8, 24, 36, original.len()-1] {
        let mut damaged = original.clone();
        damaged[at] ^= 1;
        std::fs::write(&path, damaged).unwrap();
        assert!(Vault::unlock(&path, Zeroizing::new(unlock.into())).is_err());
    }
    vault.save(&path, &profile).unwrap();
    assert_ne!(original, std::fs::read(&path).unwrap(), "fresh nonce on each write");
    assert!(!profile.supplicant().contains(password));
}

#[test]
fn weak_unlock_and_invalid_wifi_passwords_are_rejected() {
    assert!(Vault::create(Zeroizing::new("1234".into())).is_err());
    assert!(Profile::from_password("Network".into(), Zeroizing::new("short".into())).is_err());
    assert!(Profile::from_password("".into(), Zeroizing::new("password".into())).is_err());
}
