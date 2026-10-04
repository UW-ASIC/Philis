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
pub mod graph;
pub mod ir;
pub mod netrole;
pub mod pattern;
pub mod policy;
pub mod size;
pub mod symmetry;
pub mod terms;

#[cfg(test)]
mod tests;

pub use block::{Block, BlockKind};
pub use netrole::{rail_of, AnnotationConfig, NetRole, ProcessNumbers};
pub use policy::Policy;

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
    /// Every device, by id, with how recognition covers it (T7).
    pub coverage: Vec<(DeviceId, Coverage)>,
    /// Extraction's contract (EXT-12); filled from EXT-13 on.
    pub intent: analog::intent::Intent,
    /// Symmetry axes the placement emits: one per block until EXT-20 (card D-b).
    pub axis_count: usize,
}

/// How a device is covered: the one report that it got a constraint or why not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    /// Touched by a placement batch.
    Constrained,
    /// In a recognised block of this template whose roles emit nothing for it.
    Grouped(&'static str),
    /// In no constraint: `"do_not_identify"`, `"unknown size"` or `"no pattern"`.
    Unconstrained(&'static str),
}

/// What the netlist needs, so `missing` lists only families that would apply.
struct Needs {
    /// A matched leaf (DiffPair, CurrentMirror, Load, CascodePair) exists.
    matched: bool,
    /// A FET exists: gate nets an antenna rule would check.
    gate_nets: bool,
    /// A device has gate area: `classify` would budget nets (its own precondition).
    budgeted_nets: bool,
}

/// Rule families [`ProcessNumbers`] cannot instantiate, among those `needs`
/// says apply. DTI is absent from the list: a process without trenches makes
/// `DtiBand` inapplicable, not unknown.
fn missing(p: &ProcessNumbers, needs: &Needs) -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    if needs.matched && p.svt_uv_per_um.is_none() {
        out.push(("MatchedSet", "deck svt_uv_per_um — distance term unknown"));
    }
    if needs.matched && p.avt_mv_um.iter().any(Option::is_none) {
        out.push(("MatchedSet", "deck avt_n_mv_um/avt_p_mv_um"));
    }
    if needs.gate_nets && p.antenna_max_ratio.is_none() {
        out.push(("Antenna", "deck antenna ratio"));
    }
    if needs.budgeted_nets && p.gate_af_per_um2.is_none() {
        out.push(("ParasiticBudget", "deck gate_cap_af_um2"));
        out.push(("CouplingBudget", "deck gate_cap_af_um2"));
    } else if needs.budgeted_nets && p.wire_af_per_um.is_none() {
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

    // Recognised blocks in selection order, each composite with its primitive children.
    let canon = pattern::canonical_labels(&hg, &drawn, &models, &roles);
    let names: Vec<&str> = netlist.devices.iter().map(|d| d.name.as_str()).collect();
    let all = pattern::recognize_all(&hg, &drawn, &roles, cfg, &canon, &names);
    let mut claimed = vec![false; netlist.devices.len()];
    let mut blocks: Vec<Block> = pattern::select_disjoint(&all, &canon, &names)
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

    let mut placement = emit::placement(&blocks, netlist, &cfg.process, cfg.offset_sigma_mv, &cfg.policy);
    let fet = |k: pnr_core::DeviceKind| matches!(k, pnr_core::DeviceKind::Nmos | pnr_core::DeviceKind::Pmos);
    let needs = Needs {
        matched: block::leaves(&blocks)
            .iter()
            .any(|b| matches!(b.kind, BlockKind::DiffPair | BlockKind::CurrentMirror | BlockKind::Load | BlockKind::CascodePair)),
        gate_nets: netlist.devices.iter().any(|d| fet(d.kind)),
        budgeted_nets: gates.iter().any(|&g| g > 0.0),
    };
    let mut missing = missing(&cfg.process, &needs);
    // The matcher pairs FETs only: an unsized FET is a pair nobody could check.
    if netlist.devices.iter().zip(&drawn).any(|(d, s)| fet(d.kind) && size::unknown_size(d.kind, s)) {
        missing.push(("MatchedSet", "device W/L"));
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

    let mut routing = extract::routing(
        &hg,
        &net_classes,
        &gates,
        &cfg.process,
        &block::leaves(&blocks).iter().filter(|b| b.kind == BlockKind::DiffPair).map(|b| (b.devices[0], b.devices[1])).collect::<Vec<_>>(),
        &cfg.policy,
    );

    // Stable ids in emission order (permutation-invariant since EXT-06). A
    // placement batch whose first touched device is in a recognised block came
    // from that block's pattern; Isolation is cross-block, and the rest are net-class.
    let mut next = 0u32;
    let mut id = |origin| {
        next += 1;
        analog::intent::BatchMeta { id: analog::intent::ConstraintId(next - 1), origin }
    };
    let mut touched = vec![false; netlist.devices.len()];
    for arm in [&mut placement.hard, &mut placement.budget, &mut placement.cost] {
        *arm = std::mem::take(arm)
            .into_iter()
            .map(|inner| -> Box<dyn analog::RuleBatch<pnr_core::Layout>> {
                let mut ids = Vec::new();
                inner.touched(&mut ids);
                ids.iter().for_each(|&d| touched[d as usize] = true);
                let bi = ids.first().map_or(usize::MAX, |&d| block_of[d as usize]);
                let origin = if bi == usize::MAX || inner.kind().ends_with("::Isolation") {
                    analog::intent::Origin::NetClass
                } else {
                    analog::intent::Origin::Pattern { template: blocks[bi].template }
                };
                Box::new(analog::rule::Tagged { meta: id(origin), inner })
            })
            .collect();
    }
    for arm in [&mut routing.hard, &mut routing.budget, &mut routing.cost] {
        *arm = std::mem::take(arm)
            .into_iter()
            .map(|inner| -> Box<dyn analog::RuleBatch<pnr_core::Routes>> {
                Box::new(analog::rule::Tagged { meta: id(analog::intent::Origin::NetClass), inner })
            })
            .collect();
    }

    let coverage = (0..netlist.devices.len())
        .map(|d| {
            let c = if touched[d] {
                Coverage::Constrained
            } else if block_of[d] != usize::MAX {
                Coverage::Grouped(blocks[block_of[d]].template)
            } else if cfg.do_not_identify.contains(&(d as u32)) {
                Coverage::Unconstrained("do_not_identify")
            } else if size::unknown_size(netlist.devices[d].kind, &drawn[d]) {
                Coverage::Unconstrained("unknown size")
            } else {
                Coverage::Unconstrained("no pattern")
            };
            (DeviceId(d as u16), c)
        })
        .collect();

    let mut intent = analog::intent::Intent::default();
    // Symmetry seeds: the disjoint DiffPair/Load/CascodePair leaves (never contradictory),
    // each couple and the list in canonical order, names breaking exact ties.
    let ck = |d: DeviceId| (canon[d.0 as usize], names[d.0 as usize]);
    let mut seeds: Vec<(DeviceId, DeviceId, usize)> = block::leaves(&blocks)
        .iter()
        .enumerate()
        .filter(|(_, b)| matches!(b.kind, BlockKind::DiffPair | BlockKind::Load | BlockKind::CascodePair))
        .map(|(i, b)| if ck(b.devices[0]) <= ck(b.devices[1]) { (b.devices[0], b.devices[1], i) } else { (b.devices[1], b.devices[0], i) })
        .collect();
    seeds.sort_by_key(|&(a, b, _)| (ck(a), ck(b)));
    let seeds: Vec<symmetry::Seed> = seeds.into_iter().map(|(a, b, i)| symmetry::Seed::Devices(a, b, analog::intent::ConstraintId(i as u32))).collect();
    let (compounds, diags) = symmetry::analyze(&hg, &drawn, &net_classes, &seeds, &canon);
    intent.compounds = compounds;
    intent.diagnostics.extend(diags);
    let reqs = graph::requirements(&all, &intent.compounds, &[], &[], &hg, &net_classes, &canon, &cfg.policy);
    intent.tree = graph::hsmpg(netlist.devices.len(), &reqs, &canon);
    Problem {
        intent,
        axis_count: blocks.len(),
        placement,
        routing,
        coverage,
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
