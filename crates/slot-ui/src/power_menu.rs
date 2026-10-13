use crate::plate::UndoFace;
use crate::text;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PowerChoice {
    PowerOff,
    Sleep,
}

impl PowerChoice {
    pub const ALL: [PowerChoice; 2] = [PowerChoice::PowerOff, PowerChoice::Sleep];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn text(self) -> &'static str {
        match self {
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

/// Current Anbernic RG SP finishes, plus the original GBA SP artwork.
/// RGB values are visual approximations from Anbernic's product photography.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PowerSwitchColour {
    #[default]
    FlameRed,
    Platinum,
    Graphite,
    PearlBlue,
    PearlPink,
}

impl PowerSwitchColour {
    pub const ALL: [Self; 5] = [
        Self::FlameRed,
        Self::Platinum,
        Self::Graphite,
        Self::PearlBlue,
        Self::PearlPink,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::FlameRed => "flame-red",
            Self::Platinum => "platinum",
            Self::Graphite => "graphite",
            Self::PearlBlue => "pearl-blue",
            Self::PearlPink => "pearl-pink",
        }
    }

    pub fn from_key(key: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|c| c.key() == key)
            .unwrap_or_default()
    }

    pub fn step(self, right: bool) -> Self {
        let i = Self::ALL.iter().position(|c| *c == self).unwrap_or(0);
        Self::ALL[(i + if right { 1 } else { Self::ALL.len() - 1 }) % Self::ALL.len()]
    }

    pub fn value(self) -> crate::QuickValue {
        match self {
            Self::FlameRed => crate::QuickValue::FlameRed,
            Self::Platinum => crate::QuickValue::Platinum,
            Self::Graphite => crate::QuickValue::Graphite,
            Self::PearlBlue => crate::QuickValue::PearlBlue,
            Self::PearlPink => crate::QuickValue::PearlPink,
        }
    }
}

fn coloured_shell(colour: PowerSwitchColour) -> String {
    let mut svg = include_str!("../assets/power_switch.svg").to_owned();
    let base: [u8; 3] = match colour {
        PowerSwitchColour::FlameRed => return svg,
        PowerSwitchColour::Platinum => [190, 193, 194],
        PowerSwitchColour::Graphite => [55, 57, 58],
        PowerSwitchColour::PearlBlue => [160, 199, 201],
        PowerSwitchColour::PearlPink => [222, 180, 197],
    };
    // Recolour only the shell inks. The cavity and charcoal slider stay neutral.
    // Use the original red channel as relief shading, retaining every bevel,
    // moulded marking and highlight from the approved vector.
    for ink in SHELL_INKS {
        let shade = u8::from_str_radix(&ink[1..3], 16).expect("shell ink");
        let rgb = base.map(|c| {
            if shade <= 174 {
                (u16::from(c) * u16::from(shade) / 174) as u8
            } else {
                c + ((255 - u16::from(c)) * u16::from(shade - 174) / 81) as u8
            }
        });
        svg = svg.replace(ink, &format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]));
    }
    svg
}

const SHELL_INKS: [&str; 27] = [
    "#552019", "#581d18", "#70241d", "#75251d", "#7d211a", "#7f281f", "#81271f", "#84261e",
    "#8b291f", "#9a2f22", "#a53224", "#a63727", "#ad3928", "#ad422d", "#b83a25", "#b93e28",
    "#bd4930", "#d16c49", "#d36340", "#d4734c", "#d5744c", "#dd6a45", "#e07850", "#e08559",
    "#e3895b", "#e38c60", "#ef9471",
];

// The two embedded SVG layers retain the approved artwork's 1000 x 400 geometry.
// Rasterise once at startup; each animation frame only moves the slider texture.
const SWITCH_W: u32 = 600;
const SWITCH_H: u32 = 240;
const SWITCH_SCALE: f32 = 0.6;
const SLIDER_W: u32 = 324;
const SLIDER_H: u32 = 102;

pub fn power_switch_faces(colour: PowerSwitchColour) -> [UndoFace; 2] {
    let face = |svg, w, h| UndoFace {
        rgba: crate::art::render_svg(svg, w, h).expect("embedded power switch SVG"),
        w,
        h,
    };
    [
        face(&coloured_shell(colour), SWITCH_W, SWITCH_H),
        face(
            include_str!("../assets/power_switch_slider.svg"),
            SLIDER_W,
            SLIDER_H,
        ),
    ]
}

/// Slide the faithful SP artwork from ON to OFF. Restart pauses, then returns to ON.
/// Stroke timing matches the existing power-switch audio detents.
pub fn draw_power_switch(
    progress: f32,
    restart: bool,
    faces: [crate::TexId; 2],
    out: &mut Vec<crate::Draw>,
) {
    let p = progress.clamp(0.0, 1.0);
    let seconds = p * if restart { 1.1 } else { 0.7 };
    let stroke = |t: f32| {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let off = if restart && seconds > 0.58 {
        1.0 - stroke((seconds - 0.58) / 0.28)
    } else {
        stroke((seconds - 0.06) / 0.28)
    };
    let alpha = ((1.0 - p) / 0.16).clamp(0.0, 1.0);
    let x = (crate::OUT_W - SWITCH_W) as f32 * 0.5;
    let y = (crate::OUT_H - SWITCH_H) as f32 * 0.5;
    out.push(crate::Draw::Tex {
        x,
        y,
        w: SWITCH_W as f32,
        h: SWITCH_H as f32,
        tex: faces[0],
        alpha,
    });
    out.push(crate::Draw::Tex {
        x: x + (286.0 + 94.0 * (1.0 - off)) * SWITCH_SCALE,
        y: y + 140.0 * SWITCH_SCALE,
        w: SLIDER_W as f32 * SWITCH_SCALE,
        h: SLIDER_H as f32 * SWITCH_SCALE,
        tex: faces[1],
        alpha,
    });
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
