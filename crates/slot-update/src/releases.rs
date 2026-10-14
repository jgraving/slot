//! Public GitHub release discovery and bounded, CA-verified HTTPS downloads.
use crate::{hash, hex, invalid, Expected, Manifest, Verified, MAX_ARCHIVE, MAX_MANIFEST};
use serde::Deserialize;
use std::{
    io::{self, Read},
    process::{Command, Stdio},
};

const API: &str = "https://api.github.com/repos/jgraving/slot/releases";
const API_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Development,
}
impl Channel {
    fn name(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Development => "development",
        }
    }
}
#[derive(Deserialize)]
struct Release {
    id: u64,
    tag_name: String,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    assets: Vec<Asset>,
}
#[derive(Clone, Deserialize)]
struct Asset {
    name: String,
    size: usize,
    digest: Option<String>,
    browser_download_url: String,
    state: String,
}
/// Selection pins both assets to one published release; callers cannot substitute URLs.
pub struct Selection {
    pub id: u64,
    pub version: String,
    channel: Channel,
    manifest: Asset,
    archive: Asset,
}

pub trait Fetch {
    fn get(&self, url: &str, limit: usize) -> io::Result<Vec<u8>>;
}
/// Uses BaseOS's bundled curl and CA store. No shell, credentials, user curlrc,
/// proxy environment or insecure TLS fallback. An incorrect clock is an error.
pub struct BaseOsHttps;
impl Fetch for BaseOsHttps {
    fn get(&self, url: &str, limit: usize) -> io::Result<Vec<u8>> {
        if !url.starts_with("https://") {
            return Err(invalid());
        }
        let mut child = Command::new("/usr/bin/curl")
            .env_clear()
            .args([
                "-q",
                "--fail",
                "--silent",
                "--location",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--max-redirs",
                "5",
                "--connect-timeout",
                "10",
                "--max-time",
                "120",
                "--cacert",
                "/etc/ssl/certs/ca-certificates.crt",
                "--user-agent",
                "slot-updater/1",
                "--header",
                "Accept: application/vnd.github+json",
                "--max-filesize",
            ])
            .arg(limit.to_string())
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut data = Vec::new();
        let result = child
            .stdout
            .take()
            .ok_or_else(invalid)?
            .take(limit as u64 + 1)
            .read_to_end(&mut data);
        if result.is_err() || data.len() > limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(invalid());
        }
        if !child.wait()?.success() {
            return Err(io::Error::other(
                "HTTPS update download failed; check Wi-Fi and device clock",
            ));
        }
        Ok(data)
    }
}

fn valid_tag(tag: &str, channel: Channel) -> bool {
    let (prefix, separator, count) = match channel {
        Channel::Stable => ("v", '.', 3),
        Channel::Development => ("dev-", '-', 2),
    };
    tag.strip_prefix(prefix).is_some_and(|tail| {
        let parts: Vec<_> = tail.split(separator).collect();
        tag.len() <= 80
            && parts.len() == count
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    })
}
fn asset(release: &Release, name: &str, limit: usize) -> Option<Asset> {
    let matches: Vec<_> = release.assets.iter().filter(|a| a.name == name).collect();
    if matches.len() != 1 {
        return None;
    }
    let a = matches[0];
    if a.size == 0
        || a.size > limit
        || a.state != "uploaded"
        || a.browser_download_url
            != format!(
                "https://github.com/jgraving/slot/releases/download/{}/{name}",
                release.tag_name
            )
        || !a
            .digest
            .as_deref()
            .and_then(|d| d.strip_prefix("sha256:"))
            .is_some_and(|d| hex(d, 64))
    {
        return None;
    }
    Some(a.clone())
}
fn select(release: Release, channel: Channel) -> Option<Selection> {
    if release.draft
        || release.published_at.is_none()
        || release.id == 0
        || release.prerelease != (channel == Channel::Development)
        || !valid_tag(&release.tag_name, channel)
    {
        return None;
    }
    let manifest = asset(&release, "manifest.json", MAX_MANIFEST as usize)?;
    let archive = asset(&release, "slot-System.zip", MAX_ARCHIVE)?;
    Some(Selection {
        id: release.id,
        version: release.tag_name,
        channel,
        manifest,
        archive,
    })
}
/// Development examines the latest 100 published entries, newest publication first.
/// Missing/incomplete releases produce None; malformed responses and HTTP failures are errors.
pub fn discover(fetch: &impl Fetch, channel: Channel) -> io::Result<Option<Selection>> {
    let url = match channel {
        Channel::Stable => format!("{API}/latest"),
        Channel::Development => format!("{API}?per_page=100"),
    };
    let data = fetch.get(&url, API_LIMIT)?;
    if data.len() > API_LIMIT {
        return Err(invalid());
    }
    if channel == Channel::Stable {
        return Ok(select(
            serde_json::from_slice(&data).map_err(|_| invalid())?,
            channel,
        ));
    }
    let mut releases: Vec<Release> = serde_json::from_slice(&data).map_err(|_| invalid())?;
    releases.sort_by(|a, b| {
        b.published_at
            .cmp(&a.published_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(releases.into_iter().find_map(|r| select(r, channel)))
}
fn download(fetch: &impl Fetch, asset: &Asset) -> io::Result<Vec<u8>> {
    let data = fetch.get(&asset.browser_download_url, asset.size)?;
    if data.len() != asset.size
        || asset.digest.as_deref() != Some(format!("sha256:{}", hash(&data)).as_str())
    {
        return Err(invalid());
    }
    Ok(data)
}
impl Selection {
    /// Fetches and verifies without staging, installing, restarting or changing channels.
    pub fn download(&self, fetch: &impl Fetch) -> io::Result<Verified> {
        let manifest_bytes = download(fetch, &self.manifest)?;
        let manifest: Manifest = serde_json::from_slice(&manifest_bytes).map_err(|_| invalid())?;
        if manifest.version != self.version
            || manifest.channel != self.channel.name()
            || manifest.repository != "jgraving/slot"
            || manifest.target != "h700-aarch64"
            || manifest.schema != 1
            || !hex(&manifest.commit, 40)
        {
            return Err(invalid());
        }
        let bytes = download(fetch, &self.archive)?;
        let sha256 = self
            .archive
            .digest
            .as_deref()
            .and_then(|d| d.strip_prefix("sha256:"))
            .ok_or_else(invalid)?;
        let verified = Verified::verify(
            &bytes,
            &Expected {
                sha256,
                size: self.archive.size,
                commit: &manifest.commit,
                channel: self.channel.name(),
                version: &self.version,
            },
        )?;
        if serde_json::to_vec(verified.manifest())? != serde_json::to_vec(&manifest)? {
            return Err(invalid());
        }
        Ok(verified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    struct Fake(Vec<u8>);
    impl Fetch for Fake {
        fn get(&self, _: &str, _: usize) -> io::Result<Vec<u8>> {
            Ok(self.0.clone())
        }
    }
    fn release(tag: &str, prerelease: bool, date: &str) -> Value {
        json!({"id": 1, "tag_name": tag, "draft": false, "prerelease": prerelease,
        "published_at": date, "assets": [
            {"name": "manifest.json", "size": 100, "state": "uploaded", "digest": format!("sha256:{}", "a".repeat(64)), "browser_download_url": format!("https://github.com/jgraving/slot/releases/download/{tag}/manifest.json")},
            {"name": "slot-System.zip", "size": 500, "state": "uploaded", "digest": format!("sha256:{}", "b".repeat(64)), "browser_download_url": format!("https://github.com/jgraving/slot/releases/download/{tag}/slot-System.zip")}
        ]})
    }
    fn fake(value: Value) -> Fake {
        Fake(serde_json::to_vec(&value).unwrap())
    }
    #[test]
    fn stable_never_selects_prereleases_or_preview() {
        assert!(discover(
            &fake(release("v1.2.3", false, "2026-10-14T10:00:00Z")),
            Channel::Stable
        )
        .unwrap()
        .is_some());
        for (tag, pre) in [
            ("dev-1-1", true),
            ("preview-1-1", true),
            ("v1.2.3", true),
            ("v1.2.3-beta", false),
        ] {
            assert!(discover(
                &fake(release(tag, pre, "2026-10-14T10:00:00Z")),
                Channel::Stable
            )
            .unwrap()
            .is_none());
        }
    }
    #[test]
    fn development_selects_newest_complete_published_build() {
        let old = release("dev-1-1", true, "2026-10-12T10:00:00Z");
        let new = release("dev-2-1", true, "2026-10-13T10:00:00Z");
        let mut draft = release("dev-3-1", true, "2026-10-14T10:00:00Z");
        draft["draft"] = json!(true);
        let stable = release("v2.0.0", false, "2026-10-14T11:00:00Z");
        let found = discover(
            &fake(json!([old, draft, stable, new])),
            Channel::Development,
        )
        .unwrap()
        .unwrap();
        assert_eq!(found.version, "dev-2-1");
    }
    #[test]
    fn rejects_missing_hash_wrong_host_wrong_tag_and_duplicate_assets() {
        for (field, value) in [
            ("digest", Value::Null),
            (
                "browser_download_url",
                json!("https://example.com/archive.zip"),
            ),
            (
                "browser_download_url",
                json!("https://github.com/jgraving/slot/releases/download/dev-9-1/slot-System.zip"),
            ),
            ("size", json!(MAX_ARCHIVE + 1)),
            ("state", json!("new")),
        ] {
            let mut r = release("v1.2.3", false, "2026-10-14T10:00:00Z");
            r["assets"][1][field] = value;
            assert!(discover(&fake(r), Channel::Stable).unwrap().is_none());
        }
        let mut r = release("v1.2.3", false, "2026-10-14T10:00:00Z");
        let dup = r["assets"][0].clone();
        r["assets"].as_array_mut().unwrap().push(dup);
        assert!(discover(&fake(r), Channel::Stable).unwrap().is_none());
    }
    #[test]
    fn discovery_download_verification_and_staging_roundtrip() {
        use std::{collections::BTreeMap, io::Cursor};
        struct Responses(BTreeMap<String, Vec<u8>>);
        impl Fetch for Responses {
            fn get(&self, url: &str, limit: usize) -> io::Result<Vec<u8>> {
                let data = self.0.get(url).ok_or_else(invalid)?;
                assert!(data.len() <= limit);
                Ok(data.clone())
            }
        }
        let bytes = crate::tests::fixture(|_, _| {});
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut manifest = Vec::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_end(&mut manifest)
            .unwrap();
        let mut r = release("dev-1-1", true, "2026-10-14T10:00:00Z");
        let mut responses = BTreeMap::new();
        for (i, data) in [manifest, bytes].into_iter().enumerate() {
            r["assets"][i]["size"] = json!(data.len());
            r["assets"][i]["digest"] = json!(format!("sha256:{}", hash(&data)));
            responses.insert(
                r["assets"][i]["browser_download_url"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                data,
            );
        }
        responses.insert(
            format!("{API}?per_page=100"),
            serde_json::to_vec(&json!([r])).unwrap(),
        );
        let fetch = Responses(responses);
        let selected = discover(&fetch, Channel::Development).unwrap().unwrap();
        let verified = selected.download(&fetch).unwrap();
        let dir = tempfile::tempdir().unwrap();
        assert!(verified.stage(dir.path()).unwrap().join("READY").exists());
    }
    #[test]
    fn corrupted_downloads_and_invalid_api_fail_closed() {
        let selection = discover(
            &fake(release("v1.2.3", false, "2026-10-14T10:00:00Z")),
            Channel::Stable,
        )
        .unwrap()
        .unwrap();
        assert!(selection.download(&Fake(vec![0; 100])).is_err());
        assert!(discover(&Fake(b"not json".to_vec()), Channel::Stable).is_err());
        assert!(discover(&Fake(vec![0; API_LIMIT + 1]), Channel::Development).is_err());
    }
}
