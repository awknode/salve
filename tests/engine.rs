use salve::res::{band_freq, BANDS};
use salve::*;
const SR: f32 = 48000.0;
fn noise(n: usize, a: f32, seed: u32) -> Vec<f32> {
    let mut s = seed;
    (0..n).map(|_| { s = s.wrapping_mul(1664525).wrapping_add(1013904223); a * ((s >> 8) as f32 / 8388608.0 - 1.0) }).collect()
}
fn tone(n: usize, f: f32, a: f32) -> Vec<f32> {
    (0..n).map(|i| a * (2.0 * std::f32::consts::PI * f * i as f32 / SR).sin()).collect()
}
fn add(a: &[f32], b: &[f32]) -> Vec<f32> { a.iter().zip(b).map(|(x, y)| x + y).collect() }
fn rms(x: &[f32]) -> f32 { (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt() }
fn run2(e: &mut Engine, st: &State, l: &[f32], r: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let (mut a, mut b) = (l.to_vec(), r.to_vec());
    for (x, y) in a.chunks_mut(256).zip(b.chunks_mut(256)) { e.process(x, y, st); }
    (a, b)
}
fn run(e: &mut Engine, st: &State, x: &[f32]) -> Vec<f32> { run2(e, st, x, x).0 }
fn deepest(e: &Engine) -> (f32, f32) {
    let c = e.cut();
    let k = (0..BANDS).min_by(|a, b| c[*a].total_cmp(&c[*b])).unwrap();
    (c[k], band_freq(k))
}
/// Level of frequency f in x (single-bin DFT).
fn level_at(x: &[f32], f: f32) -> f32 {
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for (i, v) in x.iter().enumerate() {
        let a = 2.0 * std::f64::consts::PI * f as f64 * i as f64 / SR as f64;
        re += *v as f64 * a.cos();
        im += *v as f64 * a.sin();
    }
    (2.0 * (re * re + im * im).sqrt() / x.len() as f64) as f32
}

#[test]
fn cuts_the_whistle_not_the_noise() {
    let n = 96000;
    let st = State { safe: false, ..State::default() };
    // noise alone: barely touched
    let x = noise(n, 0.2, 1);
    let mut e = Engine::new(SR);
    let y = run(&mut e, &st, &x);
    let d = 20.0 * (rms(&y[48000..]) / rms(&x[48000..])).log10();
    let (c, _) = deepest(&e);
    println!("noise: level {d:+.2} dB, deepest cut {c:.1} dB");
    assert!(d > -1.0, "noise lost {d} dB");
    // noise + a 2.9 kHz whistle: the whistle comes down, near its own frequency
    let x = add(&noise(n, 0.2, 1), &tone(n, 2900.0, 0.2));
    let mut e = Engine::new(SR);
    let y = run(&mut e, &st, &x);
    let (c, f) = deepest(&e);
    let drop = 20.0 * (level_at(&y[48000..], 2900.0) / level_at(&x[48000..], 2900.0)).log10();
    println!("whistle: cut {c:.1} dB at {f:.0} Hz, tone level {drop:+.1} dB");
    assert!(drop < -4.0, "whistle only {drop} dB");
    assert!(f > 2300.0 && f < 3700.0, "cut at {f}");
    // more DEPTH = more cut
    let mut e2 = Engine::new(SR);
    let y2 = run(&mut e2, &State { depth: 1.0, ..st }, &x);
    let drop2 = 20.0 * (level_at(&y2[48000..], 2900.0) / level_at(&x[48000..], 2900.0)).log10();
    println!("whistle at depth 100%: {drop2:+.1} dB");
    assert!(drop2 < drop - 2.0);
}

#[test]
fn depth_zero_mix_zero_and_bypass_are_dry() {
    let x = add(&noise(24000, 0.3, 3), &tone(24000, 1200.0, 0.3));
    for st in [State { depth: 0.0, safe: false, ..State::default() }, State { mix: 0.0, safe: false, ..State::default() }, State { bypass: true, safe: false, ..State::default() }] {
        let mut e = Engine::new(SR);
        let y = run(&mut e, &st, &x);
        for i in 6000..24000 {
            assert!((y[i] - x[i]).abs() < 1e-5, "{st:?} sample {i}: {} vs {}", y[i], x[i]);
        }
    }
}

#[test]
fn delta_is_what_was_taken_out() {
    let x = add(&noise(48000, 0.2, 5), &tone(48000, 3300.0, 0.25));
    let st = State { safe: false, ..State::default() };
    let y = run(&mut Engine::new(SR), &st, &x);
    let d = run(&mut Engine::new(SR), &State { delta: true, ..st }, &x);
    for i in 2000..48000 {
        assert!((d[i] - (x[i] - y[i])).abs() < 1e-4, "sample {i}");
    }
    assert!(rms(&d[24000..]) > 0.01, "delta is silent");
}

#[test]
fn limits_and_band_editor_are_respected() {
    let n = 72000;
    let x = add(&noise(n, 0.2, 1), &tone(n, 2900.0, 0.2));
    let lvl = |st: State| {
        let y = run(&mut Engine::new(SR), &st, &x);
        20.0 * (level_at(&y[48000..], 2900.0) / level_at(&x[48000..], 2900.0)).log10()
    };
    let base = State { safe: false, ..State::default() };
    let hi_cut = lvl(State { hi: 1000.0, ..base });
    let mut nodes = base.nodes;
    nodes[3].1 = 0.0; // node 4 sits at 2.5 kHz
    nodes[4].1 = 0.0;
    let node_off = lvl(State { nodes, ..base });
    let mut nodes = base.nodes;
    nodes[3].1 = 2.0;
    let node_up = lvl(State { nodes, ..base });
    let normal = lvl(base);
    println!("normal {normal:+.1}, HI 1 kHz {hi_cut:+.1}, nodes 4+5 at 0% {node_off:+.1}, node 4 at 200% {node_up:+.1}");
    assert!(hi_cut > -0.5 && node_off > -0.5);
    assert!(node_up < normal - 1.0);
}

#[test]
fn channel_modes() {
    let n = 48000;
    let l = add(&noise(n, 0.2, 7), &tone(n, 2900.0, 0.2));
    let r = add(&noise(n, 0.2, 8), &tone(n, 4400.0, 0.2));
    for chan in 0..5 {
        let st = State { chan, safe: false, ..State::default() };
        let (a, b) = run2(&mut Engine::new(SR), &st, &l, &r);
        assert!(a.iter().chain(b.iter()).all(|v| v.is_finite() && v.abs() < 4.0), "{}", CHANNELS[chan]);
    }
    // L/R: the whistle only in L is cut in L; R (no whistle there) keeps its 2.9 kHz content
    let (a, b) = run2(&mut Engine::new(SR), &State { chan: 1, safe: false, ..State::default() }, &l, &r);
    let dl = 20.0 * (level_at(&a[24000..], 2900.0) / level_at(&l[24000..], 2900.0)).log10();
    let dr = 20.0 * (level_at(&b[24000..], 4400.0) / level_at(&r[24000..], 4400.0)).log10();
    println!("L/R: L whistle {dl:+.1} dB, R whistle {dr:+.1} dB");
    assert!(dl < -3.0 && dr < -3.0);
    // MID: a pure side signal passes untouched
    let s = add(&noise(n, 0.2, 9), &tone(n, 2900.0, 0.3));
    let ns: Vec<f32> = s.iter().map(|v| -v).collect();
    let (a, _) = run2(&mut Engine::new(SR), &State { chan: 3, safe: false, ..State::default() }, &s, &ns);
    for i in 0..n {
        assert!((a[i] - s[i]).abs() < 1e-5, "MID touched the side at {i}");
    }
}

#[test]
fn safe_never_over_zero_and_silence_stays_silent() {
    let st = State { trim_db: 12.0, depth: 1.0, ..State::default() };
    let y = run(&mut Engine::new(SR), &st, &add(&noise(48000, 0.8, 2), &tone(48000, 900.0, 0.5)));
    let pk = y.iter().fold(0.0f32, |a, v| a.max(v.abs()));
    assert!(pk <= 1.0, "peak {pk}");
    let mut e = Engine::new(SR);
    let _ = run(&mut e, &State::default(), &tone(24000, 3000.0, 0.5));
    let z = run(&mut e, &State::default(), &vec![0.0; 24000]);
    assert!(z[12000..].iter().all(|v| v.abs() < 1e-5));
}

#[test]
fn cpu() {
    let x = add(&noise(48000 * 5, 0.2, 1), &tone(48000 * 5, 2900.0, 0.2));
    for chan in [0usize, 2] {
        let st = State { chan, depth: 0.8, ..State::default() };
        let mut e = Engine::new(SR);
        let t = std::time::Instant::now();
        run2(&mut e, &st, &x, &x);
        let pct = t.elapsed().as_secs_f32() / 5.0 * 100.0;
        println!("salve {}: {pct:.2}% of one core", CHANNELS[chan]);
        assert!(pct < 8.0);
    }
}

#[test]
fn carve_cuts_where_the_key_is_loud() {
    let n = 72000;
    // main: a 60 Hz bass + a 3 kHz lead; key: a 60 Hz kick-ish tone
    let main = add(&tone(n, 60.0, 0.3), &tone(n, 3000.0, 0.3));
    let key = tone(n, 60.0, 0.5);
    let st = State { carve: true, depth: 0.6, safe: false, ..State::default() };
    let (mut a, mut b) = (main.clone(), main.clone());
    let mut e = Engine::new(SR);
    for ((x, y), k) in a.chunks_mut(256).zip(b.chunks_mut(256)).zip(key.chunks(256)) {
        e.process_key(x, y, Some((k, k)), &st);
    }
    let lo = 20.0 * (level_at(&a[48000..], 60.0) / level_at(&main[48000..], 60.0)).log10();
    let hi = 20.0 * (level_at(&a[48000..], 3000.0) / level_at(&main[48000..], 3000.0)).log10();
    println!("carve: 60 Hz {lo:+.1} dB, 3 kHz {hi:+.1} dB");
    assert!(lo < -6.0, "bass only carved {lo} dB");
    assert!(hi > -0.5, "lead was touched {hi} dB");
    // silent key: nothing happens (exact dry)
    let silent = vec![0.0; n];
    let (mut a, mut b) = (main.clone(), main.clone());
    let mut e = Engine::new(SR);
    for ((x, y), k) in a.chunks_mut(256).zip(b.chunks_mut(256)).zip(silent.chunks(256)) {
        e.process_key(x, y, Some((k, k)), &st);
    }
    for i in 0..n {
        assert!((a[i] - main[i]).abs() < 1e-5, "silent key changed sample {i}");
    }
    // CARVE off: the key is ignored
    let off = State { carve: false, ..st };
    let (mut a, mut b) = (main.clone(), main.clone());
    let (mut c, mut d) = (main.clone(), main.clone());
    let (mut e1, mut e2) = (Engine::new(SR), Engine::new(SR));
    for ((x, y), k) in a.chunks_mut(256).zip(b.chunks_mut(256)).zip(key.chunks(256)) {
        e1.process_key(x, y, Some((k, k)), &off);
    }
    for (x, y) in c.chunks_mut(256).zip(d.chunks_mut(256)) {
        e2.process(x, y, &off);
    }
    assert_eq!(a, c);
}
