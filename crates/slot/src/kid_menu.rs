use crate::{
    kid_policy::{Policy, Rule},
    parent_code::Gate,
};
use slot_input::{Action, Btn};
use slot_ui::{TexId, UndoFace, OUT_W};
use std::{path::PathBuf, time::Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Unlock,
    Set,
    Confirm,
    Notice,
}
pub struct Menu {
    root: PathBuf,
    gate: Gate,
    pub draft: Policy,
    page: Page,
    selected: usize,
    code: Vec<u8>,
    first: Vec<u8>,
    status: String,
    pub visible: bool,
    pub face: Option<TexId>,
    pub changed: Option<Policy>,
}
impl Menu {
    pub fn new(root: PathBuf, policy: Policy) -> Self {
        let gate = Gate::new(Policy::directory(&root).join("code"));
        let page = if gate.enabled() || policy.active {
            Page::Unlock
        } else {
            Page::Set
        };
        Self {
            root,
            gate,
            draft: policy,
            page,
            selected: 0,
            code: Vec::new(),
            first: Vec::new(),
            status: String::new(),
            visible: true,
            face: None,
            changed: None,
        }
    }
    pub fn notice(root: PathBuf, policy: Policy, message: &str) -> Self {
        let mut menu = Self::new(root, policy);
        menu.page = Page::Notice;
        menu.status = message.into();
        menu
    }
    pub fn lines(&self) -> Vec<String> {
        let title = match self.page {
            Page::Set => "Choose four buttons",
            Page::Confirm => "Repeat your code",
            Page::Unlock => "Parent code",
            Page::Notice => "Play time finished",
            Page::Home => "",
        };
        if self.page == Page::Home {
            let value = if self.draft.rule == Rule::Until {
                format!(
                    "Until {:02}:{:02}",
                    self.draft.bedtime / 60,
                    self.draft.bedtime % 60
                )
            } else {
                format!("{} minutes", self.draft.minutes)
            };
            let rows = [
                self.draft.rule.label().to_string(),
                value,
                "Start kid mode".into(),
                if self.draft.active {
                    "Exit kid mode"
                } else {
                    "Back to games"
                }
                .into(),
                "Change parent code".into(),
            ];
            let mut lines = vec![String::new(), self.status.clone(), "COMPACT".into()];
            for (i, row) in rows.into_iter().enumerate() {
                lines.push(format!(
                    "{} {row}",
                    if i == self.selected {
                        "SELECTED"
                    } else {
                        "ROW"
                    }
                ));
            }
            lines.push(
                if self.selected < 2 {
                    "B Back   Left/Right Change"
                } else {
                    "B Back   A Select"
                }
                .into(),
            );
            return lines;
        }
        let status = if self.page == Page::Unlock && self.gate.remaining(Instant::now()) > 0 {
            format!(
                "Try again in {} seconds",
                self.gate.remaining(Instant::now())
            )
        } else {
            self.status.clone()
        };
        vec![
            title.into(),
            status,
            String::new(),
            if self.page == Page::Notice {
                "B Back   A Parent controls".into()
            } else {
                "PARENT CODE HINTS".into()
            },
        ]
    }
    pub fn screen_face(&self) -> UndoFace {
        let mut face = crate::bluetooth::screen_face(&self.lines());
        if matches!(self.page, Page::Set | Page::Confirm | Page::Unlock) {
            let buttons = slot_ui::button_code_face(&self.code);
            blit(&mut face, &buttons, (OUT_W - buttons.w) / 2, 208);
        }
        face
    }
    fn save(&mut self, policy: Policy) {
        match policy.save(&self.root) {
            Ok(()) => {
                self.draft = policy.clone();
                self.changed = Some(policy);
                self.visible = false;
            }
            Err(_) => self.status = "Could not save parent settings".into(),
        }
    }
    pub fn input(&mut self, action: Action, wall: i64) {
        self.face = None;
        if action == Action::QuickMenu {
            self.visible = false;
            return;
        }
        if self.page == Page::Notice {
            match action {
                Action::GbaDown(Btn::A) => {
                    self.page = Page::Unlock;
                    self.status.clear();
                }
                Action::GbaDown(Btn::B) => self.visible = false,
                _ => {}
            }
            return;
        }
        if self.page == Page::Home {
            match action {
                Action::GbaDown(Btn::B) => self.visible = false,
                Action::GbaDown(Btn::Up) => self.selected = (self.selected + 4) % 5,
                Action::GbaDown(Btn::Down) => self.selected = (self.selected + 1) % 5,
                Action::GbaDown(Btn::Left) | Action::GbaDown(Btn::Right) => {
                    let by = if action == Action::GbaDown(Btn::Left) {
                        -1
                    } else {
                        1
                    };
                    if self.selected == 0 {
                        self.draft.rule = self.draft.rule.next(by);
                    }
                    if self.selected == 1 {
                        if self.draft.rule == Rule::Until {
                            self.draft.bedtime =
                                (self.draft.bedtime as i32 + 15 * by).rem_euclid(1440) as u32;
                        } else {
                            self.draft.minutes =
                                (self.draft.minutes as i32 + 5 * by).clamp(5, 240) as u32;
                        }
                    }
                }
                Action::GbaDown(Btn::A) => match self.selected {
                    0 => self.draft.rule = self.draft.rule.next(1),
                    1 => self.input(Action::GbaDown(Btn::Right), wall),
                    2 => {
                        let mut next = self.draft.clone();
                        match next.begin(wall) {
                            Ok(()) => self.save(next),
                            Err(e) => self.status = e.to_string(),
                        }
                    }
                    3 => {
                        let mut next = self.draft.clone();
                        next.leave();
                        self.save(next);
                    }
                    4 => {
                        self.page = Page::Set;
                        self.status.clear();
                        self.code.clear();
                    }
                    _ => {}
                },
                _ => {}
            }
            return;
        }
        if action == Action::GbaDown(Btn::Select) {
            self.code.pop();
            return;
        }
        if action == Action::GbaDown(Btn::Start) {
            if self.code.len() != 4 {
                self.status = "Enter all four buttons".into();
                return;
            }
            match self.page {
                Page::Unlock => match self.gate.verify(&self.code, Instant::now()) {
                    Ok(true) => {
                        self.page = Page::Home;
                        self.status.clear();
                    }
                    Ok(false) => self.status = "Code not recognized".into(),
                    Err(_) => self.status = "Parent code could not be read".into(),
                },
                Page::Set => {
                    self.first = self.code.clone();
                    self.page = Page::Confirm;
                    self.status.clear();
                }
                Page::Confirm => {
                    if self.first != self.code {
                        self.page = Page::Set;
                        self.status = "Codes did not match. Try again.".into();
                    } else if self
                        .gate
                        .set(&self.code)
                        .and_then(|_| self.draft.save(&self.root))
                        .is_ok()
                    {
                        self.page = Page::Home;
                        self.status.clear();
                    } else {
                        self.status = "Could not save parent code".into();
                    }
                    self.first.fill(0);
                    self.first.clear();
                }
                _ => {}
            }
            self.code.fill(0);
            self.code.clear();
            return;
        }
        let button = match action {
            Action::GbaDown(Btn::A) => Some(0),
            Action::GbaDown(Btn::B) => Some(1),
            Action::GbaDown(Btn::X) => Some(2),
            Action::GbaDown(Btn::Y) => Some(3),
            Action::GbaDown(Btn::Up) => Some(4),
            Action::GbaDown(Btn::Down) => Some(5),
            Action::GbaDown(Btn::Left) => Some(6),
            Action::GbaDown(Btn::Right) => Some(7),
            _ => None,
        };
        if let Some(button) = button {
            if self.code.len() < 4 {
                self.code.push(button);
            }
        }
    }
}
fn blit(dst: &mut UndoFace, src: &UndoFace, left: u32, top: u32) {
    for y in 0..src.h {
        for x in 0..src.w {
            let s = ((y * src.w + x) * 4) as usize;
            let d = (((y + top) * dst.w + x + left) * 4) as usize;
            let alpha = u32::from(src.rgba[s + 3]);
            for c in 0..3 {
                dst.rgba[d + c] = ((u32::from(src.rgba[s + c]) * alpha
                    + u32::from(dst.rgba[d + c]) * (255 - alpha))
                    / 255) as u8;
            }
        }
    }
}

impl Drop for Menu {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.code.zeroize();
        self.first.zeroize();
    }
}
