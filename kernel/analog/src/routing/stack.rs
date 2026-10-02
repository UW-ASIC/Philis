//! The routing stack's per-layer parasitics and antenna limits, from the deck:
//! what the budget rules measure drawn metal with.

use pnr_core::geom::{Rect, Shape};

/// One conductor of the stack (a metal or a cut), bottom-up in
/// [`Stack::layers`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layer {
    /// `LayerId` of the deck layer.
    pub id: u16,
    /// Ground capacitance per area and per edge length: aF/µm², aF/µm.
    pub area_af_um2: f32,
    pub fringe_af_um: f32,
    /// `ε0·k·t` in aF·nm/µm: lateral coupling to a parallel wire `gap` nm
    /// away is `lateral · run_µm / gap_nm` aF (TOPO eq. 4.3).
    pub lateral: f32,
    /// Antenna limit of this layer's etch stage (conductor/gate area); `0` =
    /// the deck checks none here.
    pub antenna_ratio: f32,
    /// Conductor thickness when the rule counts sidewall (peripheral) area,
    /// `perimeter · t`, nm; `0` = top (areal) area (Hastings p. 229).
    pub antenna_sidewall_nm: f32,
    /// Sheet resistance, Ω/□ for a metal; for a cut, Ω per cut (the deck's
    /// `pex.sheet_res_ohm_sq` on a via layer).
    pub sheet_ohm: f32,
    /// This layer is a cut (every other stack layer, from the first metal).
    pub cut: bool,
}

/// The routing stack, bottom-up (metals and cuts interleaved).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stack {
    pub layers: Vec<Layer>,
    /// The deck's antenna rules sum every layer up to the stage
    /// (`antenna_cumulative_*`); else each stage counts its own layer.
    pub antenna_cumulative: bool,
    /// The deck's diode marker: a net whose routes carry a shape on it has a
    /// protection diode (diode credit, Hastings p. 229). `None` = no credit.
    pub diode_layer: Option<u16>,
}

impl Stack {
    fn at(&self, id: u16) -> Option<(usize, &Layer)> {
        self.layers.iter().enumerate().find(|(_, l)| l.id == id)
    }

    /// Ground capacitance of `shapes`, aF: area + fringe on the perimeter's
    /// long sides (Lampaert 1999 eqs. 2.29–2.32; the deck's `pex` terms). A
    /// shape on a layer the stack does not know adds nothing.
    ///
    /// ponytail: shapes are summed, not unioned — a via pad on a wire counts
    /// twice (over-estimates by a pad per via); extraction's union is signoff's.
    #[must_use]
    pub fn ground_af(&self, shapes: &[Shape]) -> f32 {
        shapes
            .iter()
            .filter_map(|s| {
                let (_, l) = self.at(s.layer.0)?;
                let (w, len) = (s.rect.w.min(s.rect.h) as f32 / 1e3, s.rect.w.max(s.rect.h) as f32 / 1e3);
                Some(l.area_af_um2 * w * len + 2.0 * l.fringe_af_um * len)
            })
            .sum()
    }

    /// Series resistance of `shapes`, Ω: `R□·L/W` per metal shape plus one
    /// cut's R per cut (Lampaert 1999 eq. 2.33; Hastings eq. 2.33).
    ///
    /// ponytail: summed over the whole net, not solved per terminal path — a
    /// branched net's sum bounds every path's R; a mesh needs a network solve.
    #[must_use]
    pub fn resistance_ohm(&self, shapes: &[Shape]) -> f32 {
        shapes
            .iter()
            .filter_map(|s| {
                let (_, l) = self.at(s.layer.0)?;
                let (w, len) = (s.rect.w.min(s.rect.h).max(1) as f32, s.rect.w.max(s.rect.h) as f32);
                Some(if l.cut { l.sheet_ohm } else { l.sheet_ohm * len / w })
            })
            .sum()
    }

    /// Series R of the most resistive path through the net, Ω: each shape is a
    /// node weighted by its own R, touching shapes (same layer, or a cut and
    /// the metals either side) are joined, and the heaviest simple path of a
    /// spanning tree is found by a double sweep (the tree's weighted diameter).
    /// No terminal path is longer, so this bounds any terminal-to-terminal R —
    /// without charging every branch of a rail to each path as the sum does.
    ///
    /// ponytail: a whole shape counts even when the path uses part of it, and
    /// a mesh is cut to its BFS tree; the terminal-resolved drop needs the
    /// terminals and a network solve. O(k²) in shapes.
    #[must_use]
    pub fn path_resistance_ohm(&self, shapes: &[Shape]) -> f32 {
        let nodes: Vec<(usize, Rect, f32)> = shapes
            .iter()
            .filter_map(|s| {
                let (rank, l) = self.at(s.layer.0)?;
                let (w, len) = (s.rect.w.min(s.rect.h).max(1) as f32, s.rect.w.max(s.rect.h) as f32);
                Some((rank, s.rect, if l.cut { l.sheet_ohm } else { l.sheet_ohm * len / w }))
            })
            .collect();
        if nodes.is_empty() {
            return 0.0;
        }
        let touch = |a: &Rect, b: &Rect| a.x <= b.x + b.w && b.x <= a.x + a.w && a.y <= b.y + b.h && b.y <= a.y + a.h;
        let adj: Vec<Vec<usize>> = (0..nodes.len())
            .map(|i| (0..nodes.len()).filter(|&j| j != i && nodes[i].0.abs_diff(nodes[j].0) <= 1 && touch(&nodes[i].1, &nodes[j].1)).collect())
            .collect();
        // BFS tree from `root`, then the heaviest root-to-node path in it.
        let farthest = |root: usize| -> (usize, f32) {
            let mut dist = vec![f32::NAN; nodes.len()];
            dist[root] = nodes[root].2;
            let mut queue = std::collections::VecDeque::from([root]);
            while let Some(a) = queue.pop_front() {
                for &b in &adj[a] {
                    if dist[b].is_nan() {
                        dist[b] = dist[a] + nodes[b].2;
                        queue.push_back(b);
                    }
                }
            }
            dist.iter().enumerate().filter(|(_, d)| !d.is_nan()).fold((root, 0.0), |m, (i, &d)| if d > m.1 { (i, d) } else { m })
        };
        // Every connected piece: an open net's pieces each bound their own paths.
        let (mut worst, mut seen) = (0.0f32, vec![false; nodes.len()]);
        for start in 0..nodes.len() {
            if seen[start] {
                continue;
            }
            let (u, _) = farthest(start);
            let (_, d) = farthest(u);
            worst = worst.max(d);
            let mut stack = vec![start];
            seen[start] = true;
            while let Some(a) = stack.pop() {
                for &b in &adj[a] {
                    if !seen[b] {
                        seen[b] = true;
                        stack.push(b);
                    }
                }
            }
        }
        worst
    }

    /// Series R from the net's centre to each terminal, Ω ([`Stack::port_graph`]):
    /// the centre is the port whose farthest terminal is nearest, a star
    /// model of the routed tree (two terminals' branches sum to at least the
    /// path between them). `None` for a terminal no shape reaches.
    #[must_use]
    pub fn terminal_resistance_ohm(&self, shapes: &[Shape], terminals: &[Rect]) -> Vec<Option<f32>> {
        let g = self.port_graph(shapes, terminals, false);
        let reach = |r: &[Option<f32>]| r.iter().flatten().copied().fold(0.0f32, f32::max);
        (0..g.adj.len())
            .map(|c| g.from(&[c]))
            .filter(|r| r.iter().any(Option::is_some))
            .min_by(|a, b| reach(a).total_cmp(&reach(b)))
            .unwrap_or_else(|| vec![None; terminals.len()])
    }

    /// Series R from the nearest of `feeds` (where current enters: a tail
    /// device's drain, a rail's port) to each terminal, Ω. `None` for a
    /// terminal unreached, or every one when no feed is reached.
    #[must_use]
    pub fn fed_resistance_ohm(&self, shapes: &[Shape], feeds: &[Rect], terminals: &[Rect]) -> Vec<Option<f32>> {
        let all: Vec<Rect> = terminals.iter().chain(feeds).copied().collect();
        let g = self.port_graph(shapes, &all, false);
        let src: Vec<usize> = g.term[terminals.len()..].iter().flatten().copied().collect();
        if src.is_empty() {
            return vec![None; terminals.len()];
        }
        let mut out = g.from(&src);
        out.truncate(terminals.len());
        out
    }

    /// The routed net as a resistor graph: on each metal shape, a port where
    /// it touches another shape and where a terminal sits over it, joined
    /// along the shape by `R□·Δ/w` (distance along its long axis); a cut is
    /// one node reached through half its R from each metal it joins. Each
    /// link records the index into `shapes` it runs through (`u32::MAX` for a
    /// 0-Ω junction or terminal link). A terminal joins every shape it
    /// overlaps in xy, or with `lowest` only those on the lowest stack rank it
    /// overlaps: a pin is drawn on one layer, and what sits over it higher up
    /// reaches it through the cuts, which then carry its current.
    pub(crate) fn port_graph(&self, shapes: &[Shape], terminals: &[Rect], lowest: bool) -> PortGraph {
        let items: Vec<(usize, Rect, &Layer, u32)> =
            shapes.iter().enumerate().filter_map(|(i, s)| self.at(s.layer.0).map(|(k, l)| (k, s.rect, l, i as u32))).collect();
        let touch = |a: &Rect, b: &Rect| a.x <= b.x + b.w && b.x <= a.x + a.w && a.y <= b.y + b.h && b.y <= a.y + a.h;
        let centre = |a: &Rect, b: &Rect| {
            let (x0, x1) = (a.x.max(b.x), (a.x + a.w).min(b.x + b.w));
            let (y0, y1) = (a.y.max(b.y), (a.y + a.h).min(b.y + b.h));
            ((x0 + x1) / 2, (y0 + y1) / 2)
        };
        let mut g = PortGraph { adj: Vec::new(), term: vec![None; terminals.len()] };
        // Per shape: its ports `(position along the axis, node)`.
        let mut ports: Vec<Vec<(i32, usize)>> = vec![Vec::new(); items.len()];
        let along = |r: &Rect, (x, y): (i32, i32)| if r.w >= r.h { x } else { y };
        let node = |g: &mut PortGraph| {
            g.adj.push(Vec::new());
            g.adj.len() - 1
        };
        // Cuts are single nodes.
        let cut_node: Vec<Option<usize>> = items.iter().map(|(_, _, l, _)| l.cut.then(|| node(&mut g))).collect();
        for i in 0..items.len() {
            for j in i + 1..items.len() {
                let ((ri, a, la, si), (rj, b, lb, sj)) = (items[i], items[j]);
                if ri.abs_diff(rj) > 1 || !touch(&a, &b) {
                    continue;
                }
                let at = centre(&a, &b);
                match (cut_node[i], cut_node[j]) {
                    (Some(_), Some(_)) => {}
                    (Some(c), None) | (None, Some(c)) => {
                        let (m, cut, sc) = if la.cut { (j, la, si) } else { (i, lb, sj) };
                        let p = node(&mut g);
                        ports[m].push((along(&items[m].1, at), p));
                        g.link(p, c, cut.sheet_ohm / 2.0, sc);
                    }
                    (None, None) => {
                        let (p, q) = (node(&mut g), node(&mut g));
                        ports[i].push((along(&a, at), p));
                        ports[j].push((along(&b, at), q));
                        g.link(p, q, 0.0, u32::MAX);
                    }
                }
            }
        }
        for (t, r) in terminals.iter().enumerate() {
            let c = (r.x + r.w / 2, r.y + r.h / 2);
            let floor = if lowest { items.iter().filter(|(_, s, ..)| touch(r, s)).map(|it| it.0).min() } else { None };
            for (i, &(k, ref s, ..)) in items.iter().enumerate() {
                if !touch(r, s) || floor.is_some_and(|f| k != f) {
                    continue;
                }
                let n = cut_node[i].unwrap_or_else(|| {
                    let p = node(&mut g);
                    ports[i].push((along(s, c), p));
                    p
                });
                match g.term[t] {
                    Some(prev) => g.link(prev, n, 0.0, u32::MAX),
                    None => g.term[t] = Some(n),
                }
            }
        }
        for (i, &(_, s, l, si)) in items.iter().enumerate() {
            let p = &mut ports[i];
            p.sort_unstable();
            let w = s.w.min(s.h).max(1) as f32;
            for k in 1..p.len() {
                g.link(p[k - 1].1, p[k].1, l.sheet_ohm * (p[k].0 - p[k - 1].0) as f32 / w, si);
            }
        }
        g
    }

    /// Lateral coupling between two same-layer shapes, aF: `ε0·k·t·run/gap`
    /// over their parallel run; `None` when the layer's `ε·t` is unknown.
    #[must_use]
    pub fn lateral_af(&self, layer: u16, p: &Rect, q: &Rect) -> Option<f32> {
        let (_, l) = self.at(layer).filter(|(_, l)| l.lateral > 0.0)?;
        Some(parallel(p, q).map_or(0.0, |(run, gap)| l.lateral * run as f32 / 1e3 / gap.max(1) as f32))
    }

    /// `shapes` on this stack in connected pieces (same or adjacent layer,
    /// touching), as indices into `shapes`; off-stack shapes are in none.
    #[must_use]
    pub fn connected(&self, shapes: &[Shape]) -> Vec<Vec<usize>> {
        let on: Vec<usize> = (0..shapes.len()).filter(|&i| self.at(shapes[i].layer.0).is_some()).collect();
        let built: Vec<(usize, Rect)> = on.iter().map(|&i| (self.at(shapes[i].layer.0).map_or(0, |(r, _)| r), shapes[i].rect)).collect();
        pieces(&built).into_iter().map(|p| p.into_iter().map(|k| on[k]).collect()).collect()
    }

    /// Worst antenna ratio over every etch stage that has a limit, as
    /// `(ratio, limit)` of the stage with the largest `ratio/limit`; `None`
    /// when no stage has one. At stage `s` only layers up to `s` exist, so the
    /// conductor is each connected piece of them (Hastings pp. 228–229); its
    /// exposed area is layer `s`'s alone, or every layer's up to `s` when the
    /// deck's rules are cumulative. `cell` is the cells' metal on the net,
    /// scored with the wires; with `gates` (the net's gate pins) only a piece
    /// reaching one is charged — a piece a jumper cut off the gate carries no
    /// charge to it.
    ///
    /// ponytail: a charged piece is charged the net's whole gate area, which
    /// under-counts a piece that reaches only some of several gates. O(k²) per
    /// stage.
    #[must_use]
    pub fn antenna(&self, shapes: &[Shape], cell: &[Shape], gates: &[Rect], gate_nm2: i64) -> Option<(f32, f32)> {
        let all: Vec<&Shape> = shapes.iter().chain(cell).collect();
        // A junction to the substrate bleeds the plasma charge at every stage.
        // ponytail: credited from the first stage on; a diode reached only
        // through an upper metal protects only from that stage.
        if self.diode_layer.is_some_and(|d| all.iter().any(|s| s.layer.0 == d)) {
            return self.layers.iter().find(|l| l.antenna_ratio > 0.0).map(|l| (0.0, l.antenna_ratio));
        }
        let rank = |s: &Shape| self.at(s.layer.0).map(|(i, _)| i);
        let mut worst: Option<(f32, f32)> = None;
        for (stage, layer) in self.layers.iter().enumerate().filter(|(_, l)| l.antenna_ratio > 0.0) {
            let built: Vec<(usize, Rect)> = all.iter().filter_map(|s| Some((rank(s)?, s.rect))).filter(|&(r, _)| r <= stage).collect();
            // Cumulative: the stage's own kind below it too (metals with metals,
            // cuts with cuts — they alternate in the stack).
            let counts = |r: usize| r == stage || (self.antenna_cumulative && (stage - r) % 2 == 0);
            let exposed = |q: Rect| match layer.antenna_sidewall_nm {
                t if t > 0.0 => 2.0 * (q.w + q.h) as f32 * t,
                _ => q.w as f32 * q.h as f32,
            };
            for piece in pieces(&built) {
                // A piece reaches a gate when it lies over the gate's pin (the
                // router's trunk ends there before its pin access is drawn).
                if !gates.is_empty() && !piece.iter().any(|&k| gates.iter().any(|g| touches(&built[k].1, g))) {
                    continue;
                }
                let area: f32 = piece.iter().map(|&k| built[k]).filter(|&(r, _)| counts(r)).map(|(_, q)| exposed(q)).sum();
                let ratio = area / gate_nm2.max(1) as f32;
                if worst.is_none_or(|(r, l)| ratio / layer.antenna_ratio > r / l) {
                    worst = Some((ratio, layer.antenna_ratio));
                }
            }
        }
        worst
    }
}

/// [`Stack::port_graph`]'s resistor network: adjacency `(node, Ω, shape
/// index)` (`u32::MAX`: a 0-Ω junction or terminal link) and each terminal's
/// node.
pub(crate) struct PortGraph {
    pub(crate) adj: Vec<Vec<(usize, f32, u32)>>,
    pub(crate) term: Vec<Option<usize>>,
}

impl PortGraph {
    fn link(&mut self, a: usize, b: usize, r: f32, shape: u32) {
        self.adj[a].push((b, r, shape));
        self.adj[b].push((a, r, shape));
    }

    /// Least R from `src` to each terminal.
    fn from(&self, src: &[usize]) -> Vec<Option<f32>> {
        let dist = self.dijkstra(src).0;
        self.term.iter().map(|t| t.map(|n| dist[n]).filter(|d| d.is_finite())).collect()
    }

    /// The parent link `(parent, Ω, shape index)` of every node a least-R
    /// tree from `root` reaches; `None` for the root and unreached nodes.
    pub(crate) fn tree(&self, root: usize) -> Vec<Option<(usize, f32, u32)>> {
        self.dijkstra(&[root]).1
    }

    /// Least R from the nearest of `src` to every node, and each reached
    /// node's parent link on that path.
    #[allow(clippy::type_complexity)]
    fn dijkstra(&self, src: &[usize]) -> (Vec<f32>, Vec<Option<(usize, f32, u32)>>) {
        let mut dist = vec![f32::INFINITY; self.adj.len()];
        let mut parent = vec![None; self.adj.len()];
        let mut heap = std::collections::BinaryHeap::new();
        for &s in src {
            dist[s] = 0.0;
            heap.push(std::cmp::Reverse((0u32, s)));
        }
        while let Some(std::cmp::Reverse((d, a))) = heap.pop() {
            if f32::from_bits(d) > dist[a] {
                continue;
            }
            for &(b, r, shape) in &self.adj[a] {
                let nd = dist[a] + r;
                if nd < dist[b] {
                    dist[b] = nd;
                    parent[b] = Some((a, r, shape));
                    heap.push(std::cmp::Reverse((nd.to_bits(), b)));
                }
            }
        }
        (dist, parent)
    }
}

/// `(run, gap)` of two shapes separated on one axis and overlapping on the
/// other; `None` otherwise.
#[must_use]
pub fn parallel(p: &Rect, q: &Rect) -> Option<(i32, i32)> {
    let gap_x = (q.x - (p.x + p.w)).max(p.x - (q.x + q.w));
    let gap_y = (q.y - (p.y + p.h)).max(p.y - (q.y + q.h));
    let run_x = (p.x + p.w).min(q.x + q.w) - p.x.max(q.x);
    let run_y = (p.y + p.h).min(q.y + q.h) - p.y.max(q.y);
    if gap_x > 0 && run_y > 0 {
        Some((run_y, gap_x))
    } else if gap_y > 0 && run_x > 0 {
        Some((run_x, gap_y))
    } else {
        None
    }
}

/// Connected pieces of `(stack rank, rect)`: touching rects on the same or
/// adjacent ranks (a cut joins the metals either side) are one conductor.
fn touches(a: &Rect, b: &Rect) -> bool {
    a.x <= b.x + b.w && b.x <= a.x + a.w && a.y <= b.y + b.h && b.y <= a.y + a.h
}

fn pieces(built: &[(usize, Rect)]) -> Vec<Vec<usize>> {
    let touch = touches;
    let mut seen = vec![false; built.len()];
    let mut out = Vec::new();
    for s in 0..built.len() {
        if seen[s] {
            continue;
        }
        seen[s] = true;
        let (mut piece, mut stack) = (Vec::new(), vec![s]);
        while let Some(a) = stack.pop() {
            piece.push(a);
            for b in 0..built.len() {
                if !seen[b] && built[a].0.abs_diff(built[b].0) <= 1 && touch(&built[a].1, &built[b].1) {
                    seen[b] = true;
                    stack.push(b);
                }
            }
        }
        out.push(piece);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::geom::LayerId;

    fn shape(layer: u16, x: i32, y: i32, w: i32, h: i32) -> Shape {
        Shape { layer: LayerId(layer), rect: Rect { x, y, w, h } }
    }

    /// m1 (id 1), via (2), m2 (3); the via stage has no antenna rule.
    fn stack(m1: f32, m2: f32, cumulative: bool) -> Stack {
        let metal = |id, ratio| Layer { id, area_af_um2: 25.0, fringe_af_um: 40.0, lateral: 3.9 * 8.854 * 360.0, antenna_ratio: ratio, sheet_ohm: 0.125, ..Layer::default() };
        Stack { layers: vec![metal(1, m1), Layer { id: 2, sheet_ohm: 4.5, cut: true, ..Layer::default() }, metal(3, m2)], antenna_cumulative: cumulative, diode_layer: None }
    }

    #[test]
    fn ground_c_is_area_plus_fringe_per_layer() {
        // 10 µm × 0.5 µm on m1: 25·5 + 2·40·10 = 925 aF; an unknown layer adds 0.
        let c = stack(0.0, 0.0, false).ground_af(&[shape(1, 0, 0, 10_000, 500), shape(9, 0, 0, 10_000, 500)]);
        assert!((c - 925.0).abs() < 1e-3, "{c}");
    }

    #[test]
    fn resistance_is_squares_times_sheet_plus_cuts() {
        // 10 µm × 0.5 µm on m1 = 20 □ · 0.125 = 2.5 Ω; two cuts at 4.5 Ω.
        let r = stack(0.0, 0.0, false).resistance_ohm(&[shape(1, 0, 0, 10_000, 500), shape(2, 0, 0, 170, 170), shape(2, 500, 0, 170, 170)]);
        assert!((r - 11.5).abs() < 1e-4, "{r}");
    }

    /// A rail with branches: the worst path runs trunk + one branch, not the
    /// sum of every branch.
    #[test]
    fn terminal_resistance_is_each_branch_from_the_centre() {
        let s = stack(0.0, 0.0, false);
        // The same rail: stub tips are the terminals.
        let mut rail = vec![shape(1, 0, 0, 10_000, 500)];
        for x in [1_000, 5_000, 9_000] {
            rail.push(shape(1, x, 500, 500, 4_000));
        }
        let tip = |x: i32| Rect { x: x + 200, y: 4_400, w: 100, h: 100 };
        let off = Rect { x: 50_000, y: 0, w: 10, h: 10 };
        let r = s.terminal_resistance_ohm(&rail, &[tip(1_000), tip(5_000), tip(9_000), off]);
        // Tips are 3.95 µm up their stubs (0.9875 Ω); the joints 4 µm apart
        // on the trunk (1 Ω): centred on the middle joint.
        assert!((r[1].unwrap() - 0.9875).abs() < 1e-3, "{r:?}");
        assert!((r[0].unwrap() - 1.9875).abs() < 1e-3 && (r[2].unwrap() - 1.9875).abs() < 1e-3, "{r:?}");
        assert_eq!(r[3], None, "a terminal no shape reaches");
    }

    #[test]
    fn path_resistance_is_the_worst_path_not_the_sum() {
        let s = stack(0.0, 0.0, false);
        // Trunk 10 µm × 0.5 µm (2.5 Ω) with three 4 µm × 0.5 µm stubs (1 Ω each).
        let mut rail = vec![shape(1, 0, 0, 10_000, 500)];
        for x in [1_000, 5_000, 9_000] {
            rail.push(shape(1, x, 500, 500, 4_000));
        }
        assert!((s.resistance_ohm(&rail) - 5.5).abs() < 1e-3, "the sum charges all three stubs");
        let p = s.path_resistance_ohm(&rail);
        assert!((p - 4.5).abs() < 1e-3, "stub + trunk + stub: {p}");
    }

    #[test]
    fn lateral_c_falls_off_with_the_gap() {
        let s = stack(0.0, 0.0, false);
        let a = Rect { x: 0, y: 0, w: 10_000, h: 200 };
        let near = s.lateral_af(1, &a, &Rect { y: 400, ..a }).unwrap();
        let far = s.lateral_af(1, &a, &Rect { y: 1_000, ..a }).unwrap();
        assert!((near / far - 4.0).abs() < 1e-3, "1/gap: {near} vs {far}");
        // ε0·k·t ≈ 12 431 aF·nm/µm: 10 µm of run 200 nm apart ≈ 622 aF.
        assert!((near - 621.6).abs() < 1.0, "{near}");
        assert_eq!(s.lateral_af(2, &a, &a), None, "a cut has no ε·t here");
    }

    /// Hastings' worked example: a long m1 run fails at the m1 stage; a jumper
    /// through m2 splits it, so at the m1 stage each piece is short; the m2
    /// stage still checks the whole net.
    #[test]
    fn a_jumper_splits_the_lower_stage() {
        let s = stack(100.0, 400.0, false);
        let gate = 1_000_000; // 1 µm²
        // One 180 µm × 1 µm m1 run: ratio 180 at the m1 stage (limit 100).
        let long = [shape(1, 0, 0, 180_000, 1_000)];
        assert_eq!(s.antenna(&long, &[], &[], gate), Some((180.0, 100.0)));
        // The same length as 20 µm m1 + a 140 µm m2 bridge + 20 µm m1.
        let bridged = [
            shape(1, 0, 0, 20_000, 1_000),
            shape(2, 19_000, 0, 1_000, 1_000),
            shape(3, 19_000, 0, 142_000, 1_000),
            shape(2, 160_000, 0, 1_000, 1_000),
            shape(1, 160_000, 0, 20_000, 1_000),
        ];
        let (ratio, limit) = s.antenna(&bridged, &[], &[], gate).unwrap();
        assert!(ratio / limit < 1.0, "bridged: {ratio}/{limit}");
        // m1 stage: 20 each; m2 stage: 142 of 400 — the m2 stage is worst.
        assert_eq!(limit, 400.0);
        // Cumulative rules count m1 (not the cuts) at the m2 stage too: 142 + 40.
        let cum = stack(100.0, 400.0, true).antenna(&bridged, &[], &[], gate).unwrap();
        assert!((cum.0 - 182.0).abs() < 1e-3 && cum.1 == 400.0, "{cum:?}");
        // A diode on the net (its marker among the net's shapes) is credited.
        let guarded = Stack { diode_layer: Some(9), ..stack(100.0, 400.0, false) };
        let mut with_diode = long.to_vec();
        with_diode.push(shape(9, 0, 0, 500, 500));
        assert_eq!(guarded.antenna(&with_diode, &[], &[], gate), Some((0.0, 100.0)));
        assert_eq!(guarded.antenna(&long, &[], &[], gate), Some((180.0, 100.0)), "no diode, no credit");
        // A sidewall rule counts perimeter × thickness: 2·(180+1) µm · 0.36 µm.
        let mut side = stack(100.0, 400.0, false);
        side.layers[0].antenna_sidewall_nm = 360.0;
        assert!((side.antenna(&long, &[], &[], gate).unwrap().0 - 130.32).abs() < 1e-2);
    }

    /// A cell's plate on the net counts with the wires; once a jumper cuts it
    /// off the gate at a stage, it charges nothing there.
    #[test]
    fn a_cell_plate_counts_until_cut_off_the_gate() {
        let s = stack(100.0, 400.0, false);
        let gate = 1_000_000;
        let pin = Rect { x: 0, y: 0, w: 1_000, h: 1_000 };
        // A short m1 wire from the gate to a 150 µm² m1 plate: 151 > 100.
        let wire = [shape(1, 0, 0, 2_000, 1_000)];
        let plate = [shape(1, 2_000, 0, 150_000, 1_000)];
        assert_eq!(s.antenna(&wire, &plate, &[pin], gate).map(|w| w.0), Some(152.0));
        // Jumped through m2: at the m1 stage the plate is its own piece, off
        // the gate, and only the wire stub is charged.
        let jumped = [shape(1, 0, 0, 1_000, 1_000), shape(2, 500, 0, 500, 1_000), shape(3, 500, 0, 2_500, 1_000), shape(2, 2_500, 0, 500, 1_000)];
        let (ratio, limit) = s.antenna(&jumped, &plate, &[pin], gate).unwrap();
        assert!(ratio / limit < 1.0, "{ratio}/{limit}");
    }
}
