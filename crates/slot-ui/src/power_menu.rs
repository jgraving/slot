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
/// Time at which the slider reaches OFF (including the initial ON hold).
pub const POWER_SWITCH_SLIDE_MS: u64 = 330;

/// Compatibility entry point for the automatic dark/light shell selection.
pub fn power_switch_faces(light: bool) -> [UndoFace; 2] {
    power_switch_faces_for(if light { slot_store::ShellColour::Platinum } else { slot_store::ShellColour::Graphite })
}

/// The switch shares the carousel's extracted paint grain and shell palette.
pub fn power_switch_faces_for(colour: slot_store::ShellColour) -> [UndoFace; 2] {
    let mut rgba = crate::art::render_svg(include_str!("../assets/power_switch.svg"), 600, 240)
        .expect("embedded power switch housing");
    let base = crate::material::shell_rgb(colour);
    for y in 0..240usize {
        for x in 0..600usize {
            let i = (y * 600 + x) * 4;
            let r = rgba[i] as f32;
            if r > rgba[i + 1] as f32 * 1.5 && rgba[i + 3] != 0 {
                let relief = (r - 174.0) * 0.68;
                let grain = crate::material::paint_grain(x, y);
                for c in 0..3 {
                    rgba[i + c] = (base[c] as f32 + grain + relief).clamp(0.0, 255.0) as u8;
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
    // 150 ms ON, 180 ms snap, 170 ms OFF, 100 ms fade; total remains 600 ms.
    let elapsed = p * POWER_SWITCH_MS as f32;
    let t = ((elapsed - 150.0) / 180.0).clamp(0.0, 1.0);
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
