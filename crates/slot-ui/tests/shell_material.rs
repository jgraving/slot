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
        let at = ((12*SLOT_W+80)*4) as usize;
        let base = shell_rgb(colour);
        for c in 0..3 { assert!((faces[1][at+c] as i16-base[c] as i16).abs() < 20); }
    }
}

#[test]
fn paint_has_fine_low_contrast_grain_instead_of_deep_pebbles() {
    let front = shell_slot_faces(ShellColour::Platinum)[1];
    let values: Vec<f32> = (40..180).map(|x| front[((14*SLOT_W+x)*4) as usize] as f32).collect();
    let avg = values.iter().sum::<f32>()/values.len() as f32;
    let rms = (values.iter().map(|v| (v-avg).powi(2)).sum::<f32>()/values.len() as f32).sqrt();
    assert!((0.3..4.0).contains(&rms), "grain RMS {rms}");
}

#[test]
fn both_insertion_fasteners_keep_their_relief_in_every_finish() {
    for colour in ShellColour::FINISHES {
        let back = shell_slot_faces(colour)[0];
        for centre in [300, 420] {
            let mut deepest = 0i16;
            for y in 5..24 {
                for x in centre-7..centre+8 {
                    let mark = back[((y*SLOT_W+x)*4) as usize] as i16;
                    let surround = back[((y*SLOT_W+centre-20)*4) as usize] as i16;
                    deepest = deepest.max(surround-mark);
                }
            }
            assert!(deepest >= 15, "{colour:?} fastener at {centre} lost its depth");
        }
    }
}
