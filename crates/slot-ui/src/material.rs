//! Satin painted plastic shared by the carousel housing and power switch.
use std::sync::OnceLock;
use slot_store::ShellColour;

pub fn shell_rgb(colour: ShellColour) -> [u8; 3] {
    match colour {
        ShellColour::Theme | ShellColour::Graphite => [72, 76, 83],
        ShellColour::Platinum => [190, 193, 198],
        ShellColour::PearlBlue => [119, 150, 170],
        ShellColour::PearlPink => [198, 155, 170],
    }
}

pub(crate) fn paint_grain(x: usize, y: usize) -> f32 {
    static GRAIN: OnceLock<Vec<u8>> = OnceLock::new();
    let grain = GRAIN.get_or_init(|| {
        crate::art::decode_png_bytes(include_bytes!("../../../assets/rgsp_paint_grain.png"))
            .expect("embedded RG SP paint sample").0
    });
    grain[((y % 128) * 128 + x % 128) * 4] as f32 - 128.0
}

pub fn shell_slot_faces(colour: ShellColour) -> [&'static [u8]; 2] {
    static FACES: [OnceLock<[Vec<u8>; 2]>; 4] = [const { OnceLock::new() }; 4];
    let index = colour.finish_index();
    let faces = FACES[index].get_or_init(|| {
        let base = shell_rgb(ShellColour::FINISHES[index]);
        [
            include_str!("../assets/slot_back.svg"),
            include_str!("../assets/slot_front.svg"),
        ].map(|bytes| {
            let w = 720usize;
            let mut rgba = crate::art::render_svg(bytes, 720, crate::bitmaps::SLOT_H).expect("embedded SVG shell geometry");
            for (at, pixel) in rgba.chunks_exact_mut(4).enumerate() {
                let painted = pixel[0] as f32 > pixel[1] as f32 * 1.5;
                let relief = (pixel[0] as f32 - 174.0) * 0.68;
                // SVG antialiasing can mix the red paint marker with neutral metal.
                // Remove that marker from mixed edge pixels as well as solid paint.
                let neutral_edge = !painted && pixel[0] > pixel[1];
                let neutral = pixel[1].max(pixel[2]);
                for c in 0..3 {
                    pixel[c] = if painted {
                        (base[c] as f32 + relief + paint_grain(at % w as usize, at / w as usize))
                            .clamp(0.0, 255.0) as u8
                    } else if neutral_edge { neutral } else { pixel[c] };
                }
            }
            rgba
        })
    });
    [&faces[0], &faces[1]]
}
