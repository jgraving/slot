//! Offline parent rules. Wall-clock deadlines persist; gameplay uses monotonic time.
use slot_store::{atomic_write, Cart, Platform};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Daily,
    Session,
    Until,
}
impl Rule {
    pub fn next(self, direction: i32) -> Self {
        let n = match self {
            Self::Daily => 0,
            Self::Session => 1,
            Self::Until => 2,
        };
        [Self::Daily, Self::Session, Self::Until][(n + direction).rem_euclid(3) as usize]
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Daily => "Daily allowance",
            Self::Session => "Play for",
            Self::Until => "Play until",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Policy {
    pub active: bool,
    pub rule: Rule,
    pub minutes: u32,
    pub bedtime: u32,
    pub approved: Vec<(Platform, String)>,
    day: i64,
    used: u64,
    deadline: i64,
    last_wall: i64,
    clock_blocked: bool,
    credit_ms: u64,
    last_tick: Option<u64>,
    was_playing: bool,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            active: false,
            rule: Rule::Daily,
            minutes: 60,
            bedtime: 19 * 60 + 30,
            approved: Vec::new(),
            day: -1,
            used: 0,
            deadline: 0,
            last_wall: 0,
            clock_blocked: false,
            credit_ms: 0,
            last_tick: None,
            was_playing: false,
        }
    }
}
fn bad() -> io::Error {
    io::Error::other("Parent settings need attention")
}
impl Policy {
    pub fn directory(root: &Path) -> PathBuf {
        root.join("Config/kids")
    }
    pub fn read(root: &Path) -> io::Result<Self> {
        let dir = Self::directory(root);
        let text = match fs::read_to_string(dir.join("settings")) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound && !dir.join("code").exists() => {
                return Ok(Self::default())
            }
            Err(e) => return Err(e),
        };
        let mut lines = text.lines();
        if lines.next() != Some("SLKIDS01") {
            return Err(bad());
        }
        let nums = lines
            .next()
            .ok_or_else(bad)?
            .split_whitespace()
            .map(str::parse::<i64>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| bad())?;
        if nums.len() != 9
            || !(0..=1).contains(&nums[0])
            || !(0..=2).contains(&nums[1])
            || !(5..=240).contains(&nums[2])
            || !(0..1440).contains(&nums[3])
            || nums[5] < 0
            || nums[6] < 0
            || nums[7] < 0
            || !(0..=1).contains(&nums[8])
        {
            return Err(bad());
        }
        let mut p = Self {
            active: nums[0] == 1,
            rule: [Rule::Daily, Rule::Session, Rule::Until][nums[1] as usize],
            minutes: nums[2] as u32,
            bedtime: nums[3] as u32,
            day: nums[4],
            used: nums[5] as u64,
            deadline: nums[6],
            last_wall: nums[7],
            clock_blocked: nums[8] == 1,
            ..Self::default()
        };
        for line in lines {
            let (platform, stem) = line.split_once('/').ok_or_else(bad)?;
            let platform = Platform::ALL
                .into_iter()
                .find(|p| p.dir_name() == platform)
                .ok_or_else(bad)?;
            if stem.is_empty() || stem.contains(['/', '\\', '\0']) || stem == "." || stem == ".." {
                return Err(bad());
            }
            let key = (platform, stem.to_owned());
            if !p.approved.contains(&key) {
                p.approved.push(key);
            }
        }
        if p.active && !dir.join("code").is_file() {
            return Err(bad());
        }
        Ok(p)
    }
    pub fn save(&self, root: &Path) -> io::Result<()> {
        let dir = Self::directory(root);
        fs::create_dir_all(&dir)?;
        let rule = match self.rule {
            Rule::Daily => 0,
            Rule::Session => 1,
            Rule::Until => 2,
        };
        let mut text = format!(
            "SLKIDS01\n{} {} {} {} {} {} {} {} {}\n",
            u8::from(self.active),
            rule,
            self.minutes,
            self.bedtime,
            self.day,
            self.used,
            self.deadline,
            self.last_wall,
            u8::from(self.clock_blocked)
        );
        for (platform, stem) in &self.approved {
            text.push_str(&format!("{}/{stem}\n", platform.dir_name()));
        }
        atomic_write(&dir.join("settings"), text.as_bytes())
    }
    pub fn allows(&self, cart: &Cart) -> bool {
        self.approved.contains(&(cart.platform, cart.stem.clone()))
    }
    pub fn toggle(&mut self, cart: &Cart) {
        let key = (cart.platform, cart.stem.clone());
        if let Some(at) = self.approved.iter().position(|k| k == &key) {
            self.approved.remove(at);
        } else {
            self.approved.push(key);
        }
    }
    /// Parent-authorized start: daily usage is preserved; one-off sessions get a new deadline.
    pub fn begin(&mut self, wall: i64) -> io::Result<()> {
        if wall < 1_577_836_800 {
            return Err(io::Error::other("Set the clock first"));
        }
        if self.approved.is_empty() {
            return Err(io::Error::other("Approve a game with X first"));
        }
        let day = wall.div_euclid(86_400);
        if self.day != day {
            self.day = day;
            self.used = 0;
            self.credit_ms = 0;
        }
        self.deadline = match self.rule {
            Rule::Daily => 0,
            Rule::Session => wall + i64::from(self.minutes) * 60,
            Rule::Until => {
                let end = day * 86_400 + i64::from(self.bedtime) * 60;
                if end <= wall {
                    return Err(io::Error::other("Choose a later time today"));
                }
                end
            }
        };
        self.last_wall = wall;
        self.clock_blocked = false;
        self.last_tick = None;
        self.active = true;
        Ok(())
    }
    pub fn leave(&mut self) {
        self.active = false;
        self.last_tick = None;
    }
    pub fn remaining(&self, wall: i64) -> u64 {
        if self.clock_blocked || wall + 2 < self.last_wall {
            return 0;
        }
        match self.rule {
            Rule::Daily => {
                (u64::from(self.minutes) * 60).saturating_sub(self.used) + self.credit_ms / 1000
            }
            _ => self.deadline.saturating_sub(wall).max(0) as u64,
        }
    }
    /// Persist a minute before granting it. Abrupt power loss cannot refund playtime.
    /// A restart can conservatively lose at most the unused portion of that minute.
    pub fn tick(
        &mut self,
        root: &Path,
        wall: i64,
        monotonic_ms: u64,
        playing: bool,
    ) -> io::Result<bool> {
        let elapsed = if self.was_playing && playing {
            self.last_tick
                .map_or(0, |at| monotonic_ms.saturating_sub(at))
        } else {
            0
        };
        self.was_playing = playing;
        self.last_tick = Some(monotonic_ms);
        if !self.active {
            return Ok(true);
        }
        if wall < 1_577_836_800 || wall + 2 < self.last_wall {
            self.clock_blocked = true;
            self.save(root)?;
            return Ok(false);
        }
        let day = wall.div_euclid(86_400);
        if self.rule == Rule::Daily && day > self.day {
            self.day = day;
            self.used = 0;
            self.credit_ms = 0;
            self.last_wall = wall;
            self.save(root)?;
        }
        if self.clock_blocked {
            return Ok(false);
        }
        if self.rule != Rule::Daily {
            // Persist the high-water clock once a minute, not every frame.
            if wall >= self.last_wall + 60 {
                self.last_wall = wall;
                self.save(root)?;
            }
            return Ok(self.remaining(wall) > 0);
        }
        if !playing {
            return Ok(self.remaining(wall) > 0);
        }
        let excess = elapsed.saturating_sub(self.credit_ms);
        self.credit_ms = self.credit_ms.saturating_sub(elapsed);
        if self.credit_ms == 0 {
            let mut next = self.clone();
            let budget = u64::from(self.minutes) * 60;
            next.used = next.used.saturating_add(excess.div_ceil(1000)).min(budget);
            let grant = budget.saturating_sub(next.used).min(60);
            next.used += grant;
            next.last_wall = wall;
            next.save(root)?;
            next.credit_ms = grant * 1000;
            *self = next;
            return Ok(grant > 0);
        }
        Ok(true)
    }
}
