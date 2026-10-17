use slot_ui::*;

fn px(face: &CartFace, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * face.w + x) * 4) as usize;
    [
        face.rgba[at],
        face.rgba[at + 1],
        face.rgba[at + 2],
        face.rgba[at + 3],
    ]
}

#[test]
fn every_face_is_the_size_the_layout_expects() {
    let a = link_art();
    assert_eq!((a.port.w, a.port.h), (PORT_W, PORT_H));
    assert_eq!((a.port_light.w, a.port_light.h), (PORT_W, PORT_H));
    for plug in [&a.plug_host, &a.plug_join] {
        assert_eq!((plug.w, plug.h), (PLUG_W, PLUG_H));
    }
    assert_eq!((a.adapter.w, a.adapter.h), (ADAPTER_W, ADAPTER_H));
    for (i, (_, _, w, h)) in ARCS.iter().enumerate() {
        assert_eq!((a.arcs_right[i].w, a.arcs_right[i].h), (*w, *h));
        assert_eq!((a.arcs_left[i].w, a.arcs_left[i].h), (*w, *h));
    }
    assert_eq!((a.clicks.w, a.clicks.h), (CLICKS_W, CLICKS_H));
    assert_eq!((a.arrow_left.w, a.arrow_left.h), (ARROW_W, ARROW_H));
}

#[test]
fn the_host_plug_is_purple_and_the_joiners_gray() {
    let a = link_art();
    let (x, y) = (PLUG_TIP_X as u32 - 10, PLUG_H - 90);
    let host = px(&a.plug_host, x, y);
    let join = px(&a.plug_join, x, y);
    assert!(
        host[3] > 200 && join[3] > 200,
        "the housing is not where the layout puts it"
    );
    assert!(
        host[2] > host[1] + 30,
        "the host plug is not purple: {host:?}"
    );
    assert!(
        (join[0] as i32 - join[2] as i32).abs() < 12,
        "the joiner's plug is not gray: {join:?}"
    );
}

#[test]
fn the_tip_is_at_the_bottom_and_the_cable_fades_out_at_the_top() {
    let a = link_art();
    assert!(px(&a.plug_host, PLUG_TIP_X as u32, PLUG_H - 4)[3] > 200);
    assert!(px(&a.plug_host, PLUG_TIP_X as u32, 2)[3] < 40);
}

#[test]
fn the_port_retains_the_molded_alignment_mark_above_ext1() {
    let a = link_art();
    let mark:Vec<_>=(6..18).flat_map(|y|(352..368).map(move |x|(x,y)))
        .map(|(x,y)|px(&a.port,x,y)[0]).collect();
    assert!(mark.iter().max().unwrap()-mark.iter().min().unwrap()>15,"alignment triangle has no molded relief");
    assert!(a.port.rgba.chunks_exact(4).all(|p|p[3]==255),"connector occlusion plane has a hole");
}

#[test]
fn the_adapter_connector_matches_the_plug_tip() {
    let a = link_art();
    for row in 0..31 {
        let plug = span(&a.plug_host, PLUG_H - 31 + row);
        let adapter = span(&a.adapter, ADAPTER_H - 31 + row);
        assert_eq!(
            plug.1 - plug.0,
            adapter.1 - adapter.0,
            "connector row {row} is a different width"
        );
        let plug_mid = (plug.0 + plug.1) as f32 / 2.0;
        let adapter_mid = (adapter.0 + adapter.1) as f32 / 2.0;
        assert!(
            (plug_mid - (PLUG_TIP_X - 0.5)).abs() < 1.0,
            "the plug tip is off centre"
        );
        assert!(
            (adapter_mid - (ADAPTER_BASE_X - 0.5)).abs() < 1.0,
            "the adapter connector is off its seating point"
        );
    }
    assert_eq!(ADAPTER_H as f32 - ADAPTER_BASE_Y, 31.0);
}

fn span(face: &CartFace, y: u32) -> (u32, u32) {
    let mut xs = (0..face.w).filter(|x| px(face, *x, y)[3] > 16);
    let first = xs.next().expect("a connector row");
    let last = xs.next_back().unwrap_or(first);
    (first, last)
}

#[test]
fn the_adapter_label_carries_its_rows() {
    let a = link_art();
    let plate = |x: f32, y: f32| {
        px(
            &a.adapter,
            (ADAPTER_BASE_X + x * 1.75) as u32,
            (ADAPTER_BASE_Y + y * 1.75) as u32,
        )
    };
    let lit = (-30..30)
        .flat_map(|x| (-24..-6).map(move |y| (x as f32, y as f32)))
        .filter(|(x, y)| plate(*x, *y)[0] > 0x90)
        .count();
    assert!(
        lit > 40,
        "the plate has no label on it ({lit} light pixels)"
    );
}

#[test]
fn the_left_arcs_mirror_the_right() {
    let a = link_art();
    for i in 0..3 {
        let (r, l) = (&a.arcs_right[i], &a.arcs_left[i]);
        for y in 0..r.h {
            for x in 0..r.w {
                assert_eq!(px(r, x, y), px(l, r.w - 1 - x, y));
            }
        }
    }
}

#[test]
fn port_uses_each_shared_paint_finish_with_fine_grain_and_stable_embossing() {
    use slot_store::ShellColour;
    let art=link_art();
    for (n,colour) in ShellColour::FINISHES.into_iter().enumerate() {
        let f=&art.port_finishes[n];
        assert_eq!((f.w,f.h),(PORT_W,PORT_H));
        assert_eq!(f.rgba,shell_link_port(colour));
        let base=shell_rgb(colour);
        for c in 0..3 {assert!((px(f,100,50)[c] as i16-base[c] as i16).abs()<8);}
        let values:Vec<f32>=(40..250).map(|x|px(f,x,50)[0] as f32).collect();
        let mean=values.iter().sum::<f32>()/values.len() as f32;
        let rms=(values.iter().map(|v|(v-mean).powi(2)).sum::<f32>()/values.len() as f32).sqrt();
        assert!(rms<1.8,"oversized/coarse port grain: {rms}");
        let mark:Vec<_>=(27..46).flat_map(|y|(327..393).map(move |x|px(f,x,y)[0])).collect();
        assert!(mark.iter().max().unwrap()-mark.iter().min().unwrap()>20,"EXT.1 relief disappeared");
    }
}
