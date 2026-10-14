//! Parent controls gate. This does not protect against editing the SD card.
use ring::{
    pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs, io,
    num::NonZeroU32,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"SLCODE01";
const ROUNDS: u32 = 100_000;

pub struct Gate {
    path: PathBuf,
    failures: u8,
    retry: Option<Instant>,
    unlocked: bool,
}
impl Gate {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            failures: 0,
            retry: None,
            unlocked: false,
        }
    }
    pub fn enabled(&self) -> bool {
        self.path.symlink_metadata().is_ok()
    }
    pub fn locked(&self) -> bool {
        self.enabled() && !self.unlocked
    }
    pub fn relock(&mut self) {
        self.unlocked = false;
    }
    pub fn remaining(&self, now: Instant) -> u64 {
        self.retry
            .and_then(|end| end.checked_duration_since(now))
            .map(|d| d.as_secs() + 1)
            .unwrap_or(0)
    }
    pub fn verify(&mut self, code: &[u8], now: Instant) -> io::Result<bool> {
        if self.retry.is_some_and(|end| now < end) {
            return Ok(false);
        }
        self.retry = None;
        validate(code)?;
        let bytes = read_record(&self.path)?;
        let correct = pbkdf2::verify(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(ROUNDS).unwrap(),
            &bytes[8..24],
            code,
            &bytes[24..],
        )
        .is_ok();
        if correct {
            self.failures = 0;
            self.unlocked = true;
        } else {
            self.failures += 1;
            if self.failures >= 5 {
                self.retry = Some(now + Duration::from_secs(30));
                self.failures = 0;
            }
        }
        Ok(correct)
    }
    pub fn set(&mut self, code: &[u8]) -> io::Result<()> {
        if self.locked() {
            return Err(io::Error::other("Unlock before changing the code"));
        }
        validate(code)?;
        let mut salt = [0; 16];
        SystemRandom::new()
            .fill(&mut salt)
            .map_err(|_| io::Error::other("Random source unavailable"))?;
        let mut hash = Zeroizing::new([0; 32]);
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(ROUNDS).unwrap(),
            &salt,
            code,
            hash.as_mut(),
        );
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&salt);
        bytes.extend_from_slice(hash.as_ref());
        let parent = self
            .path
            .parent()
            .ok_or_else(|| io::Error::other("Invalid lock path"))?;
        fs::create_dir_all(parent)?;
        let temp = self.path.with_extension("new");
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)?;
        let result = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp, &self.path)?;
            fs::File::open(parent)?.sync_all()
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result?;
        self.unlocked = true;
        Ok(())
    }
    pub fn remove(&mut self) -> io::Result<()> {
        if self.locked() {
            return Err(io::Error::other("Unlock before disabling the code"));
        }
        if self.enabled() {
            fs::remove_file(&self.path)?;
        }
        self.unlocked = false;
        Ok(())
    }
}
fn validate(code: &[u8]) -> io::Result<()> {
    if code.len() != 4 || code.iter().any(|b| *b > 7) {
        return Err(io::Error::other("Use exactly four buttons"));
    }
    Ok(())
}
fn read_record(path: &Path) -> io::Result<Vec<u8>> {
    use std::io::Read;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() != 56 {
        return Err(io::Error::other("Damaged button lock"));
    }
    let mut bytes = Vec::new();
    file.take(57).read_to_end(&mut bytes)?;
    if bytes.len() != 56 || &bytes[..8] != MAGIC {
        return Err(io::Error::other("Unknown button lock"));
    }
    Ok(bytes)
}
