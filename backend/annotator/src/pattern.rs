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

/// How the two pins of a [`PinLink`] relate.
#[derive(Debug, Clone, Copy)]
pub enum PinRel {
    /// Same net (both pins present).
    Same,
    /// Different nets; an absent pin differs from any present one.
    Diff,
    /// Same net, and that net's role is `Signal`.
    SameSignal,
}

/// Which device kinds may fill a [`Slot`]. A `u8` names an earlier slot of the
/// same pattern.
#[derive(Debug, Clone, Copy)]
pub enum SlotKind {
    /// Any NMOS or PMOS.
    AnyFet,
    /// A FET of the same polarity as slot `r`.
    SameTypeAs(u8),
    /// A FET of the opposite polarity to slot `r` (which must be a FET).
    ComplementOf(u8),
    /// Exactly this device kind (EXT-19: bipolar slots).
    Kind(DeviceKind),
    /// The same device kind as slot `r`, any family.
    SameKindAs(u8),
}

/// Size relation to slot `r`, over [`Drawn`]: `ExactAs` is [`size::exact_as`] (known
/// W/L, model and bulk), `SameLAs` is [`size::same_l_as`] (known L and model).
#[derive(Debug, Clone, Copy)]
pub enum SizeMatch {
    /// No size relation.
    Any,
    ExactAs(u8),
    SameLAs(u8),
}

/// Whether a slot's device must be diode-connected (FET `D == G`, BJT `C == B`).
#[derive(Debug, Clone, Copy)]
pub enum DiodeReq {
    /// Must be diode-connected.
    Required,
    /// Must not be diode-connected.
    Forbidden,
    /// Either.
    Any,
}

/// One device position of a [`Pattern`]: the unary and slot-relative rules a
/// candidate device must pass.
#[derive(Debug, Clone, Copy)]
pub struct Slot {
    /// Device family rule.
    pub kind: SlotKind,
    /// Size relation to an earlier slot.
    pub size_match: SizeMatch,
    /// Diode-connection rule.
    pub diode: DiodeReq,
    /// The gate net (a BJT's base) must be a signal (not a rail/clock); an
    /// absent gate pin passes.
    pub gate_is_signal: bool,
}

/// Slot `a`'s pin `pin_a` and slot `b`'s pin `pin_b` relate by `rel`. Pin
/// names are terminal names (G, D, S, B, C, E, P, N); any other name reads as
/// an absent pin.
#[derive(Debug, Clone, Copy)]
pub struct PinLink {
    /// First slot index.
    pub a: u8,
    /// Pin of slot `a`.
    pub pin_a: &'static str,
    /// Second slot index.
    pub b: u8,
    /// Pin of slot `b`.
    pub pin_b: &'static str,
    /// Required relation between the two pins' nets.
    pub rel: PinRel,
}

/// A topology template: slots plus the pin links among them. Invariant (relied
/// on by the matcher's slot order): every slot reference in `slots` points to an earlier
/// slot, and every link's slot indices are `< slots.len()`.
#[derive(Debug, Clone, Copy)]
pub struct Pattern {
    /// Template name, unique in [`PATTERNS`].
    pub name: &'static str,
    /// Higher wins when two matches overlap.
    pub priority: u32,
    /// Device positions, in slot order.
    pub slots: &'static [Slot],
    /// Pin relations among the slots.
    pub links: &'static [PinLink],
}

/// One assignment of devices to a pattern's slots.
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// The pattern's name (`pattern.name`).
    pub template: &'static str,
    /// The matched pattern (its roles: [`crate::catalog::roles_of`]).
    pub pattern: &'static Pattern,
    /// Device ids in slot order; distinct.
    pub instances: Vec<u32>,
    /// The pattern's priority (`pattern.priority`).
    pub priority: u32,
}

/// The net on device `cell`'s terminal named `pin`, or `None` when it has no
/// such terminal.
///
/// # Panics
/// When `cell` is not a device of `hg`.
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

/// [`on_pin`] column half: 0 NMOS or NPN, 1 PMOS or PNP.
fn pol(k: DeviceKind) -> usize {
    usize::from(matches!(k, DeviceKind::Pmos | DeviceKind::Pnp))
}

/// NMOS or PMOS.
pub(crate) fn fet(k: DeviceKind) -> bool {
    matches!(k, DeviceKind::Nmos | DeviceKind::Pmos)
}

/// NPN or PNP.
fn bjt(k: DeviceKind) -> bool {
    matches!(k, DeviceKind::Npn | DeviceKind::Pnp)
}

/// Per net, the FETs and BJTs with [`pin_index`] pin `i` on that net and polarity [`pol`]
/// `p` at column `2i + p` (ascending id), so a candidate list holds only devices
/// that can satisfy the link and the slot's polarity: a rail's thousands of
/// sources never stand in for its few drains, nor NMOS for PMOS. Other kinds
/// are left out (no pattern slot takes them).
pub(crate) fn on_pin(hg: &BipartiteHypergraph, pins: &[[Option<NetId>; 8]]) -> Vec<[Vec<u32>; 16]> {
    let mut on = vec![<[Vec<u32>; 16]>::default(); hg.net_devices.len()];
    for (d, row) in pins.iter().enumerate() {
        if !fet(hg.kinds[d]) && !bjt(hg.kinds[d]) {
            continue;
        }
        for (i, n) in row.iter().enumerate() {
            if let Some(n) = n {
                on[n.0 as usize][2 * i + pol(hg.kinds[d])].push(d as u32);
            }
        }
    }
    on
}

/// [`pin_net`] over the precomputed [`pins`] table.
fn pin_of(pins: &[[Option<NetId>; 8]], cell: u32, pin: &str) -> Option<NetId> {
    pin_index(pin).and_then(|i| pins[cell as usize][i])
}

/// FET `D == G` or BJT `C == B`.
fn is_diode(pins: &[[Option<NetId>; 8]], kind: DeviceKind, cell: u32) -> bool {
    let (d, g) = if bjt(kind) { ("C", "B") } else { ("D", "G") };
    let d = pin_of(pins, cell, d);
    d.is_some() && d == pin_of(pins, cell, g)
}

/// The part of [`slot_ok`] that reads no other slot: the family, diode and
/// gate (a BJT's base) rules.
fn unary_ok(slot: &Slot, s: &Search, cell: u32) -> bool {
    let k = s.hg.kinds[cell as usize];
    let diode_ok = match slot.diode {
        DiodeReq::Required => is_diode(s.pins, k, cell),
        DiodeReq::Forbidden => !is_diode(s.pins, k, cell),
        DiodeReq::Any => true,
    };
    let gate_ok = !slot.gate_is_signal
        || pin_of(s.pins, cell, if bjt(k) { "B" } else { "G" }).map_or(true, |n| s.roles[n.0 as usize] == NetRole::Signal);
    let family_ok = match slot.kind {
        SlotKind::AnyFet | SlotKind::SameTypeAs(_) | SlotKind::ComplementOf(_) => fet(k),
        SlotKind::Kind(want) => k == want,
        SlotKind::SameKindAs(_) => true,
    };
    family_ok && diode_ok && gate_ok
}

/// Whether `cell` may fill `slot`: [`unary_ok`] plus the kind and size relations
/// to earlier slots. `assigned` is indexed by slot; every slot `slot` refers to
/// is assigned ([`slot_order`]).
fn slot_ok(slot: &Slot, s: &Search, cell: u32, assigned: &[u32]) -> bool {
    if !unary_ok(slot, s, cell) {
        return false;
    }
    let kind = |i: u32| s.hg.kinds[i as usize];
    let dt = kind(cell);
    let other = |r: u8| assigned[r as usize];
    let kind_ok = match slot.kind {
        SlotKind::AnyFet => true,
        SlotKind::SameTypeAs(r) => kind(other(r)) == dt,
        SlotKind::ComplementOf(r) => kind(other(r)) != dt && fet(kind(other(r))),
        SlotKind::Kind(_) => true,
        SlotKind::SameKindAs(r) => kind(other(r)) == dt,
    };
    let (g, gr) = (&s.drawn[cell as usize], |r: u8| &s.drawn[other(r) as usize]);
    let size_ok = match slot.size_match {
        SizeMatch::Any => true,
        SizeMatch::ExactAs(r) => size::exact_as(dt, g, gr(r), s.roles),
        SizeMatch::SameLAs(r) => size::same_l_as(g, gr(r)),
    };
    kind_ok && size_ok
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
                SlotKind::AnyFet | SlotKind::Kind(_) => None,
                SlotKind::SameTypeAs(r) | SlotKind::ComplementOf(r) | SlotKind::SameKindAs(r) => Some(r),
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

/// One pattern's backtracking search over the per-netlist tables.
struct Search<'a> {
    pat: &'static Pattern,
    /// Slot fill order ([`slot_order`]).
    order: Vec<usize>,
    hg: &'a BipartiteHypergraph,
    pins: &'a [[Option<NetId>; 8]],
    on: &'a [[Vec<u32>; 16]],
    drawn: &'a [Drawn],
    roles: &'a [NetRole],
    /// Per device: may join a match (not `do_not_identify`).
    allowed: &'a [bool],
    /// Per device: position in `(canon, name)` order; picks the representative
    /// slot assignment of a device set.
    rank: &'a [u32],
    /// Per slot, the devices [`Candidates::All`] tries; filled on first use.
    free: Vec<std::cell::OnceCell<Vec<u32>>>,
}

/// Where the next slot's devices come from.
enum Candidates {
    /// Devices with pin column `.1` on net `.0` and a polarity in `.2` ([`on_pin`]).
    Net(NetId, usize, std::ops::Range<usize>),
    /// A linking net is absent or not a signal where one must be: nothing fits.
    None,
    /// No link to an assigned slot: every allowed device passing [`unary_ok`]
    /// (listed once per slot, not rescanned per partial assignment).
    All,
}

impl Search<'_> {
    /// Over links joining slot `k`'s pin to an assigned slot's pin by `Same` or
    /// `SameSignal`, the (net, pin) with the fewest devices of the slot's polarity.
    fn candidates(&self, k: u8, assigned: &[u32]) -> Candidates {
        let at = |r: u8| pol(self.hg.kinds[assigned[r as usize] as usize]);
        let pols = match self.pat.slots[k as usize].kind {
            SlotKind::AnyFet => 0..2,
            SlotKind::SameTypeAs(r) => at(r)..at(r) + 1,
            SlotKind::ComplementOf(r) => 1 - at(r)..2 - at(r),
            SlotKind::Kind(k) => pol(k)..pol(k) + 1,
            SlotKind::SameKindAs(r) => at(r)..at(r) + 1,
        };
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
            let len = |n: NetId, pk: usize| pols.clone().map(|p| self.on[n.0 as usize][2 * pk + p].len()).sum::<usize>();
            if best.map_or(true, |(b, bk)| len(n, pk) < len(b, bk)) {
                best = Some((n, pk));
            }
        }
        best.map_or(Candidates::All, |(n, pk)| Candidates::Net(n, pk, pols))
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
            Candidates::Net(n, pk, pols) => pols.for_each(|p| self.on[n.0 as usize][2 * pk + p].iter().for_each(|&c| try_cell(c, assigned))),
            Candidates::None => {}
            Candidates::All => {
                let free = self.free[k].get_or_init(|| {
                    (0..self.allowed.len() as u32).filter(|&c| self.allowed[c as usize] && unary_ok(&self.pat.slots[k], self, c)).collect()
                });
                free.iter().for_each(|&c| try_cell(c, assigned));
            }
        }
    }
}

/// Every match of one pattern over the devices `allowed`; `pins` from [`pins`], `on` from [`on_pin`],
/// `rank` (by device) picks a device set's slot assignment ([`Search::run`]).
#[allow(clippy::too_many_arguments)] // the per-netlist tables are built once by the caller
pub(crate) fn matches(
    pat: &'static Pattern,
    hg: &BipartiteHypergraph,
    pins: &[[Option<NetId>; 8]],
    on: &[[Vec<u32>; 16]],
    drawn: &[Drawn],
    roles: &[NetRole],
    allowed: &[bool],
    rank: &[u32],
) -> Vec<PatternMatch> {
    let free = vec![std::cell::OnceCell::new(); pat.slots.len()];
    let s = Search { pat, order: slot_order(pat), hg, pins, on, drawn, roles, allowed, rank, free };
    let mut out = Vec::new();
    s.run(0, &mut vec![EMPTY; pat.slots.len()], &mut out, &mut HashMap::new());
    out
}

/// Every match of every allowed pattern; one per (template, device set), its
/// slots assigned by least `(canon, name)` per slot ([`canonical_labels`],
/// device names), so the result does not depend on netlist order.
/// `cfg.do_not_identify` devices and `cfg.do_not_use` templates are skipped.
/// `drawn`, `canon` and `names` are per device.
///
/// Cost: a backtracking search per pattern, bounded in practice by the
/// candidate lists of [`on_pin`]; worst case exponential in slot count.
#[must_use]
pub fn recognize_all(hg: &BipartiteHypergraph, drawn: &[Drawn], roles: &[NetRole], cfg: &AnnotationConfig, canon: &[u64], names: &[&str]) -> Vec<PatternMatch> {
    let allowed: Vec<bool> = (0..hg.device_count() as u32).map(|d| !cfg.do_not_identify.contains(&d)).collect();
    let pins = pins(hg);
    let on = on_pin(hg, &pins);
    let mut by: Vec<usize> = (0..canon.len()).collect();
    by.sort_by_key(|&d| (canon[d], names[d]));
    let mut rank = vec![0u32; by.len()];
    by.iter().enumerate().for_each(|(r, &d)| rank[d] = r as u32);
    PATTERNS
        .iter()
        .filter(|p| !cfg.do_not_use.contains(p.name))
        .flat_map(|p| matches(p, hg, &pins, &on, drawn, roles, &allowed, &rank))
        .collect()
}

/// Deterministic 64-bit hash (fixed-key SipHash).
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
///
/// # Panics
/// When `drawn` is shorter than the device count or a `drawn` model index is
/// out of range of `models`.
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
/// The result is pairwise device-disjoint and independent of the order of `all`.
///
/// # Panics
/// When a match names a device outside `canon`/`names`.
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

    /// [`select_disjoint`] over `devs` (name, label) and equal-priority `ms`, device
    /// ids and input order optionally reversed: the selected members' sorted names.
    fn pick(devs: &[(&'static str, u64)], ms: &[(&'static Pattern, [&str; 2])], rev_ids: bool, rev_in: bool) -> Vec<Vec<&'static str>> {
        let mut order: Vec<usize> = (0..devs.len()).collect();
        if rev_ids {
            order.reverse();
        }
        let names: Vec<&'static str> = order.iter().map(|&i| devs[i].0).collect();
        let canon: Vec<u64> = order.iter().map(|&i| devs[i].1).collect();
        let id = |n: &str| names.iter().position(|&x| x == n).unwrap() as u32;
        let mut all: Vec<PatternMatch> =
            ms.iter().map(|&(p, [a, b])| PatternMatch { template: p.name, pattern: p, instances: vec![id(a), id(b)], priority: 10 }).collect();
        if rev_in {
            all.reverse();
        }
        let sorted = |m: &PatternMatch| {
            let mut v: Vec<&'static str> = m.instances.iter().map(|&i| names[i as usize]).collect();
            v.sort_unstable();
            v
        };
        select_disjoint(&all, &canon, &names).iter().map(sorted).collect()
    }

    /// EXT-06's key below priority: a match declaring pairs beats an overlapping one
    /// declaring none, even with larger labels; label-identical (automorphic) matches
    /// fall to sorted names. Neither depends on device ids or input order.
    #[test]
    fn select_disjoint_prefers_declared_pairs_then_names() {
        use crate::catalog::{DIFF_PAIR, PUSH_PULL_PAIR};
        let roles = [("A", 5), ("B", 5), ("C", 1)];
        let twins = [("E", 7), ("F", 7), ("G", 7)];
        for (rev_ids, rev_in) in [(false, false), (false, true), (true, false), (true, true)] {
            let got = pick(&roles, &[(&DIFF_PAIR, ["A", "B"]), (&PUSH_PULL_PAIR, ["B", "C"])], rev_ids, rev_in);
            assert_eq!(got, [["A", "B"]], "declared pairs first ({rev_ids}, {rev_in})");
            let got = pick(&twins, &[(&DIFF_PAIR, ["E", "F"]), (&DIFF_PAIR, ["F", "G"])], rev_ids, rev_in);
            assert_eq!(got, [["E", "F"]], "names break label ties ({rev_ids}, {rev_in})");
        }
    }
}

/// Step-2 coverage: the pin tables, the slot order, the matcher and the
/// selection, against their doc comments and the catalog's invariants.
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::tests::{fet, nets, ota};
    use pnr_core::netlist::{Device, Netlist};

    /// Slots slot `k` references through its kind or size rule.
    fn refs(s: &Slot) -> Vec<u8> {
        let kr = match s.kind {
            SlotKind::AnyFet | SlotKind::Kind(_) => None,
            SlotKind::SameTypeAs(r) | SlotKind::ComplementOf(r) | SlotKind::SameKindAs(r) => Some(r),
        };
        let sr = match s.size_match {
            SizeMatch::Any => None,
            SizeMatch::ExactAs(r) | SizeMatch::SameLAs(r) => Some(r),
        };
        [kr, sr].into_iter().flatten().collect()
    }

    /// The catalog invariants `Pattern` documents: references to earlier
    /// slots only, link ends in range, unique names.
    #[test]
    fn catalog_respects_pattern_invariants() {
        let mut names: Vec<&str> = PATTERNS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n, "pattern names are unique");
        for p in PATTERNS {
            assert!(!p.slots.is_empty(), "{}", p.name);
            for (k, s) in p.slots.iter().enumerate() {
                assert!(refs(s).iter().all(|&r| (r as usize) < k), "{} slot {k} references a later slot", p.name);
            }
            for l in p.links {
                assert!((l.a as usize) < p.slots.len() && (l.b as usize) < p.slots.len(), "{} link out of range", p.name);
            }
        }
    }

    /// `slot_order` is a permutation starting at slot 0 in which every slot's
    /// references are placed before it.
    #[test]
    fn slot_order_is_a_dependency_respecting_permutation() {
        for p in PATTERNS {
            let order = slot_order(p);
            assert_eq!(order[0], 0, "{}", p.name);
            let mut sorted = order.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, (0..p.slots.len()).collect::<Vec<_>>(), "{}", p.name);
            let pos = |k: usize| order.iter().position(|&o| o == k).unwrap();
            for (k, s) in p.slots.iter().enumerate() {
                assert!(refs(s).iter().all(|&r| pos(r as usize) < pos(k)), "{} slot {k}", p.name);
            }
        }
    }

    /// A strong link (not S/B to S/B) is preferred over a rail-only one.
    #[test]
    fn slot_order_prefers_strong_links() {
        static SLOTS: [Slot; 3] = [
            Slot { kind: SlotKind::AnyFet, size_match: SizeMatch::Any, diode: DiodeReq::Any, gate_is_signal: false },
            Slot { kind: SlotKind::AnyFet, size_match: SizeMatch::Any, diode: DiodeReq::Any, gate_is_signal: false },
            Slot { kind: SlotKind::AnyFet, size_match: SizeMatch::Any, diode: DiodeReq::Any, gate_is_signal: false },
        ];
        static LINKS: [PinLink; 2] = [
            PinLink { a: 0, pin_a: "S", b: 1, pin_b: "S", rel: PinRel::Same },
            PinLink { a: 0, pin_a: "D", b: 2, pin_b: "G", rel: PinRel::Same },
        ];
        static P: Pattern = Pattern { name: "t", priority: 1, slots: &SLOTS, links: &LINKS };
        assert_eq!(slot_order(&P), [0, 2, 1]);
    }

    #[test]
    fn pin_tables() {
        assert_eq!(["G", "D", "S", "B", "C", "E", "P", "N"].map(pin_index), [0usize, 1, 2, 3, 4, 5, 6, 7].map(Some));
        assert_eq!(pin_index("X"), None);
        let odd = Device { name: "R".into(), kind: DeviceKind::Resistor, model: String::new(), terminals: vec![("P".into(), NetId(0)), ("Q".into(), NetId(1))], params: vec![] };
        let nl = Netlist {
            devices: vec![fet("M1", DeviceKind::Pmos, 0, 1, 2, 2, 1_000, 1_000), odd, fet("M2", DeviceKind::Nmos, 0, 0, 1, 1, 1_000, 1_000)],
            nets: nets(&["a", "b", "c"]),
            ..Default::default()
        };
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let p = pins(&hg);
        assert_eq!(p[1][6], Some(NetId(0)));
        assert!(p[1].iter().enumerate().all(|(i, n)| i == 6 || n.is_none()), "an unknown pin name reads as absent");
        assert_eq!(pin_net(&hg, 1, "Q"), Some(NetId(1)), "pin_net reads any name");
        assert_eq!(pin_of(&p, 1, "Q"), None);
        assert_eq!(pin_net(&hg, 0, "C"), None);
        let on = on_pin(&hg, &p);
        assert_eq!(on.len(), 3);
        assert_eq!(on[0][1], [0], "PMOS gate on a: column 2·G + 1");
        assert_eq!(on[0][0], [2], "NMOS gate on a: column 2·G");
        assert_eq!(on[0][2], [2], "NMOS drain on a");
        assert!(on[0][12].is_empty() && on[0][13].is_empty(), "the resistor is not listed");
        assert!(is_diode(&p, DeviceKind::Nmos, 2) && !is_diode(&p, DeviceKind::Pmos, 0));
    }

    #[test]
    fn bjt_diode_is_c_equals_b() {
        let q = |c: u16, b: u16| Device {
            name: "Q".into(),
            kind: DeviceKind::Npn,
            model: String::new(),
            terminals: vec![("C".into(), NetId(c)), ("B".into(), NetId(b)), ("E".into(), NetId(2))],
            params: vec![],
        };
        let nl = Netlist { devices: vec![q(0, 0), q(0, 1)], nets: nets(&["a", "b", "e"]), ..Default::default() };
        let p = pins(&BipartiteHypergraph::from_netlist(&nl));
        assert!(is_diode(&p, DeviceKind::Npn, 0));
        assert!(!is_diode(&p, DeviceKind::Npn, 1));
    }

    /// Every match's instances are distinct devices, in range, and its
    /// template and priority mirror its pattern.
    #[test]
    fn matches_are_well_formed() {
        let nl = crate::tests::three_stage();
        let (hg, drawn, models, roles) = tables(&nl);
        let canon = canonical_labels(&hg, &drawn, &models, &roles);
        let names: Vec<&str> = nl.devices.iter().map(|d| d.name.as_str()).collect();
        let all = recognize_all(&hg, &drawn, &roles, &AnnotationConfig::default(), &canon, &names);
        assert!(!all.is_empty());
        for m in &all {
            assert_eq!(m.instances.len(), m.pattern.slots.len());
            assert_eq!((m.template, m.priority), (m.pattern.name, m.pattern.priority));
            let mut v = m.instances.clone();
            v.sort_unstable();
            v.dedup();
            assert_eq!(v.len(), m.instances.len(), "{m:?} repeats a device");
            assert!(m.instances.iter().all(|&d| (d as usize) < nl.devices.len()));
        }
        let mut sets: Vec<(&str, Vec<u32>)> = all.iter().map(|m| {
            let mut v = m.instances.clone();
            v.sort_unstable();
            (m.template, v)
        }).collect();
        let n = sets.len();
        sets.sort();
        sets.dedup();
        assert_eq!(sets.len(), n, "one match per (template, device set)");
    }

    fn tables(nl: &Netlist) -> (BipartiteHypergraph, Vec<Drawn>, Vec<String>, Vec<NetRole>) {
        let hg = BipartiteHypergraph::from_netlist(nl);
        let roles = crate::netrole::classify_nets(&hg, &AnnotationConfig::default());
        let mut models = Vec::new();
        let drawn: Vec<Drawn> = nl.devices.iter().map(|d| size::drawn(d, &mut models)).collect();
        (hg, drawn, models, roles)
    }

    /// `(template, sorted member names)` of every match and of the selection.
    fn named(nl: &Netlist) -> (Vec<(String, Vec<String>)>, Vec<(String, Vec<String>)>) {
        let (hg, drawn, models, roles) = tables(nl);
        let canon = canonical_labels(&hg, &drawn, &models, &roles);
        let names: Vec<&str> = nl.devices.iter().map(|d| d.name.as_str()).collect();
        let all = recognize_all(&hg, &drawn, &roles, &AnnotationConfig::default(), &canon, &names);
        let key = |m: &PatternMatch| (m.template.to_string(), m.instances.iter().map(|&d| names[d as usize].to_string()).collect::<Vec<_>>());
        let mut a: Vec<_> = all.iter().map(key).collect();
        let s: Vec<_> = select_disjoint(&all, &canon, &names).iter().map(key).collect();
        a.sort();
        (a, s)
    }

    /// Metamorphic: reversing the device order changes neither the matches
    /// (slot assignment included) nor the selection order.
    #[test]
    fn recognition_is_device_order_invariant() {
        for nl in [ota(), crate::tests::three_stage()] {
            let rev = Netlist { devices: nl.devices.iter().rev().cloned().collect(), nets: nl.nets.clone(), ..Default::default() };
            assert_eq!(named(&nl), named(&rev));
        }
    }

    /// Labels are a function of the device, not its id.
    #[test]
    fn canonical_labels_follow_the_device() {
        let nl = ota();
        let rev = Netlist { devices: nl.devices.iter().rev().cloned().collect(), nets: nl.nets.clone(), ..Default::default() };
        let label = |nl: &Netlist| {
            let (hg, drawn, models, roles) = tables(nl);
            let c = canonical_labels(&hg, &drawn, &models, &roles);
            let mut v: Vec<(String, u64)> = nl.devices.iter().zip(c).map(|(d, l)| (d.name.clone(), l)).collect();
            v.sort();
            v
        };
        let a = label(&nl);
        assert_eq!(a, label(&rev));
        let of = |n: &str| a.iter().find(|x| x.0 == n).unwrap().1;
        assert_eq!(of("XM1"), of("XM2"), "the input pair is automorphic");
        assert_ne!(of("XM1"), of("XM5"), "the tail is not");
        let (hg, drawn, models, roles) = tables(&Netlist::default());
        assert!(canonical_labels(&hg, &drawn, &models, &roles).is_empty());
    }

    #[test]
    fn config_skips_devices_and_templates() {
        let nl = ota();
        let (hg, drawn, models, roles) = tables(&nl);
        let canon = canonical_labels(&hg, &drawn, &models, &roles);
        let names: Vec<&str> = nl.devices.iter().map(|d| d.name.as_str()).collect();
        let mut cfg = AnnotationConfig::default();
        cfg.do_not_identify.insert(0);
        cfg.do_not_use.insert("diff_pair".into());
        let all = recognize_all(&hg, &drawn, &roles, &cfg, &canon, &names);
        assert!(all.iter().all(|m| !m.instances.contains(&0) && m.template != "diff_pair"));
        cfg.do_not_use = PATTERNS.iter().map(|p| p.name.to_string()).collect();
        assert!(recognize_all(&hg, &drawn, &roles, &cfg, &canon, &names).is_empty());
    }

    #[test]
    fn select_disjoint_edges() {
        assert!(select_disjoint(&[], &[], &[]).is_empty());
        // A higher priority wins an overlap whatever the labels.
        use crate::catalog::{DIFF_PAIR, PUSH_PULL_PAIR};
        let m = |p: &'static Pattern, i: Vec<u32>, priority| PatternMatch { template: p.name, pattern: p, instances: i, priority };
        let all = [m(&DIFF_PAIR, vec![0, 1], 1), m(&PUSH_PULL_PAIR, vec![1, 2], 9)];
        let got = select_disjoint(&all, &[0, 0, 0], &["a", "b", "c"]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].instances, [1, 2]);
    }
}
