//! Generic N-device pattern matching engine.
//!
//! Pattern definitions live in `catalog.rs`. This module provides the
//! types (`Pattern`, `Slot`, `PinLink`, …) and the backtracking matcher.
//! It knows nothing about specific analog blocks.

use std::collections::HashSet;

use pnr_cells::netlist::{BipartiteHypergraph, NetId};
use pnr_cells::DeviceType;

use crate::catalog::PATTERNS;
use crate::{AnnotationConfig, NetRole};

// ═══════════════════════════════════════════════════════════════════════
//  Pattern DSL types — all const-compatible
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy)]
pub enum PinRel {
    Same,
    Diff,
}

#[derive(Debug, Clone, Copy)]
pub enum SlotKind {
    AnyFet,
    SameTypeAs(u8),
    #[allow(dead_code)]
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
    pub gate_is_signal: bool,
}

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
    pub priority: u32,
    pub slots: &'static [Slot],
    pub links: &'static [PinLink],
}

// ═══════════════════════════════════════════════════════════════════════
//  Match result
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub template: String,
    pub instances: Vec<u32>,
    pub instance_names: Vec<String>,
    pub priority: u32,
}

// ═══════════════════════════════════════════════════════════════════════
//  Engine internals
// ═══════════════════════════════════════════════════════════════════════

fn pin_net(hg: &BipartiteHypergraph, cell: u32, pin: &str) -> Option<NetId> {
    hg.cells[cell as usize]
        .pins
        .iter()
        .find(|(p, _)| p == pin)
        .map(|(_, n)| *n)
}

fn is_fet(hg: &BipartiteHypergraph, i: u32) -> bool {
    hg.cells[i as usize]
        .device
        .as_ref()
        .is_some_and(|d| matches!(d.device_type, DeviceType::Nmos | DeviceType::Pmos))
}

fn device_type(hg: &BipartiteHypergraph, i: u32) -> Option<DeviceType> {
    hg.cells[i as usize].device.as_ref().map(|d| d.device_type)
}

fn is_signal(net: NetId, roles: &[NetRole]) -> bool {
    roles.get(net as usize).map_or(true, |r| *r == NetRole::Signal)
}

fn is_complement(a: DeviceType, b: DeviceType) -> bool {
    matches!(
        (a, b),
        (DeviceType::Nmos, DeviceType::Pmos) | (DeviceType::Pmos, DeviceType::Nmos)
    )
}

fn slot_ok(
    slot: &Slot,
    hg: &BipartiteHypergraph,
    cell: u32,
    assigned: &[u32],
    roles: &[NetRole],
) -> bool {
    if !is_fet(hg, cell) {
        return false;
    }
    let dt = match device_type(hg, cell) {
        Some(dt) => dt,
        None => return false,
    };
    let dev = hg.cells[cell as usize].device.as_ref().unwrap();

    match slot.kind {
        SlotKind::AnyFet => {}
        SlotKind::SameTypeAs(r) => {
            if let Some(&rc) = assigned.get(r as usize) {
                if device_type(hg, rc) != Some(dt) {
                    return false;
                }
            }
        }
        SlotKind::ComplementOf(r) => {
            if let Some(&rc) = assigned.get(r as usize) {
                if let Some(rdt) = device_type(hg, rc) {
                    if !is_complement(dt, rdt) {
                        return false;
                    }
                }
            }
        }
    }

    match slot.size_match {
        SizeMatch::Any => {}
        SizeMatch::ExactAs(r) => {
            if let Some(&rc) = assigned.get(r as usize) {
                let rd = hg.cells[rc as usize].device.as_ref().unwrap();
                if dev.w != rd.w || dev.l != rd.l {
                    return false;
                }
            }
        }
        SizeMatch::SameLAs(r) => {
            if let Some(&rc) = assigned.get(r as usize) {
                let rd = hg.cells[rc as usize].device.as_ref().unwrap();
                if dev.l != rd.l {
                    return false;
                }
            }
        }
    }

    let is_diode =
        pin_net(hg, cell, "D") == pin_net(hg, cell, "G") && pin_net(hg, cell, "D").is_some();
    match slot.diode {
        DiodeReq::Required if !is_diode => return false,
        DiodeReq::Forbidden if is_diode => return false,
        _ => {}
    }

    if slot.gate_is_signal {
        if let Some(g) = pin_net(hg, cell, "G") {
            if !is_signal(g, roles) {
                return false;
            }
        }
    }

    true
}

fn partial_links_ok(links: &[PinLink], hg: &BipartiteHypergraph, assigned: &[u32]) -> bool {
    let filled = assigned.len() as u8;
    for link in links {
        if link.a >= filled || link.b >= filled {
            continue;
        }
        let na = pin_net(hg, assigned[link.a as usize], link.pin_a);
        let nb = pin_net(hg, assigned[link.b as usize], link.pin_b);
        match link.rel {
            PinRel::Same => {
                if na != nb || na.is_none() {
                    return false;
                }
            }
            PinRel::Diff => {
                if na == nb {
                    return false;
                }
            }
        }
    }
    true
}

fn find_matches(
    pat: &Pattern,
    hg: &BipartiteHypergraph,
    roles: &[NetRole],
    blocked: &HashSet<String>,
) -> Vec<PatternMatch> {
    let mut results = Vec::new();
    let mut assigned: Vec<u32> = Vec::with_capacity(pat.slots.len());
    backtrack(pat, hg, roles, blocked, 0, &mut assigned, &mut results);
    results
}

fn backtrack(
    pat: &Pattern,
    hg: &BipartiteHypergraph,
    roles: &[NetRole],
    blocked: &HashSet<String>,
    slot_idx: usize,
    assigned: &mut Vec<u32>,
    results: &mut Vec<PatternMatch>,
) {
    if slot_idx == pat.slots.len() {
        let mut pairs: Vec<(u32, String)> = assigned
            .iter()
            .map(|&i| (i, hg.cells[i as usize].name.clone()))
            .collect();
        pairs.sort_by_key(|&(id, _)| id);
        let key: Vec<u32> = pairs.iter().map(|p| p.0).collect();
        if !results.iter().any(|r| r.instances == key) {
            results.push(PatternMatch {
                template: pat.name.into(),
                instances: key,
                instance_names: pairs.into_iter().map(|p| p.1).collect(),
                priority: pat.priority,
            });
        }
        return;
    }

    let n = hg.cells.len() as u32;
    for cell in 0..n {
        if assigned.contains(&cell) {
            continue;
        }
        if blocked.contains(&hg.cells[cell as usize].name) {
            continue;
        }
        if !slot_ok(&pat.slots[slot_idx], hg, cell, assigned, roles) {
            continue;
        }
        assigned.push(cell);
        if partial_links_ok(pat.links, hg, assigned) {
            backtrack(pat, hg, roles, blocked, slot_idx + 1, assigned, results);
        }
        assigned.pop();
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Public API
// ═══════════════════════════════════════════════════════════════════════

/// Run all catalog patterns, return non-overlapping matches (greedy, priority-first).
pub fn recognize(
    hg: &BipartiteHypergraph,
    roles: &[NetRole],
    cfg: &AnnotationConfig,
) -> Vec<PatternMatch> {
    let blocked = &cfg.do_not_identify;
    let mut all = Vec::new();

    for pat in PATTERNS {
        if cfg.do_not_use.contains(pat.name) {
            continue;
        }
        all.extend(find_matches(pat, hg, roles, blocked));
    }

    all.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| a.instance_names.cmp(&b.instance_names))
    });

    let mut consumed: HashSet<u32> = HashSet::new();
    let mut selected = Vec::new();
    for m in all {
        if m.instances.iter().any(|i| consumed.contains(i)) {
            continue;
        }
        for &i in &m.instances {
            consumed.insert(i);
        }
        selected.push(m);
    }
    selected
}
