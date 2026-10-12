//! Password-protected Wi-Fi storage. No unlock key is persisted or derived from a device ID.
use ring::{
    aead, pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use std::{fs, io, num::NonZeroU32, path::Path};
use zeroize::{Zeroize, Zeroizing};

const MAGIC: &[u8; 8] = b"SLWIFI01";
const ROUNDS: u32 = 600_000;
const LIMIT: u64 = 4096;

pub struct Profile {
    pub ssid: String,
    pub psk: Zeroizing<[u8; 32]>,
}

impl Profile {
    pub fn from_password(ssid: String, password: Zeroizing<String>) -> io::Result<Self> {
        if ssid.is_empty()
            || ssid.len() > 32
            || !(8..=63).contains(&password.len())
            || !password.bytes().all(|b| (32..=126).contains(&b))
        {
            return Err(error(
                "Use a 1-32 byte network name and an 8-63 character WPA2 password",
            ));
        }
        let mut psk = Zeroizing::new([0; 32]);
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA1,
            NonZeroU32::new(4096).unwrap(),
            ssid.as_bytes(),
            password.as_bytes(),
            psk.as_mut(),
        );
        Ok(Self { ssid, psk })
    }

    /// Supplicant receives the key only through stdin; the caller must not log this buffer.
    pub fn supplicant(&self) -> Zeroizing<String> {
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        Zeroizing::new(format!("ctrl_interface=/run/slot-wifi/control\nupdate_config=0\nnetwork={{\nssid={}\npsk={}\nkey_mgmt=WPA-PSK\nproto=RSN\npairwise=CCMP\ngroup=CCMP\n}}\n", hex(self.ssid.as_bytes()), hex(self.psk.as_ref())))
    }
}

pub struct Vault {
    key: Zeroizing<[u8; 32]>,
    salt: [u8; 16],
}

fn error(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl Vault {
    pub fn create(passphrase: Zeroizing<String>) -> io::Result<Self> {
        if passphrase.len() < 12 || passphrase.len() > 128 {
            return Err(error("Use an unlock passphrase of 12-128 characters"));
        }
        let mut salt = [0; 16];
        SystemRandom::new()
            .fill(&mut salt)
            .map_err(|_| error("Random source unavailable"))?;
        Ok(Self::derive(passphrase, salt))
    }

    fn derive(passphrase: Zeroizing<String>, salt: [u8; 16]) -> Self {
        let mut key = Zeroizing::new([0; 32]);
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(ROUNDS).unwrap(),
            &salt,
            passphrase.as_bytes(),
            key.as_mut(),
        );
        Self { key, salt }
    }

    fn cipher(&self) -> io::Result<aead::LessSafeKey> {
        aead::UnboundKey::new(&aead::AES_256_GCM, self.key.as_ref())
            .map(aead::LessSafeKey::new)
            .map_err(|_| error("Encryption unavailable"))
    }

    pub fn save(&self, path: &Path, profile: &Profile) -> io::Result<()> {
        if profile.ssid.is_empty() || profile.ssid.len() > 32 {
            return Err(error("Invalid network name"));
        }
        let mut nonce = [0; 12];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| error("Random source unavailable"))?;
        let mut header = MAGIC.to_vec();
        header.extend_from_slice(&self.salt);
        header.extend_from_slice(&nonce);
        let mut body = Zeroizing::new(Vec::new());
        body.push(profile.ssid.len() as u8);
        body.extend_from_slice(profile.ssid.as_bytes());
        body.extend_from_slice(profile.psk.as_ref());
        self.cipher()?
            .seal_in_place_append_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(&header),
                &mut *body,
            )
            .map_err(|_| error("Encryption failed"))?;
        header.extend_from_slice(&body);
        write_atomic(path, &header)
    }

    pub fn unlock(path: &Path, passphrase: Zeroizing<String>) -> io::Result<(Self, Profile)> {
        if fs::metadata(path)?.len() > LIMIT || passphrase.len() > 128 {
            return Err(error("Invalid credential vault"));
        }
        let data = fs::read(path)?;
        if data.len() < 36 + 16 + 34 || &data[..8] != MAGIC {
            return Err(error("Invalid credential vault"));
        }
        let salt = data[8..24].try_into().unwrap();
        let nonce = data[24..36].try_into().unwrap();
        let vault = Self::derive(passphrase, salt);
        let mut body = Zeroizing::new(data[36..].to_vec());
        let plain = vault
            .cipher()?
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(&data[..36]),
                &mut body,
            )
            .map_err(|_| error("Wrong unlock passphrase or damaged vault"))?;
        let len = plain[0] as usize;
        if len == 0 || len > 32 || plain.len() != len + 33 {
            return Err(error("Invalid credential vault"));
        }
        let ssid = std::str::from_utf8(&plain[1..1 + len])
            .map_err(|_| error("Invalid network name"))?
            .to_owned();
        let mut psk = Zeroizing::new([0; 32]);
        psk.copy_from_slice(&plain[1 + len..]);
        body.zeroize();
        Ok((vault, Profile { ssid, psk }))
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let parent = path.parent().ok_or_else(|| error("Invalid vault path"))?;
    fs::create_dir_all(parent)?;
    let temp = path.with_extension("new");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        fs::File::open(parent)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
