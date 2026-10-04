//! Circuit-level symmetry (EXT-14): pairs propagated from seed couples
//! through mated nets, grouped into compounds that share one axis (survey
//! §3.2.2.2; the propagation is Philis design, the survey only names it).

use std::collections::{BTreeSet, VecDeque};

use analog::intent::{AxisDir, Compound, ConstraintId, Diagnostic, SymKind};
use analog::metadata::{NetClass, NetClassification};
use pnr_core::ids::{AxisId, DeviceId, NetId};
use pnr_core::netlist::DeviceKind;
use pnr_core::BipartiteHypergraph;

use crate::conflict;
use crate::size::{self, Drawn};

/// A couple symmetry starts from: declared leaf pairs now, sidecar nets and
/// selfs once EXT-26 (M3) builds them. The first device or net is half A.
#[derive(Clone, Copy, Debug)]
pub enum Seed {
    Devices(DeviceId, DeviceId, ConstraintId),
    Nets(NetId, NetId, ConstraintId),
    SelfDevice(DeviceId, ConstraintId),
}

/// What two devices must share to mirror each other (card D-e): kind, model,
/// finger W, L and finger count. Bulk is left out (a PMOS pair's bulk may be its source).
type Sig = (DeviceKind, u16, Option<i64>, Option<i64>, u32);
/// `(A device, B device, passive flipped, rank)`.
type Cand = (u32, u32, bool, (usize, usize));

fn sig(k: DeviceKind, s: &Drawn) -> Sig {
    (k, s.model, s.w_finger_nm, s.l_nm, s.fingers)
}

fn passive(k: DeviceKind) -> bool {
    matches!(k, DeviceKind::Resistor | DeviceKind::Capacitor | DeviceKind::Inductor | DeviceKind::Diode)
}

/// Channel terminals carry the device's current (D S, C E, P N); the rest are control (G, a BJT's B) or bulk.
fn channel(t: &str) -> bool {
    matches!(t, "D" | "S" | "C" | "E" | "P" | "N")
}

fn control(k: DeviceKind, t: &str) -> bool {
    t == "G" || (matches!(k, DeviceKind::Npn | DeviceKind::Pnp) && t == "B")
}

struct State<'a> {
    hg: &'a BipartiteHypergraph,
    rail: Vec<bool>,
    pair_of: Vec<Option<u32>>,
    /// Half A of its pair, and the passive orientation the pair was taken with.
    half_a: Vec<bool>,
    flip: Vec<bool>,
    is_self: Vec<bool>,
    /// `Some(self)` = self-symmetric.
    mate: Vec<Option<u32>>,
    /// Net is on half A of its pair.
    a_side: Vec<bool>,
    queue: VecDeque<(u32, u32)>,
    /// GAP-09 (a): the seed being applied, and the seed that paired or
    /// self-marked each device / mated each net (`cur` stays the last seed's
    /// during the fixpoint, where no conflict is reported).
    cur: ConstraintId,
    by: Vec<Option<ConstraintId>>,
    net_by: Vec<Option<ConstraintId>>,
}

impl State<'_> {
    fn net(&self, d: u32, t: &str) -> Option<u32> {
        let i = d as usize;
        self.hg.terminals[i].iter().position(|p| p == t).map(|k| self.hg.device_nets[i][k].0 as u32)
    }

    /// Terminal of `e` that `d`'s terminal `t` maps to: the same name, or the other
    /// end of a two-terminal passive when `flip`.
    fn mapped(t: &str, flip: bool) -> &str {
        match (flip, t) {
            (true, "P") => "N",
            (true, "N") => "P",
            _ => t,
        }
    }

    /// Nets `x` (half A) and `y` may mirror: mated to each other, both unmated,
    /// equal (a shared net carries no constraint) or both rails.
    fn consistent_nets(&self, x: u32, y: u32) -> bool {
        let (xi, yi) = (x as usize, y as usize);
        x == y || (self.rail[xi] && self.rail[yi]) || (!self.rail[xi] && !self.rail[yi] && match (self.mate[xi], self.mate[yi]) {
            (None, None) => true,
            (Some(a), Some(b)) => a == y && b == x,
            _ => false,
        })
    }

    fn consistent(&self, d: u32, e: u32, flip: bool) -> bool {
        self.hg.terminals[d as usize].iter().all(|t| match (self.net(d, t), self.net(e, Self::mapped(t, flip))) {
            (Some(x), Some(y)) => self.consistent_nets(x, y),
            _ => false,
        })
    }

    /// `(shared channel nets, shared control nets)` of `d` against `e`.
    fn rank(&self, d: u32, e: u32, flip: bool) -> (usize, usize) {
        let k = self.hg.kinds[d as usize];
        let shared = |f: &dyn Fn(&str) -> bool| {
            self.hg.terminals[d as usize].iter().filter(|t| f(t) && self.net(d, t) == self.net(e, Self::mapped(t, flip))).count()
        };
        (shared(&channel), shared(&|t| control(k, t)))
    }

    /// Mates `x` (half A) with `y`; `x == y` (a net both halves share: a tail, a
    /// shared gate) becomes self-symmetric. An already mated net stays as it is
    /// (shared, no constraint); rails stay fixed points.
    fn bind(&mut self, x: u32, y: u32) {
        let (xi, yi) = (x as usize, y as usize);
        if self.rail[xi] || self.rail[yi] || self.mate[xi].is_some() || self.mate[yi].is_some() {
            return;
        }
        self.mate[xi] = Some(y);
        self.mate[yi] = Some(x);
        self.a_side[xi] = true;
        (self.net_by[xi], self.net_by[yi]) = (Some(self.cur), Some(self.cur));
        self.queue.push_back((x, y));
    }

    fn pair(&mut self, d: u32, e: u32, flip: bool) {
        self.pair_of[d as usize] = Some(e);
        self.pair_of[e as usize] = Some(d);
        (self.by[d as usize], self.by[e as usize]) = (Some(self.cur), Some(self.cur));
        self.half_a[d as usize] = true;
        self.flip[d as usize] = flip;
        self.flip[e as usize] = flip;
        for t in self.hg.terminals[d as usize].clone() {
            if let (Some(x), Some(y)) = (self.net(d, &t), self.net(e, Self::mapped(&t, flip))) {
                self.bind(x, y);
            }
        }
    }

    /// The seed whose pairing or net mate a seed `(a, b)` contradicts: one that
    /// took `a` or `b`, else the one that mated the first inconsistent net of `a`.
    fn owner(&self, a: u32, b: u32) -> Option<ConstraintId> {
        self.by[a as usize].or(self.by[b as usize]).or_else(|| {
            self.hg.terminals[a as usize].iter().find_map(|t| {
                let (x, y) = (self.net(a, t)?, self.net(b, t)?);
                (!self.consistent_nets(x, y)).then(|| self.net_by[x as usize].or(self.net_by[y as usize])).flatten()
            })
        })
    }

    fn free(&self, d: u32) -> bool {
        self.pair_of[d as usize].is_none() && !self.is_self[d as usize]
    }

    /// Devices on `n` with the terminal names they sit on it by.
    fn on(&self, n: u32) -> Vec<(u32, String)> {
        let mut v: Vec<(u32, String)> = Vec::new();
        for d in &self.hg.net_devices[n as usize] {
            let i = d.0 as usize;
            for (t, m) in self.hg.terminals[i].iter().zip(&self.hg.device_nets[i]) {
                if m.0 as u32 == n && !v.iter().any(|(e, s)| *e == d.0 as u32 && s == t) {
                    v.push((d.0 as u32, t.clone()));
                }
            }
        }
        v
    }
}

/// Symmetry analysis over the seeds, in order. Pairs propagate through mated nets
/// (plan-01 EXT-14 steps 1-7): a candidate pair on a mated net pair shares the
/// signature and terminal name (either end for a two-terminal passive), and every
/// other terminal is consistent; the pair is taken only when it is the mutual unique
/// best by `(shared channel nets, shared control nets)`, never by id. A device whose
/// channel terminals are all self-symmetric nets or rails is self-symmetric. A
/// compound is a component of pairs and selfs joined through mated nets (any
/// terminal) and self-symmetric nets (channel terminals only, so a shared bias net
/// does not join unrelated structures); one without a pair is dropped. Compounds
/// are ordered by their smallest pair's canonical labels; `axis` is the index.
#[must_use]
pub fn analyze(hg: &BipartiteHypergraph, drawn: &[Drawn], classes: &[NetClassification], seeds: &[Seed], canon: &[u64]) -> (Vec<Compound>, Vec<Diagnostic>) {
    let (nd, nn) = (hg.device_count(), hg.net_devices.len());
    let rail: Vec<bool> = classes.iter().map(|c| matches!(c.class, NetClass::Supply | NetClass::Ground | NetClass::Substrate)).collect();
    let mut s = State { hg, rail, pair_of: vec![None; nd], half_a: vec![false; nd], flip: vec![false; nd], is_self: vec![false; nd], mate: vec![None; nn], a_side: vec![false; nn], queue: VecDeque::new(), cur: ConstraintId(0), by: vec![None; nd], net_by: vec![None; nn] };
    let sigs: Vec<_> = (0..nd).map(|d| sig(hg.kinds[d], &drawn[d])).collect();
    let pairable = |d: u32| !size::unknown_size(hg.kinds[d as usize], &drawn[d as usize]);
    let mut diags = Vec::new();
    let mut ambiguous: BTreeSet<Vec<u32>> = BTreeSet::new();

    for seed in seeds {
        s.cur = match *seed {
            Seed::Devices(.., id) | Seed::Nets(.., id) | Seed::SelfDevice(_, id) => id,
        };
        match *seed {
            Seed::Devices(a, b, id) => {
                let (a, b) = (a.0 as u32, b.0 as u32);
                if s.pair_of[a as usize] == Some(b) || sigs[a as usize] != sigs[b as usize] {
                    continue;
                }
                if !s.free(a) || !s.free(b) || !s.consistent(a, b, false) {
                    // GAP-09 (a): the earlier seed wins (user seeds first, in entry order).
                    let ids: Vec<ConstraintId> = std::iter::once(id).chain(s.owner(a, b)).collect();
                    diags.push(conflict::diag(&ids, vec![DeviceId(a as u16), DeviceId(b as u16)], "SymmetricBlocks pair contradicts an earlier pairing; dropped"));
                    continue;
                }
                s.pair(a, b, false);
            }
            Seed::Nets(x, y, _) => {
                if x == y && !s.rail[x.0 as usize] && s.mate[x.0 as usize].is_none() {
                    s.mate[x.0 as usize] = Some(x.0 as u32);
                    s.queue.push_back((x.0 as u32, x.0 as u32));
                } else {
                    s.bind(x.0 as u32, y.0 as u32);
                }
            }
            Seed::SelfDevice(d, id) => {
                let i = d.0 as usize;
                if s.free(d.0 as u32) {
                    s.is_self[i] = true;
                    s.by[i] = Some(id);
                } else if s.by[i] != Some(id) {
                    let ids: Vec<ConstraintId> = std::iter::once(id).chain(s.by[i]).collect();
                    diags.push(conflict::diag(&ids, vec![d], "self-symmetric device already paired; dropped"));
                }
            }
        }
        propagate(&mut s, &sigs, &pairable, canon, &mut ambiguous);
    }
    // Fixpoint: a pairing can resolve an earlier ambiguity.
    for _ in 0..nd {
        let before = s.pair_of.iter().filter(|p| p.is_some()).count() + s.is_self.iter().filter(|&&b| b).count();
        s.queue.extend((0..nn as u32).filter_map(|x| s.mate[x as usize].filter(|_| s.a_side[x as usize] || s.mate[x as usize] == Some(x)).map(|y| (x, y))));
        propagate(&mut s, &sigs, &pairable, canon, &mut ambiguous);
        let after = s.pair_of.iter().filter(|p| p.is_some()).count() + s.is_self.iter().filter(|&&b| b).count();
        if after == before {
            break;
        }
    }
    for g in ambiguous.into_iter().filter(|g| s.free(g[0])) {
        diags.push(Diagnostic {
            kind: "ambiguous_symmetry",
            devices: g.iter().map(|&d| DeviceId(d as u16)).collect(),
            message: "no mutual unique best mirror partner".into(),
        });
    }
    (compounds(&s, canon), diags)
}

/// Drains the queue: a self-symmetric net (x, x) makes selfs (step 5); a mated
/// pair pairs its mutual-unique-best candidates (step 4).
fn propagate(s: &mut State, sigs: &[Sig], pairable: &dyn Fn(u32) -> bool, canon: &[u64], ambiguous: &mut BTreeSet<Vec<u32>>) {
    while let Some((x, y)) = s.queue.pop_front() {
        if x == y {
            for (d, _) in s.on(x) {
                let i = d as usize;
                let ch: Vec<u32> = s.hg.terminals[i].iter().filter(|t| channel(t)).filter_map(|t| s.net(d, t)).collect();
                let selfsym = |n: u32| s.rail[n as usize] || s.mate[n as usize] == Some(n);
                if s.free(d) && pairable(d) && !ch.is_empty() && ch.iter().all(|&n| selfsym(n)) && ch.iter().any(|&n| !s.rail[n as usize]) {
                    s.is_self[i] = true;
                    s.by[i] = Some(s.cur);
                }
            }
            continue;
        }
        let (lx, ly) = (s.on(x), s.on(y));
        let mut cands: Vec<Cand> = Vec::new();
        for (d, t) in &lx {
            for (e, u) in &ly {
                let (d, e) = (*d, *e);
                if d == e || !s.free(d) || !s.free(e) || !pairable(d) || sigs[d as usize] != sigs[e as usize] {
                    continue;
                }
                let k = s.hg.kinds[d as usize];
                let flip = t != u;
                if flip && !(passive(k) && s.hg.terminals[d as usize].len() == 2) {
                    continue;
                }
                if !cands.iter().any(|c| (c.0, c.1) == (d, e)) && s.consistent(d, e, flip) {
                    cands.push((d, e, flip, s.rank(d, e, flip)));
                }
            }
        }
        // Mutual unique best, in canonical order of the A device.
        let best = |of: &dyn Fn(&Cand) -> u32, who: u32| {
            let mine: Vec<_> = cands.iter().filter(|c| of(c) == who).collect();
            let m = mine.iter().map(|c| c.3).min()?;
            let at: Vec<_> = mine.into_iter().filter(|c| c.3 == m).collect();
            Some(at)
        };
        let mut order: Vec<u32> = cands.iter().map(|c| c.0).collect();
        order.sort_by_key(|&d| (canon[d as usize], d));
        order.dedup();
        for d in order {
            let Some(at) = best(&|c| c.0, d) else { continue };
            if at.len() > 1 {
                let mut g: Vec<u32> = std::iter::once(d).chain(at.iter().map(|c| c.1)).collect();
                g[1..].sort_unstable();
                ambiguous.insert(g);
                continue;
            }
            let (_, e, flip, _) = *at[0];
            let back = best(&|c| c.1, e).unwrap_or_default();
            if back.len() != 1 {
                let mut g: Vec<u32> = std::iter::once(e).chain(back.iter().map(|c| c.0)).collect();
                g[1..].sort_unstable();
                ambiguous.insert(g);
                continue;
            }
            if back[0].0 == d && s.free(d) && s.free(e) && s.consistent(d, e, flip) {
                s.pair(d, e, flip);
            }
        }
    }
}

fn compounds(s: &State, canon: &[u64]) -> Vec<Compound> {
    let nd = s.pair_of.len();
    let sym = |d: usize| s.pair_of[d].is_some() || s.is_self[d];
    let mut parent: Vec<usize> = (0..nd).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    let union = |p: &mut Vec<usize>, a: usize, b: usize| {
        let (x, y) = (find(p, a), find(p, b));
        p[x.max(y)] = x.min(y);
    };
    for d in 0..nd {
        if let Some(e) = s.pair_of[d] {
            union(&mut parent, d, e as usize);
        }
    }
    // Per net, its symmetric devices joined (channel terminals only on a self-symmetric net).
    let mut net_dev: Vec<Option<usize>> = vec![None; s.mate.len()];
    for (n, mate) in s.mate.iter().enumerate() {
        let Some(m) = *mate else { continue };
        let members: Vec<usize> = s.on(n as u32).into_iter().filter(|(d, t)| sym(*d as usize) && (m != n as u32 || channel(t))).map(|(d, _)| d as usize).collect();
        for w in members.windows(2) {
            union(&mut parent, w[0], w[1]);
        }
        net_dev[n] = members.first().copied().or_else(|| s.on(n as u32).into_iter().map(|(d, _)| d as usize).find(|&d| sym(d)));
    }
    let mut by_root: std::collections::BTreeMap<usize, Compound> = std::collections::BTreeMap::new();
    let blank = || Compound {
        id: ConstraintId(0),
        axis: AxisId(0),
        dir: AxisDir::V,
        kind: SymKind::Mirror,
        pairs: Vec::new(),
        selfs: Vec::new(),
        net_pairs: Vec::new(),
        self_nets: Vec::new(),
        set_pairs: Vec::new(),
    };
    for d in 0..nd {
        let r = find(&mut parent, d);
        if let Some(e) = s.pair_of[d] {
            if s.half_a[d] {
                by_root.entry(r).or_insert_with(blank).pairs.push((DeviceId(d as u16), DeviceId(e as u16)));
            }
        } else if s.is_self[d] {
            by_root.entry(r).or_insert_with(blank).selfs.push(DeviceId(d as u16));
        }
    }
    for (n, mate) in s.mate.iter().enumerate() {
        let (Some(m), Some(d)) = (*mate, net_dev[n]) else { continue };
        let r = find(&mut parent, d);
        let Some(c) = by_root.get_mut(&r) else { continue };
        if m == n as u32 {
            c.self_nets.push(NetId(n as u16));
        } else if s.a_side[n] {
            c.net_pairs.push((NetId(n as u16), NetId(m as u16)));
        }
    }
    let mut out: Vec<Compound> = by_root.into_values().filter(|c| !c.pairs.is_empty()).collect();
    let pk = |c: &Compound| c.pairs.iter().map(|&(a, b)| { let (x, y) = (canon[a.0 as usize], canon[b.0 as usize]); (x.min(y), x.max(y)) }).min();
    for c in &mut out {
        c.pairs.sort_by_key(|&(a, b)| (canon[a.0 as usize].min(canon[b.0 as usize]), a.0));
        c.selfs.sort_by_key(|d| (canon[d.0 as usize], d.0));
    }
    out.sort_by_key(|c| (pk(c), c.pairs[0].0 .0));
    for (i, c) in out.iter_mut().enumerate() {
        c.id = ConstraintId(i as u32);
        c.axis = AxisId(i as u16);
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::tests::{fet, nets};
    use crate::{annotate, AnnotationConfig};
    use pnr_core::netlist::{DeviceKind, Netlist};

    /// A diff pair whose drains each carry two identical loads: no mutual unique
    /// best, so the loads stay unpaired and are reported, never paired by id.
    #[test]
    fn tied_candidates_are_ambiguous() {
        // Nets: 0=a 1=inp 2=t 3=VSS 4=b 5=inn 6=vb 7=g.
        let n = DeviceKind::Nmos;
        let nl = Netlist {
            devices: vec![
                fet("M1", n, 1, 0, 2, 3, 4_000, 500),
                fet("M2", n, 5, 4, 2, 3, 4_000, 500),
                fet("M0", n, 6, 2, 3, 3, 8_000, 500),
                fet("X1", n, 7, 0, 3, 3, 2_000, 500),
                fet("X2", n, 7, 0, 3, 3, 2_000, 500),
                fet("Y1", n, 7, 4, 3, 3, 2_000, 500),
                fet("Y2", n, 7, 4, 3, 3, 2_000, 500),
            ],
            nets: nets(&["a", "inp", "t", "VSS", "b", "inn", "vb", "g"]),
            ..Default::default()
        };
        let p = annotate(&nl, &AnnotationConfig::default());
        let c = &p.intent.compounds[0];
        let name = |d: pnr_core::ids::DeviceId| nl.devices[d.0 as usize].name.as_str();
        assert_eq!(c.pairs.iter().map(|&(a, b)| (name(a), name(b))).collect::<Vec<_>>(), [("M1", "M2")]);
        assert!(p.intent.diagnostics.iter().any(|d| d.kind == "ambiguous_symmetry"), "{:?}", p.intent.diagnostics);
    }
}
