//! Matched cells move as one (PLC-03): a set rotates together and, when the
//! members' variant spaces agree index by index, reshapes together.

use analog::Requirements;
use pnr_core::{Layout, UnionFind};

/// Orient and shape sets over cells, from every tier's `matched_pairs`.
#[derive(Clone, Debug, Default)]
pub struct Locks {
    /// Cells that rotate together. Singleton sets omitted; each set sorted, sets sorted by first member.
    pub orient: Vec<Vec<u16>>,
    /// Cells that reshape together (same variant index). Same conventions.
    pub shape: Vec<Vec<u16>>,
    /// Cell → index into `orient`; `None` = unlocked. Length `n`.
    pub orient_of: Vec<Option<u16>>,
    /// Cell → index into `shape`; `None` = unlocked. Length `n`.
    pub shape_of: Vec<Option<u16>>,
    /// Distinct matched pairs whose variant spaces differ (not shape-locked).
    pub incompatible: u32,
}

/// Locks of `n` cells. A pair is shape-compatible when both cells' variant
/// spaces list the same `(w, h)` bboxes in the same order; with `variants`
/// empty (or too short) every pair is.
#[must_use]
pub fn locks(reqs: &Requirements<Layout>, n: usize, variants: &[gp::VariantSpace]) -> Locks {
    let mut pairs = Vec::new();
    for b in reqs.hard.iter().chain(&reqs.budget).chain(&reqs.cost) {
        b.matched_pairs(&mut pairs);
    }
    pairs.retain(|&(a, b)| a != b && (a as usize) < n && (b as usize) < n);
    for p in &mut pairs {
        *p = (p.0.min(p.1), p.0.max(p.1));
    }
    pairs.sort_unstable();
    pairs.dedup();

    let dims = |i: u32| {
        variants.get(i as usize).map(|s| s.alternatives.iter().map(|m| (m.bbox.w, m.bbox.h)).collect::<Vec<_>>())
    };
    let (mut orient, mut shape) = (UnionFind::new(n), UnionFind::new(n));
    let mut incompatible = 0;
    for &(a, b) in &pairs {
        orient.union(a, b);
        if dims(a) == dims(b) {
            shape.union(a, b);
        } else {
            incompatible += 1;
        }
    }
    let (orient, orient_of) = sets(&mut orient, n);
    let (shape, shape_of) = sets(&mut shape, n);
    Locks { orient, shape, orient_of, shape_of, incompatible }
}

/// Non-singleton groups, sorted, and the cell → set map.
fn sets(uf: &mut UnionFind, n: usize) -> (Vec<Vec<u16>>, Vec<Option<u16>>) {
    let mut out: Vec<Vec<u16>> = uf
        .groups()
        .into_iter()
        .filter(|g| g.len() >= 2)
        .map(|g| {
            let mut g: Vec<u16> = g.into_iter().map(|c| c as u16).collect();
            g.sort_unstable();
            g
        })
        .collect();
    out.sort_unstable();
    let mut of = vec![None; n];
    for (i, g) in out.iter().enumerate() {
        for &c in g {
            of[usize::from(c)] = Some(i as u16);
        }
    }
    (out, of)
}

impl Locks {
    /// Cells sharing `c`'s orient (`shape = false`) or shape set; `[c]` when unlocked.
    #[must_use]
    pub fn members(&self, c: usize, shape: bool) -> Vec<usize> {
        let (sets, of) = if shape { (&self.shape, &self.shape_of) } else { (&self.orient, &self.orient_of) };
        match of.get(c).copied().flatten() {
            Some(s) => sets[usize::from(s)].iter().map(|&m| usize::from(m)).collect(),
            None => vec![c],
        }
    }

    /// Set every shape set's members to its first member's variant.
    pub fn unify(&self, assignment: &mut [u16]) {
        for set in &self.shape {
            let Some(&v) = assignment.get(usize::from(set[0])) else { continue };
            for &m in &set[1..] {
                if let Some(a) = assignment.get_mut(usize::from(m)) {
                    *a = v;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use analog::placement::symmetry::Symmetry;
    use gp::VariantSpace;
    use pnr_core::geom::Rect;
    use pnr_core::ids::{AxisId, Target};
    use pnr_core::{DeviceId, Macro};

    fn sym(a: u16, b: u16) -> Symmetry {
        Symmetry { a: Target::Device(DeviceId(a)), b: Target::Device(DeviceId(b)), axis: AxisId(0) }
    }

    fn space(w: i32, h: i32) -> VariantSpace {
        VariantSpace { alternatives: vec![Macro { bbox: Rect { x: 0, y: 0, w, h }, ..Default::default() }] }
    }

    #[test]
    fn locks_join_only_compatible_shapes() {
        let reqs = Requirements::<Layout> { cost: vec![Box::new(vec![sym(0, 1), sym(0, 2)])], ..Default::default() };
        let variants = [space(1_000, 4_000), space(1_000, 4_000), space(2_000, 2_000)];
        let k = locks(&reqs, 3, &variants);
        assert_eq!(k.orient, vec![vec![0, 1, 2]]);
        assert_eq!(k.shape, vec![vec![0, 1]]);
        assert_eq!(k.incompatible, 1);
        assert_eq!(k.shape_of[2], None);
        let mut a = [2, 0, 1];
        k.unify(&mut a);
        assert_eq!(a, [2, 2, 1]);
    }
}
