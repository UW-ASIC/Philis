//! Shielding of a sensitive route by a reference net (routing tier, budget).

use pnr_core::geom::Rect;
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use crate::rule::Rule;

/// `victim`'s routed length runs between `reference` metal on **both** sides
/// (same layer, parallel, within `max_gap_nm`) for at least
/// `min_coverage_pct` of it. The shield is a routed part of `reference`, so
/// its tie to the reference is ordinary connectivity (LVS checks the open);
/// `reference` should be a quiet, low-impedance net — a shield on a noisy
/// return couples that noise in.
///
/// A budget, not hard: a shield adds load to the victim, so it trades against
/// the victim's capacitance budget.
///
/// ponytail: one-sided and same-layer only (no top/bottom plates); tie
/// impedance is not measured.
#[derive(Clone, Copy)]
pub struct Shield {
    pub victim: NetId,
    pub reference: NetId,
    pub min_coverage_pct: u8,
    /// Farthest a shield wire may sit from the victim's edge, nm.
    pub max_gap_nm: i32,
}

impl Shield {
    /// Shielded fraction of the victim's wire length, `0..=1`; `None` when the
    /// victim has no wire.
    fn coverage(self, r: &Routes) -> Option<f32> {
        let refs = r.shapes(self.reference);
        let (mut total, mut covered) = (0i64, 0i64);
        for v in r.shapes(self.victim).iter().filter(|s| s.rect.w != s.rect.h) {
            let horiz = v.rect.w > v.rect.h;
            let (lo, hi) = if horiz { (v.rect.x, v.rect.x + v.rect.w) } else { (v.rect.y, v.rect.y + v.rect.h) };
            total += i64::from(hi - lo);
            // The run's stretches shadowed by reference metal on one side:
            // merged, clipped to [lo, hi), sorted.
            let side = |below: bool| -> Vec<(i32, i32)> {
                let mut spans: Vec<(i32, i32)> = refs
                    .iter()
                    .filter(|s| s.layer == v.layer && gap_on_side(v.rect, s.rect, horiz, below).is_some_and(|g| g <= self.max_gap_nm))
                    .map(|s| if horiz { (s.rect.x, s.rect.x + s.rect.w) } else { (s.rect.y, s.rect.y + s.rect.h) })
                    .map(|(a, b)| (a.max(lo), b.min(hi)))
                    .filter(|(a, b)| a < b)
                    .collect();
                spans.sort_unstable();
                let mut merged: Vec<(i32, i32)> = Vec::with_capacity(spans.len());
                for (a, b) in spans {
                    match merged.last_mut() {
                        Some(last) if a <= last.1 => last.1 = last.1.max(b),
                        _ => merged.push((a, b)),
                    }
                }
                merged
            };
            // Shielded = reference on both sides at the same point, not the
            // shorter of the two side lengths.
            covered += intersection_len(&side(true), &side(false));
        }
        (total > 0).then(|| covered as f32 / total as f32)
    }
}

/// Total length common to two sorted, disjoint interval lists; O(a + b).
pub(crate) fn intersection_len(p: &[(i32, i32)], q: &[(i32, i32)]) -> i64 {
    let (mut i, mut j, mut len) = (0, 0, 0i64);
    while i < p.len() && j < q.len() {
        let (a, b) = (p[i].0.max(q[j].0), p[i].1.min(q[j].1));
        len += i64::from((b - a).max(0));
        if p[i].1 < q[j].1 { i += 1 } else { j += 1 }
    }
    len
}

/// Edge gap from `v` to `s` when `s` lies wholly on one side of `v` across its
/// run (`below` = lower y for a horizontal run, lower x for a vertical one).
fn gap_on_side(v: Rect, s: Rect, horiz: bool, below: bool) -> Option<i32> {
    let (v0, v1, s0, s1) = if horiz { (v.y, v.y + v.h, s.y, s.y + s.h) } else { (v.x, v.x + v.w, s.x, s.x + s.w) };
    if below {
        (s1 <= v0).then_some(v0 - s1)
    } else {
        (s0 >= v1).then_some(s0 - v1)
    }
}

impl Rule for Shield {
    type On = Routes;
    const REPAIR: crate::RepairKind = crate::RepairKind::Shield;
    fn cost(self, r: &Routes) -> f32 {
        self.residual(r)
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.coverage(r).is_none_or(|c| c * 100.0 >= f32::from(self.min_coverage_pct))
    }
    fn known(self, r: &Routes) -> bool {
        self.coverage(r).is_some()
    }
    fn residual(self, r: &Routes) -> f32 {
        let want = f32::from(self.min_coverage_pct) / 100.0;
        self.coverage(r).map_or(0.0, |c| crate::rule::over(want - c, want))
    }
    fn usage(self, r: &Routes) -> Option<f32> {
        self.coverage(r)
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.victim.0));
        out.push(u32::from(self.reference.0));
    }
    fn shield(self) -> Option<(u32, u32)> {
        Some((u32::from(self.victim.0), u32::from(self.reference.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::geom::{LayerId, Shape};

    fn h(y: i32, x: i32, w: i32) -> Shape {
        Shape { layer: LayerId(1), rect: Rect { x, y, w, h: 140 } }
    }
    fn rule() -> Shield {
        Shield { victim: NetId(0), reference: NetId(1), min_coverage_pct: 80, max_gap_nm: 200 }
    }

    #[test]
    fn coverage_needs_both_sides_along_the_run() {
        // Victim 10 µm at y=1000; reference tracks 140 nm away above and below.
        let full = Routes { wires: vec![vec![h(1_000, 0, 10_000)], vec![h(720, 0, 10_000), h(1_280, 0, 10_000)]], ..Default::default()  };
        assert!((rule().coverage(&full).unwrap() - 1.0).abs() < 1e-6);
        assert!(rule().satisfied(&full));
        // One side only: not a shield.
        let one = Routes { wires: vec![vec![h(1_000, 0, 10_000)], vec![h(720, 0, 10_000)]], ..Default::default()  };
        assert_eq!(rule().coverage(&one), Some(0.0));
        assert!(!rule().satisfied(&one));
        // Both sides over half the run: 50%, residual (0.8 − 0.5)/0.8.
        let half = Routes { wires: vec![vec![h(1_000, 0, 10_000)], vec![h(720, 0, 5_000), h(1_280, 0, 5_000)]], ..Default::default()  };
        assert!((rule().residual(&half) - 0.375).abs() < 1e-5);
        // Too far to shield.
        let far = Routes { wires: vec![vec![h(1_000, 0, 10_000)], vec![h(400, 0, 10_000), h(1_600, 0, 10_000)]], ..Default::default()  };
        assert_eq!(rule().coverage(&far), Some(0.0));
        // Unrouted victim: unknown.
        assert!(!rule().known(&Routes { wires: vec![vec![], vec![]], ..Default::default()  }));
    }

    #[test]
    fn complementary_halves_cover_nothing() {
        // Below on [0, 5] µm, above on [5, 10] µm: no point has both sides.
        let r = Routes { wires: vec![vec![h(1_000, 0, 10_000)], vec![h(720, 0, 5_000), h(1_280, 5_000, 5_000)]], ..Default::default() };
        assert_eq!(rule().coverage(&r), Some(0.0));
    }

    #[test]
    fn overlapping_halves_cover_the_overlap() {
        // Below on [0, 7] µm, above on [3, 10] µm: both sides over [3, 7] = 40 %.
        let r = Routes { wires: vec![vec![h(1_000, 0, 10_000)], vec![h(720, 0, 7_000), h(1_280, 3_000, 7_000)]], ..Default::default() };
        assert!((rule().coverage(&r).unwrap() - 0.4).abs() < 1e-6);
    }
}
