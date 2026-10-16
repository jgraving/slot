use crate::bitmaps::{button_legend, ButtonLegend};
use crate::icon::haloed;
use crate::plate::blit;
use crate::{ease, text, Canvas, CartFace, Draw, Placed, TexId};

pub const FAVORITE_REMOVE_MS: u64 = 3000;

/// Upstream's sculpted Y key, paired with a quiet floating label. Raster at 2x.
pub fn favorite_remove_face() -> CartFace {
    let (button, bw, bh) = button_legend(ButtonLegend::Y);
    let font = text::label_font().expect("bundled label font");
    let layout = text::fit(font, "Again to remove", 400.0, 1, 32.0, 32.0);
    let tw = text::line_width(font, &layout.lines[0], layout.px, layout.tracking).ceil() as u32;
    let label = haloed(&text::coverage(tw, bh, &layout), tw, bh, [246, 244, 239]);
    let (w, h) = (bw + 12 + label.w, label.h);
    let mut rgba = vec![0; (w * h * 4) as usize];
    blit(&mut rgba, w, button, bw, bh, 0, (h - bh) / 2);
    blit(&mut rgba, w, &label.rgba, label.w, label.h, bw + 12, 0);
    CartFace { rgba, w, h }
}

pub fn draw_favorite_remove(
    canvas: Canvas,
    cart: Placed,
    (tex, width, height): (TexId, u32, u32),
    age_ms: u64,
    out: &mut Vec<Draw>,
) {
    let enter = ease((age_ms as f32 / 160.0).clamp(0.0, 1.0));
    let leave = ease((FAVORITE_REMOVE_MS.saturating_sub(age_ms) as f32 / 240.0).clamp(0.0, 1.0));
    let alpha = enter * leave;
    if alpha <= 0.0 { return; }
    let (w, h) = (width as f32 / 2.0, height as f32 / 2.0);
    let x = (cart.x + cart.w / 2.0 - w / 2.0).clamp(6.0, (canvas.w - w - 6.0).max(6.0));
    let y = (cart.y - h - 10.0 + 4.0 * (1.0 - enter)).max(6.0);
    out.push(Draw::Tex { x, y, w, h, tex, alpha });
}
