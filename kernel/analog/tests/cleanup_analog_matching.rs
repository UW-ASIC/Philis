//! Corner-case and property coverage for the analog matching unit: class
//! tables, mismatch ledger math, unit moments, common-centroid patterns, DAC
//! figures and the rule-batch plumbing. Oracles are the doc contracts, the
//! cited formulas recomputed naively, and metamorphic relations (translation,
//! scale, permutation).

use analog::intent::{BatchMeta, ConstraintId, Origin};
use analog::matching::class::{self, ClassLimit, Family, GateStrap, MatchClass, MatchKind};
use analog::matching::dac;
use analog::matching::mismatch::{self, Budget, Ledger, LedgerUnit};
use analog::matching::moments::{self, Axis, Pt};
use analog::matching::pattern::{self, Fill, Grid, Outer};
use analog::matching::sizing;
use analog::rule::{RepairKind, Rule, RuleBatch, Tagged};
use analog::Requirements;
use pnr_core::{DeviceKind, Process};

const CLASSES: [MatchClass; 3] = [MatchClass::Minimal, MatchClass::Moderate, MatchClass::Exceptional];

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

// ---------------------------------------------------------------- class.rs

/// A process with every tier key, value `base[key] + class index`, or none.
struct Tiers {
    full: bool,
    /// Key that is missing on Exceptional only.
    hole: Option<&'static str>,
}

impl Process for Tiers {
    fn layer(&self, _: &str) -> Option<pnr_core::LayerId> {
        None
    }
    fn rule(&self, _: &str, default: i32) -> i32 {
        default
    }
    fn grid(&self) -> i32 {
        1
    }
    fn tier(&self, key: &str, c: MatchClass) -> Option<i32> {
        if !self.full || (self.hole == Some(key) && c == MatchClass::Exceptional) {
            return None;
        }
        let base = match key {
            "dummy_reach_nm" => 100,
            "lod_moat_ext_nm" => 200,
            "wpe_clearance_nm" => 300,
            "gate_ext_extra_nm" => 400,
            "res_dummy_span_nm" => 500,
            "res_width_floor_permille" => 600,
            "res_length_floor_x" => 700,
            _ => return None,
        };
        Some(base + c as i32)
    }
}

#[test]
fn family_of_every_device_kind() {
    let cases = [
        (DeviceKind::Nmos, Some(Family::Mos)),
        (DeviceKind::Pmos, Some(Family::Mos)),
        (DeviceKind::Npn, Some(Family::Bipolar)),
        (DeviceKind::Pnp, Some(Family::Bipolar)),
        (DeviceKind::Diode, Some(Family::Diode)),
        (DeviceKind::Resistor, Some(Family::Resistor)),
        (DeviceKind::Capacitor, Some(Family::Capacitor)),
        (DeviceKind::Inductor, None),
    ];
    for (k, want) in cases {
        assert_eq!(Family::of(k), want, "{k:?}");
    }
}

#[test]
fn mos_env_reads_each_tier_and_scales_the_strap() {
    let p = Tiers { full: true, hole: None };
    let straps = [GateStrap::PolyBar, GateStrap::PolyBarFar, GateStrap::MetalIsolated];
    for (c, strap) in CLASSES.into_iter().zip(straps) {
        let e = class::mos_env(c, &p);
        let i = c as i32;
        assert_eq!(
            e,
            class::MosEnv { dummy_reach_nm: 100 + i, moat_nm: 200 + i, wpe_nm: 300 + i, gate_ext_extra_nm: 400 + i, gate_strap: strap },
            "{c:?}"
        );
    }
}

#[test]
fn resistor_env_always_asks_one_dummy() {
    let p = Tiers { full: true, hole: None };
    for c in CLASSES {
        let i = c as i32;
        let e = class::resistor_env(c, &p);
        assert_eq!(e, class::PassiveEnv { min_dummies: 1, dummy_span_nm: 500 + i, width_floor_permille: 600 + i, length_floor_x: 700 + i });
    }
    // No tiers at all: every number reads 0, the dummy floor stays.
    let e = class::resistor_env(MatchClass::Exceptional, &Tiers { full: false, hole: None });
    assert_eq!(e, class::PassiveEnv { min_dummies: 1, ..Default::default() });
}

#[test]
fn missing_tiers_lists_every_key_in_order_or_nothing() {
    let none: Vec<_> = class::missing_tiers(&Tiers { full: false, hole: None }).collect();
    assert_eq!(
        none,
        [
            "dummy_reach_nm tier missing",
            "gate_ext_extra_nm tier missing",
            "lod_moat_ext_nm tier missing",
            "res_dummy_span_nm tier missing",
            "res_length_floor_x tier missing",
            "res_width_floor_permille tier missing",
            "wpe_clearance_nm tier missing",
        ]
    );
    assert_eq!(class::missing_tiers(&Tiers { full: true, hole: None }).count(), 0);
    // A tier missing on one class only is still missing.
    let one: Vec<_> = class::missing_tiers(&Tiers { full: true, hole: Some("wpe_clearance_nm") }).collect();
    assert_eq!(one, ["wpe_clearance_nm tier missing"]);
    // ...and that class reads 0 for it.
    assert_eq!(class::mos_env(MatchClass::Exceptional, &Tiers { full: true, hole: Some("wpe_clearance_nm") }).wpe_nm, 0);
}

#[test]
fn limit_is_defined_exactly_where_hastings_tabulates_one() {
    let families = [Family::Mos, Family::Bipolar, Family::Diode, Family::Resistor, Family::Capacitor];
    let kinds = [MatchKind::Voltage, MatchKind::Current, MatchKind::Ratio];
    for f in families {
        for k in kinds {
            let defined = matches!(
                (f, k),
                (Family::Mos | Family::Bipolar, MatchKind::Voltage | MatchKind::Current) | (Family::Resistor | Family::Capacitor, MatchKind::Ratio)
            );
            for c in CLASSES {
                assert_eq!(class::limit(f, k, c).is_some(), defined, "{f:?} {k:?} {c:?}");
            }
            if !defined {
                continue;
            }
            // Tighter class, tighter limit (strictly), same unit throughout.
            let v: Vec<ClassLimit> = CLASSES.iter().map(|&c| class::limit(f, k, c).unwrap()).collect();
            let val = |l: ClassLimit| match l {
                ClassLimit::Mv(x) | ClassLimit::Pct(x) => x,
            };
            assert!(val(v[0]) > val(v[1]) && val(v[1]) > val(v[2]), "{f:?} {k:?}: {v:?}");
            let mv = |l: &ClassLimit| matches!(l, ClassLimit::Mv(_));
            assert!(v.iter().all(|l| mv(l) == (k == MatchKind::Voltage)), "{f:?} {k:?}: {v:?}");
        }
    }
}

#[test]
fn phi_arm_tightens_with_class() {
    assert_eq!(CLASSES.map(class::phi_arm), [None, Some(false), Some(true)]);
}

// ------------------------------------------------------------- mismatch.rs

#[test]
fn sigma_pair_equal_areas_is_a_over_root_area() {
    for a in [0.25f32, 1.0, 4.0, 100.0] {
        assert!((mismatch::sigma_pair(10.0, a, a) - 10.0 / a.sqrt()).abs() < 1e-5, "{a}");
    }
    // Symmetric in the two areas.
    assert_eq!(mismatch::sigma_pair(3.0, 2.0, 7.0), mismatch::sigma_pair(3.0, 7.0, 2.0));
}

#[test]
fn sigma_pair_is_zero_for_a_degenerate_area() {
    for (a1, a2) in [(0.0, 1.0), (1.0, 0.0), (-1.0, 4.0), (4.0, -1.0), (0.0, 0.0)] {
        assert_eq!(mismatch::sigma_pair(10.0, a1, a2), 0.0, "{a1} {a2}");
    }
}

#[test]
fn current_and_voltage_domains_reduce_to_the_vt_term_without_beta() {
    // σ_β = 0: %-domain is 0.1·G·σ_VT, mV-domain is σ_VT.
    assert!((mismatch::sigma_current_pct(2.0, 10.0, 0.0) - 2.0).abs() < 1e-6);
    assert!((mismatch::sigma_voltage_mv(2.0, 10.0, 0.0) - 2.0).abs() < 1e-6);
    // σ_VT = 0: the β term alone, 10·σ_β/G in mV.
    assert!((mismatch::sigma_voltage_mv(0.0, 10.0, 0.5) - 0.5).abs() < 1e-6);
    assert!((mismatch::sigma_current_pct(0.0, 10.0, 0.5) - 0.5).abs() < 1e-6);
}

#[test]
fn ledger_unit_names() {
    assert_eq!((LedgerUnit::Mv.as_str(), LedgerUnit::Pct.as_str()), ("mV", "%"));
    assert_eq!(LedgerUnit::default(), LedgerUnit::Mv);
}

#[test]
fn every_budget_variant_gives_its_documented_allowance() {
    assert!((Budget::Eta(0.3).allowance(2.0) - 0.6).abs() < 1e-6);
    assert_eq!(Budget::Allowance(0.42).allowance(100.0), 0.42);
    // 5² − 3² = 4² for both total budgets.
    assert!((Budget::Sigma1Mv(5.0).allowance(3.0) - 4.0).abs() < 1e-6);
    assert!((Budget::Sigma1Pct(5.0).allowance(3.0) - 4.0).abs() < 1e-6);
    // At exactly σ_rand the layout gets nothing, and never a negative share.
    assert_eq!(Budget::Sigma1Mv(3.0).allowance(3.0), 0.0);
    assert_eq!(Budget::Sigma1Pct(1.0).allowance(3.0), 0.0);
    // No random σ: the whole budget goes to the layout.
    assert_eq!(Budget::Sigma1Mv(2.0).allowance(0.0), 2.0);
}

#[test]
fn to_pct_scales_only_mv_quantities() {
    assert_eq!(Budget::Sigma1Mv(2.0).to_pct(10.0), Budget::Sigma1Pct(2.0));
    assert_eq!(Budget::Allowance(3.0).to_pct(5.0), Budget::Allowance(1.5));
    assert_eq!(Budget::Eta(0.3).to_pct(10.0), Budget::Eta(0.3));
    assert_eq!(Budget::Sigma1Pct(0.7).to_pct(10.0), Budget::Sigma1Pct(0.7));
}

#[test]
fn from_class_every_limit_kind_combination() {
    use MatchKind::{Current, Ratio, Voltage};
    assert_eq!(Budget::from_class(ClassLimit::Mv(6.0), Voltage), Budget::Sigma1Mv(1.0));
    assert_eq!(Budget::from_class(ClassLimit::Mv(6.0), Current), Budget::Eta(mismatch::GRADIENT_SHARE));
    assert_eq!(Budget::from_class(ClassLimit::Mv(6.0), Ratio), Budget::Eta(mismatch::GRADIENT_SHARE));
    for k in [Voltage, Current, Ratio] {
        assert_eq!(Budget::from_class(ClassLimit::Pct(6.0), k), Budget::Sigma1Pct(1.0), "{k:?}");
    }
}

#[test]
fn choose_takes_the_first_source_given() {
    let k = MatchKind::Current;
    assert_eq!(mismatch::choose(Some(2.0), None, None, k), Budget::Sigma1Mv(2.0));
    assert_eq!(mismatch::choose(Some(2.0), Some(0.1), None, k), Budget::Sigma1Mv(2.0));
    assert_eq!(mismatch::choose(None, Some(0.1), None, k), Budget::Allowance(0.1));
    assert_eq!(mismatch::choose(None, None, Some(ClassLimit::Pct(3.0)), k), Budget::Sigma1Pct(0.5));
    assert_eq!(mismatch::choose(None, None, None, k), Budget::Eta(mismatch::GRADIENT_SHARE));
}

#[test]
fn svt_of_l_tends_to_root_a_at_long_channel() {
    assert!((mismatch::svt_of_l(4.0, 0.0, 0.1) - 2.0).abs() < 1e-6);
    assert!((mismatch::svt_of_l(4.0, 1.0, 1e3) - 2.0).abs() < 1e-5);
    // Shorter channel, larger coefficient.
    assert!(mismatch::svt_of_l(4.0, 1.0, 0.1) > mismatch::svt_of_l(4.0, 1.0, 0.2));
}

#[test]
fn sigma_grad_is_zero_at_coincidence_and_isotropic_by_distance() {
    assert_eq!(mismatch::sigma_grad_mv((3.0, 3.0), 0.0, 0.0), 0.0);
    // Isotropic S: depends only on |Δm| (3-4-5 triangle).
    let a = mismatch::sigma_grad_mv((2.0, 2.0), 3000.0, 4000.0);
    let b = mismatch::sigma_grad_mv((2.0, 2.0), 5000.0, 0.0);
    assert!((a - b).abs() < 1e-8 && (a - 0.01).abs() < 1e-8, "{a} {b}");
    // Sign of the offset does not matter.
    assert_eq!(mismatch::sigma_grad_mv((1.0, 2.0), -100.0, -50.0), mismatch::sigma_grad_mv((1.0, 2.0), 100.0, 50.0));
}

#[test]
fn mobility_term_is_antisymmetric_and_zero_at_equal_temperature() {
    assert_eq!(mismatch::mobility_pct(1.7, 0.0, 300.0), 0.0);
    assert_eq!(mismatch::mobility_pct(1.7, -2.0, 300.0), -mismatch::mobility_pct(1.7, 2.0, 300.0));
    // Warmer member, lower mobility.
    assert!(mismatch::mobility_pct(1.5, 1.0, 300.0) < 0.0);
}

#[test]
fn bjt_vbe_is_zero_without_mismatch_and_monotone() {
    assert_eq!(mismatch::bjt_sigma_vbe_mv(0.0), 0.0);
    assert!(mismatch::bjt_sigma_vbe_mv(1.0) < mismatch::bjt_sigma_vbe_mv(2.0));
    // ln(1 + 1%) · 25.7 mV.
    assert!((mismatch::bjt_sigma_vbe_mv(1.0) - 25.7 * 0.01f32.ln_1p()).abs() < 1e-6);
}

#[test]
fn passive_thermal_term_is_linear_in_both_factors() {
    assert_eq!(mismatch::ratio_thermal_pct(0.0, 1000.0), 0.0);
    assert_eq!(mismatch::ratio_thermal_pct(100.0, 0.0), 0.0);
    // 1000 ppm/K over 1 K (1000 mK) is 0.1 %.
    assert!((mismatch::ratio_thermal_pct(1000.0, 1000.0) - 0.1).abs() < 1e-7);
}

#[test]
fn ledger_spends_every_systematic_term_once() {
    let l = Ledger { sigma_grad: 0.1, mu_thermal: 0.2, mu_lod: 0.3, sigma_rand: 9.0, ..Ledger::default() };
    assert!((l.spent() - 0.6).abs() < 1e-6, "σ_rand is not a systematic spend");
}

#[test]
fn ledger_at_its_allowance_is_fully_used_and_not_over() {
    let l = Ledger { mu_lod: 0.5, allowance: 0.5, ..Ledger::default() };
    assert!((l.usage() - 1.0).abs() < 1e-6);
    assert_eq!(l.residual(), 0.0);
}

#[test]
fn coincidence_binds_at_one() {
    let base = Ledger { mu_lod: 0.1, allowance: 1.0, ..Ledger::default() };
    // Under 1: the allowance check is the tighter one.
    let under = Ledger { coincidence: Some(0.05), ..base };
    assert!((under.usage() - 0.1).abs() < 1e-6);
    assert_eq!(under.residual(), 0.0);
    // Above 1: usage and residual follow it.
    let over = Ledger { coincidence: Some(1.5), ..base };
    assert!((over.usage() - 1.5).abs() < 1e-6);
    assert!((over.residual() - 0.5).abs() < 1e-6);
    // Exactly 1 is still a pass.
    assert_eq!(Ledger { coincidence: Some(1.0), ..base }.residual(), 0.0);
}

// -------------------------------------------------------------- moments.rs

fn p(x: f64, y: f64) -> Pt {
    Pt { x, y, w: 1.0, phi: (0, 0) }
}

#[test]
fn sums_of_nothing_carry_no_centroid() {
    let s = moments::sums(std::iter::empty());
    assert_eq!(s, moments::Sums::default());
    assert_eq!(s.axis, Axis::None);
    assert_eq!(s.centroid(), None);
    assert_eq!(s.second((5.0, 5.0)), [0.0; 3]);
    assert_eq!(s.phi(), (0.0, 0.0));
}

#[test]
fn zero_weight_units_count_for_phi_but_not_the_centroid() {
    let s = moments::sums([Pt { x: 1.0, y: 2.0, w: 0.0, phi: (1, 0) }]);
    assert_eq!(s.n, 1);
    assert_eq!(s.centroid(), None);
    assert_eq!(s.phi(), (1.0, 0.0));
    assert_eq!(s.axis, Axis::H);
}

#[test]
fn centroid_and_second_moment_match_a_naive_recomputation() {
    let pts = [Pt { x: 1.0, y: 5.0, w: 2.0, phi: (0, 1) }, Pt { x: -3.0, y: 2.0, w: 1.0, phi: (0, -1) }, Pt { x: 4.0, y: -1.0, w: 3.0, phi: (0, 1) }];
    let s = moments::sums(pts);
    let w: f64 = pts.iter().map(|q| q.w).sum();
    let cx = pts.iter().map(|q| q.w * q.x).sum::<f64>() / w;
    let cy = pts.iter().map(|q| q.w * q.y).sum::<f64>() / w;
    let (gx, gy) = s.centroid().unwrap();
    assert!(close(gx, cx, 1e-12) && close(gy, cy, 1e-12));
    let naive = [
        pts.iter().map(|q| q.w * (q.x - cx) * (q.x - cx)).sum::<f64>() / w,
        pts.iter().map(|q| q.w * (q.x - cx) * (q.y - cy)).sum::<f64>() / w,
        pts.iter().map(|q| q.w * (q.y - cy) * (q.y - cy)).sum::<f64>() / w,
    ];
    let got = s.second((cx, cy));
    for k in 0..3 {
        assert!(close(got[k], naive[k], 1e-9), "{k}: {got:?} vs {naive:?}");
    }
    assert_eq!(s.axis, Axis::V);
    assert!(close(s.phi().1, 1.0 / 3.0, 1e-12));
}

#[test]
fn phi_equal_handles_empty_and_unequal_counts() {
    let mk = |phis: &[i8]| moments::sums(phis.iter().map(|&f| Pt { x: 0.0, y: 0.0, w: 1.0, phi: (f, 0) }));
    assert!(moments::phi_equal(&mk(&[]), &mk(&[])));
    assert!(moments::phi_equal(&mk(&[1]), &mk(&[1, 1, 1])));
    assert!(!moments::phi_equal(&mk(&[1, -1]), &mk(&[1, 1])));
    // The y axis is compared too.
    let a = moments::sums([Pt { x: 0.0, y: 0.0, w: 1.0, phi: (0, 1) }]);
    let b = moments::sums([Pt { x: 0.0, y: 0.0, w: 1.0, phi: (0, -1) }]);
    assert!(!moments::phi_equal(&a, &b));
}

fn unit(owner: u8, phi: (i8, i8)) -> pnr_core::Unit {
    pnr_core::Unit { owner, x: 0, y: 0, weight: 1, phi, sa: 0, sb: 0 }
}

#[test]
fn phi_equal_all_skips_absent_members_and_foreign_owners() {
    assert!(moments::phi_equal_all(&[], 3));
    assert!(moments::phi_equal_all(&[unit(0, (1, 0))], 0), "no member to compare");
    // Member 1 has no units: skipped, members 0 and 2 agree.
    assert!(moments::phi_equal_all(&[unit(0, (1, 0)), unit(2, (1, 0))], 3));
    // Owner 5 is outside `members` and is ignored even though it disagrees.
    assert!(moments::phi_equal_all(&[unit(0, (1, 0)), unit(1, (1, 0)), unit(5, (-1, 0))], 2));
    assert!(!moments::phi_equal_all(&[unit(0, (1, 0)), unit(1, (-1, 0))], 2));
}

#[test]
fn mirror_is_allowed_only_without_net_phi_x() {
    assert!(moments::mirror_allowed(&[]));
    assert!(moments::mirror_allowed_units(&[]));
    assert!(moments::mirror_allowed_units(&[unit(0, (1, 0)), unit(0, (-1, 0)), unit(1, (0, 1))]));
    // Owner 3 alone carries net φx: refused whatever the others do.
    assert!(!moments::mirror_allowed_units(&[unit(0, (1, 0)), unit(0, (-1, 0)), unit(3, (-1, 0))]));
}

#[test]
fn pt_from_unit_keeps_every_field() {
    let u = pnr_core::Unit { owner: 2, x: -7, y: 9, weight: 1234, phi: (-1, 1), sa: 5, sb: 6 };
    assert_eq!(Pt::from(u), Pt { x: -7.0, y: 9.0, w: 1234.0, phi: (-1, 1) });
}

#[test]
fn cancelled_order_degenerate_figures_cancel_everything() {
    assert_eq!(moments::cancelled_order(&[], 3, 1e-9), (3, [0.0; 5]));
    let a = [p(5.0, 5.0)];
    let b = [p(5.0, 5.0), p(5.0, 5.0)];
    assert_eq!(moments::cancelled_order(&[&a, &b], 4, 1e-9), (4, [0.0; 5]), "every point at the centroid");
    // One device: no pair to mismatch.
    let one = [p(0.0, 0.0), p(1.0, 3.0)];
    assert_eq!(moments::cancelled_order(&[&one], 2, 1e-9).0, 2);
    // nmax 0 asks nothing.
    assert_eq!(moments::cancelled_order(&[&a, &one], 0, 1e-9), (0, [0.0; 5]));
}

#[test]
fn cancelled_order_skips_weightless_devices() {
    let a = [p(0.0, 0.0), p(3.0, 0.0)];
    let b = [p(1.0, 0.0), p(2.0, 0.0)];
    let ghost = [Pt { x: 100.0, y: 0.0, w: 0.0, phi: (0, 0) }];
    let with = moments::cancelled_order(&[&a, &b, &ghost], 2, 1e-9);
    let without = moments::cancelled_order(&[&a, &b], 2, 1e-9);
    assert_eq!(with, without);
}

#[test]
fn cancelled_order_is_translation_and_scale_invariant() {
    let a = [p(0.0, 0.0), p(3.0, 1.0), p(1.0, 4.0)];
    let b = [p(2.0, 2.0), p(1.0, 1.0)];
    let (o, r) = moments::cancelled_order(&[&a, &b], 4, 1e-9);
    let moved = |v: &[Pt]| v.iter().map(|q| p(q.x * 250.0 - 7.0, q.y * 250.0 + 1e4)).collect::<Vec<_>>();
    let (am, bm) = (moved(&a), moved(&b));
    let (om, rm) = moments::cancelled_order(&[&am, &bm], 4, 1e-9);
    assert_eq!(o, om);
    for n in 0..5 {
        assert!(close(r[n], rm[n], 1e-9), "{n}: {r:?} vs {rm:?}");
    }
    // Order and residuals do not depend on device order.
    let (os, rs) = moments::cancelled_order(&[&b, &a], 4, 1e-9);
    assert_eq!(os, o);
    for n in 0..5 {
        assert!(close(r[n], rs[n], 1e-12));
    }
}

#[test]
fn cancelled_order_stops_at_the_first_failing_order() {
    // Common centroid (order 1) but unequal second moments, then a later
    // order that happens to match must not count.
    let a = [p(-2.0, 0.0), p(2.0, 0.0)];
    let b = [p(-1.0, 0.0), p(1.0, 0.0)];
    let (o, r) = moments::cancelled_order(&[&a, &b], 3, 1e-9);
    assert_eq!(o, 1);
    assert!(r[2] > 0.0);
    assert!(r[3] <= 1e-9, "odd orders cancel by symmetry: {r:?}");
}

// -------------------------------------------------------------- pattern.rs

#[test]
fn grids_of_nothing_is_empty() {
    assert!(pattern::grids(&[], 3.0).is_empty());
    assert!(pattern::grids(&[0], 3.0).is_empty());
}

#[test]
fn grids_always_fit_and_offer_the_single_row() {
    for a in 0..=9u16 {
        for b in 0..=9u16 {
            for c in [0u16, 1, 2, 5] {
                let counts = [a, b, c];
                let t = usize::from(a + b + c);
                let g = pattern::grids(&counts, 2.0);
                if t == 0 {
                    assert!(g.is_empty());
                    continue;
                }
                assert!(g.contains(&(1, t)), "{counts:?}: {g:?}");
                // Past the aspect filter the row is the appended fallback.
                if t as f64 > 2.0 {
                    assert_eq!(*g.last().unwrap(), (1, t), "{counts:?}: {g:?}");
                }
                let one_odd = counts.iter().filter(|&&k| k % 2 == 1).count() == 1;
                for (i, &(r, k)) in g.iter().enumerate() {
                    assert!(r * k >= t, "{counts:?}: {r}×{k}");
                    if (r, k) != (1, t) {
                        assert!(r.max(k) as f64 / r.min(k) as f64 <= 2.0, "{counts:?}: {r}×{k} exceeds aspect");
                    }
                    if one_odd {
                        assert!(r % 2 == 1 && k % 2 == 1, "{counts:?}: one odd member needs a centre cell, got {r}×{k}");
                    }
                    assert!(!g[..i].contains(&(r, k)), "{counts:?}: duplicate {r}×{k}");
                }
            }
        }
    }
}

#[test]
fn cc_feasible_counts_odd_members() {
    assert!(pattern::cc_feasible(&[]));
    assert!(pattern::cc_feasible(&[3]));
    assert!(pattern::cc_feasible(&[2, 4, 7]));
    assert!(!pattern::cc_feasible(&[1, 3]));
}

#[test]
fn spiral_is_a_permutation_ordered_by_radius() {
    assert!(pattern::spiral(0, 5).is_empty());
    for (rows, cols) in [(1, 1), (1, 6), (3, 3), (4, 5), (6, 2)] {
        let s = pattern::spiral(rows, cols);
        let mut sorted = s.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..rows * cols).collect::<Vec<_>>(), "{rows}×{cols}");
        let r2 = |i: usize| {
            let (dr, dc) = (2 * (i / cols) as i64 - rows as i64 + 1, 2 * (i % cols) as i64 - cols as i64 + 1);
            dr * dr + dc * dc
        };
        assert!(s.windows(2).all(|w| r2(w[0]) <= r2(w[1])), "{rows}×{cols}: {s:?}");
        if rows % 2 == 1 && cols % 2 == 1 {
            assert_eq!(s[0], rows * cols / 2, "odd grid starts at the centre");
        }
    }
}

/// Counts kept, owners closed under the 180° rotation.
fn assert_exact(counts: &[u16], rows: usize, cols: usize, fill: Fill) {
    let (slot, exact) = pattern::centro_assign(counts, rows, cols, fill);
    let n = slot.len();
    assert_eq!(n, rows * cols);
    assert!(exact, "{counts:?} {rows}×{cols} {fill:?}: {slot:?}");
    assert!((0..n).all(|i| slot[i] == slot[n - 1 - i]), "{counts:?} {fill:?}");
    for (d, &k) in counts.iter().enumerate() {
        assert_eq!(slot.iter().filter(|&&s| s == Some(d as u8)).count(), usize::from(k), "{counts:?} {fill:?} member {d}");
    }
}

#[test]
fn every_fill_is_exact_on_every_offered_grid() {
    for fill in [Fill::Compact, Fill::Dispersed, Fill::Balanced] {
        for a in 1..=6u16 {
            for b in 1..=8u16 {
                for counts in [vec![a, b], vec![a, b, 2]] {
                    if !pattern::cc_feasible(&counts) {
                        continue;
                    }
                    for (rows, cols) in pattern::grids(&counts, 3.0) {
                        assert_exact(&counts, rows, cols, fill);
                    }
                }
            }
        }
    }
}

#[test]
fn centro_assign_corner_cases() {
    // No members: every cell empty and trivially exact.
    assert_eq!(pattern::centro_assign(&[], 2, 2, Fill::Balanced), (vec![None; 4], true));
    assert_eq!(pattern::centro_assign(&[], 0, 0, Fill::Compact), (vec![], true));
    // One unit on a 1×1 grid takes the centre.
    assert_eq!(pattern::centro_assign(&[1], 1, 1, Fill::Dispersed), (vec![Some(0)], true));
    // A member with zero units owns nothing.
    let (slot, exact) = pattern::centro_assign(&[0, 2], 1, 2, Fill::Compact);
    assert_eq!((slot, exact), (vec![Some(1), Some(1)], true));
    // Spare cells stay empty and pair up.
    assert_exact(&[2, 2], 3, 3, Fill::Balanced);
}

#[test]
fn segment_row_corner_cases() {
    assert_eq!(pattern::segment_row(&[]), (vec![], true));
    assert_eq!(pattern::segment_row(&[3]), (vec![0, 0, 0], true));
    let (row, exact) = pattern::segment_row(&[1, 1]);
    assert!(!exact);
    let mut sorted = row.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, [0, 1]);
}

#[test]
fn scale2_doubles_only_infeasible_sets() {
    assert!(pattern::scale2(&[]).is_empty());
    assert_eq!(pattern::scale2(&[1, 1]), [2, 2]);
    assert_eq!(pattern::scale2(&[1, 2, 3]), [2, 4, 6]);
    assert_eq!(pattern::scale2(&[1, 2]), [1, 2]);
    // Doubling always makes a set feasible.
    for a in 0..6u16 {
        for b in 0..6u16 {
            assert!(pattern::cc_feasible(&pattern::scale2(&[a, b, 1])));
        }
    }
}

#[test]
fn grid_reflection_and_pairs() {
    let mut g = Grid { cols: 3, slot: vec![None; 9] };
    assert_eq!((g.refl(0), g.refl(4), g.refl(8)), (8, 4, 0));
    assert!(!g.free_pair(4), "the centre of an odd grid is not a pair");
    assert!(g.free_pair(1));
    g.put_pair(1, 7);
    assert_eq!((g.slot[1], g.slot[7]), (Some(7), Some(7)));
    assert!(!g.free_pair(1) && !g.free_pair(7));
    g.slot[2] = Some(0);
    assert!(!g.free_pair(6), "a pair with one side taken is not free");
}

/// Owner → number of cells.
fn histogram(slot: &[Option<u8>]) -> std::collections::BTreeMap<u8, usize> {
    let mut h = std::collections::BTreeMap::new();
    for s in slot.iter().flatten() {
        *h.entry(*s).or_insert(0) += 1;
    }
    h
}

#[test]
fn chessboard_gives_each_bit_its_binary_weight() {
    for (rows, cols, top) in [(4usize, 4usize, 4u8), (2, 4, 3), (4, 8, 5)] {
        let mut g = Grid { cols, slot: vec![None; rows * cols] };
        g.chessboard((0, 0, rows, cols), top);
        assert!(g.slot.iter().all(Option::is_some), "{rows}×{cols}: unfilled");
        let h = histogram(&g.slot);
        assert_eq!((h[&0], h[&1]), (1, 1));
        for b in 2..=top {
            assert_eq!(h[&b], 1 << (b - 1), "{rows}×{cols} C{b}");
        }
        let n = g.slot.len();
        for i in 0..n {
            if g.slot[i].is_some_and(|s| s >= 2) {
                assert_eq!(g.slot[i], g.slot[n - 1 - i], "{rows}×{cols}: C{:?} not centro", g.slot[i]);
            }
        }
        // C0 and C1 share the most central reflected pair.
        let c0 = g.slot.iter().position(|&s| s == Some(0)).unwrap();
        assert_eq!(g.slot[n - 1 - c0], Some(1));
    }
}

#[test]
fn corridor_fills_the_ring_with_two_bits() {
    // cap_array's BlockChessboard on 4×4: C0..C2 chessboard in the centre
    // 2×2, then the corridor carries C3 (4 cells) and C4 (the other 8).
    let mut g = Grid { cols: 4, slot: vec![None; 16] };
    g.chessboard((1, 1, 2, 2), 2);
    g.corridor((0, 0, 4, 4), (1, 1, 2, 2), 3, 1);
    assert!(g.slot.iter().all(Option::is_some));
    let h = histogram(&g.slot);
    assert_eq!(h.values().copied().collect::<Vec<_>>(), [1, 1, 2, 4, 8], "{h:?}");
    let n = g.slot.len();
    for i in 0..n {
        if g.slot[i].is_some_and(|s| s >= 3) {
            assert_eq!(g.slot[i], g.slot[n - 1 - i]);
        }
    }
    // The inner rectangle is untouched by the corridor.
    for i in [5, 6, 9, 10] {
        assert!(g.slot[i].is_some_and(|s| s <= 2), "cell {i}: {:?}", g.slot[i]);
    }
}

#[test]
fn diffusion_legal_boundaries() {
    assert!(pattern::diffusion_legal(&[], Outer::Drain));
    assert!(pattern::diffusion_legal(&[3], Outer::Source));
    // Boundary after finger 0 is region 1: a source in a Drain-outer row,
    // a drain in a Source-outer row.
    assert!(pattern::diffusion_legal(&[0, 1], Outer::Drain));
    assert!(!pattern::diffusion_legal(&[0, 1], Outer::Source));
    assert!(pattern::diffusion_legal(&[0, 0, 1], Outer::Source));
    assert!(!pattern::diffusion_legal(&[0, 0, 1], Outer::Drain));
    // Same-device neighbours never violate.
    assert!(pattern::diffusion_legal(&[2, 2, 2, 2], Outer::Drain));
}

#[test]
fn nth_order_rows_order_one_is_the_row_itself() {
    assert_eq!(pattern::nth_order_rows(1, &[0, 1, 1, 0]), vec![vec![0, 1, 1, 0]]);
    assert_eq!(pattern::nth_order_rows(3, &[]), vec![Vec::<u8>::new(); 4]);
}

#[test]
fn segmentation_corner_cases() {
    assert!(pattern::segmentation(1e3, 2e3, 0).is_empty());
    // An exact 1:2 ratio matches at every N.
    for (k, &(n, m, s)) in pattern::segmentation(1e3, 2e3, 8).iter().enumerate() {
        assert_eq!((usize::from(n), m), (k + 1, 2 * n));
        assert!(s.abs() < 1e-15, "{n}: {s}");
    }
}

#[test]
fn partial_segments_on_an_exact_divisor() {
    // R0 divides both: no partial segment, S = |(N+1)/N − (M+1)/M|.
    let (m, n, j, k, s) = pattern::partial_segments(3e3, 5e3, 1e3);
    assert_eq!((m, n), (5, 3));
    assert!(j.abs() < 1e-12 && k.abs() < 1e-12);
    assert!(close(s, (4.0 / 3.0 - 6.0 / 5.0f64).abs(), 1e-12), "{s}");
}

#[test]
fn diffusion_cc_row_corner_cases() {
    assert!(pattern::diffusion_cc_row(&[], Outer::Drain).is_none());
    assert!(pattern::diffusion_cc_row(&[], Outer::Source).is_none(), "no fingers, no row");
    assert_eq!(pattern::diffusion_cc_row(&[4], Outer::Drain), Some(vec![0; 4]));
    assert_eq!(pattern::diffusion_cc_row(&[4], Outer::Source), Some(vec![0; 4]));
    // A zero-finger member owns nothing and does not block the others.
    let s = pattern::diffusion_cc_row(&[2, 0, 2], Outer::Drain).unwrap();
    assert!(!s.contains(&1));
    assert_eq!(s.len(), 4);
}

// ------------------------------------------------------------------ dac.rs

#[test]
fn gradient_caps_without_gradient_count_units() {
    let u = [(0u8, 1.0, 2.0), (2, -5.0, 3.0), (2, 4.0, -1.0)];
    assert_eq!(dac::gradient_caps(&u, 4, 0.0, 0.7), [1.0, 0.0, 2.0, 0.0]);
    assert_eq!(dac::gradient_caps(&[], 2, 1e-3, 0.0), [0.0, 0.0]);
    // θ = 0 tilts along +x: a unit at +x is thicker, so smaller.
    let c = dac::gradient_caps(&[(0, 10.0, 0.0), (1, -10.0, 0.0)], 2, 1e-3, 0.0);
    assert!(c[0] < 1.0 && c[1] > 1.0);
    assert!(close(c[0], 1.0 / 1.01, 1e-12));
}

/// Binary-weighted units for `n` bits (slot 0 the termination unit), all at
/// the origin: no gradient can mismatch them.
fn binary_at_origin(n: u8) -> Vec<(u8, f64, f64)> {
    let mut u = vec![(0u8, 0.0, 0.0)];
    for b in 1..=n {
        u.extend(std::iter::repeat_n((b, 0.0, 0.0), 1 << (b - 1)));
    }
    u
}

#[test]
fn ideal_binary_array_has_no_inl_or_dnl() {
    for n in 1..=5u8 {
        let (inl, dnl) = dac::inl_dnl(&binary_at_origin(n), n, 1e-3, 8);
        assert!(inl < 1e-12 && dnl < 1e-12, "{n}: {inl} {dnl}");
    }
}

#[test]
fn inl_dnl_degenerate_inputs_are_zero() {
    assert_eq!(dac::inl_dnl(&binary_at_origin(3), 3, 1e-3, 0), (0.0, 0.0));
    assert_eq!(dac::inl_dnl(&[], 3, 1e-3, 4), (0.0, 0.0));
}

#[test]
fn inl_dnl_matches_a_naive_transfer() {
    // One θ step (θ = 0), gradient along x, 2 bits laid out in a row.
    let u = [(0u8, -1.5, 0.0), (1, -0.5, 0.0), (2, 0.5, 0.0), (2, 1.5, 0.0)];
    let g = 0.05;
    let cap: Vec<f64> = (0..3).map(|s| u.iter().filter(|q| q.0 == s).map(|q| 1.0 / (1.0 + g * q.1)).sum()).collect();
    let total: f64 = cap.iter().sum();
    let t = |c: usize| ((c & 1) as f64 * cap[1] + ((c >> 1) & 1) as f64 * cap[2]) / total * 4.0;
    let inl = (0..4).map(|c| (t(c) - c as f64).abs()).fold(0.0, f64::max);
    let dnl = (0..3).map(|c| (t(c + 1) - t(c) - 1.0).abs()).fold(0.0, f64::max);
    let (gi, gd) = dac::inl_dnl(&u, 2, g, 1);
    assert!(close(gi, inl, 1e-12) && close(gd, dnl, 1e-12), "{gi} {gd} vs {inl} {dnl}");
}

#[test]
fn msys_corner_cases() {
    let u = [(0u8, -1.0, 0.0), (1, 1.0, 0.0)];
    assert_eq!(dac::msys(&u, &[1, 1], 0.0, 4), 0.0, "no gradient, no mismatch");
    assert_eq!(dac::msys(&u, &[1, 1], 1e-3, 0), 0.0, "no θ steps");
    assert_eq!(dac::msys(&[(0, 0.0, 0.0)], &[1], 1e-3, 4), 0.0, "one slot has nothing to compare");
    assert_eq!(dac::msys(&[], &[], 1e-3, 4), 0.0, "no slots");
    // Two units 2 µm apart along x, θ = 0: C_0 = 1/(1−g), C_1 = 1/(1+g),
    // so |C_1/C_0 − 1| = |(1−g)/(1+g) − 1|.
    let g = 1e-2;
    assert!(close(dac::msys(&u, &[1, 1], g, 1), ((1.0 - g) / (1.0 + g) - 1.0f64).abs(), 1e-12));
}

#[test]
fn second_um2_corner_cases() {
    assert_eq!(dac::second_um2(&[], 3), 0.0);
    // A single slot is the array: no residue.
    assert_eq!(dac::second_um2(&[(0, 1.0, 2.0), (0, -1.0, -2.0)], 1), 0.0);
    // Slots beyond `slots` are not reported (but still in the array mean).
    let u = [(0u8, 1.0, 0.0), (0, -1.0, 0.0), (1, 0.0, 1.0), (1, 0.0, -1.0)];
    assert_eq!(dac::second_um2(&u, 0), 0.0);
    assert!(close(dac::second_um2(&u, 1), dac::second_um2(&u, 2), 1e-12), "symmetric slots, same residue");
}

#[test]
fn attenuation_cap_edges() {
    assert_eq!(dac::attenuation_cap(0, 5), 0.0);
    assert_eq!(dac::attenuation_cap(7, 7), 1.0);
    assert!(dac::attenuation_cap(1, 0).is_infinite());
}

#[test]
fn nonunit_dims_at_the_discriminant_boundary() {
    // Unit 2×2: k = 1; A = 4 makes kA = 4, disc 0: the unit itself.
    assert_eq!(dac::nonunit_dims(4.0, 2.0, 2.0), Some((2.0, 2.0)));
    assert_eq!(dac::nonunit_dims(3.9, 2.0, 2.0), None);
    // Twice the unit area: still the unit's edge sensitivity.
    let (h, l) = dac::nonunit_dims(8.0, 2.0, 2.0).unwrap();
    assert!(h > l && close(h * l, 8.0, 1e-9) && close((h + l) / (h * l), 1.0, 1e-9), "{h} {l}");
}

#[test]
fn split_dac_every_cell_owned_and_counts_exact_on_minimal_grids() {
    for (lb, mb) in [(1u8, 1u8), (1, 3), (3, 1), (2, 3)] {
        let need = (1usize << lb) + (1usize << mb) + 1;
        let cols = need.isqrt() + 1;
        let rows = need.div_ceil(cols);
        let slot = dac::split_dac_assign(lb, mb, rows, cols);
        let h = histogram(&slot);
        let want: Vec<usize> = std::iter::once(1)
            .chain((0..lb).map(|k| 1 << k))
            .chain((0..mb).map(|k| 1 << k))
            .chain([2])
            .collect();
        for (d, &w) in want.iter().enumerate() {
            assert_eq!(h.get(&(d as u8)).copied().unwrap_or(0), w, "L={lb} M={mb} {rows}×{cols} id {d}");
        }
        assert_eq!(slot.iter().flatten().count(), need);
    }
}

// --------------------------------------------------------------- sizing.rs

#[test]
fn area_need_and_dstar_are_consistent_inverses() {
    // (6·A/ΔV)² inverted: the limit an area meets.
    for (avt, area) in [(9.5f32, 20.0f32), (5.0, 100.0), (12.5, 5625.0)] {
        let dv = 6.0 * avt / area.sqrt();
        assert!((sizing::area_need_um2(avt, dv) - area).abs() / area < 1e-5, "{avt} {area}");
    }
    // D* halves when the area quadruples.
    let d1 = sizing::dstar_um(9.5, 1.63, 25.0);
    let d4 = sizing::dstar_um(9.5, 1.63, 100.0);
    assert!((d1 / d4 - 2.0).abs() < 1e-5);
}

// ----------------------------------------------------------------- rule.rs

#[derive(Clone, Copy)]
struct Probe {
    id: u32,
    ok: bool,
    cost: f32,
    usage: Option<f32>,
    applicable: bool,
    known: bool,
    margin: f32,
    headroom: f32,
}

const PROBE: Probe = Probe { id: 0, ok: true, cost: 0.0, usage: None, applicable: true, known: true, margin: 0.0, headroom: 0.0 };

impl Rule for Probe {
    type On = ();
    fn cost(self, _: &()) -> f32 {
        self.cost
    }
    fn satisfied(self, _: &()) -> bool {
        self.ok
    }
    fn usage(self, _: &()) -> Option<f32> {
        self.usage
    }
    fn applicable(self, _: &()) -> bool {
        self.applicable
    }
    fn known(self, _: &()) -> bool {
        self.known
    }
    fn margin(self) -> f32 {
        self.margin
    }
    fn headroom(self, _: &()) -> f32 {
        self.headroom
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(self.id);
    }
}

#[test]
fn empty_batch_is_clean_and_silent() {
    let b: Vec<Probe> = Vec::new();
    assert_eq!((b.cost(&()), b.violations(&()), b.residual(&()), b.count()), (0.0, 0, 0.0, 0));
    assert_eq!((b.worst_cost(&()), b.worst_usage(&()), b.inapplicable(&()), b.unknown(&())), (0.0, None, 0, 0));
    assert_eq!(b.criticality(&()), 0.0, "nothing to be critical about");
    let mut ids = Vec::new();
    b.touched(&mut ids);
    b.violating_ids(&(), &mut ids);
    assert!(ids.is_empty());
}

#[test]
fn batch_aggregates_follow_their_definitions() {
    let b = vec![
        Probe { id: 1, cost: 2.0, usage: Some(0.4), ..PROBE },
        Probe { id: 2, ok: false, cost: 5.0, usage: Some(1.3), applicable: false, ..PROBE },
        Probe { id: 3, ok: false, cost: 3.0, known: false, ..PROBE },
    ];
    assert_eq!(b.cost(&()), 10.0);
    assert_eq!(b.violations(&()), 2);
    assert_eq!(b.worst_cost(&()), 5.0, "worst among violating rules only");
    assert_eq!(b.worst_usage(&()), Some(1.3));
    assert_eq!((b.inapplicable(&()), b.unknown(&())), (1, 1));
    assert_eq!(b.count(), 3);
    // Two violations with the default residual of 1.0 each.
    assert!((b.residual(&()) - 2.0).abs() < 1e-12);
    // Appends, never clears.
    let mut out = vec![99];
    b.violating_ids(&(), &mut out);
    assert_eq!(out, [99, 2, 3]);
}

#[test]
fn criticality_clamps_a_margin_of_one_or_more() {
    // margin ≥ 1 is clamped below 1: headroom 0 still reads fully critical,
    // and headroom 1 (all slack) reads zero.
    let tight = vec![Probe { margin: 5.0, headroom: 0.0, ..PROBE }];
    assert_eq!(tight.criticality(&()), 1.0);
    let slack = vec![Probe { margin: 5.0, headroom: 1.0, ..PROBE }];
    assert!(slack.criticality(&()) < 0.01, "{}", slack.criticality(&()));
    // Without margin, criticality is 1 − headroom, headroom clamped to [0, 1].
    assert!((vec![Probe { headroom: 0.25, ..PROBE }].criticality(&()) - 0.75).abs() < 1e-6);
    assert_eq!(vec![Probe { headroom: -3.0, ..PROBE }].criticality(&()), 1.0);
    assert_eq!(vec![Probe { headroom: 7.0, ..PROBE }].criticality(&()), 0.0);
}

/// A batch that can name its violators but not size them.
struct Named;
impl RuleBatch<()> for Named {
    fn cost(&self, _: &()) -> f32 {
        1.0
    }
    fn violations(&self, _: &()) -> u32 {
        1
    }
    fn violating_ids(&self, _: &(), out: &mut Vec<u32>) {
        out.extend([4, 5]);
    }
}

#[test]
fn default_batch_methods() {
    let b = Named;
    let mut out = Vec::new();
    b.violating_residuals(&(), &mut out);
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|&(_, r)| r.is_nan()), "unsplit residual reads NaN: {out:?}");
    assert_eq!((out[0].0, out[1].0), (4, 5));
    assert_eq!(b.residual(&()), 1.0);
    assert_eq!((b.kind(), b.repair_kind(), b.local(), b.reads_field(), b.count()), ("?", RepairKind::Reroute, false, false, 0));
    assert_eq!((b.criticality(&()), b.worst_usage(&()), b.matched_class()), (1.0, None, None));
    assert!(b.meta().is_none());
}

#[test]
fn tagged_forwards_everything_and_carries_its_meta() {
    let meta = BatchMeta { id: ConstraintId(7), origin: Origin::NetClass };
    let inner = vec![Probe { id: 1, ok: false, cost: 2.5, ..PROBE }, Probe { id: 2, cost: 1.0, ..PROBE }];
    let t = Tagged { meta, inner: Box::new(inner.clone()) };
    assert_eq!(t.meta(), Some(&meta));
    assert_eq!((t.cost(&()), t.violations(&()), t.count()), (inner.cost(&()), inner.violations(&()), 2));
    assert_eq!(t.kind(), inner.kind());
    assert_eq!(t.residual(&()), inner.residual(&()));
    let (mut a, mut b) = (Vec::new(), Vec::new());
    t.touched(&mut a);
    inner.touched(&mut b);
    assert_eq!(a, b);
    let (mut a, mut b) = (Vec::new(), Vec::new());
    t.violating_residuals(&(), &mut a);
    inner.violating_residuals(&(), &mut b);
    assert_eq!(a, b);
}

#[test]
fn requirements_default_is_empty() {
    let r: Requirements<()> = Requirements::default();
    assert!(r.hard.is_empty() && r.budget.is_empty() && r.cost.is_empty());
}
