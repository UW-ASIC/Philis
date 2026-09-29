//! Placement constraints from the recognised hierarchy.
//!
//! Each top-level block is a **stage** with axis `AxisId(block index)`. Its
//! 2-device leaves emit by kind (`a`, `b` = slot 0, slot 1):
//!
//! | kind          | emits                                                    |
//! |---------------|----------------------------------------------------------|
//! | DiffPair      | Symmetry, MatchingPair, ThermalGradient, centroid sides, DTI |
//! | CurrentMirror | Symmetry, MatchingPair, Proximity, ThermalGradient, sides, DTI |
//! | Load          | Symmetry, MatchingPair, ThermalGradient, sides, DTI      |
//! | Stack         | Proximity                                                |
//!
//! Every matched pair mirrors about its stage axis (a pair merged into one cell
//! centres on it). A stage holding a diff pair is differential: each member
//! outside a pair (the tail) is also self-symmetric with a Proximity pull to
//! the input pair.
//!
//! One batch per pair (per-batch criticality weights each pair by its own
//! urgency; one merged batch regressed the OTA). Arms: `SymmetryGroup` (one per stage) and `DtiBand` are hard + cost — the cost
//! copy is the gradient toward the hard set (and what prices dp's DTI branch
//! flip). `ThermalGradient`, `CentroidGroup` and `Proximity` (MAT-07, a
//! distance allowance) are budget + cost. `MatchingPair` is budget + cost when
//! the deck gives `A_VT` and `S_VT`, else a cost-only pull. Placement owns only the
//! distance term of Pelgrom; area is the cell generator's.
//!
//! Matching and thermal budgets come from the netlist's gate areas and the
//! deck's mismatch data; without them the documented fallbacks apply.

use analog::placement::cc::CentroidGroup;
use analog::placement::symmetry::SymmetryGroup;
use analog::placement::{DtiBand, Isolation, MatchingPair, Proximity, Symmetry, ThermalGradient};
use analog::Requirements;
use pnr_core::ids::{AxisId, BranchId, DeviceId, Target};
use pnr_core::layout::Layout;
use pnr_core::{DeviceKind, Netlist};

use crate::block::{leaves, Block, BlockKind};
use crate::ProcessNumbers;

/// ponytail: a fixed ΔT limit when the deck gives no `A_VT` or dVT/dT to
/// derive one from (see [`Pelgrom::thermal_limit_mc`]); a tuning number.
const THERMAL_MAX_DELTA_MC: i32 = 500;
/// Held back from the thermal spec so a converged run lands inside it.
const THERMAL_MARGIN_PCT: u8 = 20;
const PROXIMITY_NM: i32 = 5_000;
/// Placement's share η of a matched pair's mismatch when no offset budget is
/// given: the gradient term may reach this fraction of the random term the
/// sizing bought (σ grows ≤ 4.4%).
///
/// ponytail: Pelgrom prescribes no η; it is the circuit's to allocate. Set
/// `AnnotationConfig::offset_sigma_mv` to derive it.
const GRADIENT_SHARE: f32 = 0.3;

fn gate_um2(nl: &Netlist, d: DeviceId) -> f32 {
    crate::gate_um2(&nl.devices[d.0 as usize])
}

/// A matched pair's allowance for systematic offset beyond its random one,
/// `η·σ_rand`, mV: `η` as the matching rules allocate it (from
/// `offset_sigma_mv` when set). `None` without `A_VT` or a gate area.
#[must_use]
pub fn systematic_allowance_mv(avt: Option<f32>, gate_um2: f32, offset_sigma_mv: Option<f32>) -> Option<f32> {
    let pel = Pelgrom::new(avt, gate_um2, &ProcessNumbers::default(), offset_sigma_mv);
    pel.sigma_rand_mv.map(|s| pel.eta * s)
}

/// A matched set's Pelgrom numbers (Pelgrom & Duinmaijer 1988 eq.(1)).
#[derive(Clone, Copy)]
struct Pelgrom {
    /// Random term `σ_rand = A_VT/√(W·L)`, mV; `None` without `A_VT` or a gate.
    sigma_rand_mv: Option<f32>,
    /// η = allowed `σ_grad/σ_rand`.
    eta: f32,
    /// `S_VT/A_VT`, 1/µm²; `0` = unknown.
    s_over_a: f32,
}

impl Pelgrom {
    /// `avt` = the set's (tightest) `A_VT`, mV·µm; `gate` = its W·L, µm².
    /// With an offset budget σ_b the gradient may take what the random term
    /// leaves in quadrature: `η = √(σ_b² − σ_rand²)/σ_rand`, so
    /// `D ≤ η·A/(S·√WL)`; `η = 0` means the sizing alone spends the budget.
    fn new(avt: Option<f32>, gate: f32, p: &ProcessNumbers, offset_sigma_mv: Option<f32>) -> Self {
        let sigma_rand_mv = avt.filter(|&a| a > 0.0 && gate > 0.0).map(|a| a / gate.sqrt());
        let eta = match (sigma_rand_mv, offset_sigma_mv) {
            (Some(r), Some(b)) => (b * b - r * r).max(0.0).sqrt() / r,
            _ => GRADIENT_SHARE,
        };
        let s_over_a = avt.zip(p.svt_uv_per_um).filter(|&(a, s)| a > 0.0 && s > 0.0).map_or(0.0, |(a, s)| s * 1e-3 / a);
        Self { sigma_rand_mv, eta, s_over_a }
    }

    /// |ΔT| limit, m°C: the same allowance `η·σ_rand` spent as `TC·ΔT`
    /// (Hastings 3e eq.8.23, PDF p.388: mismatch ∝ TC·d·∂T/∂x), `tc` = the
    /// pair's polarity's |dVT/dT|, µV/K. Fixed fallback without `A_VT` or TC.
    fn thermal_limit_mc(self, tc: Option<f32>) -> i32 {
        match (self.sigma_rand_mv, tc) {
            // mV → µV, / (µV/K) → K, → mK.
            (Some(r), Some(tc)) if tc > 0.0 => ((self.eta * r * 1e3 / tc * 1e3) as i32).max(1),
            _ => THERMAL_MAX_DELTA_MC,
        }
    }
}

/// `d`'s entry of a deck `[nmos, pmos]` pair; `None` for a non-FET or a
/// missing entry.
fn by_polarity(nl: &Netlist, d: DeviceId, v: [Option<f32>; 2]) -> Option<f32> {
    match nl.devices[d.0 as usize].kind {
        DeviceKind::Nmos => v[0],
        DeviceKind::Pmos => v[1],
        _ => None,
    }
}

/// Deck `A_VT` for `d`'s polarity.
fn avt(nl: &Netlist, p: &ProcessNumbers, d: DeviceId) -> Option<f32> {
    by_polarity(nl, d, p.avt_mv_um)
}

/// Build the placement [`Requirements`] from the recognised blocks.
///
/// A `MatchingPair` is a priced budget only when the deck carries its
/// polarity's `A_VT` and the process `S_VT`; without them it is a pull whose
/// check reads unknown. `offset_sigma_mv` (1σ input-referred offset a pair
/// may spend) sets η; absent, [`GRADIENT_SHARE`].
#[must_use]
pub fn placement(
    blocks: &[Block],
    nl: &Netlist,
    p: &ProcessNumbers,
    offset_sigma_mv: Option<f32>,
) -> Requirements<Layout> {
    let dti_rule = p.dti;
    let mut r = Requirements::<Layout>::default();
    let mut dti = Vec::new();
    let td = Target::Device;

    for (bi, stage) in blocks.iter().enumerate() {
        let axis = AxisId(bi as u16);
        let pairs: Vec<(BlockKind, DeviceId, DeviceId)> = leaves(std::slice::from_ref(stage))
            .into_iter()
            .filter(|l| l.devices.len() == 2)
            .map(|l| (l.kind, l.devices[0], l.devices[1]))
            .collect();
        let (mut syms, mut a_side, mut b_side) = (Vec::new(), Vec::new(), Vec::new());

        for &(kind, a, b) in &pairs {
            let prox = vec![Proximity { a: td(a), b: td(b), max_distance_nm: PROXIMITY_NM }];
            match kind {
                BlockKind::DiffPair | BlockKind::CurrentMirror | BlockKind::Load => {}
                BlockKind::Stack => {
                    r.budget.push(Box::new(prox.clone()));
                    r.cost.push(Box::new(prox));
                    continue;
                }
                BlockKind::Group | BlockKind::Glue => continue,
            }
            syms.push(Symmetry { a: td(a), b: td(b), axis });
            let gate = gate_um2(nl, a).min(gate_um2(nl, b));
            let pel = Pelgrom::new(avt(nl, p, a), gate, p, offset_sigma_mv);
            let pair = vec![MatchingPair {
                a: td(a),
                b: td(b),
                gate_um2: gate,
                gradient_share: pel.eta,
                gradient_per_avt_um2: pel.s_over_a,
            }];
            if pel.s_over_a > 0.0 {
                r.budget.push(Box::new(pair.clone()));
            }
            r.cost.push(Box::new(pair));
            if kind == BlockKind::CurrentMirror {
                r.budget.push(Box::new(prox.clone()));
                r.cost.push(Box::new(prox));
            }
            let therm = vec![ThermalGradient {
                a: td(a),
                b: td(b),
                max_delta_mc: pel.thermal_limit_mc(by_polarity(nl, a, p.vt_tc_uv_per_k)),
                margin_pct: THERMAL_MARGIN_PCT,
            }];
            r.cost.push(Box::new(therm.clone()));
            r.budget.push(Box::new(therm));
            // Matched devices share one trench. Ids are dense in emission order,
            // which is deterministic, so a branch never transfers between pairs.
            if let Some((s_max_nm, d_dti_nm)) = dti_rule {
                dti.push(DtiBand {
                    a: td(a),
                    b: td(b),
                    s_max_nm,
                    d_dti_nm,
                    branch: BranchId(dti.len() as u16),
                    seed_isolate: false,
                });
            }
            a_side.push(a);
            b_side.push(b);
        }
        // Every stage mirrors its matched pairs (diff pair, mirror, load) about
        // its one axis (MAT-06; Lampaert 1999 §4.6–4.7 symmetry groups). A
        // differential stage also puts each member outside any pair (the
        // tail) on the axis, near the input pair.
        if let Some(dp) = pairs.iter().find(|p| p.0 == BlockKind::DiffPair) {
            let paired: Vec<DeviceId> = pairs.iter().flat_map(|p| [p.1, p.2]).collect();
            for &d in stage.devices.iter().filter(|d| !paired.contains(d)) {
                syms.push(Symmetry { a: td(d), b: td(d), axis });
                let tail = [dp.1, dp.2].map(|m| Proximity { a: td(d), b: td(m), max_distance_nm: PROXIMITY_NM }).to_vec();
                r.budget.push(Box::new(tail.clone()));
                r.cost.push(Box::new(tail));
            }
        }
        if !syms.is_empty() {
            r.cost.push(Box::new(SymmetryGroup(syms.clone())));
            r.hard.push(Box::new(SymmetryGroup(syms)));
        }
        if !a_side.is_empty() {
            // Tightest member: the largest gate and the smallest `A_VT` (any
            // member without one leaves the gradient half unknown).
            let members = || a_side.iter().chain(&b_side);
            let gate = members().map(|&d| gate_um2(nl, d)).fold(0.0, f32::max);
            let a_min = members().map(|&d| avt(nl, p, d)).try_fold(f32::INFINITY, |m, a| a.map(|a| m.min(a)));
            let pel = Pelgrom::new(a_min, gate, p, offset_sigma_mv);
            // LOD: the members' largest KVTH0 over the random σ.
            let kvth0 = members().filter_map(|&d| by_polarity(nl, d, p.lod_kvth0_mv_um)).fold(0.0, f32::max);
            let lod_per_sigma_um = pel.sigma_rand_mv.filter(|&s| s > 0.0).map_or(0.0, |s| kvth0 / s);
            let cc = CentroidGroup {
                a_side,
                b_side,
                gate_um2: gate,
                gradient_share: pel.eta,
                gradient_per_avt_um2: pel.s_over_a,
                lod_per_sigma_um,
                cell_of: Vec::new(),
            };
            // Always budgeted: coincidence in an interleaved array needs no deck data.
            r.budget.push(Box::new(cc.clone()));
            r.cost.push(Box::new(cc));
        }
    }

    if !dti.is_empty() {
        r.cost.push(Box::new(dti.clone()));
        r.hard.push(Box::new(dti));
    }
    r
}

/// Isolation saturates beyond this multiple of the epi thickness (Charbon et
/// al. 2001 ch.8, PDF p.127: 2.5–5×; Su et al. 4×): farther buys nothing.
const ISOLATION_EPI_MULTIPLE: i32 = 4;
/// ponytail: nominal epi when the deck has none, so the pull still acts;
/// the check then reads unknown. The decks' `p_epi_thickness` is not it: a
/// guard-ring depth default (3000 nm in all four decks, finfet included), and
/// sky130 is bulk p-substrate, not epi on p+, where Charbon's plateau does not
/// hold (isolation keeps improving with distance). Read `cell.epi_thickness_nm`.
const NOMINAL_EPI_NM: i32 = 2_500;

/// Substrate isolation (ENV-04; Charbon 2001 ch.2 injection → propagation →
/// reception): every device on a Clock-class net (an injector) is kept
/// `ISOLATION_EPI_MULTIPLE·t_epi` edge-to-edge from every `sensitive`
/// (matched) device. Budget + cost with the deck's epi thickness; without it a
/// cost-only pull at [`NOMINAL_EPI_NM`]. Returns whether any rule was emitted.
pub fn isolation(
    hg: &pnr_core::BipartiteHypergraph,
    classes: &[analog::metadata::NetClassification],
    sensitive: &[bool],
    epi_nm: Option<i32>,
    r: &mut Requirements<Layout>,
) -> bool {
    use analog::metadata::NetClass;
    let clocked = |d: usize| hg.device_nets[d].iter().any(|n| classes[n.0 as usize].class == NetClass::Clock);
    let n = hg.device_nets.len();
    let aggressors: Vec<usize> = (0..n).filter(|&d| clocked(d) && !sensitive[d]).collect();
    let min_distance_nm = ISOLATION_EPI_MULTIPLE * epi_nm.unwrap_or(NOMINAL_EPI_NM);
    let dev = |d: usize| Target::Device(DeviceId(d as u16));
    let rules: Vec<Isolation> = aggressors
        .iter()
        .flat_map(|&a| (0..n).filter(|&v| sensitive[v]).map(move |v| Isolation { a: dev(a), b: dev(v), min_distance_nm }))
        .collect();
    if rules.is_empty() {
        return false;
    }
    if epi_nm.is_some() {
        r.budget.push(Box::new(rules.clone()));
    }
    r.cost.push(Box::new(rules));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck() -> ProcessNumbers {
        ProcessNumbers { avt_mv_um: [Some(30.0), Some(35.0)], svt_uv_per_um: Some(4.0), vt_tc_uv_per_k: [Some(765.0), Some(1_850.0)], ..ProcessNumbers::default() }
    }

    #[test]
    fn pelgrom_numbers_come_from_the_deck_and_the_budget() {
        // Pelgrom 1988 Table 1: n-channel S/A = 4 µV/µm / 30 mV·µm ≈ 1.33e-4 /µm².
        let p = Pelgrom::new(Some(30.0), 100.0, &deck(), None);
        assert!((p.s_over_a - 1.333e-4).abs() < 1e-6);
        assert_eq!(p.eta, GRADIENT_SHARE, "no budget: the heuristic share");
        assert!((p.sigma_rand_mv.unwrap() - 3.0).abs() < 1e-6, "30/√100");

        // A 1σ budget of √1.09·σ_rand leaves the gradient exactly 0.3·σ_rand.
        let b = Pelgrom::new(Some(30.0), 100.0, &deck(), Some(3.0 * 1.09f32.sqrt()));
        assert!((b.eta - 0.3).abs() < 1e-3);
        // Sizing that already spends the budget leaves placement nothing.
        assert_eq!(Pelgrom::new(Some(30.0), 100.0, &deck(), Some(2.0)).eta, 0.0);

        // ΔT limit = η·σ_rand / TC, per polarity (sky130 ngspice TCs):
        // nfet 0.3·3 mV / 765 µV/K = 1.176 K; a pfet pair (A 35, 3.5 mV)
        // at 1850 µV/K = 0.567 K — the steeper TC buys less ΔT.
        let tc = deck().vt_tc_uv_per_k;
        assert_eq!(p.thermal_limit_mc(tc[0]), 1_176);
        assert_eq!(Pelgrom::new(Some(35.0), 100.0, &deck(), None).thermal_limit_mc(tc[1]), 567);
        let bare = ProcessNumbers::default();
        let q = Pelgrom::new(None, 100.0, &bare, None);
        assert_eq!((q.s_over_a, q.thermal_limit_mc(None)), (0.0, THERMAL_MAX_DELTA_MC), "fallbacks");
        assert_eq!(p.thermal_limit_mc(None), THERMAL_MAX_DELTA_MC, "A without TC: fallback");
    }
}
