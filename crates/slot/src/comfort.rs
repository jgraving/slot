//! Device preferences kept separately from cartridges and save states.
use slot_store::{atomic_write, SlotState};
use std::{collections::BTreeMap, io, path::Path, sync::atomic::AtomicBool};
pub static QUIET_AUDIO: AtomicBool = AtomicBool::new(false);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Display {
    pub brightness: u8,
    pub warmth: u8,
}
#[derive(Clone, Copy, Debug)]
pub struct Headset {
    pub volume: u8,
    pub muted: bool,
}
#[derive(Clone, Debug)]
pub struct Comfort {
    pub night: bool,
    pub day: Display,
    pub bedtime: Display,
    pub quiet: bool,
    pub sounds: bool,
    pub shelf: usize,
    pub positions: BTreeMap<usize, String>,
    pub headsets: BTreeMap<String, Headset>,
}
impl Comfort {
    pub fn new(state: &SlotState) -> Self {
        Self {
            night: false,
            day: Display {
                brightness: state.brightness,
                warmth: state.blue_light,
            },
            bedtime: Display {
                brightness: 2,
                warmth: 9,
            },
            quiet: false,
            sounds: true,
            shelf: 0,
            positions: BTreeMap::new(),
            headsets: BTreeMap::new(),
        }
    }
    pub fn read(root: &Path, state: &SlotState) -> io::Result<Self> {
        let mut c = Self::new(state);
        let text = match std::fs::read_to_string(root.join("Config/comfort.txt")) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(c),
            Err(e) => return Err(e),
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let display = || -> Option<Display> {
                let (b, w) = value.split_once(',')?;
                Some(Display {
                    brightness: b.parse::<u8>().ok()?.min(9),
                    warmth: w.parse::<u8>().ok()?.min(9),
                })
            };
            match key {
                "night" => c.night = value == "1",
                "quiet" => c.quiet = value == "1",
                "sounds" => c.sounds = value == "1",
                "day" => {
                    if let Some(d) = display() {
                        c.day = d
                    }
                }
                "bedtime" => {
                    if let Some(d) = display() {
                        c.bedtime = d
                    }
                }
                "shelf" => {
                    if let Ok(n @ 0..=4) = value.parse() {
                        c.shelf = n
                    }
                }
                _ => {
                    if let Some(k) = key
                        .strip_prefix("position.")
                        .and_then(|k| k.parse::<usize>().ok())
                        .filter(|n| *n < 5)
                    {
                        c.positions.insert(k, value.to_owned());
                    }
                    if let Some(address) = key
                        .strip_prefix("headset.")
                        .filter(|a| crate::bluetooth::valid_address(a))
                    {
                        if let Some((v, m)) = value.split_once(',') {
                            if let Ok(volume) = v.parse::<u8>() {
                                c.headsets.insert(
                                    address.to_ascii_uppercase(),
                                    Headset {
                                        volume: volume.min(100),
                                        muted: m == "1",
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(c)
    }
    pub fn save(&self, root: &Path) -> io::Result<()> {
        let mut text = format!(
            "night={}\nquiet={}\nsounds={}\nday={},{}\nbedtime={},{}\nshelf={}\n",
            u8::from(self.night),
            u8::from(self.quiet),
            u8::from(self.sounds),
            self.day.brightness,
            self.day.warmth,
            self.bedtime.brightness,
            self.bedtime.warmth,
            self.shelf
        );
        for (i, key) in &self.positions {
            if !key.contains(['\n', '\r']) {
                text.push_str(&format!("position.{i}={key}\n"));
            }
        }
        for (address, h) in &self.headsets {
            text.push_str(&format!(
                "headset.{address}={},{}\n",
                h.volume,
                u8::from(h.muted)
            ));
        }
        std::fs::create_dir_all(root.join("Config"))?;
        atomic_write(&root.join("Config/comfort.txt"), text.as_bytes())
    }
}
