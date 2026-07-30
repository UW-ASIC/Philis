//! Generic N-device pattern-matching engine — the **single strict recogniser**.
//!
//! Ported from `frontend/annotator/src/pattern.rs` onto `pnr_core` types. Pattern
//! definitions live in [`crate::catalog`]; this module owns the DSL types
//! (`Pattern`, `Slot`, `PinLink`, …) and the backtracking matcher. It knows
//! nothing about specific analog blocks — the catalog is data, the engine is
//! mechanism.
//!
//! ## What changed in the port
//!
//! The frontend engine read device geometry off its hypergraph (`CellNode` carried
//! `DeviceRecord { w, l, nf }`). `pnr_core::BipartiteHypergraph` deliberately does
//! *not* carry params (they live on [`pnr_core::netlist::Device::params`]), so the
//! matcher takes a parallel [`Geom`] slice the annotator builds from the netlist —
//! keeping the geometry channel out of the foundation graph. Pin lookup is by
//! *name* (`hg.terminals` / `hg.device_nets`), exactly as [`crate::strict`] already
//! does, rather than the positional G/D/S the old positional recognisers used.
//!
//! This is the ALIGN template-library path (a data catalog matched by bounded
//! subgraph assignment) where each per-slot / per-link check is a MAGICAL-style
//! pin-relation predicate — the hybrid both tools converge on for primitives.

use std::collections::HashSet;

use pnr_core::ids::NetId;
use pnr_core::netlist::DeviceKind;
use pnr_core::BipartiteHypergraph;

use crate::catalog::PATTERNS;
use crate::netrole::{AnnotationConfig, NetRole};

// ═══════════════════════════════════════════════════════════════════════
//  Pattern DSL types — all const-compatible (verbatim from frontend)
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
//  Device geometry — the W/L channel the pnr_core hypergraph doesn't carry
// ═══════════════════════════════════════════════════════════════════════

/// W/L for one device, in `nm`/PDK units, parallel to `hg` device ids. Built by
/// the annotator from [`pnr_core::netlist::Device::params`]; the matcher needs it
/// for [`SizeMatch`]. Absent params read back as `0` (matches "unknown", i.e. the
/// `SizeMatch` compares equal only when both are the same value).
#[derive(Debug, Clone, Copy, Default)]
pub struct Geom {
    pub w: i64,
    pub l: i64,
}

// ═══════════════════════════════════════════════════════════════════════
//  Match result
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub template: &'static str,
    /// Device ids (into `hg`) forming this match, sorted ascending.
    pub instances: Vec<u32>,
    pub priority: u32,
}

// ═══════════════════════════════════════════════════════════════════════
//  Engine internals
// ═══════════════════════════════════════════════════════════════════════

/// Net on device `cell`'s pin named `pin` (the catalog's `pin_net`). `None` if the
/// device has no such terminal. Pin lookup by name — `pnr_core` carries names on
/// `hg.terminals`.
fn pin_net(hg: &BipartiteHypergraph, cell: u32, pin: &str) -> Option<NetId> {
    let i = cell as usize;
    let names = hg.terminals.get(i)?;
    let nets = hg.device_nets.get(i)?;
    names.iter().position(|p| p == pin).map(|t| nets[t])
}

fn is_fet(hg: &BipartiteHypergraph, i: u32) -> bool {
    matches!(hg.kinds.get(i as usize), Some(DeviceKind::Nmos | DeviceKind::Pmos))
}

fn device_type(hg: &BipartiteHypergraph, i: u32) -> Option<DeviceKind> {
    hg.kinds.get(i as usize).copied()
}

fn is_signal(net: NetId, roles: &[NetRole]) -> bool {
    roles.get(net.0 as usize).map_or(true, |r| *r == NetRole::Signal)
}

fn is_complement(a: DeviceKind, b: DeviceKind) -> bool {
    matches!(
        (a, b),
        (DeviceKind::Nmos, DeviceKind::Pmos) | (DeviceKind::Pmos, DeviceKind::Nmos)
    )
}

fn is_diode(hg: &BipartiteHypergraph, cell: u32) -> bool {
    let (d, g) = (pin_net(hg, cell, "D"), pin_net(hg, cell, "G"));
    d.is_some() && d == g
}

fn slot_ok(
    slot: &Slot,
    hg: &BipartiteHypergraph,
    geom: &[Geom],
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

    let g = |i: u32| geom.get(i as usize).copied().unwrap_or_default();
    match slot.size_match {
        SizeMatch::Any => {}
        SizeMatch::ExactAs(r) => {
            if let Some(&rc) = assigned.get(r as usize) {
                let (a, b) = (g(cell), g(rc));
                if a.w != b.w || a.l != b.l {
                    return false;
                }
            }
        }
        SizeMatch::SameLAs(r) => {
            if let Some(&rc) = assigned.get(r as usize) {
                if g(cell).l != g(rc).l {
                    return false;
                }
            }
        }
    }

    match slot.diode {
        DiodeReq::Required if !is_diode(hg, cell) => return false,
        DiodeReq::Forbidden if is_diode(hg, cell) => return false,
        _ => {}
    }

    if slot.gate_is_signal {
        if let Some(gn) = pin_net(hg, cell, "G") {
            if !is_signal(gn, roles) {
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

#[allow(clippy::too_many_arguments)]
fn backtrack(
    pat: &Pattern,
    hg: &BipartiteHypergraph,
    geom: &[Geom],
    roles: &[NetRole],
    blocked: &HashSet<u32>,
    slot_idx: usize,
    assigned: &mut Vec<u32>,
    results: &mut Vec<PatternMatch>,
) {
    if slot_idx == pat.slots.len() {
        let mut key = assigned.clone();
        key.sort_unstable();
        if !results.iter().any(|r| r.instances == key) {
            results.push(PatternMatch { template: pat.name, instances: key, priority: pat.priority });
        }
        return;
    }

    let n = hg.device_count() as u32;
    for cell in 0..n {
        if assigned.contains(&cell) || blocked.contains(&cell) {
            continue;
        }
        if !slot_ok(&pat.slots[slot_idx], hg, geom, cell, assigned, roles) {
            continue;
        }
        assigned.push(cell);
        if partial_links_ok(pat.links, hg, assigned) {
            backtrack(pat, hg, geom, roles, blocked, slot_idx + 1, assigned, results);
        }
        assigned.pop();
    }
}

fn find_matches(
    pat: &Pattern,
    hg: &BipartiteHypergraph,
    geom: &[Geom],
    roles: &[NetRole],
    blocked: &HashSet<u32>,
) -> Vec<PatternMatch> {
    let mut results = Vec::new();
    let mut assigned: Vec<u32> = Vec::with_capacity(pat.slots.len());
    backtrack(pat, hg, geom, roles, blocked, 0, &mut assigned, &mut results);
    results
}

// ═══════════════════════════════════════════════════════════════════════
//  Public API
// ═══════════════════════════════════════════════════════════════════════

/// Run all catalog patterns; return non-overlapping matches (greedy,
/// priority-first). A device consumed by a higher-priority match cannot appear in
/// a lower-priority one — so an 8-device composite is preferred over the 2-device
/// primitives inside it. `blocked` (from [`AnnotationConfig::do_not_identify`])
/// devices never match; `do_not_use` templates are skipped.
pub fn recognize(
    hg: &BipartiteHypergraph,
    geom: &[Geom],
    roles: &[NetRole],
    cfg: &AnnotationConfig,
) -> Vec<PatternMatch> {
    let blocked = &cfg.do_not_identify;
    let mut all = Vec::new();

    for pat in PATTERNS {
        if cfg.do_not_use.contains(pat.name) {
            continue;
        }
        all.extend(find_matches(pat, hg, geom, roles, blocked));
    }

    // Priority desc, then a deterministic tiebreak on the sorted instance list.
    all.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.instances.cmp(&b.instances)));

    let mut consumed: HashSet<u32> = HashSet::new();
    let mut selected = Vec::new();
    for m in all {
        if m.instances.iter().any(|i| consumed.contains(i)) {
            continue;
        }
        consumed.extend(m.instances.iter().copied());
        selected.push(m);
    }
    selected
}

/// Recognise the **primitive tier** (≤2-device patterns: diff pair, mirror,
/// cascode, load, …) *within* the device set `subset`. Used to recover the child
/// primitives of a composite block whose greedy match swallowed them — the
/// leaves that carry the geometric constraints (hierarchy, ALIGN §2.5). Devices
/// outside `subset` are blocked, so the sub-recognition can't reach past the
/// parent block. Non-overlapping and deterministic, like [`recognize`].
pub fn recognize_primitives(
    hg: &BipartiteHypergraph,
    geom: &[Geom],
    roles: &[NetRole],
    cfg: &AnnotationConfig,
    subset: &[u32],
) -> Vec<PatternMatch> {
    let inside: HashSet<u32> = subset.iter().copied().collect();
    // Block everything not in the subset (plus the user's do_not_identify set).
    let blocked: HashSet<u32> = (0..hg.device_count() as u32)
        .filter(|d| !inside.contains(d) || cfg.do_not_identify.contains(d))
        .collect();

    let mut all = Vec::new();
    for pat in PATTERNS {
        if pat.slots.len() > 2 || cfg.do_not_use.contains(pat.name) {
            continue; // primitive tier only
        }
        all.extend(find_matches(pat, hg, geom, roles, &blocked));
    }
    all.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.instances.cmp(&b.instances)));

    let mut consumed: HashSet<u32> = HashSet::new();
    let mut selected = Vec::new();
    for m in all {
        if m.instances.iter().any(|i| consumed.contains(i)) {
            continue;
        }
        consumed.extend(m.instances.iter().copied());
        selected.push(m);
    }
    selected
}
