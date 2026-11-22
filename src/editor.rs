//! SALVE editor. Porcelain body, a deep plum display window: spectrum in and out,
//! the cuts hanging from the top in coral, and the band-editor curve in mint with six
//! draggable nodes (drag = frequency + amount, double-click = back to 100%).
//! LO / HI tabs at the bottom of the display limit the range.

use crate::analyzer::Analyzer;
use crate::awk_ui::*;
use crate::presets::PresetBar;
use crate::res::{band_freq, BANDS};
use crate::{get, range_at, weight_at, SalveParams, Shared, CHANNELS, NODES};
use nice_plug::prelude::*;
use nice_plug_egui::create_egui_editor;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind};
use std::sync::Arc;

pub const WIDTH: u32 = 1000;
pub const HEIGHT: u32 = 660;

/// Theme palette (dark by default; LIGHT in the footer). Display colours are shared.
struct Pal {
    body: Color32,
    body_lo: Color32,
    ink: Color32,
    dim: Color32,
    edge: Color32,
    track: Color32,
    disc_hi: Color32,
    disc_lo: Color32,
    on_bg: Color32,
    out_key: Color32,
}
const LIGHT: Pal = Pal {
    body: rgb(0xf3, 0xee, 0xe6),
    body_lo: rgb(0xe7, 0xdf, 0xd3),
    ink: rgb(0x2b, 0x22, 0x30),
    dim: rgb(0x8a, 0x7e, 0x86),
    edge: rgb(0xd6, 0xcc, 0xbf),
    track: rgb(0xe0, 0xd7, 0xca),
    disc_hi: rgb(0xfd, 0xfa, 0xf5),
    disc_lo: rgb(0xe9, 0xe1, 0xd5),
    on_bg: rgb(0x2b, 0x22, 0x30),
    out_key: rgb(0xc9, 0x9f, 0x86),
};
static LIGHT_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn pal() -> &'static Pal {
    if LIGHT_ON.load(std::sync::atomic::Ordering::Relaxed) {
        &LIGHT
    } else {
        &sc().dark
    }
}
/// Colour schemes (dark body + display/accent colours). Light mode keeps its own body.
struct Scheme {
    #[allow(dead_code)]
    name: &'static str,
    dark: Pal,
    plum: Color32,
    grid: Color32,
    spec_in: Color32,
    spec_out: Color32,
    coral: Color32,
    mint: Color32,
    key: Color32,
}
const SCHEMES: [Scheme; 6] = [
    Scheme {
        name: "ORIGINAL",
        dark: Pal { body: rgb(0x14, 0x10, 0x17), body_lo: rgb(0x1c, 0x16, 0x20), ink: rgb(0xf1, 0xe6, 0xdc), dim: rgb(0x9a, 0x8d, 0x97), edge: rgb(0x34, 0x2a, 0x3a), track: rgb(0x2c, 0x24, 0x31), disc_hi: rgb(0x3a, 0x30, 0x40), disc_lo: rgb(0x1d, 0x17, 0x21), on_bg: rgb(0x3b, 0x2e, 0x45), out_key: rgb(0xf3, 0xd9, 0xc6) },
        plum: rgb(0x22, 0x1a, 0x27),
        grid: rgb(0x3a, 0x2f, 0x40),
        spec_in: rgb(0x5e, 0x4f, 0x66),
        spec_out: rgb(0xf3, 0xd9, 0xc6),
        coral: rgb(0xff, 0x6f, 0x59),
        mint: rgb(0x8f, 0xe3, 0xc8),
        key: rgb(0x7f, 0xb2, 0xff),
    },
    Scheme {
        name: "GRAPHITE GOLD",
        dark: Pal { body: rgb(0x12, 0x13, 0x14), body_lo: rgb(0x19, 0x1a, 0x1c), ink: rgb(0xec, 0xea, 0xe6), dim: rgb(0x8e, 0x8d, 0x8a), edge: rgb(0x2c, 0x2d, 0x30), track: rgb(0x26, 0x27, 0x2a), disc_hi: rgb(0x34, 0x35, 0x3a), disc_lo: rgb(0x18, 0x19, 0x1b), on_bg: rgb(0x2e, 0x2f, 0x33), out_key: rgb(0xf4, 0xe6, 0xc8) },
        plum: rgb(0x0b, 0x0b, 0x0c),
        grid: rgb(0x26, 0x27, 0x2a),
        spec_in: rgb(0x4a, 0x4b, 0x50),
        spec_out: rgb(0xf4, 0xe6, 0xc8),
        coral: rgb(0xf2, 0xb8, 0x4b),
        mint: rgb(0xf4, 0xf1, 0xea),
        key: rgb(0x7f, 0xb2, 0xff),
    },
    Scheme {
        name: "DEEP SEA",
        dark: Pal { body: rgb(0x0c, 0x14, 0x18), body_lo: rgb(0x11, 0x1c, 0x21), ink: rgb(0xe3, 0xf1, 0xf2), dim: rgb(0x7f, 0x97, 0x99), edge: rgb(0x1f, 0x32, 0x38), track: rgb(0x1a, 0x2a, 0x30), disc_hi: rgb(0x25, 0x39, 0x3f), disc_lo: rgb(0x0f, 0x1a, 0x1e), on_bg: rgb(0x1d, 0x3a, 0x40), out_key: rgb(0xd8, 0xf4, 0xf2) },
        plum: rgb(0x07, 0x10, 0x14),
        grid: rgb(0x18, 0x28, 0x2e),
        spec_in: rgb(0x3a, 0x5a, 0x62),
        spec_out: rgb(0xd8, 0xf4, 0xf2),
        coral: rgb(0xff, 0x8a, 0x6b),
        mint: rgb(0x4f, 0xe3, 0xd0),
        key: rgb(0xb5, 0x9c, 0xff),
    },
    Scheme {
        name: "EMBER",
        dark: Pal { body: rgb(0x14, 0x11, 0x10), body_lo: rgb(0x1c, 0x18, 0x16), ink: rgb(0xf2, 0xe9, 0xe1), dim: rgb(0x95, 0x8a, 0x82), edge: rgb(0x33, 0x2b, 0x27), track: rgb(0x2b, 0x24, 0x20), disc_hi: rgb(0x3b, 0x32, 0x2d), disc_lo: rgb(0x1a, 0x15, 0x13), on_bg: rgb(0x3a, 0x2a, 0x22), out_key: rgb(0xff, 0xe1, 0xc4) },
        plum: rgb(0x0c, 0x0a, 0x09),
        grid: rgb(0x2a, 0x23, 0x20),
        spec_in: rgb(0x5a, 0x4c, 0x44),
        spec_out: rgb(0xff, 0xe1, 0xc4),
        coral: rgb(0xff, 0x5a, 0x36),
        mint: rgb(0xff, 0xc4, 0x6b),
        key: rgb(0x6f, 0xc3, 0xff),
    },
    Scheme {
        name: "ARCTIC ROSE",
        dark: Pal { body: rgb(0x0f, 0x11, 0x15), body_lo: rgb(0x15, 0x18, 0x20), ink: rgb(0xe8, 0xed, 0xf5), dim: rgb(0x86, 0x90, 0xa0), edge: rgb(0x26, 0x2c, 0x38), track: rgb(0x20, 0x25, 0x2f), disc_hi: rgb(0x2c, 0x33, 0x40), disc_lo: rgb(0x14, 0x17, 0x1d), on_bg: rgb(0x26, 0x30, 0x48), out_key: rgb(0xdf, 0xe8, 0xff) },
        plum: rgb(0x09, 0x0b, 0x0f),
        grid: rgb(0x1f, 0x24, 0x30),
        spec_in: rgb(0x44, 0x50, 0x6a),
        spec_out: rgb(0xdf, 0xe8, 0xff),
        coral: rgb(0xff, 0x4d, 0x7d),
        mint: rgb(0x7a, 0xa8, 0xff),
        key: rgb(0x5e, 0xf0, 0xc0),
    },
    Scheme {
        name: "SAGE CLAY",
        dark: Pal { body: rgb(0x11, 0x14, 0x12), body_lo: rgb(0x17, 0x1b, 0x18), ink: rgb(0xe9, 0xef, 0xe8), dim: rgb(0x8a, 0x96, 0x8c), edge: rgb(0x29, 0x30, 0x2b), track: rgb(0x23, 0x29, 0x25), disc_hi: rgb(0x30, 0x38, 0x33), disc_lo: rgb(0x15, 0x19, 0x16), on_bg: rgb(0x2c, 0x3a, 0x30), out_key: rgb(0xee, 0xf3, 0xdc) },
        plum: rgb(0x0b, 0x0d, 0x0c),
        grid: rgb(0x22, 0x28, 0x24),
        spec_in: rgb(0x4b, 0x57, 0x4e),
        spec_out: rgb(0xee, 0xf3, 0xdc),
        coral: rgb(0xe8, 0xa0, 0x6a),
        mint: rgb(0xb5, 0xe4, 0x8c),
        key: rgb(0x8f, 0xb8, 0xff),
    },
];
static SCHEME: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
fn fade(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}
fn sc() -> &'static Scheme {
    &SCHEMES[SCHEME.load(std::sync::atomic::Ordering::Relaxed).min(SCHEMES.len() - 1)]
}

struct St {
    presets: PresetBar,
    skin: Skin,
    an_in: Analyzer,
    an_out: Analyzer,
    an_key: Analyzer,
    cut: [f32; BANDS],
}

fn new_state(name: &'static str) -> St {
    let mut skin = Skin::new(Look::Riso, sc().coral, "SALVE", env!("CARGO_PKG_VERSION"), "");
    skin.brand = Some(sc().coral);
    St { presets: PresetBar::new(name), skin, an_in: Analyzer::default(), an_out: Analyzer::default(), an_key: Analyzer::default(), cut: [0.0; BANDS] }
}

pub(crate) fn create(params: Arc<SalveParams>, shared: Arc<Shared>) -> Option<crate::nice_app::AwkEditor> {
    let state = params.editor_state.clone();
    let st = new_state("Salve");
    create_egui_editor(state, nice_plug_egui::RepaintNotifier::new(), nice_plug_egui::EguiNiceSettings::default(), crate::nice_app::AwkApp::new(fonts, {
        let mut __st = st;
        Box::new(move |ctx: &mut egui::Ui, setter: &ParamSetter| {
            let st = &mut __st;
            frame(ctx, setter, st, params.as_ref(), shared.as_ref())
        })
    }))
}

fn fonts(ctx: &egui::Context) {
    install_fonts(ctx);
    let mut f = FontDefinitions::default();
    let faces: [(&str, &'static [u8]); 6] = [
        ("awk-d", include_bytes!("../assets/Michroma-Regular.ttf")),
        ("awk-l", include_bytes!("../assets/BarlowCondensed-SemiBold.ttf")),
        ("awk-b", include_bytes!("../assets/BarlowCondensed-Bold.ttf")),
        ("awk-m", include_bytes!("../assets/ShareTechMono-Regular.ttf")),
        ("pat-m", include_bytes!("../assets/IBMPlexMono-Medium.ttf")),
        ("salve", include_bytes!("../assets/DMSerifDisplay-Regular.ttf")),
    ];
    let fallbacks: Vec<String> = ["Ubuntu-Light", "NotoEmoji-Regular", "emoji-icon-font"].iter().filter(|n| f.font_data.contains_key(**n)).map(|s| s.to_string()).collect();
    for (name, bytes) in faces {
        f.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
        let mut fam = vec![name.to_string()];
        fam.extend(fallbacks.iter().cloned());
        f.families.insert(FontFamily::Name(name.into()), fam);
    }
    for old in ["block", "marker", "pat-h"] {
        let mut fam = vec!["awk-b".to_string()];
        fam.extend(fallbacks.iter().cloned());
        f.families.insert(FontFamily::Name(old.into()), fam);
    }
    if let Some(p) = f.families.get_mut(&FontFamily::Proportional) {
        p.insert(0, "awk-l".into());
    }
    if let Some(m) = f.families.get_mut(&FontFamily::Monospace) {
        m.insert(0, "awk-m".into());
    }
    ctx.set_fonts(f);
}
fn serif(s: f32) -> FontId {
    FontId::new(s, FontFamily::Name("salve".into()))
}
fn plex(s: f32) -> FontId {
    FontId::new(s, FontFamily::Name("pat-m".into()))
}
fn rr(r: f32) -> CornerRadius {
    CornerRadius::same(r as u8)
}
fn set_int(setter: &ParamSetter, p: &IntParam, v: i32) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}
fn set_bool(setter: &ParamSetter, p: &BoolParam, v: bool) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}
fn hz_text(f: f32) -> String {
    if f >= 1000.0 {
        format!("{:.1}k", f / 1000.0)
    } else {
        format!("{:.0}", f)
    }
}

/// A rounded pill button; `on` fills it with ink.
fn pill(ui: &mut egui::Ui, r: Rect, s: &str, on: bool, accent: Color32, id: impl std::hash::Hash + std::fmt::Debug) -> Response {
    let resp = ui.interact(r, ui.scope_id().with(("pill", id)), Sense::click());
    let pa = ui.painter();
    let hot = resp.hovered();
    let rad = r.height() * 0.5;
    if on {
        pa.rect_filled(r, rr(rad), pal().on_bg);
        txt(pa, r.center(), Align2::CENTER_CENTER, s, labb(12.5), accent, 1.6);
    } else {
        pa.rect_filled(r, rr(rad), if hot { pal().body_lo } else { pal().body });
        pa.rect_stroke(r, rr(rad), Stroke::new(1.0, pal().edge), StrokeKind::Inside);
        txt(pa, r.center(), Align2::CENTER_CENTER, s, labb(12.5), if hot { pal().ink } else { pal().dim }, 1.6);
    }
    resp
}

/// A soft porcelain knob: shadowed disc, a track ring, the value arc in coral, value inside.
#[allow(clippy::too_many_arguments)]
fn ring(ui: &mut egui::Ui, setter: &ParamSetter, c: Pos2, r: f32, param: &FloatParam, label: &str, value: &str, accent: Color32) -> Response {
    let rect = Rect::from_center_size(c, vec2(r * 2.0 + 16.0, r * 2.0 + 16.0));
    let uid = ui.scope_id().with(("ring", label));
    let resp = ui.interact(rect, uid, Sense::click_and_drag());
    knob_input(ui, setter, &resp, uid.with("d"), param);
    let pa = ui.painter();
    let v = param.unmodulated_normalized_value();
    let hot = resp.hovered() || resp.dragged();
    let arc = |u0: f32, u1: f32, rad: f32, w: f32, col: Color32| {
        let n = 48;
        let pts: Vec<Pos2> = (0..=n).map(|i| polar(c, ang(u0 + (u1 - u0) * i as f32 / n as f32), rad)).collect();
        pa.add(Shape::line(pts, Stroke::new(w, col)));
    };
    disc_shadow(pa, c, r, vec2(0.0, 6.0), 0.35);
    disc_radial(pa, c, r, pal().disc_hi, pal().disc_lo, vec2(-r * 0.2, -r * 0.35));
    pa.circle_stroke(c, r, Stroke::new(1.0, pal().edge));
    arc(0.0, 1.0, r + 6.0, 4.0, pal().track);
    if v > 0.002 {
        arc(0.0, v, r + 6.0, 4.0, accent);
    }
    // the dot rides out near the rim, right under the end of the arc
    let a = ang(v);
    pa.circle_filled(polar(c, a, r - 5.0), if hot { 3.8 } else { 3.2 }, accent);
    txt(pa, c + vec2(0.0, 2.0), Align2::CENTER_CENTER, value, plex(if r > 30.0 { 13.0 } else { 11.0 }), pal().ink, 0.0);
    txt(pa, c + vec2(0.0, r + 26.0), Align2::CENTER_CENTER, label, labb(13.0), if hot { pal().ink } else { pal().dim }, 2.4);
    resp
}

fn frame(root: &mut egui::Ui, setter: &ParamSetter, st: &mut St, p: &SalveParams, sh: &Shared) {
    let ctx_owned = root.ctx().clone();
    let ctx = &ctx_owned;
    ctx.request_repaint_after(std::time::Duration::from_millis(33));
    st.skin.begin(ctx);
    st.an_in.update(&sh.spec_in);
    st.an_out.update(&sh.spec_out);
    if p.carve.value() {
        st.an_key.update(&sh.spec_key);
    }
    for k in 0..BANDS {
        let t = get(&sh.cut[k]);
        st.cut[k] = if t > st.cut[k] { t } else { st.cut[k] * 0.8 + t * 0.2 };
    }
    LIGHT_ON.store(p.light.value(), std::sync::atomic::Ordering::Relaxed);
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(pal().body)).show(root, |ui| {
        let whole = ui.max_rect();
        let o = whole.min.to_vec2();
        let at = |x: f32, y: f32| pos2(x, y) + o;
        let pa = ui.painter().clone();
        // header: wordmark + switches
        txt(&pa, at(28.0, 86.0), Align2::LEFT_CENTER, "Salve", serif(46.0), pal().ink, 0.0);
        txt(&pa, at(156.0, 80.0), Align2::LEFT_CENTER, "resonance suppressor", plex(11.0), pal().dim, 0.5);
        txt(&pa, at(156.0, 96.0), Align2::LEFT_CENTER, "zero latency · 48 bands", plex(11.0), pal().dim, 0.5);
        let chan = p.chan.value().clamp(0, 4) as usize;
        txt(&pa, at(394.0, 72.0), Align2::LEFT_CENTER, "CHANNELS", labb(10.5), pal().dim, 2.0);
        for (i, s) in CHANNELS.iter().enumerate() {
            let r = Rect::from_min_size(at(394.0 + i as f32 * 66.0, 82.0), vec2(62.0, 24.0));
            if pill(ui, r, s, chan == i, sc().mint, ("ch", i)).clicked() {
                set_int(setter, &p.chan, i as i32);
            }
        }
        let hard = p.hard.value();
        txt(&pa, at(742.0, 72.0), Align2::LEFT_CENTER, "KNEE", labb(10.5), pal().dim, 2.0);
        for (i, s) in ["SOFT", "HARD"].iter().enumerate() {
            let r = Rect::from_min_size(at(742.0 + i as f32 * 58.0, 82.0), vec2(54.0, 24.0));
            if pill(ui, r, s, hard == (i == 1), sc().mint, ("kn", i)).clicked() {
                set_bool(setter, &p.hard, i == 1);
            }
        }
        let delta = p.delta.value();
        let dr = Rect::from_min_size(at(878.0, 82.0), vec2(94.0, 24.0));
        txt(&pa, at(878.0, 72.0), Align2::LEFT_CENTER, "LISTEN", labb(10.5), pal().dim, 2.0);
        if pill(ui, dr, "Δ DELTA", delta, sc().coral, "delta").on_hover_text("Hear only what SALVE takes out").clicked() {
            set_bool(setter, &p.delta, !delta);
        }
        display(ui, setter, st, p, Rect::from_min_size(at(24.0, 122.0), vec2(952.0, 300.0)));
        // knobs
        let ar = |v: f32| if v < 10.0 { format!("{:.1} ms", v) } else { format!("{:.0} ms", v) };
        let row: [(&FloatParam, &str, String); 5] = [
            (&p.depth, "DEPTH", format!("{:.0}%", p.depth.value() * 100.0)),
            (&p.sharp, "SHARPNESS", format!("{:.0}%", p.sharp.value() * 100.0)),
            if p.carve.value() {
                (&p.select, "sc().key THRESH", format!("{:.0} dB", -60.0 + 36.0 * p.select.value()))
            } else {
                (&p.select, "SELECTIVITY", format!("{:.0}%", p.select.value() * 100.0))
            },
            (&p.att, "ATTACK", ar(p.att.value())),
            (&p.rel, "RELEASE", ar(p.rel.value())),
        ];
        for (i, (prm, l, v)) in row.iter().enumerate() {
            ring(ui, setter, at(96.0 + i as f32 * 140.0, 508.0), 36.0, prm, l, v, sc().coral);
        }
        pa.line_segment([at(762.0, 462.0), at(762.0, 568.0)], Stroke::new(1.0, pal().edge));
        ring(ui, setter, at(828.0, 508.0), 28.0, &p.mix, "MIX", &format!("{:.0}%", p.mix.value() * 100.0), sc().mint);
        ring(ui, setter, at(928.0, 508.0), 28.0, &p.trim, "TRIM", &format!("{:+.1}", p.trim.value()), sc().mint);
        footer(ui, setter, p, sh, whole);
        let _ = st.presets.strip(ui, setter, p, whole, &st.skin, &p.mix, &p.bypass);
    });
}

fn display(ui: &mut egui::Ui, setter: &ParamSetter, st: &mut St, p: &SalveParams, r: Rect) {
    let pa = ui.painter().clone();
    pa.rect_filled(r.translate(vec2(0.0, 6.0)), rr(18.0), Color32::from_black_alpha(26));
    pa.rect_filled(r, rr(18.0), sc().plum);
    let inner = r.shrink2(vec2(16.0, 12.0));
    let clip = pa.with_clip_rect(r.shrink(2.0));
    let ux = |f: f32| ((f / 20.0).ln() / 1000f32.ln()).clamp(0.0, 1.0);
    let x_of = |f: f32| inner.left() + ux(f) * inner.width();
    let f_of = |x: f32| 20.0 * 1000f32.powf(((x - inner.left()) / inner.width()).clamp(0.0, 1.0));
    // grid
    for f in [50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0] {
        let x = x_of(f);
        clip.line_segment([pos2(x, inner.top()), pos2(x, inner.bottom() - 18.0)], Stroke::new(1.0, sc().grid));
        txt(&clip, pos2(x, inner.bottom() - 6.0), Align2::CENTER_CENTER, hz_text(f), mono(10.0), sc().spec_in, 0.0);
    }
    // spectrum: -84..0 dBFS over the lower part
    let (sy0, sy1) = (inner.top() + 40.0, inner.bottom() - 20.0);
    let y_spec = |db: f32| sy1 - ((db + 84.0) / 84.0).clamp(0.0, 1.0) * (sy1 - sy0);
    let n = 220;
    let sm = |an: &Analyzer, u: f32| {
        let d = 1.0 / n as f32;
        0.25 * an.at(u - d) + 0.5 * an.at(u) + 0.25 * an.at(u + d)
    };
    let pin: Vec<Pos2> = (0..=n).map(|i| i as f32 / n as f32).map(|u| pos2(inner.left() + u * inner.width(), y_spec(sm(&st.an_in, u)))).collect();
    let mut fill = pin.clone();
    fill.push(pos2(inner.right(), sy1));
    fill.push(pos2(inner.left(), sy1));
    for w in pin.windows(2) {
        let q = [w[0], w[1], pos2(w[1].x, sy1), pos2(w[0].x, sy1)];
        clip.add(Shape::convex_polygon(q.to_vec(), fade(sc().spec_in, 110), Stroke::NONE));
    }
    clip.add(Shape::line(pin, Stroke::new(1.0, sc().spec_in)));
    let pout: Vec<Pos2> = (0..=n).map(|i| i as f32 / n as f32).map(|u| pos2(inner.left() + u * inner.width(), y_spec(sm(&st.an_out, u)))).collect();
    clip.add(Shape::line(pout, Stroke::new(1.6, sc().spec_out)));
    if p.carve.value() {
        let pk: Vec<Pos2> = (0..=n).map(|i| i as f32 / n as f32).map(|u| pos2(inner.left() + u * inner.width(), y_spec(sm(&st.an_key, u)))).collect();
        clip.add(Shape::line(pk, Stroke::new(1.6, sc().key)));
    }
    // cuts hanging from the top: 0..24 dB
    let top = inner.top() + 6.0;
    let y_cut = |db: f32| top + (db / 24.0).clamp(0.0, 1.0).sqrt() * 150.0;
    let cut_at = |f: f32| {
        let kf = ((f / band_freq(0)).ln() / (band_freq(BANDS - 1) / band_freq(0)).ln() * (BANDS - 1) as f32).clamp(0.0, (BANDS - 1) as f32);
        let i = kf as usize;
        let j = (i + 1).min(BANDS - 1);
        let t = kf - i as f32;
        let t = t * t * (3.0 - 2.0 * t);
        st.cut[i] + (st.cut[j] - st.cut[i]) * t
    };
    let cpts: Vec<Pos2> = (0..=n).map(|i| i as f32 / n as f32).map(|u| {
        let x = inner.left() + u * inner.width();
        pos2(x, y_cut(cut_at(f_of(x))))
    }).collect();
    for w in cpts.windows(2) {
        let q = [pos2(w[0].x, top), pos2(w[1].x, top), w[1], w[0]];
        clip.add(Shape::convex_polygon(q.to_vec(), fade(sc().coral, 150), Stroke::NONE));
    }
    clip.add(Shape::line(cpts.clone(), Stroke::new(1.8, sc().coral)));
    clip.line_segment([pos2(inner.left(), top), pos2(inner.right(), top)], Stroke::new(1.0, fade(sc().coral, 90)));
    let worst = st.cut.iter().cloned().fold(0.0f32, f32::max);
    let sc_y = r.bottom() + 16.0;
    txt(&pa, pos2(r.left() + 262.0, sc_y), Align2::LEFT_CENTER, "SIDECHAIN", labb(10.5), pal().dim, 1.5);
    let carve_on = p.carve.value();
    for (i, s) in ["OFF", "CARVE"].iter().enumerate() {
        let tr = Rect::from_min_size(pos2(r.left() + 340.0 + i as f32 * 66.0, sc_y - 11.0), vec2(62.0, 22.0));
        if pill(ui, tr, s, carve_on == (i == 1), sc().key, ("sc", i)).on_hover_text("CARVE: route a kick or vocal into SALVE's sidechain input; it cuts this track only where the key is loud").clicked() {
            setter.begin_set_parameter(&p.carve);
            setter.set_parameter(&p.carve, i == 1);
            setter.end_set_parameter(&p.carve);
        }
    }
    let lb = txt(&pa, pos2(r.left() + 6.0, r.bottom() + 16.0), Align2::LEFT_CENTER, "MAX CUT", labb(10.5), pal().dim, 1.5);
    txt(&pa, pos2(lb.right() + 8.0, r.bottom() + 16.0), Align2::LEFT_CENTER, format!("−{:.1} dB", worst), plex(12.0), sc().coral, 0.0);
    // LO / HI shading + tabs
    let (lo, hi) = (p.lo.value(), p.hi.value());
    let (xl, xh) = (x_of(lo), x_of(hi));
    if xl > inner.left() + 0.5 {
        clip.rect_filled(Rect::from_min_max(pos2(r.left(), r.top()), pos2(xl, r.bottom())), 0.0, Color32::from_black_alpha(90));
    }
    if xh < inner.right() - 0.5 {
        clip.rect_filled(Rect::from_min_max(pos2(xh, r.top()), pos2(r.right(), r.bottom())), 0.0, Color32::from_black_alpha(90));
    }
    for (is_hi, x, prm, lbl) in [(false, xl, &p.lo, "LO"), (true, xh, &p.hi, "HI")] {
        let tab = Rect::from_center_size(pos2(x + if is_hi { -15.0 } else { 15.0 }, inner.bottom() - 30.0), vec2(30.0, 18.0));
        let id = ui.scope_id().with(("lim", lbl));
        let resp = ui.interact(tab.expand(4.0), id, Sense::click_and_drag());
        if resp.drag_started() {
            setter.begin_set_parameter(prm);
        }
        if resp.dragged() {
            if let Some(q) = resp.interact_pointer_pos() {
                let f = f_of(q.x + if is_hi { 15.0 } else { -15.0 });
                let f = if is_hi { f.max(lo * 1.5) } else { f.min(hi / 1.5) };
                setter.set_parameter(prm, f.clamp(20.0, 20000.0));
            }
        }
        if resp.drag_stopped() {
            setter.end_set_parameter(prm);
        }
        if resp.double_clicked() {
            setter.begin_set_parameter(prm);
            setter.set_parameter(prm, if is_hi { 20000.0 } else { 20.0 });
            setter.end_set_parameter(prm);
        }
        clip.line_segment([pos2(x, inner.top()), pos2(x, inner.bottom() - 20.0)], Stroke::new(1.0, if resp.hovered() || resp.dragged() { sc().mint } else { sc().spec_in }));
        clip.rect_filled(tab, rr(5.0), if resp.hovered() || resp.dragged() { sc().mint } else { sc().grid });
        txt(&clip, tab.center(), Align2::CENTER_CENTER, lbl, labb(11.0), if resp.hovered() || resp.dragged() { sc().plum } else { sc().spec_out }, 1.0);
        if resp.hovered() || resp.dragged() {
            txt(&clip, tab.center() + vec2(0.0, -18.0), Align2::CENTER_CENTER, prm.to_string(), mono(10.0), sc().mint, 0.0);
        }
    }
    // band editor: weight 0..2 mapped around a centre line
    let (nf, ng) = (p.node_f(), p.node_g());
    let nodes: [(f32, f32); NODES] = std::array::from_fn(|i| (nf[i].value(), ng[i].value()));
    let mid = 0.5 * (inner.top() + inner.bottom() - 20.0);
    let y_w = |w: f32| mid - (w - 1.0) * 70.0;
    clip.line_segment([pos2(inner.left(), mid), pos2(inner.right(), mid)], Stroke::new(1.0, fade(sc().mint, 22)));
    let wpts: Vec<Pos2> = (0..=n).map(|i| {
        let x = inner.left() + i as f32 / n as f32 * inner.width();
        let f = f_of(x);
        pos2(x, y_w(weight_at(&nodes, f) * range_at(lo, hi, f)))
    }).collect();
    clip.add(Shape::line(wpts, Stroke::new(2.0, sc().mint)));
    for i in 0..NODES {
        let c = pos2(x_of(nodes[i].0), y_w(nodes[i].1));
        let id = ui.scope_id().with(("node", i));
        let resp = ui.interact(Rect::from_center_size(c, vec2(26.0, 26.0)), id, Sense::click_and_drag()).on_hover_text("Drag: frequency and amount. Double-click: back to 100%");
        if resp.drag_started() {
            setter.begin_set_parameter(nf[i]);
            setter.begin_set_parameter(ng[i]);
        }
        if resp.dragged() {
            if let Some(q) = resp.interact_pointer_pos() {
                setter.set_parameter(nf[i], f_of(q.x).clamp(20.0, 20000.0));
                setter.set_parameter(ng[i], (1.0 + (mid - q.y) / 70.0).clamp(0.0, 2.0));
            }
        }
        if resp.drag_stopped() {
            setter.end_set_parameter(nf[i]);
            setter.end_set_parameter(ng[i]);
        }
        if resp.double_clicked() {
            setter.begin_set_parameter(ng[i]);
            setter.set_parameter(ng[i], 1.0);
            setter.end_set_parameter(ng[i]);
        }
        let hot = resp.hovered() || resp.dragged();
        clip.circle_filled(c, if hot { 11.0 } else { 9.0 }, sc().plum);
        clip.circle_stroke(c, if hot { 11.0 } else { 9.0 }, Stroke::new(2.0, sc().mint));
        txt(&clip, c, Align2::CENTER_CENTER, format!("{}", i + 1), labb(11.0), sc().mint, 0.0);
        if hot {
            let s = format!("{} Hz · {:.0}%", hz_text(nodes[i].0), nodes[i].1 * 100.0);
            let tr = Rect::from_center_size(c + vec2(0.0, -26.0), vec2(108.0, 18.0));
            clip.rect_filled(tr, rr(9.0), sc().mint);
            txt(&clip, tr.center(), Align2::CENTER_CENTER, s, mono(10.0), sc().plum, 0.0);
        }
    }
    // legend under the window
    let mut x = r.right() - 290.0;
    let carve = p.carve.value();
    if carve {
        x -= 52.0;
    }
    let keys: Vec<(Color32, &str)> = if carve { vec![(sc().coral, "CUT"), (sc().mint, "BAND EDITOR"), (pal().out_key, "OUT"), (sc().spec_in, "IN"), (sc().key, "sc().key")] } else { vec![(sc().coral, "CUT"), (sc().mint, "BAND EDITOR"), (pal().out_key, "OUT"), (sc().spec_in, "IN")] };
    for (col, t) in keys {
        pa.circle_filled(pos2(x + 4.0, r.bottom() + 16.0), 3.5, col);
        let rct = txt(&pa, pos2(x + 12.0, r.bottom() + 16.0), Align2::LEFT_CENTER, t, labb(10.5), pal().dim, 1.5);
        x = rct.right() + 16.0;
    }
    pa.rect_stroke(r, rr(18.0), Stroke::new(1.0, pal().edge), StrokeKind::Inside);
}

fn footer(ui: &mut egui::Ui, setter: &ParamSetter, p: &SalveParams, sh: &Shared, whole: Rect) {
    let r = Rect::from_min_max(pos2(whole.left(), whole.bottom() - 46.0), whole.right_bottom());
    let pa = ui.painter().clone();
    pa.rect_filled(r, 0.0, pal().body_lo);
    pa.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, pal().edge));
    let cy = r.center().y;
    let gr = -sh.meters.db(4);
    let safe = p.safe.value();
    let lbl = if safe { if gr > 0.1 { format!("SAFE −{:.1}", gr) } else { "SAFE 0 dB".to_string() } } else { "SAFE OFF".to_string() };
    if pill(ui, Rect::from_min_size(pos2(r.left() + 20.0, cy - 12.0), vec2(112.0, 24.0)), &lbl, safe, sc().coral, "safe").on_hover_text("Never over 0 dB").clicked() {
        set_bool(setter, &p.safe, !safe);
    }
    txt(&pa, pos2(r.left() + 152.0, cy), Align2::LEFT_CENTER, format!("RANGE  {} – {}", p.lo, p.hi), plex(11.0), pal().dim, 0.5);
    txt(&pa, pos2(r.left() + 390.0, cy), Align2::LEFT_CENTER, "LATENCY 0", plex(11.0), pal().dim, 0.5);
    let light = p.light.value();
    for (i, s) in ["DARK", "LIGHT"].iter().enumerate() {
        let tr = Rect::from_min_size(pos2(r.left() + 520.0 + i as f32 * 58.0, cy - 11.0), vec2(54.0, 22.0));
        if pill(ui, tr, s, light == (i == 1), sc().mint, ("theme", i)).clicked() {
            set_bool(setter, &p.light, i == 1);
        }
    }
    txt(&pa, pos2(r.right() - 20.0, cy), Align2::RIGHT_CENTER, format!("awk studios · salve v{}", env!("CARGO_PKG_VERSION")), plex(11.0), pal().dim, 0.5);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, State};
    use nice_plug::context::gui::GuiContextInner;
    use nice_plug::plugin::PluginState;
    use egui::{epaint, Event, ImageData, Modifiers, PointerButton, RawInput, TextureId, TextureWrapMode, Vec2};
    use std::collections::HashMap;

    struct Mock;
    impl GuiContextInner for Mock {
        fn plugin_api(&self) -> PluginApi {
            PluginApi::Vst3
        }
        fn request_restart(&self) {}
        unsafe fn raw_begin_set_parameter(&self, _p: ParamPtr) {}
        unsafe fn raw_set_parameter_normalized(&self, _p: ParamPtr, _n: f32) {}
        unsafe fn raw_end_set_parameter(&self, _p: ParamPtr) {}
        fn get_state(&self) -> PluginState {
            PluginState { version: String::new(), params: Default::default(), fields: Default::default() }
        }
        fn set_state(&self, _s: PluginState) {}
    }
    struct Tex {
        w: usize,
        h: usize,
        px: Vec<[f32; 4]>,
        repeat: bool,
    }
    fn apply_delta(texs: &mut HashMap<TextureId, Tex>, out: &egui::FullOutput) {
        for (id, d) in out.textures_delta.set.iter().flat_map(|(id, ds)| ds.iter().map(move |d| (id, d))) {
            let (w, h, px): (usize, usize, Vec<[f32; 4]>) = match &d.image {
                ImageData::Color(c) => (c.size[0], c.size[1], c.pixels.iter().map(|p| [p.r() as f32 / 255.0, p.g() as f32 / 255.0, p.b() as f32 / 255.0, p.a() as f32 / 255.0]).collect()),
            };
            let repeat = d.options.wrap_mode == TextureWrapMode::Repeat;
            if let Some(pos) = d.pos {
                let t = texs.get_mut(id).unwrap();
                for y in 0..h {
                    for x in 0..w {
                        t.px[(pos[1] + y) * t.w + pos[0] + x] = px[y * w + x];
                    }
                }
            } else {
                texs.insert(*id, Tex { w, h, px, repeat });
            }
        }
    }
    fn render(ctx: &egui::Context, out: egui::FullOutput, texs: &mut HashMap<TextureId, Tex>, path: &str) {
        let img = raster(ctx, out, texs);
        let bytes: Vec<u8> = img.iter().flat_map(|p| p.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8).collect::<Vec<_>>()).collect();
        std::fs::write(path, bytes).unwrap();
    }
    fn raster(ctx: &egui::Context, out: egui::FullOutput, texs: &mut HashMap<TextureId, Tex>) -> Vec<[f32; 3]> {
        apply_delta(texs, &out);
        let (w, h) = (WIDTH as usize, HEIGHT as usize);
        let mut img = vec![[0.0f32; 3]; w * h];
        let prims = ctx.tessellate(out.shapes, 1.0);
        for cp in prims {
            let epaint::Primitive::Mesh(mesh) = cp.primitive else { continue };
            let clip = cp.clip_rect;
            let tex = texs.get(&mesh.texture_id);
            for tri in mesh.indices.chunks(3) {
                let v = [mesh.vertices[tri[0] as usize], mesh.vertices[tri[1] as usize], mesh.vertices[tri[2] as usize]];
                let (x0, x1) = (v.iter().map(|q| q.pos.x).fold(f32::MAX, f32::min).max(clip.left()).max(0.0), v.iter().map(|q| q.pos.x).fold(f32::MIN, f32::max).min(clip.right()).min(w as f32));
                let (y0, y1) = (v.iter().map(|q| q.pos.y).fold(f32::MAX, f32::min).max(clip.top()).max(0.0), v.iter().map(|q| q.pos.y).fold(f32::MIN, f32::max).min(clip.bottom()).min(h as f32));
                let area = (v[1].pos.x - v[0].pos.x) * (v[2].pos.y - v[0].pos.y) - (v[2].pos.x - v[0].pos.x) * (v[1].pos.y - v[0].pos.y);
                if area.abs() < 1e-6 {
                    continue;
                }
                for py in (y0.floor() as usize)..(y1.ceil() as usize).min(h) {
                    for px in (x0.floor() as usize)..(x1.ceil() as usize).min(w) {
                        let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                        let e = |a: &epaint::Vertex, b: &epaint::Vertex| (b.pos.x - a.pos.x) * (fy - a.pos.y) - (fx - a.pos.x) * (b.pos.y - a.pos.y);
                        let (w0, w1, w2) = (e(&v[1], &v[2]) / area, e(&v[2], &v[0]) / area, e(&v[0], &v[1]) / area);
                        if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                            continue;
                        }
                        let mut c = [0.0f32; 4];
                        for i in 0..4 {
                            c[i] = (w0 * v[0].color.to_array()[i] as f32 + w1 * v[1].color.to_array()[i] as f32 + w2 * v[2].color.to_array()[i] as f32) / 255.0;
                        }
                        if let Some(t) = tex {
                            let mut u = w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x;
                            let mut vv = w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y;
                            if t.repeat {
                                u = u.rem_euclid(1.0);
                                vv = vv.rem_euclid(1.0);
                            }
                            let tx = ((u * t.w as f32) as usize).min(t.w - 1);
                            let ty = ((vv * t.h as f32) as usize).min(t.h - 1);
                            let s = t.px[ty * t.w + tx];
                            for i in 0..4 {
                                c[i] *= s[i];
                            }
                        }
                        let d = &mut img[py * w + px];
                        for i in 0..3 {
                            d[i] = c[i] + d[i] * (1.0 - c[3]);
                        }
                    }
                }
            }
        }
        img
    }
    fn raw(i: usize, ev: Vec<Event>) -> RawInput {
        RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(WIDTH as f32, HEIGHT as f32))), time: Some(i as f64 * 0.04), events: ev, ..Default::default() }
    }

    fn sig(n: usize, sr: f32) -> Vec<f32> {
        // pinkish noise + three resonances (a boomy 320 Hz, a harsh 2.9 kHz, a whistle 4.6 kHz)
        let mut seed = 77u32;
        let (mut b0, mut b1) = (0.0f32, 0.0f32);
        (0..n).map(|i| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let w = (seed >> 8) as f32 / 8388608.0 - 1.0;
            b0 = 0.995 * b0 + 0.06 * w;
            b1 = 0.6 * b1 + 0.4 * w;
            let t = i as f32 / sr;
            0.5 * (b0 + 0.25 * b1) + 0.05 * (2.0 * std::f32::consts::PI * 320.0 * t).sin() + 0.06 * (2.0 * std::f32::consts::PI * 2900.0 * t).sin() + 0.04 * (2.0 * std::f32::consts::PI * 4600.0 * t).sin()
        }).collect()
    }

    #[test]
    fn draws_and_clicks() {
        let ctx = egui::Context::default();
        fonts(&ctx);
        let p = SalveParams::default();
        let sh = Shared::default();
        let mock = Mock;
        let setter = ParamSetter::new(&mock);
        let mut st = new_state("SalveTest");
        let mut i = 0;
        for y in (50..650).step_by(19) {
            for x in (10..990).step_by(37) {
                let q = Pos2::new(x as f32, y as f32);
                let _ = ctx.run_ui(raw(i, vec![Event::PointerMoved(q), Event::PointerButton { pos: q, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]), |c| frame(c, &setter, &mut st, &p, &sh));
                let _ = ctx.run_ui(raw(i + 1, vec![Event::PointerMoved(q + Vec2::new(6.0, -6.0)), Event::PointerButton { pos: q + Vec2::new(6.0, -6.0), button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]), |c| frame(c, &setter, &mut st, &p, &sh));
                i += 2;
            }
        }
    }

    /// SHOT_DIR: 0 = default on the test signal, 1 = deep + sharp, node 4 pushed, LO/HI set, M/S, 2 = DELTA + HARD.
    #[test]
    fn screenshots() {
        let Ok(dir) = std::env::var("SHOT_DIR") else { return };
        let ctx = egui::Context::default();
        fonts(&ctx);
        let mock = Mock;
        let setter = ParamSetter::new(&mock);
        let mut texs = HashMap::new();
        let u = |n: &str, v: f32| FloatParam::new(n, v, FloatRange::Linear { min: 0.0, max: 1.0 });
        let hz = |n: &str, v: f32| FloatParam::new(n, v, FloatRange::Skewed { min: 20.0, max: 20000.0, factor: FloatRange::skew_factor(-2.2) });
        for n in 0..4 {
            let mut p = SalveParams::default();
            if n == 3 {
                p.carve = BoolParam::new("c", true);
                p.depth = u("d", 0.7);
                p.hi = hz("hi", 400.0);
            }
            if n == 1 {
                p.depth = u("d", 0.8);
                p.sharp = u("s", 0.75);
                p.ng3 = FloatParam::new("g", 1.6, FloatRange::Linear { min: 0.0, max: 2.0 });
                p.ng0 = FloatParam::new("g", 0.3, FloatRange::Linear { min: 0.0, max: 2.0 });
                p.lo = hz("lo", 90.0);
                p.hi = hz("hi", 14000.0);
                p.chan = IntParam::new("c", 2, IntRange::Linear { min: 0, max: 4 });
            }
            if n == 2 {
                p.depth = u("d", 0.7);
                p.delta = BoolParam::new("dl", true);
                p.hard = BoolParam::new("h", true);
            }
            let sh = Arc::new(Shared::default());
            let mut e = Engine::new(48000.0).with_shared(sh.clone());
            let st_a: State = p.state();
            let x = sig(48000, 48000.0);
            let (mut l, mut r) = (x.clone(), x.clone());
            // key for shot 3: a four-on-the-floor kick
            let kick: Vec<f32> = (0..x.len()).map(|i| {
                let t = (i % 24000) as f32 / 48000.0;
                0.8 * (-t * 14.0).exp() * (2.0 * std::f32::consts::PI * (50.0 + 90.0 * (-t * 40.0).exp()) * t).sin()
            }).collect();
            for ((a, b), k) in l.chunks_mut(256).zip(r.chunks_mut(256)).zip(kick.chunks(256)) {
                e.process_key(a, b, Some((k, k)), &st_a);
            }
            let mut st = new_state("SalveShot");
            let mut last = None;
            for f in 0..3 {
                last = Some(ctx.run_ui(raw(n * 10 + f, vec![]), |c| frame(c, &setter, &mut st, &p, &sh)));
                if f < 2 {
                    apply_delta(&mut texs, last.as_ref().unwrap());
                }
            }
            render(&ctx, last.unwrap(), &mut texs, &format!("{}/salve{}.rgb", dir, n));
        }
        if std::env::var("SCHEME_SHOTS").is_err() {
            return;
        }
        for k in 0..SCHEMES.len() {
            SCHEME.store(k, std::sync::atomic::Ordering::Relaxed);
            for n in [0usize, 3] {
                let mut p = SalveParams::default();
                if n == 3 {
                    p.carve = BoolParam::new("c", true);
                    p.depth = u("d", 0.7);
                    p.hi = hz("hi", 400.0);
                }
                let sh = Arc::new(Shared::default());
                let mut e = Engine::new(48000.0).with_shared(sh.clone());
                let st_a: State = p.state();
                let x = sig(48000, 48000.0);
                let (mut l, mut r) = (x.clone(), x.clone());
                let kick: Vec<f32> = (0..x.len()).map(|i| {
                    let t = (i % 24000) as f32 / 48000.0;
                    0.8 * (-t * 14.0).exp() * (2.0 * std::f32::consts::PI * (50.0 + 90.0 * (-t * 40.0).exp()) * t).sin()
                }).collect();
                for ((a, b), kk) in l.chunks_mut(256).zip(r.chunks_mut(256)).zip(kick.chunks(256)) {
                    e.process_key(a, b, Some((kk, kk)), &st_a);
                }
                let mut st = new_state("SalveShot");
                let mut last = None;
                for f in 0..3 {
                    last = Some(ctx.run_ui(raw(1000 + k * 100 + n * 10 + f, vec![]), |c| frame(c, &setter, &mut st, &p, &sh)));
                    if f < 2 {
                        apply_delta(&mut texs, last.as_ref().unwrap());
                    }
                }
                render(&ctx, last.unwrap(), &mut texs, &format!("{}/scheme{}_{}.rgb", dir, k, n));
            }
        }
        SCHEME.store(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Text must sit dead centre in every pill (measured on the rendered pixels, both themes,
    /// on and off). Guards awk_ui::txt's optical centring against font or egui changes.
    #[test]
    fn pill_text_is_centred() {
        let labels = ["STEREO", "L / R", "M / S", "MID", "SIDE", "SOFT", "HARD", "Δ DELTA", "SAFE 0 dB", "DARK", "LIGHT"];
        for light in [false, true] {
            LIGHT_ON.store(light, std::sync::atomic::Ordering::Relaxed);
            for on in [false, true] {
                let ctx = egui::Context::default();
                fonts(&ctx);
                let mut texs = HashMap::new();
                let mut rects = vec![];
                for i in 0..labels.len() {
                    let (col, row) = (i % 4, i / 4);
                    rects.push(Rect::from_min_size(pos2(40.0 + col as f32 * 200.0, 80.0 + row as f32 * 80.0), vec2(if labels[i].len() > 6 { 112.0 } else { 66.0 }, 24.0)));
                }
                let mut last = None;
                for f in 0..3 {
                    let out = ctx.run_ui(raw(f, vec![]), |c| {
                        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(pal().body)).show(c, |ui| {
                            for (i, r) in rects.iter().enumerate() {
                                let _ = pill(ui, *r, labels[i], on, sc().mint, ("t", i));
                            }
                        });
                    });
                    if f < 2 {
                        apply_delta(&mut texs, &out);
                    }
                    last = Some(out);
                }
                let img = raster(&ctx, last.unwrap(), &mut texs);
                let w = WIDTH as usize;
                for (i, r) in rects.iter().enumerate() {
                    let bg = img[(r.center().y as usize) * w + (r.left() + r.height() * 0.5 + 1.0) as usize];
                    let inner = r.shrink2(vec2(r.height() * 0.5, 3.0));
                    let (mut x0, mut x1, mut y0, mut y1) = (usize::MAX, 0, usize::MAX, 0);
                    for y in inner.top() as usize..inner.bottom() as usize {
                        for x in inner.left() as usize..inner.right() as usize {
                            let q = img[y * w + x];
                            if (q[0] - bg[0]).abs() + (q[1] - bg[1]).abs() + (q[2] - bg[2]).abs() > 0.45 {
                                x0 = x0.min(x);
                                x1 = x1.max(x);
                                y0 = y0.min(y);
                                y1 = y1.max(y);
                            }
                        }
                    }
                    assert!(x0 < x1, "{}: no text found", labels[i]);
                    let (cx, cy) = ((x0 + x1 + 1) as f32 * 0.5, (y0 + y1 + 1) as f32 * 0.5);
                    let (dx, dy) = (cx - r.center().x, cy - r.center().y);
                    println!("{:10} light {light} on {on}: dx {dx:+.1} dy {dy:+.1}", labels[i]);
                    assert!(dx.abs() <= 1.0 && dy.abs() <= 1.0, "{} is off centre by ({dx:+.1}, {dy:+.1}) px (light {light}, on {on})", labels[i]);
                }
            }
        }
        LIGHT_ON.store(false, std::sync::atomic::Ordering::Relaxed);
    }
}
