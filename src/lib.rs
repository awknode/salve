//! SALVE — awk studios. Low-CPU, zero-latency resonance suppressor.
//!
//! in ─► detector (STEREO linked / L-R / M-S / MID / SIDE) ─► 48 dynamic bells
//!   ─► MIX against the dry ─► DELTA (hear only what's taken out) ─► TRIM ─► SAFE
//!
//! DEPTH, SHARPNESS, SELECTIVITY, ATTACK, RELEASE, SOFT/HARD; a 6-node band editor
//! weights how hard each part of the spectrum is treated; LO/HI limit the range.

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub mod analyzer;
mod awk_ui;
pub mod dsp;
mod editor;
mod presets;
pub mod res;

use res::{band_freq, Res, BANDS};

pub const CHANNELS: [&str; 5] = ["STEREO", "L / R", "M / S", "MID", "SIDE"];
pub const NODES: usize = 6;
pub const NODE_F: [f32; NODES] = [80.0, 250.0, 800.0, 2500.0, 6000.0, 12000.0];

pub struct Shared {
    pub meters: awk_ui::Meters,
    pub spec_in: awk_ui::SpecRing,
    pub spec_out: awk_ui::SpecRing,
    pub spec_key: awk_ui::SpecRing,
    /// current cut per band, dB (positive = cut)
    pub cut: [AtomicU32; BANDS],
}
impl Default for Shared {
    fn default() -> Self {
        Self {
            meters: awk_ui::Meters::default(),
            spec_in: awk_ui::SpecRing::new(analyzer::FFT),
            spec_out: awk_ui::SpecRing::new(analyzer::FFT),
            spec_key: awk_ui::SpecRing::new(analyzer::FFT),
            cut: std::array::from_fn(|_| AtomicU32::new(0)),
        }
    }
}
pub fn get(a: &AtomicU32) -> f32 {
    f32::from_bits(a.load(Ordering::Relaxed))
}

fn hz(name: &str, def: f32) -> FloatParam {
    FloatParam::new(name, def, FloatRange::Skewed { min: 20.0, max: 20000.0, factor: FloatRange::skew_factor(-2.2) })
        .with_value_to_string(Arc::new(|v| if v >= 1000.0 { format!("{:.1} kHz", v / 1000.0) } else { format!("{:.0} Hz", v) }))
        .with_string_to_value(Arc::new(|s| {
            let t = s.trim().to_lowercase();
            if let Some(k) = t.strip_suffix("khz") { k.trim().parse::<f32>().ok().map(|v| v * 1000.0) } else { t.trim_end_matches("hz").trim().parse().ok() }
        }))
}
fn unit(name: &str, def: f32) -> FloatParam {
    FloatParam::new(name, def, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
        .with_unit("%")
}
fn weight(name: &str) -> FloatParam {
    FloatParam::new(name, 1.0, FloatRange::Linear { min: 0.0, max: 2.0 }).with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0)))
}

#[derive(Params)]
pub struct SalveParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
    #[id = "depth"]
    pub depth: FloatParam,
    #[id = "sharp"]
    pub sharp: FloatParam,
    #[id = "select"]
    pub select: FloatParam,
    #[id = "att"]
    pub att: FloatParam,
    #[id = "rel"]
    pub rel: FloatParam,
    #[id = "hard"]
    pub hard: BoolParam,
    #[id = "chan"]
    pub chan: IntParam,
    #[id = "delta"]
    pub delta: BoolParam,
    /// sidechain CARVE: cut where the key (sidechain input) is loud
    #[id = "carve"]
    pub carve: BoolParam,
    #[id = "gmix"]
    pub mix: FloatParam,
    #[id = "trim"]
    pub trim: FloatParam,
    #[id = "lo"]
    pub lo: FloatParam,
    #[id = "hi"]
    pub hi: FloatParam,
    #[id = "nf0"]
    pub nf0: FloatParam,
    #[id = "nf1"]
    pub nf1: FloatParam,
    #[id = "nf2"]
    pub nf2: FloatParam,
    #[id = "nf3"]
    pub nf3: FloatParam,
    #[id = "nf4"]
    pub nf4: FloatParam,
    #[id = "nf5"]
    pub nf5: FloatParam,
    #[id = "ng0"]
    pub ng0: FloatParam,
    #[id = "ng1"]
    pub ng1: FloatParam,
    #[id = "ng2"]
    pub ng2: FloatParam,
    #[id = "ng3"]
    pub ng3: FloatParam,
    #[id = "ng4"]
    pub ng4: FloatParam,
    #[id = "ng5"]
    pub ng5: FloatParam,
    #[id = "safe"]
    pub safe: BoolParam,
    /// editor theme: dark by default
    #[id = "light"]
    pub light: BoolParam,
    #[id = "bypass"]
    pub bypass: BoolParam,
}

impl Default for SalveParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(editor::WIDTH, editor::HEIGHT),
            depth: unit("Depth", 0.5),
            sharp: unit("Sharpness", 0.5),
            select: unit("Selectivity", 0.5),
            att: FloatParam::new("Attack", 4.0, FloatRange::Skewed { min: 0.5, max: 100.0, factor: FloatRange::skew_factor(-1.6) }).with_unit(" ms").with_value_to_string(Arc::new(|v| if v < 10.0 { format!("{:.1}", v) } else { format!("{:.0}", v) })),
            rel: FloatParam::new("Release", 60.0, FloatRange::Skewed { min: 5.0, max: 500.0, factor: FloatRange::skew_factor(-1.4) }).with_unit(" ms").with_value_to_string(formatters::v2s_f32_rounded(0)),
            hard: BoolParam::new("Hard", false).with_value_to_string(Arc::new(|v| if v { "HARD".into() } else { "SOFT".into() })),
            chan: IntParam::new("Channels", 0, IntRange::Linear { min: 0, max: 4 }).with_value_to_string(Arc::new(|v| CHANNELS[v.clamp(0, 4) as usize].to_string())),
            delta: BoolParam::new("Delta", false),
            carve: BoolParam::new("Sidechain Carve", false).with_value_to_string(Arc::new(|v| if v { "CARVE".into() } else { "OFF".into() })),
            mix: awk_ui::mix_param(),
            trim: FloatParam::new("Trim", 0.0, FloatRange::Linear { min: -12.0, max: 12.0 }).with_smoother(SmoothingStyle::Linear(20.0)).with_unit(" dB").with_value_to_string(formatters::v2s_f32_rounded(1)),
            lo: hz("Low Limit", 20.0),
            hi: hz("High Limit", 20000.0),
            nf0: hz("Node 1 Freq", NODE_F[0]),
            nf1: hz("Node 2 Freq", NODE_F[1]),
            nf2: hz("Node 3 Freq", NODE_F[2]),
            nf3: hz("Node 4 Freq", NODE_F[3]),
            nf4: hz("Node 5 Freq", NODE_F[4]),
            nf5: hz("Node 6 Freq", NODE_F[5]),
            ng0: weight("Node 1 Amount"),
            ng1: weight("Node 2 Amount"),
            ng2: weight("Node 3 Amount"),
            ng3: weight("Node 4 Amount"),
            ng4: weight("Node 5 Amount"),
            ng5: weight("Node 6 Amount"),
            safe: awk_ui::safe_param(),
            light: BoolParam::new("Light Theme", false).non_automatable(),
            bypass: awk_ui::bypass_param(),
        }
    }
}
impl SalveParams {
    pub fn node_f(&self) -> [&FloatParam; NODES] {
        [&self.nf0, &self.nf1, &self.nf2, &self.nf3, &self.nf4, &self.nf5]
    }
    pub fn node_g(&self) -> [&FloatParam; NODES] {
        [&self.ng0, &self.ng1, &self.ng2, &self.ng3, &self.ng4, &self.ng5]
    }
    pub fn state(&self) -> State {
        let (nf, ng) = (self.node_f(), self.node_g());
        State {
            depth: self.depth.value(),
            sharp: self.sharp.value(),
            select: self.select.value(),
            attack_ms: self.att.value(),
            release_ms: self.rel.value(),
            hard: self.hard.value(),
            chan: self.chan.value().clamp(0, 4) as usize,
            delta: self.delta.value(),
            carve: self.carve.value(),
            mix: self.mix.value(),
            trim_db: self.trim.value(),
            lo: self.lo.value(),
            hi: self.hi.value(),
            nodes: std::array::from_fn(|i| (nf[i].value(), ng[i].value())),
            safe: self.safe.value(),
            bypass: self.bypass.value(),
        }
    }
}

/// The band-editor curve: amount (0..2) at frequency f, eased between nodes in log-frequency.
pub fn weight_at(nodes: &[(f32, f32); NODES], f: f32) -> f32 {
    let mut n = *nodes;
    n.sort_by(|a, b| a.0.total_cmp(&b.0));
    if f <= n[0].0 {
        return n[0].1;
    }
    for i in 0..NODES - 1 {
        let (a, b) = (n[i], n[i + 1]);
        if f <= b.0 {
            let t = ((f.ln() - a.0.ln()) / (b.0.ln() - a.0.ln()).max(1e-6)).clamp(0.0, 1.0);
            let t = t * t * (3.0 - 2.0 * t);
            return a.1 + (b.1 - a.1) * t;
        }
    }
    n[NODES - 1].1
}
/// LO/HI limits as a soft (1/3 octave) fade, so a cut never stops dead at the edge.
pub fn range_at(lo: f32, hi: f32, f: f32) -> f32 {
    let o = 1.0 / 3.0;
    let a = (((f / lo.max(1.0)).log2() + o) / o).clamp(0.0, 1.0);
    let b = (((hi / f).log2() + o) / o).clamp(0.0, 1.0);
    a.min(b)
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub depth: f32,
    pub sharp: f32,
    pub select: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub hard: bool,
    pub chan: usize,
    pub delta: bool,
    pub carve: bool,
    pub mix: f32,
    pub trim_db: f32,
    pub lo: f32,
    pub hi: f32,
    pub nodes: [(f32, f32); NODES],
    pub safe: bool,
    pub bypass: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            depth: 0.5,
            sharp: 0.5,
            select: 0.5,
            attack_ms: 4.0,
            release_ms: 60.0,
            hard: false,
            chan: 0,
            delta: false,
            carve: false,
            mix: 1.0,
            trim_db: 0.0,
            lo: 20.0,
            hi: 20000.0,
            nodes: std::array::from_fn(|i| (NODE_F[i], 1.0)),
            safe: true,
            bypass: false,
        }
    }
}
impl State {
    pub fn settings(&self) -> res::Settings {
        res::Settings {
            depth: self.depth,
            sharp: self.sharp,
            select: self.select,
            attack_ms: self.attack_ms,
            release_ms: self.release_ms,
            hard: self.hard,
            carve: self.carve,
            weight: std::array::from_fn(|k| {
                let f = band_freq(k);
                weight_at(&self.nodes, f) * range_at(self.lo, self.hi, f)
            }),
        }
    }
}

pub struct Engine {
    pub sr: f32,
    core: [Res; 2],
    chan: usize,
    mix: f32,
    trim: f32,
    dlt: f32,
    byp: awk_ui::BypassRamp,
    ceil: awk_ui::Ceiling,
    meter: awk_ui::PeakFollower,
    pub shared: Arc<Shared>,
}

impl Engine {
    pub fn new(sr: f32) -> Self {
        let mut e = Self {
            sr,
            core: [Res::new(sr), Res::new(sr)],
            chan: 0,
            mix: -1.0,
            trim: 1.0,
            dlt: 0.0,
            byp: awk_ui::BypassRamp::default(),
            ceil: awk_ui::Ceiling::default(),
            meter: awk_ui::PeakFollower::default(),
            shared: Arc::new(Shared::default()),
        };
        e.byp.set_sr(sr);
        e.ceil.set_sr(sr);
        e.meter.set_sr(sr);
        e
    }
    pub fn with_shared(mut self, s: Arc<Shared>) -> Self {
        s.spec_in.sr.store(self.sr.to_bits(), Ordering::Relaxed);
        s.spec_out.sr.store(self.sr.to_bits(), Ordering::Relaxed);
        s.spec_key.sr.store(self.sr.to_bits(), Ordering::Relaxed);
        self.shared = s;
        self
    }
    pub fn reset(&mut self) {
        self.core.iter_mut().for_each(|c| c.clear());
    }
    /// The deepest cut per band right now (dB, <= 0) across the active detectors.
    pub fn cut(&self) -> [f32; BANDS] {
        std::array::from_fn(|k| self.core[0].cut[k].min(self.core[1].cut[k]))
    }
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], st: &State) {
        self.process_key(l, r, None, st);
    }
    /// With a sidechain key (CARVE listens to it; otherwise it is ignored).
    pub fn process_key(&mut self, l: &mut [f32], r: &mut [f32], key: Option<(&[f32], &[f32])>, st: &State) {
        if st.chan != self.chan {
            self.reset();
            self.chan = st.chan;
        }
        let s = st.settings();
        let a = 1.0 - (-1.0 / (0.02 * self.sr)).exp();
        let trim_t = dsp::db(st.trim_db);
        if self.mix < 0.0 {
            self.mix = st.mix;
            self.trim = trim_t;
            self.dlt = if st.delta { 1.0 } else { 0.0 };
        }
        let [c0, c1] = &mut self.core;
        for i in 0..l.len() {
            let (dl, dr) = (l[i], r[i]);
            // what the detectors hear: the signal itself, or the key in CARVE
            let (kl, kr) = if st.carve {
                match key {
                    Some((a, b)) if i < a.len() && i < b.len() => (a[i], b[i]),
                    _ => (0.0, 0.0),
                }
            } else {
                (dl, dr)
            };
            if st.carve {
                self.shared.spec_key.push(0.5 * (kl + kr));
            }
            let (wl, wr) = match st.chan {
                0 => {
                    c0.detect(0.5 * (kl + kr), &s);
                    (c0.apply(0, dl), c0.apply(1, dr))
                }
                1 => {
                    c0.detect(kl, &s);
                    c1.detect(kr, &s);
                    (c0.apply(0, dl), c1.apply(0, dr))
                }
                _ => {
                    let (m, sd) = (0.5 * (dl + dr), 0.5 * (dl - dr));
                    let (km, ks) = (0.5 * (kl + kr), 0.5 * (kl - kr));
                    let mut m2 = m;
                    let mut s2 = sd;
                    if st.chan != 4 {
                        c0.detect(km, &s);
                        m2 = c0.apply(0, m);
                    }
                    if st.chan != 3 {
                        c1.detect(ks, &s);
                        s2 = c1.apply(0, sd);
                    }
                    (m2 + s2, m2 - s2)
                }
            };
            self.mix += a * (st.mix - self.mix);
            self.trim += a * (trim_t - self.trim);
            self.dlt += a * ((if st.delta { 1.0 } else { 0.0 }) - self.dlt);
            let (ml, mr) = (dl + (wl - dl) * self.mix, dr + (wr - dr) * self.mix);
            // DELTA: what was taken out, at the level it was taken out
            let (ol, or) = (ml + self.dlt * (dl - 2.0 * ml), mr + self.dlt * (dr - 2.0 * mr));
            let (ol, or) = (ol * self.trim, or * self.trim);
            let w = self.byp.next(st.bypass);
            let (ol, or) = (dl + (ol - dl) * w, dr + (or - dr) * w);
            let (ol, or) = self.ceil.tick(ol, or, st.safe);
            let (ol, or) = if ol.is_finite() && or.is_finite() { (ol, or) } else { (0.0, 0.0) };
            self.meter.feed(dl, dr, ol, or);
            self.shared.spec_in.push(0.5 * (dl + dr));
            self.shared.spec_out.push(0.5 * (ol + or));
            l[i] = ol;
            r[i] = or;
        }
        if self.chan == 0 {
            self.core[1].cut = [0.0; BANDS];
        }
        let cut = self.cut();
        for k in 0..BANDS {
            self.shared.cut[k].store((-cut[k]).max(0.0).to_bits(), Ordering::Relaxed);
        }
        self.meter.store(&self.shared.meters);
        self.ceil.store(&self.shared.meters);
    }
}

pub struct Salve {
    params: Arc<SalveParams>,
    shared: Arc<Shared>,
    eng: Engine,
}

impl Default for Salve {
    fn default() -> Self {
        let shared = Arc::new(Shared::default());
        Self { params: Arc::new(SalveParams::default()), shared: shared.clone(), eng: Engine::new(48000.0).with_shared(shared) }
    }
}

impl Plugin for Salve {
    const NAME: &'static str = "Salve";
    const VENDOR: &'static str = "awk studios";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[new_nonzero_u32(2)],
        names: PortNames { layout: Some("Stereo + Sidechain"), main_input: Some("Input"), main_output: Some("Output"), aux_inputs: &["Sidechain"], aux_outputs: &[] },
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.shared.clone())
    }
    fn initialize(&mut self, _layout: &AudioIOLayout, cfg: &BufferConfig, _ctx: &mut impl InitContext<Self>) -> bool {
        self.eng = Engine::new(cfg.sample_rate).with_shared(self.shared.clone());
        true
    }
    fn reset(&mut self) {
        self.eng.reset();
    }
    fn process(&mut self, buffer: &mut Buffer, aux: &mut AuxiliaryBuffers, _ctx: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let st = self.params.state();
        let sl = buffer.as_slice();
        if sl.len() < 2 {
            return ProcessStatus::Normal;
        }
        let (a, b) = sl.split_at_mut(1);
        let key = aux.inputs.first_mut().map(|k| k.as_slice_immutable()).filter(|k| k.len() >= 2).map(|k| (&k[0][..], &k[1][..]));
        self.eng.process_key(&mut *a[0], &mut *b[0], key, &st);
        ProcessStatus::Normal
    }
}

impl ClapPlugin for Salve {
    const CLAP_ID: &'static str = "com.awknode.salve";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Resonance suppressor");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Equalizer, ClapFeature::Stereo];
}

impl Vst3Plugin for Salve {
    const VST3_CLASS_ID: [u8; 16] = *b"AwknodeSalveRes1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Eq, Vst3SubCategory::Dynamics];
}

nih_export_clap!(Salve);
nih_export_vst3!(Salve);
