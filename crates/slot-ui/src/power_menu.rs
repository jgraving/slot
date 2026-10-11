use crate::plate::UndoFace;
use crate::text;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PowerChoice {
    Restart,
    PowerOff,
    Sleep,
}

impl PowerChoice {
    pub const ALL: [PowerChoice; 3] = [
        PowerChoice::Restart,
        PowerChoice::PowerOff,
        PowerChoice::Sleep,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn text(self) -> &'static str {
        match self {
            PowerChoice::Restart => "Restart",
            PowerChoice::PowerOff => "Power Off",
            PowerChoice::Sleep => "Sleep",
        }
    }
}

pub(crate) const MENU_PX: f32 = 30.0;
const MENU_MIN_PX: f32 = 18.0;
pub(crate) const MENU_H: u32 = 40;
pub const MENU_PAD: u32 = 18;
pub(crate) const MENU_INK: [u8; 3] = [0xf6, 0xf4, 0xef];

pub fn menu_face(label: &str) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let ink = text::line_width(font, label, MENU_PX, 0.0).ceil() as u32;
    let w = ink + 2 * MENU_PAD;
    let mut rgba = vec![0u8; (w * MENU_H * 4) as usize];
    let layout = text::fit(font, label, w as f32, 1, MENU_PX, MENU_MIN_PX);
    text::draw_centred(&mut rgba, w, MENU_H, &layout, MENU_INK);
    UndoFace { rgba, w, h: MENU_H }
}

/// A compact, ridged SP-style switch sliding into its off detent.
pub fn draw_power_switch(progress: f32, out: &mut Vec<crate::Draw>) {
    let p = (progress / 0.72).clamp(0.0, 1.0);
    let p = p * p * (3.0 - 2.0 * p);
    let fade = ((1.0 - progress) / 0.22).clamp(0.0, 1.0);
    let mut rect = |x, y, w, h, shade: f32| {
        out.push(crate::Draw::Rect {
            x,
            y,
            w,
            h,
            colour: [shade, shade, shade * 1.03, fade],
        })
    };
    rect(210.0, 194.0, 300.0, 92.0, 0.22);
    rect(214.0, 198.0, 292.0, 84.0, 0.34);
    rect(230.0, 213.0, 260.0, 54.0, 0.035);
    let x = 398.0 - 160.0 * p;
    rect(x + 3.0, 215.0, 86.0, 50.0, 0.12);
    rect(x, 211.0, 86.0, 50.0, 0.62);
    rect(x + 3.0, 214.0, 80.0, 3.0, 0.82);
    for i in 0..6 {
        rect(x + 17.0 + i as f32 * 10.0, 222.0, 3.0, 29.0, 0.28);
    }
    out.push(crate::Draw::Rect {
        x: 496.0,
        y: 230.0,
        w: 6.0,
        h: 12.0,
        colour: [0.18, 0.75 * (1.0 - p) + 0.08, 0.13, fade],
    });
}
