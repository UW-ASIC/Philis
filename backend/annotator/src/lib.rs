//! `annotator`: netlist → recognised analog blocks → constraints.
//!
//! [`annotate`] runs the [`pattern`] catalog over the netlist, turns the
//! non-overlapping matches into blocks (one group each, plus a trailing glue
//! block for the unclaimed devices), recovers each composite's primitive
//! children, and emits placement ([`emit`]), routing ([`extract`]), net-class
//! ([`classify`]) and cell-tier ([`constraints`]) constraints from them.

pub mod block;
pub mod catalog;
pub mod classify;
pub mod constraints;
pub mod emit;
pub mod extract;
pub mod netrole;
pub mod pattern;

#[cfg(test)]
mod tests;

pub use block::{Block, BlockKind};
pub use netrole::{AnnotationConfig, NetRole};

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
}

/// Assemble the [`Problem`]. Deterministic.
#[must_use]
pub fn annotate(netlist: &Netlist, cfg: &AnnotationConfig) -> Problem {
    let hg = pnr_core::BipartiteHypergraph::from_netlist(netlist);
    let geom: Vec<pattern::Geom> = netlist
        .devices
        .iter()
        .map(|d| pattern::Geom { w: param(d, "w", 0), l: param(d, "l", 0) })
        .collect();
    let roles = netrole::classify_nets(&hg, cfg);
    let all: Vec<u32> = (0..netlist.devices.len() as u32).collect();

    // Recognised blocks, each composite with its primitive children.
    let mut claimed = vec![false; netlist.devices.len()];
    let mut blocks: Vec<Block> = pattern::recognize(&hg, &geom, &roles, cfg, &all, usize::MAX)
        .into_iter()
        .map(|m| {
            for &d in &m.instances {
                claimed[d as usize] = true;
            }
            let mut b = Block::from_match(&m);
            if b.devices.len() > 2 {
                b.sub_blocks = pattern::recognize(&hg, &geom, &roles, cfg, &m.instances, 2)
                    .iter()
                    .map(Block::from_match)
                    .collect();
            }
            b
        })
        .collect();
    let glue = (0..netlist.devices.len() as u16).filter(|&d| !claimed[d as usize]).map(DeviceId);
    blocks.push(Block { kind: BlockKind::Glue, devices: glue.collect(), injected: false, sub_blocks: Vec::new() });

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
    let net_classes = classify::classify(&hg, &roles, &sensitive, &gates);

    Problem {
        placement: emit::placement(&blocks, netlist),
        routing: extract::routing(&hg, &net_classes, &gates, cfg.antenna_max_ratio),
        constraints: constraints::assemble(netlist, &blocks),
        net_classes,
        groups,
        abutment,
        blocks,
    }
}

/// Gate area `W·L·fingers` of a FET, µm²; `0` for anything else or when the
/// netlist omits W/L.
pub(crate) fn gate_um2(dev: &pnr_core::netlist::Device) -> f32 {
    if !matches!(dev.kind, pnr_core::DeviceKind::Nmos | pnr_core::DeviceKind::Pmos) {
        return 0.0;
    }
    let (w, l) = (param(dev, "w", 0) as f32, param(dev, "l", 0) as f32);
    w * l * 1e-6 * f32::from(constraints::fingers(dev))
}

/// SPICE param `key` of `dev`, or `default`.
pub(crate) fn param(dev: &pnr_core::netlist::Device, key: &str, default: i64) -> i64 {
    dev.params.iter().find(|(k, _)| k == key).map_or(default, |&(_, v)| v)
}
