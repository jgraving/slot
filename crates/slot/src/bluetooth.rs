//! Asynchronous headphone management through BaseOS's existing BlueZ/BlueALSA stack.
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use slot_input::{Action, Btn};
use slot_ui::{TexId, UndoFace, OUT_H, OUT_W};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Headphones {
    pub address: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
}

pub fn parse_devices(text: &str) -> Vec<Headphones> {
    let mut devices = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("DEVICE ") else {
            continue;
        };
        let Some((address, name)) = rest.split_once(' ') else {
            continue;
        };
        let raw = name.trim().replace([':', '-'], "");
        let named = !(name.trim().is_empty()
            || raw.len() == 12 && raw.bytes().all(|b| b.is_ascii_hexdigit()));
        if named
            && valid_address(address)
            && !devices.iter().any(|d: &Headphones| d.address == address)
        {
            devices.push(Headphones {
                address: address.to_owned(),
                name: name.to_owned(),
                paired: false,
                connected: false,
            });
        }
    }
    for line in text.lines() {
        let words: Vec<_> = line.split_whitespace().collect();
        if words.len() == 4 && words[0] == "STATE" {
            if let Some(d) = devices.iter_mut().find(|d| d.address == words[1]) {
                d.paired = words[2] == "yes";
                d.connected = words[3] == "yes";
            }
        }
    }
    devices
}

fn valid_address(address: &str) -> bool {
    let parts: Vec<_> = address.split(':').collect();
    parts.len() == 6
        && parts
            .iter()
            .all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

struct ResultMessage {
    devices: Vec<Headphones>,
    status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Home,
    Pair,
    Saved,
    Device(usize),
    Forget(usize),
    Output,
    Diagnostics,
}

pub struct Manager {
    root: PathBuf,
    pub page: Page,
    stack: Vec<Page>,
    pub devices: Vec<Headphones>,
    pub selected: usize,
    pub status: String,
    pending: Option<Receiver<ResultMessage>>,
    pub face: Option<TexId>,
}

impl Manager {
    pub fn new(root: PathBuf) -> Self {
        let mut manager = Self {
            root,
            page: Page::Home,
            stack: Vec::new(),
            devices: Vec::new(),
            selected: 0,
            status: "Loading Bluetooth...".into(),
            pending: None,
            face: None,
        };
        manager.ask("list", None);
        manager
    }

    fn ask(&mut self, verb: &'static str, address: Option<String>) {
        if self.pending.is_some() {
            return;
        }
        let root = self.root.clone();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.status = match verb {
            "scan" => "Scanning; put headphones in pairing mode",
            "connect" => "Pairing and connecting...",
            "forget" => "Forgetting headphones...",
            "speaker" => "Switching to speaker...",
            _ => "Loading Bluetooth...",
        }
        .into();
        std::thread::spawn(move || {
            let _ = tx.send(run(root, verb, address));
        });
    }

    pub fn poll(&mut self) {
        let Some(rx) = &self.pending else { return };
        match rx.try_recv() {
            Ok(message) => {
                if !message.status.starts_with("Error:")
                    && !message.status.starts_with("Saved Bluetooth/")
                {
                    self.devices = message.devices;
                }
                self.status = message.status;
                self.selected = self.selected.min(self.rows().len().saturating_sub(1));
                self.pending = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.status = "Error: Bluetooth worker stopped".into();
                self.pending = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    fn open(&mut self, page: Page) {
        self.stack.push(self.page.clone());
        self.page = page;
        self.selected = 0;
        self.status.clear();
    }

    fn device_indices(&self) -> Vec<usize> {
        self.devices
            .iter()
            .enumerate()
            .filter(|(_, d)| self.page != Page::Saved || d.paired)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn rows(&self) -> Vec<String> {
        match self.page {
            Page::Home => vec![
                "Pair headphones",
                "Saved headphones",
                "Audio output",
                "Diagnostics",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            Page::Pair | Page::Saved => {
                let mut rows = vec![if self.page == Page::Pair {
                    "Scan for headphones"
                } else {
                    "Refresh saved headphones"
                }
                .into()];
                rows.extend(self.device_indices().iter().map(|i| {
                    let d = &self.devices[*i];
                    format!(
                        "{}{}",
                        d.name,
                        if d.connected { " · Connected" } else { "" }
                    )
                }));
                rows
            }
            Page::Device(_) => vec!["Connect headphones", "Use speaker", "Forget headphones"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            Page::Forget(_) => vec!["Cancel".into(), "Forget headphones".into()],
            Page::Output => vec!["Use speaker".into(), "Choose saved headphones".into()],
            Page::Diagnostics => vec!["Save diagnostic report".into()],
        }
    }

    pub fn input(&mut self, action: Action) -> bool {
        if action == Action::QuickMenu {
            return true;
        }
        if action == Action::GbaDown(Btn::B) {
            if let Some(page) = self.stack.pop() {
                self.page = page;
                self.selected = 0;
            } else {
                return true;
            }
            return false;
        }
        if self.pending.is_some() {
            return false;
        }
        let count = self.rows().len();
        match action {
            Action::GbaDown(Btn::Up) => self.selected = (self.selected + count - 1) % count,
            Action::GbaDown(Btn::Down) => self.selected = (self.selected + 1) % count,
            Action::GbaDown(Btn::A) => match self.page.clone() {
                Page::Home => self.open(match self.selected {
                    0 => Page::Pair,
                    1 => Page::Saved,
                    2 => Page::Output,
                    _ => Page::Diagnostics,
                }),
                Page::Pair | Page::Saved => {
                    if self.selected == 0 {
                        self.ask(
                            if self.page == Page::Pair {
                                "scan"
                            } else {
                                "list"
                            },
                            None,
                        );
                    } else if let Some(i) = self.device_indices().get(self.selected - 1) {
                        self.open(Page::Device(*i));
                    }
                }
                Page::Device(i) => {
                    if let Some(d) = self.devices.get(i) {
                        let address = d.address.clone();
                        match self.selected {
                            0 => self.ask("connect", Some(address)),
                            1 => self.ask("speaker", None),
                            _ => self.open(Page::Forget(i)),
                        }
                    }
                }
                Page::Forget(i) => {
                    if self.selected == 1 {
                        let address = self.devices.get(i).map(|d| d.address.clone());
                        self.page = Page::Saved;
                        self.stack.clear();
                        self.stack.push(Page::Home);
                        self.selected = 0;
                        self.ask("forget", address);
                    } else {
                        self.page = Page::Device(i);
                        self.stack.pop();
                        self.selected = 0;
                    }
                }
                Page::Output => {
                    if self.selected == 0 {
                        self.ask("speaker", None);
                    } else {
                        self.open(Page::Saved);
                    }
                }
                Page::Diagnostics => self.ask("diagnose", None),
            },
            _ => {}
        }
        false
    }

    pub fn lines(&self) -> Vec<String> {
        let title = match self.page {
            Page::Home => "Bluetooth audio",
            Page::Pair => "Pair headphones",
            Page::Saved => "Saved headphones",
            Page::Output => "Audio output",
            Page::Diagnostics => "Diagnostics",
            Page::Forget(_) => "Forget headphones?",
            Page::Device(i) => self
                .devices
                .get(i)
                .map(|d| d.name.as_str())
                .unwrap_or("Headphones"),
        };
        let message = if self.status.is_empty() {
            match self.page {
                Page::Pair => "Put headphones in pairing mode, then scan.",
                Page::Diagnostics => "Reports: Bluetooth/latest-report.txt",
                Page::Forget(_) => "You will need to pair this device again.",
                _ => "",
            }
            .to_owned()
        } else {
            self.status.clone()
        };
        let mut lines = vec![title.to_owned(), message];
        if self.status.starts_with("Error:") {
            lines.push("Log: Bluetooth/latest-error.txt".into());
        } else {
            lines.push(String::new());
        }
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
        lines.push(
            if self.pending.is_some() {
                "B Back · Please wait"
            } else {
                "B Back · A Select"
            }
            .into(),
        );
        lines
    }
}

#[cfg(feature = "device")]
fn run(root: PathBuf, verb: &str, address: Option<String>) -> ResultMessage {
    let mut command = std::process::Command::new("timeout");
    command
        .arg("180")
        .arg("/bin/sh")
        .arg(root.join("System/bluetooth.sh"))
        .arg(verb)
        .env("SLOT_ROOT", &root);
    if let Some(address) = address {
        command.arg(address);
    }
    let result = command.output();
    let fallback = match &result {
        Ok(o)
            if !o.status.success()
                && !String::from_utf8_lossy(&o.stderr)
                    .contains("REPORT Bluetooth/latest-error.txt") =>
        {
            Some(format!(
                "Operation: {verb}\nExit: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
                o.status,
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            ))
        }
        Err(e) => Some(format!(
            "Operation: {verb}\nCould not start Bluetooth helper: {e}"
        )),
        _ => None,
    };
    if let Some(mut report) = fallback {
        if let Ok(log) = std::fs::read_to_string(root.join("slot.log")) {
            report.push_str("\n--- Application log ---\n");
            let lines: Vec<_> = log.lines().collect();
            report.push_str(&lines[lines.len().saturating_sub(120)..].join("\n"));
        }
        let _ = std::fs::create_dir_all(root.join("Bluetooth"));
        let _ = std::fs::write(root.join("Bluetooth/latest-error.txt"), report);
    }
    match result {
        Ok(output) if output.status.success() => ResultMessage {
            devices: parse_devices(&String::from_utf8_lossy(&output.stdout)),
            status: match verb {
                "connect" => "Headphones connected",
                "speaker" => "Speaker selected",
                "forget" => "Headphones forgotten",
                "diagnose" => "Saved Bluetooth/latest-report.txt",
                "scan" => "Scan complete. Select your headphones.",
                _ => "Select a menu to continue",
            }
            .into(),
        },
        Ok(output) => ResultMessage {
            devices: Vec::new(),
            status: format!(
                "Error: {}",
                String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .rfind(|s| !s.starts_with("REPORT ") && !s.is_empty())
                    .unwrap_or("Bluetooth failed or timed out")
            ),
        },
        Err(error) => ResultMessage {
            devices: Vec::new(),
            status: format!("Error: {error}"),
        },
    }
}

#[cfg(not(feature = "device"))]
fn run(_root: PathBuf, _verb: &str, _address: Option<String>) -> ResultMessage {
    ResultMessage {
        devices: Vec::new(),
        status: "Error: Bluetooth requires the handheld".into(),
    }
}

pub fn screen_face(lines: &[String]) -> UndoFace {
    let bg = slot_ui::opening().map(|c| (c * 255.0) as u8);
    let selected = slot_ui::edge().map(|c| (c * 255.0) as u8);
    let mut rgba = vec![0u8; (OUT_W * OUT_H * 4) as usize];
    for p in rgba.chunks_exact_mut(4) {
        p.copy_from_slice(&bg);
    }
    let Some(font) = slot_ui::text::label_font() else {
        return UndoFace {
            rgba,
            w: OUT_W,
            h: OUT_H,
        };
    };
    let label_w = OUT_W - 64;
    for (i, line) in lines.iter().enumerate() {
        let (label, top, h, px, max_lines, highlight) = match i {
            0 => (line.as_str(), 16, 46, 30.0, 1, false),
            1 => (line.as_str(), 66, 82, 22.0, 3, false),
            2 => (line.as_str(), 146, 30, 20.0, 1, false),
            _ if i == lines.len() - 1 => (line.as_str(), 427, 38, 22.0, 1, false),
            _ => (
                line.strip_prefix("SELECTED ")
                    .or_else(|| line.strip_prefix("ROW "))
                    .unwrap_or(line),
                184 + (i as u32 - 3) * 52,
                44,
                30.0,
                1,
                line.starts_with("SELECTED "),
            ),
        };
        if highlight {
            for y in top..top + h {
                for x in 0..OUT_W {
                    let o = ((y * OUT_W + x) * 4) as usize;
                    rgba[o..o + 4].copy_from_slice(&selected);
                }
            }
        }
        let layout = slot_ui::text::fit(font, label, label_w as f32, max_lines, px, 18.0);
        let row = i >= 3 && i != lines.len() - 1;
        let face_w = if row {
            layout
                .lines
                .first()
                .map(|l| {
                    (slot_ui::text::line_width(font, l, layout.px, layout.tracking).ceil() as u32
                        + 2)
                    .min(label_w)
                })
                .unwrap_or(label_w)
        } else {
            label_w
        };
        let mut face = vec![0u8; (face_w * h * 4) as usize];
        slot_ui::text::draw_centred(&mut face, face_w, h, &layout, [246, 244, 239]);
        for y in 0..h {
            for x in 0..face_w {
                let src = ((y * face_w + x) * 4) as usize;
                let dst = (((top + y) * OUT_W + x + 32) * 4) as usize;
                let alpha = u32::from(face[src + 3]);
                for c in 0..3 {
                    rgba[dst + c] = ((u32::from(face[src + c]) * alpha
                        + u32::from(rgba[dst + c]) * (255 - alpha))
                        / 255) as u8;
                }
            }
        }
    }
    UndoFace {
        rgba,
        w: OUT_W,
        h: OUT_H,
    }
}

#[cfg(test)]
mod menu_tests {
    use super::*;
    fn menu() -> Manager {
        Manager {
            root: PathBuf::new(),
            page: Page::Home,
            stack: Vec::new(),
            devices: vec![Headphones {
                address: "AA:BB:CC:DD:EE:FF".into(),
                name: "WF-C510".into(),
                paired: true,
                connected: false,
            }],
            selected: 0,
            status: String::new(),
            pending: None,
            face: None,
        }
    }
    #[test]
    fn pairing_is_a_separate_page_and_back_restores_the_parent() {
        let mut m = menu();
        assert!(!m.input(Action::GbaDown(Btn::A)));
        assert_eq!(m.page, Page::Pair);
        m.input(Action::GbaDown(Btn::Down));
        m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.page, Page::Device(0));
        m.input(Action::GbaDown(Btn::B));
        assert_eq!(m.page, Page::Pair);
        m.input(Action::GbaDown(Btn::B));
        assert_eq!(m.page, Page::Home);
        assert!(m.input(Action::GbaDown(Btn::B)));
    }
    #[test]
    fn forgetting_opens_confirmation_with_cancel_selected() {
        let mut m = menu();
        m.open(Page::Device(0));
        m.selected = 2;
        m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.page, Page::Forget(0));
        assert_eq!(m.selected, 0);
        m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.page, Page::Device(0));
        assert!(m.pending.is_none());
    }
}
