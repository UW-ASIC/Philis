//! Constraint digestion: raw pnr-constraints types → placement/routing-ready views.
//!
//! Consumers (placement, routing) currently define their own `ConstraintRecord`.
//! This module provides a unified digest that both can consume, plus helpers to
//! extract constraints from the annotation/hypergraph context.

use pnr_constraints::{
    CcGroup, CrosstalkExclusion, DeviceId, IsolationConstraint, MatchingTier,
    NetClassification, ParasiticBudget, ProximityRule, StraightNet, SymmetryGroup,
    ThermalGradientConstraint,
};

/// Digested constraint set ready for backend consumption.
/// Same shape as placement's `ConstraintRecord` today — will evolve to add
/// routing-specific views (shielding, length-matching) and feedback hints.
#[derive(Clone, Default)]
pub struct DigestedConstraints {
    pub symmetry: Vec<SymmetryGroup>,
    pub cc: Vec<CcGroup>,
    pub proximity: Vec<ProximityRule>,
    pub isolation: Vec<IsolationConstraint>,
    pub thermal: Vec<ThermalGradientConstraint>,
    pub net_class: Vec<NetClassification>,
    pub crosstalk: Vec<CrosstalkExclusion>,
    pub straight: Vec<StraightNet>,
    pub parasitic: Vec<ParasiticBudget>,
}

impl DigestedConstraints {
    /// Highest matching tier assigned to `cell_idx` across all symmetry pairs.
    pub fn cell_tier(&self, cell_idx: u32) -> MatchingTier {
        let mut best = MatchingTier::None;
        for sg in &self.symmetry {
            for mp in &sg.pairs {
                if mp.device_a.0 == cell_idx || mp.device_b.0 == cell_idx {
                    if mp.tier > best {
                        best = mp.tier;
                    }
                }
            }
        }
        best
    }

    /// Remap all DeviceIds through an old→new index map.
    pub fn remap(&mut self, map: &[u32]) {
        let r = |id: DeviceId| -> DeviceId { DeviceId(map[id.0 as usize]) };

        for sg in &mut self.symmetry {
            sg.pairs.retain(|mp| r(mp.device_a).0 != r(mp.device_b).0);
            for mp in &mut sg.pairs {
                mp.device_a = r(mp.device_a);
                mp.device_b = r(mp.device_b);
            }
            sg.self_symmetric = sg.self_symmetric.iter().map(|&d| r(d)).collect();
            sg.self_symmetric.sort_unstable_by_key(|d| d.0);
            sg.self_symmetric.dedup_by_key(|d| d.0);
        }
        for cc in &mut self.cc {
            cc.group_a = cc.group_a.iter().map(|&d| r(d)).collect();
            cc.group_b = cc.group_b.iter().map(|&d| r(d)).collect();
        }
        for p in &mut self.proximity {
            p.device_a = r(p.device_a);
            p.device_b = r(p.device_b);
        }
        for iso in &mut self.isolation {
            iso.device_a = r(iso.device_a);
            iso.device_b = r(iso.device_b);
        }
    }
}
