//! Common-node resistance balance (routing tier, budget).

use pnr_core::geom::{Rect, Shape};
use pnr_core::ids::NetId;
use pnr_core::routes::{conductor_layers_meet, Join, Routes};

use crate::rule::RuleBatch;
use super::Stack;

/// Terminal groups sharing a net (a pair's tail, a mirror's rail; a star's
/// branches; a Kelvin force/sense split): the routed R from the feeds to each
/// group's pins must agree, or the degeneration differs. `ΔR` in series with a source carrying
/// `I` reads as an offset `I·ΔR` (Hastings §13.3.3; a 10 Ω skew at 1 mS is a
/// 1% mirror error), so the budget is the offset share the pair allows,
/// `max_delta_ohm = η·σ_rand / I`.
///
/// Built per routed layout: the pins are placed geometry. Each side's pins
/// combine in parallel ([`Stack::terminal_resistance_ohm`]).
#[derive(Clone, Debug)]
pub struct CommonNode {
    pub net: NetId,
    /// Terminal groups (a pair's members' source pins; a star's branches;
    /// Kelvin `[force, sense]`): each group's branch is its own subtree.
    pub groups: Vec<Vec<Rect>>,
    /// Where the current enters the net: every other pin on it (a tail
    /// device's drain, a rail's port; the Kelvin tap). Empty = the net's centre.
    pub feeds: Vec<Rect>,
    /// Allowed max − min of the groups' R, Ω; `≤ 0` = unknown (no current or
    /// `A_VT`), or no balance check (Kelvin).
    pub max_delta_ohm: f32,
    /// Branches may meet only inside the root halo (Hastings §15.6.2 star;
    /// §15.6.3 Kelvin: the sense lead carries no force current).
    pub star: bool,
}

/// Every common node of one routed layout, measured on `stack`.
#[derive(Clone)]
pub struct CommonNodes {
    pub nodes: Vec<CommonNode>,
    pub stack: &'static Stack,
    /// The root halo: a star's feeds grown by this, nm (the lattice pitch).
    pub halo_nm: i32,
    /// What joins two layers (`Routes::debug_check_joined`'s list), for the
    /// star's components.
    pub joins: Vec<Join>,
}

impl CommonNodes {
    /// Max − min of the groups' parallel R, Ω (two groups: `|R_a − R_b|`);
    /// `None` when a group's pins are not all reached.
    fn delta_ohm(&self, n: &CommonNode, r: &Routes) -> Option<f32> {
        let pins: Vec<Rect> = n.groups.iter().flatten().copied().collect();
        let branch = if n.feeds.is_empty() {
            self.stack.terminal_resistance_ohm(r.shapes(n.net), &pins)
        } else {
            self.stack.fed_resistance_ohm(r.shapes(n.net), &n.feeds, &pins)
        };
        let side = |k: std::ops::Range<usize>| -> Option<f32> {
            let g: f32 = branch[k].iter().map(|r| r.map(|r| 1.0 / r.max(1e-3))).sum::<Option<f32>>()?;
            (g > 0.0).then(|| 1.0 / g)
        };
        let mut start = 0;
        let mut rs = Vec::with_capacity(n.groups.len());
        for g in &n.groups {
            rs.push(side(start..start + g.len())?);
            start += g.len();
        }
        let (lo, hi) = rs.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &x| (a.min(x), b.max(x)));
        (hi >= lo).then_some(hi - lo)
    }

    fn usage(&self, n: &CommonNode, r: &Routes) -> Option<f32> {
        (n.max_delta_ohm > 0.0).then_some(())?;
        Some(self.delta_ohm(n, r)? / n.max_delta_ohm)
    }

    /// A star broken: with the feeds grown by `halo_nm` cut out of the net's
    /// shapes (the remainder kept: a trunk leaving the halo still joins the
    /// branches it carries), one connected piece (by [`Self::joins`]) touches
    /// pins of two groups. `false` for a node that is no star.
    #[must_use]
    pub fn star_broken(&self, n: &CommonNode, r: &Routes) -> bool {
        if !n.star {
            return false;
        }
        let h = self.halo_nm;
        let mut shapes: Vec<Shape> = r.shapes(n.net).to_vec();
        for f in &n.feeds {
            let g = Rect { x: f.x - h, y: f.y - h, w: f.w + 2 * h, h: f.h + 2 * h };
            shapes = shapes.into_iter().flat_map(|s| minus(s.rect, g).into_iter().map(move |rect| Shape { rect, ..s })).collect();
        }
        let mut seen = vec![false; shapes.len()];
        for first in 0..shapes.len() {
            if seen[first] {
                continue;
            }
            seen[first] = true;
            let (mut stack, mut piece) = (vec![first], vec![first]);
            while let Some(a) = stack.pop() {
                for b in 0..shapes.len() {
                    if !seen[b] && shapes[a].rect.touches(&shapes[b].rect) && conductor_layers_meet(&shapes[a], &shapes[b], &self.joins) {
                        seen[b] = true;
                        stack.push(b);
                        piece.push(b);
                    }
                }
            }
            let groups = n.groups.iter().filter(|g| g.iter().any(|p| piece.iter().any(|&k| shapes[k].rect.touches(p)))).count();
            if groups > 1 {
                return true;
            }
        }
        false
    }
}

/// `a` less the interior of `g`: up to four pieces (left and right full
/// height, bottom and top between them); `a` whole when they do not overlap.
fn minus(a: Rect, g: Rect) -> Vec<Rect> {
    let (ar, at, gr, gt) = (a.x + a.w, a.y + a.h, g.x + g.w, g.y + g.h);
    if a.x >= gr || g.x >= ar || a.y >= gt || g.y >= at {
        return vec![a];
    }
    let (l, r) = (a.x.max(g.x), ar.min(gr));
    [
        Rect { x: a.x, y: a.y, w: l - a.x, h: a.h },
        Rect { x: r, y: a.y, w: ar - r, h: a.h },
        Rect { x: l, y: a.y, w: r - l, h: g.y - a.y },
        Rect { x: l, y: gt, w: r - l, h: at - gt },
    ]
    .into_iter()
    .filter(|p| p.w > 0 && p.h > 0)
    .collect()
}

impl RuleBatch<Routes> for CommonNodes {
    fn cost(&self, r: &Routes) -> f32 {
        self.nodes.iter().filter_map(|n| self.usage(n, r)).sum()
    }
    /// Star breaks plus ΔR overshoots.
    fn violations(&self, r: &Routes) -> u32 {
        self.nodes.iter().map(|n| u32::from(self.star_broken(n, r)) + u32::from(self.usage(n, r).is_some_and(|u| u > 1.0))).sum()
    }
    fn residual(&self, r: &Routes) -> f64 {
        self.nodes.iter().filter_map(|n| self.usage(n, r)).map(|u| f64::from((u - 1.0).max(0.0))).sum()
    }
    fn kind(&self) -> &'static str {
        "CommonNode"
    }
    fn repair_kind(&self) -> crate::RepairKind {
        crate::RepairKind::Balance
    }
    fn count(&self) -> usize {
        self.nodes.len()
    }
    fn worst_usage(&self, r: &Routes) -> Option<f32> {
        self.nodes.iter().filter_map(|n| self.usage(n, r)).reduce(f32::max)
    }
    fn unknown(&self, r: &Routes) -> u32 {
        self.nodes.iter().filter(|n| self.usage(n, r).is_none()).count() as u32
    }
    fn touched(&self, out: &mut Vec<u32>) {
        out.extend(self.nodes.iter().map(|n| u32::from(n.net.0)));
    }
    fn violating_ids(&self, r: &Routes, out: &mut Vec<u32>) {
        out.extend(self.nodes.iter().filter(|n| self.star_broken(n, r) || self.usage(n, r).is_some_and(|u| u > 1.0)).map(|n| u32::from(n.net.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::stack::Layer;
    use pnr_core::geom::{LayerId, Shape};

    /// A tail rail fed from one end: the near member's source sees less R
    /// than the far one's. Balanced (fed from the middle), the skew vanishes.
    #[test]
    fn two_groups_match_the_old_a_b_measure() {
        let stack: &'static Stack = Box::leak(Box::new(Stack {
            layers: vec![Layer { id: 1, sheet_ohm: 0.125, ..Layer::default() }],
            antenna_cumulative: false,
            diode: None,
        }));
        let wire = |x, y, w, h| Shape { layer: LayerId(1), rect: Rect { x, y, w, h } };
        let pin = |x| Rect { x, y: 0, w: 100, h: 100 };
        let mut routes = Routes { wires: vec![Vec::new()], ..Default::default()  };
        // 40 µm rail at 0.5 µm: members at x = 0 and 10 µm, fed at 40 µm.
        routes.wires[0] = vec![wire(0, 0, 40_100, 500)];
        let node = |feed: i32, max| CommonNodes {
            nodes: vec![CommonNode { net: NetId(0), groups: vec![vec![pin(0)], vec![pin(10_000)]], feeds: vec![pin(feed)], max_delta_ohm: max, star: false }],
            stack,
            halo_nm: 0,
            joins: Vec::new(),
        };
        assert_eq!(node(40_000, 0.0).unknown(&routes), 1, "no budget: unknown, never a pass");
        // 10 µm more rail to a: 20 squares, 2.5 Ω of skew.
        let u = node(40_000, 1.0).worst_usage(&routes).unwrap();
        assert!((u - 2.5).abs() < 1e-3, "{u}");
        assert_eq!(node(40_000, 1.0).violations(&routes), 1);
        // Fed midway between the members: no skew.
        assert!(node(5_000, 1.0).worst_usage(&routes).unwrap() < 1e-3);
    }

    /// Two groups' branches from a feed at x = 0: `a_end`/`b_end` pins, the
    /// branches sharing the rail up to `shared` nm before they split.
    fn star(shared: i32) -> (CommonNodes, Routes) {
        let stack: &'static Stack = Box::leak(Box::new(Stack { layers: vec![Layer { id: 1, sheet_ohm: 0.125, ..Layer::default() }], antenna_cumulative: false, diode: None }));
        let wire = |x, y, w, h| Shape { layer: LayerId(1), rect: Rect { x, y, w, h } };
        let pin = |x, y| Rect { x, y, w: 100, h: 100 };
        // Trunk from the feed to x = shared, then one branch up, one down.
        let routes = Routes {
            wires: vec![vec![wire(0, 0, shared + 100, 100), wire(shared, 0, 100, 10_000), wire(shared, -10_000, 100, 10_000)]],
            ..Default::default()
        };
        let node = CommonNode { net: NetId(0), groups: vec![vec![pin(shared, 9_900)], vec![pin(shared, -10_000)]], feeds: vec![pin(0, 0)], max_delta_ohm: 0.0, star: true };
        (CommonNodes { nodes: vec![node], stack, halo_nm: 420, joins: Vec::new() }, routes)
    }

    /// Branches sharing 5 µm of rail outside the root halo: one star break.
    #[test]
    fn a_shared_segment_breaks_the_star() {
        let (c, r) = star(5_000);
        assert_eq!(c.violations(&r), 1);
    }

    /// One trunk drawn from the feed out to 5 µm, the branches leaving it at
    /// x = 2 µm and 4 µm (they never touch each other): the trunk beyond the
    /// halo is shared, one star break.
    #[test]
    fn branches_leaving_one_trunk_apart_break_the_star() {
        let (mut c, mut r) = star(5_000);
        let wire = |x, y, w, h| Shape { layer: LayerId(1), rect: Rect { x, y, w, h } };
        r.wires[0] = vec![wire(0, 0, 5_100, 100), wire(2_000, 0, 100, 10_000), wire(4_000, -10_000, 100, 10_000)];
        c.nodes[0].groups = vec![vec![Rect { x: 2_000, y: 9_900, w: 100, h: 100 }], vec![Rect { x: 4_000, y: -10_000, w: 100, h: 100 }]];
        assert_eq!(c.violations(&r), 1);
    }

    /// The same branches meeting inside the feed grown by `p0`: a star.
    #[test]
    fn branches_meeting_at_the_root_are_a_star() {
        let (c, r) = star(300);
        assert_eq!(c.violations(&r), 0);
    }
}
