//! Cell-tier [`analog::Constraints`], read by `cells` when drawing devices.
//!
//! For every device of every recognised block:
//! - **Unitization**, one per (kind, W, L) class of the block — the only channel
//!   by which W/L/nf reach `cells`. One class per unitization: `unit_w`/`unit_l`
//!   are group scalars, and a mixed group draws members at the wrong size (LVS
//!   `parameter_mismatch`).
//! - **Guard ring** for FETs, flavour by the *device's* polarity, tied to its bulk
//!   net. Bipolars get none: their `B` terminal is the base, not the bulk.

use analog::cell::{GuardRingRequirement, GuardRingType, SeriesParallel, Unitization};
use analog::Constraints;
use pnr_core::ids::DeviceId;
use pnr_core::netlist::DeviceKind;
use pnr_core::Netlist;

use crate::block::{Block, BlockKind};
use crate::param;

#[must_use]
pub fn assemble(netlist: &Netlist, blocks: &[Block]) -> Constraints {
    let mut c = Constraints::default();
    let clamp = |v: i64| v.clamp(0, i64::from(i32::MAX)) as i32;
    let class_of = |d: DeviceId| {
        let dev = &netlist.devices[d.0 as usize];
        (dev.kind, clamp(param(dev, "w", 0)), clamp(param(dev, "l", 0)))
    };

    for b in blocks.iter().filter(|b| b.kind != BlockKind::Glue) {
        let mut classes: Vec<(DeviceKind, i32, i32)> = Vec::new();
        for &d in &b.devices {
            if !classes.contains(&class_of(d)) {
                classes.push(class_of(d));
            }
        }
        for (kind, unit_w, unit_l) in classes {
            let devices: Vec<DeviceId> =
                b.devices.iter().copied().filter(|&d| class_of(d) == (kind, unit_w, unit_l)).collect();
            let dev_nf: Vec<u16> = devices
                .iter()
                .map(|&d| param(&netlist.devices[d.0 as usize], "nf", 1).clamp(1, i64::from(u16::MAX)) as u16)
                .collect();
            c.unitization.push(Unitization {
                devices,
                device_type: kind,
                target_ratio: dev_nf.clone(),
                dev_nf,
                unit_w,
                unit_l,
                series_parallel: match kind {
                    DeviceKind::Resistor | DeviceKind::Capacitor => SeriesParallel::Series,
                    _ => SeriesParallel::Parallel,
                },
                same_variant_required: true,
                dummy_required: true,
                route_matching_required: true,
            });
        }

        for &d in &b.devices {
            let dev = &netlist.devices[d.0 as usize];
            let ring_type = match dev.kind {
                DeviceKind::Pmos => GuardRingType::Hcgr,
                DeviceKind::Nmos => GuardRingType::Ecgr,
                _ => continue,
            };
            let Some(&(_, bulk)) = dev.terminals.iter().find(|(t, _)| t == "B") else { continue };
            c.guard_rings.push(GuardRingRequirement {
                device: d,
                ring_type,
                // Victim rings merge with same-net same-type neighbours (post_cell).
                shareable: true,
                tap_pitch_nm: 2_000,
                min_width_nm: 500,
                max_ring_resistance_mohm: 100_000,
                enclosure_complete: true,
                connection_net: bulk,
            });
        }
    }
    c
}
