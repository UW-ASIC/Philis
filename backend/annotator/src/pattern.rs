//! Pattern DSL + backtracking matcher. The patterns themselves are data in
//! [`crate::catalog`].

use pnr_core::ids::NetId;
use pnr_core::netlist::DeviceKind;
use pnr_core::BipartiteHypergraph;

use crate::catalog::PATTERNS;
use crate::netrole::{AnnotationConfig, NetRole};

#[derive(Debug, Clone, Copy)]
pub enum PinRel {
    Same,
    Diff,
}

#[derive(Debug, Clone, Copy)]
pub enum SlotKind {
    AnyFet,
    SameTypeAs(u8),
    ComplementOf(u8),
}

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

/// Device W/L (nm), from SPICE params; `0` when absent.
#[derive(Debug, Clone, Copy, Default)]
pub struct Geom {
    pub w: i64,
    pub l: i64,
}

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub template: &'static str,
    /// Device ids in slot order.
    pub instances: Vec<u32>,
    pub priority: u32,
}

fn pin_net(hg: &BipartiteHypergraph, cell: u32, pin: &str) -> Option<NetId> {
    let i = cell as usize;
    hg.terminals[i].iter().position(|p| p == pin).map(|t| hg.device_nets[i][t])
}

fn is_diode(hg: &BipartiteHypergraph, cell: u32) -> bool {
    let d = pin_net(hg, cell, "D");
    d.is_some() && d == pin_net(hg, cell, "G")
}

fn slot_ok(slot: &Slot, hg: &BipartiteHypergraph, geom: &[Geom], cell: u32, assigned: &[u32], roles: &[NetRole]) -> bool {
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
    let (g, gr) = (geom[cell as usize], |r: u8| geom[other(r) as usize]);
    let size_ok = match slot.size_match {
        SizeMatch::Any => true,
        SizeMatch::ExactAs(r) => g.w == gr(r).w && g.l == gr(r).l,
        SizeMatch::SameLAs(r) => g.l == gr(r).l,
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
fn links_ok(links: &[PinLink], hg: &BipartiteHypergraph, assigned: &[u32]) -> bool {
    let filled = assigned.len() as u8;
    links.iter().filter(|l| l.a < filled && l.b < filled).all(|l| {
        let na = pin_net(hg, assigned[l.a as usize], l.pin_a);
        let nb = pin_net(hg, assigned[l.b as usize], l.pin_b);
        match l.rel {
            PinRel::Same => na.is_some() && na == nb,
            PinRel::Diff => na != nb,
        }
    })
}

struct Search<'a> {
    pat: &'a Pattern,
    hg: &'a BipartiteHypergraph,
    geom: &'a [Geom],
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
                out.push(PatternMatch { template: self.pat.name, instances: assigned.clone(), priority: self.pat.priority });
            }
            return;
        }
        let slot = &self.pat.slots[assigned.len()];
        for cell in 0..self.allowed.len() as u32 {
            if !self.allowed[cell as usize]
                || assigned.contains(&cell)
                || !slot_ok(slot, self.hg, self.geom, cell, assigned, self.roles)
            {
                continue;
            }
            assigned.push(cell);
            if links_ok(self.pat.links, self.hg, assigned) {
                self.run(assigned, out, seen);
            }
            assigned.pop();
        }
    }
}

/// Match every catalog pattern with at most `max_slots` slots over the devices in
/// `subset`, and keep a non-overlapping set greedily: priority first, then the
/// sorted device list. `cfg.do_not_identify` devices and `cfg.do_not_use`
/// templates are skipped.
#[must_use]
pub fn recognize(
    hg: &BipartiteHypergraph,
    geom: &[Geom],
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
            let s = Search { pat, hg, geom, roles, allowed: &allowed };
            s.run(&mut Vec::with_capacity(pat.slots.len()), &mut all, &mut Vec::new());
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
