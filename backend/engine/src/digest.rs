//! Constraint digestion: raw pnr-constraints types → placement/routing-ready views.
//!
//! Single digest consumed by placement, routing, and cell-gen. Each consumer
//! reads the fields it needs. `PlaceCold` does the domain-specific SoA transform.

use pnr_constraints::{
    AgingConstraint, AntennaConstraint, BiasCurrentTag, CcGroup, CrosstalkExclusion,
    CurrentFlowTag, DeviceId, DtiPair, DummyConstraint, EnvironmentalConstraint,
    EsdConstraint, GuardRingRequirement, IsolationConstraint, LdeBound, MatchingSpec,
    MatchingTier, NetClassification, ParasiticBudget, ProximityRule, StressConstraint,
    StraightNet, SymmetryGroup, ThermalGradientConstraint, UnitizationConstraint,
};

/// Digested constraint set ready for backend consumption.
#[derive(Clone, Default)]
pub struct DigestedConstraints {
    // -- placement-level --
    pub symmetry: Vec<SymmetryGroup>,
    pub cc: Vec<CcGroup>,
    pub proximity: Vec<ProximityRule>,
    pub isolation: Vec<IsolationConstraint>,
    pub thermal: Vec<ThermalGradientConstraint>,
    pub matching_spec: Vec<MatchingSpec>,
    pub dti: Vec<DtiPair>,
    pub stress: Vec<StressConstraint>,

    // -- routing-level --
    pub net_class: Vec<NetClassification>,
    pub crosstalk: Vec<CrosstalkExclusion>,
    pub straight: Vec<StraightNet>,
    pub parasitic: Vec<ParasiticBudget>,
    pub antenna: Vec<AntennaConstraint>,
    pub current_flow: Vec<CurrentFlowTag>,
    pub esd: Vec<EsdConstraint>,

    // -- cell-level --
    pub dummy: Vec<DummyConstraint>,
    pub guard_ring: Vec<GuardRingRequirement>,
    pub unitization: Vec<UnitizationConstraint>,
    pub lde: Vec<LdeBound>,
    pub aging: Vec<AgingConstraint>,
    pub environment: Vec<EnvironmentalConstraint>,

    // -- metadata --
    pub bias_current: Vec<BiasCurrentTag>,
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
        for d in &mut self.dti {
            d.device_a = r(d.device_a);
            d.device_b = r(d.device_b);
        }
        for s in &mut self.stress {
            s.device_id = r(s.device_id);
        }
        for d in &mut self.dummy {
            d.device_id = r(d.device_id);
        }
        for g in &mut self.guard_ring {
            g.device_id = r(g.device_id);
        }
        for u in &mut self.unitization {
            u.devices = u.devices.iter().map(|&d| r(d)).collect();
        }
        for l in &mut self.lde {
            l.pair = (r(l.pair.0), r(l.pair.1));
        }
        for a in &mut self.aging {
            a.device_id = r(a.device_id);
        }
        for e in &mut self.environment {
            e.scope = e.scope.iter().map(|&d| r(d)).collect();
        }
        for b in &mut self.bias_current {
            b.device_id = r(b.device_id);
        }
        for c in &mut self.current_flow {
            c.device_id = r(c.device_id);
        }
    }

    /// Read-only view of cell-level constraints for cell generators.
    pub fn cell_constraints(&self) -> CellConstraintView<'_> {
        CellConstraintView {
            dummy: &self.dummy,
            guard_ring: &self.guard_ring,
            unitization: &self.unitization,
            lde: &self.lde,
            aging: &self.aging,
            environment: &self.environment,
            stress: &self.stress,
        }
    }
}

/// Read-only view into cell-level constraints, passed to cell generators.
pub struct CellConstraintView<'a> {
    pub dummy: &'a [DummyConstraint],
    pub guard_ring: &'a [GuardRingRequirement],
    pub unitization: &'a [UnitizationConstraint],
    pub lde: &'a [LdeBound],
    pub aging: &'a [AgingConstraint],
    pub environment: &'a [EnvironmentalConstraint],
    pub stress: &'a [StressConstraint],
}
