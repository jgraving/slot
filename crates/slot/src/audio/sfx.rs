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
            Sfx::Cartridge(_) => 0.0,
        }
    }

    pub fn tail(self) -> f32 {
        let pcm = match self {
            Sfx::Insert => INSERT,
            Sfx::Eject => EJECT,
            Sfx::Cartridge(_) => return 0.028,
        };
        pcm.len() as f32 / 2.0 / ASSET_HZ - self.lead()
    }

    pub fn render(self, sample_rate: u32) -> Vec<i16> {
        if let Sfx::Cartridge(_) = self {
            return tactile(self, sample_rate);
        }
        let pcm = match self {
            Sfx::Insert => INSERT,
            Sfx::Eject => EJECT,
            Sfx::Cartridge(_) => unreachable!(),
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

// Short band-limited noise impulses excite two damped plastic resonances.
// Fixed seeds make previews reproducible; cartridge variants avoid a machine-gun tone.
fn tactile(sound: Sfx, rate: u32) -> Vec<i16> {
    use std::f32::consts::TAU;
    if rate == 0 {
        return Vec::new();
    }
    let (duration, pitch, gain, taps): (f32, f32, f32, &[(f32, f32)]) = match sound {
        Sfx::Cartridge(v) => (
            0.028,
            1.0 + (v % 5) as f32 * 0.023,
            0.095,
            &[(0.0, 1.0), (0.004, 0.22)],
        ),
        _ => return Vec::new(),
    };
    let mut rng = 0x724a_35b1u32;
    let mut low = 0.0f32;
    let mut out = Vec::with_capacity((duration * rate as f32) as usize * 2);
    for i in 0..(duration * rate as f32) as usize {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
        low += (1.0 - (-TAU * 4200.0 / rate as f32).exp()) * (noise - low);
        let t = i as f32 / rate as f32;
        let mut sample = 0.0;
        for &(start, strength) in taps {
            let age = t - start;
            if age < 0.0 {
                continue;
            }
            let attack = (age / 0.0006).min(1.0);
            let body = (TAU * 1480.0 * pitch * age).sin() * (-age / 0.0038).exp()
                + 0.35 * (TAU * 2630.0 * pitch * age).sin() * (-age / 0.0024).exp();
            sample += strength * attack * (0.58 * body + 0.42 * low * (-age / 0.0017).exp());
        }
        let fade = ((duration - t) / 0.004).clamp(0.0, 1.0);
        let value = (sample * fade * gain * 32767.0).clamp(-32767.0, 32767.0) as i16;
        out.extend([value, value]);
    }
    out
}
