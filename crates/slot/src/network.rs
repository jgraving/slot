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
}

pub struct Manager {
    root: PathBuf,
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
            root,
            page: Page::Home,
            visible: true,
            ready: false,
            restore_on_ready: false,
            face: None,
            selected: 0,
            status: "WPA2 networks · encrypted saved credentials".into(),
            networks: Vec::new(),
            ssid: String::new(),
            text: Zeroizing::new(String::new()),
            bank: 0,
            cursor: 0,
            profile: None,
            pending: None,
        };
        manager.ask(Work::Prepare);
        manager
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
            ],
            Page::Scan => std::iter::once("Scan again".into())
                .chain(self.networks.iter().map(|n| n.ssid.clone()))
                .collect(),
            Page::SaveChoice => vec![
                "Connect for this session".into(),
                "Save encrypted credentials".into(),
            ],
            Page::Forget => vec!["Cancel".into(), "Delete encrypted credentials".into()],
            Page::Entry(_) => Vec::new(),
        }
    }
    pub fn input(&mut self, action: Action) {
        if self.pending.is_some() {
            return;
        }
        if matches!(action, Action::QuickMenu | Action::GbaDown(Btn::B)) {
            self.text.zeroize();
            if self.page == Page::Home {
                self.visible = false;
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
                Page::Entry(_) => {}
            },
            _ => {}
        }
    }
    pub fn lines(&self) -> Vec<String> {
        let title = match self.page {
            Page::Entry(Entry::Password) => "Wi-Fi password",
            Page::Forget => "Forget saved Wi-Fi?",
            Page::SaveChoice => "Remember this network?",
            _ => "Network",
        };
        let message = if matches!(self.page, Page::Entry(_)) {
            format!("{}\n{}", "•".repeat(self.text.len().min(40)), self.status)
        } else {
            self.status.clone()
        };
        let mut lines = vec![
            title.into(),
            message,
            if matches!(self.page, Page::Entry(_)) {
                "X Delete · L1/R1 Page · START Done".into()
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
            let first = self.selected.saturating_sub(3);
            for (i, row) in self.rows().iter().enumerate().skip(first).take(4) {
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
            Ok("Wi-Fi is optional · choose a network".into())
        }
        Work::Scan => {
            networks = Some(parse_scan(&helper(&root, "scan", None)?));
            Ok("Select a WPA2 network".into())
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
        message: result
            .unwrap_or_else(|_| "Operation failed; check saved network, password or radio".into()),
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
        .stderr(Stdio::null())
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
        return Err(io::Error::other("Network helper failed"));
    }
    if output.stdout.len() > 64 * 1024 {
        return Err(io::Error::other("Network reply too large"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
