//! Cell-tier [`analog::Constraints`], read by `cells` when drawing devices.
//!
//! One **Unitization** per matched set (EXT-15) with one finger W and L over
//! its members (deck-independent: the inferred unit is not drawn): members, ratio reduced by the gcd, dummies from the
//! class (MOS, R and C only, card D-g). Drawn in schematic fingers (`dev_nf` =
//! `nf·m`, `unit_w` = W_f), not the set's inferred unit (`MatchSpec::unit`,
//! `Member::parallel`/`series`): the LVS reference expands `nf·m` fingers of
//! W_f, so a set drawn in `W_u ≠ W_f` units is `lvs.unpaired_device` (measured
//! on `mirror_ratio` and `ota`). Adopting the unit is a CELL/FLOW item. Devices no such set
//! covers keep today's rules (card D-f), below, and then identical parallel MOS
//! (all terminals equal, one size and model) still uncovered share one
//! Unitization with neither dummies nor route matching (cellgen's rule).
//!
//! For every device of every recognised block:
//! - **Unitization**, one per (kind, model, bulk, finger W, L) class of the block — the only channel
//!   by which W/L/nf reach `cells`. One class per unitization: `unit_w`/`unit_l`
//!   are group scalars, and a mixed group draws members at the wrong size (LVS
//!   `parameter_mismatch`). Model and bulk split classes too: two flavours or two
//!   wells are never one drawn unit (AA-20).
//!
//! Guard rings are not emitted here: [`crate::rings::plan`] places them by role (REL-07).

use analog::cell::{SeriesParallel, Unitization};
use analog::intent::{MatchClass, MatchKind, MatchSpec, Origin};
use analog::Constraints;
use pnr_core::ids::DeviceId;
use pnr_core::netlist::DeviceKind;
use pnr_core::Netlist;

use crate::block::{Block, BlockKind};
use crate::size::Drawn;

/// Drawn fingers `nf·m` ([`pnr_core::MosSize::fingers`]; cells and the LVS
/// reference both expand that many unit devices); `1` without a MOS size.
pub(crate) fn fingers(dev: &pnr_core::netlist::Device) -> u16 {
    dev.mos_size().map_or(1, |s| s.fingers().min(u32::from(u16::MAX)) as u16)
}

/// How a block's units of `kind` combine: a resistor or capacitor drawn
/// outside a matched set is a series string of units, anything else parallel.
fn series_parallel(kind: DeviceKind) -> SeriesParallel {
    match kind {
        DeviceKind::Resistor | DeviceKind::Capacitor => SeriesParallel::Series,
        _ => SeriesParallel::Parallel,
    }
}

/// Dummies by class (Hastings MOS rule 12: minimal need none, moderate and
/// exceptional do), MOS, R and C only: bipolar sets keep none (card D-g).
pub(crate) fn dummy_required(kind: DeviceKind, class: MatchClass) -> bool {
    !matches!(kind, DeviceKind::Npn | DeviceKind::Pnp) && class >= MatchClass::Moderate
}

/// The member of `s` whose `P` and `N` nets are both `P` nets of other members
/// (a split DAC's bridge), if exactly one is.
fn split_bridge(netlist: &Netlist, s: &MatchSpec) -> Option<DeviceId> {
    let net = |d: DeviceId, t: &str| netlist.devices[d.0 as usize].terminals.iter().find(|(n, _)| n == t).map(|x| x.1);
    let tops = |d: DeviceId| s.members.iter().filter(|m| m.device != d).filter_map(|m| net(m.device, "P")).collect::<Vec<_>>();
    let mut it = s.members.iter().map(|m| m.device).filter(|&d| {
        let t = tops(d);
        [net(d, "P"), net(d, "N")].iter().all(|n| n.is_some_and(|n| t.contains(&n)))
    });
    let b = it.next();
    it.next().is_none().then_some(b).flatten()
}

/// A Unitization outside any matched set (no class, kind, style or series):
/// one target ratio per device equal to its drawn fingers ([`fingers`]).
/// `matched` sets both `dummy_required` and `route_matching_required`.
fn plain_unit(netlist: &Netlist, devices: Vec<DeviceId>, kind: DeviceKind, unit_w: i32, unit_l: i32, series_parallel: SeriesParallel, matched: bool) -> Unitization {
    let dev_nf: Vec<u16> = devices.iter().map(|&d| fingers(&netlist.devices[d.0 as usize])).collect();
    Unitization {
        devices,
        device_type: kind,
        target_ratio: dev_nf.clone(),
        dev_nf,
        unit_w,
        unit_l,
        series_parallel,
        dummy_required: matched,
        route_matching_required: matched,
        class: None,
        kind: None,
        series: Vec::new(),
        style: None,
    }
}

/// The cell-tier constraints of the module doc, in three passes: one
/// Unitization per matched set in `sets` whose banks share one finger W and L
/// (a set that does not is skipped and falls through), then per non-glue
/// block one per drawn class of its still-uncovered devices, then one per
/// group of identical parallel MOS left over. Every device lands in at most
/// one Unitization. Sizes are clamped to `0..=i32::MAX` nm, an unknown size
/// drawing as 0.
///
/// `drawn` is indexed by device id and must cover every device of
/// `netlist`; panics otherwise, or when a set or block names a device past it.
#[must_use]
pub fn assemble(netlist: &Netlist, drawn: &[Drawn], blocks: &[Block], sets: &[MatchSpec]) -> Constraints {
    let mut c = Constraints::default();
    let mut covered = vec![false; netlist.devices.len()];
    // An unknown size draws as 0 (and is reported missing by `annotate`).
    let clamp = |v: Option<i64>| v.unwrap_or(0).clamp(0, i64::from(i32::MAX)) as i32;
    for s in sets {
        // Drawn in schematic fingers (see the module doc): one common finger W and L.
        // A split DAC's bridge is drawn off the unit (`CapArray::split`), so only the
        // banks must share it; the bridge is the member whose `P` and `N` are both
        // other members' `P` (CELL-21).
        let bridge = if matches!(s.origin, Origin::PassiveSet { rule: "split_dac" }) { split_bridge(netlist, s) } else { None };
        let banks: Vec<DeviceId> = s.members.iter().map(|m| m.device).filter(|&d| Some(d) != bridge).collect();
        let d0 = drawn[banks[0].0 as usize];
        let same = banks.iter().all(|d| (drawn[d.0 as usize].w_finger_nm, drawn[d.0 as usize].l_nm) == (d0.w_finger_nm, d0.l_nm));
        if !same {
            continue;
        }
        let kind = netlist.devices[s.members[0].device.0 as usize].kind;
        let nf = |d: DeviceId| drawn[d.0 as usize].fingers.min(u32::from(u16::MAX)) as u16;
        // A bank or bipolar array lists its members as cellgen did: by count, the
        // reference (a terminating unit) first among equals, then by id.
        let mut members: Vec<DeviceId> = s.members.iter().map(|m| m.device).collect();
        if !matches!(kind, DeviceKind::Nmos | DeviceKind::Pmos) {
            let r = s.reference.map(|i| s.members[i].device);
            members.sort_by_key(|&d| (nf(d), Some(d) != r, d.0));
        }
        // `nf·m` for a MOS, `m` otherwise (`Drawn::fingers`).
        let dev_nf: Vec<u16> = members.iter().map(|&d| nf(d)).collect();
        let g = dev_nf.iter().fold(0, |a, &b| gcd(a, b)).max(1);
        members.iter().for_each(|d| covered[d.0 as usize] = true);
        c.unitization.push(Unitization {
            devices: members,
            device_type: kind,
            target_ratio: dev_nf.iter().map(|&n| n / g).collect(),
            dev_nf,
            unit_w: clamp(d0.w_finger_nm),
            unit_l: clamp(d0.l_nm),
            // A set's units are in parallel (`m` capacitors are `m` plates); a
            // resistor set with series units falls back above (one finger L).
            series_parallel: SeriesParallel::Parallel,
            dummy_required: dummy_required(kind, s.class),
            route_matching_required: s.kind != MatchKind::Ratio || s.class >= MatchClass::Moderate,
            class: Some(s.class),
            kind: Some(s.kind),
            series: vec![1; s.members.len()],
            style: Some(s.style),
        });
    }
    let class_of = |d: DeviceId| {
        let s = &drawn[d.0 as usize];
        (netlist.devices[d.0 as usize].kind, s.model, s.bulk, clamp(s.w_finger_nm), clamp(s.l_nm))
    };

    for b in blocks.iter().filter(|b| b.kind != BlockKind::Glue) {
        let mut classes = Vec::new();
        for &d in b.devices.iter().filter(|d| !covered[d.0 as usize]) {
            if !classes.contains(&class_of(d)) {
                classes.push(class_of(d));
            }
        }
        for class in classes {
            let (kind, _, _, unit_w, unit_l) = class;
            let devices: Vec<DeviceId> = b.devices.iter().copied().filter(|&d| !covered[d.0 as usize] && class_of(d) == class).collect();
            c.unitization.push(plain_unit(netlist, devices, kind, unit_w, unit_l, series_parallel(kind), true));
        }
        // A device shared by two blocks is drawn once, by the first.
        b.devices.iter().for_each(|d| covered[d.0 as usize] = true);
    }
    let keyed = netlist.devices.iter().enumerate().filter_map(|(i, d)| {
        if covered[i] || !matches!(d.kind, DeviceKind::Nmos | DeviceKind::Pmos) || drawn[i].w_finger_nm.is_none() {
            return None;
        }
        let mut t: Vec<(&str, u16)> = d.terminals.iter().map(|(n, x)| (n.as_str(), x.0)).collect();
        t.sort_unstable();
        Some(((d.kind as u8, drawn[i].model, drawn[i].w_finger_nm, drawn[i].l_nm, t), DeviceId(i as u16)))
    });
    for (key, devices) in crate::sets::group(keyed).into_iter().filter(|(_, v)| v.len() > 1) {
        let kind = netlist.devices[devices[0].0 as usize].kind;
        c.unitization.push(plain_unit(netlist, devices, kind, clamp(key.2), clamp(key.3), SeriesParallel::Parallel, false));
    }
    c
}

/// Greatest common divisor; `gcd(0, 0) = 0`.
fn gcd(a: u16, b: u16) -> u16 {
    if b == 0 { a } else { gcd(b, a % b) }
}
