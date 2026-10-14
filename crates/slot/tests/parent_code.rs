use slot::parent_code::Gate;
use std::time::{Duration, Instant};
#[test]
fn default_off_and_saved_code_roundtrips_without_plaintext() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("button-lock");
    let mut gate = Gate::new(path.clone());
    assert!(!gate.enabled());
    assert!(!gate.locked());
    gate.set(&[0, 1, 2, 3]).unwrap();
    let data = std::fs::read(&path).unwrap();
    assert!(!data.windows(4).any(|w| w == [0, 1, 2, 3]));
    let mut reopened = Gate::new(path);
    assert!(reopened.locked());
    assert!(reopened.remove().is_err());
    assert!(reopened.set(&[1, 1, 1, 1]).is_err());
    assert!(reopened.verify(&[0, 1, 2, 3], Instant::now()).unwrap());
    reopened.relock();
    assert!(reopened.locked());
    assert!(reopened.verify(&[0, 1, 2, 3], Instant::now()).unwrap());
    reopened.remove().unwrap();
    assert!(!reopened.enabled());
}
#[test]
fn wrong_attempts_are_throttled_and_correct_code_works_after_delay() {
    let dir = tempfile::tempdir().unwrap();
    let mut gate = Gate::new(dir.path().join("lock"));
    gate.set(&[4, 5, 6, 7]).unwrap();
    gate.relock();
    let now = Instant::now();
    for _ in 0..5 {
        assert!(!gate.verify(&[0; 4], now).unwrap());
    }
    assert!(!gate.verify(&[4, 5, 6, 7], now).unwrap());
    assert!(gate.remaining(now) >= 30);
    assert!(gate
        .verify(&[4, 5, 6, 7], now + Duration::from_secs(31))
        .unwrap());
}
#[test]
fn damaged_lock_fails_closed_and_invalid_codes_cannot_be_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lock");
    let mut gate = Gate::new(path.clone());
    assert!(gate.set(&[0; 3]).is_err());
    assert!(gate.set(&[9; 4]).is_err());
    std::fs::write(path, b"broken").unwrap();
    assert!(gate.locked());
    assert!(gate.verify(&[0; 4], Instant::now()).is_err());
}
