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

/// Short-travel SP thumb switch, with the broad grip and three raised ribs.
/// Restart makes the same off stroke, pauses, then switches back on.
pub fn draw_power_switch(progress: f32, restart: bool, out: &mut Vec<crate::Draw>) {
    let seconds = progress.clamp(0.0, 1.0) * if restart { 1.1 } else { 0.7 };
    let stroke = |t: f32| {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let off = if restart && seconds > 0.58 {
        1.0 - stroke((seconds - 0.58) / 0.28)
    } else {
        stroke((seconds - 0.06) / 0.28)
    };
    let fade = ((1.0 - progress) / 0.16).clamp(0.0, 1.0);
    let mut rect = |x, y, w, h, shade: f32| {
        out.push(crate::Draw::Rect {
            x,
            y,
            w,
            h,
            colour: [shade * 0.98, shade, shade * 1.03, fade],
        });
    };
    // Bevelled shell surround, recessed slot, and its upper shadow.
    rect(232.0, 184.0, 256.0, 112.0, 0.36);
    rect(228.0, 188.0, 264.0, 104.0, 0.36);
    rect(232.0, 188.0, 256.0, 104.0, 0.52);
    rect(236.0, 188.0, 248.0, 2.0, 0.68);
    rect(244.0, 204.0, 232.0, 46.0, 0.18);
    rect(248.0, 208.0, 224.0, 38.0, 0.07);
    let x = 302.0 - 50.0 * off;
    rect(x + 2.0, 211.0, 166.0, 34.0, 0.14);
    rect(x, 207.0, 164.0, 32.0, 0.49);
    rect(x + 2.0, 207.0, 160.0, 3.0, 0.72);
    for i in 0..3 {
        let rib = x + 37.0 + i as f32 * 36.0;
        rect(rib + 2.0, 215.0, 8.0, 22.0, 0.28);
        rect(rib, 213.0, 5.0, 21.0, 0.65);
    }
    // Small moulded OFF / ON markings; deliberately part of the hardware graphic.
    let letters = [
        (264.0, [14, 17, 17, 17, 14]),
        (276.0, [31, 16, 30, 16, 16]),
        (288.0, [31, 16, 30, 16, 16]),
        (406.0, [14, 17, 17, 17, 14]),
        (418.0, [17, 25, 21, 19, 17]),
    ];
    for (x, rows) in letters {
        for (y, bits) in rows.into_iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    rect(x + col as f32 * 2.0, 265.0 + y as f32 * 2.0, 2.0, 2.0, 0.24);
                }
            }
        }
    }
    rect(306.0, 268.0, 4.0, 4.0, 0.27);
    rect(394.0, 268.0, 4.0, 4.0, 0.27);
}

/// Close the screen into a thin phosphor line, then let the line fade.
pub fn draw_sleep(progress: f32, out: &mut Vec<crate::Draw>) {
    let p = progress.clamp(0.0, 1.0);
    let close = (p / 0.68).clamp(0.0, 1.0);
    let half = crate::OUT_H as f32 * 0.5;
    let cover = half * (1.0 - (1.0 - close).powi(3));
    for y in [0.0, crate::OUT_H as f32 - cover] {
        out.push(crate::Draw::Rect {
            x: 0.0,
            y,
            w: crate::OUT_W as f32,
            h: cover,
            colour: [0.0, 0.0, 0.0, 1.0],
        });
    }
    if (0.6..1.0).contains(&p) {
        let tail = ((p - 0.68) / 0.32).clamp(0.0, 1.0);
        let w = crate::OUT_W as f32 * (1.0 - tail).powi(2);
        out.push(crate::Draw::Rect {
            x: (crate::OUT_W as f32 - w) * 0.5,
            y: half - 1.0,
            w,
            h: 2.0,
            colour: [0.78, 0.84, 0.76, (1.0 - tail) * 0.7],
        });
    }
}
