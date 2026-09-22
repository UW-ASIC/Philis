//! # Side-by-side example: user macro + auto place-and-route
//!
//! A 3-stage MOSFET op-amp. The user hands the pipeline a **pre-drawn macro** for
//! one part of the design — the big output PMOS `M8` — via the `macro_master`
//! registry, and the library auto-generates and places/routes everything else.
//! Downstream can't tell the injected macro from a generated one; the only
//! difference is that `M8`'s DAG block is flagged `injected`.
//!
//! Run: `cargo run -p three_stage_opamp -- <deck.json>`
//! (the deck is a gdsverify PDK JSON — the same one the `philis` CLI takes).

use library::{run, signoff, Config, Macros};
use pnr_core::{LayerId, Macro, NetId, Pin, Process, Rect, Shape};
use verify::Pdk;

/// A 3-stage op-amp, MOSFETs only: input diff pair + active load (stage 1), a
/// common-source gain stage (stage 2), a push-pull output (stage 3), tail/bias.
const OPAMP_3STAGE: &str = r"
.subckt opamp3 vin_p vin_n vout vdd vss vbias
* stage 1 — input differential pair + tail current source
M1 n1 vin_p tail vss nfet w=4u l=0.5u
M2 n2 vin_n tail vss nfet w=4u l=0.5u
M3 tail vbias vss vss nfet w=8u l=0.5u
* stage 1 — active load (PMOS current mirror)
M4 n1 n1 vdd vdd pfet w=6u l=0.5u
M5 n2 n1 vdd vdd pfet w=6u l=0.5u
* stage 2 — common-source gain
M6 n3 n2 vdd vdd pfet w=12u l=0.5u
M7 n3 vbias vss vss nfet w=6u l=0.5u
* stage 3 — output driver (M8 is user-provided below)
M8 vout n3 vdd vdd pfet w=40u l=0.5u
M9 vout vbias vss vss nfet w=20u l=0.5u
.ends
";

fn main() {
    let Some(deck_path) = std::env::args().nth(1) else {
        eprintln!("usage: three_stage_opamp <deck.json>");
        std::process::exit(2);
    };
    let deck = std::fs::read_to_string(&deck_path).expect("read deck json");
    let pdk = Pdk::from_json(&deck).expect("parse pdk deck");

    // ── The part of the design the *user* does themselves ──
    // Register a pre-drawn macro for the output PMOS `M8` under its instance name.
    // In a full flow you author this as a `macro_master::Generator` and realise it
    // against the PDK; here we hand in the `Macro` directly. The library uses it
    // verbatim and auto-generates M1–M7, M9.
    let mut macros = Macros::default();
    macros.register("M8", user_output_pmos(&pdk));

    let cfg = Config::default();
    let sol = run(OPAMP_3STAGE, &pdk, &macros, &cfg).expect("place-and-route");

    println!(
        "placed {} devices ({} routed nets); M8 was user-injected, the rest auto-drawn",
        sol.macros.len(),
        sol.routes.wires.len()
    );

    let report = signoff(&sol, &pdk);
    if report.hard_violations.is_empty() {
        println!("signoff CLEAN — cost {:.3}", report.cost);
    } else {
        println!("signoff — {} hard violation(s)", report.hard_violations.len());
    }

    // ── TEMP: attribute each DRC violation to owning shapes ──
    let n_dev = sol.layout.x.len();
    let mut tagged: Vec<(String, Shape)> = Vec::new();
    for (i, m) in sol.macros.iter().enumerate() {
        let (cx, cy) = if i < n_dev { (sol.layout.x[i], sol.layout.y[i]) } else { (0, 0) };
        let src = if i >= n_dev { format!("ring{i}") } else if i == 7 { "M8".into() } else { format!("dev{i}") };
        for s in &m.shapes {
            tagged.push((src.clone(), Shape { layer: s.layer, rect: Rect { x: s.rect.x + cx, y: s.rect.y + cy, w: s.rect.w, h: s.rect.h } }));
        }
    }
    for net in &sol.routes.wires { for s in net { tagged.push(("route".into(), *s)); } }
    let shapes: Vec<Shape> = tagged.iter().map(|(_, s)| *s).collect();
    // The specific shapes on the violating layer straddling (vx,vy): source + rect.
    let culprits = |vx: i32, vy: i32, role: &str| -> Vec<String> {
        let lid = pdk.layer(role);
        let win = 250;
        tagged.iter()
            .filter(|(_, s)| Some(s.layer) == lid
                && vx >= s.rect.x - win && vx <= s.rect.x + s.rect.w + win
                && vy >= s.rect.y - win && vy <= s.rect.y + s.rect.h + win)
            .map(|(src, s)| format!("{src}[{},{} {}x{}]", s.rect.x, s.rect.y, s.rect.w, s.rect.h))
            .collect()
    };
    let drc = verify::drc(&shapes, &[], &pdk);
    eprintln!("\n=== {} DRC, culprit shapes ===", drc.len());
    for v in &drc {
        eprintln!("{}:{} shortfall={}nm @({},{})\n     {:?}",
            v.rule, v.layer, v.margin_nm, v.x, v.y,
            culprits(v.x as i32, v.y as i32, &v.layer));
    }

    // Show the placed-and-routed layout (blocks until the window closes; a no-op
    // on a headless host — `Probe::open` reports "no display" and returns).
    let polys = library::visualizer::polys_from_shapes(&shapes);
    let mut probe = library::visualizer::Probe::open("three_stage_opamp");
    probe.send(&polys, Some("three_stage_opamp: placed + routed"));
    probe.wait();
}

/// A crude hand-drawn output PMOS: a diffusion bar, a poly gate, and G/S/D landing
/// pads. Stand-in for what a `macro_master::Generator` would emit; the point is
/// that the library consumes it as-is.
///
/// (Pin nets are placeholders — remapping an injected macro's ports onto the
/// instance's terminal nets is the next refinement of the injection path.)
fn user_output_pmos(pdk: &Pdk) -> Macro {
    let met1 = pdk.layer("met1").or_else(|| pdk.layer("li")).unwrap_or(LayerId(0));
    let poly = pdk.layer("poly").unwrap_or(LayerId(0));
    let diff = pdk.layer("diff").unwrap_or(LayerId(0));

    let shapes = vec![
        Shape { layer: diff, rect: Rect { x: 0, y: 0, w: 4000, h: 1000 } },
        Shape { layer: poly, rect: Rect { x: 1800, y: -200, w: 400, h: 1400 } },
        Shape { layer: met1, rect: Rect { x: 0, y: 0, w: 600, h: 600 } },
        Shape { layer: met1, rect: Rect { x: 3400, y: 0, w: 600, h: 600 } },
    ];
    let pins = vec![
        Pin { name: "G".into(), net: NetId(0), at: Rect { x: 1800, y: 0, w: 400, h: 400 }, layer: poly },
        Pin { name: "S".into(), net: NetId(0), at: Rect { x: 0, y: 0, w: 600, h: 600 }, layer: met1 },
        Pin { name: "D".into(), net: NetId(0), at: Rect { x: 3400, y: 0, w: 600, h: 600 }, layer: met1 },
    ];
    Macro { shapes, pins, bbox: Rect { x: -200, y: -200, w: 4400, h: 1400 } }
}
