//! USB playback detection through ALSA's kernel inventory, without probing microphones.
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub route: String,
    pub name: String,
}

pub fn route_path() -> PathBuf {
    std::env::var_os("SLOT_AUDIO_ROUTE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/run/slot-audio-device"))
}

pub fn valid_route(route: &str) -> bool {
    let Some((id, device)) = route
        .strip_prefix("plughw:CARD=")
        .and_then(|s| s.split_once(",DEV="))
    else {
        return false;
    };
    !id.is_empty()
        && id.len() <= 32
        && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        && !device.is_empty()
        && device.len() <= 3
        && device.bytes().all(|c| c.is_ascii_digit())
}

pub fn discover(root: &Path) -> Vec<Device> {
    let mut devices = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return devices;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(number) = name.strip_prefix("card") else {
            continue;
        };
        if number.is_empty() || !number.bytes().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let card = entry.path();
        // usbid is exported by snd-usb-audio, not the internal codec or HDMI.
        if !card.join("usbid").is_file() {
            continue;
        }
        let Ok(id) = std::fs::read_to_string(card.join("id")) else {
            continue;
        };
        let id = id.trim();
        let description = std::fs::read_to_string(card.join("stream0"))
            .ok()
            .and_then(|s| s.lines().next().map(str::to_owned))
            .and_then(|s| s.split(" at ").next().map(str::to_owned))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| id.to_owned());
        let Ok(pcms) = std::fs::read_dir(&card) else {
            continue;
        };
        for pcm in pcms.flatten() {
            let name = pcm.file_name().to_string_lossy().into_owned();
            let Some(device) = name.strip_prefix("pcm").and_then(|s| s.strip_suffix('p')) else {
                continue;
            };
            let route = format!("plughw:CARD={id},DEV={device}");
            if valid_route(&route) {
                devices.push(Device {
                    route,
                    name: format!("USB · {description}"),
                });
            }
        }
    }
    devices.sort_by(|a, b| a.route.cmp(&b.route));
    devices
}

pub fn devices() -> Vec<Device> {
    discover(Path::new("/proc/asound"))
}

pub fn write_route(route: &str) -> std::io::Result<()> {
    if !route.is_empty() && !valid_route(route) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid USB audio output",
        ));
    }
    let path = route_path();
    let temp = path.with_extension("usb.tmp");
    std::fs::write(&temp, format!("{route}\n"))?;
    std::fs::rename(temp, path)
}

#[derive(Default)]
pub struct Monitor {
    checked: Option<std::time::Instant>,
    pub devices: Vec<Device>,
}
impl Monitor {
    pub fn poll(&mut self) {
        if self.checked.is_some_and(|t| t.elapsed().as_secs() < 1) {
            return;
        }
        self.checked = Some(std::time::Instant::now());
        self.devices = devices();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_only_usb_playback_and_uses_card_ids_not_numbers() {
        let root = tempfile::tempdir().unwrap();
        for (number, usb, playback) in [(0, false, true), (3, true, true), (4, true, false)] {
            let card = root.path().join(format!("card{number}"));
            std::fs::create_dir_all(card.join(if playback { "pcm0p" } else { "pcm0c" })).unwrap();
            std::fs::write(card.join("id"), format!("Audio{number}\n")).unwrap();
            if usb {
                std::fs::write(card.join("usbid"), "1234:5678").unwrap();
            }
        }
        let found = discover(root.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].route, "plughw:CARD=Audio3,DEV=0");
        std::fs::rename(root.path().join("card3"), root.path().join("card7")).unwrap();
        assert_eq!(discover(root.path()), found);
    }
    #[test]
    fn arbitrary_alsa_plugins_and_malformed_ids_are_rejected() {
        assert!(valid_route("plughw:CARD=Audio,DEV=0"));
        for s in [
            "default",
            "plughw:3,0",
            "plughw:CARD=Audio,DEV=0,foo=1",
            "plughw:CARD=../x,DEV=0",
            "plughw:CARD=,DEV=0",
        ] {
            assert!(!valid_route(s));
        }
    }
}
