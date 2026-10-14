use crate::UndoFace;

pub fn network_indicator_face() -> UndoFace {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20"><path d="M2 7Q10 0 18 7M5 10Q10 5 15 10M8 13Q10 11 12 13" fill="none" stroke="#f6f4ef" stroke-width="1.8" stroke-linecap="round"/><circle cx="10" cy="16" r="1.3" fill="#f6f4ef"/></svg>"##;
    UndoFace {
        rgba: crate::art::render_svg(svg, 20, 20).unwrap_or_else(|| vec![0; 20 * 20 * 4]),
        w: 20,
        h: 20,
    }
}
