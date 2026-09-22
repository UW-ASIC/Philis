//! `verify` — physical verification through GPurify (`gdsverify`).
//!
//! [`signoff`] runs DRC/ERC/LVS/PEX in one engine pass and folds the result
//! into a [`pnr_core::Report`]. [`LiveOracle`] is the in-loop
//! [`pnr_core::Oracle`] over the same engine; [`drc`]/[`erc`] are standalone
//! probes. [`Pdk`] (the process schema every crate reads) lives here too.

pub mod checker;
pub mod geom;
pub mod netlist;
pub mod pdk;
pub mod reference;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use analog::{Requirements, Rule};
use gdsverify::check::report::Measurement;
use gdsverify::engine::{StageStatus, Summary};
use pnr_core::{Layout, Report, Shape, Violation};

pub use checker::Checker;
pub use gdsverify::engine::Checks;
pub use geom::LabeledPin;
pub use netlist::{extract_spice, Detail};
pub use pdk::Pdk;
pub use reference::{RefDeviceIn, RefInput, RefKind};

/// nm shortfall of one violation row: `limit − measured` for a length pair
/// (1 dbu = 1 nm), floored at 0; `1` for any non-length (boolean) fail.
#[must_use]
pub fn shortfall_nm(limit: Measurement, measured: Measurement) -> i64 {
    match (limit, measured) {
        (Measurement::Length(l), Measurement::Length(m)) => (l.raw() - m.raw()).max(0),
        _ => 1,
    }
}

/// Full signoff over drawn geometry, its pin labels and its schematic
/// reference; the [`Duration`] is wall time.
///
/// Every violation row (error or warning) becomes a hard [`Violation`] named
/// `{domain}/{rule}:{layer}` with its nm shortfall as margin. A stage the
/// engine skipped or refused, or an engine failure, is a hard `engine/…`
/// violation: a check that could not run never reads as clean. Rules skipped
/// inside a stage that ran are logged by name.
#[must_use]
pub fn signoff(
    shapes: &[Shape],
    pins: &[LabeledPin],
    reference: &RefInput,
    pdk: &Pdk,
) -> (Report, Duration) {
    let t0 = Instant::now();
    let mut report = Report::default();
    let fail = |report: &mut Report, rule: String| {
        report.hard_violations.push(Violation { rule, margin: 0 });
    };

    let mut checker = match Checker::new(pdk, false) {
        Ok(c) => c,
        Err(e) => {
            fail(&mut report, format!("engine/load: {e}"));
            return (report, t0.elapsed());
        }
    };
    match checker.set_reference(reference) {
        Err(e) => fail(&mut report, format!("engine/reference: {e}")),
        Ok(skipped) => {
            if skipped > 0 {
                eprintln!(
                    "verify::signoff: {skipped} schematic device(s) have no deck recogniser; \
                     left out of LVS"
                );
            }
            match checker.run(shapes, pins, Checks::ALL) {
                // A label short aborts extraction. Report it, then re-run
                // label-free so DRC/ERC/PEX still count honestly.
                Err(e) if e.starts_with(checker::LABEL_SHORT) => {
                    fail(&mut report, format!("lvs/{e}"));
                    match checker.run(shapes, &[], Checks::ALL) {
                        Err(e) => fail(&mut report, format!("engine/run: {e}")),
                        Ok(summary) => harvest(&checker, &summary, &mut report),
                    }
                }
                Err(e) => fail(&mut report, format!("engine/run: {e}")),
                Ok(summary) => harvest(&checker, &summary, &mut report),
            }
        }
    }
    (report, t0.elapsed())
}

fn harvest(checker: &Checker, summary: &Summary, report: &mut Report) {
    let out = checker.outputs();
    for i in 0..out.violations.len() {
        let v = out.violations.get(i);
        report.hard_violations.push(Violation {
            rule: format!(
                "{}/{}:{}",
                checker.domain_of(v.rule),
                checker.rule_name(v.rule),
                checker.layer_name(v.layer)
            ),
            margin: shortfall_nm(v.limit, v.measured),
        });
    }
    for (stage, status) in [
        ("drc", &summary.drc),
        ("erc", &summary.erc),
        ("lvs", &summary.lvs),
        ("pex", &summary.pex),
    ] {
        if let Some(why) = denied(status) {
            report
                .hard_violations
                .push(Violation { rule: format!("engine/{stage}: {why}"), margin: 0 });
        }
    }
    let skipped = checker.skipped_rules();
    if !skipped.is_empty() {
        eprintln!("verify::signoff: rules not run: {skipped:?}");
    }
    report.cost = checker.total_cap_ff();
}

/// `Some(reason)` for a stage that was requested but did not run.
fn denied(status: &StageStatus) -> Option<String> {
    match status {
        StageStatus::Ran | StageStatus::NotSelected => None,
        StageStatus::Skipped(why) => Some(format!("skipped: {why}")),
        StageStatus::Refused(why) => Some(format!("refused: {why}")),
    }
}

/// The live [`pnr_core::Oracle`]: one reusable density-stripped [`Checker`].
/// Deterministic: the engine runs single-threaded and sorts its output.
pub struct LiveOracle {
    // ponytail: Mutex serializes; thread_local sessions if a parallel batch
    // lane appears.
    session: Mutex<Checker>,
}

impl LiveOracle {
    /// # Errors
    /// The deck failing to re-parse (cannot happen for a loaded `Pdk`).
    pub fn new(pdk: &Pdk) -> Result<Self, String> {
        Ok(Self { session: Mutex::new(Checker::new(pdk, true)?) })
    }

    const DRC_ONLY: Checks = Checks { drc: true, erc: false, lvs: false, pex: false };
    const PEX_ONLY: Checks = Checks { drc: false, erc: false, lvs: false, pex: true };
}

impl pnr_core::Oracle for LiveOracle {
    fn drc(&self, shapes: &[Shape]) -> pnr_core::DrcSummary {
        let mut session = self.session.lock().expect("a panicked oracle call");
        if session.run(shapes, &[], Self::DRC_ONLY).is_err() {
            // Fail closed: geometry the engine cannot load is not clean.
            return pnr_core::DrcSummary { violations: 1, shortfall_nm: 1 };
        }
        let v = &session.outputs().violations;
        pnr_core::DrcSummary {
            violations: v.len() as u32,
            shortfall_nm: (0..v.len()).map(|i| shortfall_nm(v.limit[i], v.measured[i])).sum(),
        }
    }

    fn pex_cap_ff(&self, shapes: &[Shape]) -> f32 {
        let mut session = self.session.lock().expect("a panicked oracle call");
        match session.run(shapes, &[], Self::PEX_ONLY) {
            Err(_) => 0.0,
            Ok(_) => session.total_cap_ff(),
        }
    }

    /// Ambiguous when extraction aborts (the implant-merge signal) or finds
    /// the wrong device count (two devices fused, or one split).
    fn merge_ambiguous(&self, shapes: &[Shape], expected_devices: usize) -> bool {
        let mut session = self.session.lock().expect("a panicked oracle call");
        session.device_count(shapes) != Some(expected_devices)
    }
}

/// One located finding, for callers that want rows rather than a Report.
#[derive(Clone, Debug)]
pub struct Finding {
    /// The deck's rule id, or `engine/…` for a run that could not conclude.
    pub rule: String,
    /// Layer name, `"-"` for findings with no layer.
    pub layer: String,
    /// nm shortfall, `1` for a boolean fail.
    pub margin_nm: i64,
    pub x: i64,
    pub y: i64,
}

/// Standalone DRC: a fresh full [`Checker`] per call; an engine failure is a
/// single fail-closed `engine/…` finding.
#[must_use]
pub fn drc(shapes: &[Shape], pins: &[LabeledPin], pdk: &Pdk) -> Vec<Finding> {
    standalone(shapes, pins, pdk, Checks { drc: true, erc: false, lvs: false, pex: false })
}

/// Standalone ERC, as [`drc`].
#[must_use]
pub fn erc(shapes: &[Shape], pins: &[LabeledPin], pdk: &Pdk) -> Vec<Finding> {
    standalone(shapes, pins, pdk, Checks { drc: false, erc: true, lvs: false, pex: false })
}

fn standalone(shapes: &[Shape], pins: &[LabeledPin], pdk: &Pdk, checks: Checks) -> Vec<Finding> {
    let engine_fail =
        |rule: String| vec![Finding { rule, layer: "-".into(), margin_nm: 1, x: 0, y: 0 }];
    let mut checker = match Checker::new(pdk, false) {
        Ok(c) => c,
        Err(e) => return engine_fail(format!("engine/load: {e}")),
    };
    let summary = match checker.run(shapes, pins, checks) {
        Ok(s) => s,
        Err(e) => return engine_fail(format!("engine/run: {e}")),
    };
    let out = checker.outputs();
    let mut findings: Vec<Finding> = (0..out.violations.len())
        .map(|i| {
            let v = out.violations.get(i);
            Finding {
                rule: checker.rule_name(v.rule).to_string(),
                layer: checker.layer_name(v.layer).to_string(),
                margin_nm: shortfall_nm(v.limit, v.measured),
                x: v.at.x.raw(),
                y: v.at.y.raw(),
            }
        })
        .collect();
    for (stage, status) in [("drc", &summary.drc), ("erc", &summary.erc)] {
        if let Some(why) = denied(status) {
            findings.extend(engine_fail(format!("engine/{stage}: {why}")));
        }
    }
    findings
}

/// A captured DRC total, frozen into a Hard rule until the next re-measure.
/// Never satisfied; costs its nm shortfall (clamped ≥ 1).
#[derive(Clone, Copy)]
struct DrcSpacing {
    margin: i32,
}

impl DrcSpacing {
    fn from_margin(m: i64) -> Self {
        Self { margin: m.clamp(1, i64::from(i32::MAX)) as i32 }
    }
}

impl Rule for DrcSpacing {
    type On = Layout;
    fn cost(self, _state: &Layout) -> f32 {
        self.margin as f32
    }
    fn satisfied(self, _state: &Layout) -> bool {
        false
    }
}

/// In-loop placement feedback: measure `shapes` through the oracle and freeze
/// any DRC shortfall into a Hard [`Requirements<Layout>`] batch. Also returns
/// the raw summary, whose `violations` count is the honest severity term (the
/// batch is one rule however many findings stand).
#[must_use]
pub fn drc_feedback(
    oracle: &LiveOracle,
    shapes: &[Shape],
) -> (Requirements<Layout>, pnr_core::DrcSummary) {
    use pnr_core::Oracle as _;
    let summary = oracle.drc(shapes);
    let mut req = Requirements::default();
    if summary.violations > 0 {
        req.hard.push(Box::new(vec![DrcSpacing::from_margin(summary.shortfall_nm)]));
    }
    (req, summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gdsverify::geom::Dbu;
    use pnr_core::{Layout, Process, Rect};

    fn empty_layout() -> Layout {
        Layout {
            x: vec![],
            y: vec![],
            hw: vec![],
            hh: vec![],
            axis: vec![],
            groups: vec![],
            orient: vec![],
            variant: vec![],
            branch: vec![],
            power_uw: vec![],
            temp_mc: vec![],
        }
    }

    fn sky130() -> Pdk {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/sky130.json");
        Pdk::from_json(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn rect(pdk: &Pdk, layer: &str, x: i32, y: i32, w: i32, h: i32) -> Shape {
        let layer = pdk.layer(layer).unwrap_or_else(|| panic!("no layer {layer}"));
        Shape { layer, rect: Rect { x, y, w, h } }
    }

    // A captured violation must read as unsatisfied (Hard) and cost its margin.
    #[test]
    fn drc_spacing_rule_is_hard_and_costs_margin() {
        let r = DrcSpacing::from_margin(140);
        let s = empty_layout();
        assert!(!r.satisfied(&s), "a real DRC finding is never satisfied");
        assert_eq!(r.cost(&s), 140.0);
        // A zero/negative margin still costs ≥ 1 and stays unsatisfied.
        let r0 = DrcSpacing::from_margin(0);
        assert!(!r0.satisfied(&s));
        assert_eq!(r0.cost(&s), 1.0);
    }

    // Metal reaching no device is floating — unless a label makes it a port,
    // which leaves the cell (the bulk-only VSS rail case).
    #[test]
    fn unconnected_metal_is_floating_unless_it_is_a_port() {
        let pdk = sky130();
        let shapes = [rect(&pdk, "met1", 0, 0, 1000, 1000)];
        let floating = |pins: &[LabeledPin]| {
            erc(&shapes, pins, &pdk).iter().filter(|f| f.rule == "floating_interconnect").count()
        };
        assert_eq!(floating(&[]), 1);
        let met1 = pdk.layer("met1").unwrap().0;
        let pin = LabeledPin { name: "VSS".into(), layer: met1, x: 500, y: 500 };
        assert_eq!(floating(&[pin]), 0);
    }

    // The one margin arithmetic every consumer shares: nm for a length pair,
    // floored at zero; a boolean 1 for anything else.
    #[test]
    fn shortfall_is_nm_for_lengths_and_boolean_otherwise() {
        let len = |nm: i64| Measurement::Length(Dbu::new_unchecked(nm));
        assert_eq!(shortfall_nm(len(170), len(100)), 70);
        assert_eq!(shortfall_nm(len(100), len(170)), 0, "a graze cannot go negative");
        assert_eq!(shortfall_nm(Measurement::Count(1), Measurement::Count(0)), 1);
        assert_eq!(shortfall_nm(len(170), Measurement::Ratio(0.5)), 1);
    }

    // A deliberately-too-narrow li wire trips exactly li's min_width with the
    // correct nm margin.
    #[test]
    fn checker_reports_a_min_width_shortfall_in_nm() {
        let pdk = sky130();
        // li min_width is 170 nm; 100 nm wide is 70 short. Long enough to
        // clear li_min_area (side 236 → 55696 nm²) and on the 5 nm grid.
        let shapes = [rect(&pdk, "li", 0, 0, 100, 1000)];
        let mut checker = Checker::new(&pdk, true).unwrap();
        let summary = checker
            .run(&shapes, &[], Checks { drc: true, erc: false, lvs: false, pex: false })
            .unwrap();
        assert!(summary.violations >= 1, "a 100 nm li wire must fail min_width");
        let out = checker.outputs();
        let hit = (0..out.violations.len())
            .map(|i| out.violations.get(i))
            .find(|v| checker.rule_name(v.rule) == "li_min_width")
            .expect("the finding is attributed to li_min_width");
        assert_eq!(shortfall_nm(hit.limit, hit.measured), 70);
        assert_eq!(checker.domain_of(hit.rule), "drc");
    }

    // A fat legal rect is clean — and provably *checked* clean: rules ran.
    // Guards the false-clean failure (an engine that silently checked nothing).
    #[test]
    fn checker_runs_clean_on_legal_geometry_and_proves_it_ran() {
        let pdk = sky130();
        let shapes = [rect(&pdk, "li", 0, 0, 500, 500)];
        let mut checker = Checker::new(&pdk, true).unwrap();
        let summary = checker
            .run(&shapes, &[], Checks { drc: true, erc: false, lvs: false, pex: false })
            .unwrap();
        assert_eq!(summary.violations, 0, "a 500×500 li plate is legal");
        assert!(summary.rules_clean > 0, "zero findings must come from rules that ran");
        assert_eq!(summary.drc, StageStatus::Ran);
    }

    // The reference builder: two NMOS on the sky130 deck → two device rows of
    // the deck's model, three card-order terminals each, the stated ports —
    // and real device params, interned into the checker's table in SI metres,
    // one CSR run per device (which is what arms the engine's layout-side
    // measured-param emission for those names).
    #[test]
    fn reference_builder_compiles_two_devices() {
        let pdk = sky130();
        let mut checker = Checker::new(&pdk, false).unwrap();
        let input = RefInput {
            devices: vec![
                RefDeviceIn {
                    kind: RefKind::Nmos,
                    model: None,
                    terminals: vec!["out".into(), "in".into(), "vss".into()],
                    params: vec![("w".into(), 2e-6), ("l".into(), 5e-7)],
                },
                RefDeviceIn {
                    kind: RefKind::Nmos,
                    model: None,
                    terminals: vec!["vss".into(), "bias".into(), "out".into()],
                    params: vec![("w".into(), 1e-6)],
                },
            ],
            ports: vec!["in".into(), "out".into(), "vss".into()],
        };
        let skipped = checker.set_reference(&input).unwrap();
        assert_eq!(skipped, 0);
        let n = checker.loaded.reference.as_ref().unwrap();
        assert_eq!(n.subckt_count(), 1);
        assert_eq!(n.device_name.len(), 2);
        assert_eq!(n.device_terminal_start, vec![0, 3, 6], "sky130 ngate arity is 3");
        assert_eq!(n.port_net.len(), 3);
        assert_eq!(n.device_param_start, vec![0, 2, 3], "one param run per device");
        let w = checker.loaded.strings.get("w").expect("'w' is interned by the builder");
        let l = checker.loaded.strings.get("l").expect("'l' is interned by the builder");
        assert_eq!(n.param, vec![(w, 2e-6), (l, 5e-7), (w, 1e-6)]);
        let model = checker.loaded.strings.resolve(n.device_model[0]);
        assert_eq!(model, "sky130_fd_pr__nfet_01v8");
        // A capacitor has no recogniser in this deck: skipped, not mismatched
        // (MOM comb recognition needs merged-region binding — see the module
        // doc in `reference.rs`).
        let with_cap = RefInput {
            devices: vec![RefDeviceIn {
                kind: RefKind::Capacitor,
                model: None,
                terminals: vec!["a".into(), "b".into()],
                params: vec![],
            }],
            ports: vec![],
        };
        assert_eq!(checker.set_reference(&with_cap).unwrap(), 1);
    }

    // Parametric LVS is live end to end: the extractor measures the channel
    // (W = 200 nm, L = 100 nm here), the reference declares w/l in metres,
    // and the comparison pairs them by name — so a wrong declared width is a
    // reported mismatch, and the right one is a clean Match. This is the
    // guard against the old vacuous shape (both sides params-empty).
    #[test]
    fn a_wrong_reference_width_is_a_parameter_mismatch_and_the_right_one_matches() {
        let pdk = sky130();
        // The minimal recognisable MOS from `device_count_sees_one_mos`:
        // channel = poly AND diff AND nsdm = x 200..300, y 100..300.
        let shapes = [
            rect(&pdk, "poly", 200, 0, 100, 400),
            rect(&pdk, "diff", 0, 100, 260, 200),
            rect(&pdk, "diff", 240, 100, 260, 200),
            rect(&pdk, "nsdm", 0, 0, 500, 400),
        ];
        let reference = |w_m: f64| RefInput {
            devices: vec![RefDeviceIn {
                kind: RefKind::Nmos,
                model: None,
                terminals: vec!["d".into(), "g".into(), "s".into()],
                params: vec![("w".into(), w_m), ("l".into(), 1e-7)],
            }],
            ports: vec![],
        };
        let lvs_rows = |w_m: f64| -> Vec<String> {
            let mut checker = Checker::new(&pdk, true).unwrap();
            checker.set_reference(&reference(w_m)).unwrap();
            checker
                .run(&shapes, &[], Checks { drc: false, erc: false, lvs: true, pex: false })
                .unwrap();
            let out = checker.outputs();
            (0..out.violations.len())
                .map(|i| checker.rule_name(out.violations.get(i).rule).to_owned())
                .filter(|r| r.starts_with("lvs."))
                .collect()
        };

        assert_eq!(
            lvs_rows(2e-7),
            Vec::<String>::new(),
            "the declared 200 nm width matches the measured channel"
        );
        let wrong = lvs_rows(3e-7);
        assert!(
            wrong.iter().any(|r| r == "lvs.parameter_mismatch"),
            "a 300 nm declared width against a 200 nm channel must be a \
             parameter mismatch, got {wrong:?}"
        );
    }

    // The diode is recognisable now: a `diom` marker over the junction with
    // two `li` pads under it binds anode and cathode, so the deck extracts
    // exactly one device — and the reference builder no longer skips diodes.
    #[test]
    fn device_count_sees_one_diode_under_a_diom_marker() {
        let pdk = sky130();
        // The generator's shape in miniature: a diff body, the marker over it,
        // and the two li pads (A low, K high) strictly inside.
        let shapes = [
            rect(&pdk, "diff", 0, 0, 500, 1000),
            rect(&pdk, "diom", 0, 0, 500, 1000),
            rect(&pdk, "li", 130, 130, 240, 240),
            rect(&pdk, "li", 130, 630, 240, 240),
        ];
        let mut checker = Checker::new(&pdk, true).unwrap();
        assert_eq!(checker.device_count(&shapes), Some(1));

        let with_diode = RefInput {
            devices: vec![RefDeviceIn {
                kind: RefKind::Diode,
                model: None,
                terminals: vec!["a".into(), "k".into()],
                params: vec![],
            }],
            ports: vec![],
        };
        let mut fresh = Checker::new(&pdk, false).unwrap();
        assert_eq!(
            fresh.set_reference(&with_diode).unwrap(),
            0,
            "a deck with a diom recogniser must not skip diode cards"
        );
    }

    // A minimal MOS-like stack: poly crossing two overlapping diff plates under
    // an nsdm implant derives one ngate marker → one recognised device. The
    // merge oracle's whole currency is this count.
    #[test]
    fn device_count_sees_one_mos_in_a_minimal_stack() {
        let pdk = sky130();
        let shapes = [
            rect(&pdk, "poly", 200, 0, 100, 400),
            rect(&pdk, "diff", 0, 100, 260, 200),
            rect(&pdk, "diff", 240, 100, 260, 200),
            rect(&pdk, "nsdm", 0, 0, 500, 400),
        ];
        let mut checker = Checker::new(&pdk, true).unwrap();
        assert_eq!(checker.device_count(&shapes), Some(1));
        // And no geometry at all is zero devices, not an abort.
        assert_eq!(checker.device_count(&[]), Some(0));
    }
}
