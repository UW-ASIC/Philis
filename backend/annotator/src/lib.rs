//! `annotator`: netlist → recognised analog blocks → constraints.
//!
//! [`annotate`] runs the [`pattern`] catalog over the netlist, turns the
//! non-overlapping matches into blocks (one group each, plus a trailing glue
//! block for the unclaimed devices), gives each composite its declared children
//! ([`catalog::roles_of`]), and emits placement ([`emit`]), routing ([`extract`]), net-class
//! ([`classify`]) and cell-tier ([`constraints`]) constraints from them.

pub mod block;
pub mod catalog;
pub mod classify;
pub mod constraints;
pub mod emit;
pub mod extract;
pub mod ir;
pub mod netrole;
pub mod pattern;
pub mod size;
pub mod terms;

#[cfg(test)]
mod tests;

pub use block::{Block, BlockKind};
pub use netrole::{rail_of, AnnotationConfig, NetRole, ProcessNumbers};

use pnr_core::ids::DeviceId;
use pnr_core::Netlist;

/// Everything the annotator hands the generators, placer and router.
pub struct Problem {
    /// Recognised blocks, then the glue block. Index = [`pnr_core::GroupId`] and
    /// the stage's symmetry [`pnr_core::ids::AxisId`].
    pub blocks: Vec<Block>,
    /// Cell-tier directives (unitization, dummies, guard rings), read by `cells`.
    pub constraints: analog::Constraints,
    pub placement: analog::Requirements<pnr_core::Layout>,
    pub routing: analog::Requirements<pnr_core::Routes>,
    /// Per-net class + budgets, indexed by [`pnr_core::NetId`].
    pub net_classes: Vec<analog::metadata::NetClassification>,
    /// `blocks[i].devices` — feeds [`pnr_core::Layout::groups`]. Composites may mix
    /// polarities.
    pub groups: Vec<Vec<DeviceId>>,
    /// Diffusion-sharing permission, parallel to `groups`: a mixed-polarity group is
    /// cut to one member (NMOS/PMOS abutment merges implants — DRC-clean, LVS-fatal).
    /// One member, not zero: an empty group panics `Layout::bbox`.
    pub abutment: Vec<Vec<DeviceId>>,
    /// `(rule kind, missing input)` for every family left unemitted because the
    /// deck lacks a number: **unknown**, never a pass.
    pub missing: Vec<(&'static str, &'static str)>,
}

/// Rule families [`ProcessNumbers`] cannot instantiate. DTI is absent from the
/// list: a process without trenches makes `DtiBand` inapplicable, not unknown.
fn missing(p: &ProcessNumbers) -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    if p.svt_uv_per_um.is_none() {
        out.push(("MatchingPair", "deck svt_uv_per_um"));
    }
    if p.avt_mv_um.iter().any(Option::is_none) {
        out.push(("MatchingPair", "deck avt_n_mv_um/avt_p_mv_um"));
    }
    if p.antenna_max_ratio.is_none() {
        out.push(("Antenna", "deck antenna ratio"));
    }
    if p.gate_af_per_um2.is_none() {
        out.push(("ParasiticBudget", "deck gate_cap_af_um2"));
        out.push(("CouplingBudget", "deck gate_cap_af_um2"));
    } else if p.wire_af_per_um.is_none() {
        out.push(("ParasiticBudget", "deck wire capacitance"));
    }
    out
}

/// Assemble the [`Problem`]. Deterministic.
///
/// # Panics
/// When the netlist has more than 65535 devices or nets: ids are `u16` (AA-35).
#[must_use]
pub fn annotate(netlist: &Netlist, cfg: &AnnotationConfig) -> Problem {
    for n in [netlist.devices.len(), netlist.nets.len()] {
        assert!(n <= usize::from(u16::MAX), "annotator: {n} devices/nets exceed the u16 id space (65535)");
    }
    let hg = pnr_core::BipartiteHypergraph::from_netlist(netlist);
    let mut models = Vec::new();
    let drawn: Vec<size::Drawn> = netlist.devices.iter().map(|d| size::drawn(d, &mut models)).collect();
    let roles = netrole::classify_nets(&hg, cfg);
    let all: Vec<u32> = (0..netlist.devices.len() as u32).collect();

    // Recognised blocks, each composite with its primitive children.
    let mut claimed = vec![false; netlist.devices.len()];
    let mut blocks: Vec<Block> = pattern::recognize(&hg, &drawn, &roles, cfg, &all, usize::MAX)
        .into_iter()
        .map(|m| {
            for &d in &m.instances {
                claimed[d as usize] = true;
            }
            Block::from_match(&m)
        })
        .collect();
    let glue = (0..netlist.devices.len() as u16).filter(|&d| !claimed[d as usize]).map(DeviceId);
    blocks.push(Block { kind: BlockKind::Glue, template: "glue", devices: glue.collect(), injected: false, sub_blocks: Vec::new(), selfs: Vec::new() });

    let groups: Vec<Vec<DeviceId>> = blocks.iter().map(|b| b.devices.clone()).collect();
    let abutment = groups
        .iter()
        .map(|g| {
            let k0 = g.first().map(|d| netlist.devices[d.0 as usize].kind);
            if g.iter().all(|d| Some(netlist.devices[d.0 as usize].kind) == k0) {
                g.clone()
            } else {
                g[..1].to_vec()
            }
        })
        .collect();

    // A net feeding a matched device's gate is the small-signal path whose coupling
    // shows up as offset: classified Sensitive, tight budgets.
    let mut sensitive = vec![false; netlist.devices.len()];
    for b in block::leaves(&blocks) {
        if b.kind.is_sensitive() {
            for d in &b.devices {
                sensitive[d.0 as usize] = true;
            }
        }
    }
    let gates: Vec<f32> = netlist.devices.iter().map(gate_um2).collect();
    let net_classes = classify::classify(&hg, &roles, &sensitive, &gates, cfg.process.gate_af_per_um2);

    let mut placement = emit::placement(&blocks, netlist, &cfg.process, cfg.offset_sigma_mv);
    let mut missing = missing(&cfg.process);
    if netlist.devices.iter().zip(&drawn).any(|(d, s)| size::unknown_size(d.kind, s)) {
        missing.push(("MatchingPair", "device W/L"));
    }
    if netlist.devices.iter().any(|d| d.kind == pnr_core::DeviceKind::Capacitor) {
        missing.push(("ParasiticBudget", "capacitor-plate nets: settling / code-error spec (ARR-03, ARR-05)"));
    }
    // Same entry of `blocks`, glue excluded: glue is no stage.
    let mut block_of = vec![usize::MAX; netlist.devices.len()];
    for (bi, b) in blocks.iter().enumerate().filter(|(_, b)| b.kind != BlockKind::Glue) {
        b.devices.iter().for_each(|d| block_of[d.0 as usize] = bi);
    }
    let same_block = |a: usize, v: usize| block_of[a] != usize::MAX && block_of[a] == block_of[v];
    let p = &cfg.process;
    if let Some(why) = emit::isolation(&hg, &net_classes, &sensitive, &same_block, p.substrate, p.epi_nm, &mut placement) {
        missing.push(("Isolation", why));
    }

    Problem {
        placement,
        routing: extract::routing(&hg, &net_classes, &gates, &cfg.process),
        constraints: constraints::assemble(netlist, &drawn, &blocks),
        net_classes,
        groups,
        abutment,
        blocks,
        missing,
    }
}

/// Gate area `W_total·L·m` of a FET, µm²; `0` for anything else or when the
/// netlist omits W/L.
pub(crate) fn gate_um2(dev: &pnr_core::netlist::Device) -> f32 {
    dev.gate_area_um2() as f32
}

/// SPICE param `key` of `dev`, or `default`.
pub(crate) fn param(dev: &pnr_core::netlist::Device, key: &str, default: i64) -> i64 {
    dev.params.iter().find(|(k, _)| k == key).map_or(default, |&(_, v)| v)
}
