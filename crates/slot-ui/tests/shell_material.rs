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
        let at = ((56*SLOT_W+80)*4) as usize;
        let base = shell_rgb(colour);
        for c in 0..3 { assert!((faces[1][at+c] as i16-base[c] as i16).abs() < 20); }
    }
}

#[test]
fn paint_has_fine_low_contrast_grain_instead_of_deep_pebbles() {
    let front = shell_slot_faces(ShellColour::Platinum)[1];
    let values: Vec<f32> = (40..180).map(|x| front[((56*SLOT_W+x)*4) as usize] as f32).collect();
    let avg = values.iter().sum::<f32>()/values.len() as f32;
    let rms = (values.iter().map(|v| (v-avg).powi(2)).sum::<f32>()/values.len() as f32).sqrt();
    assert!((0.3..4.0).contains(&rms), "grain RMS {rms}");
}

#[test]
fn svg_slot_keeps_an_open_throat_and_shallow_bowed_lip() {
    for colour in ShellColour::FINISHES {
        let [back, front] = shell_slot_faces(colour);
        let at = |x: usize, y: usize| (y * SLOT_W as usize + x) * 4;
        // Cartridge passes through the transparent centre; the front then hides its foot.
        assert_eq!(front[at(360, 25)+3], 0);
        assert_eq!(front[at(360, 46)+3], 255);
        assert!(back[at(360, 27)] < 90);
        // Real molded ribs are painted; the small metal screw stays neutral across finishes.
        assert!(back[at(284, 10)+3] > 250);
        let screw = at(359, 9);
        assert_eq!(&back[screw..screw+4], &shell_slot_faces(ShellColour::Graphite)[0][screw..screw+4]);
    }
}

#[test]
fn lip_has_flat_shoulders_and_a_flat_centre_with_curved_transitions() {
    let front = shell_slot_faces(ShellColour::Platinum)[1];
    let top = |x: usize| (0..58).find(|y| front[(y*720+x)*4+3] > 200).unwrap();
    assert_eq!(top(100),top(180));
    assert_eq!(top(340),top(360));
    assert_eq!(top(360),top(380));
    assert!(top(275)>top(180) && top(275)<top(340));
    // Removed end screws must be ordinary painted surface.
    for x in [211,509] { assert!(front[(56*720+x)*4] > 150); }
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
fn slot_lip_has_thickness_and_blends_into_the_unbounded_shell() {
    let front = shell_slot_faces(ShellColour::Platinum)[1];
    let at = |x: usize,y: usize| (y*720+x)*4;
    // Plain shell continues to either side; the centre remains an opening.
    assert!(front[at(215,10)+3]>250);
    assert!(front[at(505,10)+3]>250);
    assert_eq!(front[at(360,30)+3],0);
    // Forward crest, shaded front face, and lower deck have separate lighting.
    assert!(front[at(360,43)]>front[at(360,49)]+12);
    assert!(front[at(360,56)]>front[at(360,50)]+12);
}

#[test]
fn shell_has_no_device_shoulders_or_outer_seams() {
    for colour in ShellColour::FINISHES {
        let front=shell_slot_faces(colour)[1]; let base=shell_rgb(colour);
        for x in [40usize,100,200,215,505,520,620,680] {
            for y in 0..58 {
                let p=&front[(y*720+x)*4..][..4];
                assert_eq!(p[3],255,"outer shell interrupted at {x},{y}");
                for c in 0..3 { assert!((p[c] as i16-base[c] as i16).abs()<15,
                    "outer outline or shoulder at {x},{y}: {p:?}"); }
            }
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
