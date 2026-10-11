const INSERT: &[u8] = include_bytes!("../../assets/insert.pcm");
const EJECT: &[u8] = include_bytes!("../../assets/eject.pcm");
const ASSET_HZ: f32 = 48_000.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Sfx {
    Insert,
    Eject,
    Cartridge(u8),
}

impl Sfx {
    pub fn lead(self) -> f32 {
        match self {
            Sfx::Insert => 0.097,
            Sfx::Eject => 0.021,
            _ => 0.0,
        }
    }

    pub fn tail(self) -> f32 {
        if let Self::Cartridge(_) = self {
            return 0.054;
        }
        let pcm = match self {
            Sfx::Insert => INSERT,
            Sfx::Eject => EJECT,
            _ => unreachable!(),
        };
        pcm.len() as f32 / 2.0 / ASSET_HZ - self.lead()
    }

    pub fn render(self, sample_rate: u32) -> Vec<i16> {
        if !matches!(self, Self::Insert | Self::Eject) {
            return tactile(self, sample_rate);
        }
        let pcm = match self {
            Sfx::Insert => INSERT,
            Sfx::Eject => EJECT,
            _ => unreachable!(),
        };
        let mut out = Vec::with_capacity(pcm.len());
        for v in resampled(pcm, sample_rate) {
            let s = v.clamp(-32768.0, 32767.0) as i16;
            out.push(s);
            out.push(s);
        }
        out
    }
}

fn resampled(pcm: &[u8], sample_rate: u32) -> Vec<f32> {
    let src: Vec<f32> = pcm
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32)
        .collect();
    if sample_rate == ASSET_HZ as u32 || src.is_empty() {
        return src;
    }
    let ratio = sample_rate as f32 / ASSET_HZ;
    let n = (src.len() as f32 * ratio) as usize;
    (0..n)
        .map(|i| {
            let x = i as f32 / ratio;
            let a = x as usize;
            let f = x - a as f32;
            let b = (a + 1).min(src.len() - 1);
            src[a] * (1.0 - f) + src[b] * f
        })
        .collect()
}

// A soft friction envelope followed by a damped contact tap, like plastic
// sliding across a tabletop. Fixed seeds keep the small variations reproducible.
fn tactile(sound: Sfx, rate: u32) -> Vec<i16> {
    use std::f32::consts::TAU;
    let Sfx::Cartridge(variant) = sound else {
        return Vec::new();
    };
    if rate == 0 {
        return Vec::new();
    }
    let duration = 0.054;
    let pitch = 1.0 + (variant % 5) as f32 * 0.023;
    let mut rng = 0x724a_35b1u32.wrapping_add(u32::from(variant % 5) * 7919);
    let (mut low, mut bass) = (0.0f32, 0.0f32);
    let low_pass = 1.0 - (-TAU * 2400.0 * pitch / rate as f32).exp();
    let high_pass = 1.0 - (-TAU * 420.0 / rate as f32).exp();
    let mut out = Vec::with_capacity((duration * rate as f32) as usize * 2);
    for i in 0..(duration * rate as f32) as usize {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
        low += low_pass * (noise - low);
        bass += high_pass * (low - bass);
        let t = i as f32 / rate as f32;
        let attack = (t / 0.007).clamp(0.0, 1.0);
        let release = ((0.043 - t) / 0.018).clamp(0.0, 1.0);
        let grain = 0.8 + 0.2 * (TAU * 135.0 * pitch * t).sin();
        let friction = (low - bass) * attack * release * grain;
        let age = (t - 0.036).max(0.0);
        let tap =
            (age / 0.002).min(1.0) * (TAU * 760.0 * pitch * age).sin() * (-age / 0.0035).exp();
        let fade = ((duration - t) / 0.006).clamp(0.0, 1.0);
        let sample = (friction * 0.8 + tap * 0.18) * fade * 0.105;
        let value = (sample * 32767.0).clamp(-32767.0, 32767.0) as i16;
        out.extend([value, value]);
    }
    out
}
