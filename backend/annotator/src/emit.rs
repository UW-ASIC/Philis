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

use analog::matching::mismatch::{Budget, Coeffs, MatchKind};
use analog::placement::symmetry::SymmetryGroup;
use analog::placement::{DtiBand, Isolation, MatchedSet, OrientCheck, OrientationSet, Proximity, Symmetry};
use analog::Requirements;
use pnr_core::ids::{AxisId, BranchId, DeviceId, Target};
use pnr_core::layout::Layout;
use pnr_core::{DeviceKind, Netlist};

use crate::block::{leaves, Block, BlockKind};
use crate::ProcessNumbers;

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

/// The pair budget: the 1σ offset when given, else [`GRADIENT_SHARE`].
fn budget(offset_sigma_mv: Option<f32>) -> Budget {
    offset_sigma_mv.map_or(Budget::Eta(GRADIENT_SHARE), Budget::Sigma1Mv)
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
/// may spend) sets the allowance; absent, [`GRADIENT_SHARE`]`·σ_rand`.
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
        let mut syms = Vec::new();

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
            let set = MatchedSet {
                members: vec![a, b],
                kind: if kind == BlockKind::DiffPair { MatchKind::Voltage } else { MatchKind::Current },
                mos: matches!(nl.devices[a.0 as usize].kind, DeviceKind::Nmos | DeviceKind::Pmos),
                coeffs: Coeffs {
                    avt_mv_um: avt(nl, p, a),
                    svt_uv_per_um: p.svt_uv_per_um,
                    kvth0_mv_um: by_polarity(nl, a, p.lod_kvth0_mv_um),
                    tc_uv_per_k: by_polarity(nl, a, p.vt_tc_uv_per_k),
                },
                budget: budget(offset_sigma_mv),
                gate_um2: vec![gate_um2(nl, a), gate_um2(nl, b)],
                tol_nm: p.lattice_nm.max(1) as f32 / 2.0,
                cell_of: Vec::new(),
            };
            r.budget.push(Box::new(set.clone()));
            r.cost.push(Box::new(set));
            let orient = |check| OrientationSet { members: vec![a, b], check, cell_of: Vec::new() };
            r.hard.push(Box::new(orient(OrientCheck::Axis)));
            r.budget.push(Box::new(orient(OrientCheck::Phi)));
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
