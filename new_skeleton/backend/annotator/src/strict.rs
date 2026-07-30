//! Block-DAG construction from the DSL recogniser's matches.
//!
//! The parallelization unit is a **block**: a device set the [`crate::pattern`]
//! recogniser matched against a catalog template (a diff pair, a mirror, a
//! telescopic-OTA core). This module turns the recogniser's non-overlapping
//! [`PatternMatch`]es into the block DAG the placer processes concurrently.
//!
//! ## What changed (Q5a)
//!
//! Grouping used to be a side effect of `analog::Rule::extract` unioning devices
//! into a `UnionFind`, canonicalized here into `GroupId`s. That path is retired:
//! the DSL matcher is now the single authoritative source of *which devices form a
//! block* — a union-find over ≤2-FET primitives could never see an 8–12 device
//! composite (telescopic OTA, Gilbert cell), which is exactly the block we want to
//! recognise and reuse. So `build_dag` consumes matches directly; the old
//! `classify_pair` topology ladder (5 kinds, ≤2 FETs) is gone — the block's kind
//! now comes from [`BlockKind::from_template`] over the matched template.
//!
//! ## The DAG
//!
//! Recognised blocks are independent leaves (empty `depends_on`); the unclaimed
//! remainder forms one top-level **glue** block that depends on every recognised
//! block (it wires them together, so it is placed after them). Acyclic by
//! construction. `recognize` already returns matches in a deterministic order
//! (priority desc, then sorted instance list), so block ids are stable.

use crate::block::{Block, BlockKind};
use crate::pattern::PatternMatch;
use pnr_core::ids::{DeviceId, GroupId};

/// The canonical group table plus the block DAG.
pub struct Dag {
    /// One block per node. Recognised blocks first (leaves), then the glue block
    /// last (depends on all of them). A block's index here equals its `GroupId`.
    pub blocks: Vec<Block>,
    /// `Layout.groups`: member devices per [`GroupId`], index = id. 1:1 with
    /// `blocks` (each block owns exactly one group id, including glue).
    pub groups: Vec<Vec<DeviceId>>,
}

/// Build the block DAG from the recogniser's non-overlapping matches.
/// `device_count` is the total device count (to find the unclaimed glue
/// remainder). Matches are assumed device-disjoint (the guarantee `recognize`
/// gives); a device claimed by a match is never glue.
#[must_use]
pub fn build_dag(matches: &[PatternMatch], device_count: usize) -> Dag {
    let mut blocks = Vec::with_capacity(matches.len() + 1);
    let mut groups = Vec::with_capacity(matches.len() + 1);
    let mut claimed = vec![false; device_count];

    for (i, m) in matches.iter().enumerate() {
        let devices: Vec<DeviceId> = m
            .instances
            .iter()
            .map(|&d| {
                claimed[d as usize] = true;
                DeviceId(d as u16)
            })
            .collect();
        groups.push(devices.clone());
        blocks.push(Block {
            kind: BlockKind::from_template(m.template),
            template: m.template,
            devices,
            group: GroupId(i as u16),
            depends_on: Vec::new(), // leaves: no inter-block deps yet
            injected: false,        // the orchestrator flags user-injected blocks
            sub_blocks: Vec::new(), // populated by `annotate` (hierarchy)
        });
    }

    // Top-level glue: every device no match claimed. One block, depends on all
    // recognised blocks (placed after them). It also gets a group id so it is a
    // uniform DAG node addressable as Target::Group.
    let glue: Vec<DeviceId> = (0..device_count as u16)
        .filter(|&d| !claimed[d as usize])
        .map(DeviceId)
        .collect();
    let glue_gid = GroupId(blocks.len() as u16);
    let depends_on: Vec<usize> = (0..blocks.len()).collect();
    groups.push(glue.clone());
    blocks.push(Block {
        kind: BlockKind::Glue,
        template: "",
        devices: glue,
        group: glue_gid,
        depends_on,
        injected: false,
        sub_blocks: Vec::new(),
    });

    Dag { blocks, groups }
}
