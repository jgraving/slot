fn at(x: usize, y: usize) -> usize {
    let x = (360.0+(x as f32-360.0)*308.0/254.0).round() as usize;
    let y = (5.0+y as f32*1.05).round() as usize;
    (y*720+x)*4
}
use slot_store::ShellColour;
use slot_ui::{power_switch_faces_for, shell_rgb, shell_slot_faces, SLOT_W, SLOT_H};

#[test]
fn painted_shells_preserve_geometry_and_leave_the_switch_cavity_neutral() {
    let original = shell_slot_faces(ShellColour::Graphite);
    let [dark, slider] = power_switch_faces_for(ShellColour::Graphite);
    for colour in ShellColour::FINISHES {
        let faces = shell_slot_faces(colour);
        for layer in 0..2 {
            assert_eq!(faces[layer].len(), (SLOT_W*SLOT_H*4) as usize);
            assert!(faces[layer].chunks_exact(4).zip(original[layer].chunks_exact(4)).all(|(a,b)| a[3] == b[3]));
        }
        let [switch, handle] = power_switch_faces_for(colour);
        assert_eq!(handle.rgba, slider.rgba);
        let cavity = ((110*600+230)*4) as usize;
        assert_eq!(&switch.rgba[cavity..cavity+4], &dark.rgba[cavity..cavity+4]);
        let at = ((66*SLOT_W+240)*4) as usize;
        let base = shell_rgb(colour);
        for c in 0..3 { assert!((faces[1][at+c] as i16-base[c] as i16).abs() < 20); }
    }
}

#[test]
fn paint_has_fine_low_contrast_grain_instead_of_deep_pebbles() {
    let front = shell_slot_faces(ShellColour::Platinum)[1];
    let values: Vec<f32> = (230..350).map(|x| front[((66*SLOT_W+x)*4) as usize] as f32).collect();
    let avg = values.iter().sum::<f32>()/values.len() as f32;
    let rms = (values.iter().map(|v| (v-avg).powi(2)).sum::<f32>()/values.len() as f32).sqrt();
    assert!((0.3..4.0).contains(&rms), "grain RMS {rms}");
}

#[test]
fn svg_slot_keeps_an_open_throat_and_shallow_bowed_lip() {
    for colour in ShellColour::FINISHES {
        let [back, front] = shell_slot_faces(colour);
        let at = at;
        // Cartridge passes through the transparent centre; the front then hides its foot.
        assert_eq!(front[at(360, 25)+3], 0);
        assert_eq!(front[at(360, 46)+3], 255);
        let metal=at(360,30);
        assert!((115..210).contains(&back[metal]),"liner must read as silver metal");
        assert_eq!(&back[metal..metal+4],&shell_slot_faces(ShellColour::Graphite)[0][metal..metal+4],
            "metal liner must not inherit the shell paint color");
        // Real molded ribs are painted; the small metal screw stays neutral across finishes.
        assert!(back[at(284, 10)+3] > 250);
        let screw = at(359, 9);
        assert_eq!(&back[screw..screw+4], &shell_slot_faces(ShellColour::Graphite)[0][screw..screw+4]);
    }
}

#[test]
fn lip_has_flat_shoulders_and_a_flat_centre_with_curved_transitions() {
    let front = shell_slot_faces(ShellColour::Platinum)[1];
    let top = |x: usize| (0..58).find(|y| front[at(x,*y)+3] > 200).unwrap();
    assert_eq!(top(210),top(220));
    assert_eq!(top(340),top(360));
    assert_eq!(top(360),top(380));
    assert!(top(275)>top(220) && top(275)<top(340));

}

#[test]
fn screw_edges_never_leak_the_svg_red_paint_marker() {
    for layer in shell_slot_faces(ShellColour::Graphite) {
        for pixel in layer.chunks_exact(4).filter(|p|p[3]>0) {
            assert!(pixel[0] <= pixel[1]+2, "red marker leaked at a metal/paint boundary: {pixel:?}");
        }
    }
}

#[test]
fn lower_shell_stays_flat_without_a_bulge_or_cast_shadow() {
    for colour in ShellColour::FINISHES {
        let front=shell_slot_faces(colour)[1];let base=shell_rgb(colour);
        for x in 250..471 { for y in 44..58 {
            let p=&front[at(x,y)..][..4];
            assert_eq!(p[3],255);
            for c in 0..3 { assert!((p[c] as i16-base[c] as i16).abs()<15,
                "bulge or shadow below slot at {x},{y}: {p:?}"); }
        }}
        assert_eq!(front[at(360,30)+3],0);
    }
}

#[test]
fn sp_body_has_rounded_shoulders_and_recessed_corner_screws() {
    for colour in ShellColour::FINISHES {
        let [back,front]=shell_slot_faces(colour);
        for x in [40usize,100,620,680] {
            for y in 0..72 {
                assert_eq!(back[(y*720+x)*4+3],0);
                assert_eq!(front[(y*720+x)*4+3],0);
            }
        }
        assert_eq!(front[(2*720+123)*4+3],0,"shoulder is square");
        assert_eq!(front[(20*720+140)*4+3],255);
        let mean=|x0:usize,x1:usize| {
            let mut sum=0.0;let mut n=0;
            for y in 44..60 { for x in x0..x1 {sum+=front[(y*720+x)*4] as f32;n+=1;} }
            sum/n as f32
        };
        assert!((mean(193,197)-mean(198,202)).abs()<4.0,
            "vertical seam between shoulder and flat deck");
        assert!((mean(523,527)-mean(518,522)).abs()<4.0,
            "vertical seam between right shoulder and flat deck");
        for x in [158,562] {
            assert!(front[(32*720+x)*4]<80,"corner screw well missing");
            assert!(front[(46*720+x)*4]>80,"metal screw head missing");
        }
    }
}

#[test]
fn power_labels_and_both_dots_have_readable_embossed_relief() {
    for colour in ShellColour::FINISHES {
        let face=power_switch_faces_for(colour)[0].rgba.clone();
        for (x0,x1) in [(200,256),(338,375),(257,278),(313,334)] {
            let mut lo=255u8;let mut hi=0u8;
            for y in 165..191 { for x in x0..x1 {
                let v=face[(y*600+x)*4];lo=lo.min(v);hi=hi.max(v);
            }}
            assert!(hi-lo>=65,"{colour:?} marking at {x0}: relief {}",hi-lo);
        }
    }
}

#[test]
fn slot_end_returns_have_recessed_volume_inside_the_aperture() {
    for colour in ShellColour::FINISHES {
        let [back,front]=shell_slot_faces(colour);let at=at;
        assert_eq!(back[at(239,10)+3],255);
        assert_eq!(front[at(239,10)+3],0,"grip must pass in front of the left end wall");
        assert_eq!(back[at(481,10)+3],255);
        assert_eq!(front[at(481,10)+3],0,"grip must pass in front of the right end wall");
        assert_eq!(front[at(255,10)+3],0);
        assert_eq!(front[at(465,10)+3],0);
        let mean=|x0:usize,x1:usize| {
            let values:Vec<_>=(18..26).flat_map(|y|(x0..x1).map(move |x|back[(y*720+x)*4] as f32)).collect();
            values.iter().sum::<f32>()/values.len() as f32
        };
        assert!(mean(175,185)>mean(195,205)+3.0,"left rolled return lost its depth");
        assert!(mean(535,545)>mean(515,525)+3.0,"right rolled return lost its depth");
    }
}

#[test]
fn enlarged_slot_keeps_the_central_screw_head_circular() {
    let a=shell_slot_faces(ShellColour::Graphite)[0];
    let b=shell_slot_faces(ShellColour::PearlPink)[0];
    let mut xs=Vec::new();let mut ys=Vec::new();
    // The untinted metal head is identical across paint finishes. Restrict the
    // window above the liner so its bounds measure the actual fastener.
    for y in 4..31 { for x in 342..379 {
        let at=(y*720+x)*4;
        if a[at..at+3]==b[at..at+3] { xs.push(x);ys.push(y); }
    }}
    let w=xs.iter().max().unwrap()-xs.iter().min().unwrap()+1;
    let h=ys.iter().max().unwrap()-ys.iter().min().unwrap()+1;
    assert!(w.abs_diff(h)<=1,"central metal screw is warped: {w} by {h}");
    assert!(w>=17 && h>=17,"fastener disappeared");
    let centre_y=(*ys.iter().min().unwrap()+*ys.iter().max().unwrap()+1) as f32/2.0;
    assert!((centre_y-(1.0+31.25)/2.0).abs()<=0.5,
        "screw must be vertically centered in the upper plastic strip: {centre_y}");
    let mut dark=Vec::new();
    for y in 8..24 { for x in 352..368 {
        let dx=x as f32+0.5-360.0;let dy=y as f32+0.5-16.125;
        if dx*dx+dy*dy<49.0 && a[(y*720+x)*4]<85 { dark.push((x as f32+0.5,y as f32+0.5)); }
    }}
    let cx=dark.iter().map(|p|p.0).sum::<f32>()/dark.len() as f32;
    let cy=dark.iter().map(|p|p.1).sum::<f32>()/dark.len() as f32;
    assert!((cx-360.0).abs()<0.5 && (cy-16.125).abs()<0.5,
        "driver recess is off-center: {cx},{cy}");
}

#[test]
fn visible_gba_grip_meets_the_aperture_and_gb_bodies_match_its_lower_body() {
    use slot_store::{Cart, Platform};
    use slot_ui::{cart_face, seated_box};
    let [back, _] = shell_slot_faces(ShellColour::Platinum);
    let [other, _] = shell_slot_faces(ShellColour::PearlPink);
    // Untinted liner bounds measure the actual opening, independent of painted relief.
    let metal: Vec<_> = (180..540).filter(|x| {
        let i=(34*720+x)*4;
        back[i..i+3]==other[i..i+3]
    }).collect();
    let left=*metal.first().unwrap() as f32;
    let right=(*metal.last().unwrap()+1) as f32;
    let face = |platform| cart_face(&Cart { platform, stem: "Test".into(),
        title: "Test".into(), rom: "test.gba".into(), code: String::new(),
        shell: None, label: None });
    let gba=face(Platform::Gba);
    let span = |f: &slot_ui::CartFace, y: u32| {
        let xs: Vec<_>=(0..f.w).filter(|x|f.rgba[((y*f.w+x)*4+3) as usize]>200).collect();
        (*xs.first().unwrap() as f32, (*xs.last().unwrap()+1) as f32)
    };
    let scale=seated_box(Platform::Gba).0 as f32/gba.w as f32;
    let grip=(0..gba.h/5).map(|y|span(&gba,y)).max_by(|a,b| (a.1-a.0).total_cmp(&(b.1-b.0))).unwrap();
    let cart_left=360.0-gba.w as f32*scale/2.0+grip.0*scale;
    let cart_right=360.0-gba.w as f32*scale/2.0+grip.1*scale;
    assert!((cart_left-left).abs()<=1.0 && (cart_right-right).abs()<=1.0,
        "visible GBA grip {cart_left}..{cart_right} leaves a gap in aperture {left}..{right}");
    let body=span(&gba,gba.h/2);
    let body_width=(body.1-body.0)*scale;
    for platform in [Platform::Gb,Platform::Gbc] {
        let f=face(platform);let body=span(&f,f.h/2);
        let width=(body.1-body.0)*seated_box(platform).0 as f32/f.w as f32;
        assert!((width-body_width).abs()<1.0,"{platform:?} body {width} versus GBA {body_width}");
    }
}

#[test]
fn shoulder_and_rib_paint_never_crosses_the_metal_boundary() {
    let a=shell_slot_faces(ShellColour::Platinum)[0];
    let b=shell_slot_faces(ShellColour::PearlPink)[0];
    for y in 32..46 { for x in 207..513 {
        let i=(y*720+x)*4;
        assert_eq!(&a[i..i+4],&b[i..i+4],"paint spills onto metal at {x},{y}");
    }}
}
