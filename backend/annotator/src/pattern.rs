//! Pattern DSL + backtracking matcher. The patterns themselves are data in
//! [`crate::catalog`].

use std::cmp::Reverse;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use pnr_core::ids::NetId;
use pnr_core::netlist::DeviceKind;
use pnr_core::BipartiteHypergraph;

use crate::catalog::{roles_of, PATTERNS};
use crate::netrole::{AnnotationConfig, NetRole};
use crate::size::{self, Drawn};

#[derive(Debug, Clone, Copy)]
pub enum PinRel {
    Same,
    Diff,
    /// Same net, and that net's role is `Signal`.
    SameSignal,
}

#[derive(Debug, Clone, Copy)]
pub enum SlotKind {
    AnyFet,
    SameTypeAs(u8),
    ComplementOf(u8),
}

/// Size relation to slot `r`, over [`Drawn`]: `ExactAs` is [`size::exact_as`] (known
/// W/L, model and bulk), `SameLAs` is [`size::same_l_as`] (known L and model).
#[derive(Debug, Clone, Copy)]
pub enum SizeMatch {
    Any,
    ExactAs(u8),
    SameLAs(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum DiodeReq {
    Required,
    Forbidden,
    Any,
}

#[derive(Debug, Clone, Copy)]
pub struct Slot {
    pub kind: SlotKind,
    pub size_match: SizeMatch,
    pub diode: DiodeReq,
    /// The gate net must be a signal (not a rail/clock).
    pub gate_is_signal: bool,
}

/// `a.pin_a` and `b.pin_b` are on the same (or different) nets.
#[derive(Debug, Clone, Copy)]
pub struct PinLink {
    pub a: u8,
    pub pin_a: &'static str,
    pub b: u8,
    pub pin_b: &'static str,
    pub rel: PinRel,
}

#[derive(Debug, Clone, Copy)]
pub struct Pattern {
    pub name: &'static str,
    /// Higher wins when two matches overlap.
    pub priority: u32,
    pub slots: &'static [Slot],
    pub links: &'static [PinLink],
}

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub template: &'static str,
    /// The matched pattern (its roles: [`crate::catalog::roles_of`]).
    pub pattern: &'static Pattern,
    /// Device ids in slot order.
    pub instances: Vec<u32>,
    pub priority: u32,
}

pub(crate) fn pin_net(hg: &BipartiteHypergraph, cell: u32, pin: &str) -> Option<NetId> {
    let i = cell as usize;
    hg.terminals[i].iter().position(|p| p == pin).map(|t| hg.device_nets[i][t])
}

/// Column of `pin` in a [`pins`] row: G, D, S, B, C, E, P, N.
fn pin_index(pin: &str) -> Option<usize> {
    ["G", "D", "S", "B", "C", "E", "P", "N"].iter().position(|p| *p == pin)
}

/// Every device's net per [`pin_index`] column, built once so the matcher never
/// searches pin names. A pin name outside the table reads as absent.
pub(crate) fn pins(hg: &BipartiteHypergraph) -> Vec<[Option<NetId>; 8]> {
    hg.terminals
        .iter()
        .zip(&hg.device_nets)
        .map(|(ts, ns)| {
            let mut row = [None; 8];
            for (t, &n) in ts.iter().zip(ns) {
                if let Some(i) = pin_index(t) {
                    row[i] = Some(n);
                }
            }
            row
        })
        .collect()
}

fn pin_of(pins: &[[Option<NetId>; 8]], cell: u32, pin: &str) -> Option<NetId> {
    pin_index(pin).and_then(|i| pins[cell as usize][i])
}

fn is_diode(pins: &[[Option<NetId>; 8]], cell: u32) -> bool {
    let d = pin_of(pins, cell, "D");
    d.is_some() && d == pin_of(pins, cell, "G")
}

/// `assigned` is indexed by slot; every slot `slot` refers to is assigned ([`slot_order`]).
fn slot_ok(slot: &Slot, s: &Search, cell: u32, assigned: &[u32]) -> bool {
    let kind = |i: u32| s.hg.kinds[i as usize];
    let dt = kind(cell);
    if !matches!(dt, DeviceKind::Nmos | DeviceKind::Pmos) {
        return false;
    }
    let other = |r: u8| assigned[r as usize];
    let kind_ok = match slot.kind {
        SlotKind::AnyFet => true,
        SlotKind::SameTypeAs(r) => kind(other(r)) == dt,
        SlotKind::ComplementOf(r) => kind(other(r)) != dt && matches!(kind(other(r)), DeviceKind::Nmos | DeviceKind::Pmos),
    };
    let (g, gr) = (&s.drawn[cell as usize], |r: u8| &s.drawn[other(r) as usize]);
    let size_ok = match slot.size_match {
        SizeMatch::Any => true,
        SizeMatch::ExactAs(r) => size::exact_as(dt, g, gr(r), s.roles),
        SizeMatch::SameLAs(r) => size::same_l_as(g, gr(r)),
    };
    let diode_ok = match slot.diode {
        DiodeReq::Required => is_diode(s.pins, cell),
        DiodeReq::Forbidden => !is_diode(s.pins, cell),
        DiodeReq::Any => true,
    };
    let gate_ok = !slot.gate_is_signal
        || pin_of(s.pins, cell, "G").map_or(true, |n| s.roles[n.0 as usize] == NetRole::Signal);
    kind_ok && size_ok && diode_ok && gate_ok
}

/// Links touching slot `k` whose both ends are assigned hold (the others were
/// checked when their later end was assigned).
fn links_ok(s: &Search, k: u8, assigned: &[u32]) -> bool {
    let set = |x: u8| assigned[x as usize] != EMPTY;
    s.pat.links.iter().filter(|l| (l.a == k || l.b == k) && set(l.a) && set(l.b)).all(|l| {
        let na = pin_of(s.pins, assigned[l.a as usize], l.pin_a);
        let nb = pin_of(s.pins, assigned[l.b as usize], l.pin_b);
        match l.rel {
            PinRel::Same => na.is_some() && na == nb,
            PinRel::Diff => na != nb,
            PinRel::SameSignal => na.is_some_and(|n| Some(n) == nb && s.roles[n.0 as usize] == NetRole::Signal),
        }
    })
}

/// An unassigned slot.
const EMPTY: u32 = u32::MAX;

/// The order [`Search`] fills `p`'s slots in: slot 0, then repeatedly the
/// smallest unplaced slot whose `SlotKind`/`SizeMatch` references are placed and
/// that has a `Same`/`SameSignal` link to a placed slot on a pin pair not both
/// in {S, B} (a rail-sourced slot 0 would otherwise hand the next slot the whole
/// rail); else one with any such link; else the smallest with its references
/// placed. Catalog references point to earlier slots, so the smallest unplaced
/// slot always qualifies.
fn slot_order(p: &Pattern) -> Vec<usize> {
    let n = p.slots.len();
    let mut placed = vec![false; n];
    let mut order = Vec::with_capacity(n);
    while order.len() < n {
        let refs_placed = |k: usize| {
            let s = &p.slots[k];
            let kr = match s.kind {
                SlotKind::AnyFet => None,
                SlotKind::SameTypeAs(r) | SlotKind::ComplementOf(r) => Some(r),
            };
            let sr = match s.size_match {
                SizeMatch::Any => None,
                SizeMatch::ExactAs(r) | SizeMatch::SameLAs(r) => Some(r),
            };
            [kr, sr].into_iter().flatten().all(|r| placed[r as usize])
        };
        let linked = |k: usize, strong: bool| {
            let sb = |pin: &str| matches!(pin, "S" | "B");
            p.links.iter().any(|l| {
                let (a, b) = (l.a as usize, l.b as usize);
                matches!(l.rel, PinRel::Same | PinRel::SameSignal)
                    && ((a == k && placed[b]) || (b == k && placed[a]))
                    && (!strong || !(sb(l.pin_a) && sb(l.pin_b)))
            })
        };
        let ready: Vec<usize> = (0..n).filter(|&k| !placed[k] && refs_placed(k)).collect();
        let k = if order.is_empty() {
            0
        } else {
            ready.iter().copied().find(|&k| linked(k, true)).or_else(|| ready.iter().copied().find(|&k| linked(k, false))).unwrap_or(ready[0])
        };
        placed[k] = true;
        order.push(k);
    }
    order
}

struct Search<'a> {
    pat: &'static Pattern,
    order: Vec<usize>,
    hg: &'a BipartiteHypergraph,
    pins: &'a [[Option<NetId>; 8]],
    drawn: &'a [Drawn],
    roles: &'a [NetRole],
    allowed: &'a [bool],
    rank: &'a [u32],
}

/// Where the next slot's devices come from.
enum Candidates {
    /// Devices on net `.0` at pin column `.1`.
    Net(NetId, usize),
    /// A linking net is absent or not a signal where one must be: nothing fits.
    None,
    /// No link to an assigned slot: every device.
    All,
}

impl Search<'_> {
    /// Over links joining slot `k`'s pin to an assigned slot's pin by `Same` or
    /// `SameSignal`, the net with the fewest devices.
    fn candidates(&self, k: u8, assigned: &[u32]) -> Candidates {
        let mut best: Option<(NetId, usize)> = None;
        for l in self.pat.links.iter().filter(|l| matches!(l.rel, PinRel::Same | PinRel::SameSignal)) {
            let (pk, o, po) = if l.a == k { (l.pin_a, l.b, l.pin_b) } else if l.b == k { (l.pin_b, l.a, l.pin_a) } else { continue };
            if o == k || assigned[o as usize] == EMPTY {
                continue;
            }
            let (Some(pk), Some(n)) = (pin_index(pk), pin_of(self.pins, assigned[o as usize], po)) else { return Candidates::None };
            if matches!(l.rel, PinRel::SameSignal) && self.roles[n.0 as usize] != NetRole::Signal {
                return Candidates::None;
            }
            if best.map_or(true, |(b, _)| self.hg.net_devices[n.0 as usize].len() < self.hg.net_devices[b.0 as usize].len()) {
                best = Some((n, pk));
            }
        }
        best.map_or(Candidates::All, |(n, pk)| Candidates::Net(n, pk))
    }

    /// Every assignment of devices to slots, filled in `order`; one match per
    /// device *set*: the assignment whose slot-order `rank`s are lexicographically
    /// least, so automorphic assignments (a mirror's interchangeable outputs) resolve
    /// the same way under any netlist order.
    fn run(&self, depth: usize, assigned: &mut [u32], out: &mut Vec<PatternMatch>, seen: &mut HashMap<Vec<u32>, usize>) {
        let Some(&k) = self.order.get(depth) else {
            let mut key = assigned.to_vec();
            key.sort_unstable();
            let m = PatternMatch { template: self.pat.name, pattern: self.pat, instances: assigned.to_vec(), priority: self.pat.priority };
            let ranks = |i: &[u32]| i.iter().map(|&d| self.rank[d as usize]).collect::<Vec<_>>();
            match seen.get(&key) {
                None => {
                    seen.insert(key, out.len());
                    out.push(m);
                }
                Some(&i) if ranks(assigned) < ranks(&out[i].instances) => out[i] = m,
                Some(_) => {}
            }
            return;
        };
        let mut try_cell = |cell: u32, assigned: &mut [u32]| {
            if !self.allowed[cell as usize] || assigned.contains(&cell) || !slot_ok(&self.pat.slots[k], self, cell, assigned) {
                return;
            }
            assigned[k] = cell;
            if links_ok(self, k as u8, assigned) {
                self.run(depth + 1, assigned, out, seen);
            }
            assigned[k] = EMPTY;
        };
        match self.candidates(k as u8, assigned) {
            Candidates::Net(n, pk) => {
                let mut prev = None;
                for d in &self.hg.net_devices[n.0 as usize] {
                    let c = u32::from(d.0);
                    if prev != Some(c) && self.pins[c as usize][pk] == Some(n) {
                        try_cell(c, assigned);
                    }
                    prev = Some(c);
                }
            }
            Candidates::None => {}
            Candidates::All => (0..self.allowed.len() as u32).for_each(|c| try_cell(c, assigned)),
        }
    }
}

/// Every match of one pattern over the devices `allowed`; `pins` from [`pins`],
/// `rank` (by device) picks a device set's slot assignment ([`Search::run`]).
pub(crate) fn matches(
    pat: &'static Pattern,
    hg: &BipartiteHypergraph,
    pins: &[[Option<NetId>; 8]],
    drawn: &[Drawn],
    roles: &[NetRole],
    allowed: &[bool],
    rank: &[u32],
) -> Vec<PatternMatch> {
    let s = Search { pat, order: slot_order(pat), hg, pins, drawn, roles, allowed, rank };
    let mut out = Vec::new();
    s.run(0, &mut vec![EMPTY; pat.slots.len()], &mut out, &mut HashMap::new());
    out
}

/// Every match of every allowed pattern; one per (template, device set), its
/// slots assigned by least `(canon, name)` per slot ([`canonical_labels`],
/// device names), so the result does not depend on netlist order.
/// `cfg.do_not_identify` devices and `cfg.do_not_use` templates are skipped.
#[must_use]
pub fn recognize_all(hg: &BipartiteHypergraph, drawn: &[Drawn], roles: &[NetRole], cfg: &AnnotationConfig, canon: &[u64], names: &[&str]) -> Vec<PatternMatch> {
    let allowed: Vec<bool> = (0..hg.device_count() as u32).map(|d| !cfg.do_not_identify.contains(&d)).collect();
    let pins = pins(hg);
    let mut by: Vec<usize> = (0..canon.len()).collect();
    by.sort_by_key(|&d| (canon[d], names[d]));
    let mut rank = vec![0u32; by.len()];
    by.iter().enumerate().for_each(|(r, &d)| rank[d] = r as u32);
    PATTERNS
        .iter()
        .filter(|p| !cfg.do_not_use.contains(p.name))
        .flat_map(|p| matches(p, hg, &pins, drawn, roles, &allowed, &rank))
        .collect()
}

fn hash<T: Hash>(t: &T) -> u64 {
    let mut h = DefaultHasher::new();
    t.hash(&mut h);
    h.finish()
}

/// Permutation-invariant labels: 3 rounds of WL refinement on the device–net
/// bipartite graph, indexed by device id. Round 0 hashes kind, drawn size,
/// model name and each pin's (name, net role, net degree); never an id or a
/// model index (both depend on netlist order). `DefaultHasher::new()` has fixed
/// keys, so labels are deterministic.
#[must_use]
pub fn canonical_labels(hg: &BipartiteHypergraph, drawn: &[Drawn], models: &[String], roles: &[NetRole]) -> Vec<u64> {
    let role = |n: NetId| roles[n.0 as usize] as u8;
    let mut dev: Vec<u64> = (0..hg.device_count())
        .map(|d| {
            let s = &drawn[d];
            let mut p: Vec<(&str, u8, usize)> = hg.terminals[d]
                .iter()
                .zip(&hg.device_nets[d])
                .map(|(t, &n)| (t.as_str(), role(n), hg.net_devices[n.0 as usize].len()))
                .collect();
            p.sort_unstable();
            hash(&(hg.kinds[d] as u8, s.w_finger_nm, s.l_nm, s.fingers, &models[s.model as usize], p))
        })
        .collect();
    for _ in 0..3 {
        let net: Vec<u64> = (0..hg.net_devices.len())
            .map(|n| {
                let mut v: Vec<u64> = hg.net_devices[n].iter().map(|d| dev[d.0 as usize]).collect();
                v.sort_unstable();
                hash(&(roles[n] as u8, v))
            })
            .collect();
        dev = (0..dev.len())
            .map(|d| {
                let mut v: Vec<(&str, u64)> = hg.terminals[d].iter().zip(&hg.device_nets[d]).map(|(t, n)| (t.as_str(), net[n.0 as usize])).collect();
                v.sort_unstable();
                hash(&(dev[d], v))
            })
            .collect();
    }
    dev
}

/// Disjoint subset for `Problem::blocks` (interim, until EXT-13), in selection
/// order: priority first, then patterns declaring pairs, then the sorted
/// [`canonical_labels`] of the members, then their sorted names; kept greedily.
#[must_use]
pub fn select_disjoint(all: &[PatternMatch], canon: &[u64], names: &[&str]) -> Vec<PatternMatch> {
    let key = |m: &PatternMatch| {
        let mut c: Vec<u64> = m.instances.iter().map(|&i| canon[i as usize]).collect();
        c.sort_unstable();
        // ponytail: names break exact label ties (automorphic instances); ids would not be permutation-invariant.
        let mut n: Vec<&str> = m.instances.iter().map(|&i| names[i as usize]).collect();
        n.sort_unstable();
        (Reverse(m.priority), roles_of(m.pattern).pairs.is_empty(), c, n)
    };
    let mut keyed: Vec<_> = all.iter().map(|m| (key(m), m)).collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    let mut consumed = vec![false; canon.len()];
    let mut out = Vec::new();
    for (_, m) in keyed {
        if m.instances.iter().all(|&i| !consumed[i as usize]) {
            m.instances.iter().for_each(|&i| consumed[i as usize] = true);
            out.push(m.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognize_all_is_a_superset_of_select_disjoint() {
        let nl = crate::tests::ota();
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let cfg = AnnotationConfig::default();
        let roles = crate::netrole::classify_nets(&hg, &cfg);
        let mut models = Vec::new();
        let drawn: Vec<_> = nl.devices.iter().map(|d| size::drawn(d, &mut models)).collect();
        let canon = canonical_labels(&hg, &drawn, &models, &roles);
        let names: Vec<&str> = nl.devices.iter().map(|d| d.name.as_str()).collect();
        let all = recognize_all(&hg, &drawn, &roles, &cfg, &canon, &names);
        let sel = select_disjoint(&all, &canon, &names);
        assert!(!sel.is_empty());
        for m in &sel {
            assert!(all.iter().any(|a| a.template == m.template && a.instances == m.instances), "{m:?} not in recognize_all");
        }
        for (i, a) in sel.iter().enumerate() {
            for b in &sel[i + 1..] {
                assert!(a.instances.iter().all(|d| !b.instances.contains(d)), "{a:?} overlaps {b:?}");
            }
        }
        assert!(all.len() > sel.len(), "{} matches, {} selected", all.len(), sel.len());
    }
}
