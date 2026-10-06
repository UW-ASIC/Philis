//! Matched cells move as one (PLC-03): a set rotates together and, when the
//! members' variant spaces agree index by index, reshapes together.

use analog::Requirements;
use pnr_core::{Layout, Orient, UnionFind};

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
    /// Cell → its orient relative to its orient set's first member (PLC-21):
    /// `Mx180` across each Mirror pair, `R0` otherwise. Length `n`.
    pub rel: Vec<Orient>,
}

/// Locks of `n` cells from every batch's `matched_pairs` (orient and shape)
/// and `mirrored_pairs` (`rel`). Pairs naming a cell `>= n` or a cell twice
/// are dropped. A pair is shape-compatible when both cells' variant spaces
/// list the same `(w, h)` bboxes in the same order, or neither cell has a
/// space (`variants` empty or too short); a pair where only one cell has a
/// space is incompatible.
///
/// `n` must fit cell ids in `u16` (`n <= 65_536`, as [`pnr_core::DeviceId`]).
/// Cost: O((pairs) log pairs + n α(n)) plus the variant comparisons.
#[must_use]
pub fn locks(reqs: &Requirements<Layout>, n: usize, variants: &[gp::VariantSpace]) -> Locks {
    let (mut pairs, mut mirrored) = (Vec::new(), Vec::new());
    for b in reqs.hard.iter().chain(&reqs.budget).chain(&reqs.cost) {
        b.matched_pairs(&mut pairs);
        b.mirrored_pairs(&mut mirrored);
    }
    for v in [&mut pairs, &mut mirrored] {
        v.retain(|&(a, b)| a != b && (a as usize) < n && (b as usize) < n);
        for p in v.iter_mut() {
            *p = (p.0.min(p.1), p.0.max(p.1));
        }
        v.sort_unstable();
        v.dedup();
    }

    let same_shapes = |a: u32, b: u32| match (variants.get(a as usize), variants.get(b as usize)) {
        (Some(p), Some(q)) => {
            p.alternatives.len() == q.alternatives.len()
                && p.alternatives.iter().zip(&q.alternatives).all(|(m, k)| (m.bbox.w, m.bbox.h) == (k.bbox.w, k.bbox.h))
        }
        (None, None) => true,
        _ => false,
    };
    let (mut orient, mut shape) = (UnionFind::new(n), UnionFind::new(n));
    let mut incompatible = 0;
    for &(a, b) in &pairs {
        orient.union(a, b);
        if same_shapes(a, b) {
            shape.union(a, b);
        } else {
            incompatible += 1;
        }
    }
    let (orient, orient_of) = sets(&mut orient, n);
    let (shape, shape_of) = sets(&mut shape, n);
    let rel = relative(&orient, &pairs, &mirrored, n);
    Locks { orient, shape, orient_of, shape_of, incompatible, rel }
}

/// BFS from each set's first member over the pair edges: a Mirror edge
/// composes `Mx180`, any other keeps the orient. First assignment wins.
// ponytail: an inconsistent cycle (odd Mirror loop) is left to `Symmetry::satisfied` to report.
fn relative(sets: &[Vec<u16>], pairs: &[(u32, u32)], mirrored: &[(u32, u32)], n: usize) -> Vec<Orient> {
    let mut adj: Vec<Vec<(usize, bool)>> = vec![Vec::new(); n];
    for &(a, b) in pairs {
        let m = mirrored.binary_search(&(a, b)).is_ok();
        adj[a as usize].push((b as usize, m));
        adj[b as usize].push((a as usize, m));
    }
    let mut rel = vec![Orient::R0; n];
    let mut seen = vec![false; n];
    for set in sets {
        let mut queue = std::collections::VecDeque::from([usize::from(set[0])]);
        seen[usize::from(set[0])] = true;
        while let Some(a) = queue.pop_front() {
            for &(b, m) in &adj[a] {
                if !std::mem::replace(&mut seen[b], true) {
                    rel[b] = if m { rel[a].then(Orient::Mx180) } else { rel[a] };
                    queue.push_back(b);
                }
            }
        }
    }
    rel
}

/// Non-singleton groups (each sorted, sorted by first member) and the
/// cell → set map of length `n`.
fn sets(uf: &mut UnionFind, n: usize) -> (Vec<Vec<u16>>, Vec<Option<u16>>) {
    debug_assert!(n <= usize::from(u16::MAX) + 1, "locks: {n} cells overflow u16 cell ids");
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
    /// Cells sharing `c`'s orient (`shape = false`) or shape set, ascending;
    /// `[c]` when unlocked or `c` is past the map. Allocates the result.
    #[must_use]
    pub fn members(&self, c: usize, shape: bool) -> Vec<usize> {
        let (sets, of) = if shape { (&self.shape, &self.shape_of) } else { (&self.orient, &self.orient_of) };
        match of.get(c).copied().flatten() {
            Some(s) => sets[usize::from(s)].iter().map(|&m| usize::from(m)).collect(),
            None => vec![c],
        }
    }

    /// Re-derive every orient set's members from its first member:
    /// `orient[m] = orient[set[0]].then(rel[m])`. Members past `orient` (or
    /// `rel`) are skipped. Extents are not touched: the caller swaps `hw`/`hh`
    /// of any member whose orient changed axis parity.
    pub fn align(&self, orient: &mut [Orient]) {
        for set in &self.orient {
            let Some(&o0) = orient.get(usize::from(set[0])) else { continue };
            for &m in &set[1..] {
                if let (Some(o), Some(&r)) = (orient.get_mut(usize::from(m)), self.rel.get(usize::from(m))) {
                    *o = o0.then(r);
                }
            }
        }
    }

    /// Set every shape set's members to its first member's variant; members
    /// past `assignment` are skipped.
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
    use analog::placement::symmetry::{SymMode, Symmetry};
    use gp::VariantSpace;
    use pnr_core::geom::Rect;
    use pnr_core::ids::{AxisId, Target};
    use pnr_core::{DeviceId, Macro};

    fn sym(a: u16, b: u16) -> Symmetry {
        Symmetry { a: Target::Device(DeviceId(a)), b: Target::Device(DeviceId(b)), axis: AxisId(0), mode: SymMode::Perfect }
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

    #[test]
    fn mirror_partner_is_related_by_mx180() {
        let reqs = Requirements::<Layout> {
            hard: vec![Box::new(analog::placement::symmetry::SymmetryGroup(vec![Symmetry { mode: SymMode::Mirror, ..sym(0, 1) }]))],
            ..Default::default()
        };
        let k = locks(&reqs, 2, &[]);
        assert_eq!(k.rel, [Orient::R0, Orient::Mx180]);
        let mut o = [Orient::R90, Orient::R0];
        k.align(&mut o);
        assert_eq!(o, [Orient::R90, Orient::R90.then(Orient::Mx180)]);
    }

    /// A batch that only declares pairs.
    struct Pairs {
        matched: Vec<(u32, u32)>,
        mirrored: Vec<(u32, u32)>,
    }

    impl analog::RuleBatch<Layout> for Pairs {
        fn cost(&self, _: &Layout) -> f32 {
            0.0
        }
        fn violations(&self, _: &Layout) -> u32 {
            0
        }
        fn matched_pairs(&self, out: &mut Vec<(u32, u32)>) {
            out.extend_from_slice(&self.matched);
        }
        fn mirrored_pairs(&self, out: &mut Vec<(u32, u32)>) {
            out.extend_from_slice(&self.mirrored);
        }
    }

    fn reqs_of(matched: &[(u32, u32)], mirrored: &[(u32, u32)]) -> Requirements<Layout> {
        Requirements { budget: vec![Box::new(Pairs { matched: matched.to_vec(), mirrored: mirrored.to_vec() })], ..Default::default() }
    }

    #[test]
    fn no_pairs_means_no_locks() {
        for n in [0, 1, 4] {
            let k = locks(&Requirements::default(), n, &[]);
            assert!(k.orient.is_empty() && k.shape.is_empty());
            assert_eq!((k.orient_of.len(), k.shape_of.len(), k.rel.len()), (n, n, n));
            assert!(k.orient_of.iter().chain(&k.shape_of).all(Option::is_none));
            assert!(k.rel.iter().all(|&o| o == Orient::R0));
            assert_eq!(k.incompatible, 0);
        }
    }

    #[test]
    fn self_and_out_of_range_pairs_are_dropped() {
        let k = locks(&reqs_of(&[(1, 1), (0, 3), (3, 7)], &[]), 3, &[]);
        assert!(k.orient.is_empty(), "{:?}", k.orient);
    }

    /// `(a, b)` and `(b, a)` are one pair: counted once as incompatible.
    #[test]
    fn reversed_duplicates_count_once() {
        let k = locks(&reqs_of(&[(0, 1), (1, 0), (0, 1)], &[]), 2, &[space(1, 2), space(2, 1)]);
        assert_eq!(k.incompatible, 1);
        assert_eq!(k.orient, vec![vec![0, 1]]);
        assert!(k.shape.is_empty());
    }

    #[test]
    fn sets_are_transitive_and_sorted() {
        let k = locks(&reqs_of(&[(4, 3), (2, 0), (5, 4)], &[]), 6, &[]);
        assert_eq!(k.orient, vec![vec![0, 2], vec![3, 4, 5]]);
        assert_eq!(k.orient, k.shape, "no variant spaces: every pair is shape-compatible");
        assert_eq!(k.orient_of, vec![Some(0), None, Some(0), Some(1), Some(1), Some(1)]);
        assert_eq!(k.shape_of, k.orient_of);
    }

    /// Shape compatibility is the whole `(w, h)` list, in order.
    #[test]
    fn shape_compatibility_needs_the_same_list_in_the_same_order() {
        let two = |a: (i32, i32), b: (i32, i32)| VariantSpace {
            alternatives: [a, b].iter().map(|&(w, h)| Macro { bbox: Rect { x: 0, y: 0, w, h }, ..Default::default() }).collect(),
        };
        let pair = |p: VariantSpace, q: VariantSpace| locks(&reqs_of(&[(0, 1)], &[]), 2, &[p, q]).incompatible;
        assert_eq!(pair(two((1, 2), (3, 4)), two((1, 2), (3, 4))), 0);
        assert_eq!(pair(two((1, 2), (3, 4)), two((3, 4), (1, 2))), 1, "order matters");
        assert_eq!(pair(two((1, 2), (3, 4)), space(1, 2)), 1, "length matters");
        // Different origins, same extents: compatible.
        let shifted = VariantSpace { alternatives: vec![Macro { bbox: Rect { x: 50, y: -9, w: 1, h: 2 }, ..Default::default() }] };
        assert_eq!(pair(space(1, 2), shifted), 0);
    }

    /// Only one cell of the pair has a variant space: not shape-locked.
    #[test]
    fn a_pair_with_one_variant_space_is_incompatible() {
        let k = locks(&reqs_of(&[(0, 1)], &[]), 2, &[space(1, 2)]);
        assert_eq!((k.incompatible, k.shape.len(), k.orient.len()), (1, 0, 1));
    }

    /// Mirror composes along a chain: a reflection of a reflection is upright.
    #[test]
    fn rel_composes_mirror_edges_along_a_chain() {
        let k = locks(&reqs_of(&[(0, 1), (1, 2), (2, 3)], &[(1, 0), (1, 2)]), 4, &[]);
        assert_eq!(k.rel, vec![Orient::R0, Orient::Mx180, Orient::R0, Orient::R0]);
    }

    /// A mirrored pair that is not matched has no orient set, so no relation.
    #[test]
    fn mirrored_but_unmatched_pairs_relate_nothing() {
        let k = locks(&reqs_of(&[], &[(0, 1)]), 2, &[]);
        assert_eq!(k.rel, vec![Orient::R0, Orient::R0]);
    }

    #[test]
    fn members_returns_the_set_or_the_cell_alone() {
        let k = locks(&reqs_of(&[(0, 2)], &[]), 4, &[]);
        assert_eq!(k.members(2, false), vec![0, 2]);
        assert_eq!(k.members(0, true), vec![0, 2]);
        assert_eq!(k.members(1, false), vec![1]);
        assert_eq!(k.members(99, true), vec![99], "past the map reads as unlocked");
    }

    #[test]
    fn align_and_unify_skip_members_past_the_slice() {
        let k = locks(&reqs_of(&[(0, 1), (0, 3)], &[(0, 3)]), 4, &[]);
        let mut o = [Orient::R90, Orient::R0];
        k.align(&mut o);
        assert_eq!(o, [Orient::R90, Orient::R90]);
        let mut a = [2u16, 0];
        k.unify(&mut a);
        assert_eq!(a, [2, 2]);
        let (mut none, mut empty): ([Orient; 0], [u16; 0]) = ([], []);
        k.align(&mut none);
        k.unify(&mut empty);
    }

    /// `align` keeps a Perfect partner equal and a Mirror partner reflected
    /// for every starting orient of the first member.
    #[test]
    fn align_holds_for_every_orient() {
        let k = locks(&reqs_of(&[(0, 1), (0, 2)], &[(0, 2)]), 3, &[]);
        for o0 in Orient::ALL {
            let mut o = [o0, Orient::R270, Orient::Mx];
            k.align(&mut o);
            assert_eq!(o, [o0, o0, o0.then(Orient::Mx180)], "{o0:?}");
        }
    }
}
