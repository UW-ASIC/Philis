//! DC current distribution over a routed net's drawn shapes: what the
//! electromigration rule checks each segment and via against.

use pnr_core::geom::Shape;
use pnr_core::routes::Terminal;
use super::Stack;

/// DC current distribution of one routed net.
pub struct NetFlow {
    /// Worst-case current through each of the net's `shapes`, µA (`0` for a
    /// shape off the stack or on no path between terminals).
    pub shape_ua: Vec<f32>,
    /// Largest DC drop between any two points of the net, µV (REL-04).
    pub drop_uv: f32,
}

/// Each segment carries the sum of the terminal currents on one side of it
/// (Lienig & Thiele 2018 eqs. 3.5–3.7; KCL at a DC point), over the least-R
/// spanning tree of [`Stack::port_graph`] rooted at terminal 0. A terminal
/// joins the lowest stack layer it overlaps, so its pin cut carries its
/// current. When the currents do not sum to zero, a port whose position is
/// unknown supplies the rest, so a tree edge carries the larger side
/// (`max(|S|, |total − S|)`), which holds wherever that port sits. A link off
/// the tree (a mesh chord) and every link of the loop it closes carry at most
/// the largest tree-edge current on the tree path between its ends: a
/// parallel branch splits what the path carries.
///
/// `None` when a terminal's current is unknown, no terminal touches the
/// routes, or a terminal is unreached (open).
///
/// ponytail: the chord bound is a general-mesh bound; a nodal solve replaces
/// it if a test ever disproves it. O(k²) in shapes, like `port_graph`.
#[must_use]
pub fn net_flow(stack: &Stack, shapes: &[Shape], terms: &[Terminal]) -> Option<NetFlow> {
    terms.first()?;
    let ua: Vec<f32> = terms.iter().map(|t| t.ua).collect::<Option<_>>()?;
    let rects: Vec<_> = terms.iter().map(|t| t.at).collect();
    let g = stack.port_graph(shapes, &rects, true);
    let root = (*g.term.first()?)?;
    let parent = g.tree(root);
    let reached = |n: usize| n == root || parent[n].is_some();
    let nodes: Vec<usize> = g.term.iter().map(|t| t.filter(|&n| reached(n))).collect::<Option<_>>()?;

    // Injection per node, then subtree sums leaf-up (children before parents:
    // reverse of a preorder from the root).
    let mut sum = vec![0.0f32; g.adj.len()];
    for (&n, &i) in nodes.iter().zip(&ua) {
        sum[n] += i;
    }
    let total: f32 = ua.iter().sum();
    let bal = total.abs() <= 1e-3 * ua.iter().fold(0.0f32, |m, i| m.max(i.abs()));
    let mut kids: Vec<Vec<usize>> = vec![Vec::new(); g.adj.len()];
    for (n, p) in parent.iter().enumerate() {
        if let Some((p, ..)) = p {
            kids[*p].push(n);
        }
    }
    let (mut order, mut depth) = (vec![root], vec![0u32; g.adj.len()]);
    for k in 0.. {
        let Some(&n) = order.get(k) else { break };
        for &c in &kids[n] {
            depth[c] = depth[n] + 1;
            order.push(c);
        }
    }
    // Current on the tree edge into each node, µA.
    let mut edge = vec![0.0f32; g.adj.len()];
    for &n in order.iter().rev().filter(|&&n| n != root) {
        let s = sum[n];
        edge[n] = if bal { s.abs() } else { s.abs().max((total - s).abs()) };
        if let Some((p, ..)) = parent[n] {
            sum[p] += s;
        }
    }

    // A link off the tree (a chord, 0-Ω ones included) closes a loop with
    // its tree path: every link of that loop may carry the largest tree-edge
    // current on the path (current splits between the loop's two sides, and
    // the tree leaves one side dangling at 0).
    let mut bound = edge.clone();
    let mut shape_ua = vec![0.0f32; shapes.len()];
    let up = |n: usize| parent[n].map_or(root, |p| p.0);
    let path = |mut x: usize, mut y: usize, f: &mut dyn FnMut(usize)| {
        while x != y {
            let n = if depth[x] >= depth[y] { &mut x } else { &mut y };
            f(*n);
            *n = up(*n);
        }
    };
    for (a, links) in g.adj.iter().enumerate().filter(|&(a, _)| reached(a)) {
        for &(b, _, shape) in links.iter().filter(|l| l.0 > a && reached(l.0)) {
            if parent[b].is_some_and(|p| p.0 == a) || parent[a].is_some_and(|p| p.0 == b) {
                continue;
            }
            let mut m = 0.0f32;
            path(a, b, &mut |n| m = m.max(edge[n]));
            path(a, b, &mut |n| bound[n] = bound[n].max(m));
            if let Some(s) = shape_ua.get_mut(shape as usize) {
                *s = s.max(m);
            }
        }
    }
    for &n in &order {
        if let Some((_, _, shape)) = parent[n] {
            if let Some(s) = shape_ua.get_mut(shape as usize) {
                *s = s.max(bound[n]);
            }
        }
    }

    // Weighted diameter of the tree at R·I per edge (Ω·µA = µV), by a double
    // sweep: the farthest node from the root, then the farthest from it.
    let mut tadj: Vec<Vec<(usize, f32)>> = vec![Vec::new(); g.adj.len()];
    for &n in &order {
        if let Some((p, r, _)) = parent[n] {
            tadj[n].push((p, r * edge[n]));
            tadj[p].push((n, r * edge[n]));
        }
    }
    let far = |src: usize| -> (usize, f32) {
        let mut dist = vec![f32::NAN; tadj.len()];
        dist[src] = 0.0;
        let mut stack = vec![src];
        let mut best = (src, 0.0f32);
        while let Some(a) = stack.pop() {
            for &(b, w) in &tadj[a] {
                if dist[b].is_nan() {
                    dist[b] = dist[a] + w;
                    best = if dist[b] > best.1 { (b, dist[b]) } else { best };
                    stack.push(b);
                }
            }
        }
        best
    };
    let drop_uv = far(far(root).0).1;
    Some(NetFlow { shape_ua, drop_uv })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::stack::Layer;
    use pnr_core::geom::{LayerId, Rect};

    /// met1 only, 0.125 Ω/□.
    fn stack() -> Stack {
        Stack { layers: vec![Layer { id: 1, sheet_ohm: 0.125, ..Layer::default() }], ..Stack::default() }
    }
    fn m1(x: i32, y: i32, w: i32, h: i32) -> Shape {
        Shape { layer: LayerId(1), rect: Rect { x, y, w, h } }
    }
    fn term(x: i32, y: i32, w: i32, h: i32, ua: Option<f32>) -> Terminal {
        Terminal { at: Rect { x, y, w, h }, ua }
    }

    #[test]
    fn no_terminals_an_unknown_current_or_an_unreached_terminal_is_none() {
        let st = stack();
        let wire = [m1(0, 0, 10_000, 500)];
        assert!(net_flow(&st, &wire, &[]).is_none(), "no terminals");
        assert!(net_flow(&st, &wire, &[term(0, 0, 200, 500, Some(1.0)), term(9_800, 0, 200, 500, None)]).is_none(), "unknown current");
        assert!(net_flow(&st, &wire, &[term(50_000, 0, 10, 10, Some(1.0))]).is_none(), "touches nothing");
        assert!(net_flow(&st, &wire, &[term(0, 0, 200, 500, Some(1.0)), term(50_000, 0, 10, 10, Some(-1.0))]).is_none(), "open");
        assert!(net_flow(&st, &[], &[term(0, 0, 200, 500, Some(0.0))]).is_none(), "nothing routed");
    }

    #[test]
    fn a_lone_terminal_carries_nothing() {
        let f = net_flow(&stack(), &[m1(0, 0, 10_000, 500)], &[term(0, 0, 200, 500, Some(0.0))]).unwrap();
        assert_eq!(f.shape_ua, vec![0.0]);
        assert_eq!(f.drop_uv, 0.0);
    }

    /// ±300 µA end to end: the wire carries 300 µA and drops I·R between the
    /// pin centres, 300 µA · 0.125 Ω/□ · 9.8 µm / 0.5 µm = 735 µV. A shape
    /// off the stack carries nothing.
    #[test]
    fn a_straight_wire_carries_its_current_and_drops_i_r() {
        let wire = [m1(0, 0, 10_000, 500), Shape { layer: LayerId(9), rect: Rect { x: 0, y: 0, w: 10_000, h: 500 } }];
        let terms = [term(0, 0, 200, 500, Some(300.0)), term(9_800, 0, 200, 500, Some(-300.0))];
        let f = net_flow(&stack(), &wire, &terms).unwrap();
        assert!((f.shape_ua[0] - 300.0).abs() < 1e-3, "{:?}", f.shape_ua);
        assert_eq!(f.shape_ua[1], 0.0, "off the stack");
        assert!((f.drop_uv - 735.0).abs() < 0.1, "{}", f.drop_uv);
    }

    /// Metamorphic: reversing every current or translating the whole net
    /// changes neither the per-shape currents nor the drop.
    #[test]
    fn reversing_or_translating_the_net_changes_nothing() {
        let wires = [m1(0, 0, 10_000, 1_000), m1(4_000, 1_000, 500, 3_000), m1(8_000, 1_000, 500, 3_000)];
        let terms = [term(0, 400, 200, 200, Some(-500.0)), term(4_000, 3_900, 500, 100, Some(300.0)), term(8_000, 3_900, 500, 100, Some(200.0))];
        let base = net_flow(&stack(), &wires, &terms).unwrap();
        let rev: Vec<Terminal> = terms.iter().map(|t| Terminal { ua: t.ua.map(|i| -i), ..*t }).collect();
        let flipped = net_flow(&stack(), &wires, &rev).unwrap();
        assert_eq!(base.shape_ua, flipped.shape_ua);
        assert!((base.drop_uv - flipped.drop_uv).abs() < 1e-3);
        let mv = |r: Rect| Rect { x: r.x + 12_345, y: r.y - 6_789, ..r };
        let w2: Vec<Shape> = wires.iter().map(|s| Shape { rect: mv(s.rect), ..*s }).collect();
        let t2: Vec<Terminal> = terms.iter().map(|t| Terminal { at: mv(t.at), ..*t }).collect();
        let moved = net_flow(&stack(), &w2, &t2).unwrap();
        for (a, b) in base.shape_ua.iter().zip(&moved.shape_ua) {
            assert!((a - b).abs() < 1e-3, "{:?} vs {:?}", base.shape_ua, moved.shape_ua);
        }
        assert!((base.drop_uv - moved.drop_uv).abs() < 1e-2);
    }
}
