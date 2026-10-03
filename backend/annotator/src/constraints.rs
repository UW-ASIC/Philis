//! Cell-tier [`analog::Constraints`], read by `cells` when drawing devices.
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

#[must_use]
pub fn assemble(netlist: &Netlist, drawn: &[Drawn], blocks: &[Block]) -> Constraints {
    let mut c = Constraints::default();
    // An unknown size draws as 0 (and is reported missing by `annotate`).
    let clamp = |v: Option<i64>| v.unwrap_or(0).clamp(0, i64::from(i32::MAX)) as i32;
    let class_of = |d: DeviceId| {
        let s = &drawn[d.0 as usize];
        (netlist.devices[d.0 as usize].kind, s.model, s.bulk, clamp(s.w_finger_nm), clamp(s.l_nm))
    };

    for b in blocks.iter().filter(|b| b.kind != BlockKind::Glue) {
        let mut classes = Vec::new();
        for &d in &b.devices {
            if !classes.contains(&class_of(d)) {
                classes.push(class_of(d));
            }
        }
        for class in classes {
            let (kind, _, _, unit_w, unit_l) = class;
            let devices: Vec<DeviceId> = b.devices.iter().copied().filter(|&d| class_of(d) == class).collect();
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
