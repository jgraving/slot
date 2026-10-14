//! Strict verification and isolated staging. This module never installs or executes updates.
use ring::digest::{digest, SHA256};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
};

const MAX_ARCHIVE: usize = 64 * 1024 * 1024;
const MAX_FILE: u64 = 32 * 1024 * 1024;
const MAX_MANIFEST: u64 = 64 * 1024;
const ALLOWED: &[&str] = &[
    "System/slot",
    "System/build.json",
    "System/launch_frontend.sh",
    "System/bluetooth.sh",
    "System/network.sh",
    "System/slotlink.sh",
    "System/led.sh",
    "System/coresave.sh",
    "System/bootlogo.bmp",
    "System/bezels/640x480.png",
];
const EXTRAS: &[&str] = &["manifest.json", "LICENSE", "SOURCE.txt", "INSTALL.txt"];

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub target: String,
    pub repository: String,
    pub commit: String,
    pub channel: String,
    pub version: String,
    pub files: Vec<Entry>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub mode: u32,
}
/// Obtained from trusted HTTPS release discovery, not from the archive itself.
/// Preview packages deliberately cannot be selected for automatic updates.
pub struct Expected<'a> {
    pub sha256: &'a str,
    pub size: usize,
    pub commit: &'a str,
    pub channel: &'a str,
    pub version: &'a str,
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid update package")
}
fn hash(data: &[u8]) -> String {
    digest(&SHA256, data)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// All bytes are checked before any filesystem writes. The private fields prevent
/// callers from constructing a verified update without passing verification.
pub struct Verified {
    manifest: Manifest,
    files: Vec<(String, Vec<u8>, u32)>,
}
impl Verified {
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn verify(bytes: &[u8], expected: &Expected<'_>) -> io::Result<Self> {
        if bytes.len() > MAX_ARCHIVE
            || bytes.len() != expected.size
            || !hex(expected.sha256, 64)
            || hash(bytes) != expected.sha256
            || !hex(expected.commit, 40)
            || !matches!(expected.channel, "stable" | "development")
            || expected.version.is_empty()
            || expected.version.len() > 80
            || !expected
                .version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(invalid());
        }
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| invalid())?;
        if zip.len() > ALLOWED.len() + EXTRAS.len() {
            return Err(invalid());
        }
        let mut names = BTreeSet::new();
        let mut total = 0u64;
        for i in 0..zip.len() {
            let f = zip.by_index(i).map_err(|_| invalid())?;
            if !names.insert(f.name().to_string())
                || f.is_dir()
                || (!ALLOWED.contains(&f.name()) && !EXTRAS.contains(&f.name()))
                || f.size() > MAX_FILE
                || f.unix_mode()
                    .is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000)
            {
                return Err(invalid());
            }
            total = total.checked_add(f.size()).ok_or_else(invalid)?;
            if total > MAX_ARCHIVE as u64 {
                return Err(invalid());
            }
        }
        let read = |zip: &mut zip::ZipArchive<Cursor<&[u8]>>,
                    name: &str,
                    limit: u64|
         -> io::Result<Vec<u8>> {
            let f = zip.by_name(name).map_err(|_| invalid())?;
            if f.size() > limit {
                return Err(invalid());
            }
            let mut data = Vec::new();
            f.take(limit + 1).read_to_end(&mut data)?;
            if data.len() as u64 > limit {
                return Err(invalid());
            }
            Ok(data)
        };
        let manifest: Manifest =
            serde_json::from_slice(&read(&mut zip, "manifest.json", MAX_MANIFEST)?)
                .map_err(|_| invalid())?;
        if manifest.schema != 1
            || manifest.target != "h700-aarch64"
            || manifest.repository != "jgraving/slot"
            || manifest.commit != expected.commit
            || manifest.channel != expected.channel
            || manifest.version != expected.version
        {
            return Err(invalid());
        }
        let mut listed = BTreeSet::new();
        let mut files = Vec::new();
        for entry in &manifest.files {
            if !ALLOWED.contains(&entry.path.as_str())
                || !listed.insert(entry.path.clone())
                || entry.size > MAX_FILE
                || !hex(&entry.sha256, 64)
                || !matches!(entry.mode, 0o644 | 0o755)
                || (entry.path == "System/slot" && entry.mode != 0o755)
            {
                return Err(invalid());
            }
            let data = read(&mut zip, &entry.path, entry.size)?;
            if data.len() as u64 != entry.size || hash(&data) != entry.sha256 {
                return Err(invalid());
            }
            if entry.path == "System/slot"
                && (data.get(..6) != Some(b"\x7fELF\x02\x01")
                    || data.get(18..20) != Some(b"\xb7\x00"))
            {
                return Err(invalid());
            }
            files.push((entry.path.clone(), data, entry.mode));
        }
        if !listed.contains("System/slot")
            || !listed.contains("System/build.json")
            || names
                .iter()
                .any(|name| name.starts_with("System/") && !listed.contains(name))
        {
            return Err(invalid());
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Build {
            schema: u32,
            commit: String,
            channel: String,
            version: String,
        }
        let build: Build =
            serde_json::from_slice(&read(&mut zip, "System/build.json", MAX_MANIFEST)?)
                .map_err(|_| invalid())?;
        if build.schema != 1
            || build.commit != manifest.commit
            || build.channel != manifest.channel
            || build.version != manifest.version
        {
            return Err(invalid());
        }
        Ok(Self { manifest, files })
    }

    /// Caller supplies an existing trusted staging parent on persistent storage.
    /// A random temporary directory is removed on failure; completion is marked
    /// only after every payload and manifest has been flushed. No live paths are used.
    pub fn stage(&self, parent: &Path) -> io::Result<PathBuf> {
        for ancestor in parent.ancestors() {
            if ancestor.as_os_str().is_empty() {
                continue;
            }
            let meta = fs::symlink_metadata(ancestor)?;
            if !meta.is_dir() || meta.file_type().is_symlink() {
                return Err(invalid());
            }
        }
        let temp = tempfile::Builder::new()
            .prefix("slot-update-")
            .tempdir_in(parent)?;
        for (name, data, mode) in &self.files {
            let path = temp.path().join(name);
            fs::create_dir_all(path.parent().ok_or_else(invalid)?)?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            file.write_all(data)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(*mode))?;
            }
            file.sync_all()?;
        }
        let mut manifest = fs::File::create(temp.path().join("manifest.json"))?;
        serde_json::to_writer(&mut manifest, &self.manifest)?;
        manifest.sync_all()?;
        // Nested directory entries must reach storage before the ready marker.
        if temp.path().join("System/bezels").exists() {
            fs::File::open(temp.path().join("System/bezels"))?.sync_all()?;
        }
        fs::File::open(temp.path().join("System"))?.sync_all()?;
        fs::File::open(temp.path())?.sync_all()?;
        let ready = fs::File::create(temp.path().join("READY"))?;
        ready.sync_all()?;
        fs::File::open(temp.path())?.sync_all()?;
        fs::File::open(parent)?.sync_all()?;
        Ok(temp.keep())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;
    const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    fn fixture(edit: impl FnOnce(&mut Manifest, &mut Vec<(String, Vec<u8>)>)) -> Vec<u8> {
        let mut elf = vec![0; 64];
        elf[..6].copy_from_slice(b"\x7fELF\x02\x01");
        elf[18..20].copy_from_slice(b"\xb7\x00");
        let build = serde_json::json!({"schema": 1, "commit": COMMIT, "channel": "development", "version": "dev-1-1"});
        let mut files: Vec<(String, Vec<u8>)> = vec![
            ("System/slot".into(), elf),
            (
                "System/build.json".into(),
                serde_json::to_vec(&build).unwrap(),
            ),
        ];
        let mut manifest = Manifest {
            schema: 1,
            target: "h700-aarch64".into(),
            repository: "jgraving/slot".into(),
            commit: COMMIT.into(),
            channel: "development".into(),
            version: "dev-1-1".into(),
            files: files
                .iter()
                .map(|(p, d)| Entry {
                    path: p.clone(),
                    size: d.len() as u64,
                    sha256: hash(d),
                    mode: if p == "System/slot" { 0o755 } else { 0o644 },
                })
                .collect(),
        };
        edit(&mut manifest, &mut files);
        files.push((
            "manifest.json".into(),
            serde_json::to_vec(&manifest).unwrap(),
        ));
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (path, data) in files {
            zip.start_file(
                path,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
            zip.write_all(&data).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }
    fn check(bytes: &[u8]) -> io::Result<Verified> {
        Verified::verify(
            bytes,
            &Expected {
                sha256: &hash(bytes),
                size: bytes.len(),
                commit: COMMIT,
                channel: "development",
                version: "dev-1-1",
            },
        )
    }
    #[test]
    fn valid_package_stages_without_touching_live_files() {
        let bytes = fixture(|_, _| {});
        let verified = check(&bytes).unwrap();
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("System")).unwrap();
        fs::write(root.path().join("System/slot"), b"old binary").unwrap();
        let staged = verified.stage(root.path()).unwrap();
        assert!(staged.join("READY").is_file());
        assert!(staged.join("manifest.json").is_file());
        assert_eq!(
            fs::read(root.path().join("System/slot")).unwrap(),
            b"old binary"
        );
        assert_eq!(fs::read(staged.join("System/slot")).unwrap().len(), 64);
    }
    #[test]
    fn refuses_identity_mismatch_and_payload_tampering() {
        for bytes in [
            fixture(|m, _| m.commit = "b".repeat(40)),
            fixture(|m, _| m.target = "other-device".into()),
            fixture(|_, f| f[0].1[30] = 1),
            fixture(|m, _| m.schema = 2),
            fixture(|m, _| m.files[0].mode = 0o4755),
        ] {
            assert!(check(&bytes).is_err());
        }
        let bytes = fixture(|_, _| {});
        assert!(Verified::verify(
            &bytes,
            &Expected {
                sha256: &"0".repeat(64),
                size: bytes.len(),
                commit: COMMIT,
                channel: "development",
                version: "dev-1-1"
            }
        )
        .is_err());
    }
    #[test]
    fn refuses_path_escape_cores_unlisted_and_oversized_entries() {
        for path in [
            "../escape",
            "/etc/shadow",
            "System/mgba_libretro.so",
            "System/network/../../escape",
            "System/password",
        ] {
            assert!(check(&fixture(|_, files| files.push((path.into(), vec![1])))).is_err());
        }
        assert!(check(&fixture(|m, _| {
            m.files.remove(0);
        }))
        .is_err());
        assert!(check(&fixture(|m, _| m.files[0].size = MAX_FILE + 1)).is_err());
        assert!(check(&fixture(|m, _| m.files.push(m.files[0].clone()))).is_err());
    }
    #[test]
    fn refuses_truncation_and_wrong_build_identity() {
        let mut bytes = fixture(|_, _| {});
        bytes.truncate(bytes.len() / 2);
        assert!(check(&bytes).is_err());
        let bytes = fixture(|m, f| {
            f[1].1 =
                br#"{"schema":1,"commit":"wrong","version":"dev-1-1","channel":"development"}"#
                    .to_vec();
            m.files[1].size = f[1].1.len() as u64;
            m.files[1].sha256 = hash(&f[1].1);
        });
        assert!(check(&bytes).is_err());
    }
    #[test]
    fn accepts_real_python_packager_output() {
        let temp = tempfile::tempdir().unwrap();
        let mut elf = vec![0; 64];
        elf[..6].copy_from_slice(b"\x7fELF\x02\x01");
        elf[18..20].copy_from_slice(b"\xb7\x00");
        let binary = temp.path().join("slot");
        fs::write(&binary, elf).unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let status = std::process::Command::new("python3")
            .current_dir(&root)
            .arg("tools/package-system.py")
            .arg("--binary")
            .arg(binary)
            .arg("--output")
            .arg(temp.path())
            .args([
                "--commit",
                COMMIT,
                "--channel",
                "development",
                "--version",
                "dev-1-1",
            ])
            .status()
            .unwrap();
        assert!(status.success());
        let bytes = fs::read(temp.path().join("slot-System.zip")).unwrap();
        let verified = check(&bytes).unwrap();
        assert!(verified
            .manifest()
            .files
            .iter()
            .any(|f| f.path == "System/network.sh"));
    }
    #[test]
    fn refuses_zip_symlink() {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.add_symlink("System/slot", "/etc/shadow", SimpleFileOptions::default())
            .unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        assert!(check(&bytes).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn refuses_symlink_staging_parent() {
        let root = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(root.path(), root.path().join("link")).unwrap();
        assert!(check(&fixture(|_, _| {}))
            .unwrap()
            .stage(&root.path().join("link"))
            .is_err());
    }
}
