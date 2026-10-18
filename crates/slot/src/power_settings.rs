//! Sleep preferences; reads the earlier comfort file without rewriting it.
use std::{path::Path, time::Duration};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PowerSettings {
    pub lid_close: u8,
    pub sleep_timeout: u8,
}
impl PowerSettings {
    pub fn read(root: &Path) -> Self {
        let stored = std::fs::read_to_string(root.join("Config/power.txt"))
            .or_else(|_| std::fs::read_to_string(root.join("Config/comfort.txt")))
            .ok();
        let text = stored.clone().unwrap_or_default();
        let mut settings = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match (key, value.parse::<u8>()) {
                ("lid_close", Ok(v @ 0..=2)) => settings.lid_close = v,
                ("sleep_timeout", Ok(v @ 0..=6)) => settings.sleep_timeout = v,
                _ => {}
            }
        }
        if stored.is_none() {
            if let Some(minutes) = slot_store::read_slot_state(root).lid_off_min {
                settings.sleep_timeout = match minutes {
                    1 => 4,
                    5 => 5,
                    10 => 6,
                    15 => 1,
                    30 => 2,
                    _ => 0,
                };
            }
        }
        settings
    }
    pub fn save(self, root: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(root.join("Config"))?;
        slot_store::atomic_write(
            &root.join("Config/power.txt"),
            format!(
                "lid_close={}\nsleep_timeout={}\n",
                self.lid_close, self.sleep_timeout
            )
            .as_bytes(),
        )
    }
    pub fn duration(self, default: Duration) -> Option<Duration> {
        match self.sleep_timeout {
            0 => Some(default),
            1 => Some(Duration::from_secs(900)),
            2 => Some(Duration::from_secs(1800)),
            4 => Some(Duration::from_secs(60)),
            5 => Some(Duration::from_secs(300)),
            6 => Some(Duration::from_secs(600)),
            _ => None,
        }
    }
}
