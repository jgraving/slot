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
            include_bytes!("../../../assets/slot_back_material.png").as_slice(),
            include_bytes!("../../../assets/slot_front_material.png").as_slice(),
        ].map(|bytes| {
            let (mut rgba, w, _) = crate::art::decode_png_bytes(bytes).expect("embedded shell geometry");
            for (at, pixel) in rgba.chunks_exact_mut(4).enumerate() {
                let painted = pixel[1] > 128;
                let relief = pixel[0] as f32 - 128.0;
                let cavity = pixel[0];
                for c in 0..3 {
                    pixel[c] = if painted {
                        (base[c] as f32 + relief + paint_grain(at % w as usize, at / w as usize))
                            .clamp(0.0, 255.0) as u8
                    } else { cavity };
                }
            }
            rgba
        })
    });
    [&faces[0], &faces[1]]
}
