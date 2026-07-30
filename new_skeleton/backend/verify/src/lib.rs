//! # `verify` — physical verification via GPurify (`gdsverify`), in-loop *and* at
//! signoff.
//!
//! One engine, two roles. As a final gate [`signoff`] runs full DRC/LVS/PEX/ERC,
//! times each, and merges them into one [`pnr_core::Report`] (hard violations
//! from DRC/LVS/ERC, `cost` from PEX). **In the loop** the same engine feeds the
//! optimiser: DRC/LVS/ERC violations return as **Hard** [`analog::Rule`]s and PEX
//! as a **Cost** rule, so verification *drives* the next iteration instead of only
//! judging the last. DRC/ERC use the GPU backend when the `gpu` feature is on
//! (see [`drc::inloop_backend`]).
//!
//! [`Pdk`] (Philis's process schema) lives here; the frontend reads it via
//! gdsverify's JSON deck reader ([`Pdk::from_json`]) and hands it to
//! cells/stages/verify.

#![allow(dead_code)]

pub mod drc;
pub mod erc;
pub mod geom;
pub mod lvs;
pub mod pdk;
pub mod pex;

use std::time::{Duration, Instant};

use analog::{Requirements, Rule};
use pnr_core::{Layout, Report, Routes, Shape, Violation};

pub use pdk::Pdk;

// ═══════════════════════════════════════════════════════════════════════
//  Signoff — run all four, time each, merge into one Report
// ═══════════════════════════════════════════════════════════════════════

/// Per-check wall time, recorded so signoff logs where the time went.
#[derive(Clone, Copy, Debug, Default)]
pub struct Timings {
    pub drc: Duration,
    pub lvs: Duration,
    pub pex: Duration,
    pub erc: Duration,
}

/// Is a signoff-suite check applicable given the evidence in `erc_config`?
///
/// Each of gdsverify's six families runs only when its required input is
/// supplied (an `Option` in [`erc::SignoffConfig`]); with the input absent the
/// family reports NotRun/Error purely because it *cannot run*, not because it
/// found a fault. That is a NotApplicable condition, so signoff must not let it
/// block. Returns `true` only when the input the family needs is present —
/// then a non-clean result is a genuine ran-and-failed and does block.
fn erc_check_applicable(check: erc::SignoffCheck, cfg: &erc::SignoffConfig) -> bool {
    use erc::SignoffCheck::*;
    match check {
        // Antenna needs antenna rules or a fabrication layer order to analyze.
        Antenna => cfg.antenna.is_some() || cfg.layer_order.is_some(),
        DensityCmp => cfg.density_cmp.is_some(),
        // IR-drop and EM both solve the supplied power grid; no grid, no analysis.
        IrDrop | Electromigration => cfg.power.is_some(),
        Reliability => cfg.reliability.is_some(),
        EsdLatchup => cfg.esd_latchup.is_some(),
    }
}

/// Full signoff over drawn geometry and its schematic reference.
///
/// Runs DRC + ERC + PEX (over `shapes`) and LVS (against `reference`), timing
/// each, and merges the outcome into one [`pnr_core::Report`]: every DRC/LVS/ERC
/// failure becomes a [`Violation`] in `hard_violations`; PEX total capacitance
/// (fF) becomes the report `cost`. `timings` returns the per-check wall time
/// alongside — the caller logs it.
///
/// `erc_config` supplies the six-family signoff evidence; pass
/// [`erc::SignoffConfig::default`] when only the connectivity/heuristic ERC
/// checks are wanted. A signoff family with no evidence supplied is
/// NotApplicable and does not block (see [`erc_check_applicable`]); one whose
/// evidence IS supplied blocks when it does not come back clean.
#[must_use]
pub fn signoff(
    shapes: &[Shape],
    reference: &lvs::RefNetlist,
    erc_config: &erc::SignoffConfig,
    pdk: &Pdk,
) -> (Report, Timings) {
    let mut t = Timings::default();

    let t0 = Instant::now();
    let drc = drc::run_drc(shapes, pdk);
    t.drc = t0.elapsed();

    let t0 = Instant::now();
    let lvs = lvs::run_lvs(shapes, pdk, reference);
    t.lvs = t0.elapsed();

    let t0 = Instant::now();
    let erc = erc::run_erc(shapes, pdk, erc_config);
    t.erc = t0.elapsed();

    let t0 = Instant::now();
    let pex = pex::run_pex(shapes, pdk);
    t.pex = t0.elapsed();

    let mut hard_violations = Vec::new();

    // DRC: each rule violation, margin = limit − measured (shortfall in nm).
    for v in &drc.violations {
        hard_violations.push(Violation {
            rule: format!("drc/{}:{}", v.kind, v.layer),
            margin: v.limit - v.measured,
        });
    }

    // LVS: a mismatch is a single boolean-fail hard violation.
    if !lvs.matched {
        hard_violations.push(Violation {
            rule: format!("lvs:{}", lvs.reason),
            margin: 0,
        });
    }

    // ERC: heuristic findings + the six-family signoff suite.
    //
    // A suite check reports NotRun/Error either because it ran and genuinely
    // failed, or because the *input it needs was never supplied* at this stage.
    // gdsverify gates each family on an `Option` in `erc_config`; when that
    // input is `None` the check is inapplicable — it has no evidence to analyze
    // and cannot find a real violation — so it must not block. Only a check
    // whose input IS present blocks when it is not clean (ran-and-failed, or
    // errored on inputs that exist). See `erc_check_applicable` for the mapping.
    for v in &erc.violations {
        // `erc_extraction_error` means connectivity extraction couldn't build a
        // netlist: there is no netlist to run ERC against, so this is a
        // stage-inapplicable condition, not a real electrical failure.
        if v.check == "erc_extraction_error" {
            continue;
        }
        hard_violations.push(Violation {
            rule: format!("erc/{}", v.check),
            margin: 0,
        });
    }
    for check in erc.signoff.checks() {
        if check.is_clean() {
            continue;
        }
        if !erc_check_applicable(check.check, erc_config) {
            // NotApplicable: no input supplied for this family at this stage,
            // so a NotRun/Error is inability-to-run, not a violation. Skipped.
            continue;
        }
        hard_violations.push(Violation {
            rule: format!("erc.signoff/{:?}:{:?}", check.check, check.status),
            margin: 0,
        });
    }
    // PEX diagnostics (geometry the extractor refused) block tapeout.
    if !pex.is_complete() {
        hard_violations.push(Violation {
            rule: "pex:incomplete".into(),
            margin: 0,
        });
    }

    let report = Report {
        hard_violations,
        // Signoff has no Θ tier. Every finding above is strict legality — DRC, LVS
        // structure, ERC, an incomplete extraction — and none of them is a *budget*
        // with a live residual to price. Budgets are declared by the circuit and
        // scored by the placement/routing stages against `Requirements`; signoff is
        // a gate on the finished artifact, not a step in the search.
        budget_violations: Vec::new(),
        cost: pex::total_cap_ff(&pex),
    };
    (report, t)
}

// ═══════════════════════════════════════════════════════════════════════
//  LiveOracle — the pnr_core::Oracle over gdsverify (PLAN §1, D4-revised)
// ═══════════════════════════════════════════════════════════════════════

/// The live [`pnr_core::Oracle`]: gdsverify measurements at stage-call
/// granularity. This is the free-oracle tier's real implementation — `dp`
/// consumes the trait, `frontend/library` constructs this over the run's PDK,
/// and `pnr_core::NullOracle` stands in wherever pre-oracle behaviour must be
/// reproduced byte-for-byte.
///
/// Determinism (the trait's contract): every path below is a pure function of
/// `shapes`. DRC goes through [`drc::run_drc_inloop`], whose GPU backend is
/// documented by gdsverify as verdict-identical to CPU ("the verdicts are
/// identical either way" — an exact-result spacing prefilter, not an
/// approximation), so gating on it is safe under either backend. PEX is the
/// analytical extractor (sheet-R, area/fringe/coupling C — closed-form, no
/// sampling). LVS extraction pins `Backend::Cpu` inside gdsverify itself.
pub struct LiveOracle<'a> {
    pub pdk: &'a Pdk,
}

impl pnr_core::Oracle for LiveOracle<'_> {
    fn drc(&self, shapes: &[Shape]) -> pnr_core::DrcSummary {
        let rep = drc::run_drc_inloop(shapes, self.pdk);
        pnr_core::DrcSummary {
            violations: rep.violations.len() as u32,
            // Same margin arithmetic as `signoff` and `drc_feedback`: nm
            // shortfall, floored at 0 per finding so a boolean-fail rule
            // (measured = -1) cannot subtract severity from a real one.
            shortfall_nm: rep.violations.iter().map(|v| (v.limit - v.measured).max(0)).sum(),
        }
    }

    fn pex_cap_ff(&self, shapes: &[Shape]) -> f32 {
        pex::total_cap_ff(&pex::run_pex(shapes, self.pdk))
    }

    fn merge_ambiguous(&self, shapes: &[Shape], expected_devices: usize) -> bool {
        // None-or-mismatch ⇒ ambiguous: an extraction abort is the implant-merge
        // signal itself, and a clean extraction of the wrong count means two
        // devices fused into one (or one split) without tripping any DRC rule.
        match lvs::extract_device_count(shapes, self.pdk) {
            Some(n) => n != expected_devices,
            None => true,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  In-loop feedback — violations as analog Rules
// ═══════════════════════════════════════════════════════════════════════
//
// A live DRC/LVS/ERC/PEX pass yields findings tied to *drawn geometry*, not to
// the device-centre `Layout` / route `Routes` the optimiser mutates. So these
// rules are **captured snapshots**: each carries the finding's severity and
// reports it as a fixed penalty against whatever state is scored, until the next
// iteration redraws geometry and re-runs verify. That is the standard "freeze
// the finding into a cost the optimiser pays" feedback pattern.

/// A captured DRC/ERC spacing/width/connectivity violation. Hard when applied as
/// legality (`satisfied() == false` while the finding stands), and its `margin`
/// (nm shortfall, or 1 for a boolean fail) is the cost it contributes.
#[derive(Clone, Copy)]
pub struct DrcSpacing {
    /// nm shortfall (`limit − measured`), clamped ≥ 1 so a real finding always
    /// costs something and always reads as unsatisfied.
    pub margin: i32,
}

impl DrcSpacing {
    #[must_use]
    fn from_margin(m: i64) -> Self {
        Self { margin: m.clamp(1, i32::MAX as i64) as i32 }
    }
}

impl Rule for DrcSpacing {
    type On = Layout;
    #[inline]
    fn cost(self, _state: &Layout) -> f32 {
        self.margin as f32
    }
    #[inline]
    fn satisfied(self, _state: &Layout) -> bool {
        false
    }
}

/// A captured routing-tier violation (antenna, routing-DRC, ERC on routed nets).
/// The [`Routes`] analogue of [`DrcSpacing`].
#[derive(Clone, Copy)]
pub struct AntennaViol {
    pub margin: i32,
}

impl AntennaViol {
    #[must_use]
    fn from_margin(m: i64) -> Self {
        Self { margin: m.clamp(1, i32::MAX as i64) as i32 }
    }
}

impl Rule for AntennaViol {
    type On = Routes;
    #[inline]
    fn cost(self, _state: &Routes) -> f32 {
        self.margin as f32
    }
    #[inline]
    fn satisfied(self, _state: &Routes) -> bool {
        false
    }
}

/// A captured PEX parasitic total — a pure **Cost** rule (never a legality
/// gate): the optimiser trades it off against everything else.
#[derive(Clone, Copy)]
pub struct PexCost {
    /// Total extracted capacitance in fF.
    pub cap_ff: f32,
}

impl Rule for PexCost {
    type On = Routes;
    #[inline]
    fn cost(self, _state: &Routes) -> f32 {
        self.cap_ff
    }
    // satisfied() defaults to true — Cost only, never rejects a candidate.
}

/// **In-loop placement feedback.** Run live DRC + ERC over the current drawn
/// geometry and turn every violation into a **Hard** [`DrcSpacing`] rule, so the
/// placement optimiser merges them into its stage [`Requirements<Layout>`] this
/// iteration. DRC uses the GPU backend when built with `gpu`.
///
/// `reference` (schematic) enables LVS feedback too; pass `None` to skip it
/// (LVS mid-placement is often not meaningful before routing exists).
#[must_use]
pub fn drc_feedback(
    shapes: &[Shape],
    pdk: &Pdk,
    reference: Option<&lvs::RefNetlist>,
) -> Requirements<Layout> {
    let mut rules: Vec<DrcSpacing> = Vec::new();

    let drc = drc::run_drc_inloop(shapes, pdk);
    for v in &drc.violations {
        rules.push(DrcSpacing::from_margin(v.limit - v.measured));
    }

    let erc = erc::run_erc_inloop(shapes, pdk);
    for _ in &erc.violations {
        rules.push(DrcSpacing::from_margin(1));
    }

    if let Some(reference) = reference {
        let lvs = lvs::run_lvs(shapes, pdk, reference);
        if !lvs.matched {
            rules.push(DrcSpacing::from_margin(1));
        }
    }

    let mut req = Requirements::default();
    if !rules.is_empty() {
        req.hard.push(Box::new(rules));
    }
    req
}

/// **In-loop routing feedback.** Run live DRC + ERC over the drawn routing and
/// turn violations into **Hard** [`AntennaViol`] rules; run PEX and add its
/// capacitance total as a **Cost** [`PexCost`] rule. The routing optimiser
/// merges the result into its stage [`Requirements<Routes>`] each iteration.
#[must_use]
pub fn route_feedback(routes: &Routes, pdk: &Pdk) -> Requirements<Routes> {
    let shapes: Vec<Shape> = routes.wires.iter().flatten().copied().collect();

    let mut hard: Vec<AntennaViol> = Vec::new();

    let drc = drc::run_drc_inloop(&shapes, pdk);
    for v in &drc.violations {
        hard.push(AntennaViol::from_margin(v.limit - v.measured));
    }

    let erc = erc::run_erc_inloop(&shapes, pdk);
    for _ in &erc.violations {
        hard.push(AntennaViol::from_margin(1));
    }

    let pex = pex::run_pex(&shapes, pdk);
    let cost = PexCost { cap_ff: pex::total_cap_ff(&pex) };

    let mut req = Requirements::default();
    if !hard.is_empty() {
        req.hard.push(Box::new(hard));
    }
    req.cost.push(Box::new(vec![cost]));
    req
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::Layout;

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

    // A family with no evidence supplied is NotApplicable (does not block); once
    // its input is present it becomes applicable and a non-clean result blocks.
    #[test]
    fn erc_check_applicability_tracks_supplied_evidence() {
        use erc::SignoffCheck::*;
        let empty = erc::SignoffConfig::default();
        for c in [Antenna, DensityCmp, IrDrop, Electromigration, Reliability, EsdLatchup] {
            assert!(!erc_check_applicable(c, &empty), "{c:?} inapplicable with no evidence");
        }
        // Supplying the power grid makes both power-family checks applicable.
        let mut cfg = erc::SignoffConfig::default();
        cfg.power = Some(power_signoff_stub());
        assert!(erc_check_applicable(IrDrop, &cfg));
        assert!(erc_check_applicable(Electromigration, &cfg));
        // ...but not the unrelated families.
        assert!(!erc_check_applicable(Antenna, &cfg));
    }

    // Minimal empty-grid PowerSignoffConfig to flip the power input to Some.
    fn power_signoff_stub() -> gdsverify::PowerSignoffConfig {
        gdsverify::PowerSignoffConfig {
            grid: gdsverify::PowerGrid::default(),
            solver: gdsverify::PowerSolveConfig::default(),
            ir_drop: None,
            electromigration: None,
        }
    }

    // PexCost is Cost-only: satisfied regardless, contributes its cap.
    #[test]
    fn pex_cost_is_objective_only() {
        let routes = Routes { wires: vec![] };
        let c = PexCost { cap_ff: 12.5 };
        assert!(c.satisfied(&routes));
        assert_eq!(c.cost(&routes), 12.5);
    }
}
