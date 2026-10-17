use slot_ui::{
    centred_hints, core_button_legends, footer_ink, link_button_legends, quick_legend_faces,
    Canvas, TexId, UndoFace, HINT_EDGE, LEGEND_GAP, HINT_H, OUT_W,
};

#[test]
fn a_row_of_hints_is_centred_by_what_shows_of_each() {
    let (a, b) = (TexId::from_raw(1), TexId::from_raw(2));
    let (aw, bw) = (71 + HINT_EDGE, 110 + HINT_EDGE);
    let placed = centred_hints(Canvas::FRAMED, &[(a, aw), (b, bw)], LEGEND_GAP);
    let x = ((OUT_W as f32 - (71.0 + LEGEND_GAP + 110.0)) / 2.0).round();
    assert_eq!(
        placed,
        vec![(a, aw, x), (b, bw, (x + 71.0 + LEGEND_GAP).round())]
    );
}

#[test]
fn a_lone_hint_is_centred_by_what_shows_of_it() {
    let a = TexId::from_raw(1);
    let aw = 150 + HINT_EDGE;
    assert_eq!(
        centred_hints(Canvas::FRAMED, &[(a, aw)], LEGEND_GAP),
        vec![(a, aw, ((OUT_W as f32 - 150.0) / 2.0).round())]
    );
}

#[test]
fn footer_legends_keep_the_hint_on_the_inner_side() {
    let menu = quick_legend_faces(footer_ink(false));
    let light = quick_legend_faces(footer_ink(true));
    assert_eq!(hint_ink(&menu[0], true), footer_ink(false));
    assert_eq!(hint_ink(&light[0], true), footer_ink(true));
    assert_eq!(hint_ink(&light[2], false), footer_ink(true));
    assert!(menu.iter().all(|f| f.h == HINT_H));
    assert!(menu[3].w > menu[0].w, "Resume is a longer hint than Back");
    assert!(
        hint_is_beside(&menu[0], true),
        "Back sits to the right of B"
    );
    assert!(
        hint_is_beside(&menu[2], false),
        "Open sits to the left of A"
    );
    assert!(
        hint_is_beside(&menu[4], false),
        "Restart sits to the left of X"
    );
    let link = link_button_legends([0xff, 0xff, 0xff]);
    assert_eq!(link.len(), 7);
    assert!(link.iter().all(|f| f.h == HINT_H));
    assert!(
        hint_is_beside(&link[1], true),
        "Mode sits to the right of Select"
    );
    assert!(
        hint_is_beside(&link[2], false),
        "Swap sits to the left of the d-pad"
    );
    let core = core_button_legends();
    assert!(core.iter().all(|f| f.h == HINT_H));
    assert!(
        hint_is_beside(&core[0], true),
        "Cancel sits to the right of B"
    );
    assert!(
        hint_is_beside(&core[1], true),
        "Swap sits to the right of the d-pad"
    );
    assert!(
        hint_is_beside(&core[2], false),
        "Choose sits to the left of A"
    );
}

fn hint_is_beside(face: &slot_ui::UndoFace, on_right: bool) -> bool {
    let button = if on_right {0..HINT_H} else {face.w-HINT_EDGE-HINT_H..face.w-HINT_EDGE};
    painted(face, button, 1..face.h-1) && hint_ink(face,on_right).iter().any(|c|*c>0)
}

fn hint_ink(face: &UndoFace, on_right: bool) -> [u8; 3] {
    let xs = match on_right {
        true => face.w - HINT_EDGE - 24..face.w - HINT_EDGE,
        false => 0..24,
    };
    let mut best = (0u8, [0u8; 3]);
    for y in 0..face.h {
        for x in xs.clone() {
            let i = ((y * face.w + x) * 4) as usize;
            if face.rgba[i + 3] > best.0 {
                best = (face.rgba[i + 3], face.rgba[i..i + 3].try_into().unwrap());
            }
        }
    }
    assert!(best.0 > 16, "the hint has no ink");
    best.1
}

fn painted(face: &slot_ui::UndoFace, xs: std::ops::Range<u32>, ys: std::ops::Range<u32>) -> bool {
    ys.flat_map(|y| xs.clone().map(move |x| (x, y)))
        .any(|(x, y)| face.rgba[((y * face.w + x) * 4 + 3) as usize] > 16)
}

#[test]
fn round_buttons_stay_round_at_the_compact_hint_height() {
    let f=&quick_legend_faces(footer_ink(false))[0];
    assert_eq!(f.h,HINT_H);
    let xs:Vec<_>=(0..HINT_H).filter(|&x|(0..f.h).any(|y| f.rgba[((y*f.w+x)*4+3) as usize]>128)).collect();
    let ys:Vec<_>=(0..f.h).filter(|&y|(0..HINT_H).any(|x| f.rgba[((y*f.w+x)*4+3) as usize]>128)).collect();
    assert!((xs.len() as i32-ys.len() as i32).abs()<=1,"button icon was stretched");
}
