use crate::{
    cart::{cart_box, gb_shell_of, label_rect},
    Draw, Placed, TexId,
};
use slot_store::Cart;

/// A sticker in cartridge coordinates, sharing the shell's scale and rotation.
/// Draw before the slot front so the lip covers it along with the cartridge.
pub fn draw_star(
    cart: &Cart,
    (tex, sw, sh): (TexId, f32, f32),
    body: Placed,
    alpha: f32,
    turn: f32,
    out: &mut Vec<Draw>,
) {
    let (cw, ch) = cart_box(cart.platform);
    let (sx, sy) = (body.w / cw as f32, body.h / ch as f32);
    let (lx, ly, lw, _) = label_rect(gb_shell_of(cart));
    let (w, h) = (sw * sx, sh * sy);
    let x = body.x + (lx + lw) * body.w - w - 5.0 * sx;
    let y = body.y + ly * body.h + 5.0 * sy;
    if turn == 0.0 {
        out.push(Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha,
        });
    } else {
        let (cx, cy) = (body.x + body.w / 2.0, body.y + body.h / 2.0);
        let (dx, dy) = (x + w / 2.0 - cx, y + h / 2.0 - cy);
        let (sin, cos) = turn.sin_cos();
        out.push(Draw::Turned {
            x: cx + dx * cos - dy * sin - w / 2.0,
            y: cy + dx * sin + dy * cos - h / 2.0,
            w,
            h,
            tex,
            alpha,
            turn,
        });
    }
}
