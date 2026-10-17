//! Export production carousel and switch draws for all four RG SP finishes.
use slot_store::{Cart, Platform, ShellColour};
use slot_ui::{Canvas, Draw, Shelf, TexId};
use std::{fs::File, io::Write, path::Path};

fn png(dir: &Path, name: &str, w: u32, h: u32, rgba: &[u8]) {
    let mut enc = png::Encoder::new(File::create(dir.join(name)).unwrap(), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgba).unwrap();
}

fn main() {
    let output = std::env::args().nth(1).expect("output directory");
    let mode = std::env::args().nth(2);
    let animation = mode.as_deref() == Some("animation");
    let seats: Vec<Option<f32>> = if animation { (0..=40).map(|i|Some(i as f32/40.0)).collect() }
        else { vec![mode.map(|s|s.parse().unwrap())] };
    let platform = match std::env::args().nth(3).as_deref() { Some("gb")=>Platform::Gb, Some("gbc")=>Platform::Gbc, _=>Platform::Gba };
    for colour in ShellColour::FINISHES {
        let dir = Path::new(&output).join(colour.key());
        std::fs::create_dir_all(&dir).unwrap();
        let mut shelf = Shelf::new(["CLASSICS", "ADVENTURE", "ARCADE"].iter().map(|name| Cart {
            platform, stem: name.to_string(), rom: format!("{name}.gba").into(),
            label: None, code: String::new(),
            shell: (platform==Platform::Gbc).then(||slot_store::ShellChoice::parse("rounded 7585a4 solid").unwrap()),
            title: name.to_string(),
        }).collect());
        for (i, cart) in shelf.carts.iter().enumerate() {
            let f = slot_ui::cart_face(cart);
            png(&dir, &format!("{i}.png"), f.w, f.h, &f.rgba);
        }
        shelf.set_faces((0..3).map(TexId::from_raw).collect());
        shelf.select(1);
        for (i, bytes) in slot_ui::shell_slot_faces(colour).iter().enumerate() {
            png(&dir, &format!("{}.png", i+3), slot_ui::SLOT_W, slot_ui::SLOT_H, bytes);
        }
        png(&dir, "5.png", 720, 480, slot_ui::default_background().unwrap());
        for (frame, seat) in seats.iter().copied().enumerate() {
        let mut draws = Vec::new();
        slot_ui::draw_ground(Canvas::FRAMED, Some(TexId::from_raw(5)), &mut draws);
        if let Some(seat) = seat {
            let (rest, scale) = shelf.selected_at(Canvas::FRAMED);
            slot_ui::SlotChrome { cart: &shelf.carts[1], face: Some(TexId::from_raw(1)), rest, scale, seat,
                alert: None, dim: 0.0, screen: 0.0, game: false, over: true, behind: [None,None],
                slot: Some((TexId::from_raw(3),TexId::from_raw(4))), canvas: Canvas::FRAMED }.draw(&mut draws);
        } else {
            shelf.draw(Canvas::FRAMED, 0.0, Some((TexId::from_raw(3), TexId::from_raw(4))), &mut draws);
        }
        let mut csv = File::create(dir.join(if animation { format!("draws-{frame:03}.csv") } else { "draws.csv".into() })).unwrap();
        writeln!(csv, "id,x,y,w,h,alpha,u,v,uw,vh").unwrap();
        for draw in draws {
            let (x,y,w,h,tex,alpha,uv) = match draw {
                Draw::Tex { x,y,w,h,tex,alpha } => (x,y,w,h,tex,alpha,[0.0,0.0,1.0,1.0]),
                Draw::Part { x,y,w,h,tex,alpha,uv } => (x,y,w,h,tex,alpha,uv),
                _ => continue,
            };
            let id = (0..6).find(|i| TexId::from_raw(*i) == tex).unwrap();
            writeln!(csv, "{id},{x},{y},{w},{h},{alpha},{},{},{},{}",uv[0],uv[1],uv[2],uv[3]).unwrap();
        }
        }
        for (i, f) in slot_ui::power_switch_faces_for(colour).iter().enumerate() {
            png(&dir, &format!("switch-{i}.png"), f.w, f.h, &f.rgba);
        }
    }
}
