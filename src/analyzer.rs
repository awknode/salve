//! Editor-side spectrum analyzer for awk studios plugins (needs the `realfft` crate).
//! Master copy in ~/Projects/awk-ui/analyzer.rs.
//! Reads a `SpecRing`, gives `POINTS` log-spaced levels in dBFS (sine peak = 0 dB)
//! from 20 Hz to 20 kHz, with fast rise and slow fall.

#![allow(dead_code)]

use crate::awk_ui::SpecRing;
use realfft::{num_complex::Complex, RealFftPlanner, RealToComplex};
use std::sync::Arc;

pub const FFT: usize = 4096;
pub const POINTS: usize = 200;

pub struct Analyzer {
    fft: Arc<dyn RealToComplex<f32>>,
    tbuf: Vec<f32>,
    raw: Vec<f32>,
    spec: Vec<Complex<f32>>,
    win: Vec<f32>,
    last_w: usize,
    pub db: Vec<f32>,
}

impl Default for Analyzer {
    fn default() -> Self {
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(FFT);
        let win: Vec<f32> = (0..FFT).map(|k| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * k as f32 / FFT as f32).cos()).collect();
        Self { spec: fft.make_output_vec(), fft, tbuf: vec![0.0; FFT], raw: vec![0.0; FFT], win, last_w: usize::MAX, db: vec![-120.0; POINTS] }
    }
}

impl Analyzer {
    /// Call once per frame.
    pub fn update(&mut self, ring: &SpecRing) {
        let w = ring.written();
        if w == self.last_w {
            // Audio stopped: let the display fall.
            self.db.iter_mut().for_each(|v| *v = (*v - 1.5).max(-120.0));
            return;
        }
        self.last_w = w;
        let sr = f32::from_bits(ring.sr.load(std::sync::atomic::Ordering::Relaxed)).max(8000.0);
        ring.read(&mut self.raw);
        for k in 0..FFT {
            self.tbuf[k] = self.raw[k] * self.win[k];
        }
        let _ = self.fft.process(&mut self.tbuf, &mut self.spec);
        // Hann window coherent gain 0.5: a full-scale sine reads |X| = FFT/4.
        let norm = 20.0 * (FFT as f32 / 4.0).log10();
        for i in 0..POINTS {
            let f0 = 20.0 * 1000f32.powf(i as f32 / POINTS as f32);
            let f1 = 20.0 * 1000f32.powf((i + 1) as f32 / POINTS as f32);
            let b0 = ((f0 / sr * FFT as f32) as usize).max(1);
            let b1 = ((f1 / sr * FFT as f32) as usize).max(b0 + 1).min(FFT / 2);
            let mut m = 0.0f32;
            for b in b0..b1 {
                m = m.max(self.spec[b].norm_sqr());
            }
            let v = 10.0 * m.max(1e-14).log10() - norm;
            let o = &mut self.db[i];
            *o = if v > *o { v } else { *o * 0.85 + v * 0.15 };
        }
    }
    /// Level at log position u (0..1).
    pub fn at(&self, u: f32) -> f32 {
        let f = (u.clamp(0.0, 1.0) * (POINTS - 1) as f32).max(0.0);
        let i = f as usize;
        let j = (i + 1).min(POINTS - 1);
        self.db[i] + (self.db[j] - self.db[i]) * (f - i as f32)
    }
}
