//! Placement constraints from the recognised hierarchy.
//!
//! Each top-level block is a **stage** with axis `AxisId(block index)`. Its
//! 2-device leaves emit by kind (`a`, `b` = slot 0, slot 1):
//!
//! | kind          | emits                                                              |
//! |---------------|--------------------------------------------------------------------|
//! | DiffPair      | MatchingPair(Cross), ThermalGradient, centroid sides, DTI |
//! | CurrentMirror | MatchingPair(Mirror), Proximity, ThermalGradient, sides, DTI |
//! | Load          | MatchingPair(Mirror), ThermalGradient, sides, DTI          |
//! | Stack         | Proximity                                                 |
//!
//! A stage holding a diff pair is differential: every matched pair mirrors about
//! the stage axis (a pair merged into one cell centres on it), and each member
//! outside a pair (the tail) is self-symmetric with a Proximity pull to the
//! input pair.
//!
//! One batch per pair (per-batch criticality weights each pair by its own
//! urgency; one merged batch regressed the OTA). Arms: `SymmetryGroup` (one per stage) and `DtiBand` are hard + cost — the cost
//! copy is the gradient toward the hard set (and what prices dp's DTI branch
//! flip). `ThermalGradient` is budget + cost — its residual reads the
//! epoch-frozen temperature field, so the cost copy is the per-move pull.
//! `MatchingPair`, `Proximity`, `CentroidGroup` are cost only: placement owns
//! only the distance term of Pelgrom; area is the cell generator's.
//!
//! Scalars are representative defaults (no PDK handle here).

use analog::placement::cc::CentroidGroup;
use analog::placement::symmetry::SymmetryGroup;
use analog::placement::{DtiBand, Matching, MatchingPair, Proximity, Symmetry, ThermalGradient};
use analog::Requirements;
use pnr_core::ids::{AxisId, BranchId, DeviceId, Target};
use pnr_core::layout::Layout;
use pnr_core::{BipartiteHypergraph, DeviceKind, Netlist};

use crate::block::{leaves, Block, BlockKind};

const THERMAL_MAX_DELTA_MC: i32 = 500;
/// Held back from the thermal spec so a converged run lands inside it.
const THERMAL_MARGIN_PCT: u8 = 20;
const PROXIMITY_NM: i32 = 5_000;
const MAX_DVTH_MV10: i32 = 10;
// ponytail: one DTI band for the whole die; the real values are a PDK entry.
const DTI_S_MAX_NM: i32 = 200;
const DTI_D_DTI_NM: i32 = 2_000;

/// Pelgrom `A_Vth`, µV·µm (representative; real value is a PDK entry).
fn avt(kind: DeviceKind) -> i32 {
    if kind == DeviceKind::Pmos { 5000 } else { 4000 }
}

/// `(W·nf of a, W·nf of b)` reduced by their gcd; `(1, 1)` when unknown.
fn w_ratio(nl: &Netlist, a: DeviceId, b: DeviceId) -> (u16, u16) {
    let wn = |d: DeviceId| {
        let dev = &nl.devices[d.0 as usize];
        crate::param(dev, "w", 0) * crate::param(dev, "nf", 1).max(1)
    };
    let (x, y) = (wn(a), wn(b));
    if x <= 0 || y <= 0 {
        return (1, 1);
    }
    let g = gcd(x, y);
    let (x, y) = (x / g, y / g);
    if x > i64::from(u16::MAX) || y > i64::from(u16::MAX) { (1, 1) } else { (x as u16, y as u16) }
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Build the placement [`Requirements`] from the recognised blocks.
#[must_use]
pub fn placement(blocks: &[Block], hg: &BipartiteHypergraph, nl: &Netlist) -> Requirements<Layout> {
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
            let matching = match kind {
                BlockKind::DiffPair => Matching::Cross,
                BlockKind::CurrentMirror | BlockKind::Load => Matching::Mirror,
                BlockKind::Stack => {
                    r.cost.push(Box::new(prox));
                    continue;
                }
                BlockKind::Group | BlockKind::Glue => continue,
            };
            syms.push(Symmetry { a: td(a), b: td(b), axis });
            r.cost.push(Box::new(vec![MatchingPair {
                a: td(a),
                b: td(b),
                max_dvth_mv10: MAX_DVTH_MV10,
                w_ratio: w_ratio(nl, a, b),
                avt_uv_um: avt(hg.kinds[a.0 as usize]),
                matching,
            }]));
            if kind == BlockKind::CurrentMirror {
                r.cost.push(Box::new(prox));
            }
            let therm = vec![ThermalGradient {
                a: td(a),
                b: td(b),
                max_delta_mc: THERMAL_MAX_DELTA_MC,
                margin_pct: THERMAL_MARGIN_PCT,
            }];
            r.cost.push(Box::new(therm.clone()));
            r.budget.push(Box::new(therm));
            // Matched devices share one trench. Ids are dense in emission order,
            // which is deterministic, so a branch never transfers between pairs.
            dti.push(DtiBand {
                a: td(a),
                b: td(b),
                s_max_nm: DTI_S_MAX_NM,
                d_dti_nm: DTI_D_DTI_NM,
                branch: BranchId(dti.len() as u16),
                seed_isolate: false,
            });
            a_side.push(a);
            b_side.push(b);
        }
        // A differential stage mirrors every matched pair about its one axis; a
        // member outside any pair (the tail) sits on the axis, near the input pair.
        if let Some(dp) = pairs.iter().find(|p| p.0 == BlockKind::DiffPair) {
            let paired: Vec<DeviceId> = pairs.iter().flat_map(|p| [p.1, p.2]).collect();
            for &d in stage.devices.iter().filter(|d| !paired.contains(d)) {
                syms.push(Symmetry { a: td(d), b: td(d), axis });
                r.cost.push(Box::new(
                    [dp.1, dp.2].map(|m| Proximity { a: td(d), b: td(m), max_distance_nm: PROXIMITY_NM }).to_vec(),
                ));
            }
        } else {
            syms.clear();
        }
        if !syms.is_empty() {
            r.cost.push(Box::new(SymmetryGroup(syms.clone())));
            r.hard.push(Box::new(SymmetryGroup(syms)));
        }
        if !a_side.is_empty() {
            r.cost.push(Box::new(CentroidGroup { a_side, b_side }));
        }
    }

    if !dti.is_empty() {
        r.cost.push(Box::new(dti.clone()));
        r.hard.push(Box::new(dti));
    }
    r
}
