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

struct ResultMessage {
    devices: Vec<Headphones>,
    status: String,
    output: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Home,
    Pair,
    Device(usize),
    Forget(usize),
}

pub struct Manager {
    root: PathBuf,
    pub page: Page,
    stack: Vec<Page>,
    pub devices: Vec<Headphones>,
    pub selected: usize,
    pub status: String,
    output: String,
    pending: Option<Receiver<ResultMessage>>,
    pub face: Option<TexId>,
    pub mute_on_disconnect: bool,
    pub interface_sounds: bool,
    pub disconnect_notice: bool,
    pub route_choice: Option<String>,
    choosing_route: bool,
}

impl Manager {
    pub fn new(root: PathBuf) -> Self {
        let mut manager = Self {
            root,
            page: Page::Home,
            stack: Vec::new(),
            devices: Vec::new(),
            selected: 0,
            status: String::new(),
            output: "speaker".into(),
            pending: None,
            face: None,
            mute_on_disconnect: false,
            interface_sounds: true,
            disconnect_notice: false,
            route_choice: None,
            choosing_route: false,
        };
        manager.ask("list", None);
        manager
    }

    fn ask(&mut self, verb: &'static str, address: Option<String>) {
        if self.pending.is_some() {
            return;
        }
        self.choosing_route = matches!(verb, "speaker" | "connect");
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
            "forget" => "Forgetting headphones...",
            "speaker" => "Switching to speaker...",
            "diagnose" => "Saving report...",
            _ => "",
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
                if let Some(output) = message.output {
                    if self.choosing_route && !message.status.starts_with("Error:") {
                        self.route_choice = Some(output.clone());
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
        self.stack.push(self.page.clone());
        self.page = page;
        self.selected = 0;
        self.status.clear();
    }

    fn device_indices(&self) -> Vec<usize> {
        self.devices
            .iter()
            .enumerate()
            .filter(|(_, d)| self.page == Page::Pair || d.paired)
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
                let mut rows = vec![if self.active_device().is_none()
                    && !(self.mute_on_disconnect && valid_address(&self.output))
                {
                    "ACTIVE Internal speaker"
                } else {
                    "Internal speaker"
                }
                .into()];
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
                rows.push("Pair audio device".into());
                rows.push(format!(
                    "TOGGLE {}",
                    if self.mute_on_disconnect { "On" } else { "Off" }
                ));
                rows.push(format!(
                    "SOUNDS {}",
                    if self.interface_sounds { "On" } else { "Off" }
                ));
                rows
            }
            Page::Pair => {
                let mut rows = vec!["Scan for headphones".into()];
                rows.extend(
                    self.device_indices()
                        .iter()
                        .map(|i| format!("DEVICE {}", self.devices[*i].name)),
                );
                rows
            }
            Page::Device(_) => vec!["Connect".into(), "Forget".into()],
            Page::Forget(_) => vec!["Cancel".into(), "Forget headphones".into()],
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
            Action::GbaDown(Btn::Left | Btn::Right)
                if self.page == Page::Home && self.selected == 2 + self.device_indices().len() =>
            {
                self.mute_on_disconnect = !self.mute_on_disconnect;
            }
            Action::GbaDown(Btn::Left | Btn::Right)
                if self.page == Page::Home && self.selected == 3 + self.device_indices().len() =>
            {
                self.interface_sounds = !self.interface_sounds;
            }
            Action::GbaDown(Btn::X) => self.ask("diagnose", None),
            Action::GbaDown(Btn::Y) if self.page == Page::Home => {
                if let Some(i) = self
                    .selected
                    .checked_sub(1)
                    .and_then(|n| self.device_indices().get(n).copied())
                {
                    self.open(Page::Device(i));
                }
            }
            Action::GbaDown(Btn::A) => match self.page.clone() {
                Page::Home => match self.selected {
                    0 => self.ask("speaker", None),
                    selected if selected == 1 + self.device_indices().len() => {
                        self.open(Page::Pair)
                    }
                    selected if selected == 2 + self.device_indices().len() => {
                        self.mute_on_disconnect = !self.mute_on_disconnect;
                    }
                    selected if selected == 3 + self.device_indices().len() => {
                        self.interface_sounds = !self.interface_sounds;
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
                        if self.devices[i].paired {
                            self.ask("connect", Some(self.devices[i].address.clone()));
                        } else {
                            self.open(Page::Device(i));
                        }
                    }
                }
                Page::Device(i) => {
                    if let Some(d) = self.devices.get(i) {
                        let address = d.address.clone();
                        match self.selected {
                            0 => self.ask("connect", Some(address)),
                            _ => self.open(Page::Forget(i)),
                        }
                    }
                }
                Page::Forget(i) => {
                    if self.selected == 1 {
                        let address = self.devices.get(i).map(|d| d.address.clone());
                        self.page = Page::Home;
                        self.stack.clear();
                        self.selected = 0;
                        self.ask("forget", address);
                    } else {
                        self.page = Page::Device(i);
                        self.stack.pop();
                        self.selected = 0;
                    }
                }
            },
            _ => {}
        }
        false
    }

    pub fn lines(&self) -> Vec<String> {
        let title = match self.page {
            Page::Home | Page::Pair => "",
            Page::Forget(_) => "Forget headphones?",
            Page::Device(i) => self
                .devices
                .get(i)
                .map(|d| d.name.as_str())
                .unwrap_or("Headphones"),
        };
        let message = if self.status.is_empty() && matches!(self.page, Page::Forget(_)) {
            "You will need to pair this device again.".to_owned()
        } else if self.status.is_empty() && self.disconnect_notice {
            "Headphones disconnected — choose output".into()
        } else if self.status.is_empty() && self.page == Page::Pair {
            "Put headphones in pairing mode".into()
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
            if self.page == Page::Home && self.selected >= 2 + self.device_indices().len() {
                if self.disconnect_notice {
                    "Change"
                } else {
                    "B Back · Change"
                }
            } else if self.disconnect_notice && self.page == Page::Home {
                "A Select"
            } else if self.page == Page::Home
                && self.selected >= 1
                && self.selected < 1 + self.device_indices().len()
            {
                "B Back · A Select · Y Manage"
            } else {
                "B Back · A Select"
            }
            .into(),
        );
        lines
    }
}

pub fn return_to_speaker(root: PathBuf) {
    std::thread::spawn(move || {
        let result = run(root, "speaker", None);
        if result.status.starts_with("Error:") {
            eprintln!("slot: {}", result.status);
        }
    });
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
            output: String::from_utf8_lossy(&output.stdout)
                .lines()
                .find_map(|line| line.strip_prefix("OUTPUT ").map(str::to_owned)),
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
            devices: Vec::new(),
            output: None,
            status: format!("Error: {error}"),
        },
    }
}

#[cfg(not(feature = "device"))]
fn run(_root: PathBuf, _verb: &str, _address: Option<String>) -> ResultMessage {
    ResultMessage {
        devices: Vec::new(),
        output: None,
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
                row_top + (i as u32 - 3) * 52,
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
        if i == lines.len() - 1 {
            if line == "CODE HINTS" {
                let hint = slot_ui::hint_face("MENU", "Cancel");
                blit(&mut rgba, &hint, (OUT_W - hint.w) / 2, 427);
                continue;
            }
            let mut hints = Vec::new();
            if line.contains("B Back") {
                hints.push(slot_ui::hint_face("B", "Back"));
            }
            hints.push(if line.contains("Change") {
                slot_ui::arrows_hint_face("Change")
            } else {
                slot_ui::hint_face("A", "Select")
            });
            if line.contains("Y Manage") {
                hints.push(slot_ui::hint_face("Y", "Manage"));
            }
            let gap = slot_ui::LEGEND_GAP as u32;
            let total = hints.iter().map(|h| h.w).sum::<u32>() + gap * (hints.len() as u32 - 1);
            let mut x = (OUT_W - total) / 2;
            for hint in hints {
                blit(&mut rgba, &hint, x, 427);
                x += hint.w + gap;
            }
            continue;
        }
        if let Some(value) = label
            .strip_prefix("TOGGLE ")
            .or_else(|| label.strip_prefix("SOUNDS "))
        {
            // Use the Screen menu's text, dimming and caret assets and spacing.
            let name = slot_ui::quick_value_face(
                if label.starts_with("SOUNDS ") {
                    "Cartridge sounds"
                } else {
                    "Mute on disconnect"
                },
                true,
            );
            blit(&mut rgba, &name, 14, top);
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
                blit(&mut rgba, &value, OUT_W - 32 + 18 - value.w, top);
            }
            continue;
        }
        let active = label.starts_with("ACTIVE ");
        let label = label.strip_prefix("ACTIVE ").unwrap_or(label);
        let device = label.starts_with("DEVICE ");
        let label = label.strip_prefix("DEVICE ").unwrap_or(label);
        let inset = if device { 68 } else { 32 };
        let label_w = label_w - (inset - 32);
        if device {
            blit(&mut rgba, &indicator_face(), 36, top + 12);
            let is_device = |line: &String| {
                let label = line
                    .strip_prefix("ROW ")
                    .or_else(|| line.strip_prefix("SELECTED "))
                    .unwrap_or(line);
                label
                    .strip_prefix("ACTIVE ")
                    .unwrap_or(label)
                    .starts_with("DEVICE ")
            };
            for (boundary, y) in [
                (!lines.get(i - 1).is_some_and(is_device), top - 4),
                (!lines.get(i + 1).is_some_and(is_device), top + 48),
            ] {
                if boundary {
                    for x in 32..OUT_W - 32 {
                        let at = ((y * OUT_W + x) * 4) as usize;
                        rgba[at..at + 4].copy_from_slice(&[40, 40, 46, 255]);
                    }
                }
            }
        }
        let layout = slot_ui::text::fit(
            font,
            label,
            (label_w - if active { 44 } else { 0 }) as f32,
            max_lines,
            if device { 26.0 } else { px },
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
            draw_check(&mut rgba, OUT_W - 56, top + h / 2);
        }
        let mut face = vec![0u8; (face_w * h * 4) as usize];
        slot_ui::text::draw_centred(&mut face, face_w, h, &layout, [246, 244, 239]);
        for y in 0..h {
            for x in 0..face_w {
                let src = ((y * face_w + x) * 4) as usize;
                let dst = (((top + y) * OUT_W + x + inset) * 4) as usize;
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

fn blit(dst: &mut [u8], face: &UndoFace, left: u32, top: u32) {
    for y in 0..face.h {
        for x in 0..face.w {
            let src = ((y * face.w + x) * 4) as usize;
            let dest = (((top + y) * OUT_W + left + x) * 4) as usize;
            let alpha = u32::from(face.rgba[src + 3]);
            for c in 0..3 {
                dst[dest + c] = ((u32::from(face.rgba[src + c]) * alpha
                    + u32::from(dst[dest + c]) * (255 - alpha))
                    / 255) as u8;
            }
        }
    }
}

fn draw_check(dst: &mut [u8], x: u32, y: u32) {
    // Draw the mark directly: the original slot label font has no check glyph.
    for i in 0..10 {
        for thickness in 0..3 {
            let offset = (((y - 3 + i + thickness) * OUT_W + x + i) * 4) as usize;
            dst[offset..offset + 4].copy_from_slice(&[246, 244, 239, 255]);
        }
    }
    for i in 0..19 {
        for thickness in 0..3 {
            let offset = (((y + 6 - i + thickness) * OUT_W + x + 9 + i) * 4) as usize;
            dst[offset..offset + 4].copy_from_slice(&[246, 244, 239, 255]);
        }
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
            output: "speaker".into(),
            pending: None,
            face: None,
            mute_on_disconnect: false,
            interface_sounds: true,
            disconnect_notice: false,
            route_choice: None,
            choosing_route: false,
        }
    }
    #[test]
    fn pairing_is_separate_and_back_restores_the_parent() {
        let mut m = menu();
        m.devices[0].paired = false;
        m.selected = 1;
        m.input(Action::GbaDown(Btn::A));
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
    fn saved_headphones_are_on_home_and_manage_requires_y() {
        let mut m = menu();
        assert_eq!(
            m.rows(),
            vec![
                "ACTIVE Internal speaker",
                "DEVICE WF-C510",
                "Pair audio device",
                "TOGGLE Off",
                "SOUNDS On"
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
        m.selected = 3;
        m.input(Action::GbaDown(Btn::Right));
        assert!(m.mute_on_disconnect);
        assert_eq!(m.lines().last().unwrap(), "B Back · Change");
        m.input(Action::GbaDown(Btn::Left));
        assert!(!m.mute_on_disconnect);
        if let Ok(path) = std::env::var("SLOT_SOUND_TOGGLE_PREVIEW") {
            std::fs::write(path, screen_face(&m.lines()).rgba).unwrap();
        }
        m.selected = 1;
        m.input(Action::GbaDown(Btn::Y));
        assert_eq!(m.page, Page::Device(0));
        m.selected = 1;
        m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.page, Page::Forget(0));
        assert_eq!(m.selected, 0);
        m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.page, Page::Device(0));
        assert!(m.pending.is_none());
    }
    #[test]
    fn active_route_not_connection_alone_controls_checkmark() {
        let mut m = menu();
        m.devices[0].connected = true;
        assert_eq!(m.rows()[0], "ACTIVE Internal speaker");
        m.output = m.devices[0].address.clone();
        assert_eq!(m.rows()[0], "Internal speaker");
        assert_eq!(m.rows()[1], "ACTIVE DEVICE WF-C510");
        m.devices[0].connected = false;
        assert_eq!(m.rows()[0], "ACTIVE Internal speaker");
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
        assert_eq!(m.rows(), vec!["Scan for headphones", "DEVICE WF-C510"]);
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
    fn diagnostic_results_preserve_devices_and_route() {
        let mut m = menu();
        let (tx, rx) = mpsc::channel();
        m.pending = Some(rx);
        m.output = m.devices[0].address.clone();
        tx.send(ResultMessage {
            devices: vec![],
            output: None,
            status: "Saved Bluetooth/latest-report.txt".into(),
        })
        .unwrap();
        m.poll();
        assert_eq!(m.devices.len(), 1);
        assert_eq!(m.output, m.devices[0].address);
    }
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
    pub fn address(&self) -> Option<String> {
        (!self.output.is_empty()).then(|| self.output.clone())
    }
    pub fn poll(&mut self) -> bool {
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
        if self
            .checked
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(1))
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
                face.rgba[offset..offset + 4].copy_from_slice(&[246, 244, 239, 255]);
            }
        }
    }
    face
}

#[cfg(test)]
mod indicator_tests {
    use super::*;
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
