//! `verify` — physical verification through GPurify (`gdsverify`).
//!
//! [`signoff_checked`] runs DRC/ERC/LVS/PEX in one engine pass and returns a
//! [`Signoff`]: errors as a [`pnr_core::Report`], deck warnings and what was
//! not checked ([`Coverage`]) apart from it; [`drc`]/[`erc`] are standalone probes. [`Pdk`] (the process schema every crate reads) lives here too.

pub mod checker;
mod decks;
pub mod geom;
pub mod netlist;
pub mod pdk;
pub mod reference;
pub mod sidecar;

use std::time::{Duration, Instant};

use gdsverify::check::report::{Measurement, Severity};
use gdsverify::engine::{StageStatus, Summary};
use pnr_core::{Report, Shape, Violation};

pub use checker::Checker;
pub use gdsverify::ingest::deck::DeviceKind;
pub use gdsverify::engine::Checks;
pub use geom::LabeledPin;
pub use netlist::{extract_spice, Detail};
pub use pdk::{EmLimit, FetLimit, Pdk};
pub use reference::{RefDeviceIn, RefInput, RefKind};

/// Shortfall of one violation row, in one of two units. A length pair is nm
/// (1 dbu = 1 nm): `|limit − measured|` — every row is a violation, so that
/// is a minimum's shortfall and a maximum's overshoot alike (a row does not
/// carry its rule's `LimitSense`; LU.2's 20 µm tap distance against 15 µm
/// reads 5000). Any other
/// same-dimension numeric pair (area, ratio, count, voltage, current,
/// resistance) is ‰ of the limit, `ceil(1000·|measured − limit| / |limit|)`
/// (`|limit|` floored at 1e-12), so an antenna ratio 2× over reads 1000 and
/// 1 % over reads 10; a NaN reads `i64::MAX`. Mismatched dimensions read 1.
#[must_use]
pub fn shortfall(limit: Measurement, measured: Measurement) -> i64 {
    use Measurement as M;
    // A NaN reading is no graze: it ranks as the worst miss (`as` would make it 0).
    let permille = |l: f64, m: f64| {
        let x = (1000.0 * (m - l).abs() / l.abs().max(1e-12)).ceil();
        if x.is_nan() { i64::MAX } else { x as i64 }
    };
    match (limit, measured) {
        (M::Length(l), M::Length(m)) => (l.raw() - m.raw()).abs(),
        (M::Area(l), M::Area(m)) => permille(l.raw() as f64, m.raw() as f64),
        (M::Ratio(l), M::Ratio(m)) => permille(l, m),
        (M::Count(l), M::Count(m)) => permille(f64::from(l), f64::from(m)),
        (M::Voltage(l), M::Voltage(m)) => permille(l.raw(), m.raw()),
        (M::Current(l), M::Current(m)) => permille(l.raw(), m.raw()),
        (M::Resistance(l), M::Resistance(m)) => permille(l.raw(), m.raw()),
        _ => 1,
    }
}

/// Everything one signoff concluded: errors, warnings and what was not
/// checked, kept apart.
#[derive(Default)]
pub struct Signoff {
    /// Errors only: DRC/ERC/LVS rows of `Severity::Error`, `engine/…`, the
    /// LVS `lvs/extract: label short`, one `lvs-coverage/…` per
    /// [`Coverage::unverified`] entry. Its `cost` is PEX's total C, fF.
    pub report: Report,
    /// Rows the deck states as warnings. Never in `report.hard_violations`.
    pub warnings: Vec<Violation>,
    /// What this run did not check: LVS-unverified devices, skipped rules.
    pub coverage: Coverage,
    /// PEX C over labelled nets, fF ([`Checker::cap_matrix`]); empty without
    /// PEX, still filled on the label-short fallback.
    pub caps: CapMatrix,
    /// Wall time.
    pub elapsed: Duration,
}

/// What a signoff did not check. Never a pass: a certificate requires
/// `unverified` empty, and every skipped rule is listed with its reason.
#[derive(Clone, Debug, Default)]
pub struct Coverage {
    /// Schematic devices LVS did not compare: `(kind, model hint, count)`.
    pub unverified: Vec<(RefKind, Option<String>, usize)>,
    /// Rules this run did not execute, `(rule, reason)`: deck skips, waivers, chip-level deferrals.
    pub skipped_rules: Vec<(String, String)>,
    /// `(ran, in deck)` over the EM (`EM*`) and `ir_drop` rules; ran =
    /// `Outcome::Ran` with `examined > 0` (a rule that ran over no node
    /// checked nothing).
    pub em_ir: (u32, u32),
    /// The same `(ran, in deck)` over `ir_drop` alone: `em_ir` cannot tell
    /// a grid drop check from the EM rules an operating point arms.
    pub ir: (u32, u32),
}

/// GPurify's layout-only range checks, recorded `Skipped(NotInDeck)` because
/// the deck carries no limits; the reference parameter comparison runs under
/// `lvs.parameter_mismatch` for MOS W/L only, so these skips do not mean LVS
/// ignored W/L. Other kinds' values are the [`NON_MOS_VALUES`] row.
const RANGE_ONLY: [&str; 3] = ["lvs.device_count_mos", "lvs.device_count_bjt", "lvs.parametric"];

impl std::fmt::Display for Coverage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (kind, model, n) in &self.unverified {
            writeln!(f, "  LVS unverified: {n} × {kind:?} {}", model.as_deref().unwrap_or("(no model)"))?;
        }
        if !self.skipped_rules.is_empty() {
            writeln!(f, "  rules not run ({}):", self.skipped_rules.len())?;
        }
        for (rule, why) in &self.skipped_rules {
            let note = if RANGE_ONLY.contains(&rule.as_str()) {
                " (range limits; MOS W/L are compared as lvs.parameter_mismatch, other values are not)"
            } else {
                ""
            };
            writeln!(f, "    {rule}: {why}{note}")?;
        }
        Ok(())
    }
}

/// Full signoff over drawn geometry, its pin labels and its schematic
/// reference: [`signoff_checked`] without intent or warnings; the
/// [`Duration`] is wall time. Coverage is kept only as the report's
/// `lvs-coverage/…` hard rows.
#[must_use]
pub fn signoff(
    shapes: &[Shape],
    pins: &[LabeledPin],
    reference: &RefInput,
    pdk: &Pdk,
) -> (Report, Duration) {
    let (report, t, _) = signoff_with_caps(shapes, pins, reference, pdk);
    (report, t)
}

/// Design intent signoff hands the deck's intent-gated rules (EM, IR drop):
/// supply nets and the current each carries. Default = none (those rules skip).
#[derive(Clone, Debug, Default)]
pub struct Intent {
    /// `(net name, nominal voltage mV, is ground)`.
    pub supplies: Vec<(String, f64, bool)>,
    /// `(net name, DC current µA)` from the operating point.
    pub currents: Vec<(String, f64)>,
    /// `(net name, allowed DC drop mV)`, from the op headroom
    /// (`annotator::ir`): what arms `ir_drop`.
    pub max_drop_mv: Vec<(String, f64)>,
}

/// Extracted capacitance between labelled nets, fF: `(net, None, C)` to
/// ground, `(a, Some(b), C)` coupling (see [`Checker::cap_matrix`]).
pub type CapMatrix = Vec<(String, Option<String>, f64)>;

/// [`signoff`], plus the extracted [`CapMatrix`] (empty when PEX did not run).
#[must_use]
pub fn signoff_with_caps(
    shapes: &[Shape],
    pins: &[LabeledPin],
    reference: &RefInput,
    pdk: &Pdk,
) -> (Report, Duration, CapMatrix) {
    signoff_with_intent(shapes, pins, reference, &Intent::default(), pdk)
}

/// [`signoff_checked`]'s errors, wall time and caps; warnings are dropped,
/// coverage is kept as the report's `lvs-coverage/…` hard rows.
#[must_use]
pub fn signoff_with_intent(
    shapes: &[Shape],
    pins: &[LabeledPin],
    reference: &RefInput,
    intent: &Intent,
    pdk: &Pdk,
) -> (Report, Duration, CapMatrix) {
    let s = signoff_checked(shapes, pins, reference, intent, pdk);
    (s.report, s.elapsed, s.caps)
}

/// Full signoff with design [`Intent`] (so the deck's EM/IR rules run on the
/// operating-point currents instead of skipping).
///
/// A deck error row becomes a hard [`Violation`] named
/// `{domain}/{rule}:{layer}` with its [`shortfall`] as margin; a deck warning
/// row, named the same way, goes to [`Signoff::warnings`] only. A stage the
/// engine skipped or refused, or an engine failure, is a hard `engine/…`
/// violation: a check that could not run never reads as clean. Rules skipped
/// inside a stage that ran, and schematic devices no deck recogniser
/// extracts, are this run's [`Coverage`]; each `(kind, model, n)` of the
/// latter is also a hard `lvs-coverage/unverified:{kind}:{model}` row of
/// margin `n`.
#[must_use]
pub fn signoff_checked(
    shapes: &[Shape],
    pins: &[LabeledPin],
    reference: &RefInput,
    intent: &Intent,
    pdk: &Pdk,
) -> Signoff {
    let t0 = Instant::now();
    let mut s = Signoff::default();
    let fail = |s: &mut Signoff, rule: String| {
        s.report.hard_violations.push(Violation { rule, margin: 0 });
    };

    let mut checker = match Checker::new(pdk, false) {
        Ok(c) => c,
        Err(e) => {
            fail(&mut s, format!("engine/load: {e}"));
            s.elapsed = t0.elapsed();
            return s;
        }
    };
    defer_chip_level(&mut checker, shapes);
    if let Err(e) = checker.set_intent(intent) {
        fail(&mut s, format!("engine/intent: {e}"));
    }
    match checker.set_reference(reference) {
        Err(e) => fail(&mut s, format!("engine/reference: {e}")),
        Ok(skipped) => {
            for (kind, model) in skipped {
                match s.coverage.unverified.iter_mut().find(|u| u.0 == kind && u.1 == model) {
                    Some(u) => u.2 += 1,
                    None => s.coverage.unverified.push((kind, model, 1)),
                }
            }
            // Unknown is never pass: an uncompared device is a hard row (the
            // same count every epoch, so ranking is unchanged) that keeps the
            // run from reading LVS-clean.
            for (kind, model, n) in &s.coverage.unverified {
                let rule = format!("lvs-coverage/unverified:{kind:?}:{}", model.as_deref().unwrap_or("-"));
                s.report.hard_violations.push(Violation { rule, margin: *n as i64 });
            }
            match checker.run(shapes, pins, Checks::ALL) {
                // A label short aborts extraction and is LVS's verdict. Report
                // it, then re-run DRC/ERC/PEX with one label per shorted net,
                // so ERC still sees the ports and PEX still names the nets.
                Err(e) if e.starts_with(checker::LABEL_SHORT) => {
                    fail(&mut s, format!("lvs/{e}"));
                    let gone: Vec<String> = checker.shorted_labels().into_iter().flat_map(|g| g.into_iter().skip(1)).collect();
                    let kept: Vec<LabeledPin> = pins.iter().filter(|p| !gone.contains(&p.name)).cloned().collect();
                    match checker.run(shapes, &kept, Checks { lvs: false, ..Checks::ALL }) {
                        Err(e) => fail(&mut s, format!("engine/run: {e}")),
                        Ok(summary) => {
                            harvest(&checker, &summary, &mut s);
                            s.caps = checker.cap_matrix();
                        }
                    }
                }
                Err(e) => fail(&mut s, format!("engine/run: {e}")),
                Ok(summary) => {
                    harvest(&checker, &summary, &mut s);
                    s.caps = checker.cap_matrix();
                }
            }
            // Only MOS cards carry params: a compared R/D/C/BJT matches by
            // connectivity alone, its value (rc_filter's R) unchecked. Listed,
            // so a MATCH never reads as value-checked.
            let valueless = |k: RefKind| !matches!(k, RefKind::Nmos | RefKind::Pmos);
            let cards = reference.devices.iter().filter(|d| valueless(d.kind)).count();
            let uncompared: usize = s.coverage.unverified.iter().filter(|u| valueless(u.0)).map(|u| u.2).sum();
            if let n @ 1.. = cards.saturating_sub(uncompared) {
                s.coverage.skipped_rules.push((NON_MOS_VALUES.to_string(), format!("NotCompared({n} R/D/C/BJT cards matched by connectivity only)")));
            }
        }
    }
    s.elapsed = t0.elapsed();
    s
}

/// The [`Coverage::skipped_rules`] row for compared devices whose value LVS
/// does not compare (every kind but MOS).
pub const NON_MOS_VALUES: &str = "lvs.parameter_mismatch(non-MOS values)";

/// A density window wider than the block (the shapes' union bbox) is chip
/// integration's check: [`Checker::defer_density_wider_than`].
fn defer_chip_level(checker: &mut Checker, shapes: &[Shape]) {
    let (x0, y0, x1, y1) = shapes.iter().fold((i64::MAX, i64::MAX, i64::MIN, i64::MIN), |(a, b, c, d), s| {
        let r = s.rect;
        (a.min(i64::from(r.x)), b.min(i64::from(r.y)), c.max(i64::from(r.x + r.w)), d.max(i64::from(r.y + r.h)))
    });
    if x1 > x0 && y1 > y0 {
        checker.defer_density_wider_than(x1 - x0, y1 - y0);
    }
}

/// Errors to the hard tier, everything else (GPurify's `Severity` is
/// `Warning` | `Error`) to the warnings: the pure split [`harvest`] applies.
fn split_by_severity(rows: impl Iterator<Item = (Violation, Severity)>) -> (Vec<Violation>, Vec<Violation>) {
    let (mut hard, mut warn) = (Vec::new(), Vec::new());
    for (v, sev) in rows {
        if sev == Severity::Error { hard.push(v) } else { warn.push(v) }
    }
    (hard, warn)
}

fn harvest(checker: &Checker, summary: &Summary, s: &mut Signoff) {
    let out = checker.outputs();
    let rows = (0..out.violations.len()).map(|i| {
        let v = out.violations.get(i);
        let rule = format!("{}/{}:{}", checker.domain_of(v.rule), checker.rule_name(v.rule), checker.layer_name(v.layer));
        (Violation { rule, margin: shortfall(v.limit, v.measured) }, v.severity)
    });
    let (hard, warn) = split_by_severity(rows);
    s.report.hard_violations.extend(hard);
    s.warnings.extend(warn);
    for (stage, status) in [
        ("drc", &summary.drc),
        ("erc", &summary.erc),
        ("lvs", &summary.lvs),
        ("pex", &summary.pex),
    ] {
        if let Some(why) = denied(status) {
            s.report
                .hard_violations
                .push(Violation { rule: format!("engine/{stage}: {why}"), margin: 0 });
        }
    }
    s.coverage.skipped_rules = checker.skipped_rules().into_iter().map(|(r, why)| (r.to_string(), why)).collect();
    for r in &out.runs {
        let name = checker.rule_name(r.rule);
        let ran = u32::from(r.outcome == gdsverify::check::report::Outcome::Ran && r.examined > 0);
        if name.starts_with("EM") || name == "ir_drop" {
            s.coverage.em_ir.1 += 1;
            s.coverage.em_ir.0 += ran;
        }
        if name == "ir_drop" {
            s.coverage.ir.1 += 1;
            s.coverage.ir.0 += ran;
        }
    }
    s.report.cost = checker.total_cap_ff();
}

/// `Some(reason)` for a stage that was requested but did not run.
fn denied(status: &StageStatus) -> Option<String> {
    match status {
        StageStatus::Ran | StageStatus::NotSelected => None,
        StageStatus::Skipped(why) => Some(format!("skipped: {why}")),
        StageStatus::Refused(why) => Some(format!("refused: {why}")),
    }
}

/// One located finding, for callers that want rows rather than a Report.
#[derive(Clone, Debug)]
pub struct Finding {
    /// The deck's rule id, or `engine/…` for a run that could not conclude.
    pub rule: String,
    /// Layer name, `"-"` for findings with no layer.
    pub layer: String,
    /// [`shortfall`], in [`Finding::unit`].
    pub margin: i64,
    /// `"nm"` for a length rule, `"permille"` (of the limit) for any other —
    /// including the placeholder 1 of mismatched dimensions and `engine/…`.
    pub unit: &'static str,
    pub x: i64,
    pub y: i64,
    /// A row the deck states as a warning: [`Signoff::warnings`] in a full
    /// signoff, so never a DRC/ERC violation count.
    pub warning: bool,
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
        |rule: String| vec![Finding { rule, layer: "-".into(), margin: 1, unit: "permille", x: 0, y: 0, warning: false }];
    let mut checker = match Checker::new(pdk, false) {
        Ok(c) => c,
        Err(e) => return engine_fail(format!("engine/load: {e}")),
    };
    defer_chip_level(&mut checker, shapes);
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
                margin: shortfall(v.limit, v.measured),
                unit: match (v.limit, v.measured) {
                    (Measurement::Length(_), Measurement::Length(_)) => "nm",
                    _ => "permille",
                },
                x: v.at.x.raw(),
                y: v.at.y.raw(),
                warning: v.severity != Severity::Error,
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

#[cfg(test)]
mod tests {
    use super::*;
    use gdsverify::geom::Dbu;
    use pnr_core::{Process, Rect};

    fn sky130() -> Pdk {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/sky130.json");
        Pdk::from_json(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn rect(pdk: &Pdk, layer: &str, x: i32, y: i32, w: i32, h: i32) -> Shape {
        let layer = pdk.layer(layer).unwrap_or_else(|| panic!("no layer {layer}"));
        Shape { layer, rect: Rect { x, y, w, h } }
    }

    // Metal reaching no device is floating — unless a label makes it a port,
    // which leaves the cell (the bulk-only VSS rail case).
    #[test]
    fn unconnected_metal_is_floating_unless_it_is_a_port() {
        let pdk = sky130();
        let shapes = [rect(&pdk, "met1", 0, 0, 1000, 1000)];
        let floating = |pins: &[LabeledPin]| {
            // sky130 x.22: unconnected conductor.
            erc(&shapes, pins, &pdk).iter().filter(|f| f.rule == "x.22").count()
        };
        assert_eq!(floating(&[]), 1);
        let met1 = pdk.layer("met1").unwrap().0;
        let pin = LabeledPin { name: "VSS".into(), layer: met1, x: 500, y: 500 };
        assert_eq!(floating(&[pin]), 0);
    }

    // AV-04: a label names a net for LVS, it does not make it a port. A gate
    // labelled `g` that only the block's port list can drive is floating
    // unless `g` is declared external; with no list the exemption is blanket
    // and the coverage says so.
    #[test]
    fn an_internal_gate_only_net_is_floating_even_when_labelled() {
        let pdk = sky130();
        // `device_count_sees_one_mos_in_a_minimal_stack`'s MOS: the poly is
        // its gate and reaches nothing else.
        let shapes = [
            rect(&pdk, "poly", 200, 0, 100, 400),
            rect(&pdk, "diff", 0, 100, 260, 200),
            rect(&pdk, "diff", 240, 100, 260, 200),
            rect(&pdk, "nsdm", 0, 0, 500, 400),
        ];
        let pins = [LabeledPin { name: "g".into(), layer: pdk.layer("poly").unwrap().0, x: 250, y: 350 }];
        let floating = |external_ports: Option<Vec<String>>| {
            let reference = RefInput { devices: vec![], ports: vec!["g".into()], external_ports };
            let mut checker = Checker::new(&pdk, true).unwrap();
            checker.set_reference(&reference).unwrap();
            checker.run(&shapes, &pins, Checks { drc: false, erc: true, lvs: false, pex: false }).unwrap();
            let out = checker.outputs();
            let rows = (0..out.violations.len()).filter(|&i| checker.rule_name(out.violations.get(i).rule) == "floating_gate").count();
            let blanket = checker.skipped_rules().iter().any(|(r, why)| *r == "floating_gate" && why.contains("no port list"));
            (rows, blanket)
        };
        assert_eq!(floating(Some(vec![])), (1, false), "an internal gate-only net is floating");
        assert_eq!(floating(Some(vec!["g".into()])), (0, false), "a declared port is driven from outside");
        assert_eq!(floating(None), (0, true), "no port list: exempt, and reported as such");
    }

    // The one margin arithmetic every consumer shares: nm for a length pair,
    // floored at zero; 1 for a pair of different dimensions.
    #[test]
    fn shortfall_is_nm_for_lengths() {
        let len = |nm: i64| Measurement::Length(Dbu::new_unchecked(nm));
        assert_eq!(shortfall(len(170), len(100)), 70);
        assert_eq!(shortfall(len(100), len(170)), 70, "a maximum's overshoot is no graze");
        assert_eq!(shortfall(len(170), Measurement::Ratio(0.5)), 1);
    }

    // AV-31: a non-length margin is ‰ of the limit, so an antenna ratio 2×
    // over outranks one 1 % over (both read 1 before).
    #[test]
    fn shortfall_reads_ratio_overshoot_in_permille() {
        use Measurement::{Count, Ratio};
        assert_eq!(shortfall(Ratio(400.0), Ratio(800.0)), 1000);
        assert_eq!(shortfall(Ratio(400.0), Ratio(404.0)), 10);
        assert_eq!(shortfall(Count(1), Count(0)), 1000);
        assert_eq!(shortfall(Ratio(400.0), Ratio(f64::NAN)), i64::MAX, "NaN is not a graze");
    }

    #[test]
    fn split_by_severity_keeps_warnings_out_of_the_hard_tier() {
        let row = |r: &str, sev| (Violation { rule: r.into(), margin: 1 }, sev);
        let rows = [row("drc/a", Severity::Error), row("erc/b", Severity::Warning), row("drc/c", Severity::Error)];
        let (hard, warn) = split_by_severity(rows.into_iter());
        assert_eq!(hard.iter().map(|v| v.rule.as_str()).collect::<Vec<_>>(), ["drc/a", "drc/c"]);
        assert_eq!(warn.iter().map(|v| v.rule.as_str()).collect::<Vec<_>>(), ["erc/b"]);
    }

    // AV-26: the skip set is returned by every run, not logged once per process.
    #[test]
    fn skipped_rules_are_reported_on_every_run() {
        let pdk = sky130();
        for w in [500, 900] {
            let s = signoff_checked(&[rect(&pdk, "met1", 0, 0, w, w)], &[], &RefInput::default(), &Intent::default(), &pdk);
            assert!(!s.coverage.skipped_rules.is_empty(), "{w} nm plate: no skipped rules reported");
            // sky130 ir_drop needs intent this run does not give.
            assert!(s.coverage.skipped_rules.iter().any(|(_, why)| why.contains("NoDesignIntent")), "{:?}", s.coverage.skipped_rules);
        }
    }

    // AV-25: a label short still extracts capacitance (on the label-free re-run).
    #[test]
    fn the_label_short_fallback_still_returns_caps() {
        let pdk = sky130();
        let met1 = pdk.layer("met1").unwrap().0;
        let pins = [
            LabeledPin { name: "A".into(), layer: met1, x: 500, y: 500 },
            LabeledPin { name: "B".into(), layer: met1, x: 1500, y: 500 },
        ];
        let reference = RefInput { devices: vec![], ports: vec!["A".into(), "B".into()], external_ports: None };
        let s = signoff_checked(&[rect(&pdk, "met1", 0, 0, 2000, 1000)], &pins, &reference, &Intent::default(), &pdk);
        let rules: Vec<&str> = s.report.hard_violations.iter().map(|v| v.rule.as_str()).collect();
        assert!(rules.iter().any(|r| r.starts_with("lvs/extract: label short")), "{rules:?}");
        assert!(!s.caps.is_empty(), "the fallback returned no cap matrix");
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
            .find(|v| checker.rule_name(v.rule) == "li.1")
            .expect("the finding is attributed to li.1 (li width)");
        assert_eq!(shortfall(hit.limit, hit.measured), 70);
        assert_eq!(checker.domain_of(hit.rule), "drc");
    }

    // The deck's EM rules skip for want of design intent; with the supplies
    // and their currents installed they no longer do (ihp: EM.Metal*).
    #[test]
    fn intent_arms_the_em_rules() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/ihp_sg13g2.json");
        let pdk = Pdk::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
        let shapes = [rect(&pdk, "met1", 0, 0, 20_000, 1_000), rect(&pdk, "met1", 0, 5_000, 20_000, 1_000)];
        let met1 = pdk.layer("met1").unwrap().0;
        let pins = [
            LabeledPin { name: "VDD".into(), layer: met1, x: 500, y: 500 },
            LabeledPin { name: "VSS".into(), layer: met1, x: 500, y: 5_500 },
        ];
        let no_intent = |c: &Checker| c.skipped_rules().iter().filter(|(r, why)| r.starts_with("EM") && why.contains("NoDesignIntent")).count();
        let mut checker = Checker::new(&pdk, true).unwrap();
        checker.run(&shapes, &pins, Checks { drc: false, erc: true, lvs: false, pex: false }).unwrap();
        assert!(no_intent(&checker) > 0, "the EM rules need intent: {:?}", checker.skipped_rules());
        let intent = Intent {
            supplies: vec![("VDD".into(), 1_800.0, false), ("VSS".into(), 1_800.0, true)],
            currents: vec![("VDD".into(), 100.0), ("VSS".into(), 100.0)],
            ..Default::default()
        };
        checker.set_intent(&intent).unwrap();
        checker.run(&shapes, &pins, Checks { drc: false, erc: true, lvs: false, pex: false }).unwrap();
        assert_eq!(no_intent(&checker), 0, "{:?}", checker.skipped_rules());
    }

    // PERF-17: supplies alone leave `ir_drop` running over no node (no net
    // states a drop limit); a supply's `max_drop_mv` makes it examine the grid.
    #[test]
    fn max_drop_arms_ir_drop() {
        use gdsverify::check::report::Outcome;
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/sky130.json");
        let pdk = Pdk::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
        let shapes = [rect(&pdk, "met1", 0, 0, 20_000, 1_000)];
        let met1 = pdk.layer("met1").unwrap().0;
        let pins = [LabeledPin { name: "VDD".into(), layer: met1, x: 500, y: 500 }];
        let examined_ir = |c: &Checker| c.outputs().runs.iter().find(|r| c.rule_name(r.rule) == "ir_drop").map(|r| (r.outcome, r.examined));
        let checks = Checks { drc: false, erc: true, lvs: false, pex: false };
        let mut checker = Checker::new(&pdk, true).unwrap();
        checker.set_intent(&Intent { supplies: vec![("VDD".into(), 1_800.0, false)], ..Default::default() }).unwrap();
        checker.run(&shapes, &pins, checks).unwrap();
        assert_eq!(examined_ir(&checker).map(|r| r.1), Some(0), "supplies alone examine nothing");
        let intent = Intent { supplies: vec![("VDD".into(), 1_800.0, false)], max_drop_mv: vec![("VDD".into(), 10.0)], ..Default::default() };
        checker.set_intent(&intent).unwrap();
        checker.run(&shapes, &pins, checks).unwrap();
        let (outcome, examined) = examined_ir(&checker).expect("ir_drop in the deck");
        assert!(outcome == Outcome::Ran && examined > 0, "{outcome:?}, examined {examined}");
        assert!(!checker.skipped_rules().iter().any(|(r, _)| *r == "ir_drop"), "{:?}", checker.skipped_rules());
    }

    // GPurify refuses a net listed twice: a net's current budget and drop
    // limit go in one object.
    #[test]
    fn set_intent_merges_a_nets_limits() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/sky130.json");
        let pdk = Pdk::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
        let mut checker = Checker::new(&pdk, true).unwrap();
        let intent = Intent {
            supplies: vec![("VDD".into(), 1_800.0, false)],
            currents: vec![("VDD".into(), 100.0)],
            max_drop_mv: vec![("VDD".into(), 5.0)],
        };
        assert_eq!(checker.set_intent(&intent), Ok(()));
    }

    // A density window wider than the block is chip-level: taken out and
    // reported as not run; a block wider than the window keeps the rule
    // (ihp AFil.g2: activ density >= 25% in 800 um windows).
    #[test]
    fn density_wider_than_the_block_is_chip_level() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/ihp_sg13g2.json");
        let pdk = Pdk::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(pdk.density_rules().iter().any(|&(_, w, lim, max)| w == 800_000 && (lim - 0.25).abs() < 1e-9 && !max), "AFil.g2 read as a minimum");
        let mut small = Checker::new(&pdk, false).unwrap();
        let gone = small.defer_density_wider_than(34_000, 21_000);
        assert!(gone.iter().any(|n| n == "AFil.g2"), "{gone:?}");
        assert!(small.skipped_rules().iter().any(|(n, why)| *n == "AFil.g2" && why.starts_with("ChipLevel(window 800000 nm")));
        let mut chip = Checker::new(&pdk, false).unwrap();
        assert!(!chip.defer_density_wider_than(2_000_000, 2_000_000).contains(&"AFil.g2".to_string()));
    }

    // AV-18: sky130 states metal density as `density_cmp` (700 um windows,
    // `partial_windows: false`); on a block it examined zero windows and
    // read `Ran`. Now chip-level, and not among the rules that ran.
    #[test]
    fn density_cmp_wider_than_the_block_is_chip_level_on_sky130() {
        let pdk = sky130();
        let shapes = [rect(&pdk, "met1", 0, 0, 10_000, 10_000)];
        let names = ["m1.density", "m2.density", "m3.density", "m4.density"];
        let s = signoff_checked(&shapes, &[], &RefInput::default(), &Intent::default(), &pdk);
        for n in names {
            assert!(
                s.coverage.skipped_rules.iter().any(|(r, why)| r == n && why.starts_with("ChipLevel(window 700000 nm > block 10000x10000 nm)")),
                "{n} not chip-level: {:?}",
                s.coverage.skipped_rules
            );
        }
        let mut checker = Checker::new(&pdk, false).unwrap();
        defer_chip_level(&mut checker, &shapes);
        checker.run(&shapes, &[], Checks::ALL).unwrap();
        let ran: Vec<&str> = checker.outputs().runs.iter().filter(|r| r.outcome == gdsverify::check::report::Outcome::Ran).map(|r| checker.rule_name(r.rule)).collect();
        assert!(!ran.is_empty(), "no rule ran at all");
        assert!(names.iter().all(|n| !ran.contains(n)), "{ran:?}");
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
                    terminals: vec!["out".into(), "in".into(), "vss".into(), "vss".into()],
                    params: vec![("w".into(), 2e-6), ("l".into(), 5e-7)],
                },
                RefDeviceIn {
                    kind: RefKind::Nmos,
                    model: None,
                    terminals: vec!["vss".into(), "bias".into(), "out".into(), "vss".into()],
                    params: vec![("w".into(), 1e-6)],
                },
            ],
            ports: vec!["in".into(), "out".into(), "vss".into()],
            external_ports: None,
        };
        let skipped = checker.set_reference(&input).unwrap();
        assert!(skipped.is_empty(), "{skipped:?}");
        let n = checker.loaded.reference.as_ref().unwrap();
        assert_eq!(n.subckt_count(), 1);
        assert_eq!(n.device_model.len(), 2);
        assert_eq!(n.device_terminal_start, vec![0, 4, 8], "sky130 MOS arity is 4: d g s b, bulk = psub for an nfet");
        assert_eq!(n.port_net.len(), 3);
        assert_eq!(n.device_param_start, vec![0, 2, 3], "one param run per device");
        let w = checker.loaded.strings.get("w").expect("'w' is interned by the builder");
        let l = checker.loaded.strings.get("l").expect("'l' is interned by the builder");
        assert_eq!(n.param, vec![(w, 2e-6), (l, 5e-7), (w, 1e-6)]);
        let model = checker.loaded.strings.resolve(n.device_model[0]);
        assert_eq!(model, "sky130_fd_pr__nfet_01v8");
        // Both bipolars have a row now; an inductor still has none: skipped, not mismatched.
        let one = |kind, n: usize| RefInput {
            devices: vec![RefDeviceIn {
                kind,
                model: None,
                terminals: ["a", "b", "c"][..n].iter().map(|&t| t.into()).collect(),
                params: vec![],
            }],
            ports: vec![],
            external_ports: None,
        };
        assert_eq!(checker.set_reference(&one(RefKind::Npn, 3)).unwrap(), []);
        assert_eq!(checker.set_reference(&one(RefKind::Pnp, 3)).unwrap(), []);
        assert_eq!(checker.set_reference(&one(RefKind::Inductor, 2)).unwrap(), [(RefKind::Inductor, None)]);
    }

    // AV-01/NOTES-02: a schematic device no recogniser extracts is named in
    // the coverage, not left on stderr (sky130 has no VPP capacitor recogniser).
    #[test]
    fn a_skipped_reference_device_is_listed_in_coverage() {
        let pdk = sky130();
        let dev = |kind, terminals: &[&str]| RefDeviceIn {
            kind,
            model: None,
            terminals: terminals.iter().map(|&t| t.into()).collect(),
            params: vec![],
        };
        let reference = RefInput {
            devices: vec![
                dev(RefKind::Nmos, &["out", "in", "vss", "vss"]),
                dev(RefKind::Nmos, &["vss", "bias", "out", "vss"]),
                RefDeviceIn {
                    model: Some("sky130_fd_pr__cap_vpp_02p4x04p6_m1m2_noshield".into()),
                    ..dev(RefKind::Capacitor, &["a", "b"])
                },
            ],
            ports: vec![],
            external_ports: None,
        };
        let shapes = [rect(&pdk, "li", 0, 0, 500, 500)];
        let s = signoff_checked(&shapes, &[], &reference, &Intent::default(), &pdk);
        assert!(
            matches!(s.coverage.unverified.as_slice(), [(RefKind::Capacitor, _, 1)]),
            "{:?}",
            s.coverage.unverified
        );
    }

    /// A compared resistor's value is not checked (only MOS cards carry
    /// params), and coverage says so; an uncompared MOM is not counted there.
    #[test]
    fn a_compared_resistor_value_is_listed_as_not_compared() {
        let pdk = sky130();
        let card = |kind, model: Option<&str>| RefDeviceIn { kind, model: model.map(Into::into), terminals: vec!["a".into(), "b".into()], params: vec![] };
        let reference = RefInput {
            devices: vec![card(RefKind::Resistor, None), card(RefKind::Capacitor, Some("cap_generic_m1m2"))],
            ports: vec![],
            external_ports: None,
        };
        let s = signoff_checked(&[], &[], &reference, &Intent::default(), &pdk);
        let row: Vec<_> = s.coverage.skipped_rules.iter().filter(|(r, _)| r == NON_MOS_VALUES).map(|(_, why)| why.as_str()).collect();
        assert_eq!(row, ["NotCompared(1 R/D/C/BJT cards matched by connectivity only)"], "{:?}", s.coverage.skipped_rules);
    }

    // AV-01: sky130's `capm` recognises a MIM; a MOM card (no marker, no
    // recogniser), named or model-less (an elaborated composition's card),
    // must be uncompared, not compared as `capm` and unpaired.
    #[test]
    fn a_mom_card_is_unverified_not_compared_as_mim() {
        let pdk = sky130();
        for (model, rule) in [
            (Some("cap_generic_m1m2"), "lvs-coverage/unverified:Capacitor:cap_generic_m1m2"),
            (None, "lvs-coverage/unverified:Capacitor:-"),
        ] {
            let reference = RefInput {
                devices: vec![RefDeviceIn {
                    kind: RefKind::Capacitor,
                    model: model.map(Into::into),
                    terminals: vec!["top".into(), "bot".into()],
                    params: vec![],
                }],
                ports: vec![],
                external_ports: None,
            };
            // No geometry: nothing to extract, so any `lvs/` row is the card.
            let s = signoff_checked(&[], &[], &reference, &Intent::default(), &pdk);
            assert_eq!(s.coverage.unverified, [(RefKind::Capacitor, model.map(Into::into), 1)]);
            let rules: Vec<&str> = s.report.hard_violations.iter().map(|v| v.rule.as_str()).collect();
            assert!(!rules.iter().any(|r| r.starts_with("lvs/")), "{rules:?}");
            let cov: Vec<_> = s.report.hard_violations.iter().filter(|v| v.rule.starts_with("lvs-coverage/")).map(|v| (v.rule.as_str(), v.margin)).collect();
            assert_eq!(cov, [(rule, 1)]);
        }
    }

    // No deck recognises an inductor: it is uncompared, and that is coverage,
    // not an engine failure.
    #[test]
    fn an_inductor_is_unverified() {
        let pdk = sky130();
        let reference = RefInput {
            devices: vec![RefDeviceIn { kind: RefKind::Inductor, model: None, terminals: vec!["a".into(), "b".into()], params: vec![] }],
            ports: vec![],
            external_ports: None,
        };
        let s = signoff_checked(&[rect(&pdk, "li", 0, 0, 500, 500)], &[], &reference, &Intent::default(), &pdk);
        assert!(matches!(s.coverage.unverified.as_slice(), [(RefKind::Inductor, _, 1)]), "{:?}", s.coverage.unverified);
        let rules: Vec<&str> = s.report.hard_violations.iter().map(|v| v.rule.as_str()).collect();
        assert!(!rules.iter().any(|r| r.starts_with("engine/")), "{rules:?}");
        assert!(rules.contains(&"lvs-coverage/unverified:Inductor:-"), "{rules:?}");
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
                terminals: vec!["d".into(), "g".into(), "s".into(), "b".into()],
                params: vec![("w".into(), w_m), ("l".into(), 1e-7)],
            }],
            ports: vec![],
            external_ports: None,
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

    /// The body is compared: an nfet reference whose bulk is the source net,
    /// which the layout does not tie, is an LVS mismatch.
    #[test]
    fn a_wrong_body_tie_is_an_lvs_mismatch() {
        let pdk = sky130();
        let shapes = [
            rect(&pdk, "poly", 200, 0, 100, 400),
            rect(&pdk, "diff", 0, 100, 260, 200),
            rect(&pdk, "diff", 240, 100, 260, 200),
            rect(&pdk, "nsdm", 0, 0, 500, 400),
        ];
        let lvs_rows = |t: [&str; 4]| -> Vec<String> {
            let reference = RefInput {
                devices: vec![RefDeviceIn {
                    kind: RefKind::Nmos,
                    model: None,
                    terminals: t.iter().map(|&n| n.into()).collect(),
                    params: vec![("w".into(), 2e-7), ("l".into(), 1e-7)],
                }],
                ports: vec![],
                external_ports: None,
            };
            let mut checker = Checker::new(&pdk, true).unwrap();
            checker.set_reference(&reference).unwrap();
            checker.run(&shapes, &[], Checks { drc: false, erc: false, lvs: true, pex: false }).unwrap();
            let out = checker.outputs();
            (0..out.violations.len())
                .map(|i| checker.rule_name(out.violations.get(i).rule).to_owned())
                .filter(|r| r.starts_with("lvs."))
                .collect()
        };
        assert_eq!(lvs_rows(["d", "g", "s", "b"]), Vec::<String>::new());
        let wrong = lvs_rows(["d", "g", "s", "s"]);
        assert!(!wrong.is_empty(), "a body on the source net must mismatch");
    }

    /// Each n-well is its own net: two pfets in two wells sharing one bulk
    /// net in the reference is an LVS mismatch.
    #[test]
    fn a_pmos_in_a_foreign_well_is_an_lvs_mismatch() {
        let pdk = sky130();
        let stack = |x: i32| {
            [
                rect(&pdk, "poly", x + 200, 0, 100, 400),
                rect(&pdk, "diff", x, 100, 260, 200),
                rect(&pdk, "diff", x + 240, 100, 260, 200),
                rect(&pdk, "psdm", x, 0, 500, 400),
                rect(&pdk, "nwell", x - 200, -200, 900, 800),
            ]
        };
        let shapes: Vec<_> = stack(0).into_iter().chain(stack(5900)).collect();
        assert_eq!(Checker::new(&pdk, true).unwrap().device_count(&shapes), Some(2));
        let lvs_rows = |b1: &str, b2: &str| -> Vec<String> {
            let card = |d: &str, g: &str, s: &str, b: &str| RefDeviceIn {
                kind: RefKind::Pmos,
                model: None,
                terminals: [d, g, s, b].iter().map(|&n| n.into()).collect(),
                params: vec![("w".into(), 2e-7), ("l".into(), 1e-7)],
            };
            let reference = RefInput {
                devices: vec![card("d1", "g1", "s1", b1), card("d2", "g2", "s2", b2)],
                ports: vec![],
                external_ports: None,
            };
            let mut checker = Checker::new(&pdk, true).unwrap();
            checker.set_reference(&reference).unwrap();
            checker.run(&shapes, &[], Checks { drc: false, erc: false, lvs: true, pex: false }).unwrap();
            let out = checker.outputs();
            (0..out.violations.len())
                .map(|i| checker.rule_name(out.violations.get(i).rule).to_owned())
                .filter(|r| r.starts_with("lvs."))
                .collect()
        };
        assert_eq!(lvs_rows("w1", "w2"), Vec::<String>::new());
        let shared = lvs_rows("w", "w");
        assert!(!shared.is_empty(), "two wells on one bulk net must mismatch");
    }

    // The diode is recognisable now: a `diom` marker over the junction with
    // two `li` pads under it binds anode and cathode, so the deck extracts
    // exactly one device — and the reference builder no longer skips diodes.
    #[test]
    fn device_count_sees_one_diode_under_a_diom_marker() {
        let pdk = sky130();
        // sky130's diode_pw2nd in miniature: n+ diffusion under the diode
        // marker (the cathode; the substrate is the anode), li pads inside.
        let shapes = [
            rect(&pdk, "diff", 0, 0, 500, 1000),
            rect(&pdk, "nsdm", -125, -125, 750, 1250),
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
            external_ports: None,
        };
        let mut fresh = Checker::new(&pdk, false).unwrap();
        assert_eq!(
            fresh.set_reference(&with_diode).unwrap(),
            [],
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


