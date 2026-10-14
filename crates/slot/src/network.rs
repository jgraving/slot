//! Wi-Fi UI and bounded workers. Secrets never enter argv, environment or logs.
use crate::wifi_vault::{Profile, Vault};
use slot_input::{Action, Btn};
use slot_ui::TexId;
use std::{
    io,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone, PartialEq, Eq)]
pub struct Network {
    pub ssid: String,
}

pub fn parse_scan(text: &str) -> Vec<Network> {
    let mut networks = Vec::new();
    for line in text.lines() {
        let fields: Vec<_> = line.splitn(5, '\t').collect();
        if fields.len() != 5 || !fields[3].contains("WPA2-PSK") || !fields[3].contains("CCMP") {
            continue;
        }
        let raw = fields[4].as_bytes();
        let mut bytes = Vec::new();
        let mut i = 0;
        while i < raw.len() {
            if raw[i] == b'\\' && i + 3 < raw.len() && raw[i + 1] == b'x' {
                if let Ok(h) = std::str::from_utf8(&raw[i + 2..i + 4]) {
                    if let Ok(b) = u8::from_str_radix(h, 16) {
                        bytes.push(b);
                        i += 4;
                        continue;
                    }
                }
            }
            if raw[i] == b'\\' && raw.get(i + 1) == Some(&b'\\') {
                i += 1;
            }
            bytes.push(raw[i]);
            i += 1;
        }
        let Ok(ssid) = String::from_utf8(bytes) else {
            continue;
        };
        if ssid.is_empty() || ssid.len() > 32 || ssid.chars().any(char::is_control) {
            continue;
        }
        if !networks.iter().any(|n: &Network| n.ssid == ssid) {
            networks.push(Network { ssid });
        }
    }
    networks.sort_by(|a, b| a.ssid.cmp(&b.ssid));
    networks
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Entry {
    Password,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Scan,
    Entry(Entry),
    Forget,
    SaveChoice,
    UnlockCode,
    SetCode,
    ConfirmCode,
}

pub struct Manager {
    root: PathBuf,
    gate: crate::network_lock::Gate,
    code: Zeroizing<Vec<u8>>,
    new_code: Zeroizing<Vec<u8>>,
    page: Page,
    pub visible: bool,
    ready: bool,
    restore_on_ready: bool,
    pub face: Option<TexId>,
    pub selected: usize,
    pub status: String,
    pub networks: Vec<Network>,
    ssid: String,
    text: Zeroizing<String>,
    bank: usize,
    cursor: usize,
    profile: Option<Profile>,
    pending: Option<Receiver<Reply>>,
}

struct Reply {
    message: String,
    networks: Option<Vec<Network>>,
    profile: Option<Profile>,
    need_vault: bool,
    prepared: bool,
}

enum Work {
    Prepare,
    Scan,
    Password(String, Zeroizing<String>),
    Save,
    Reconnect,
    Disconnect,
    Forget,
}

impl Manager {
    pub fn new(root: PathBuf) -> Self {
        let mut manager = Self {
            gate: crate::network_lock::Gate::new("/data/slot-network/button-lock".into()),
            code: Zeroizing::new(Vec::new()),
            new_code: Zeroizing::new(Vec::new()),
            root,
            page: Page::Home,
            visible: true,
            ready: false,
            restore_on_ready: false,
            face: None,
            selected: 0,
            status: String::new(),
            networks: Vec::new(),
            ssid: String::new(),
            text: Zeroizing::new(String::new()),
            bank: 0,
            cursor: 0,
            profile: None,
            pending: None,
        };
        manager.open();
        manager.ask(Work::Prepare);
        manager
    }
    pub fn lock_controls(&mut self) {
        self.gate.relock();
        self.code.zeroize();
        self.new_code.zeroize();
        self.text.zeroize();
        self.page = if self.gate.locked() {
            self.status.clear();
            Page::UnlockCode
        } else {
            Page::Home
        };
        self.selected = 0;
        self.face = None;
    }
    pub fn open(&mut self) {
        self.face = None;
        self.visible = true;
        self.page = if self.gate.locked() {
            self.status.clear();
            Page::UnlockCode
        } else {
            Page::Home
        };
        self.code.zeroize();
        self.new_code.zeroize();
        self.selected = 0;
    }
    pub fn screen_face(&self) -> slot_ui::UndoFace {
        let mut face = crate::bluetooth::screen_face(&self.lines());
        if matches!(
            self.page,
            Page::UnlockCode | Page::SetCode | Page::ConfirmCode
        ) {
            let icons = slot_ui::button_code_face(&self.code);
            let left = (face.w - icons.w) / 2;
            for y in 0..icons.h {
                for x in 0..icons.w {
                    let src = ((y * icons.w + x) * 4) as usize;
                    let dst = (((y + 208) * face.w + x + left) * 4) as usize;
                    let alpha = icons.rgba[src + 3] as u32;
                    for c in 0..3 {
                        face.rgba[dst + c] = ((icons.rgba[src + c] as u32 * alpha
                            + face.rgba[dst + c] as u32 * (255 - alpha))
                            / 255) as u8;
                    }
                    face.rgba[dst + 3] = 255;
                }
            }
        }
        if matches!(self.page, Page::Home | Page::Scan) {
            let count = self.rows().len();
            if count > 6 {
                let height = 6 * 52 - 8;
                let thumb = height * 6 / count;
                let first = self.selected.saturating_sub(5);
                let offset = (height - thumb) * first / (count - 6);
                for (top, h, colour) in [
                    (88, height, [51, 51, 61, 255]),
                    (88 + offset, thumb, [153, 153, 163, 255]),
                ] {
                    for y in top..top + h {
                        for x in face.w - 12..face.w - 9 {
                            let at = (y * face.w as usize + x as usize) * 4;
                            face.rgba[at..at + 4].copy_from_slice(&colour);
                        }
                    }
                }
            }
        }
        face
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn boot(root: PathBuf) -> Option<Self> {
        if !std::path::Path::new("/data/slot-network/enabled").is_file() {
            return None;
        }
        let mut manager = Self::new(root);
        manager.visible = false;
        manager.restore_on_ready = true;
        Some(manager)
    }
    fn entry(&mut self, kind: Entry) {
        self.page = Page::Entry(kind);
        self.text.zeroize();
        self.bank = 0;
        self.cursor = 0;
        self.status.clear();
    }
    fn ask(&mut self, work: Work) {
        if self.pending.is_some() {
            return;
        }
        let root = self.root.clone();
        let profile = self.profile.take();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.status = "Working…".into();
        std::thread::spawn(move || {
            let _ = tx.send(worker(root, work, profile));
        });
    }
    pub fn poll(&mut self) {
        let Some(rx) = &self.pending else {
            return;
        };
        match rx.try_recv() {
            Ok(reply) => {
                self.pending = None;
                self.status = reply.message;
                self.ready |= reply.prepared;
                if self.restore_on_ready && self.ready {
                    self.restore_on_ready = false;
                    self.ask(Work::Reconnect);
                    return;
                }
                if let Some(networks) = reply.networks {
                    self.networks = networks;
                }
                self.profile = reply.profile;
                if reply.need_vault {
                    self.page = Page::SaveChoice;
                    self.selected = 0;
                } else if matches!(self.page, Page::Entry(_) | Page::Forget | Page::SaveChoice) {
                    self.page = Page::Home;
                    self.selected = 0;
                }
                if self.visible && self.gate.locked() {
                    self.page = Page::UnlockCode;
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                self.status = "Network worker stopped".into();
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    pub fn rows(&self) -> Vec<String> {
        match self.page {
            Page::Home => vec![
                "Find a network".into(),
                "Connect saved network".into(),
                "Turn Wi-Fi off".into(),
                "Forget saved network".into(),
                if self.gate.enabled() {
                    "Disable button lock"
                } else {
                    "Set button lock (off)"
                }
                .into(),
            ],
            Page::Scan => std::iter::once("Scan again".into())
                .chain(self.networks.iter().map(|n| format!("DEVICE {}", n.ssid)))
                .collect(),
            Page::SaveChoice => vec![
                "Connect for this session".into(),
                "Save encrypted credentials".into(),
            ],
            Page::Forget => vec!["Cancel".into(), "Delete encrypted credentials".into()],
            Page::Entry(_) | Page::UnlockCode | Page::SetCode | Page::ConfirmCode => Vec::new(),
        }
    }
    pub fn input(&mut self, action: Action) {
        if self.pending.is_some() {
            return;
        }
        if matches!(
            self.page,
            Page::UnlockCode | Page::SetCode | Page::ConfirmCode
        ) {
            self.code_input(action);
            return;
        }
        if matches!(action, Action::QuickMenu | Action::GbaDown(Btn::B)) {
            self.text.zeroize();
            if self.page == Page::Home {
                self.visible = false;
                self.gate.relock();
            } else {
                self.page = Page::Home;
                self.selected = 0;
            }
            return;
        }
        if !self.ready {
            self.status = "Secure network setup unavailable; retry after restarting".into();
            return;
        }
        if let Page::Entry(kind) = self.page {
            match action {
                Action::GbaDown(Btn::Left) => self.cursor = (self.cursor + 39) % 40,
                Action::GbaDown(Btn::Right) => self.cursor = (self.cursor + 1) % 40,
                Action::GbaDown(Btn::Up) => self.cursor = (self.cursor + 30) % 40,
                Action::GbaDown(Btn::Down) => self.cursor = (self.cursor + 10) % 40,
                Action::GbaDown(Btn::L1) => self.bank = (self.bank + 2) % 3,
                Action::GbaDown(Btn::R1) => self.bank = (self.bank + 1) % 3,
                Action::GbaDown(Btn::X) => {
                    self.text.pop();
                }
                Action::GbaDown(Btn::A) => {
                    let code = 32 + self.bank * 40 + self.cursor;
                    if code <= 126 && self.text.len() < 63 {
                        self.text.push(code as u8 as char);
                    }
                }
                Action::GbaDown(Btn::Start) => {
                    let input = std::mem::replace(&mut self.text, Zeroizing::new(String::new()));
                    match kind {
                        Entry::Password => self.ask(Work::Password(self.ssid.clone(), input)),
                    }
                }
                _ => {}
            }
            return;
        }
        let count = self.rows().len();
        match action {
            Action::GbaDown(Btn::Up) => self.selected = (self.selected + count - 1) % count,
            Action::GbaDown(Btn::Down) => self.selected = (self.selected + 1) % count,
            Action::GbaDown(Btn::A) => match self.page {
                Page::Home => match self.selected {
                    0 => {
                        self.page = Page::Scan;
                        self.selected = 0;
                        self.ask(Work::Scan);
                    }
                    1 => self.ask(Work::Reconnect),
                    2 => self.ask(Work::Disconnect),
                    4 => {
                        if self.gate.enabled() {
                            self.status = if self.gate.remove().is_ok() {
                                "Button lock disabled"
                            } else {
                                "Could not disable lock"
                            }
                            .into();
                        } else {
                            self.page = Page::SetCode;
                            self.code.zeroize();
                            self.status.clear();
                        }
                    }
                    _ => {
                        self.page = Page::Forget;
                        self.selected = 0;
                    }
                },
                Page::Scan => {
                    if self.selected == 0 {
                        self.ask(Work::Scan);
                    } else if let Some(network) = self.networks.get(self.selected - 1) {
                        self.ssid = network.ssid.clone();
                        self.entry(Entry::Password);
                    }
                }
                Page::SaveChoice if self.selected == 1 => self.ask(Work::Save),
                Page::SaveChoice => {
                    self.page = Page::Home;
                    self.selected = 0;
                    self.status = "Connected for this session; nothing saved".into();
                }
                Page::Forget if self.selected == 1 => self.ask(Work::Forget),
                Page::Forget => {
                    self.page = Page::Home;
                    self.selected = 0;
                }
                Page::Entry(_) | Page::UnlockCode | Page::SetCode | Page::ConfirmCode => {}
            },
            _ => {}
        }
    }
    fn code_input(&mut self, action: Action) {
        self.face = None;
        if matches!(action, Action::QuickMenu) {
            self.code.zeroize();
            self.new_code.zeroize();
            self.gate.relock();
            self.visible = false;
            return;
        }
        if !self.ready {
            return;
        }
        let Action::GbaDown(button) = action else {
            return;
        };
        if button == Btn::Select {
            self.code.pop();
            return;
        }
        if button == Btn::Start {
            if self.code.len() != 4 {
                self.status = "Enter all four buttons".into();
                return;
            }
            match self.page {
                Page::UnlockCode => {
                    let now = std::time::Instant::now();
                    match self.gate.verify(&self.code, now) {
                        Ok(true) => {
                            self.page = Page::Home;
                            self.status = "Unlocked".into();
                        }
                        _ => {
                            self.status = if self.gate.remaining(now) > 0 {
                                "Wait 30 seconds before trying again"
                            } else {
                                "Code did not match"
                            }
                            .into();
                        }
                    }
                }
                Page::SetCode => {
                    self.new_code = Zeroizing::new(self.code.to_vec());
                    self.page = Page::ConfirmCode;
                    self.status.clear();
                }
                Page::ConfirmCode => {
                    if *self.code != *self.new_code {
                        self.status = "Codes did not match. Try again.".into();
                    } else if self.gate.set(&self.code).is_ok() {
                        self.page = Page::Home;
                        self.status = "Button lock enabled".into();
                        self.new_code.zeroize();
                    } else {
                        self.status = "Could not save button lock".into();
                    }
                }
                _ => {}
            }
            self.code.zeroize();
            return;
        }
        let value = match button {
            Btn::A => 0,
            Btn::B => 1,
            Btn::X => 2,
            Btn::Y => 3,
            Btn::Up => 4,
            Btn::Down => 5,
            Btn::Left => 6,
            Btn::Right => 7,
            _ => return,
        };
        if self.code.len() < 4 {
            self.code.push(value);
        }
    }
    pub fn lines(&self) -> Vec<String> {
        if matches!(
            self.page,
            Page::UnlockCode | Page::SetCode | Page::ConfirmCode
        ) {
            return vec![
                match self.page {
                    Page::SetCode => "Press four buttons",
                    Page::ConfirmCode => "Repeat your code",
                    _ => "Enter your code",
                }
                .into(),
                self.status.clone(),
                String::new(),
                "ROW ".into(),
                "ROW ".into(),
                "ROW ".into(),
                "CODE HINTS".into(),
            ];
        }
        let title = match self.page {
            Page::Entry(Entry::Password) => "Wi-Fi password",
            Page::Forget => "Forget saved Wi-Fi?",
            Page::SaveChoice => "Remember this network?",
            _ => "",
        };
        let message = if matches!(self.page, Page::Entry(_)) {
            format!("{}\n{}", "•".repeat(self.text.len().min(40)), self.status)
        } else {
            self.status.clone()
        };
        let compact = matches!(self.page, Page::Home | Page::Scan);
        let mut lines = vec![
            title.into(),
            message,
            if matches!(self.page, Page::Entry(_)) {
                "X Delete · L1/R1 Page · START Done".into()
            } else if compact {
                "COMPACT".into()
            } else {
                String::new()
            },
        ];
        if matches!(self.page, Page::Entry(_)) {
            for row in 0..4 {
                let mut line = String::from("ROW ");
                for col in 0..10 {
                    let i = row * 10 + col;
                    let code = 32 + self.bank * 40 + i;
                    let ch = if code > 126 {
                        ' '
                    } else if code == 32 {
                        '␣'
                    } else {
                        code as u8 as char
                    };
                    if i == self.cursor {
                        line.push_str(&format!("[{ch}]"));
                    } else {
                        line.push_str(&format!(" {ch} "));
                    }
                }
                lines.push(line);
            }
        } else {
            let count = if compact { 6 } else { 4 };
            let first = self.selected.saturating_sub(count - 1);
            for (i, row) in self.rows().iter().enumerate().skip(first).take(count) {
                lines.push(format!(
                    "{}{}",
                    if i == self.selected {
                        "SELECTED "
                    } else {
                        "ROW "
                    },
                    row
                ));
            }
        }
        lines.push("B Back · A Select".into());
        lines
    }
}

fn worker(root: PathBuf, work: Work, mut profile: Option<Profile>) -> Reply {
    let private = std::path::Path::new("/data/slot-network");
    let path = private.join("wifi.vault");
    let key = private.join("vault.key");
    let enabled = private.join("enabled");
    let mut networks = None;
    let mut need_vault = false;
    let mut prepared = false;
    let result: io::Result<String> = (|| match work {
        Work::Prepare => {
            helper(&root, "prepare", None)?;
            prepared = true;
            Ok(String::new())
        }
        Work::Scan => {
            let found = parse_scan(&helper(&root, "scan", None)?);
            let message = if found.is_empty() {
                "No WPA2 networks found"
            } else {
                ""
            };
            networks = Some(found);
            Ok(message.into())
        }
        Work::Password(ssid, password) => {
            let p = Profile::from_password(ssid, password)?;
            let status = helper(&root, "connect", Some(p.supplicant()))?;
            need_vault = true;
            profile = Some(p);
            Ok(connection_status(&status))
        }
        Work::Save => {
            if path.exists() && !key.exists() {
                return Err(io::Error::other(
                    "Saved key is missing; forget the old network first",
                ));
            }
            let vault = Vault::device(&key)?;
            vault.save(
                &path,
                profile
                    .as_ref()
                    .ok_or_else(|| io::Error::other("No network to save"))?,
            )?;
            std::fs::write(enabled, b"1")?;
            Ok("Saved · reconnects automatically after boot".into())
        }
        Work::Reconnect => {
            if !path.is_file() || !key.is_file() {
                return Err(io::Error::other("No saved network"));
            }
            let p = Vault::device(&key)?.read(&path)?;
            let status = helper(&root, "connect", Some(p.supplicant()))?;
            std::fs::write(enabled, b"1")?;
            profile = Some(p);
            Ok(connection_status(&status))
        }
        Work::Disconnect | Work::Forget => {
            // Disable auto reconnect even if the radio has disappeared.
            if enabled.exists() {
                std::fs::remove_file(enabled)?;
            }
            helper(&root, "disconnect", None)?;
            profile = None;
            if matches!(work, Work::Forget) {
                if path.exists() {
                    std::fs::remove_file(path)?;
                }
                if key.exists() {
                    std::fs::remove_file(key)?;
                }
            }
            Ok("Wi-Fi off · automatic reconnect disabled".into())
        }
    })();
    Reply {
        message: result.unwrap_or_else(|error| safe_error(&error.to_string()).into()),
        networks,
        profile,
        need_vault,
        prepared,
    }
}

fn connection_status(text: &str) -> String {
    let ip = text
        .lines()
        .find_map(|l| l.strip_prefix("IP "))
        .unwrap_or("address unavailable");
    format!("Connected · {ip}")
}

// Only our fixed messages may reach the UI. Never echo arbitrary tool output:
// it could contain network identifiers or configuration details.
fn safe_error(text: &str) -> &'static str {
    const MESSAGES: &[&str] = &[
        "Wi-Fi requires the BaseOS frontend",
        "RAM runtime unavailable",
        "Persistent private storage unavailable",
        "Unsafe Wi-Fi directory",
        "Wi-Fi is busy",
        "Could not disable persistent swap",
        "Swap still active",
        "Unknown BaseOS login layout",
        "Could not stop password SSH",
        "Finish link play before using home Wi-Fi",
        "Wi-Fi radio unavailable",
        "Could not stop old Wi-Fi session",
        "Could not start Wi-Fi",
        "Connection failed; check network and password",
        "Could not obtain an IP address",
        "Invalid WPA2 credentials",
        "Saved key is missing; forget the old network first",
        "No saved network",
        "Wrong device key or damaged vault",
        "Unsafe device key",
    ];
    text.lines()
        .find_map(|line| MESSAGES.iter().copied().find(|message| *message == line))
        .unwrap_or("Wi-Fi operation failed; try again")
}

fn helper(
    root: &std::path::Path,
    verb: &str,
    input: Option<Zeroizing<String>>,
) -> io::Result<String> {
    let mut child = Command::new("timeout")
        .args(["80", "sh"])
        .arg(root.join("System/network.sh"))
        .arg(verb)
        .env("SLOT_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(input) = input {
        let result = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("No input pipe"))?
            .write_all(input.as_bytes());
        if let Err(error) = result {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    } else {
        child.stdin.take();
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(io::Error::other(safe_error(&String::from_utf8_lossy(
            &output.stderr,
        ))));
    }
    if output.stdout.len() > 64 * 1024 {
        return Err(io::Error::other("Network reply too large"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod lock_tests {
    use super::*;
    fn tap(m: &mut Manager, b: Btn) {
        m.input(Action::GbaDown(b));
    }
    fn code(m: &mut Manager) {
        for b in [Btn::A, Btn::B, Btn::X, Btn::Y, Btn::Start] {
            tap(m, b);
        }
    }
    #[test]
    fn code_screen_has_one_prompt_four_slots_and_a_consistent_footer() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = Manager::new(dir.path().to_owned());
        m.pending = None;
        m.ready = true;
        m.status.clear();
        m.gate = crate::network_lock::Gate::new(dir.path().join("button-lock"));
        m.page = Page::SetCode;
        let empty = m.screen_face();
        assert_eq!(m.lines()[0], "Press four buttons");
        assert!(m.lines()[2].is_empty());
        tap(&mut m, Btn::Start);
        assert!(m.visible);
        assert!(matches!(m.page, Page::SetCode));
        m.status.clear();
        tap(&mut m, Btn::A);
        let entered = m.screen_face();
        let start = (208 * empty.w * 4) as usize;
        let end = (272 * empty.w * 4) as usize;
        assert_ne!(&empty.rgba[start..end], &entered.rgba[start..end]);
        let footer = (427 * empty.w * 4) as usize;
        assert_eq!(&empty.rgba[footer..], &entered.rgba[footer..]);
        tap(&mut m, Btn::Select);
        assert!(m.code.is_empty());
        code(&mut m);
        assert_eq!(m.lines()[0], "Repeat your code");
        assert!(m.status.is_empty());
        let confirm = m.screen_face();
        assert_eq!(&empty.rgba[footer..], &confirm.rgba[footer..]);
        if let Ok(path) = std::env::var("SLOT_NETWORK_PREVIEW_DIR") {
            std::fs::create_dir_all(&path).unwrap();
            for (name, face) in [
                ("code-empty", empty),
                ("code-entered", entered),
                ("code-confirm", confirm),
            ] {
                let file = std::fs::File::create(format!("{path}/{name}.png")).unwrap();
                let mut encoder = png::Encoder::new(file, face.w, face.h);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&face.rgba)
                    .unwrap();
            }
        }
    }
    #[test]
    fn optional_code_requires_confirmation_and_relocks_on_close() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = Manager::new(dir.path().to_owned());
        m.pending = None;
        m.ready = true;
        m.gate = crate::network_lock::Gate::new(dir.path().join("button-lock"));
        m.open();
        m.selected = 4;
        tap(&mut m, Btn::A);
        assert!(matches!(m.page, Page::SetCode));
        code(&mut m);
        assert!(matches!(m.page, Page::ConfirmCode));
        assert!(!m.gate.enabled());
        code(&mut m);
        assert!(m.gate.enabled());
        tap(&mut m, Btn::B);
        assert!(!m.visible);
        m.open();
        assert!(matches!(m.page, Page::UnlockCode));
        tap(&mut m, Btn::A);
        assert_eq!(&*m.code, &[0]);
        tap(&mut m, Btn::Select);
        assert!(m.code.is_empty());
        code(&mut m);
        assert!(matches!(m.page, Page::Home));
    }
}

#[cfg(test)]
mod error_tests {
    use super::safe_error;
    #[test]
    fn diagnostic_messages_are_useful_without_echoing_tool_output() {
        assert_eq!(
            safe_error("Wi-Fi radio unavailable\n"),
            "Wi-Fi radio unavailable"
        );
        assert_eq!(
            safe_error("password=sentinel-secret"),
            "Wi-Fi operation failed; try again"
        );
        assert_eq!(
            safe_error("Could not start Wi-Fi: sentinel-secret"),
            "Wi-Fi operation failed; try again"
        );
    }
}
