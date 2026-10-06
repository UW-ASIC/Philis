//! Thermal separation of matched sets from power sources (placement tier, PLC-14).

use crate::rule::Rule;
use pnr_core::ids::{DeviceId, Target};
use pnr_core::layout::Layout;
use pnr_core::MatchClass;

/// Hastings rule 14 (L42569–42589): ≥ 1 µm per mW edge-to-edge between a
/// power source and a matched set. A separate type from
/// [`super::Isolation`] so it keeps its own report row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeatSeparation {
    /// Cell of the matched set being protected.
    pub victim: Target,
    /// Dissipating cell.
    pub source: Target,
    /// Required edge-to-edge gap, nm; `≤ 0` is always satisfied.
    pub min_gap_nm: i32,
}

impl Rule for HeatSeparation {
    type On = Layout;
    /// `((min − gap)⁺ / min)²` (PLC-18: dimensionless).
    fn cost(self, l: &Layout) -> f32 {
        let m = self.min_gap_nm.max(1) as f32;
        let s = (m - l.edge_gap(self.victim, self.source)).max(0.0) / m;
        s * s
    }
    fn satisfied(self, l: &Layout) -> bool {
        l.edge_gap(self.victim, self.source) >= self.min_gap_nm as f32
    }
    /// Linear shortfall / required gap (squares do not sum across rules).
    fn residual(self, l: &Layout) -> f32 {
        let floor = self.min_gap_nm as f32;
        crate::rule::over(floor - l.edge_gap(self.victim, self.source), floor)
    }
    fn touches(self, out: &mut Vec<u32>) {
        super::push_devices(out, &[self.victim, self.source]);
    }
    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { victim: self.victim.retarget(cell_of), source: self.source.retarget(cell_of), ..self }
    }
}

/// Returns one rule per distinct (victim cell, source cell) with
/// `power_uw[source] ≥ source_uw`, the source outside the victim's set and the
/// set's class not Minimal; `min_gap_nm = power_uw[source]` (k = 1 µm/mW =
/// 1 nm/µW for Moderate and Exceptional; Exceptional is policy).
///
/// `sets` are cell-indexed (a set merged into one cell is one victim);
/// `power_uw` is indexed by cell, so it must hold at most `u16::MAX + 1` cells.
/// Cost: O(Σ |set| · cells).
#[must_use]
pub fn separations(sets: &[(MatchClass, Vec<u16>)], power_uw: &[i32], source_uw: i32) -> Vec<HeatSeparation> {
    let mut out = Vec::new();
    for (_, cells) in sets.iter().filter(|(c, _)| *c != MatchClass::Minimal) {
        let mut victims = cells.clone();
        victims.sort_unstable();
        victims.dedup();
        for (s, &p) in power_uw.iter().enumerate() {
            if p < source_uw || victims.binary_search(&(s as u16)).is_ok() {
                continue;
            }
            for &v in &victims {
                out.push(HeatSeparation { victim: Target::Device(DeviceId(v)), source: Target::Device(DeviceId(s as u16)), min_gap_nm: p });
            }
        }
    }
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PLC-10: dp re-scores a batch only when a touched cell moves.
    #[test]
    fn touched_names_both_targets() {
        use crate::rule::RuleBatch;
        let sep = |victim, source| HeatSeparation { victim, source, min_gap_nm: 1_000 };
        let (d, g) = (|i| Target::Device(DeviceId(i)), Target::Group(pnr_core::GroupId(0)));
        let mut ids = Vec::new();
        vec![sep(d(4), d(1)), sep(g, g)].touched(&mut ids);
        assert_eq!(ids, [4, 1]);
    }

    #[test]
    fn heat_separation_scales_with_power() {
        let set = |c| [(c, vec![0u16, 1])];
        let r = separations(&set(MatchClass::Moderate), &[0, 0, 2000], 1000);
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|h| h.min_gap_nm == 2000 && h.source == Target::Device(DeviceId(2))));
        assert!(separations(&set(MatchClass::Minimal), &[0, 0, 2000], 1000).is_empty());
        assert!(separations(&set(MatchClass::Moderate), &[0, 0, 999], 1000).is_empty());
        assert!(separations(&set(MatchClass::Moderate), &[2000, 0, 0], 1000).is_empty(), "source inside the set");
    }

    #[test]
    fn satisfied_iff_gap_reaches_min() {
        // Two 2 µm boxes on one row; gap = centre distance − 2000.
        let at = |gap: i32| Layout {
            x: vec![0, 2000 + gap],
            y: vec![0, 0],
            hw: vec![1000; 2],
            hh: vec![1000; 2],
            axis: vec![],
            groups: vec![],
            orient: vec![Default::default(); 2],
            variant: vec![0; 2],
            branch: Vec::new(),
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
            units: Default::default(),
        };
        let h = HeatSeparation { victim: Target::Device(DeviceId(0)), source: Target::Device(DeviceId(1)), min_gap_nm: 2000 };
        assert!(!h.satisfied(&at(1999)));
        assert!(h.residual(&at(1999)) > 0.0);
        assert!(h.satisfied(&at(2000)));
        assert_eq!(h.residual(&at(2000)), 0.0);
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::rule::RuleBatch;

    fn d(i: u16) -> Target {
        Target::Device(DeviceId(i))
    }

    /// Two 2 µm boxes on one row, edge gap `gap` nm.
    fn at(gap: i32) -> Layout {
        Layout {
            x: vec![0, 2_000 + gap],
            y: vec![0, 0],
            hw: vec![1_000; 2],
            hh: vec![1_000; 2],
            axis: vec![],
            groups: vec![],
            orient: vec![Default::default(); 2],
            variant: vec![0; 2],
            branch: Vec::new(),
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
            units: Default::default(),
        }
    }

    #[test]
    fn cost_is_the_squared_relative_shortfall() {
        let h = HeatSeparation { victim: d(0), source: d(1), min_gap_nm: 2_000 };
        assert!((h.cost(&at(0)) - 1.0).abs() < 1e-6, "touching: the whole gap missing");
        assert!((h.cost(&at(1_000)) - 0.25).abs() < 1e-6);
        assert!((h.residual(&at(1_000)) - 0.5).abs() < 1e-6, "residual is linear");
        assert_eq!(h.cost(&at(2_000)), 0.0);
        assert_eq!(h.cost(&at(50_000)), 0.0);
    }

    #[test]
    fn a_zero_or_negative_gap_requirement_costs_nothing() {
        // Satisfied at any gap, so the pull must be zero too: a rule with no
        // requirement must not push touching cells apart.
        for min in [0, -5] {
            let h = HeatSeparation { victim: d(0), source: d(1), min_gap_nm: min };
            assert!(h.satisfied(&at(0)));
            assert_eq!(h.residual(&at(0)), 0.0);
            assert_eq!(h.cost(&at(0)), 0.0, "min {min}");
        }
    }

    #[test]
    fn separations_is_empty_without_sets_or_sources() {
        assert!(separations(&[], &[5_000, 5_000], 1_000).is_empty());
        assert!(separations(&[(MatchClass::Moderate, vec![0, 1])], &[], 1_000).is_empty());
        assert!(separations(&[(MatchClass::Moderate, vec![])], &[5_000], 1_000).is_empty());
    }

    #[test]
    fn a_source_exactly_at_the_threshold_counts() {
        let r = separations(&[(MatchClass::Exceptional, vec![0])], &[0, 1_000], 1_000);
        assert_eq!(r, [HeatSeparation { victim: d(0), source: d(1), min_gap_nm: 1_000 }]);
    }

    #[test]
    fn a_repeated_cell_in_one_set_is_one_victim() {
        let r = separations(&[(MatchClass::Moderate, vec![1, 0, 1, 0])], &[0, 0, 3_000], 1_000);
        assert_eq!(r.len(), 2, "{r:?}");
    }

    #[test]
    fn overlapping_sets_emit_each_victim_source_pair_once() {
        // Sets {0,1} and {1,2} share cell 1; sources 3 and 4 heat both.
        let sets = [(MatchClass::Moderate, vec![0, 1]), (MatchClass::Moderate, vec![1, 2])];
        let r = separations(&sets, &[0, 0, 0, 5_000, 6_000], 1_000);
        let distinct: std::collections::HashSet<(Target, Target)> = r.iter().map(|h| (h.victim, h.source)).collect();
        assert_eq!(r.len(), distinct.len(), "duplicate rules double-charge one pair: {r:?}");
        assert_eq!(r.len(), 6, "victims {{0,1,2}} × sources {{3,4}}");
    }

    #[test]
    fn retarget_maps_both_sides() {
        let h = HeatSeparation { victim: d(0), source: d(2), min_gap_nm: 7 }.retarget(&[4, 4, 9]);
        assert_eq!(h, HeatSeparation { victim: d(4), source: d(9), min_gap_nm: 7 });
    }

    #[test]
    fn a_batch_counts_violations_at_the_boundary() {
        let h = HeatSeparation { victim: d(0), source: d(1), min_gap_nm: 2_000 };
        assert_eq!(vec![h].violations(&at(1_999)), 1);
        assert_eq!(vec![h].violations(&at(2_000)), 0);
    }
}
