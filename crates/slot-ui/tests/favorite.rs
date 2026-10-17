use slot_store::{Cart, Platform};
use slot_ui::{draw_star, star_face, Canvas, Draw, Placed, Shelf, SlotChrome, TexId, STAR_INK};

fn cart(platform: Platform) -> Cart {
    Cart {
        platform,
        stem: "Favorite".into(),
        title: "Favorite".into(),
        rom: "missing.gba".into(),
        label: None,
        code: String::new(),
        shell: None,
    }
}
fn quad(draw: &Draw) -> (f32, f32, f32, f32, f32) {
    match *draw {
        Draw::Tex {
            x, y, w, h, alpha, ..
        }
        | Draw::Turned {
            x, y, w, h, alpha, ..
        } => (x, y, w, h, alpha),
        _ => panic!("texture expected"),
    }
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.001, "{a} != {b}");
}

#[test]
fn sticker_scales_and_fades_with_its_shell() {
    let cart = cart(Platform::Gba);
    let star = (TexId::from_raw(2), 33., 33.);
    let mut out = vec![];
    for k in [1., 0.78, 0.4] {
        draw_star(
            &cart,
            star,
            Placed {
                x: 20.,
                y: 30.,
                w: 312. * k,
                h: 180. * k,
            },
            0.6,
            0.,
            &mut out,
        );
    }
    let (x, y, w, h, _) = quad(&out[0]);
    for (i, k) in [(1, 0.78), (2, 0.4)] {
        let (xx, yy, ww, hh, a) = quad(&out[i]);
        close((xx - 20.) / k, x - 20.);
        close((yy - 30.) / k, y - 30.);
        close(ww / k, w);
        close(hh / k, h);
        close(a, 0.6);
    }
}

#[test]
fn sticker_rotates_around_the_cartridge_not_its_own_fixed_position() {
    let body = Placed {
        x: 30.,
        y: 40.,
        w: 312.,
        h: 180.,
    };
    let mut out = vec![];
    for turn in [0., std::f32::consts::FRAC_PI_2] {
        draw_star(
            &cart(Platform::Gba),
            (TexId::from_raw(2), 33., 33.),
            body,
            1.,
            turn,
            &mut out,
        );
    }
    let (x, y, w, h, _) = quad(&out[0]);
    let (xx, yy, _, _, _) = quad(&out[1]);
    let (cx, cy) = (body.x + body.w / 2., body.y + body.h / 2.);
    close(xx + w / 2. - cx, -(y + h / 2. - cy));
    close(yy + h / 2. - cy, x + w / 2. - cx);
    assert!(matches!(out[1], Draw::Turned { .. }));
}

#[test]
fn shelf_and_insertion_share_sticker_placement_and_slot_occludes_it() {
    let (face, star, back, front) = (
        TexId::from_raw(1),
        TexId::from_raw(2),
        TexId::from_raw(3),
        TexId::from_raw(4),
    );
    for platform in [Platform::Gba, Platform::Gb, Platform::Gbc] {
        let c = cart(platform);
        let mut shelf = Shelf::new(vec![c.clone()]);
        shelf.set_faces(vec![face]);
        shelf.set_star(star, 33, 33);
        shelf.mark_starred(|_| true);
        let mut row = vec![];
        shelf.draw_row(Canvas::FRAMED, None, 0., 0., 1., &mut row);
        let shelf_star = row
            .iter()
            .find(|d| matches!(d,Draw::Tex{tex,..} if *tex==star))
            .unwrap();
        let (rest, scale) = shelf.selected_at(Canvas::FRAMED);
        for seat in [0., 0.25, 0.5, 0.75, 1.] {
            let mut out = vec![];
            SlotChrome {
                cart: &c,
                face: Some(face),
                rest,
                scale,
                seat,
                alert: None,
                dim: 0.,
                screen: 0.,
                game: false,
                over: false,
                behind: [None, None],
                slot: Some((back, front)),
                canvas: Canvas::FRAMED,
            }
            .draw_with_star(shelf.star_for(&c.stem), &mut out);
            let star_at = out
                .iter()
                .position(|d| matches!(d,Draw::Tex{tex,..} if *tex==star))
                .unwrap();
            let face_at = out
                .iter()
                .position(|d| matches!(d,Draw::Tex{tex,..} if *tex==face))
                .unwrap();
            let front_at = out
                .iter()
                .position(|d| matches!(d,Draw::Part{tex,..} if *tex==front))
                .unwrap();
            assert!(face_at < star_at && star_at < front_at);
            if seat == 0. {
                assert_eq!(quad(shelf_star), quad(&out[star_at]));
            }
            let (_, _, cw, _, ca) = quad(&out[face_at]);
            let (_, _, sw, _, sa) = quad(&out[star_at]);
            close(sw / cw, 33. / slot_ui::cart_box(platform).0 as f32);
            close(ca, sa);
        }
        shelf.mark_starred(|_| false);
        assert!(shelf.star_for(&c.stem).is_none());
    }
}

#[test]
fn foil_has_transparent_edges_and_metallic_tones() {
    let f = star_face(12, STAR_INK);
    assert_eq!(&f.rgba[..4], &[0, 0, 0, 0]);
    assert!(f.rgba.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
    let tones: std::collections::HashSet<_> = f
        .rgba
        .chunks_exact(4)
        .filter(|p| p[3] == 255)
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    assert!(tones.len() > 100);
}

#[test]
fn glossy_sticker_interior_is_opaque_at_every_texture_size() {
    // Independent outline, inset from the rounded/antialiased edge. Highlights
    // and texture must never punch holes through to the cartridge label.
    let outline = [
        (64., 9.),
        (79., 43.),
        (115., 47.),
        (87., 71.),
        (95., 107.),
        (64., 89.),
        (33., 107.),
        (41., 71.),
        (14., 47.),
        (49., 43.),
    ];
    for scale in [3, 12, 24] {
        let f = star_face(scale, STAR_INK);
        let mut checked = 0;
        let (mut darkest, mut lightest) = (255u8, 0u8);
        for y in 0..f.h {
            for x in 0..f.w {
                let (px, py) = (
                    (x as f32 + 0.5) * 128. / f.w as f32,
                    (y as f32 + 0.5) * 128. / f.h as f32,
                );
                let mut inside = false;
                let mut distance = f32::MAX;
                for i in 0..outline.len() {
                    let (ax, ay) = outline[i];
                    let (bx, by) = outline[(i + 1) % outline.len()];
                    if (ay > py) != (by > py) && px < (bx - ax) * (py - ay) / (by - ay) + ax {
                        inside = !inside;
                    }
                    let (dx, dy) = (bx - ax, by - ay);
                    let t = (((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)).clamp(0., 1.);
                    distance = distance.min((px - ax - t * dx).hypot(py - ay - t * dy));
                }
                if inside && distance > 5. {
                    let p = &f.rgba[((y * f.w + x) * 4) as usize..][..4];
                    assert_eq!(p[3], 255, "hole at {x},{y}, scale {scale}");
                    assert!(p[0] > p[2].saturating_add(20), "foil lost its gold tint at {x},{y}");
                    darkest = darkest.min(p[0]);
                    lightest = lightest.max(p[0]);
                    checked += 1;
                }
            }
        }
        assert!(checked > 100);
        assert!(lightest - darkest > 70, "foil reflections need visible contrast");
    }
}

#[test]
fn departure_moves_every_cartridge_layer_up_and_shrinks_to_insertion_size() {
    for platform in [Platform::Gba, Platform::Gb, Platform::Gbc] {
        let mut shelf = Shelf::new(vec![cart(platform)]);
        let face = TexId::from_raw(201);
        let star = TexId::from_raw(202);
        shelf.set_faces(vec![face]);
        shelf.set_star(star, 33, 33);
        shelf.mark_starred(|_| true);
        let canvas = Canvas::FRAMED;
        let mut previous = None;
        for progress in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let mut draws = vec![];
            shelf.draw_departing(canvas, progress, None, &mut draws);
            let body = draws.iter().find(|d| matches!(d, Draw::Tex { tex, .. } if *tex == face)).unwrap();
            let sticker = draws.iter().find(|d| matches!(d, Draw::Tex { tex, .. } if *tex == star)).unwrap();
            let (_, y, w, h, a) = quad(body);
            let (_, _, sw, _, sa) = quad(sticker);
            if let Some((py, pw)) = previous { assert!(y < py && w <= pw+0.001); }
            assert_eq!(a, 1.0);
            assert_eq!(sa, 1.0);
            close(sw / w, 33.0 / slot_ui::cart_box(platform).0 as f32);
            if progress == 1.0 { assert!(y + h < 0.0); }
            previous = Some((y, w));
        }
    }
}

#[test]
fn next_favorite_moves_into_place_during_departure_including_last_and_two_carts() {
    for count in [2, 3, 4] {
        for selected in 0..count {
            let carts: Vec<_> = (0..count).map(|i| { let mut c = cart(Platform::Gba); c.stem = i.to_string(); c }).collect();
            let mut shelf = Shelf::new(carts);
            shelf.set_faces((0..count).map(|i| TexId::from_raw(300 + i)).collect());
            shelf.select(selected);
            let next = if selected + 1 < count { selected + 1 } else { selected - 1 };
            let tex = TexId::from_raw(300 + next);
            let mut distance = f32::MAX;
            for progress in [0.0, 0.5, 1.0] {
                let mut draws = vec![];
                shelf.draw_departing(Canvas::FRAMED, progress, None, &mut draws);
                let nearest = draws.iter().filter(|d| matches!(d, Draw::Tex { tex: t, .. } if *t == tex))
                    .map(|d| { let (x, _, w, _, _) = quad(d); (x + w / 2.0 - Canvas::FRAMED.w / 2.0).abs() })
                    .fold(f32::MAX, f32::min);
                assert!(nearest < distance);
                if progress == 1.0 { close(nearest, 0.0); }
                distance = nearest;
            }
        }
    }
}
