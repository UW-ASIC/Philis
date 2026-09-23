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
//! Matching budgets come from the netlist's gate areas; the remaining scalars
//! are tuning defaults (no PDK handle here).

use analog::placement::cc::CentroidGroup;
use analog::placement::symmetry::SymmetryGroup;
use analog::placement::{DtiBand, Matching, MatchingPair, Proximity, Symmetry, ThermalGradient};
use analog::Requirements;
use pnr_core::ids::{AxisId, BranchId, DeviceId, Target};
use pnr_core::layout::Layout;
use pnr_core::Netlist;

use crate::block::{leaves, Block, BlockKind};

const THERMAL_MAX_DELTA_MC: i32 = 500;
/// Held back from the thermal spec so a converged run lands inside it.
const THERMAL_MARGIN_PCT: u8 = 20;
const PROXIMITY_NM: i32 = 5_000;
/// Placement's share of a matched pair's mismatch: the gradient term may reach
/// this fraction of the random term the sizing bought (σ grows ≤ 4.4%).
const GRADIENT_SHARE: f32 = 0.3;
// ponytail: one DTI band for the whole die; the real values are a PDK entry.
const DTI_S_MAX_NM: i32 = 200;
const DTI_D_DTI_NM: i32 = 2_000;

/// Gate area `W·L·fingers` of `d`, µm² (`0` when the netlist omits W/L).
fn gate_um2(nl: &Netlist, d: DeviceId) -> f32 {
    let dev = &nl.devices[d.0 as usize];
    let (w, l) = (crate::param(dev, "w", 0) as f32, crate::param(dev, "l", 0) as f32);
    w * l * 1e-6 * f32::from(crate::constraints::fingers(dev))
}

/// Build the placement [`Requirements`] from the recognised blocks.
#[must_use]
pub fn placement(blocks: &[Block], nl: &Netlist) -> Requirements<Layout> {
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
                gate_um2: gate_um2(nl, a).min(gate_um2(nl, b)),
                gradient_share: GRADIENT_SHARE,
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
            let gate = a_side.iter().chain(&b_side).map(|&d| gate_um2(nl, d)).fold(0.0, f32::max);
            r.cost.push(Box::new(CentroidGroup { a_side, b_side, gate_um2: gate, gradient_share: GRADIENT_SHARE }));
        }
    }

    if !dti.is_empty() {
        r.cost.push(Box::new(dti.clone()));
        r.hard.push(Box::new(dti));
    }
    r
}
