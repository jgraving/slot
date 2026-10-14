//! Small controller-button SVGs for the optional network access code.
use crate::UndoFace;

pub fn button_code_face(code: &[u8]) -> UndoFace {
    let mut svg = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="320" height="64" viewBox="0 0 320 64">"#,
    );
    let glyphs = [
        "M24 41L32 23L40 41M27 35H37",
        "M25 23V41H34Q44 41 39 32Q43 23 34 23ZM25 32H35",
        "M24 23L40 41M40 23L24 41",
        "M24 23L32 32L40 23M32 32V41",
        "M32 43V22M23 31L32 22L41 31",
        "M32 21V42M23 33L32 42L41 33",
        "M43 32H22M31 23L22 32L31 41",
        "M21 32H42M33 23L42 32L33 41",
    ];
    for i in 0..4 {
        svg.push_str(&format!(r#"<g transform="translate({} 0)">"#, i * 80));
        if let Some(path) = code.get(i).and_then(|b| glyphs.get(*b as usize)) {
            svg.push_str(&format!(r##"<circle cx="32" cy="32" r="27" fill="#34343b" stroke="#eee3c9" stroke-width="2"/><path d="{path}" fill="none" stroke="#eee3c9" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>"##));
        } else {
            svg.push_str(r##"<circle cx="32" cy="32" r="25" fill="none" stroke="#77777e" stroke-width="2" stroke-dasharray="3 5"/>"##);
        }
        svg.push_str("</g>");
    }
    svg.push_str("</svg>");
    UndoFace {
        rgba: crate::art::render_svg(&svg, 320, 64).unwrap_or_else(|| vec![0; 320 * 64 * 4]),
        w: 320,
        h: 64,
    }
}
