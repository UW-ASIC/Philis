//! # `annotator` — the block-DAG parallelizer that *assembles* the problem
//!
//! The annotator turns a flat [`pnr_core::Netlist`] into a [`Problem`]: a DAG of
//! blocks that can be processed concurrently, plus the applied analog rule-sets.
//! It owns **scheduling and assembly, not theory** — it does *not* find
//! constraints. Each `analog::Rule::extract` walks the hypergraph and returns
//! every instance of itself (recognition is the theory's job); the annotator only
//! *calls* those extracts, batches and partitions the results, and reads the
//! rule-coalesced union-find into the group table + DAG.
//!
//! ## What it produces — [`Problem`]
//!
//! - `blocks` — the **DAG**. One node per rule-coalesced group (leaf) plus one
//!   top-level glue node depending on all of them. Acyclic by construction; a node
//!   is ready once its `depends_on` are done, so the placer can walk it in
//!   parallel. Built in [`strict::build_dag`].
//! - `placement` / `routing` — [`analog::Requirements`] over [`pnr_core::Layout`]
//!   / [`pnr_core::Routes`], assembled by calling every rule's `extract` and
//!   partitioning `hard`/`budget`/`cost` by enforcement — **exactly one arm each**.
//!   Built in [`emit::placement`] / [`extract::routing_classified`].
//! - `groups` — the `Vec<Vec<DeviceId>>` group table (feeds
//!   [`pnr_core::Layout::groups`]); index = [`pnr_core::GroupId`]. Built alongside
//!   the DAG; see [`strict`] for group canonicalization.
//! - `constraints` — cell-tier structural [`analog::Constraints`] (unitization,
//!   guard rings, dummies, LDE, stress). These are *not* Rules (no `extract`), so
//!   the annotator builds them; see [`constraints`].
//!
//! ## Ordering
//!
//! Strict first, inference on the remainder: [`strict::build_dag`] materialises
//! the authoritative rule-coalesced groups; [`inference`] then augments the DAG
//! over whatever devices strict left as glue. Inference is stubbed
//! ([`inference::NoInference`]) until the recogniser lands.

#![allow(dead_code)]

pub mod block;
pub mod classify;
pub mod catalog;
pub mod constraints;
pub mod emit;
pub mod extract;
pub mod inference;
pub mod netrole;
pub mod pattern;
pub mod reuse;
pub mod s3det;
pub mod strict;

#[cfg(test)]
mod tests;

pub use block::{Block, BlockKind};
pub use inference::{BlockInference, NoInference};
pub use s3det::S3Det;
pub use netrole::{AnnotationConfig, NetRole};
pub use pattern::{Geom, PatternMatch};

use pnr_core::ids::DeviceId;
use pnr_core::Netlist;

/// The assembled analog problem: a DAG of blocks and the applied rule-sets.
///
/// This is the annotator's whole output — everything the generators, placer, and
/// router downstream consume. `placement`/`routing` are already partitioned into
/// `hard`/`budget`/`cost` (existence-based mode — PLAN §3b's three lexicographic
/// tiers, one arm per batch); `groups` is the [`pnr_core::Layout`] group table;
/// `blocks` is the concurrency schedule.
pub struct Problem {
    /// The block DAG: leaves (rule-coalesced groups) then the glue node. A block's
    /// index equals its [`pnr_core::GroupId`].
    pub blocks: Vec<Block>,
    /// Cell-tier structural directives, read cold by `cells`.
    pub constraints: analog::Constraints,
    /// Placement rule-set (Device↔Device), scored against [`pnr_core::Layout`].
    pub placement: analog::Requirements<pnr_core::Layout>,
    /// Routing rule-set (Net arity), scored against [`pnr_core::Routes`].
    pub routing: analog::Requirements<pnr_core::Routes>,
    /// Per-net class + budgets, indexed by [`pnr_core::NetId`]. The front of the
    /// routing-constraint chain: every routing budget is keyed to one of these.
    pub net_classes: Vec<analog::metadata::NetClassification>,
    /// Member devices per group — feeds [`pnr_core::Layout::groups`]; index = id.
    ///
    /// This is the **recognition** table: a composite block (a whole OTA core) is one
    /// group here, which is what `Target::Group` rules must see so a rule can score a
    /// composite exactly like a device.
    pub groups: Vec<Vec<DeviceId>>,
    /// Groups that may legitimately **share diffusion** — abutment permission,
    /// parallel to [`Problem::groups`] and *not* the same thing.
    ///
    /// The two were conflated, and the orchestrator was deriving this one itself by
    /// filtering `groups` down to single-kind members. That is recognition semantics,
    /// so it belongs here: only the annotator knows what a group *means*.
    ///
    /// Why the distinction is load-bearing rather than tidy: `groups` deliberately
    /// contains composites that span both polarities. Granting abutment on that table
    /// lets an NMOS and a PMOS abut, their opposite implants merge, and extraction
    /// then reports a channel that "ambiguously matches MOS rules [nmos, pmos]" — a
    /// layout that is perfectly DRC-legal and LVS-fatal, which no downstream stage can
    /// repair because nothing is illegal. Devices of opposite polarity never match and
    /// never share diffusion.
    ///
    /// A group is truncated to one member rather than emptied when it is mixed-kind:
    /// `group_index` ignores groups with fewer than two members (so a one-member group
    /// grants no abutment), while an *empty* one panics `Layout::bbox`
    /// ("group has no members") for any rule targeting it.
    pub abutment: Vec<Vec<DeviceId>>,
    /// **Reuse classes** (Q4): structurally-identical recognition leaves (top-level
    /// primitives *or* a composite's identical children) that may share one
    /// generated layout/variant. This is a *coupling* the place/route loop resolves
    /// (ADR 0005), not frozen geometry — see [`reuse`].
    pub reuse: Vec<reuse::ReuseClass>,
    /// **Parallel device groups**: devices wired fully in parallel with identical
    /// geometry (paralleled fingers, passive-array unit cells). Draw one, stamp N
    /// (ALIGN multiplier). Each inner `Vec` has ≥2 members. See [`reuse`].
    pub parallel: Vec<Vec<DeviceId>>,
}

/// Per-device geometry (W/L) parallel to `netlist.devices`, read from SPICE
/// `params`. The recogniser needs it for [`pattern::SizeMatch`]; the `pnr_core`
/// hypergraph deliberately doesn't carry params, so we build the channel here.
fn build_geom(netlist: &Netlist) -> Vec<Geom> {
    let read = |dev: &pnr_core::netlist::Device, key: &str| {
        dev.params.iter().find(|(k, _)| k == key).map(|(_, v)| *v).unwrap_or(0)
    };
    netlist
        .devices
        .iter()
        .map(|d| Geom { w: read(d, "w"), l: read(d, "l") })
        .collect()
}

/// Assemble the [`Problem`] from a netlist.
///
/// 1. **Extract** — assemble the placement/routing [`analog::Requirements`] by
///    calling every rule's `extract` (see [`extract::extract`]). *(Grouping no
///    longer flows from here — Q5a; the union-find it still returns is ignored.)*
/// 2. **Recognise (strict, single source)** — the [`pattern`] DSL matcher runs the
///    catalog over the hypergraph and returns non-overlapping matches; those matches
///    *are* the blocks. [`strict::build_dag`] turns them into the block DAG.
/// 3. **Inference (remainder)** — augment the DAG over the glue devices the
///    recogniser left unclaimed (S3DET; stubbed until [`inference`] lands).
/// 4. **Constraints** — assemble the cell-tier structural directives.
///
/// Pure: same `(netlist, infer, cfg)` ⇒ same [`Problem`] (`recognize` returns
/// matches in a deterministic priority/instance order).
#[must_use]
pub fn annotate(netlist: &Netlist, infer: &dyn BlockInference, cfg: &AnnotationConfig) -> Problem {
    // 1. Recognise: the DSL catalog is the single authoritative block source.
    let hg = pnr_core::BipartiteHypergraph::from_netlist(netlist);
    let geom = build_geom(netlist);
    let roles = netrole::classify_nets(&hg, cfg);
    let matches = pattern::recognize(&hg, &geom, &roles, cfg);
    let mut dag = strict::build_dag(&matches, netlist.devices.len());

    // 2. Build the hierarchy (ALIGN §2.5): a composite block (>2 devices) keeps its
    //    internal primitives as `sub_blocks`, recovered by primitive-tier
    //    recognition within its device set. This is what lets constraint emission
    //    reach the diff pair a composite would otherwise swallow.
    for b in &mut dag.blocks {
        if matches!(b.kind, block::BlockKind::Glue) || b.devices.len() <= 2 {
            continue;
        }
        let subset: Vec<u32> = b.devices.iter().map(|d| u32::from(d.0)).collect();
        let subs = pattern::recognize_primitives(&hg, &geom, &roles, cfg, &subset);
        b.sub_blocks = subs
            .iter()
            .map(|m| Block {
                kind: BlockKind::from_template(m.template),
                template: m.template,
                devices: m.instances.iter().map(|&d| DeviceId(d as u16)).collect(),
                group: b.group, // placed as part of the parent; not a top-level group
                depends_on: Vec::new(),
                injected: false,
                sub_blocks: Vec::new(),
            })
            .collect();
    }

    // 3. Inference over the remainder (the glue block's devices). Stubbed today.
    //    ponytail: inferred blocks are appended after glue; when S3DET lands
    //    (task 4) they become leaves and glue must depend on them — restructure then.
    let inferred = infer.infer(netlist, &dag.blocks);
    for mut b in inferred {
        b.group = pnr_core::ids::GroupId(dag.blocks.len() as u16);
        dag.groups.push(b.devices.clone());
        dag.blocks.push(b);
    }

    // 4. Constraints. Placement is recognition-driven (emit from the block
    //    hierarchy, Q6a); routing rules still self-extract; cell-tier structural
    //    directives assemble from the recognised blocks.
    let placement = emit::placement(&dag.blocks, &hg);

    // Net classification keys the routing budgets. The devices whose matching the
    // layout must protect are the recognised matched blocks: a net feeding one of
    // their gates is the small-signal path whose coupling shows up as offset, so
    // it is classified Sensitive and gets the tight budgets.
    let mut sensitive = vec![false; netlist.devices.len()];
    mark_sensitive(&dag.blocks, &mut sensitive);
    let net_classes = classify::classify(&hg, &roles, &sensitive);
    let routing = extract::routing_classified(netlist, &net_classes);
    let constraints = constraints::assemble(netlist, &dag.blocks);

    // 5. Reuse detection (Q4): structurally-identical blocks / parallel devices
    //    that may share one layout. Coupling data, not frozen geometry (ADR 0005).
    let reuse = reuse::reuse_classes(&dag.blocks, netlist);
    let parallel = reuse::parallel_groups(netlist);

    // 6. Abutment permission: the same-polarity subset of the recognition groups.
    //    Derived here rather than by the orchestrator — what a group *means* is
    //    recognition's business, and the caller filtering it itself is how the two
    //    tables came to be conflated. See [`Problem::abutment`].
    let abutment = abutment_groups(&dag.groups, netlist);

    Problem {
        blocks: dag.blocks,
        constraints,
        placement,
        routing,
        net_classes,
        groups: dag.groups,
        abutment,
        reuse,
        parallel,
    }
}

/// Keep only the groups whose members are all the same [`pnr_core::DeviceKind`] —
/// the ones that may share diffusion. See [`Problem::abutment`] for why mixed-kind
/// groups must not, and why they are truncated to one member rather than emptied.
fn abutment_groups(groups: &[Vec<DeviceId>], netlist: &Netlist) -> Vec<Vec<DeviceId>> {
    groups
        .iter()
        .map(|members| {
            let mut kinds = members
                .iter()
                .filter_map(|d| netlist.devices.get(d.0 as usize).map(|dev| dev.kind));
            let first = kinds.next();
            if kinds.all(|k| Some(k) == first) {
                members.clone()
            } else {
                members.iter().take(1).copied().collect()
            }
        })
        .collect()
}

/// Flag devices belonging to a block whose recognition implies matching — the
/// devices whose mismatch the layout is responsible for.
fn mark_sensitive(blocks: &[Block], out: &mut [bool]) {
    use crate::block::BlockKind;
    for b in blocks {
        mark_sensitive(&b.sub_blocks, out);
        if matches!(
            b.kind,
            BlockKind::DiffPair | BlockKind::CurrentMirror | BlockKind::Cascode | BlockKind::Load
        ) {
            for d in &b.devices {
                if let Some(s) = out.get_mut(d.0 as usize) {
                    *s = true;
                }
            }
        }
    }
}
