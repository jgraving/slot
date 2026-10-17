use slot_gfx::{Canvas, Draw, TexId, OUT_H};
use slot_power::Battery;

use crate::battery::{draw_gauge, GAUGE_H};
use crate::hud::{HUD_INK, PLATE_H};
use crate::plate::HINT_H;
use crate::slot_chrome::MOUTH_H;

pub const FOOTER_Y: f32 = OUT_H as f32 - MOUTH_H + (MOUTH_H - HINT_H as f32) / 2.0;
const FOOTER_MARGIN: f32 = 24.0;
const CORNER_MARGIN: f32 = 16.0;

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Printed {
    pub face: Option<TexId>,
    pub w: u32,
}

impl Printed {
    pub fn new(face: TexId, w: u32) -> Self {
        Printed {
            face: Some(face),
            w,
        }
    }
}

pub fn draw_footer(
    canvas: Canvas,
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    clock: Printed,
    ink: [u8; 3],
    out: &mut Vec<Draw>,
) {
    let y = FOOTER_Y + (HINT_H as f32 - GAUGE_H) / 2.0;
    draw_gauge(FOOTER_MARGIN, y, battery, percent, bolt, ink, out);
    printed(canvas.w - FOOTER_MARGIN - clock.w as f32, clock, out);
}

pub fn draw_corners(
    canvas: Canvas,
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    clock: Printed,
    out: &mut Vec<Draw>,
) {
    draw_status(canvas, battery, percent, bolt, clock, HUD_INK, out);
}

/// Persistent status at the same top corners used by the in-game overlay.
pub fn draw_status(
    canvas: Canvas,
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    clock: Printed,
    ink: [u8; 3],
    out: &mut Vec<Draw>,
) {
    let y = (PLATE_H - GAUGE_H) / 2.0;
    draw_gauge(CORNER_MARGIN, y, battery, percent, bolt, ink, out);
    let x = canvas.w - CORNER_MARGIN - clock.w as f32;
    draw_printed(x, (PLATE_H - HINT_H as f32) / 2.0, clock, out);
}

pub(crate) fn draw_printed(x: f32, y: f32, p: Printed, out: &mut Vec<Draw>) {
    if p.w == 0 {
        return;
    }
    let (w, h) = (p.w as f32, HINT_H as f32);
    out.push(match p.face {
        Some(tex) => Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha: 1.0,
        },
        None => Draw::Rect {
            x,
            y,
            w,
            h,
            colour: [1.0, 1.0, 1.0, 0.08],
        },
    });
}

fn printed(x: f32, p: Printed, out: &mut Vec<Draw>) {
    draw_printed(x, FOOTER_Y, p, out);
}

/// Place a connection icon after the complete battery gauge and percentage.
pub fn draw_footer_connection(
    battery: Option<Battery>,
    percent: Printed,
    icon: Option<TexId>,
    out: &mut Vec<Draw>,
) {
    draw_connection(
        battery,
        percent,
        icon,
        FOOTER_MARGIN,
        FOOTER_Y + (HINT_H as f32 - 20.0) / 2.0,
        out,
    );
}

pub fn draw_corner_connection(
    battery: Option<Battery>,
    percent: Printed,
    icon: Option<TexId>,
    out: &mut Vec<Draw>,
) {
    draw_connection(
        battery,
        percent,
        icon,
        CORNER_MARGIN,
        (PLATE_H - 20.0) / 2.0,
        out,
    );
}

fn draw_connection(
    battery: Option<Battery>,
    percent: Printed,
    icon: Option<TexId>,
    margin: f32,
    y: f32,
    out: &mut Vec<Draw>,
) {
    if let Some(tex) = icon {
        let x = margin
            + if battery.is_some() {
                crate::battery::gauge_width(percent) + 10.0
            } else {
                0.0
            };
        out.push(Draw::Tex {
            x,
            y,
            w: 20.0,
            h: 20.0,
            tex,
            alpha: 1.0,
        });
    }
}

#[cfg(test)]
mod connection_tests {
    use super::*;
    #[test]
    fn bluetooth_sits_after_the_percentage_and_is_absent_without_a_texture() {
        let mut draws = vec![];
        let percent = Printed::new(TexId::from_raw(1), 40);
        let battery = Some(Battery {
            percent: 75,
            charge: slot_power::Charge::Discharging,
        });
        draw_footer_connection(battery, percent, None, &mut draws);
        assert!(draws.is_empty());
        draw_footer_connection(battery, percent, Some(TexId::from_raw(2)), &mut draws);
        assert!(
            matches!(draws[0], Draw::Tex { x, w: 20.0, h: 20.0, .. } if x > FOOTER_MARGIN + 40.0 + 40.0)
        );
    }
}
