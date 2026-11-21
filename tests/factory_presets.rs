//! Factory presets: every id is a real param with a legal value; each plays bounded and
//! finite; with PRESET_OUT=dir the .awkp files are written. Unlisted params load at default.
use nih_plug::prelude::*;
use serde_json::{json, Map, Value};
use salve::*;

type Preset = (&'static str, &'static [(&'static str, f64)]);
// nodes default 80 / 250 / 800 / 2.5k / 6k / 12k at 100% (ng = 0..2)
const PRESETS: &[Preset] = &[
    ("VOCAL - Smooth Harshness", &[("depth", 0.45), ("sharp", 0.55), ("select", 0.5), ("att", 3.0), ("rel", 70.0), ("lo", 300.0), ("ng3", 1.4), ("ng4", 1.3)]),
    ("VOCAL - Sibilance Tamer", &[("depth", 0.6), ("sharp", 0.4), ("select", 0.35), ("att", 0.8), ("rel", 40.0), ("lo", 4000.0), ("ng4", 1.6), ("ng5", 1.3)]),
    ("VOCAL - Nasal Honk Out", &[("depth", 0.5), ("sharp", 0.7), ("select", 0.6), ("att", 6.0), ("rel", 90.0), ("lo", 500.0), ("hi", 2500.0), ("nf2", 1000.0), ("ng2", 1.6)]),
    ("VOCAL - Rap Vocal Polish", &[("depth", 0.4), ("sharp", 0.5), ("select", 0.45), ("att", 2.0), ("rel", 60.0), ("lo", 180.0), ("ng1", 0.6), ("ng3", 1.3), ("ng4", 1.4)]),
    ("VOCAL - Gentle Air", &[("depth", 0.25), ("sharp", 0.35), ("select", 0.3), ("att", 5.0), ("rel", 120.0), ("lo", 2000.0), ("gmix", 0.8)]),
    ("MIX BUS - Light Touch", &[("depth", 0.2), ("sharp", 0.4), ("select", 0.6), ("att", 8.0), ("rel", 150.0), ("gmix", 0.7)]),
    ("MIX BUS - De-Harsh Tops", &[("depth", 0.35), ("sharp", 0.5), ("select", 0.55), ("att", 4.0), ("rel", 100.0), ("lo", 1500.0), ("ng3", 1.3)]),
    ("MASTER - Transparent Resonance Control", &[("depth", 0.18), ("sharp", 0.6), ("select", 0.7), ("att", 10.0), ("rel", 200.0), ("ng0", 0.5)]),
    ("MASTER - Side Harshness Only", &[("depth", 0.35), ("sharp", 0.5), ("select", 0.5), ("chan", 4.0), ("lo", 1500.0)]),
    ("DRUMS - Overhead Ring", &[("depth", 0.5), ("sharp", 0.8), ("select", 0.65), ("att", 1.0), ("rel", 80.0), ("lo", 2000.0), ("hard", 1.0)]),
    ("DRUMS - Snare Ring Killer", &[("depth", 0.6), ("sharp", 0.85), ("select", 0.7), ("att", 2.0), ("rel", 150.0), ("lo", 150.0), ("hi", 1500.0)]),
    ("DRUMS - Room Boom", &[("depth", 0.5), ("sharp", 0.5), ("select", 0.5), ("att", 10.0), ("rel", 180.0), ("lo", 60.0), ("hi", 500.0)]),
    ("808 - Low Mid Mud", &[("depth", 0.45), ("sharp", 0.6), ("select", 0.55), ("att", 12.0), ("rel", 160.0), ("lo", 120.0), ("hi", 600.0), ("ng1", 1.4)]),
    ("BASS - Boomy Notes Evened", &[("depth", 0.55), ("sharp", 0.7), ("select", 0.6), ("att", 15.0), ("rel", 220.0), ("lo", 40.0), ("hi", 350.0)]),
    ("GUITAR - Fizz and Pick", &[("depth", 0.45), ("sharp", 0.5), ("select", 0.45), ("att", 2.0), ("rel", 60.0), ("lo", 2500.0), ("ng4", 1.4)]),
    ("GUITAR - Acoustic Boom", &[("depth", 0.4), ("sharp", 0.55), ("select", 0.55), ("att", 8.0), ("rel", 140.0), ("lo", 80.0), ("hi", 400.0)]),
    ("SYNTH - Resonant Peak Smoother", &[("depth", 0.6), ("sharp", 0.8), ("select", 0.6), ("att", 1.5), ("rel", 50.0), ("hard", 1.0)]),
    ("SAMPLE - Lo-Fi Loop Cleanup", &[("depth", 0.5), ("sharp", 0.4), ("select", 0.4), ("att", 5.0), ("rel", 90.0), ("lo", 200.0)]),
    ("STRINGS - Edge Off", &[("depth", 0.35), ("sharp", 0.45), ("select", 0.4), ("att", 6.0), ("rel", 130.0), ("lo", 2000.0), ("hi", 9000.0)]),
    ("PODCAST - Voice Clean", &[("depth", 0.4), ("sharp", 0.5), ("select", 0.5), ("att", 3.0), ("rel", 80.0), ("lo", 120.0), ("ng4", 1.3)]),
    ("FX - Heavy Squash (Hard)", &[("depth", 0.9), ("sharp", 0.5), ("select", 0.2), ("att", 0.5), ("rel", 30.0), ("hard", 1.0)]),
    ("FX - Delta Listen (What It Takes)", &[("depth", 0.6), ("delta", 1.0)]),
    // SIDECHAIN: route the key (kick, vocal) into SALVE's sidechain input
    ("SIDECHAIN - Kick Carves the Bass", &[("carve", 1.0), ("depth", 0.55), ("sharp", 0.5), ("select", 0.45), ("att", 1.0), ("rel", 90.0), ("hi", 300.0)]),
    ("SIDECHAIN - Vocal Pocket in the Beat", &[("carve", 1.0), ("depth", 0.4), ("sharp", 0.4), ("select", 0.5), ("att", 4.0), ("rel", 120.0), ("lo", 250.0), ("hi", 6000.0)]),
    ("SIDECHAIN - Snare Space in Guitars", &[("carve", 1.0), ("depth", 0.45), ("sharp", 0.55), ("select", 0.5), ("att", 1.5), ("rel", 80.0), ("lo", 150.0), ("hi", 4000.0)]),
];

fn build(params: &SalveParams, over: &[(&str, f64)]) -> Value {
    let map = params.param_map();
    let mut out = Map::new();
    for (id, v) in over {
        let ptr = map.iter().find(|(i, _, _)| i == id).unwrap_or_else(|| panic!("no param {id}")).1;
        let val = unsafe {
            match ptr {
                ParamPtr::FloatParam(p) => {
                    let p = &*p;
                    assert!((p.preview_plain(p.preview_normalized(*v as f32)) - *v as f32).abs() < 1e-3 * (1.0 + v.abs() as f32), "{id} out of range: {v}");
                    json!({ "f32": *v as f32 })
                }
                ParamPtr::IntParam(p) => {
                    let p = &*p;
                    assert_eq!(p.preview_plain(p.preview_normalized(*v as i32)), *v as i32, "{id} out of range");
                    json!({ "i32": *v as i32 })
                }
                ParamPtr::BoolParam(_) => json!({ "bool": *v != 0.0 }),
                _ => panic!("{id}"),
            }
        };
        out.insert(id.to_string(), val);
    }
    json!({ "version": env!("CARGO_PKG_VERSION"), "params": out, "fields": {} })
}

fn state(p: &Preset) -> State {
    let mut s = State::default();
    for (id, x) in p.1 {
        let f = *x as f32;
        match *id {
            "depth" => s.depth = f,
            "sharp" => s.sharp = f,
            "select" => s.select = f,
            "att" => s.attack_ms = f,
            "rel" => s.release_ms = f,
            "hard" => s.hard = f != 0.0,
            "chan" => s.chan = f as usize,
            "delta" => s.delta = f != 0.0,
            "carve" => s.carve = f != 0.0,
            "gmix" => s.mix = f,
            "lo" => s.lo = f,
            "hi" => s.hi = f,
            id if id.starts_with("nf") => s.nodes[id[2..].parse::<usize>().unwrap()].0 = f,
            id if id.starts_with("ng") => s.nodes[id[2..].parse::<usize>().unwrap()].1 = f,
            other => panic!("state(): unhandled {other}"),
        }
    }
    s
}

#[test]
fn factory_presets_are_valid() {
    let params = SalveParams::default();
    let dir = std::env::var("PRESET_OUT").ok();
    for p in PRESETS {
        assert!(!p.0.contains('/'));
        let v = build(&params, p.1);
        let _: nih_plug::wrapper::state::PluginState = serde_json::from_value(v.clone()).unwrap();
        let st = state(p);
        let mut e = Engine::new(48000.0);
        let mut s = 11u32;
        let mut l: Vec<f32> = (0..48000).map(|i| {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            0.4 * ((s >> 8) as f32 / 8388608.0 - 1.0) + 0.3 * (i as f32 * 0.37).sin()
        }).collect();
        let mut r = l.clone();
        for (a, b) in l.chunks_mut(512).zip(r.chunks_mut(512)) { e.process(a, b, &st); }
        let pk = l.iter().chain(r.iter()).fold(0.0f32, |a, v| a.max(v.abs()));
        assert!(pk.is_finite() && pk <= 1.0, "{}: {pk}", p.0);
        if let Some(d) = &dir {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(format!("{d}/{}.awkp", p.0), serde_json::to_string_pretty(&v).unwrap()).unwrap();
        }
    }
    assert!(PRESETS.len() >= 20);
}
