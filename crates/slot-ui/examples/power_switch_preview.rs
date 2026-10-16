//! Export the production textures and draw positions for tools/power-switch-gif.py.
//! cargo run -p slot-ui --example power_switch_preview -- /tmp/power-switch
use slot_ui::{draw_power_switch, power_switch_faces, Canvas, Draw, TexId};
use std::{fs::File, io::Write, path::PathBuf};

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&dir).unwrap();
    for light in [false, true] {
        let name = if light { "light" } else { "dark" };
        for (i, face) in power_switch_faces(light).iter().enumerate() {
            let file = File::create(dir.join(format!("{name}-{i}.png"))).unwrap();
            let mut enc = png::Encoder::new(file, face.w, face.h);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header()
                .unwrap()
                .write_image_data(&face.rgba)
                .unwrap();
        }
    }
    let mut poses = File::create(dir.join("poses.csv")).unwrap();
    writeln!(poses, "frame,layer,x,y,w,h,alpha").unwrap();
    for frame in 0..=30 {
        let mut draws = Vec::new();
        draw_power_switch(
            Canvas::FRAMED,
            frame as f32 / 30.0,
            [TexId::from_raw(0), TexId::from_raw(1)],
            &mut draws,
        );
        for (i, draw) in draws.iter().enumerate() {
            if let Draw::Tex {
                x, y, w, h, alpha, ..
            } = draw
            {
                writeln!(poses, "{frame},{i},{x},{y},{w},{h},{alpha}").unwrap();
            }
        }
    }
}
