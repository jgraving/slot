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

pub fn valid_address(address: &str) -> bool {
    let parts: Vec<_> = address.split(':').collect();
    parts.len() == 6
        && parts
            .iter()
            .all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn bluetooth_enabled(root: &std::path::Path) -> bool {
    !root.join("Config/bluetooth-off").exists()
        && (root.join("Config/bluetooth-enabled").exists()
            || root.join("Config/bluetooth-headphones").exists())
}

fn startup_verb(root: &std::path::Path) -> &'static str {
    if !bluetooth_enabled(root) {
        "off"
    } else if root.join("Config/bluetooth-headphones").exists() {
        "restore"
    } else {
        "on"
    }
}

struct ResultMessage {
    // None means this operation did not return an authoritative device list.
    devices: Option<Vec<Headphones>>,
    status: String,
    output: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Home,
    Pair,
}

pub struct Manager {
    root: PathBuf,
    pub page: Page,
    stack: Vec<Page>,
    pub devices: Vec<Headphones>,
    pub usb_devices: Vec<crate::usb_audio::Device>,
    pub selected: usize,
    pub status: String,
    output: String,
    pending: Option<Receiver<ResultMessage>>,
    pub face: Option<TexId>,
    pub progress_face: Option<TexId>,
    pub wired: bool,
    pub bluetooth_available: bool,
    pub disconnect_volume: u8,
    pub disconnect_notice: bool,
    pub route_choice: Option<String>,
    choosing_route: bool,
    forget_confirmation: Option<String>,
}

impl Manager {
    pub fn new(root: PathBuf, bluetooth_available: bool) -> Self {
        let disconnect_volume = disconnect_volume(&root);
        let mut manager = Self {
            disconnect_volume,
            bluetooth_available,
            root,
            page: Page::Home,
            stack: Vec::new(),
            devices: Vec::new(),
            usb_devices: Vec::new(),
            selected: 0,
            status: String::new(),
            output: "speaker".into(),
            pending: None,
            face: None,
            progress_face: None,
            wired: false,
            disconnect_notice: false,
            route_choice: None,
            choosing_route: false,
            forget_confirmation: None,
        };
        if bluetooth_available {
            manager.ask("list", None);
        }
        manager
    }

    pub fn set_available(&mut self, available: bool) {
        if available && !self.bluetooth_available {
            self.bluetooth_available = true;
            self.ask("list", None);
        }
    }

    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    pub fn select_usb(&mut self, route: String) {
        self.ask("usb", Some(route));
    }

    // An automatic switch can start with the menu closed. Hand its worker to
    // the visible menu when it opens, so user actions cannot race that switch.
    pub fn adopt_usb_switch(&mut self, other: &mut Self) {
        self.pending = other.pending.take();
        self.choosing_route = other.choosing_route;
        self.status = std::mem::take(&mut other.status);
        self.route_choice = other.route_choice.take();
    }

    fn ask(&mut self, verb: &'static str, address: Option<String>) {
        if self.pending.is_some() {
            return;
        }
        self.forget_confirmation = None;
        self.choosing_route = matches!(verb, "speaker" | "connect" | "off" | "usb" | "local");
        let root = self.root.clone();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.status = match verb {
            "scan" => "Scanning…",
            "connect"
                if self
                    .devices
                    .iter()
                    .any(|d| Some(&d.address) == address.as_ref() && d.paired) =>
            {
                "Connecting..."
            }
            "connect" => "Pairing and connecting...",
            "forget" => "Forgetting device...",
            "speaker" | "local" | "usb" => "Switching output...",
            "off" => "Turning Bluetooth off...",
            "on" => "Turning Bluetooth on...",
            "diagnose" => "Saving report...",
            _ => "",
        }
        .into();
        let bluetooth_available = self.bluetooth_available;
        std::thread::spawn(move || {
            let result = if verb == "usb" || verb == "local" {
                run_usb(root, address, bluetooth_available)
            } else {
                run(root, verb, address)
            };
            let _ = tx.send(result);
        });
    }

    pub fn poll(&mut self) {
        let Some(rx) = &self.pending else { return };
        match rx.try_recv() {
            Ok(message) => {
                if !message.status.starts_with("Error:")
                    && !message.status.starts_with("Saved Bluetooth/")
                {
                    if let Some(devices) = message.devices {
                        self.devices = devices;
                    }
                }
                if let Some(output) = message.output {
                    if self.choosing_route && !message.status.starts_with("Error:") {
                        self.route_choice = Some(output.clone());
                        self.page = Page::Home;
                        self.stack.clear();
                        self.selected = self
                            .device_indices()
                            .iter()
                            .position(|i| self.devices[*i].address == output)
                            .map(|i| i + 1)
                            .or_else(|| {
                                self.usb_devices
                                    .iter()
                                    .position(|d| d.route == output)
                                    .map(|i| 1 + self.device_indices().len() + i)
                            })
                            .unwrap_or(0);
                    }
                    self.choosing_route = false;
                    self.output = output;
                }
                self.status = if message.status.starts_with("Error:")
                    || message.status.starts_with("Saved Bluetooth/")
                {
                    message.status
                } else {
                    String::new()
                };
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

    pub fn observe_output(&mut self, address: Option<&str>, connected: bool) {
        self.output = address.unwrap_or("speaker").to_owned();
        for device in &mut self.devices {
            if Some(device.address.as_str()) == address {
                device.connected = connected;
            }
        }
    }

    fn open(&mut self, page: Page) {
        self.forget_confirmation = None;
        self.stack.push(self.page.clone());
        self.page = page;
        self.selected = 0;
        self.status.clear();
    }

    fn disconnect_row(&self) -> usize {
        1 + self.device_indices().len()
            + self.usb_devices.len()
            + usize::from(self.bluetooth_available)
    }

    fn device_indices(&self) -> Vec<usize> {
        self.devices
            .iter()
            .enumerate()
            .filter(|(_, d)| self.bluetooth_available && (self.page == Page::Pair || d.paired))
            .map(|(i, _)| i)
            .collect()
    }

    fn active_device(&self) -> Option<usize> {
        self.devices
            .iter()
            .position(|d| d.address == self.output && d.connected)
    }

    pub fn rows(&self) -> Vec<String> {
        match self.page {
            Page::Home => {
                let local = if self.wired {
                    "WIRED_DEVICE Wired headphones"
                } else {
                    "SPEAKER_DEVICE Internal speaker"
                };
                let mut rows = vec![format!(
                    "{}{}",
                    if valid_address(&self.output) || crate::usb_audio::valid_route(&self.output) {
                        ""
                    } else {
                        "ACTIVE "
                    },
                    local
                )];
                rows.extend(self.device_indices().iter().map(|i| {
                    format!(
                        "{}DEVICE {}",
                        if self.active_device() == Some(*i) {
                            "ACTIVE "
                        } else {
                            ""
                        },
                        self.devices[*i].name
                    )
                }));
                rows.extend(self.usb_devices.iter().map(|d| {
                    format!(
                        "{}USB_DEVICE {}",
                        if d.route == self.output {
                            "ACTIVE "
                        } else {
                            ""
                        },
                        d.name
                    )
                }));
                if self.bluetooth_available {
                    rows.push("Pair audio device".into());
                }
                rows.push(format!("DISCONNECT {}%", self.disconnect_volume));
                if self.bluetooth_available {
                    rows.push(format!(
                        "BLUETOOTH {}",
                        if bluetooth_enabled(&self.root) {
                            "On"
                        } else {
                            "Off"
                        }
                    ));
                }
                rows
            }
            Page::Pair => {
                let mut rows = vec!["Scan for devices".into()];
                rows.extend(
                    self.device_indices()
                        .iter()
                        .map(|i| format!("DEVICE {}", self.devices[*i].name)),
                );
                rows
            }
        }
    }

    pub fn input(&mut self, action: Action) -> bool {
        if action == Action::GbaDown(Btn::B) && self.forget_confirmation.take().is_some() {
            return false;
        }
        if !matches!(action, Action::GbaDown(Btn::Y))
            && matches!(action, Action::GbaDown(_) | Action::QuickMenu)
        {
            self.forget_confirmation = None;
        }
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
            Action::GbaDown(Btn::Left | Btn::Right)
                if self.page == Page::Home && self.selected == self.disconnect_row() =>
            {
                self.change_disconnect_volume(action == Action::GbaDown(Btn::Right));
            }
            Action::GbaDown(Btn::Left | Btn::Right)
                if self.page == Page::Home
                    && self.bluetooth_available
                    && self.selected == self.disconnect_row() + 1 =>
            {
                self.toggle_bluetooth();
            }
            Action::GbaDown(Btn::X) if self.bluetooth_available => self.ask("diagnose", None),
            Action::GbaDown(Btn::Y) if self.page == Page::Home => {
                if let Some(i) = self
                    .selected
                    .checked_sub(1)
                    .and_then(|n| self.device_indices().get(n).copied())
                {
                    let address = self.devices[i].address.clone();
                    if self.forget_confirmation.as_ref() == Some(&address) {
                        self.ask("forget", Some(address));
                    } else {
                        self.forget_confirmation = Some(address);
                        self.status.clear();
                    }
                }
            }
            Action::GbaDown(Btn::A) => match self.page.clone() {
                Page::Home => match self.selected {
                    0 if self.bluetooth_available => self.ask("speaker", None),
                    0 => self.ask("local", None),
                    selected
                        if self.bluetooth_available
                            && selected
                                == 1 + self.device_indices().len() + self.usb_devices.len() =>
                    {
                        self.open(Page::Pair);
                        if !bluetooth_enabled(&self.root) {
                            self.ask("on", None);
                        }
                    }
                    selected if selected == self.disconnect_row() => {}
                    selected
                        if self.bluetooth_available && selected == self.disconnect_row() + 1 =>
                    {
                        self.toggle_bluetooth()
                    }
                    selected
                        if selected > self.device_indices().len()
                            && selected <= self.device_indices().len() + self.usb_devices.len() =>
                    {
                        let route = self.usb_devices[selected - 1 - self.device_indices().len()]
                            .route
                            .clone();
                        if self.output != route {
                            self.ask("usb", Some(route));
                        }
                    }
                    selected => {
                        if let Some(i) = self.device_indices().get(selected - 1) {
                            self.ask("connect", Some(self.devices[*i].address.clone()));
                        }
                    }
                },
                Page::Pair => {
                    if self.selected == 0 {
                        self.ask("scan", None);
                    } else if let Some(&i) = self.device_indices().get(self.selected - 1) {
                        self.ask("connect", Some(self.devices[i].address.clone()));
                    }
                }
            },
            _ => {}
        }
        false
    }

    fn toggle_bluetooth(&mut self) {
        self.ask(
            if bluetooth_enabled(&self.root) {
                "off"
            } else {
                "on"
            },
            None,
        );
    }

    fn change_disconnect_volume(&mut self, increase: bool) {
        let value = if increase {
            self.disconnect_volume.saturating_add(5).min(100)
        } else {
            self.disconnect_volume.saturating_sub(5)
        };
        match save_disconnect_volume(&self.root, value) {
            Ok(()) => {
                self.disconnect_volume = value;
                self.status.clear();
            }
            Err(_) => self.status = "Error: Could not save volume setting".into(),
        }
    }

    pub fn lines(&self) -> Vec<String> {
        let title = "";
        let message = if self.forget_confirmation.is_some() {
            "CONFIRM_FORGET".to_owned()
        } else if self.busy() {
            String::new()
        } else if self.status.is_empty() && self.disconnect_notice {
            "Device disconnected — choose output".into()
        } else {
            self.status.clone()
        };
        let compact = matches!(self.page, Page::Home | Page::Pair);
        let mut lines = vec![
            title.to_owned(),
            message,
            if compact {
                "COMPACT".into()
            } else if self.status.starts_with("Error:") {
                "Log: Bluetooth/latest-error.txt".into()
            } else {
                String::new()
            },
        ];
        let visible = if compact { 6 } else { 4 };
        let first = self.selected.saturating_sub(visible - 1);
        for (i, row) in self.rows().iter().enumerate().skip(first).take(visible) {
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
            if self.page == Page::Home
                && (self.selected == self.disconnect_row()
                    || (self.bluetooth_available && self.selected == self.disconnect_row() + 1))
            {
                "B Back · Change"
            } else if self.disconnect_notice && self.page == Page::Home {
                "A Select"
            } else if self.page == Page::Home
                && self.selected >= 1
                && self.selected < 1 + self.device_indices().len()
            {
                "B Back · A Connect · Y Forget"
            } else if self.page == Page::Pair && self.selected > 0 {
                "B Back · A Connect"
            } else {
                "B Back · A Select"
            }
            .into(),
        );
        lines
    }
}

fn run_usb(root: PathBuf, route: Option<String>, bluetooth_available: bool) -> ResultMessage {
    let error = |s: &str| ResultMessage {
        devices: None,
        output: None,
        status: format!("Error: {s}"),
    };
    if let Some(route) = &route {
        if !crate::usb_audio::devices()
            .iter()
            .any(|d| d.route == *route)
        {
            return error("USB audio device is no longer connected");
        }
    }
    #[cfg(feature = "device")]
    if let Some(route) = &route {
        // One second of digital silence verifies playback/resampling without
        // changing mixer gain or selecting this output before it can open.
        let ready = std::process::Command::new("timeout")
            .args([
                "3",
                "aplay",
                "-q",
                "-D",
                route,
                "-t",
                "raw",
                "-f",
                "S16_LE",
                "-r",
                "48000",
                "-c",
                "2",
                "-d",
                "1",
                "/dev/zero",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if !ready {
            return error("Could not open USB audio playback");
        }
    }
    let mut result = if bluetooth_available || root.join("Config/bluetooth-headphones").exists() {
        if route.is_some() {
            run(root, "usb", route.clone())
        } else {
            run(root, "speaker", None)
        }
    } else {
        ResultMessage {
            devices: None,
            output: None,
            status: String::new(),
        }
    };
    if result.status.starts_with("Error:") {
        return result;
    }
    if let Some(route) = &route {
        if !crate::usb_audio::devices()
            .iter()
            .any(|d| d.route == *route)
        {
            return error("USB audio device was unplugged");
        }
    }
    if crate::usb_audio::write_route(route.as_deref().unwrap_or("")).is_err() {
        return error("Could not select audio output");
    }
    result.output = Some(route.unwrap_or_else(|| "speaker".into()));
    result.status.clear();
    result
}

pub fn fallback_to_local(root: PathBuf, address: String) {
    std::thread::spawn(move || {
        let result = run(root, "fallback", Some(address));
        if result.status.starts_with("Error:") {
            eprintln!("slot: {}", result.status);
        }
    });
}

pub fn disconnect_volume(root: &std::path::Path) -> u8 {
    std::fs::read_to_string(root.join("Config/sound.conf"))
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("disconnect_volume=")?.parse::<u8>().ok())
        })
        .filter(|v| *v <= 100)
        .unwrap_or(0)
}

fn save_disconnect_volume(root: &std::path::Path, value: u8) -> std::io::Result<()> {
    let config = root.join("Config");
    std::fs::create_dir_all(&config)?;
    let temp = config.join("sound.conf.tmp");
    std::fs::write(&temp, format!("disconnect_volume={}\n", value.min(100)))?;
    std::fs::rename(temp, config.join("sound.conf"))
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
            if !matches!(verb, "restore" | "reconnect")
                && !o.status.success()
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
        Err(e) if !matches!(verb, "restore" | "reconnect") => Some(format!(
            "Operation: {verb}\nCould not start Bluetooth helper: {e}"
        )),
        _ => None,
    };
    if let Some(mut report) = fallback {
        if let Ok(mut file) = std::fs::File::open(root.join("Logs/slot.log")) {
            use std::io::{Read, Seek, SeekFrom};
            let size = file.metadata().map_or(0, |m| m.len());
            let _ = file.seek(SeekFrom::Start(size.saturating_sub(65536)));
            let mut bytes = Vec::new();
            let _ = file.take(65536).read_to_end(&mut bytes);
            report.push_str("\n--- Application log ---\n");
            report.push_str(&String::from_utf8_lossy(&bytes));
        }
        let _ = std::fs::create_dir_all(root.join("Bluetooth"));
        let _ = std::fs::write(
            root.join("Bluetooth/latest-error.txt"),
            &report.as_bytes()[..report.len().min(131072)],
        );
    }
    match result {
        Ok(output) if output.status.success() => ResultMessage {
            devices: String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|l| l == "DEVICES")
                .then(|| parse_devices(&String::from_utf8_lossy(&output.stdout))),
            output: String::from_utf8_lossy(&output.stdout)
                .lines()
                .find_map(|line| line.strip_prefix("OUTPUT ").map(str::to_owned)),
            status: match verb {
                "connect" => "Device connected",
                "speaker" => "Speaker selected",
                "off" => "Bluetooth off",
                "on" => "Bluetooth on",
                "forget" => "Device forgotten",
                "diagnose" => "Saved Bluetooth/latest-report.txt",
                "scan" => "Scan complete.",
                _ => "Select a menu to continue",
            }
            .into(),
        },
        Ok(output) => ResultMessage {
            devices: None,
            output: None,
            status: format!(
                "Error: {}",
                String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .rfind(|s| !s.starts_with("REPORT ") && !s.is_empty())
                    .unwrap_or("Bluetooth failed or timed out")
            ),
        },
        Err(error) => ResultMessage {
            devices: None,
            output: None,
            status: format!("Error: {error}"),
        },
    }
}

#[cfg(not(feature = "device"))]
fn run(_root: PathBuf, _verb: &str, _address: Option<String>) -> ResultMessage {
    ResultMessage {
        devices: None,
        output: None,
        status: "Error: Bluetooth requires the handheld".into(),
    }
}

pub fn screen_face(lines: &[String]) -> UndoFace {
    screen_face_with_ink(lines, slot_ui::footer_ink(false))
}

pub fn screen_face_with_ink(lines: &[String], footer_ink: [u8; 3]) -> UndoFace {
    let mut rgba = vec![0u8; (OUT_W * OUT_H * 4) as usize];
    let Some(font) = slot_ui::text::label_font() else {
        return UndoFace {
            rgba,
            w: OUT_W,
            h: OUT_H,
        };
    };
    let label_w = OUT_W - 96;
    let compact = lines.get(2).is_some_and(|line| line == "COMPACT");
    let clean = lines.iter().take(3).all(String::is_empty);
    let row_count = lines.len().saturating_sub(4) as u32;
    // Reserve six row positions so status text and new scan results cannot
    // recenter the menu or change the visible window.
    let row_top = if compact {
        88
    } else if clean {
        (OUT_H - row_count * 52) / 2 + 4
    } else {
        184
    };
    for (i, line) in lines.iter().enumerate() {
        if compact && (i == 0 || i == 2) {
            continue;
        }
        let (label, top, h, px, max_lines, highlight) = match i {
            1 if compact => (line.as_str(), 12, 64, 20.0, 2, false),
            0 => (line.as_str(), 16, 46, 30.0, 1, false),
            1 => (line.as_str(), 66, 82, 22.0, 3, false),
            2 => (line.as_str(), 146, 30, 20.0, 1, false),
            _ if i == lines.len() - 1 => (line.as_str(), 427, 38, 22.0, 1, false),
            _ => (
                line.strip_prefix("SELECTED ")
                    .or_else(|| line.strip_prefix("ROW "))
                    .unwrap_or(line),
                row_top + (i as u32 - 3) * 52 + 4,
                40u32,
                30.0,
                1,
                line.starts_with("SELECTED "),
            ),
        };
        let row = i >= 3 && i != lines.len() - 1;
        let ink = if row && !highlight {
            [0x9a, 0x9a, 0xa4]
        } else {
            [246, 244, 239]
        };
        if highlight {
            let pointer = slot_ui::quick_pointer_face();
            blit(&mut rgba, &pointer, 18, top.saturating_sub(4));
        }
        if i == lines.len() - 1 {
            let faces = slot_ui::sound_button_legends(footer_ink);
            let left: Vec<_> = [line.contains("B Back").then_some(0)]
                .into_iter()
                .flatten()
                .collect();
            let right: Vec<_> = [
                line.contains("Y Forget").then_some(2),
                Some(if line.contains("Change") {
                    4
                } else if line.contains("A Connect") {
                    1
                } else {
                    3
                }),
            ]
            .into_iter()
            .flatten()
            .collect();
            for (indices, right) in [(&left, false), (&right, true)] {
                let seen = |i: usize| faces[i].w.saturating_sub(slot_ui::HINT_EDGE);
                let gap = 16;
                let total = indices.iter().map(|&i| seen(i)).sum::<u32>()
                    + gap * indices.len().saturating_sub(1) as u32;
                let mut x = if right {
                    OUT_W.saturating_sub(total)
                } else {
                    0
                };
                for &i in indices {
                    blit(&mut rgba, &faces[i], x, OUT_H - slot_ui::MOUTH_H as u32);
                    x += seen(i) + gap;
                }
            }
            continue;
        }
        if label == "CONFIRM_FORGET" {
            let hint = slot_ui::sound_forget_confirmation();
            blit(&mut rgba, &hint, (OUT_W - hint.w) / 2, 12);
            continue;
        }
        if let Some((name, value)) = label
            .strip_prefix("DISCONNECT ")
            .map(|v| ("Volume on disconnect", v))
            .or_else(|| label.strip_prefix("BLUETOOTH ").map(|v| ("Bluetooth", v)))
        {
            let name = slot_ui::quick_value_face(name, highlight);
            blit(&mut rgba, &name, 64 - slot_ui::MENU_PAD, top);
            let value = slot_ui::quick_value_face(value, highlight);
            if highlight {
                let left = slot_ui::quick_caret_face(false);
                let right = slot_ui::quick_caret_face(true);
                let rx = OUT_W - 32 - right.w;
                let vx = rx - 14 + 18 - value.w;
                blit(&mut rgba, &right, rx, top);
                blit(&mut rgba, &value, vx, top);
                blit(&mut rgba, &left, vx + 18 - 14 - left.w, top);
            } else {
                // Align visible ink, not the font's trailing side bearing.
                let ink_right = value
                    .rgba
                    .chunks_exact(4)
                    .enumerate()
                    .filter(|(_, p)| p[3] > 127)
                    .map(|(i, _)| i as u32 % value.w)
                    .max()
                    .unwrap_or(0);
                blit(&mut rgba, &value, OUT_W - 33 - ink_right, top);
            }
            continue;
        }
        let active = label.starts_with("ACTIVE ");
        let label = label.strip_prefix("ACTIVE ").unwrap_or(label);
        let usb = label.starts_with("USB_DEVICE ");
        let wired = label.starts_with("WIRED_DEVICE ");
        let speaker = label.starts_with("SPEAKER_DEVICE ");
        let device = usb || wired || speaker || label.starts_with("DEVICE ");
        let label = label
            .strip_prefix("DEVICE ")
            .or_else(|| label.strip_prefix("USB_DEVICE "))
            .or_else(|| label.strip_prefix("WIRED_DEVICE "))
            .or_else(|| label.strip_prefix("SPEAKER_DEVICE "))
            .unwrap_or(label);
        let inset = if !row {
            48
        } else if device {
            100
        } else {
            64
        };
        let label_w = label_w - if device { 36 } else { 0 };
        if device {
            let icon = slot_ui::sound_device_icon(
                if speaker {
                    slot_ui::Icon::Volume
                } else if usb || wired {
                    slot_ui::Icon::Headphones
                } else {
                    slot_ui::Icon::Bluetooth
                },
                ink,
            );
            let center = slot_ui::quick_text_centre();
            blit(
                &mut rgba,
                &icon,
                64 + 28u32.saturating_sub(icon.w) / 2,
                top + (center - icon.h as f32 / 2.).round().max(0.) as u32,
            );
        }
        let layout = slot_ui::text::fit(
            font,
            label,
            (label_w - if active { 44 } else { 0 }) as f32,
            max_lines,
            px,
            18.0,
        );
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
        if active {
            draw_check(&mut rgba, OUT_W - 32 - 28, top + h / 2, ink);
        }
        let mut face = vec![0u8; (face_w * h * 4) as usize];
        slot_ui::text::draw_centred(&mut face, face_w, h, &layout, ink);
        blit(
            &mut rgba,
            &UndoFace {
                rgba: face,
                w: face_w,
                h,
            },
            inset,
            top,
        );
    }
    UndoFace {
        rgba,
        w: OUT_W,
        h: OUT_H,
    }
}

fn blit(dst: &mut [u8], face: &UndoFace, left: u32, top: u32) {
    for y in 0..face.h.min(OUT_H.saturating_sub(top)) {
        for x in 0..face.w.min(OUT_W.saturating_sub(left)) {
            let src = ((y * face.w + x) * 4) as usize;
            let dest = (((top + y) * OUT_W + left + x) * 4) as usize;
            let a = u32::from(face.rgba[src + 3]);
            let behind = u32::from(dst[dest + 3]) * (255 - a) / 255;
            let alpha = a + behind;
            if alpha == 0 {
                continue;
            }
            for c in 0..3 {
                dst[dest + c] = ((u32::from(face.rgba[src + c]) * a
                    + u32::from(dst[dest + c]) * behind)
                    / alpha) as u8;
            }
            dst[dest + 3] = alpha as u8;
        }
    }
}

fn draw_check(dst: &mut [u8], x: u32, y: u32, ink: [u8; 3]) {
    // Draw the mark directly: the original slot label font has no check glyph.
    for i in 0..10 {
        for thickness in 0..3 {
            let offset = (((y - 3 + i + thickness) * OUT_W + x + i) * 4) as usize;
            dst[offset..offset + 4].copy_from_slice(&[ink[0], ink[1], ink[2], 255]);
        }
    }
    for i in 0..19 {
        for thickness in 0..3 {
            let offset = (((y + 6 - i + thickness) * OUT_W + x + 9 + i) * 4) as usize;
            dst[offset..offset + 4].copy_from_slice(&[ink[0], ink[1], ink[2], 255]);
        }
    }
}

#[cfg(test)]
mod menu_tests {
    use super::*;
    fn menu() -> Manager {
        Manager {
            root: PathBuf::new(),
            bluetooth_available: true,
            disconnect_volume: 0,
            page: Page::Home,
            stack: Vec::new(),
            usb_devices: Vec::new(),
            devices: vec![Headphones {
                address: "AA:BB:CC:DD:EE:FF".into(),
                name: "WF-C510".into(),
                paired: true,
                connected: false,
            }],
            selected: 0,
            status: String::new(),
            output: "speaker".into(),
            pending: None,
            face: None,
            progress_face: None,
            wired: false,
            disconnect_notice: false,
            route_choice: None,
            choosing_route: false,
            forget_confirmation: None,
        }
    }
    #[test]
    fn busy_work_uses_progress_without_hiding_failures_or_moving_rows() {
        let mut m = menu();
        let idle = m.lines();
        let (tx, rx) = mpsc::channel();
        m.pending = Some(rx);
        for status in [
            "Scanning…",
            "Connecting...",
            "Turning Bluetooth on...",
            "Saving report...",
        ] {
            m.status = status.into();
            let busy = m.lines();
            assert!(busy[1].is_empty());
            assert_eq!(&busy[3..], &idle[3..]);
        }
        tx.send(ResultMessage {
            devices: None,
            output: None,
            status: "Error: Bluetooth could not connect".into(),
        })
        .unwrap();
        m.poll();
        assert!(!m.busy());
        assert!(m.lines()[1].starts_with("Error:"));
        assert_eq!(&m.lines()[3..], &idle[3..]);
    }

    #[test]
    fn opening_sound_adopts_an_automatic_usb_switch() {
        let mut worker = menu();
        let (tx, rx) = mpsc::channel();
        worker.pending = Some(rx);
        worker.choosing_route = true;
        worker.status = "Switching output...".into();
        let mut visible = menu();
        visible.adopt_usb_switch(&mut worker);
        assert!(visible.busy());
        assert!(!worker.busy());
        tx.send(ResultMessage {
            devices: None,
            output: Some("plughw:CARD=Audio,DEV=0".into()),
            status: String::new(),
        })
        .unwrap();
        visible.poll();
        assert_eq!(
            visible.route_choice.as_deref(),
            Some("plughw:CARD=Audio,DEV=0")
        );
        assert!(!visible.busy());
        assert_eq!(
            visible.devices.len(),
            1,
            "an output-only reply must preserve paired devices"
        );
    }

    #[test]
    fn an_authoritative_empty_list_removes_forgotten_devices() {
        let mut m = menu();
        let (tx, rx) = mpsc::channel();
        m.pending = Some(rx);
        tx.send(ResultMessage {
            devices: Some(vec![]),
            output: None,
            status: "Device forgotten".into(),
        })
        .unwrap();
        m.poll();
        assert!(m.devices.is_empty());
    }

    #[test]
    fn forget_confirmation_cancels_on_navigation_and_x_remains_hidden() {
        let mut m = menu();
        m.selected = 1;
        assert!(m.lines().last().unwrap().contains("A Connect · Y Forget"));
        assert!(!m.lines().last().unwrap().contains("X"));
        m.input(Action::GbaDown(Btn::Y));
        m.input(Action::GbaDown(Btn::Down));
        assert!(m.forget_confirmation.is_none());
        m.input(Action::GbaDown(Btn::Up));
        m.input(Action::GbaDown(Btn::Y));
        assert!(m.pending.is_none(), "moving away disarms the first press");
        m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.status, "Connecting...");
        assert!(m.forget_confirmation.is_none());
        let mut m = menu();
        m.input(Action::GbaDown(Btn::X));
        assert_eq!(m.status, "Saving report...");
    }

    #[test]
    fn usb_outputs_appear_without_bluetooth_and_show_selected_output() {
        let mut m = menu();
        m.bluetooth_available = false;
        m.usb_devices = vec![crate::usb_audio::Device {
            route: "plughw:CARD=Audio,DEV=0".into(),
            name: "AB13X Audio".into(),
        }];
        assert_eq!(
            m.rows(),
            vec![
                "ACTIVE SPEAKER_DEVICE Internal speaker",
                "USB_DEVICE AB13X Audio",
                "DISCONNECT 0%"
            ]
        );
        m.observe_output(Some("plughw:CARD=Audio,DEV=0"), true);
        assert_eq!(m.rows()[0], "SPEAKER_DEVICE Internal speaker");
        assert!(m.rows()[1].starts_with("ACTIVE USB"));
        assert_eq!(m.disconnect_row(), 2);
    }

    #[test]
    fn bluetooth_defaults_off_and_remembers_explicit_enable_or_existing_headphones() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("Config")).unwrap();
        assert_eq!(startup_verb(root.path()), "off");
        std::fs::write(root.path().join("Config/bluetooth-enabled"), "").unwrap();
        assert_eq!(startup_verb(root.path()), "on");
        std::fs::remove_file(root.path().join("Config/bluetooth-enabled")).unwrap();
        std::fs::write(
            root.path().join("Config/bluetooth-headphones"),
            "AA:BB:CC:DD:EE:FF",
        )
        .unwrap();
        assert_eq!(startup_verb(root.path()), "restore");
        std::fs::write(root.path().join("Config/bluetooth-off"), "").unwrap();
        assert_eq!(startup_verb(root.path()), "off");
    }

    #[test]
    fn pair_entry_enables_bluetooth_and_toggle_has_change_hint() {
        let root = tempfile::tempdir().unwrap();
        let mut m = menu();
        m.root = root.path().to_owned();
        m.selected = m.disconnect_row() + 1;
        assert!(m.lines().last().unwrap().contains("Change"));
        m.selected = m.disconnect_row() - 1;
        m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.page, Page::Pair);
        assert!(m.busy());
        assert_eq!(m.status, "Turning Bluetooth on...");
    }

    #[test]
    fn off_is_available_only_with_bluetooth_and_reflects_the_saved_state() {
        let root = tempfile::tempdir().unwrap();
        let mut m = menu();
        m.root = root.path().to_owned();
        assert_eq!(m.rows().last().unwrap(), "BLUETOOTH Off");
        std::fs::create_dir(root.path().join("Config")).unwrap();
        std::fs::write(root.path().join("Config/bluetooth-off"), "").unwrap();
        assert_eq!(m.rows().last().unwrap(), "BLUETOOTH Off");
        m.bluetooth_available = false;
        assert!(!m.rows().iter().any(|r| r.starts_with("BLUETOOTH ")));
    }

    #[test]
    fn reconnect_is_slow_bounded_and_respects_off_and_manual_output() {
        assert_eq!(
            [0, 1, 2, 3, 100].map(|n| retry_delay(n).as_secs()),
            [5, 15, 30, 60, 60]
        );
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("System")).unwrap();
        std::fs::write(root.path().join("System/bluetooth.sh"), "").unwrap();
        std::fs::create_dir(root.path().join("Config")).unwrap();
        let mut retry = Reconnect::default();
        retry.checked = Some(std::time::Instant::now() - std::time::Duration::from_secs(100));
        retry.poll(root.path(), false);
        assert!(retry.pending.is_none(), "no selection means no reconnect");
        std::fs::write(
            root.path().join("Config/bluetooth-headphones"),
            "AA:BB:CC:DD:EE:FF",
        )
        .unwrap();
        std::fs::write(root.path().join("Config/bluetooth-off"), "").unwrap();
        retry.checked = Some(std::time::Instant::now() - std::time::Duration::from_secs(100));
        retry.poll(root.path(), false);
        assert!(retry.pending.is_none(), "off overrides a stale selection");
        retry.failures = 10;
        retry.poll(root.path(), true);
        assert_eq!(retry.failures, 0);
    }

    #[test]
    fn pairing_is_separate_and_back_restores_the_parent() {
        let mut m = menu();
        m.devices[0].paired = false;
        m.selected = 1;
        m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.page, Page::Pair);
        m.input(Action::GbaDown(Btn::B));
        assert_eq!(m.page, Page::Home);
        assert!(m.input(Action::GbaDown(Btn::B)));
    }
    #[test]
    fn saved_headphones_are_on_home_and_forget_requires_two_y_presses() {
        let mut m = menu();
        assert_eq!(
            m.rows(),
            vec![
                "ACTIVE SPEAKER_DEVICE Internal speaker",
                "DEVICE WF-C510",
                "Pair audio device",
                "DISCONNECT 0%",
                "BLUETOOTH Off"
            ]
        );
        assert!(m.lines()[0].is_empty());
        assert!(m.lines()[1].is_empty());
        let idle = screen_face(&m.lines());
        for status in ["Connecting...", "Scanning…", "Error: Connection failed"] {
            m.status = status.into();
            let busy = screen_face(&m.lines());
            assert_eq!(
                &idle.rgba[(88 * OUT_W * 4) as usize..],
                &busy.rgba[(88 * OUT_W * 4) as usize..],
                "status must not move or redraw the menu rows or hints"
            );
        }
        m.status.clear();
        m.selected = 1;
        m.input(Action::GbaDown(Btn::Y));
        assert_eq!(m.page, Page::Home);
        assert_eq!(m.selected, 1);
        assert!(m.forget_confirmation.is_some());
        assert!(m.lines()[1].contains("CONFIRM_FORGET"));
        assert!(m.pending.is_none());
        m.input(Action::GbaDown(Btn::B));
        assert_eq!(m.page, Page::Home);
        assert!(m.forget_confirmation.is_none());
        m.input(Action::GbaDown(Btn::Y));
        m.input(Action::GbaDown(Btn::Y));
        assert_eq!(m.page, Page::Home);
        assert!(m.pending.is_some());
        assert_eq!(m.status, "Forgetting device...");
    }
    #[test]
    fn active_route_not_connection_alone_controls_checkmark() {
        let mut m = menu();
        m.devices[0].connected = true;
        assert_eq!(m.rows()[0], "ACTIVE SPEAKER_DEVICE Internal speaker");
        m.output = m.devices[0].address.clone();
        assert_eq!(m.rows()[0], "SPEAKER_DEVICE Internal speaker");
        assert_eq!(m.rows()[1], "ACTIVE DEVICE WF-C510");
        m.devices[0].connected = false;
        assert_eq!(m.rows()[0], "SPEAKER_DEVICE Internal speaker");
    }
    #[test]
    #[cfg(feature = "device")]
    fn saved_headphones_in_scan_results_connect_directly() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("System")).unwrap();
        std::fs::write(
            root.path().join("System/bluetooth.sh"),
            "printf '%s %s' \"$1\" \"$2\" > \"$SLOT_ROOT/called\"\n",
        )
        .unwrap();
        let mut m = menu();
        m.root = root.path().into();
        m.open(Page::Pair);
        assert_eq!(m.rows(), vec!["Scan for devices", "DEVICE WF-C510"]);
        m.selected = 1;
        m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.page, Page::Pair);
        assert_eq!(m.status, "Connecting...");
        m.pending
            .take()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(root.path().join("called")).unwrap(),
            "connect AA:BB:CC:DD:EE:FF"
        );
    }

    #[test]
    fn connected_device_returns_to_sound_and_is_selected_and_checked() {
        let mut m = menu();
        m.open(Page::Pair);
        let (tx, rx) = mpsc::channel();
        m.pending = Some(rx);
        m.choosing_route = true;
        let mut devices = m.devices.clone();
        devices[0].connected = true;
        tx.send(ResultMessage {
            output: Some(devices[0].address.clone()),
            devices: Some(devices),
            status: "Device connected".into(),
        })
        .unwrap();
        m.poll();
        assert_eq!(m.page, Page::Home);
        assert!(m.stack.is_empty());
        assert_eq!(m.selected, 1);
        assert_eq!(m.rows()[1], "ACTIVE DEVICE WF-C510");
        if let Ok(path) = std::env::var("SLOT_SOUND_PREVIEW") {
            let face = screen_face(&m.lines());
            let file = std::fs::File::create(path).unwrap();
            let mut encoder = png::Encoder::new(file, face.w, face.h);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&face.rgba)
                .unwrap();
        }
        assert!(m.lines()[0].is_empty());
        assert!(m.lines()[1].is_empty());
        assert!(m.input(Action::GbaDown(Btn::B)));
    }

    #[test]
    fn failed_connection_stays_on_scan_and_preserves_output() {
        let mut m = menu();
        m.open(Page::Pair);
        let (tx, rx) = mpsc::channel();
        m.pending = Some(rx);
        m.choosing_route = true;
        tx.send(ResultMessage {
            output: None,
            devices: None,
            status: "Error: unavailable".into(),
        })
        .unwrap();
        m.poll();
        assert_eq!(m.page, Page::Pair);
        assert_eq!(m.output, "speaker");
        assert!(m.route_choice.is_none());
    }

    #[test]
    fn local_output_follows_jack_and_does_not_claim_bluetooth_output() {
        let mut m = menu();
        m.wired = true;
        assert_eq!(m.rows()[0], "ACTIVE WIRED_DEVICE Wired headphones");
        m.observe_output(Some("AA:BB:CC:DD:EE:FF"), true);
        assert_eq!(m.rows()[0], "WIRED_DEVICE Wired headphones");
        assert_eq!(m.rows()[1], "ACTIVE DEVICE WF-C510");
        m.wired = false;
        assert_eq!(m.rows()[0], "SPEAKER_DEVICE Internal speaker");
        m.observe_output(None, false);
        assert_eq!(m.rows()[0], "ACTIVE SPEAKER_DEVICE Internal speaker");
        m.open(Page::Pair);
        assert!(m.lines()[0].is_empty());
        assert!(m.lines()[1].is_empty());
        assert_eq!(m.rows()[0], "Scan for devices");
    }

    #[test]
    fn local_sound_settings_do_not_expose_bluetooth_when_unavailable() {
        let root = tempfile::tempdir().unwrap();
        let mut m = Manager::new(root.path().into(), false);
        assert_eq!(
            m.rows(),
            vec!["ACTIVE SPEAKER_DEVICE Internal speaker", "DISCONNECT 0%"]
        );
        assert!(m.pending.is_none());
        m.selected = 1;
        for _ in 0..25 {
            m.input(Action::GbaDown(Btn::Right));
        }
        assert_eq!(m.disconnect_volume, 100);
        assert_eq!(disconnect_volume(root.path()), 100);
        for _ in 0..25 {
            m.input(Action::GbaDown(Btn::Left));
        }
        assert_eq!(m.disconnect_volume, 0);
        assert_eq!(disconnect_volume(root.path()), 0);
        m.input(Action::GbaDown(Btn::Right));
        assert_eq!(Manager::new(root.path().into(), false).disconnect_volume, 5);
        std::fs::write(
            root.path().join("Config/sound.conf"),
            "disconnect_volume=999",
        )
        .unwrap();
        assert_eq!(disconnect_volume(root.path()), 0);
    }

    #[test]
    fn diagnostic_results_preserve_devices_and_route() {
        let mut m = menu();
        let (tx, rx) = mpsc::channel();
        m.pending = Some(rx);
        m.output = m.devices[0].address.clone();
        tx.send(ResultMessage {
            devices: None,
            output: None,
            status: "Saved Bluetooth/latest-report.txt".into(),
        })
        .unwrap();
        m.poll();
        assert_eq!(m.devices.len(), 1);
        assert_eq!(m.output, m.devices[0].address);
    }
}

/// A passive BlueZ signal listener. It never powers the adapter or starts discovery.
/// Signals only invalidate cached state; the helper still verifies the saved device.
#[derive(Default)]
pub struct Events {
    child: Option<std::process::Child>,
    incoming: Option<Receiver<()>>,
    attempted: Option<std::time::Instant>,
    selection: String,
    sampled: Option<std::time::Instant>,
    asleep: bool,
}

impl Events {
    pub fn poll(&mut self, root: &std::path::Path, awake: bool) -> (bool, bool) {
        if !awake {
            if !self.asleep {
                self.stop();
                self.selection.clear();
                self.sampled = None;
                self.attempted = None;
                // Cancel only our automatic helper, never a manual pairing.
                let lock = std::env::var_os("SLOT_BT_LOCK")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/run/slot-bluetooth.lock"));
                if std::fs::read_to_string(lock.join("verb"))
                    .is_ok_and(|v| matches!(v.trim(), "restore" | "reconnect"))
                {
                    let _ = std::fs::write(lock.join("cancel"), "");
                }
            }
            self.asleep = true;
            return (false, false);
        }
        self.asleep = false;
        if self.sampled.is_none_or(|t| t.elapsed().as_secs() >= 1) {
            self.sampled = Some(std::time::Instant::now());
            let selected = if !root.join("Config/bluetooth-off").exists() {
                std::fs::read_to_string(root.join("Config/bluetooth-headphones"))
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            } else {
                String::new()
            };
            if self.selection != selected {
                self.stop();
                self.selection = selected;
                self.attempted = None;
            }
        }
        if !valid_address(&self.selection) {
            return (false, false);
        }
        if self
            .child
            .as_mut()
            .is_some_and(|c| !matches!(c.try_wait(), Ok(None)))
        {
            self.stop();
        }
        if self.child.is_none() && self.attempted.is_none_or(|t| t.elapsed().as_secs() >= 60) {
            self.attempted = Some(std::time::Instant::now());
            self.start();
        }
        let changed = self
            .incoming
            .as_ref()
            .is_some_and(|rx| rx.try_recv().is_ok());
        (changed, self.child.is_some())
    }

    #[cfg(feature = "device")]
    fn start(&mut self) {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};
        // Accept any adapter, but only the selected device's connection events.
        let rule = "type='signal',sender='org.bluez',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',arg0='org.bluez.Device1'";
        let Ok(mut child) = Command::new("dbus-monitor")
            .args(["--system", rule])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            return;
        };
        let stdout = child.stdout.take().unwrap();
        let address = self.selection.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut selected = false;
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if connection_event(&line, &address, &mut selected) {
                    // Coalesce events; the authoritative property read follows.
                    let _ = tx.try_send(());
                }
            }
        });
        self.child = Some(child);
        self.incoming = Some(rx);
    }

    #[cfg(not(feature = "device"))]
    fn start(&mut self) {}

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.incoming = None;
    }
}
impl Drop for Events {
    fn drop(&mut self) {
        self.stop();
    }
}

fn connection_event(line: &str, address: &str, selected: &mut bool) -> bool {
    if line.starts_with("signal ") {
        let component = format!("/dev_{};", address.replace(':', "_"));
        *selected = line
            .to_ascii_uppercase()
            .contains(&component.to_ascii_uppercase());
        false
    } else {
        *selected && line.trim() == "string \"Connected\""
    }
}

/// Retry only the saved choice. No discovery, new pairing, or UI-thread commands.
#[derive(Default)]
pub struct Reconnect {
    checked: Option<std::time::Instant>,
    pending: Option<Receiver<()>>,
    failures: u32,
}

impl Reconnect {
    pub fn changed(&mut self) {
        self.checked = Some(std::time::Instant::now() - std::time::Duration::from_secs(60));
    }
    pub fn poll(&mut self, root: &std::path::Path, connected: bool) {
        if let Some(rx) = &self.pending {
            if !matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)) {
                self.pending = None;
            }
        }
        let now = std::time::Instant::now();
        if connected {
            self.failures = 0;
            self.checked = Some(now);
            return;
        }
        let delay = retry_delay(self.failures);
        let Some(checked) = self.checked else {
            self.checked = Some(now);
            return;
        };
        if self.pending.is_some() || now.duration_since(checked) < delay {
            return;
        }
        self.checked = Some(now);
        if root.join("Config/bluetooth-off").exists()
            || !root.join("Config/bluetooth-headphones").is_file()
            || !root.join("System/bluetooth.sh").is_file()
        {
            self.failures = 0;
            return;
        }
        self.failures = self.failures.saturating_add(1);
        let root = root.to_owned();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        std::thread::spawn(move || {
            let _ = run(root, "reconnect", None);
            let _ = tx.send(());
        });
    }
}

fn retry_delay(failures: u32) -> std::time::Duration {
    std::time::Duration::from_secs(match failures {
        0 => 5,
        1 => 15,
        2 => 30,
        _ => 60,
    })
}

/// Read-only connection checks; never starts the radio or changes audio routing.
#[derive(Default)]
pub struct Indicator {
    checked: Option<std::time::Instant>,
    pending: Option<Receiver<(String, Option<bool>)>>,
    output: String,
    active: bool,
}

impl Indicator {
    pub fn changed(&mut self) {
        self.checked = None;
        self.pending = None;
    }

    pub fn select(&mut self, address: Option<String>) {
        self.active = address.is_some();
        self.output = address.unwrap_or_default();
        self.pending = None;
        self.checked = Some(std::time::Instant::now());
    }

    pub fn address(&self) -> Option<String> {
        (!self.output.is_empty()).then(|| self.output.clone())
    }
    pub fn poll(&mut self, _watching: bool) -> bool {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok((address, connected)) => {
                    if self.output == address {
                        if let Some(connected) = connected {
                            self.active = connected;
                        }
                    }
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let path = std::env::var_os("SLOT_AUDIO_ROUTE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/run/slot-audio-device"));
        let route = std::fs::read_to_string(path).unwrap_or_default();
        let address = route_address(&route).unwrap_or_default().to_owned();
        if address != self.output {
            self.active = false;
            self.output = address.clone();
            self.checked = None;
        }
        if address.is_empty() {
            self.active = false;
            return false;
        }
        // Keep a short read-only fallback even with a listener: a missed signal
        // must not leave the game on a disconnected output for 30 seconds.
        if self
            .checked
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(2))
        {
            return self.active;
        }
        self.checked = Some(std::time::Instant::now());
        if self.pending.is_none() {
            let (tx, rx) = mpsc::channel();
            self.pending = Some(rx);
            std::thread::spawn(move || {
                #[cfg(feature = "device")]
                let connected = std::process::Command::new("timeout")
                    .args(["2", "bluetoothctl", "info", &address])
                    .output()
                    .ok()
                    .and_then(|output| {
                        if !output.status.success() {
                            return None;
                        }
                        let info = String::from_utf8_lossy(&output.stdout);
                        if connection_present(&info) {
                            Some(true)
                        } else if info.lines().any(|l| l.trim() == "Connected: no") {
                            Some(false)
                        } else {
                            None
                        }
                    });
                #[cfg(not(feature = "device"))]
                let connected = Some(false);
                let _ = tx.send((address, connected));
            });
        }
        self.active
    }
}

fn route_address(route: &str) -> Option<&str> {
    let address = route
        .trim()
        .strip_prefix("bluealsa:DEV=")?
        .strip_suffix(",PROFILE=a2dp")?;
    valid_address(address).then_some(address)
}

#[cfg(any(feature = "device", test))]
fn connection_present(info: &str) -> bool {
    info.lines().any(|line| line.trim() == "Connected: yes")
}

pub fn indicator_face() -> UndoFace {
    indicator_face_with_ink([246, 244, 239])
}

pub fn indicator_face_with_ink(ink: [u8; 3]) -> UndoFace {
    let mut face = UndoFace {
        rgba: vec![0; 20 * 20 * 4],
        w: 20,
        h: 20,
    };
    for ((x0, y0), (x1, y1)) in [
        ((9, 1), (9, 18)),
        ((9, 1), (16, 6)),
        ((16, 6), (3, 15)),
        ((9, 18), (16, 13)),
        ((16, 13), (3, 4)),
    ] {
        let steps = i32::abs(x1 - x0).max(i32::abs(y1 - y0));
        for step in 0..=steps {
            let x = x0 + (x1 - x0) * step / steps;
            let y = y0 + (y1 - y0) * step / steps;
            for dx in 0..2 {
                let offset = ((y * 20 + x + dx) * 4) as usize;
                face.rgba[offset..offset + 4].copy_from_slice(&[ink[0], ink[1], ink[2], 255]);
            }
        }
    }
    face
}

#[cfg(test)]
mod indicator_tests {
    use super::*;
    #[test]
    fn events_ignore_other_devices_and_unrelated_properties() {
        let address = "AA:BB:CC:DD:EE:FF";
        let mut selected = false;
        assert!(!connection_event(
            "signal path=/org/bluez/hci1/dev_AA_BB_CC_DD_EE_FF; interface=x",
            address,
            &mut selected
        ));
        assert!(!connection_event(
            "   string \"Alias\"",
            address,
            &mut selected
        ));
        assert!(connection_event(
            "   string \"Connected\"",
            address,
            &mut selected
        ));
        assert!(!connection_event(
            "signal path=/org/bluez/hci0/dev_00_11_22_33_44_55; interface=x",
            address,
            &mut selected
        ));
        assert!(!connection_event(
            "   string \"Connected\"",
            address,
            &mut selected
        ));
    }

    #[test]
    fn no_selection_off_and_sleep_do_not_start_a_listener() {
        let root = tempfile::tempdir().unwrap();
        let mut events = Events::default();
        assert_eq!(events.poll(root.path(), true), (false, false));
        std::fs::create_dir(root.path().join("Config")).unwrap();
        std::fs::write(
            root.path().join("Config/bluetooth-headphones"),
            "AA:BB:CC:DD:EE:FF",
        )
        .unwrap();
        std::fs::write(root.path().join("Config/bluetooth-off"), "").unwrap();
        events.sampled = None;
        assert_eq!(events.poll(root.path(), true), (false, false));
        assert_eq!(events.poll(root.path(), false), (false, false));
        assert!(events.child.is_none());
    }

    #[test]
    fn only_a_valid_selected_headphone_route_can_show_an_indicator() {
        assert_eq!(
            route_address("bluealsa:DEV=AA:BB:CC:DD:EE:FF,PROFILE=a2dp\n"),
            Some("AA:BB:CC:DD:EE:FF")
        );
        for route in [
            "",
            "speaker",
            "bluealsa:DEV=bad,PROFILE=a2dp",
            "bluealsa:DEV=AA:BB:CC:DD:EE:FF,PROFILE=sco",
        ] {
            assert_eq!(route_address(route), None);
        }
        assert!(connection_present("Name: WF-C510\n\tConnected: yes\n"));
        assert!(!connection_present("Connected: no\nAlias: Connected: yes"));
    }
}

/// Check the device asynchronously: unsupported hosts never expose the menu.
/// The helper owns BaseOS initialization; the UI has no model-name assumptions.
pub fn probe(root: &std::path::Path) -> Option<Receiver<bool>> {
    if !cfg!(feature = "device") || !root.join("System/bluetooth.sh").is_file() {
        return None;
    }
    let root = root.to_owned();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let supported = std::process::Command::new("timeout")
            .args(["25", "/bin/sh"])
            .arg(root.join("System/bluetooth.sh"))
            .arg("supported")
            .env("SLOT_ROOT", &root)
            .output()
            .is_ok_and(|o| o.status.success());
        let _ = tx.send(supported);
        if supported {
            let verb = startup_verb(&root);
            let result = run(root, verb, None);
            if result.status.starts_with("Error:") {
                eprintln!("slot: headphone reconnect unavailable");
            }
        }
    });
    Some(rx)
}

/// Small antialiased progress ring. Frames are cached once by the frontend;
/// animation never rerasterizes the full menu or delays its worker.
pub fn progress_face(frame: usize) -> UndoFace {
    let (w, h) = (32, 32);
    let mut rgba = vec![0; (w * h * 4) as usize];
    let turn = std::f32::consts::TAU;
    let angle = (frame % 12) as f32 * turn / 12.;
    for y in 0..h {
        for x in 0..w {
            let mut alpha = 0.;
            for sy in 0..4 {
                for sx in 0..4 {
                    let dx = x as f32 + (sx as f32 + 0.5) / 4. - 16.;
                    let dy = y as f32 + (sy as f32 + 0.5) / 4. - 16.;
                    let r = (dx * dx + dy * dy).sqrt();
                    if (10.5..13.).contains(&r) {
                        let trail = (angle - dy.atan2(dx)).rem_euclid(turn) / turn;
                        alpha += 0.16 + 0.84 * (1. - trail).powi(3);
                    }
                }
            }
            let at = ((y * w + x) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&[246, 244, 239, (alpha * 255. / 16.).round() as u8]);
        }
    }
    UndoFace { rgba, w, h }
}
