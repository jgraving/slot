//! Conservative cleanup of opaque black scanner backgrounds around rounded labels.
//! Runs on source pixels once, before resizing; the device caches the result.

/// Returns the number of pixels made transparent. Ambiguous images stay unchanged.
pub(crate) fn clear_corner_background(rgba: &mut [u8], w: u32, h: u32) -> usize {
    let (w, h) = (w as usize, h as usize);
    let n = w * h;
    if w.min(h) < 32 || rgba.len() != n * 4 || rgba.chunks_exact(4).any(|p| p[3] != 255) {
        return 0; // Respect existing alpha, including artist-supplied masks.
    }
    let black = |i: usize| rgba[i * 4..i * 4 + 3].iter().all(|&c| c <= 8);
    let corners = [0, w - 1, (h - 1) * w, n - 1];
    if corners.iter().any(|&i| !black(i)) {
        return 0;
    }
    // A few isolated black corner pixels or a one-pixel frame are not a scan cutout.
    let diagonal = (w.min(h) / 200).max(2);
    let diagonals = [
        diagonal * w + diagonal,
        diagonal * w + w - 1 - diagonal,
        (h - 1 - diagonal) * w + diagonal,
        (h - 1 - diagonal) * w + w - 1 - diagonal,
    ];
    if diagonals.iter().any(|&i| !black(i)) {
        return 0;
    }
    let rim = w.min(h) / 8;
    let budget = n / 12;
    let mut seen = vec![false; n];
    let mut queue = Vec::new();
    for i in corners {
        seen[i] = true;
        queue.push(i);
    }
    let mut at = 0;
    while at < queue.len() {
        if queue.len() > budget {
            return 0;
        }
        let i = queue[at];
        let (x, y) = (i % w, i / w);
        if x >= rim && x < w - rim && y >= rim && y < h - rim {
            return 0; // Black flowing into the artwork is not a safe background mask.
        }
        at += 1;
        for next in [
            (x > 0).then(|| i - 1),
            (x + 1 < w).then_some(i + 1),
            (y > 0).then(|| i - w),
            (y + 1 < h).then_some(i + w),
        ]
        .into_iter()
        .flatten()
        {
            if !seen[next] && black(next) {
                seen[next] = true;
                queue.push(next);
            }
        }
    }
    if diagonals.iter().any(|&i| !seen[i]) {
        return 0;
    }
    // Do not change any bytes until the complete candidate has passed every guard.
    for &i in &queue {
        rgba[i * 4 + 3] = 0;
    }
    queue.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan() -> Vec<u8> {
        let mut rgba = vec![255; 128 * 64 * 4];
        for y in 0usize..64 {
            for x in 0usize..128 {
                if x.min(127 - x) + y.min(63 - y) < 7 {
                    rgba[(y * 128 + x) * 4..][..3].fill(0);
                }
            }
        }
        rgba
    }

    #[test]
    fn clears_only_corner_connected_scanner_black_and_preserves_printed_black() {
        let mut rgba = scan();
        let ink = (30 * 128 + 60) * 4;
        rgba[ink..ink + 3].fill(0);
        let before = rgba.clone();
        assert_eq!(clear_corner_background(&mut rgba, 128, 64), 112);
        assert_eq!(&rgba[ink..ink + 4], &[0, 0, 0, 255]);
        for (old, new) in before.chunks_exact(4).zip(rgba.chunks_exact(4)) {
            assert_eq!(&old[..3], &new[..3]);
            if new[3] != 0 {
                assert_eq!(old, new);
            }
        }
    }

    #[test]
    fn ambiguous_black_artwork_is_left_byte_for_byte_unchanged() {
        let mut dark = vec![0; 128 * 64 * 4];
        for px in dark.chunks_exact_mut(4) {
            px[3] = 255;
        }
        let before = dark.clone();
        assert_eq!(clear_corner_background(&mut dark, 128, 64), 0);
        assert_eq!(dark, before);
        let mut spill = scan();
        for t in 0..32 {
            spill[(t * 128 + t) * 4..][..3].fill(0);
        }
        // Four-connected bridge to black artwork in the central protected area.
        for t in 0..31 {
            spill[(t * 128 + t + 1) * 4..][..3].fill(0);
        }
        let before = spill.clone();
        assert_eq!(clear_corner_background(&mut spill, 128, 64), 0);
        assert_eq!(spill, before);
    }

    #[test]
    fn existing_transparency_and_square_art_are_not_reinterpreted() {
        let mut transparent = scan();
        transparent[3] = 0;
        let before = transparent.clone();
        assert_eq!(clear_corner_background(&mut transparent, 128, 64), 0);
        assert_eq!(transparent, before);
        let mut square = vec![255; 128 * 64 * 4];
        for i in [0, 127, 63 * 128, 128 * 64 - 1] {
            square[i * 4..i * 4 + 3].fill(0);
        }
        let before = square.clone();
        assert_eq!(clear_corner_background(&mut square, 128, 64), 0);
        assert_eq!(square, before);
    }
}
