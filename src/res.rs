//! The SALVE core: zero-latency resonance suppression, grown from TAME.
//!
//! A bank of narrow band-pass detectors watches one signal. Each band is compared with
//! the average of the bands around it, so broad tone and tilt are left alone and only
//! what sticks out (a resonance, a whistle, a harsh "ess") counts. That excess turns
//! into a dynamic bell cut on the signal path. No FFT, so no latency and little CPU.

use std::f32::consts::PI;

pub const BANDS: usize = 48;
pub const F_LO: f32 = 30.0;
pub const F_HI: f32 = 19000.0;
/// Control-rate step (samples): detectors -> cuts -> bell coefficients.
const CR: usize = 32;
pub const MAX_CUT_DB: f32 = 24.0;
/// Widest neighbourhood (bands each side) the reference is taken from.
const MAX_W: usize = 8;

pub fn band_freq(k: usize) -> f32 {
    F_LO * (F_HI / F_LO).powf(k as f32 / (BANDS - 1) as f32)
}
fn spacing_q() -> f32 {
    let oct = (F_HI / F_LO).log2() / (BANDS - 1) as f32;
    1.0 / (2f32.powf(oct) - 1.0)
}

/// Band-pass detector (Simper SVF, band output, unity peak).
#[derive(Clone, Copy, Default)]
struct Bp {
    g: f32,
    k: f32,
    s1: f32,
    s2: f32,
}
impl Bp {
    fn set(&mut self, f: f32, q: f32, sr: f32) {
        self.g = (PI * (f / sr).min(0.49)).tan();
        self.k = 1.0 / q;
    }
    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        let v3 = x - self.s2;
        let v1 = a1 * self.s1 + self.g * a1 * v3;
        let v2 = self.s2 + self.g * v1;
        self.s1 = 2.0 * v1 - self.s1;
        self.s2 = 2.0 * v2 - self.s2;
        v1 * self.k
    }
}

/// Peaking bell (RBJ), transposed direct form II, two channel states.
#[derive(Clone, Copy)]
struct Bell {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z: [[f32; 2]; 2],
    active: bool,
}
impl Default for Bell {
    fn default() -> Self {
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, z: [[0.0; 2]; 2], active: false }
    }
}
impl Bell {
    fn set(&mut self, f: f32, q: f32, gain_db: f32, sr: f32) {
        if gain_db > -0.05 {
            if self.active {
                self.active = false;
                self.z = [[0.0; 2]; 2];
            }
            return;
        }
        self.active = true;
        let a = 10f32.powf(gain_db / 40.0);
        let w = 2.0 * PI * (f / sr).min(0.49);
        let (sn, cs) = w.sin_cos();
        let al = sn / (2.0 * q);
        let a0 = 1.0 + al / a;
        self.b0 = (1.0 + al * a) / a0;
        self.b1 = -2.0 * cs / a0;
        self.b2 = (1.0 - al * a) / a0;
        self.a1 = -2.0 * cs / a0;
        self.a2 = (1.0 - al / a) / a0;
    }
    #[inline]
    fn tick(&mut self, ch: usize, x: f32) -> f32 {
        if !self.active {
            return x;
        }
        let z = &mut self.z[ch];
        let y = self.b0 * x + z[0];
        z[0] = self.b1 * x - self.a1 * y + z[1];
        z[1] = self.b2 * x - self.a2 * y;
        y
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// 0..1: how hard resonances get pushed down (and the cut ceiling, up to 24 dB)
    pub depth: f32,
    /// 0..1: narrow detectors and narrow cuts at 1, broad at 0
    pub sharp: f32,
    /// 0..1: how far a band must stick out before it counts; high = only the worst offenders
    pub select: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    /// HARD: full cut as soon as it's over; SOFT: eases in over a 3 dB knee
    pub hard: bool,
    /// CARVE: the detector hears the sidechain key; a band is cut by how loud the KEY is
    /// in it (above the SELECTIVITY threshold), not by how much it sticks out.
    pub carve: bool,
    /// per-band multiplier from the band editor (0..2), already including LO/HI
    pub weight: [f32; BANDS],
}
impl Default for Settings {
    fn default() -> Self {
        Self { depth: 0.5, sharp: 0.5, select: 0.5, attack_ms: 4.0, release_ms: 60.0, hard: false, carve: false, weight: [1.0; BANDS] }
    }
}

pub struct Res {
    sr: f32,
    det: [Bp; BANDS],
    env: [f32; BANDS],
    acc: [f32; BANDS],
    /// current cut per band in dB (<= 0)
    pub cut: [f32; BANDS],
    /// last detector level and how far each band stuck out above its reference (dB)
    pub level: [f32; BANDS],
    pub excess: [f32; BANDS],
    bells: [Bell; BANDS],
    n: usize,
    q_set: f32,
    /// per-band detector attack/release coefficients: at least a few cycles of the band
    ea: [f32; BANDS],
    er: [f32; BANDS],
}

impl Res {
    pub fn new(sr: f32) -> Self {
        let mut t = Self { sr, det: [Bp::default(); BANDS], env: [0.0; BANDS], acc: [0.0; BANDS], cut: [0.0; BANDS], level: [-120.0; BANDS], excess: [0.0; BANDS], bells: [Bell::default(); BANDS], n: 0, q_set: -1.0, ea: [0.0; BANDS], er: [0.0; BANDS] };
        let dt = CR as f32 / sr;
        for k in 0..BANDS {
            let f = band_freq(k);
            t.ea[k] = (-dt / (1.5 / f).max(0.001)).exp();
            t.er[k] = (-dt / (6.0 / f).max(0.02)).exp();
        }
        t.set_q(0.5);
        t
    }
    fn set_q(&mut self, sharp: f32) {
        let q = spacing_q() * (0.9 + sharp * 0.8);
        for (k, d) in self.det.iter_mut().enumerate() {
            d.set(band_freq(k), q, self.sr);
        }
        self.q_set = sharp;
    }
    pub fn clear(&mut self) {
        for d in self.det.iter_mut() {
            d.s1 = 0.0;
            d.s2 = 0.0;
        }
        for b in self.bells.iter_mut() {
            b.z = [[0.0; 2]; 2];
            b.active = false;
        }
        self.env = [0.0; BANDS];
        self.acc = [0.0; BANDS];
        self.cut = [0.0; BANDS];
        self.n = 0;
    }

    /// Feed the detector one sample (runs the control step every CR samples).
    #[inline]
    pub fn detect(&mut self, x: f32, s: &Settings) {
        for k in 0..BANDS {
            let y = self.det[k].tick(x);
            self.acc[k] += y * y;
        }
        self.n += 1;
        if self.n >= CR {
            self.control(s);
            self.n = 0;
        }
    }

    /// Run one sample of channel `ch` (0 or 1) through the current cuts.
    #[inline]
    pub fn apply(&mut self, ch: usize, x: f32) -> f32 {
        let mut a = x;
        for bell in self.bells.iter_mut() {
            a = bell.tick(ch, a);
        }
        a
    }

    fn control(&mut self, s: &Settings) {
        if (s.sharp - self.q_set).abs() > 0.02 {
            self.set_q(s.sharp);
        }
        let dt = CR as f32 / self.sr;
        let mut db = [0.0f32; BANDS];
        for k in 0..BANDS {
            let p = self.acc[k] / CR as f32;
            self.acc[k] = 0.0;
            let c = if p > self.env[k] { self.ea[k] } else { self.er[k] };
            self.env[k] = p + c * (self.env[k] - p);
            db[k] = 10.0 * (self.env[k] + 1e-12).log10();
        }
        // Low selectivity compares with a wide neighbourhood (catches broad bumps too);
        // high selectivity compares with close neighbours and wants a bigger peak.
        let w = (3.0 + (1.0 - s.select) * 5.0).round().min(MAX_W as f32) as isize;
        let thr = 1.0 + 5.0 * s.select;
        let scale = 0.3 + 1.4 * s.depth;
        let ceiling = MAX_CUT_DB * s.depth;
        let (ga, gr) = ((-dt / (s.attack_ms.max(0.1) * 1e-3)).exp(), (-dt / (s.release_ms.max(1.0) * 1e-3)).exp());
        let q_bell = spacing_q() * (0.45 + s.sharp * 1.0);
        for k in 0..BANDS {
            let f = band_freq(k);
            let wt = s.weight[k];
            let mut target = 0.0;
            if s.carve {
                // key threshold -60..-24 dB from SELECTIVITY
                let over = db[k] - (-60.0 + 36.0 * s.select);
                if wt > 0.001 && s.depth > 0.0 && over > 0.0 && f < 0.45 * self.sr {
                    let shaped = if s.hard { over } else { over * over / (over + 3.0) };
                    target = -(shaped * 0.5 * scale * wt).min(ceiling * wt).min(MAX_CUT_DB);
                }
                self.level[k] = db[k];
                let g = if target < self.cut[k] { ga } else { gr };
                self.cut[k] = target + g * (self.cut[k] - target);
                self.bells[k].set(f, q_bell, self.cut[k], self.sr);
                continue;
            }
            if wt > 0.001 && s.depth > 0.0 && db[k] > -80.0 && f < 0.45 * self.sr {
                // Reference = median of the neighbourhood, skipping the bands right next
                // to this one (a resonance leaks into them) so the peak can't raise its own bar.
                let mut nb = [0.0f32; 2 * MAX_W];
                let mut n = 0;
                for j in -w..=w {
                    if j.abs() < 2 {
                        continue;
                    }
                    let i = k as isize + j;
                    let m = k as isize - j;
                    if i >= 0 && (i as usize) < BANDS {
                        nb[n] = db[i as usize];
                        n += 1;
                    } else if m >= 0 && (m as usize) < BANDS {
                        // At the ends of the range: reflect the other side through this band,
                        // so a plain slope doesn't look like a peak.
                        nb[n] = 2.0 * db[k] - db[m as usize];
                        n += 1;
                    }
                }
                let nb = &mut nb[..n];
                nb.sort_unstable_by(|a, b| a.total_cmp(b));
                let reference = if n == 0 { db[k] } else if n % 2 == 1 { nb[n / 2] } else { 0.5 * (nb[n / 2 - 1] + nb[n / 2]) };
                let excess = db[k] - reference - thr;
                self.excess[k] = excess + thr;
                if excess > 0.0 {
                    let shaped = if s.hard { excess } else { excess * excess / (excess + 3.0) };
                    target = -(shaped * scale * wt).min(ceiling * wt).min(MAX_CUT_DB);
                }
            }
            self.level[k] = db[k];
            let g = if target < self.cut[k] { ga } else { gr };
            self.cut[k] = target + g * (self.cut[k] - target);
            self.bells[k].set(f, q_bell, self.cut[k], self.sr);
        }
    }
}
