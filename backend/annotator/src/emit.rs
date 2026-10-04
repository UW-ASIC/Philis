//! Placement constraints from the recognised hierarchy.
//!
//! Each top-level block is a **stage** with axis `AxisId(block index)`. Its
//! 2-device leaves emit by kind (`a`, `b` = slot 0, slot 1):
//!
//! | kind          | emits                                                    |
//! |---------------|----------------------------------------------------------|
//! | DiffPair      | Symmetry, MatchedSet (Voltage), Orientation, DTI         |
//! | CurrentMirror | Symmetry, MatchedSet (Current), Orientation, Proximity,  |
//! |               | DTI                                                      |
//! | Load          | Symmetry, MatchedSet (Current), Orientation, DTI         |
//! | CascodePair   | Symmetry, MatchedSet (Current), Orientation, DTI         |
//! | Stack         | Proximity                                                |
//!
//! Every matched pair mirrors about its stage axis (a pair merged into one cell
//! centres on it), except one sharing a device with an earlier pair's Symmetry
//! (a multi-output mirror's reference): it keeps MatchedSet, Orientation,
//! Proximity and DTI but no Symmetry. A stage holding a diff
//! pair is differential: each declared self (tail, shared bias) is also
//! self-symmetric with a Proximity pull to the input pair.
//!
//! One batch per pair (per-batch criticality weights each pair by its own
//! urgency; one merged batch regressed the OTA). Arms: `SymmetryGroup` (one per stage) and `DtiBand` are hard + cost — the cost
//! copy is the gradient toward the hard set (and what prices dp's DTI branch
//! flip). `MatchedSet` (one pair's gradient, thermal and LOD terms against one
//! allowance, plus coincidence when its unit counts admit a centroid row) and
//! `Proximity` (MAT-07, a distance allowance) are budget + cost; a
//! `MatchedSet` pair without deck data or units reads unknown and only pulls.
//! Placement owns the systematic terms of Pelgrom; area is the cell generator's.
//! `Orientation` (MAT-05) is Axis hard (a quarter-turned partner is illegal)
//! and Φ budget only: Φ changes by a discrete flip, so a cost copy has no
//! gradient to give; Θ prices it.
//!
//! Matching and thermal budgets come from the netlist's gate areas and the
//! deck's mismatch data; without them the documented fallbacks apply.

use analog::matching::class::{phi_arm, Family, MatchClass};
use analog::matching::mismatch::{self, Budget, Coeffs, MatchKind};
use analog::placement::symmetry::SymmetryGroup;
use analog::placement::{DtiBand, Isolation, MatchedSet, OrientCheck, OrientationSet, Proximity, Symmetry};
use analog::Requirements;
use pnr_core::ids::{AxisId, BranchId, DeviceId, Target};
use pnr_core::layout::Layout;
use pnr_core::{DeviceKind, Netlist};

use crate::block::{leaves, Block, BlockKind};
use crate::ProcessNumbers;

fn gate_um2(nl: &Netlist, d: DeviceId) -> f32 {
    crate::gate_um2(&nl.devices[d.0 as usize])
}

/// The pair budget: the 1σ offset when given, else
/// [`mismatch::GRADIENT_SHARE`] (no allowance or class source yet: EXT-20).
fn budget(offset_sigma_mv: Option<f32>, kind: MatchKind) -> Budget {
    mismatch::choose(offset_sigma_mv, None, None, kind)
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
/// A `MatchedSet` pair is priced against its allowance when the deck carries
/// its polarity's `A_VT`; the terms whose coefficient is missing (`S_VT`, TC,
/// `KVTH0`) spend nothing. `offset_sigma_mv` (1σ input-referred offset a pair
/// may spend) sets the allowance; absent, [`mismatch::GRADIENT_SHARE`]`·σ_rand`.
#[must_use]
pub fn placement(
    blocks: &[Block],
    nl: &Netlist,
    p: &ProcessNumbers,
    offset_sigma_mv: Option<f32>,
    policy: &crate::policy::Policy,
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
        let mut syms = Vec::new();
        let mut in_sym: Vec<DeviceId> = Vec::new();

        for &(kind, a, b) in &pairs {
            let prox = vec![Proximity { a: td(a), b: td(b), max_distance_nm: policy.proximity_nm }];
            match kind {
                BlockKind::DiffPair | BlockKind::CurrentMirror | BlockKind::Load | BlockKind::CascodePair => {}
                BlockKind::Stack => {
                    r.budget.push(Box::new(prox.clone()));
                    r.cost.push(Box::new(prox));
                    continue;
                }
                BlockKind::Group | BlockKind::Glue => continue,
            }
            // Only an inductor has no family, and an inductor pair is never matched; checked before
            // the Symmetry push so a familyless pair gets no batch at all.
            let Some(family) = Family::of(nl.devices[a.0 as usize].kind) else { continue };
            // ponytail: pairwise emission; a multi-output mirror's pairs share their reference.
            let shared = in_sym.contains(&a) || in_sym.contains(&b);
            if !shared {
                syms.push(Symmetry { a: td(a), b: td(b), axis });
                in_sym.extend([a, b]);
            }
            let match_kind = if kind == BlockKind::DiffPair { MatchKind::Voltage } else { MatchKind::Current };
            let set = MatchedSet {
                members: vec![a, b],
                kind: match_kind,
                family,
                // ponytail: every set Moderate; EXT-20 sets it from intent::MatchedSet.
                class: MatchClass::Moderate,
                coeffs: Coeffs {
                    avt_mv_um: avt(nl, p, a),
                    svt_uv_per_um: p.svt_uv_per_um,
                    kvth0_mv_um: by_polarity(nl, a, p.lod_kvth0_mv_um),
                    tc_uv_per_k: by_polarity(nl, a, p.vt_tc_uv_per_k),
                },
                budget: budget(offset_sigma_mv, match_kind),
                gate_um2: vec![gate_um2(nl, a), gate_um2(nl, b)],
                tol_nm: p.lattice_nm.max(1) as f32 / 2.0,
                cell_of: Vec::new(),
            };
            let phi = phi_arm(set.class);
            r.budget.push(Box::new(set.clone()));
            r.cost.push(Box::new(set));
            let orient = |check| OrientationSet { members: vec![a, b], check, cell_of: Vec::new() };
            r.hard.push(Box::new(orient(OrientCheck::Axis)));
            match phi {
                Some(true) => r.hard.push(Box::new(orient(OrientCheck::Phi))),
                Some(false) => r.budget.push(Box::new(orient(OrientCheck::Phi))),
                None => {}
            }
            if kind == BlockKind::CurrentMirror {
                r.budget.push(Box::new(prox.clone()));
                r.cost.push(Box::new(prox));
            }
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
        }
        // Every stage mirrors its matched pairs (diff pair, mirror, load) about
        // its one axis (MAT-06; Lampaert 1999 §4.6–4.7 symmetry groups). A
        // differential stage also puts each declared self (tail, shared bias)
        // on the axis, near the input pair.
        if let Some(dp) = pairs.iter().find(|p| p.0 == BlockKind::DiffPair) {
            for &d in &stage.selfs {
                syms.push(Symmetry { a: td(d), b: td(d), axis });
                let tail = [dp.1, dp.2].map(|m| Proximity { a: td(d), b: td(m), max_distance_nm: policy.proximity_nm }).to_vec();
                r.budget.push(Box::new(tail.clone()));
                r.cost.push(Box::new(tail));
            }
        }
        if !syms.is_empty() {
            r.cost.push(Box::new(SymmetryGroup(syms.clone())));
            r.hard.push(Box::new(SymmetryGroup(syms)));
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
/// ponytail: nominal epi when the distance is uncalibrated, so the pull still
/// acts; the check then reads unknown. The decks' `p_epi_thickness` is not it:
/// a guard-ring depth default (3000 nm in all four decks, finfet included),
/// and sky130 is bulk p-substrate, not epi on p+, where Charbon's plateau does
/// not hold (isolation keeps improving with distance; SUB-32).
const NOMINAL_EPI_NM: i32 = 2_500;

/// Substrate isolation (ENV-04; Charbon 2001 ch.2 injection → propagation →
/// reception): every device on a Clock-class net (an injector) is kept
/// `ISOLATION_EPI_MULTIPLE·t_epi` edge-to-edge from every `sensitive`
/// (matched) device, except where `same_group(aggressor, victim)`: a stage's
/// own clocked tail sits by its pair by design (AA-13), so isolating them
/// contradicts the stage's Proximity.
///
/// Budget + cost only on `EpiOnLowRes` with `epi_nm` known (the plateau
/// distance). Otherwise a cost-only pull at [`NOMINAL_EPI_NM`], and the
/// returned `Some(why)` is the `missing` input that leaves it unknown: bulk has
/// no plateau distance. `None` when calibrated or nothing was emitted.
pub fn isolation(
    hg: &pnr_core::BipartiteHypergraph,
    classes: &[analog::metadata::NetClassification],
    sensitive: &[bool],
    same_group: &dyn Fn(usize, usize) -> bool,
    kind: pnr_core::SubstrateKind,
    epi_nm: Option<i32>,
    r: &mut Requirements<Layout>,
) -> Option<&'static str> {
    use analog::metadata::NetClass;
    use pnr_core::SubstrateKind;
    let clocked = |d: usize| hg.device_nets[d].iter().any(|n| classes[n.0 as usize].class == NetClass::Clock);
    let n = hg.device_nets.len();
    let calibrated = match (kind, epi_nm) {
        (SubstrateKind::EpiOnLowRes, Some(epi)) => Ok(epi),
        (SubstrateKind::EpiOnLowRes, None) => Err("deck epi_thickness_nm"),
        (SubstrateKind::Bulk, _) => Err("bulk substrate: no plateau distance"),
        (SubstrateKind::Unknown, _) => Err("substrate kind unknown"),
    };
    let min_distance_nm = ISOLATION_EPI_MULTIPLE * calibrated.unwrap_or(NOMINAL_EPI_NM);
    let dev = |d: usize| Target::Device(DeviceId(d as u16));
    let rules: Vec<Isolation> = (0..n)
        .filter(|&a| clocked(a) && !sensitive[a])
        .flat_map(|a| {
            (0..n).filter(move |&v| sensitive[v] && !same_group(a, v)).map(move |v| Isolation { a: dev(a), b: dev(v), min_distance_nm })
        })
        .collect();
    if rules.is_empty() {
        return None;
    }
    if calibrated.is_ok() {
        r.budget.push(Box::new(rules.clone()));
    }
    r.cost.push(Box::new(rules));
    calibrated.err()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// StrongARM-like stage: input pair `mn1`/`mn2` on a tail node, `mn0` the
    /// tail on `clk`, PMOS loads on `clk` (the StrongARM's precharge pair); plus `XS`, a lone clocked
    /// switch outside every block. Nets: 0=outp 1=inp 2=tail 3=VSS 4=outn
    /// 5=inn 6=clk 7=VDD 8=sw 9=sw2.
    fn strongarm_like() -> (Netlist, crate::AnnotationConfig) {
        use crate::tests::{fet, nets};
        let nl = Netlist {
            devices: vec![
                fet("mn1", DeviceKind::Nmos, 1, 0, 2, 3, 10_000, 1_000),
                fet("mn2", DeviceKind::Nmos, 5, 4, 2, 3, 10_000, 1_000),
                fet("mp3", DeviceKind::Pmos, 6, 0, 7, 7, 20_000, 1_000),
                fet("mp4", DeviceKind::Pmos, 6, 4, 7, 7, 20_000, 1_000),
                fet("mn0", DeviceKind::Nmos, 6, 2, 3, 3, 40_000, 1_000),
                fet("XS", DeviceKind::Nmos, 6, 8, 9, 3, 1_000, 150),
            ],
            nets: nets(&["outp", "inp", "tail", "VSS", "outn", "inn", "clk", "VDD", "sw", "sw2"]),
            ..Default::default()
        };
        let cfg = crate::AnnotationConfig {
            supply_nets: vec!["VDD".into()],
            ground_nets: vec!["VSS".into()],
            clock_nets: vec!["clk".into()],
            ..crate::AnnotationConfig::default()
        };
        (nl, cfg)
    }

    type Arm = Vec<Box<dyn analog::RuleBatch<Layout>>>;

    /// Device id pairs of every `Isolation` batch in `arm`.
    fn isolated(arm: &Arm) -> Vec<(u32, u32)> {
        let mut ids = Vec::new();
        arm.iter().filter(|b| b.kind().ends_with("::Isolation")).for_each(|b| b.touched(&mut ids));
        ids.chunks(2).map(|p| (p[0], p[1])).collect()
    }

    /// Point devices all at the origin except `XS` (id 5) at `x`: every edge
    /// gap from `XS` is `x`.
    fn xs_at(x: i32) -> Layout {
        let n = 6;
        Layout {
            x: (0..n).map(|d| if d == 5 { x } else { 0 }).collect(),
            y: vec![0; n],
            hw: vec![0; n],
            hh: vec![0; n],
            axis: vec![0; 8],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); n],
            variant: vec![0; n],
            branch: Vec::new(),
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    /// The smallest `XS` distance (nm, 1 nm resolution) at which `arm`'s
    /// Isolation batches are all satisfied.
    fn isolation_distance(arm: &Arm) -> i32 {
        let ok = |x| arm.iter().filter(|b| b.kind().ends_with("::Isolation")).all(|b| b.violations(&xs_at(x)) == 0);
        let (mut lo, mut hi) = (0, 1_000_000);
        assert!(ok(hi) && !ok(lo));
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if ok(mid) { hi = mid } else { lo = mid }
        }
        hi
    }

    /// MAT-07: every set is Moderate, so each matched pair keeps Axis hard and
    /// Φ as a budget; nothing is lost or moved between arms.
    #[test]
    fn default_class_is_moderate_mos() {
        let nl = crate::tests::ota();
        let cfg = crate::AnnotationConfig::default();
        let blocks = crate::annotate(&nl, &cfg).blocks;
        let r = placement(&blocks, &nl, &cfg.process, None, &cfg.policy);
        let count = |arm: &Arm, k: &str| arm.iter().filter(|b| b.kind() == k).count();
        let pairs = count(&r.budget, "MatchedSet");
        assert!(pairs > 0);
        assert_eq!(count(&r.hard, "Orientation"), pairs, "Axis hard");
        assert_eq!(count(&r.budget, "Orientation"), pairs, "Phi budget");
    }

    /// REL C3 / AA-13: the stage's clocked tail sits by its pair (Proximity);
    /// isolating it from the pair would contradict that. A clocked device
    /// outside the stage is still isolated from the pair.
    #[test]
    fn a_clocked_tail_is_not_isolated_from_its_own_pair() {
        let (nl, cfg) = strongarm_like();
        let p = crate::annotate(&nl, &cfg);
        let (mn1, mn2, mn0, xs) = (0u32, 1, 4, 5);
        let block = |d: u32| p.blocks.iter().position(|b| b.kind != BlockKind::Glue && b.devices.contains(&DeviceId(d as u16)));
        assert!(block(mn0).is_some() && block(mn0) == block(mn1), "the tail is in the pair's stage");
        let iso = isolated(&p.placement.cost);
        assert!(!iso.iter().any(|&(a, v)| a == mn0 && (v == mn1 || v == mn2)), "tail isolated from its pair: {iso:?}");
        assert!(iso.contains(&(xs, mn1)) && iso.contains(&(xs, mn2)), "an outside aggressor still is: {iso:?}");
    }

    /// REL C3: epi on p+ saturates at 4·t_epi (Su), a budget the search pays.
    #[test]
    fn epi_decks_get_a_plateau_budget() {
        let (nl, mut cfg) = strongarm_like();
        cfg.process.substrate = pnr_core::SubstrateKind::EpiOnLowRes;
        cfg.process.epi_nm = Some(10_000);
        let p = crate::annotate(&nl, &cfg);
        assert!(!isolated(&p.placement.budget).is_empty(), "in the budget arm");
        assert_eq!(isolation_distance(&p.placement.budget), 40_000, "min_distance_nm");
        assert!(!p.missing.iter().any(|m| m.0 == "Isolation"), "{:?}", p.missing);
    }

    /// Bulk (sky130's `substrate_kind`) has no plateau distance: a cost-only
    /// pull at 4·2500 nm, reported unknown, whatever epi the deck states.
    #[test]
    fn bulk_is_cost_only_and_unknown() {
        let (nl, mut cfg) = strongarm_like();
        assert_eq!(pnr_core::SubstrateKind::from_key(Some("epi_on_pplus")), pnr_core::SubstrateKind::EpiOnLowRes);
        cfg.process.substrate = pnr_core::SubstrateKind::from_key(Some("bulk"));
        cfg.process.epi_nm = Some(10_000);
        let p = crate::annotate(&nl, &cfg);
        assert!(isolated(&p.placement.budget).is_empty(), "not a budget");
        assert!(!isolated(&p.placement.cost).is_empty(), "a pull");
        assert_eq!(isolation_distance(&p.placement.cost), 4 * NOMINAL_EPI_NM);
        assert!(p.missing.contains(&("Isolation", "bulk substrate: no plateau distance")), "{:?}", p.missing);

        cfg.process.substrate = pnr_core::SubstrateKind::Unknown;
        let p = crate::annotate(&nl, &cfg);
        assert!(p.missing.contains(&("Isolation", "substrate kind unknown")));
        assert!(isolated(&p.placement.budget).is_empty());
    }
}
