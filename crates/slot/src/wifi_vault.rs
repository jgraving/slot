//! Encrypted Wi-Fi profiles, using a random key in BaseOS's private system storage.
use ring::{
    aead, pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::{
    fs,
    io::{self, Read, Write},
    num::NonZeroU32,
    path::Path,
};
use zeroize::{Zeroize, Zeroizing};
const MAGIC: &[u8; 8] = b"SLWIFI02";

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
            return Err(error("Invalid WPA2 credentials"));
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
    pub fn supplicant(&self) -> Zeroizing<String> {
        let hex =
            |b: &[u8]| Zeroizing::new(b.iter().map(|b| format!("{b:02x}")).collect::<String>());
        Zeroizing::new(format!("ctrl_interface=/run/slot-wifi/control\nupdate_config=0\nnetwork={{\nssid={}\npsk={}\nkey_mgmt=WPA-PSK\nproto=RSN\npairwise=CCMP\ngroup=CCMP\n}}\n", hex(self.ssid.as_bytes()).as_str(), hex(self.psk.as_ref()).as_str()))
    }
}

pub struct Vault {
    key: Zeroizing<[u8; 32]>,
}
fn error(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
impl Vault {
    /// Production path is /data/slot-network/vault.key, never the games-card Config folder.
    pub fn device(path: &Path) -> io::Result<Self> {
        let parent = path
            .parent()
            .ok_or_else(|| error("Invalid device key location"))?;
        for ancestor in path.ancestors().skip(1) {
            if ancestor.exists() && fs::symlink_metadata(ancestor)?.file_type().is_symlink() {
                return Err(error("Unsafe key directory"));
            }
        }
        fs::create_dir_all(parent)?;
        if fs::symlink_metadata(parent)?.file_type().is_symlink() {
            return Err(error("Unsafe key directory"));
        }
        if fs::metadata(parent)?.uid() != unsafe { libc::geteuid() } {
            return Err(error("Unsafe key directory owner"));
        }
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        let mut key = Zeroizing::new([0; 32]);
        SystemRandom::new()
            .fill(key.as_mut())
            .map_err(|_| error("Random source unavailable"))?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
        {
            Ok(mut file) => {
                let result = (|| {
                    file.write_all(key.as_ref())?;
                    file.sync_all()?;
                    fs::File::open(parent)?.sync_all()
                })();
                if result.is_err() {
                    let _ = fs::remove_file(path);
                }
                result?;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let mut file = fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(path)?;
                let meta = file.metadata()?;
                if !meta.is_file()
                    || meta.len() != 32
                    || meta.permissions().mode() & 0o077 != 0
                    || meta.uid() != unsafe { libc::geteuid() }
                {
                    return Err(error("Unsafe device key"));
                }
                file.read_exact(key.as_mut())?;
            }
            Err(e) => return Err(e),
        }
        Ok(Self { key })
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
    pub fn read(&self, path: &Path) -> io::Result<Profile> {
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)?;
        if !file.metadata()?.is_file() || file.metadata()?.len() > 4096 {
            return Err(error("Invalid credential vault"));
        }
        let mut data = Vec::new();
        file.take(4097).read_to_end(&mut data)?;
        if data.len() < 20 + 16 + 34 || &data[..8] != MAGIC {
            return Err(error("Invalid credential vault"));
        }
        let nonce = data[8..20].try_into().unwrap();
        let mut body = Zeroizing::new(data[20..].to_vec());
        let plain = self
            .cipher()?
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(&data[..20]),
                &mut body,
            )
            .map_err(|_| error("Wrong device key or damaged vault"))?;
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
        Ok(Profile { ssid, psk })
    }
}
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| error("Invalid vault path"))?;
    fs::create_dir_all(parent)?;
    let mut random = [0; 8];
    SystemRandom::new()
        .fill(&mut random)
        .map_err(|_| error("Random source unavailable"))?;
    let name = random
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let temp = parent.join(format!("wifi-{name}.new"));
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
