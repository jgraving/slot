//! Day/night display presets, with read-only migration from the earlier fork.
use slot_store::SlotState;
use std::{io, path::Path};
#[derive(Clone, Copy, Debug)]
pub struct DisplaySettings {
    pub night: bool,
    pub day: (u8, u8),
    pub bedtime: (u8, u8),
}
impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            night: false,
            day: (9, 0),
            bedtime: (2, 9),
        }
    }
}
impl DisplaySettings {
    pub fn read(root: &Path, state: &SlotState) -> Self {
        let mut s = Self {
            day: (state.brightness, state.blue_light),
            ..Self::default()
        };
        let text = match std::fs::read_to_string(root.join("Config/display.txt")) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                std::fs::read_to_string(root.join("Config/comfort.txt")).unwrap_or_default()
            }
            Err(_) => String::new(),
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key == "night" {
                s.night = value == "1";
            }
            if key == "day" || key == "bedtime" {
                if let Some((b, w)) = value.split_once(',') {
                    if let (Ok(b @ 0..=9), Ok(w @ 0..=9)) = (b.parse::<u8>(), w.parse::<u8>()) {
                        if key == "day" {
                            s.day = (b, w);
                        } else {
                            s.bedtime = (b, w);
                        }
                    }
                }
            }
        }
        s
    }
    pub fn current(&self) -> (u8, u8) {
        if self.night {
            self.bedtime
        } else {
            self.day
        }
    }
    pub fn remember(&mut self, b: u8, w: u8) {
        if self.night {
            self.bedtime = (b, w);
        } else {
            self.day = (b, w);
        }
    }
    pub fn save(&self, root: &Path) -> io::Result<()> {
        std::fs::create_dir_all(root.join("Config"))?;
        slot_store::atomic_write(
            &root.join("Config/display.txt"),
            format!(
                "night={}\nday={},{}\nbedtime={},{}\n",
                u8::from(self.night),
                self.day.0,
                self.day.1,
                self.bedtime.0,
                self.bedtime.1
            )
            .as_bytes(),
        )
    }
}
