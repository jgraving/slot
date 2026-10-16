use crate::plate::UndoFace;
use crate::text;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PowerChoice {
    Restart,
    PowerOff,
}

impl PowerChoice {
    pub const ALL: [PowerChoice; 2] = [PowerChoice::Restart, PowerChoice::PowerOff];

    pub fn index(self) -> usize {
        self as usize
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

/// A short physical ON-to-OFF stroke, followed by a fade to black.
pub const POWER_SWITCH_MS: u64 = 600;

/// Build both finishes once, using the carousel's own pebbled housing pixels.
/// The red source vector identifies shell material; the neutral cavity is left alone.
pub fn power_switch_faces(light: bool) -> [UndoFace; 2] {
    let mut rgba = crate::art::render_svg(include_str!("../assets/power_switch.svg"), 600, 240)
        .expect("embedded power switch housing");
    let material = if light {
        crate::slot_front_light()
    } else {
        crate::slot_front()
    };
    for y in 0..240usize {
        for x in 0..600usize {
            let i = (y * 600 + x) * 4;
            let r = rgba[i] as f32;
            if r > rgba[i + 1] as f32 * 1.5 && rgba[i + 3] != 0 {
                // Sample the existing grain at native scale. Remove its broad lighting
                // so the switch's own bevels supply the relief, without mirrored seams.
                let tx = 12 + (x + (y / 32) * 47) % 196;
                let ty = 12 + y % 32;
                let m = (ty * crate::SLOT_W as usize + tx) * 4;
                let base = if light {
                    [210.0, 210.0, 212.0]
                } else {
                    [38.0, 38.0, 43.0]
                };
                let relief = (r - 174.0) * if light { 0.8 } else { 0.5 };
                for c in 0..3 {
                    let mut local = 0.0;
                    for dy in [0, 2, 4] {
                        for dx in [0, 2, 4] {
                            local += material
                                [((ty + dy - 2) * crate::SLOT_W as usize + tx + dx - 2) * 4 + c]
                                as f32;
                        }
                    }
                    let grain = (material[m + c] as f32 - local / 9.0) * 0.55;
                    rgba[i + c] = (base[c] + grain + relief).clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
    [
        UndoFace {
            rgba,
            w: 600,
            h: 240,
        },
        UndoFace {
            rgba: crate::art::render_svg(
                include_str!("../assets/power_switch_slider.svg"),
                324,
                102,
            )
            .expect("embedded power switch slider"),
            w: 324,
            h: 102,
        },
    ]
}

/// Draw on the current canvas, including narrow and framed panels.
pub fn draw_power_switch(
    canvas: crate::Canvas,
    progress: f32,
    faces: [crate::TexId; 2],
    out: &mut Vec<crate::Draw>,
) {
    let p = progress.clamp(0.0, 1.0);
    // 450 ms slide, 50 ms at OFF, then 100 ms fade; total remains 600 ms.
    let elapsed = p * POWER_SWITCH_MS as f32;
    let t = (elapsed / 450.0).clamp(0.0, 1.0);
    let off = t * t * (3.0 - 2.0 * t);
    let alpha = ((POWER_SWITCH_MS as f32 - elapsed) / 100.0).clamp(0.0, 1.0);
    let scale = (canvas.w / 720.0).min(1.0);
    let w = 600.0 * scale;
    let h = 240.0 * scale;
    let x = canvas.centred(w);
    let y = (crate::OUT_H as f32 - h) * 0.5;
    out.push(crate::Draw::Tex {
        x,
        y,
        w,
        h,
        tex: faces[0],
        alpha,
    });
    out.push(crate::Draw::Tex {
        x: x + (286.0 + 94.0 * (1.0 - off)) * 0.6 * scale,
        y: y + 140.0 * 0.6 * scale,
        w: 324.0 * 0.6 * scale,
        h: 102.0 * 0.6 * scale,
        tex: faces[1],
        alpha,
    });
}
