//! Pattern DSL + backtracking matcher. The patterns themselves are data in
//! [`crate::catalog`].

use pnr_core::ids::NetId;
use pnr_core::netlist::DeviceKind;
use pnr_core::BipartiteHypergraph;

use crate::catalog::PATTERNS;
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

fn is_diode(hg: &BipartiteHypergraph, cell: u32) -> bool {
    let d = pin_net(hg, cell, "D");
    d.is_some() && d == pin_net(hg, cell, "G")
}

fn slot_ok(slot: &Slot, hg: &BipartiteHypergraph, drawn: &[Drawn], cell: u32, assigned: &[u32], roles: &[NetRole]) -> bool {
    let kind = |i: u32| hg.kinds[i as usize];
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
    let (g, gr) = (&drawn[cell as usize], |r: u8| &drawn[other(r) as usize]);
    let size_ok = match slot.size_match {
        SizeMatch::Any => true,
        SizeMatch::ExactAs(r) => size::exact_as(dt, g, gr(r), roles),
        SizeMatch::SameLAs(r) => size::same_l_as(g, gr(r)),
    };
    let diode_ok = match slot.diode {
        DiodeReq::Required => is_diode(hg, cell),
        DiodeReq::Forbidden => !is_diode(hg, cell),
        DiodeReq::Any => true,
    };
    let gate_ok = !slot.gate_is_signal
        || pin_net(hg, cell, "G").map_or(true, |n| roles[n.0 as usize] == NetRole::Signal);
    kind_ok && size_ok && diode_ok && gate_ok
}

/// Links whose both ends are already assigned hold.
fn links_ok(links: &[PinLink], hg: &BipartiteHypergraph, assigned: &[u32], roles: &[NetRole]) -> bool {
    let filled = assigned.len() as u8;
    links.iter().filter(|l| l.a < filled && l.b < filled).all(|l| {
        let na = pin_net(hg, assigned[l.a as usize], l.pin_a);
        let nb = pin_net(hg, assigned[l.b as usize], l.pin_b);
        match l.rel {
            PinRel::Same => na.is_some() && na == nb,
            PinRel::Diff => na != nb,
            PinRel::SameSignal => na.is_some_and(|n| Some(n) == nb && roles[n.0 as usize] == NetRole::Signal),
        }
    })
}

struct Search<'a> {
    pat: &'static Pattern,
    hg: &'a BipartiteHypergraph,
    drawn: &'a [Drawn],
    roles: &'a [NetRole],
    allowed: &'a [bool],
}

impl Search<'_> {
    /// Every assignment of devices to slots; one match per device *set* (the first
    /// slot order found wins).
    fn run(&self, assigned: &mut Vec<u32>, out: &mut Vec<PatternMatch>, seen: &mut Vec<Vec<u32>>) {
        if assigned.len() == self.pat.slots.len() {
            let mut key = assigned.clone();
            key.sort_unstable();
            if !seen.contains(&key) {
                seen.push(key);
                out.push(PatternMatch { template: self.pat.name, pattern: self.pat, instances: assigned.clone(), priority: self.pat.priority });
            }
            return;
        }
        let slot = &self.pat.slots[assigned.len()];
        for cell in 0..self.allowed.len() as u32 {
            if !self.allowed[cell as usize]
                || assigned.contains(&cell)
                || !slot_ok(slot, self.hg, self.drawn, cell, assigned, self.roles)
            {
                continue;
            }
            assigned.push(cell);
            if links_ok(self.pat.links, self.hg, assigned, self.roles) {
                self.run(assigned, out, seen);
            }
            assigned.pop();
        }
    }
}

/// Every match of one pattern over the devices `allowed`.
pub(crate) fn matches(
    pat: &'static Pattern,
    hg: &BipartiteHypergraph,
    drawn: &[Drawn],
    roles: &[NetRole],
    allowed: &[bool],
) -> Vec<PatternMatch> {
    let s = Search { pat, hg, drawn, roles, allowed };
    let mut out = Vec::new();
    s.run(&mut Vec::with_capacity(pat.slots.len()), &mut out, &mut Vec::new());
    out
}

/// Match every catalog pattern with at most `max_slots` slots over the devices in
/// `subset`, and keep a non-overlapping set greedily: priority first, then the
/// sorted device list. `cfg.do_not_identify` devices and `cfg.do_not_use`
/// templates are skipped.
#[must_use]
pub fn recognize(
    hg: &BipartiteHypergraph,
    drawn: &[Drawn],
    roles: &[NetRole],
    cfg: &AnnotationConfig,
    subset: &[u32],
    max_slots: usize,
) -> Vec<PatternMatch> {
    let mut allowed = vec![false; hg.device_count()];
    for &d in subset {
        allowed[d as usize] = !cfg.do_not_identify.contains(&d);
    }
    let mut all = Vec::new();
    for pat in PATTERNS {
        if pat.slots.len() <= max_slots && !cfg.do_not_use.contains(pat.name) {
            all.extend(matches(pat, hg, drawn, roles, &allowed));
        }
    }
    let sorted = |m: &PatternMatch| {
        let mut k = m.instances.clone();
        k.sort_unstable();
        k
    };
    all.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| sorted(a).cmp(&sorted(b))));

    let mut consumed = vec![false; hg.device_count()];
    all.retain(|m| {
        if m.instances.iter().any(|&i| consumed[i as usize]) {
            return false;
        }
        for &i in &m.instances {
            consumed[i as usize] = true;
        }
        true
    });
    all
}
