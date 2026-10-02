//! DC electromigration (routing tier, hard), plus the per-layer limit model
//! the detailed router sizes segments and via arrays with.

use pnr_core::geom::Shape;
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use crate::rule::Rule;
use super::current::net_flow;
use super::Stack;

/// Boltzmann's constant, eV/K.
const K_EV: f32 = 8.617e-5;

/// Allowed-current factor for a limit rated at `t_ref_k`, at conductor
/// temperature `t_k`: `F = exp[(Ea/(n·k))·(1/T − 1/T_ref)]`, `< 1` when hotter.
/// Hastings eqs. 15.24–15.25; Lienig & Thiele 2018 eq. 3.11 (from Black, eq. 3.8).
/// Never credits a conductor cooler than the rating: `F ≤ 1`.
#[must_use]
pub fn derate(t_k: f32, t_ref_k: f32, ea_ev: f32, n: f32) -> f32 {
    if t_k <= t_ref_k || n <= 0.0 {
        return 1.0;
    }
    ((ea_ev / (n * K_EV)) * (1.0 / t_k - 1.0 / t_ref_k)).exp()
}

/// One layer's deck EM limits, already derated to the conductor temperature.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Limit {
    /// DC current per µm of width, µA/µm (the PDK's unit carries thickness);
    /// `0` = unknown.
    pub ua_per_um: f32,
    /// DC current per cut, µA; `0` = unknown.
    pub ua_per_cut: f32,
    /// Blech product `(jL)_B` in (µA/µm)·µm; `0` = the deck has none.
    pub blech: f32,
}

impl Limit {
    /// Scale both current limits by a [`derate`] factor.
    #[must_use]
    pub fn derated(self, f: f32) -> Self {
        Self { ua_per_um: self.ua_per_um * f, ua_per_cut: self.ua_per_cut * f, ..self }
    }

    /// Width a segment carrying `i_ua` needs, nm: `w ≥ I/J` (Lienig & Thiele
    /// 2018 eq. 3.21; Hastings eq. 15.24) — unless it is Blech-immortal,
    /// `(I/w)·L < (jL)_B` (Lienig eq. 4.1), i.e. `w > I·L/(jL)_B`. `domain_nm`
    /// is the diffusion domain's length (`dr` passes the net's whole run on
    /// the layer, [`Electromigration`] the shape's long side). `0` when the
    /// limit is unknown.
    #[must_use]
    pub fn width_nm(self, i_ua: f32, domain_nm: f32) -> f32 {
        if self.ua_per_um <= 0.0 {
            return 0.0;
        }
        let density = i_ua.abs() * 1_000.0 / self.ua_per_um;
        if self.blech > 0.0 {
            density.min(i_ua.abs() * domain_nm / self.blech)
        } else {
            density
        }
    }

    /// Cuts a via carrying `i_ua` needs: `n = ⌈I / I_cut⌉` (Lienig & Thiele 2018
    /// eq. 3.25; its temperature factor f(T) is divided out here, via
    /// [`Limit::derated`] — the book prints it multiplied, which would let a
    /// hotter via have fewer cuts). `1` when the limit is unknown.
    #[must_use]
    pub fn cuts(self, i_ua: f32) -> u32 {
        if self.ua_per_cut <= 0.0 {
            return 1;
        }
        ((i_ua.abs() / self.ua_per_cut).ceil() as u32).max(1)
    }
}

/// Most layers (metals and cuts) a rule carries limits for.
pub const MAX_LAYERS: usize = 16;

/// Every routed segment and via of `net` carries its DC current at its
/// layer's limit: Lienig & Thiele 2018 eq. 3.21 (`w ≥ I/J`) per metal shape
/// and eq. 3.25 (`n ≥ ⌈I/I_cut⌉`) per via group; "a lead must meet
/// electromigration rules at every point along its length" (Hastings
/// §15.4.4), so a narrow access jog fails even beside a wide trunk. Currents
/// come from [`net_flow`] over the routed shapes, the cells' metal (a
/// terminal joined only through its cell's strap stays reached) and the
/// routed terminals ([`Routes::terminals`]); only the routed shapes are
/// checked (cell straps are the cell's).
///
/// The Blech domain is a shape's own long side.
/// [UNVERIFIED: this bounds the Lienig segment (§4.3.1: a segment ends only
/// at vias or branches) only if no two same-net, same-layer shapes abut
/// collinearly; such a pair is one longer segment. Dormant: no shipped deck
/// supplies `blech_limit`.]
///
/// Unknown (not passed) without the stack, with a terminal current unknown,
/// an open terminal, or nothing routed on a limited layer. A known **zero**
/// current is a real pass.
///
/// ponytail: DC average only; RMS/peak need waveforms.
#[derive(Clone, Copy)]
pub struct Electromigration {
    pub net: NetId,
    /// `(layer, limit)` per metal **and cut** the deck limits (pin-access
    /// layer and cut included); unused slots `u16::MAX`.
    pub limits: [(u16, Limit); MAX_LAYERS],
    pub stack: Option<&'static Stack>,
}

impl Electromigration {
    fn limit(self, layer: u16) -> Option<Limit> {
        self.limits.iter().find(|(l, _)| *l == layer).map(|&(_, lim)| lim)
    }

    /// `(worst residual, worst need/have)` over the routed metal shapes and
    /// via groups on limited layers; a via's need/have is `I/(I_cut·n)`, so
    /// both read `≤ 1` together exactly when every check passes. `None` when
    /// unknown (see the type).
    fn check(self, r: &Routes) -> Option<(f32, f32)> {
        let stack = self.stack?;
        let routed = r.shapes(self.net);
        let all = [routed, r.cell_metal(self.net)].concat();
        let flow = net_flow(stack, &all, r.terminals(self.net))?;
        let (mut worst, mut checked) = ((0.0f32, 0.0f32), false);
        let mut fold = |need: f32, have: f32, ratio: f32| {
            checked = true;
            worst = (worst.0.max(crate::rule::over(need - have, need)), worst.1.max(ratio));
        };
        for (s, &ua) in routed.iter().zip(&flow.shape_ua) {
            let Some(lim) = self.limit(s.layer.0).filter(|l| l.ua_per_um > 0.0) else { continue };
            let have = s.rect.w.min(s.rect.h).max(1) as f32;
            let need = lim.width_nm(ua, s.rect.w.max(s.rect.h) as f32);
            fold(need, have, need / have);
        }
        // Via groups: cuts of one layer in parallel between the same metals,
        // i.e. landing on a common shape below and a common shape above (by
        // index into `all`), share the group's current.
        let rank = |l: u16| stack.layers.iter().position(|x| x.id == l);
        let lands = |c: &Shape, side: usize| -> Vec<usize> {
            (0..all.len()).filter(|&m| rank(all[m].layer.0) == Some(side) && all[m].rect.touches(&c.rect)).collect()
        };
        let cuts: Vec<(usize, Limit, Vec<usize>, Vec<usize>)> = routed
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let lim = self.limit(c.layer.0).filter(|l| l.ua_per_cut > 0.0)?;
                // A metal's limit keeps its deck rule's per-cut figure (`Pdk::em_limit`
                // zeroes only a cut's per-µm one): only a cut is a via.
                let k = rank(c.layer.0).filter(|&k| stack.layers[k].cut)?;
                Some((i, lim, lands(c, k.wrapping_sub(1)), lands(c, k + 1)))
            })
            .collect();
        // Any shared shape, not an identical landing set: `dr`'s array cuts
        // spread past the original cut's pads, so each cut lands on its own
        // pad plus the common trunk.
        // ponytail: transitive, so a cut over two unjoined same-layer shapes
        // merges the cuts under each into one group (max I, n = all): not
        // conservative there. Needs same-net shapes closer than a cut is wide;
        // split groups per landing-shape pair if dr ever draws that.
        let shares = |a: &[usize], b: &[usize]| a.iter().any(|m| b.contains(m));
        let mut uf = pnr_core::UnionFind::new(cuts.len());
        for i in 0..cuts.len() {
            for j in i + 1..cuts.len() {
                let (p, q) = (&cuts[i], &cuts[j]);
                if routed[p.0].layer == routed[q.0].layer && shares(&p.2, &q.2) && shares(&p.3, &q.3) {
                    uf.union(i as u32, j as u32);
                }
            }
        }
        // Per group root: (cuts, largest member current).
        let mut group = vec![(0u32, 0.0f32); cuts.len()];
        for (i, cut) in cuts.iter().enumerate() {
            let g = &mut group[uf.find(i as u32) as usize];
            *g = (g.0 + 1, g.1.max(flow.shape_ua[cut.0]));
        }
        for (i, &(have, ua)) in group.iter().enumerate().filter(|(_, g)| g.0 > 0) {
            let lim = cuts[i].1;
            fold(lim.cuts(ua) as f32, have as f32, ua / (lim.ua_per_cut * have as f32));
        }
        checked.then_some(worst)
    }
}

impl Rule for Electromigration {
    type On = Routes;
    const REPAIR: crate::RepairKind = crate::RepairKind::Em;
    fn cost(self, r: &Routes) -> f32 {
        self.residual(r)
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.check(r).is_none_or(|(res, _)| res <= 0.0)
    }
    fn known(self, r: &Routes) -> bool {
        self.check(r).is_some()
    }
    /// Worst `(need − have)/need` over the checked shapes and via groups.
    fn residual(self, r: &Routes) -> f32 {
        self.check(r).map_or(0.0, |(res, _)| res)
    }
    /// `1 − max(need/have)`; an unknown puts no pressure.
    fn headroom(self, r: &Routes) -> f32 {
        self.check(r).map_or(1.0, |(_, q)| 1.0 - q)
    }
    /// Worst `need/have` (`1.0` = at the limit; T3's `min(w/need)` inverted).
    fn usage(self, r: &Routes) -> Option<f32> {
        self.check(r).map(|(_, q)| q)
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::current::net_flow;
    use crate::routing::stack::Layer;
    use pnr_core::geom::{LayerId, Rect};
    use pnr_core::routes::Terminal;

    /// met1 (id 1, 0.125 Ω/□), via (2, 4.5 Ω/cut), met2 (3).
    fn stack() -> &'static Stack {
        let metal = |id| Layer { id, sheet_ohm: 0.125, ..Layer::default() };
        Box::leak(Box::new(Stack { layers: vec![metal(1), Layer { id: 2, sheet_ohm: 4.5, cut: true, ..Layer::default() }, metal(3)], ..Stack::default() }))
    }

    /// The sky130 values (`pdks/decks/sky130.deck:496-497`) as `em_limits`
    /// copies them: met1 2800 µA/µm keeping `EM.met1_mcon`'s 360 µA/cut, via
    /// 290 µA/cut, met2 2800 µA/µm keeping `EM.met2_via1`'s 290 µA/cut.
    fn em() -> Electromigration {
        let mut limits = [(u16::MAX, Limit::default()); MAX_LAYERS];
        limits[0] = (1, Limit { ua_per_um: 2_800.0, ua_per_cut: 360.0, ..Limit::default() });
        limits[1] = (2, Limit { ua_per_cut: 290.0, ..Limit::default() });
        limits[2] = (3, Limit { ua_per_um: 2_800.0, ua_per_cut: 290.0, ..Limit::default() });
        Electromigration { net: NetId(0), limits, stack: Some(stack()) }
    }

    fn shape(layer: u16, x: i32, y: i32, w: i32, h: i32) -> Shape {
        Shape { layer: LayerId(layer), rect: Rect { x, y, w, h } }
    }

    fn term(x: i32, y: i32, w: i32, h: i32, ua: Option<f32>) -> Terminal {
        Terminal { at: Rect { x, y, w, h }, ua }
    }

    fn routes(wires: Vec<Shape>, terms: Vec<Terminal>) -> Routes {
        Routes { wires: vec![wires], terms: vec![terms], ..Default::default() }
    }

    fn need(ua: f32) -> f32 {
        em().limit(1).unwrap().width_nm(ua, 0.0)
    }

    /// A 1 µm met1 trunk and a 140 nm × 400 nm jog up to B.
    fn jog(b_ua: Option<f32>) -> Routes {
        routes(
            vec![shape(1, 0, 0, 10_000, 1_000), shape(1, 9_000, 1_000, 140, 400)],
            vec![term(0, 400, 200, 200, Some(500.0)), term(9_000, 1_300, 140, 100, b_ua)],
        )
    }

    #[test]
    fn a_narrow_access_jog_fails_even_if_the_trunk_is_wide() {
        let r = jog(Some(-500.0));
        // need = 500·1000/2800 = 178.6 nm: the trunk has it, the jog does not.
        assert!((need(500.0) - 178.6).abs() < 0.05 && need(500.0) <= 1_000.0, "the old best-segment rule passes on the trunk");
        let e = em();
        assert!(e.known(&r) && !e.satisfied(&r));
        assert!((e.residual(&r) - (178.57 - 140.0) / 178.57).abs() < 1e-3, "{}", e.residual(&r));
        assert!((e.residual(&r) - 0.216).abs() < 1e-3);
    }

    #[test]
    fn a_trunk_carries_the_sum_of_its_branches() {
        // Root sink at the trunk's left end; branches up at x = 4 µm (+300)
        // and 8 µm (+200).
        let wires = vec![shape(1, 0, 0, 10_000, 1_000), shape(1, 4_000, 1_000, 500, 3_000), shape(1, 8_000, 1_000, 500, 3_000)];
        let terms = vec![term(0, 400, 200, 200, Some(-500.0)), term(4_000, 3_900, 500, 100, Some(300.0)), term(8_000, 3_900, 500, 100, Some(200.0))];
        let flow = net_flow(stack(), &wires, &terms).unwrap();
        let needs: Vec<f32> = flow.shape_ua.iter().map(|&i| need(i)).collect();
        assert!((needs[0] - 178.6).abs() < 0.05, "trunk carries 300 + 200: {needs:?}");
        assert!((needs[1] - 107.1).abs() < 0.05 && (needs[2] - 71.4).abs() < 0.05, "{needs:?}");
        assert!(em().known(&routes(wires.clone(), terms.clone())) && em().satisfied(&routes(wires, terms)));
    }

    #[test]
    fn a_via_group_needs_ceil_i_over_i_cut() {
        // met1 → two via cuts → met2, 700 µA end to end.
        let wires = vec![shape(1, 0, 0, 5_000, 1_000), shape(2, 4_200, 200, 200, 200), shape(2, 4_600, 600, 200, 200), shape(3, 4_000, 0, 6_000, 1_000)];
        let r = routes(wires, vec![term(0, 400, 200, 200, Some(700.0)), term(9_800, 400, 200, 200, Some(-700.0))]);
        assert_eq!(em().limit(2).unwrap().cuts(700.0), 3, "⌈700/290⌉");
        let e = em();
        assert!(e.known(&r) && !e.satisfied(&r));
        assert!((e.residual(&r) - 1.0 / 3.0).abs() < 1e-4, "(3 − 2)/3: {}", e.residual(&r));
        // A third cut in the group meets it.
        let mut wide = r.wires[0].clone();
        wide.push(shape(2, 4_200, 600, 200, 200));
        let r3 = routes(wide, r.terms[0].clone());
        assert!(e.known(&r3) && e.satisfied(&r3), "{}", e.residual(&r3));
    }

    /// A metal limit carries its deck rule's per-cut figure too ([`em`]): a
    /// met1 wire is no via group. 500 µA (> 360 µA/cut) needs 178.6 nm of the
    /// 1 µm drawn.
    #[test]
    fn a_metal_with_a_per_cut_limit_is_not_a_via() {
        let e = em();
        let r = routes(vec![shape(1, 0, 0, 10_000, 1_000)], vec![term(0, 0, 200, 1_000, Some(500.0)), term(9_800, 0, 200, 1_000, Some(-500.0))]);
        assert!(e.known(&r) && e.satisfied(&r), "{}", e.residual(&r));
        assert!((e.usage(&r).unwrap() - need(500.0) / 1_000.0).abs() < 1e-4, "{:?}", e.usage(&r));
    }

    /// `dr`'s via array: each cut lands on its own met2 pad, and every pad
    /// overlaps the one met2 trunk the cuts also touch. The landing sets
    /// differ ({pad k, trunk}), yet the three cuts are one group in parallel.
    #[test]
    fn array_cuts_on_separate_pads_over_one_trunk_are_one_group() {
        let mut wires = vec![shape(1, 0, 0, 5_000, 1_000), shape(3, 4_000, 0, 6_000, 1_000)];
        for x in [4_200, 4_600, 5_000] {
            wires.push(shape(3, x - 75, 300, 350, 1_200));
            wires.push(shape(2, x, 400, 200, 200));
        }
        let terms = vec![term(0, 400, 200, 200, Some(700.0)), term(9_800, 400, 200, 200, Some(-700.0))];
        let r = routes(wires.clone(), terms.clone());
        let e = em();
        assert!(e.known(&r) && e.satisfied(&r), "⌈700/290⌉ = 3 cuts in one group: {}", e.residual(&r));
        // usage = I/(I_cut·n) pins n = 3 (a pad may carry all 700 µA: 250/350 nm).
        assert!((e.usage(&r).unwrap() - 700.0 / (290.0 * 3.0)).abs() < 1e-4, "{:?}", e.usage(&r));
        // Two cuts left: still one group, now short.
        wires.pop();
        assert!((e.residual(&routes(wires, terms)) - 1.0 / 3.0).abs() < 1e-4);
    }

    /// A terminal joined only through its cell's strap is reached (the cell
    /// metal is in the flow), and the strap itself is the cell's, not checked:
    /// at 30 nm it would need 179 nm.
    #[test]
    fn a_cell_strap_joins_its_terminal_and_is_not_checked() {
        let r = jog(Some(-500.0));
        let wires = vec![r.wires[0][0]];
        let terms = vec![r.terms[0][0], term(12_000, 400, 200, 200, Some(-500.0))];
        let strap = vec![shape(1, 10_000, 450, 2_200, 30)];
        let open = routes(wires.clone(), terms.clone());
        assert!(!em().known(&open), "the strap is what joins the second terminal");
        let r = Routes { cell: vec![strap], ..routes(wires, terms) };
        assert!(em().known(&r) && em().satisfied(&r), "{}", em().residual(&r));
    }

    /// A pin on met1 under the met2 it is routed to: the terminal joins its
    /// own (lowest) layer, so its current reaches met2 only through the pin
    /// cut — never a 0-Ω shortcut to the met2 over it.
    #[test]
    fn a_pin_cut_carries_its_terminal_current() {
        let mut wires = vec![shape(1, 0, 0, 400, 400), shape(3, 0, 0, 10_000, 400), shape(2, 100, 100, 200, 200)];
        let terms = vec![term(0, 0, 400, 400, Some(700.0)), term(9_800, 0, 200, 400, Some(-700.0))];
        let r = routes(wires.clone(), terms.clone());
        assert!(em().known(&r) && !em().satisfied(&r));
        assert!((em().residual(&r) - 2.0 / 3.0).abs() < 1e-4, "⌈700/290⌉ = 3 cuts, 1 drawn: {}", em().residual(&r));
        // Without the cut the pin is open, not joined through the met2 over it.
        wires.pop();
        assert!(!em().known(&routes(wires, terms)));
    }

    #[test]
    fn an_unknown_terminal_current_is_unknown() {
        let r = jog(None);
        assert!(!em().known(&r) && em().satisfied(&r), "search cannot act on an unknown");
        assert_eq!(em().residual(&r), 0.0);
    }

    #[test]
    fn an_unbalanced_net_charges_the_larger_side() {
        // +300 and +200 on one wire, no sink: the edge carries max(200, 500 − 200).
        let wires = vec![shape(1, 0, 0, 10_000, 500)];
        let flow = net_flow(stack(), &wires, &[term(0, 0, 200, 500, Some(300.0)), term(9_800, 0, 200, 500, Some(200.0))]).unwrap();
        assert!((flow.shape_ua[0] - 300.0).abs() < 1e-3, "{:?}", flow.shape_ua);
        // Drop end to end: 0.125 Ω/□ · 9.8 µm / 0.5 µm = 2.45 Ω at 300 µA.
        assert!((flow.drop_uv - 735.0).abs() < 0.1, "{}", flow.drop_uv);
    }

    #[test]
    fn parallel_paths_are_bounded_by_the_tree_path() {
        // Two equal met1 wires between end bars: A(+1000) left, B(−1000) right.
        let wires = vec![
            shape(1, 0, 0, 200, 2_200),
            shape(1, 9_800, 0, 200, 2_200),
            shape(1, 0, 2_000, 10_000, 200),
            shape(1, 0, 0, 10_000, 200),
        ];
        let flow = net_flow(stack(), &wires, &[term(0, 1_000, 200, 200, Some(1_000.0)), term(9_800, 1_000, 200, 200, Some(-1_000.0))]).unwrap();
        assert!((flow.shape_ua[2] - 1_000.0).abs() < 1e-3 && (flow.shape_ua[3] - 1_000.0).abs() < 1e-3, "{:?}", flow.shape_ua);
    }

    /// Unknown without the stack, without a limit on any routed layer, or
    /// without routed terminals; a known **zero** current is a real pass.
    #[test]
    fn missing_stack_limit_or_terminals_is_unknown_and_zero_is_known() {
        let r = jog(Some(-500.0));
        assert!(!Electromigration { stack: None, ..em() }.known(&r), "no stack");
        let mut other = em();
        other.limits = [(u16::MAX, Limit::default()); MAX_LAYERS];
        other.limits[0] = (3, Limit { ua_per_um: 2_800.0, ..Limit::default() });
        assert!(!other.known(&r), "a limit listed for another layer does not apply to met1");
        assert!(!em().known(&routes(r.wires[0].clone(), Vec::new())), "no terminals");
        let open = routes(r.wires[0].clone(), vec![r.terms[0][0], term(50_000, 0, 10, 10, Some(-500.0))]);
        assert!(!em().known(&open), "an unreached terminal");
        let zero = routes(r.wires[0].clone(), vec![term(0, 400, 200, 200, Some(0.0)), term(9_000, 1_300, 140, 100, Some(0.0))]);
        assert!(em().known(&zero) && em().satisfied(&zero));
    }

    /// Hastings' worked derating: 398 K on a 378 K rating with Ea 0.7 eV, n 2
    /// gives F ≈ 0.58 ("25 mA safe at ref → 15 mA"). Hotter never needs fewer
    /// cuts or less width; cooler is never credited.
    #[test]
    fn hotter_never_needs_fewer_cuts_or_less_width() {
        let f = derate(398.0, 378.0, 0.7, 2.0);
        assert!((f - 0.58).abs() < 0.02, "Hastings eq. 15.25 example: {f}");
        assert_eq!(derate(300.0, 378.0, 0.7, 2.0), 1.0, "cooler than the rating is not credited");
        let base = Limit { ua_per_um: 1_000.0, ua_per_cut: 100.0, blech: 0.0 };
        let mut prev = (0u32, 0.0f32);
        for t in [378.0, 398.0, 423.0, 448.0] {
            let hot = base.derated(derate(t, 378.0, 0.9, 2.0));
            let now = (hot.cuts(250.0), hot.width_nm(250.0, 0.0));
            assert!(now.0 >= prev.0 && now.1 >= prev.1, "T {t}: {now:?} after {prev:?}");
            prev = now;
        }
        assert_eq!(base.cuts(250.0), 3, "⌈250/100⌉ at the rating");
        assert!(prev.0 > 3, "hot enough to need more cuts: {prev:?}");
    }

    /// A short enough run is Blech-immortal and needs less than `I/J`; without
    /// a deck Blech product the density limit always applies.
    #[test]
    fn a_short_run_below_the_blech_product_needs_less_width() {
        let l = Limit { ua_per_um: 1_000.0, ua_per_cut: 0.0, blech: 54_000.0 };
        // 500 µA: I/J = 500 nm. Over 20 µm, I·L/B = 500·20_000/54_000 ≈ 185 nm.
        assert!((l.width_nm(500.0, 20_000.0) - 185.2).abs() < 1.0);
        // A long run is bounded by density again.
        assert!((l.width_nm(500.0, 1_000_000.0) - 500.0).abs() < 1e-3);
        assert!((Limit { blech: 0.0, ..l }.width_nm(500.0, 20_000.0) - 500.0).abs() < 1e-3);
    }
}
