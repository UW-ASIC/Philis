//! Symmetry-island connectivity (PLC-12; Balasa Def. 2.1): the members of one
//! symmetry group should form one connected island, each touching another
//! across a shared edge, not merely sit near the same axis.

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use pnr_core::UnionFind;

use crate::rule::RuleBatch;

/// One symmetry group whose members must form a single edge-connected island.
/// The batch is one condition; its cost and residual are the islands past
/// the first.
#[derive(Clone, Debug)]
pub struct SymmetryIsland {
    /// The group's members (cells after retarget).
    pub members: Vec<Target>,
    /// Largest edge gap that still counts as touching, nm.
    pub touch_nm: i32,
}

/// `a` and `b` share an edge: strictly overlapping projections on one axis and
/// an edge distance ≤ `touch_nm` on the other (a corner contact is not
/// adjacency, so this is not `Layout::edge_gap`, which is 0 there).
fn adjacent(l: &Layout, a: Target, b: Target, touch_nm: i32) -> bool {
    let (ax, ay, aw, ah) = l.bbox(a);
    let (bx, by, bw, bh) = l.bbox(b);
    let (dx, dy) = ((ax - bx).abs(), (ay - by).abs());
    (dx < aw + bw && dy - (ah + bh) <= touch_nm) || (dy < ah + bh && dx - (aw + bw) <= touch_nm)
}

/// Returns the number of connected components of `members` under edge
/// adjacency (a shared edge within `touch_nm`; a corner contact does not
/// connect); `0` with no members.
///
/// Cost: O(n²) pair tests, n = `members.len()`.
///
/// # Panics
/// On a target out of range of `l` (see [`Layout::bbox`]).
#[must_use]
pub fn components(l: &Layout, members: &[Target], touch_nm: i32) -> u32 {
    let n = members.len();
    let mut uf = UnionFind::new(n);
    for i in 0..n {
        for j in i + 1..n {
            if adjacent(l, members[i], members[j], touch_nm) {
                uf.union(i as u32, j as u32);
            }
        }
    }
    let mut roots: Vec<u32> = (0..n as u32).map(|i| uf.find(i)).collect();
    roots.sort_unstable();
    roots.dedup();
    roots.len() as u32
}

impl SymmetryIsland {
    /// Islands past the first (`0` = connected or empty).
    fn extra(&self, l: &Layout) -> u32 {
        components(l, &self.members, self.touch_nm).saturating_sub(1)
    }
}

impl RuleBatch<Layout> for SymmetryIsland {
    /// Islands past the first.
    fn cost(&self, l: &Layout) -> f32 {
        self.extra(l) as f32
    }
    fn violations(&self, l: &Layout) -> u32 {
        u32::from(self.extra(l) > 0)
    }
    fn residual(&self, l: &Layout) -> f64 {
        f64::from(self.extra(l))
    }
    fn kind(&self) -> &'static str {
        "SymmetryIsland"
    }
    fn count(&self) -> usize {
        1
    }
    fn touched(&self, out: &mut Vec<u32>) {
        super::push_devices(out, &self.members);
    }
    fn retarget(&mut self, cell_of: &[u16]) {
        for t in &mut self.members {
            *t = t.retarget(cell_of);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::ids::DeviceId;

    /// 1000 nm squares at the given centres.
    fn squares(c: &[(i32, i32)]) -> Layout {
        let n = c.len();
        Layout {
            x: c.iter().map(|p| p.0).collect(),
            y: c.iter().map(|p| p.1).collect(),
            hw: vec![500; n],
            hh: vec![500; n],
            axis: vec![0],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); n],
            variant: vec![0; n],
            branch: Vec::new(),
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    fn island(n: u16, touch_nm: i32) -> SymmetryIsland {
        SymmetryIsland { members: (0..n).map(|i| Target::Device(DeviceId(i))).collect(), touch_nm }
    }

    #[test]
    fn abutting_pair_is_one_island() {
        let l = squares(&[(0, 0), (1_000, 0)]);
        assert_eq!((island(2, 280).residual(&l), island(2, 280).violations(&l)), (0.0, 0));
    }

    #[test]
    fn a_gap_splits_the_island() {
        let ok = squares(&[(0, 0), (1_280, 0)]);
        assert_eq!(island(2, 280).residual(&ok), 0.0);
        let split = squares(&[(0, 0), (1_000 + 280 + 10, 0)]);
        assert_eq!((island(2, 280).residual(&split), island(2, 280).violations(&split)), (1.0, 1));
    }

    #[test]
    fn diagonal_corner_touch_is_not_adjacent() {
        let l = squares(&[(0, 0), (1_000, 1_000)]);
        assert_eq!(island(2, 280).residual(&l), 1.0);
    }

    #[test]
    fn touched_names_every_member() {
        let mut ids = Vec::new();
        let mut i = island(3, 0);
        i.members.push(Target::Group(pnr_core::GroupId(0)));
        i.touched(&mut ids);
        assert_eq!(ids, [0, 1, 2]);
    }
}
