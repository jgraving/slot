use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use fontdue::{Font, FontSettings};

use crate::CartFace;

const SYMBOLS_TTF: &[u8] = include_bytes!("../assets/SymbolsNerdFontMono-Regular.ttf");

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Icon {
    Volume,
    VolumeZero,
    VolumeMuted,
    Brightness,
    BlueLight,
    FastForward,
    FastForwardLatched,
    Rewind,
    Alert,
    Charging,
    Headphones,
    HeadphonesMuted,
    Bluetooth,
    BluetoothMuted,
    Usb,
    UsbMuted,
}

impl Icon {
    pub const ALL: [Icon; 16] = [
        Icon::Volume,
        Icon::VolumeZero,
        Icon::VolumeMuted,
        Icon::Brightness,
        Icon::BlueLight,
        Icon::FastForward,
        Icon::FastForwardLatched,
        Icon::Rewind,
        Icon::Alert,
        Icon::Charging,
        Icon::Headphones,
        Icon::HeadphonesMuted,
        Icon::Bluetooth,
        Icon::BluetoothMuted,
        Icon::Usb,
        Icon::UsbMuted,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn glyph(self) -> char {
        match self {
            Icon::Volume => '\u{f028}',
            Icon::VolumeZero => '\u{f026}',
            Icon::VolumeMuted => '\u{f075f}',
            Icon::Brightness => '\u{f185}',
            Icon::BlueLight => '\u{f186}',
            Icon::FastForward => '\u{f06d2}',
            Icon::FastForwardLatched => '\u{f0211}',
            Icon::Rewind => '\u{f04a}',
            Icon::Alert => '\u{f0026}',
            Icon::Charging => '\u{f0e7}',
            Icon::Headphones => '\u{f025}',
            Icon::HeadphonesMuted => '\u{f07ce}',
            Icon::Bluetooth | Icon::BluetoothMuted => '\u{f294}',
            Icon::Usb | Icon::UsbMuted => '\u{f287}',
        }
    }
}

const HALO: [u8; 3] = [0x08, 0x08, 0x0a];
pub const HALO_PX: u32 = 1;

pub fn haloed(cov: &[u8], cw: u32, ch: u32, colour: [u8; 3]) -> CartFace {
    let pad = HALO_PX as usize;
    let (cw, ch) = (cw as usize, ch as usize);
    let (w, h) = (cw + 2 * pad, ch + 2 * pad);
    let at = |x: isize, y: isize| -> u8 {
        if x < 0 || y < 0 || x >= cw as isize || y >= ch as isize {
            0
        } else {
            cov[y as usize * cw + x as usize]
        }
    };

    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let (gx, gy) = (x as isize - pad as isize, y as isize - pad as isize);
            let ink = at(gx, gy);
            let mut halo = 0u8;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    halo = halo.max(at(gx + dx, gy + dy));
                }
            }
            if ink > 0 {
                let a = ink as u32;
                let inv = 255 - a;
                let mix = |c: u8, s: u8| ((c as u32 * a + s as u32 * inv) / 255) as u8;
                rgba.extend_from_slice(&[
                    mix(colour[0], HALO[0]),
                    mix(colour[1], HALO[1]),
                    mix(colour[2], HALO[2]),
                    ink.max(halo),
                ]);
            } else {
                rgba.extend_from_slice(&[HALO[0], HALO[1], HALO[2], halo]);
            }
        }
    }
    CartFace {
        rgba,
        w: w as u32,
        h: h as u32,
    }
}

pub fn icon_face(icon: Icon, px: f32, colour: [u8; 3]) -> CartFace {
    let Some(r) = raster(icon, px) else {
        return CartFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    if matches!(
        icon,
        Icon::Bluetooth | Icon::BluetoothMuted | Icon::HeadphonesMuted | Icon::Usb | Icon::UsbMuted
    ) {
        // Keep the common icon box, but no black halo on these fine strokes.
        let (w, h) = (r.w + 2 * HALO_PX, r.h + 2 * HALO_PX);
        let mut rgba = vec![0; (w * h * 4) as usize];
        for y in 0..r.h {
            for x in 0..r.w {
                let at = (((y + HALO_PX) * w + x + HALO_PX) * 4) as usize;
                rgba[at..at + 4].copy_from_slice(&[
                    colour[0],
                    colour[1],
                    colour[2],
                    r.cov[(y * r.w + x) as usize],
                ]);
            }
        }
        return CartFace { rgba, w, h };
    }
    haloed(&r.cov, r.w, r.h, colour)
}

pub fn icon_box(px: f32) -> (u32, u32) {
    match raster(Icon::Volume, px) {
        Some(r) => (r.w + 2 * HALO_PX, r.h + 2 * HALO_PX),
        None => (0, 0),
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Badge {
    Link,
    LinkBroken,
}

impl Badge {
    pub fn glyph(self) -> char {
        match self {
            Badge::Link => '\u{f0339}',
            Badge::LinkBroken => '\u{f033a}',
        }
    }
}

/// Antialiased foil artwork; rendered once when textures are loaded.
pub fn star_face(scale: u32, colour: [u8; 3]) -> CartFace {
    let w = 11 * scale.max(1);
    let svg = include_str!("../assets/favorite-star.svg").replace(
        "#f6c32c",
        &format!("#{:02x}{:02x}{:02x}", colour[0], colour[1], colour[2]),
    );
    // Average full pixel coverage before upload: GL_LINEAR only samples four
    // texels and cannot reliably minify a 4x texture's narrow sticker border.
    let hi = crate::art::render_svg(&svg, w * 4, w * 4).expect("embedded favorite star SVG");
    let rgba = prefilter_star(&hi, w);
    CartFace { rgba, w, h: w }
}

fn prefilter_star(src: &[u8], w: u32) -> Vec<u8> {
    let mut out = vec![0; (w * w * 4) as usize];
    for y in 0..w {
        for x in 0..w {
            let mut alpha = 0u32;
            let mut colour = [0u32; 3];
            for dy in 0..4 {
                for dx in 0..4 {
                    let i = (((y * 4 + dy) * w * 4 + x * 4 + dx) * 4) as usize;
                    let a = src[i + 3] as u32;
                    alpha += a;
                    for c in 0..3 {
                        colour[c] += src[i + c] as u32 * a;
                    }
                }
            }
            let i = ((y * w + x) * 4) as usize;
            if alpha > 0 {
                for c in 0..3 {
                    out[i + c] = ((colour[c] + alpha / 2) / alpha) as u8;
                }
            }
            out[i + 3] = ((alpha + 8) / 16) as u8;
        }
    }
    // Preserve edge colour when linear filtering blends with transparent pixels.
    let pixels = out.clone();
    for y in 0..w as i32 {
        for x in 0..w as i32 {
            let i = ((y * w as i32 + x) * 4) as usize;
            if pixels[i + 3] != 0 {
                continue;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (xx, yy) = (x + dx, y + dy);
                if xx < 0 || yy < 0 || xx >= w as i32 || yy >= w as i32 {
                    continue;
                }
                let j = ((yy * w as i32 + xx) * 4) as usize;
                if pixels[j + 3] > 0 {
                    out[i..i + 3].copy_from_slice(&pixels[j..j + 3]);
                    break;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod favorite_filter_tests {
    #[test]
    fn averages_coverage_without_darkening_translucent_edges() {
        let mut src = vec![0u8; 4 * 4 * 4];
        src[..4].copy_from_slice(&[240, 220, 180, 255]);
        assert_eq!(super::prefilter_star(&src, 1), vec![240, 220, 180, 16]);
        for p in src.chunks_exact_mut(4) {
            p.copy_from_slice(&[170, 130, 65, 255]);
        }
        assert_eq!(super::prefilter_star(&src, 1), vec![170, 130, 65, 255]);
    }
}

pub fn badge_face(badge: Badge, px: f32, colour: [u8; 3]) -> CartFace {
    let Some(font) = symbols_font() else {
        return CartFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let f = frame(font, px);
    let (m, cov) = font.rasterize(badge.glyph(), px);
    let x0 = m.xmin - f.left;
    let y0 = f.top - (m.ymin + m.height as i32);
    let mut out = vec![0u8; (f.w * f.h) as usize];
    for gy in 0..m.height {
        for gx in 0..m.width {
            let (dx, dy) = (x0 + gx as i32, y0 + gy as i32);
            if dx < 0 || dy < 0 || dx >= f.w as i32 || dy >= f.h as i32 {
                continue;
            }
            out[(dy as u32 * f.w + dx as u32) as usize] = cov[gy * m.width + gx];
        }
    }
    haloed(&out, f.w, f.h, colour)
}

struct Raster {
    cov: Vec<u8>,
    w: u32,
    h: u32,
}

type Cache = Mutex<HashMap<(Icon, u32), Arc<Raster>>>;

fn raster(icon: Icon, px: f32) -> Option<Arc<Raster>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    let key = (icon, px.to_bits());
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(r) = cache.get(&key) {
        return Some(r.clone());
    }
    let r = Arc::new(rasterise(icon, px)?);
    cache.insert(key, r.clone());
    Some(r)
}

fn rasterise(icon: Icon, px: f32) -> Option<Raster> {
    let font = symbols_font()?;
    let f = frame(font, px);
    if matches!(icon, Icon::BluetoothMuted | Icon::HeadphonesMuted) {
        return muted_audio_raster(icon, px);
    }
    if matches!(
        icon,
        Icon::Bluetooth | Icon::BluetoothMuted | Icon::Usb | Icon::UsbMuted
    ) {
        return Some(audio_raster(icon, f.w, f.h, px));
    }

    let (m, cov) = font.rasterize(icon.glyph(), px);
    let x0 = m.xmin - f.left;
    let y0 = f.top - (m.ymin + m.height as i32);

    let mut out = vec![0u8; (f.w * f.h) as usize];
    for gy in 0..m.height {
        for gx in 0..m.width {
            let (dx, dy) = ((x0 + gx as i32) as u32, (y0 + gy as i32) as u32);
            out[(dy * f.w + dx) as usize] = cov[gy * m.width + gx];
        }
    }
    Some(Raster {
        cov: out,
        w: f.w,
        h: f.h,
    })
}

// Keep the device symbol full-size and copy the speaker's exact mute mark.
// Call rasterise directly: the cache mutex is already held here.
fn muted_audio_raster(icon: Icon, px: f32) -> Option<Raster> {
    let speaker = rasterise(Icon::VolumeMuted, px)?;
    let base = rasterise(
        if icon == Icon::BluetoothMuted {
            Icon::Bluetooth
        } else {
            Icon::Headphones
        },
        px,
    )?;
    let font = symbols_font()?;
    let f = frame(font, px);
    let m = font.metrics(Icon::VolumeMuted.glyph(), px);
    // Separate font contours: speaker ends at 972/2048, X starts at 1188/2048.
    let split = (m.xmin - f.left) as u32 + (m.width as u32 * 1080 / 2048);
    let bounds = |r: &Raster, start: u32| {
        let points: Vec<_> = r
            .cov
            .iter()
            .enumerate()
            .filter(|(i, a)| **a > 0 && *i as u32 % r.w >= start)
            .map(|(i, _)| (i as u32 % r.w, i as u32 / r.w))
            .collect();
        (
            points.iter().map(|p| p.0).min().unwrap(),
            points.iter().map(|p| p.0).max().unwrap(),
            points.iter().map(|p| p.1).min().unwrap(),
            points.iter().map(|p| p.1).max().unwrap(),
        )
    };
    let (bl, br, bt, bb) = bounds(&base, 0);
    let (ml, mr, mt, mb) = bounds(&speaker, split);
    let gap = (px / 12.).round().max(1.) as u32;
    let content_w = br - bl + 1 + gap + mr - ml + 1;
    let w = base.w.max(content_w + 2);
    let mut out = Raster {
        cov: vec![0; (w * base.h) as usize],
        w,
        h: base.h,
    };
    let x0 = (w - content_w) / 2;
    let mark_y = ((bt + bb) as i32 - (mt + mb) as i32) / 2;
    for y in 0..base.h {
        for x in bl..=br {
            out.cov[(y * w + x0 + x - bl) as usize] = base.cov[(y * base.w + x) as usize];
        }
        let dy = y as i32 + mark_y;
        if dy >= 0 && dy < out.h as i32 {
            for x in ml..=mr {
                out.cov[(dy as u32 * w + x0 + br - bl + 1 + gap + x - ml) as usize] =
                    speaker.cov[(y * speaker.w + x) as usize];
            }
        }
    }
    Some(out)
}

pub(crate) fn glyph_box(icon: Icon, px: f32) -> (u32, u32) {
    raster(icon, px)
        .map(|r| (r.w + 2 * HALO_PX, r.h + 2 * HALO_PX))
        .unwrap_or((0, 0))
}

// Rasterize in a square design space with uniform scaling and round strokes.
// Supersampling avoids the gaps and horizontal bias of integer line stepping.
fn audio_raster(icon: Icon, w: u32, h: u32, px: f32) -> Raster {
    let usb = matches!(icon, Icon::Usb | Icon::UsbMuted);
    let muted = matches!(icon, Icon::BluetoothMuted | Icon::UsbMuted);
    let mut segments: Vec<((f32, f32), (f32, f32))> = if usb {
        vec![
            ((12., 20.), (12., 3.)),
            ((12., 3.), (9.5, 6.)),
            ((12., 3.), (14.5, 6.)),
            ((12., 14.), (7., 10.)),
            ((7., 10.), (7., 7.)),
            ((12., 17.), (17., 13.)),
            ((17., 13.), (17., 9.)),
            ((15.5, 7.), (18.5, 7.)),
            ((18.5, 7.), (18.5, 10.)),
            ((18.5, 10.), (15.5, 10.)),
            ((15.5, 10.), (15.5, 7.)),
        ]
    } else {
        vec![
            ((10., 3.), (10., 21.)),
            ((10., 3.), (16., 8.)),
            ((16., 8.), (6., 16.)),
            ((10., 21.), (16., 16.)),
            ((16., 16.), (6., 8.)),
        ]
    };
    if muted {
        segments.extend([((19., 17.), (23., 21.)), ((23., 17.), (19., 21.))]);
    }
    let scale = (px / 24.).min(w as f32 / 26.).min(h as f32 / 26.);
    let ox = (w as f32 - 24. * scale) / 2.;
    let oy = (h as f32 - 24. * scale) / 2.;
    let mut cov = vec![0; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let mut covered = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let qx = (x as f32 + (sx as f32 + 0.5) / 4. - ox) / scale;
                    let qy = (y as f32 + (sy as f32 + 0.5) / 4. - oy) / scale;
                    let stroke = segments.iter().any(|&((ax, ay), (bx, by))| {
                        let (dx, dy) = (bx - ax, by - ay);
                        let t =
                            (((qx - ax) * dx + (qy - ay) * dy) / (dx * dx + dy * dy)).clamp(0., 1.);
                        (qx - ax - t * dx).powi(2) + (qy - ay - t * dy).powi(2) <= 0.85 * 0.85
                    });
                    let dot = usb
                        && [(7., 7., 1.5), (12., 20., 1.8)]
                            .iter()
                            .any(|&(cx, cy, r)| (qx - cx).powi(2) + (qy - cy).powi(2) <= r * r);
                    covered += u32::from(stroke || dot);
                }
            }
            cov[(y * w + x) as usize] = ((covered * 255 + 8) / 16) as u8;
        }
    }
    Raster { cov, w, h }
}

struct Frame {
    w: u32,
    h: u32,
    left: i32,
    top: i32,
}

fn frame(font: &Font, px: f32) -> Frame {
    let (mut left, mut right, mut top, mut bottom) = (i32::MAX, i32::MIN, i32::MIN, i32::MAX);
    for icon in Icon::ALL {
        let m = font.metrics(icon.glyph(), px);
        left = left.min(m.xmin);
        right = right.max((m.xmin + m.width as i32).max(m.advance_width.ceil() as i32));
        top = top.max(m.ymin + m.height as i32);
        bottom = bottom.min(m.ymin);
    }
    Frame {
        w: (right - left).max(1) as u32,
        h: (top - bottom).max(1) as u32,
        left,
        top,
    }
}

pub fn warm_fonts() {
    let _ = symbols_font();
    let _ = crate::text::label_font();
}

pub(crate) fn symbols_font() -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    FONT.get_or_init(|| Font::from_bytes(SYMBOLS_TTF, FontSettings::default()).ok())
        .as_ref()
}

#[cfg(test)]
mod mute_tests {
    use super::*;
    fn ink(r: &Raster, start: u32, end: u32) -> (Vec<u8>, u32, u32) {
        let points: Vec<_> = (start..end)
            .flat_map(|x| (0..r.h).map(move |y| (x, y)))
            .filter(|&(x, y)| r.cov[(y * r.w + x) as usize] > 0)
            .collect();
        let l = points.iter().map(|p| p.0).min().unwrap();
        let rr = points.iter().map(|p| p.0).max().unwrap();
        let t = points.iter().map(|p| p.1).min().unwrap();
        let b = points.iter().map(|p| p.1).max().unwrap();
        (
            (t..=b)
                .flat_map(|y| (l..=rr).map(move |x| r.cov[(y * r.w + x) as usize]))
                .collect(),
            t,
            b,
        )
    }
    fn gap(r: &Raster) -> u32 {
        (0..r.w)
            .rev()
            .find(|&x| {
                (0..r.h).all(|y| r.cov[(y * r.w + x) as usize] == 0)
                    && (x + 1..r.w).any(|rx| (0..r.h).any(|y| r.cov[(y * r.w + rx) as usize] > 0))
            })
            .unwrap()
    }
    #[test]
    fn mute_marks_match_speaker_without_shrinking_device_symbols() {
        for px in [20., 24., 32., 40.] {
            let speaker = rasterise(Icon::VolumeMuted, px).unwrap();
            let mark = ink(&speaker, gap(&speaker) + 1, speaker.w).0;
            for (muted, base) in [
                (Icon::BluetoothMuted, Icon::Bluetooth),
                (Icon::HeadphonesMuted, Icon::Headphones),
            ] {
                let r = rasterise(muted, px).unwrap();
                let base = rasterise(base, px).unwrap();
                let split = gap(&r);
                let left = ink(&r, 0, split);
                let right = ink(&r, split + 1, r.w);
                assert_eq!(right.0, mark);
                assert_eq!(left.0, ink(&base, 0, base.w).0);
                assert!(((left.1 + left.2) as i32 - (right.1 + right.2) as i32).abs() <= 2);
            }
        }
    }
}

/// Device rows use equal ink heights, independent of each glyph's font padding.
pub fn sound_device_icon(icon: Icon, ink: [u8; 3]) -> crate::UndoFace {
    let bounds = |r: &Raster| {
        let points: Vec<_> = r
            .cov
            .iter()
            .enumerate()
            .filter(|(_, a)| **a > 0)
            .map(|(i, _)| (i as u32 % r.w, i as u32 / r.w))
            .collect();
        (
            points.iter().map(|p| p.0).min().unwrap_or(0),
            points.iter().map(|p| p.0).max().unwrap_or(0),
            points.iter().map(|p| p.1).min().unwrap_or(0),
            points.iter().map(|p| p.1).max().unwrap_or(0),
        )
    };
    let Some(reference) = raster(Icon::Headphones, 24.) else {
        return crate::UndoFace {
            rgba: vec![],
            w: 0,
            h: 0,
        };
    };
    let Some(source) = raster(icon, 96.) else {
        return crate::UndoFace {
            rgba: vec![],
            w: 0,
            h: 0,
        };
    };
    let (_, _, rt, rb) = bounds(&reference);
    let (l, r, t, b) = bounds(&source);
    let h = rb - rt + 1;
    let scale = (b - t + 1) as f32 / h as f32;
    let w = ((r - l + 1) as f32 / scale).ceil() as u32;
    let mut rgba = vec![0; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let mut alpha = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let bx = l + ((x as f32 + (sx as f32 + 0.5) / 4.) * scale) as u32;
                    let by = t + ((y as f32 + (sy as f32 + 0.5) / 4.) * scale) as u32;
                    if bx <= r && by <= b {
                        alpha += source.cov[(by * source.w + bx) as usize] as u32;
                    }
                }
            }
            let at = ((y * w + x) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&[ink[0], ink[1], ink[2], ((alpha + 8) / 16) as u8]);
        }
    }
    crate::UndoFace { rgba, w, h }
}
