//! Reuse detection (Q4) — *recognition ≠ reuse*, so this is a separate pass over
//! the recognised blocks. Two non-destructive detections, both pure data:
//!
//! - [`reuse_classes`] — **block-level.** Structurally-identical top-level blocks
//!   (same template, same member geometry) that should share **one** generated
//!   layout/variant. Covers both of the user's cases with one mechanism: a
//!   100-cell array is a 100-member class; two matched diff pairs across the
//!   circuit are a 2-member class.
//! - [`parallel_groups`] — **device-level.** Devices wired fully in parallel with
//!   identical geometry (paralleled fingers, a bank of unit caps/resistors the
//!   catalog never recognises as a *block*). ALIGN's "one primitive × multiplier":
//!   draw one, stamp N.
//!
//! **This is coupling, not frozen geometry** (ADR 0005). The annotator only
//! *detects* the equivalence classes; it emits no layout. Whether a class actually
//! shares one variant is the place/route loop's call — it couples the variant
//! choice across the class and still picks per merit. Detecting the class here is
//! what makes that coupling *possible*; freezing a layout here is the "static
//! collapse" ADR 0005 rejects.

use std::collections::HashMap;

use pnr_core::ids::DeviceId;
use pnr_core::Netlist;

use crate::block::{Block, BlockKind};

/// A set of structurally-identical recognised units that may share one generated
/// layout/variant. A member is a unit's device set (not a `GroupId` — a reuse
/// member can be a *child* leaf inside a composite, which has no top-level group).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReuseClass {
    pub members: Vec<Vec<DeviceId>>,
}

/// A device's geometry signature (kind discriminant + W/L/nf), for structural
/// equality. Kind is the enum's `u8` discriminant so the sig is `Hash`/`Ord`
/// without deriving those on the foundation `DeviceKind`.
type GeomSig = (u8, i64, i64, i64);

fn param(nl: &Netlist, d: DeviceId, key: &str, default: i64) -> i64 {
    nl.devices
        .get(d.0 as usize)
        .and_then(|dev| dev.params.iter().find(|(k, _)| k == key).map(|(_, v)| *v))
        .unwrap_or(default)
}

fn geom_sig(nl: &Netlist, d: DeviceId) -> GeomSig {
    let kind = nl.devices.get(d.0 as usize).map_or(0, |dev| dev.kind as u8);
    (kind, param(nl, d, "w", 0), param(nl, d, "l", 0), param(nl, d, "nf", 1))
}

/// Collect the recognition **leaves** of the hierarchy: every block with no
/// children (a top-level primitive, or a composite's child primitive), excluding
/// glue. These are the reusable units — a composite is a grouping node, its leaves
/// are what actually gets drawn.
fn leaves<'a>(blocks: &'a [Block], out: &mut Vec<&'a Block>) {
    for b in blocks {
        if !b.sub_blocks.is_empty() {
            leaves(&b.sub_blocks, out);
        } else if !matches!(b.kind, BlockKind::Glue) && !b.devices.is_empty() {
            out.push(b);
        }
    }
}

/// Group the hierarchy's leaves into reuse-equivalence classes: same template +
/// same **multiset** of member geometries ⇒ structurally identical ⇒ one shared
/// layout. Nets are deliberately ignored — two diff pairs are reuse-identical
/// however they are wired, and two identical mirrors buried inside one composite
/// are still a reuse class. Only classes with ≥2 members are returned. Members are
/// device sets (children have no top-level group id).
#[must_use]
pub fn reuse_classes(blocks: &[Block], nl: &Netlist) -> Vec<ReuseClass> {
    let mut units = Vec::new();
    leaves(blocks, &mut units);

    let mut by_sig: HashMap<(&'static str, Vec<GeomSig>), Vec<Vec<DeviceId>>> = HashMap::new();
    for b in units {
        let mut sig: Vec<GeomSig> = b.devices.iter().map(|&d| geom_sig(nl, d)).collect();
        sig.sort_unstable(); // multiset: order-independent
        let mut devs = b.devices.clone();
        devs.sort_unstable_by_key(|d| d.0);
        by_sig.entry((b.template, sig)).or_default().push(devs);
    }
    let mut classes: Vec<ReuseClass> = by_sig
        .into_values()
        .filter(|members| members.len() >= 2)
        .map(|mut members| {
            members.sort_unstable_by_key(|m| m[0].0);
            ReuseClass { members }
        })
        .collect();
    classes.sort_unstable_by_key(|c| c.members[0][0].0); // deterministic order
    classes
}

/// Maximal sets of devices wired **fully in parallel** with identical geometry:
/// same kind, same W/L/nf, and the exact same `(pin → net)` mapping. These are the
/// unit cells of a passive array / paralleled fingers — draw one, stamp N. Only
/// sets of ≥2 are returned. Deterministic (sorted).
#[must_use]
pub fn parallel_groups(nl: &Netlist) -> Vec<Vec<DeviceId>> {
    // Key: geometry + the sorted terminal (pin,net) set. Identical key ⇒ parallel.
    let mut by_key: HashMap<(GeomSig, Vec<(String, u16)>), Vec<DeviceId>> = HashMap::new();
    for (i, dev) in nl.devices.iter().enumerate() {
        let d = DeviceId(i as u16);
        let mut term: Vec<(String, u16)> =
            dev.terminals.iter().map(|(p, n)| (p.clone(), n.0)).collect();
        term.sort();
        by_key.entry((geom_sig(nl, d), term)).or_default().push(d);
    }
    let mut groups: Vec<Vec<DeviceId>> = by_key
        .into_values()
        .filter(|g| g.len() >= 2)
        .map(|mut g| {
            g.sort_unstable_by_key(|d| d.0);
            g
        })
        .collect();
    groups.sort_unstable_by_key(|g| g[0].0);
    groups
}
