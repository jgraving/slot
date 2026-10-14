#[test]
fn svg_button_faces_are_distinct_and_nonempty() {
    let empty = slot_ui::button_code_face(&[]);
    assert_eq!((empty.w, empty.h), (320, 64));
    let mut faces = Vec::new();
    for button in 0..8 {
        let face = slot_ui::button_code_face(&[button]);
        assert!(face.rgba.chunks_exact(4).any(|p| p[3] > 0));
        assert_ne!(face.rgba, empty.rgba);
        assert!(!faces.contains(&face.rgba));
        faces.push(face.rgba);
    }
}
