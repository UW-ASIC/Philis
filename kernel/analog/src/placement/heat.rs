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
