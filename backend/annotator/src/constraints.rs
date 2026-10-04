//! Cell-tier [`analog::Constraints`], read by `cells` when drawing devices.
//!
//! One **Unitization** per matched set (EXT-15) that has a unit and one finger
//! W and L over its members: members, ratio reduced by the gcd, dummies from the
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
//! - **Guard ring** for FETs, flavour by the *device's* polarity, tied to its bulk
//!   net. Bipolars get none: their `B` terminal is the base, not the bulk.

use analog::cell::{GuardRingRequirement, GuardRingType, SeriesParallel, Unitization};
use analog::intent::{MatchClass, MatchKind, MatchSpec};
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

#[must_use]
pub fn assemble(netlist: &Netlist, drawn: &[Drawn], blocks: &[Block], sets: &[MatchSpec]) -> Constraints {
    let mut c = Constraints::default();
    let mut covered = vec![false; netlist.devices.len()];
    // An unknown size draws as 0 (and is reported missing by `annotate`).
    let clamp = |v: Option<i64>| v.unwrap_or(0).clamp(0, i64::from(i32::MAX)) as i32;
    for s in sets {
        // Drawn in schematic fingers (see the module doc): one common finger W and L.
        let d0 = drawn[s.members[0].device.0 as usize];
        let same = s.members.iter().all(|m| (drawn[m.device.0 as usize].w_finger_nm, drawn[m.device.0 as usize].l_nm) == (d0.w_finger_nm, d0.l_nm));
        if s.unit.is_none() || !same {
            continue;
        }
        let kind = netlist.devices[s.members[0].device.0 as usize].kind;
        // `nf·m` for a MOS, `m` otherwise (`Drawn::fingers`).
        let dev_nf: Vec<u16> = s.members.iter().map(|m| drawn[m.device.0 as usize].fingers.min(u32::from(u16::MAX)) as u16).collect();
        let g = dev_nf.iter().fold(0, |a, &b| gcd(a, b)).max(1);
        s.members.iter().for_each(|m| covered[m.device.0 as usize] = true);
        c.unitization.push(Unitization {
            devices: s.members.iter().map(|m| m.device).collect(),
            device_type: kind,
            target_ratio: dev_nf.iter().map(|&n| n / g).collect(),
            dev_nf,
            unit_w: clamp(d0.w_finger_nm),
            unit_l: clamp(d0.l_nm),
            series_parallel: series_parallel(kind),
            same_variant_required: true,
            dummy_required: dummy_required(kind, s.class),
            route_matching_required: s.kind != MatchKind::Ratio || s.class >= MatchClass::Moderate,
            class: Some(s.class),
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
            let dev_nf: Vec<u16> = devices
                .iter()
                .map(|&d| fingers(&netlist.devices[d.0 as usize]))
                .collect();
            c.unitization.push(Unitization {
                devices,
                device_type: kind,
                target_ratio: dev_nf.clone(),
                dev_nf,
                unit_w,
                unit_l,
                series_parallel: series_parallel(kind),
                same_variant_required: true,
                dummy_required: true,
                route_matching_required: true,
                class: None, series: Vec::new(), style: None,
            });
        }

        for &d in &b.devices {
            let dev = &netlist.devices[d.0 as usize];
            let ring_type = match dev.kind {
                DeviceKind::Pmos => GuardRingType::Tap { in_well: true },
                DeviceKind::Nmos => GuardRingType::Tap { in_well: false },
                _ => continue,
            };
            let Some(&(_, bulk)) = dev.terminals.iter().find(|(t, _)| t == "B") else { continue };
            c.guard_rings.push(GuardRingRequirement {
                device: d,
                ring_type,
                // Victim rings merge with same-net same-type neighbours (post_cell).
                shareable: true,
                min_width_nm: 500,
                max_ring_resistance_mohm: 100_000,
                connection_net: bulk,
            });
        }
    }
    for u in &c.unitization {
        u.devices.iter().for_each(|d| covered[d.0 as usize] = true);
    }
    let mut parallel: Vec<(_, Vec<DeviceId>)> = Vec::new();
    for (i, d) in netlist.devices.iter().enumerate() {
        if covered[i] || !matches!(d.kind, DeviceKind::Nmos | DeviceKind::Pmos) || drawn[i].w_finger_nm.is_none() {
            continue;
        }
        let mut t: Vec<(&str, u16)> = d.terminals.iter().map(|(n, x)| (n.as_str(), x.0)).collect();
        t.sort_unstable();
        let key = (d.kind, drawn[i].model, drawn[i].w_finger_nm, drawn[i].l_nm, t);
        match parallel.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => v.push(DeviceId(i as u16)),
            None => parallel.push((key, vec![DeviceId(i as u16)])),
        }
    }
    for (key, devices) in parallel.into_iter().filter(|(_, v)| v.len() > 1) {
        let dev_nf: Vec<u16> = devices.iter().map(|&d| fingers(&netlist.devices[d.0 as usize])).collect();
        c.unitization.push(Unitization {
            devices,
            device_type: key.0,
            target_ratio: dev_nf.clone(),
            dev_nf,
            unit_w: clamp(key.2),
            unit_l: clamp(key.3),
            series_parallel: SeriesParallel::Parallel,
            same_variant_required: true,
            dummy_required: false,
            route_matching_required: false,
            class: None, series: Vec::new(), style: None,
        });
    }
    c
}

fn gcd(a: u16, b: u16) -> u16 {
    if b == 0 { a } else { gcd(b, a % b) }
}
