//! Cell-tier structural [`analog::Constraints`] assembly.
//!
//! Unitization, guard rings, dummies, LDE, stress — these are **structural
//! directives, not Rules**: they have no `extract`, so the annotator builds them
//! here (migrated from `backend/constraints/src/cell_level/*` and the derivation
//! that populated them in `backend/src/flow.rs`). `cells` reads them cold at
//! generation time to pick and draw device variants.
//!
//! ## Where the numbers come from
//!
//! Sizing (`unit_w`/`unit_l`/`dev_nf`) is read from each device's netlist
//! `params` — the SPICE `W`/`L`/`nf`/`m` values, already in `nm`/PDK units. This
//! is the **only** channel by which per-device geometry reaches the pure `cells`
//! `Cell` trait (which is handed a `DeviceGroup` of ids + `Constraints`, never the
//! `Netlist`); see `cells/src/builder.rs::sizing`. So a `Unitization` is emitted
//! for *every* matched block, not just ratio-`>1:1` groups.
//!
//! The remaining scalars (guard-ring tap pitch / min width / resistance, dummy
//! moat extension + poly clearance, LDE well-edge distances + `ΔVth`/LOD budgets,
//! stress centroid bound) are genuinely PDK-derived. The annotator has **no PDK
//! handle**, so — per the task — each uses the representative default the old
//! `flow.rs` derivation used, in the `nm`/µV/etc. units the skeleton types now
//! carry. `cells` re-fits any that need real PDK/placement geometry downstream.

use crate::block::{Block, BlockKind};
use analog::cell::{
    DummyRequirement, DummyType, GuardRingRequirement, GuardRingType, LdeBound, SeriesParallel,
    StressBound, Unitization,
};
use analog::Constraints;
use pnr_core::ids::{DeviceId, NetId};
use pnr_core::netlist::DeviceKind;
use pnr_core::Netlist;

/// Read a device `param` by name (SPICE key), or `default` if absent.
fn param(nl: &Netlist, d: DeviceId, key: &str, default: i64) -> i64 {
    nl.devices
        .get(d.0 as usize)
        .and_then(|dev| dev.params.iter().find(|(k, _)| k == key).map(|(_, v)| *v))
        .unwrap_or(default)
}

/// A block that carries a recognised, matched device set (everything but glue).
fn is_matched(b: &Block) -> bool {
    !matches!(b.kind, BlockKind::Glue) && !b.devices.is_empty()
}

/// Assemble the cell-tier structural constraints for `netlist`'s recognised
/// `blocks`, migrating the derivation from `backend/src/flow.rs` §6/§12–15.
#[must_use]
pub fn assemble(netlist: &Netlist, blocks: &[Block]) -> Constraints {
    let mut c = Constraints::default();

    for b in blocks {
        if !is_matched(b) {
            continue;
        }

        // ── Unitization (flow.rs §15, generalised to every matched block) ──
        // The data channel for per-device W/L/nf into the pure `cells` trait.
        // `nf` = fingers/segments per instance; ratios between instances are the
        // finger counts themselves (a 1:4 mirror is `dev_nf = [1, 4]`). Unit
        // geometry is the shared finger W/L (params "w"/"l", already nm).
        let kind = netlist
            .devices
            .get(b.devices[0].0 as usize)
            .map_or(DeviceKind::Nmos, |d| d.kind);
        let dev_nf: Vec<u16> = b
            .devices
            .iter()
            .map(|&d| param(netlist, d, "nf", 1).clamp(1, i64::from(u16::MAX)) as u16)
            .collect();
        let unit_w = param(netlist, b.devices[0], "w", 0).clamp(0, i64::from(i32::MAX)) as i32;
        let unit_l = param(netlist, b.devices[0], "l", 0).clamp(0, i64::from(i32::MAX)) as i32;
        c.unitization.push(Unitization {
            devices: b.devices.clone(),
            device_type: kind,
            target_ratio: dev_nf.clone(),
            dev_nf,
            unit_w,
            unit_l,
            // Resistors/caps stack in series; FETs and their mirrors parallel fingers.
            series_parallel: match kind {
                DeviceKind::Resistor | DeviceKind::Capacitor => SeriesParallel::Series,
                _ => SeriesParallel::Parallel,
            },
            same_variant_required: true,
            dummy_required: true,
            route_matching_required: true,
        });

        // Per-device structural directives over the block's members.
        for &d in &b.devices {
            // ── Stress (flow.rs §6): pull matched devices toward the centroid ──
            // Old bound was 50 µm; skeleton field is nm.
            c.stress.push(StressBound { device: d, max_centroid_distance_nm: 50_000 });

            // ── Dummies (flow.rs §14): full edge dummies for LOD matching ──
            // Old moat_ext 0.2 µm → 200 nm; poly clearance 0.1 µm → 100 nm (PDK
            // representatives; `cells` re-fits to real design rules).
            c.dummies.push(DummyRequirement {
                device: d,
                dummy_type: DummyType::Full,
                moat_ext_nm: 200,
                min_poly_clearance_nm: 100,
            });

            // ── Guard rings: isolation-sensitive FETs get a well/sub ring ──
            // flow.rs §12 emitted rings only for pad-net *injectors*; the annotator
            // has no pad-net list, so it guards every matched FET (the isolation-
            // sensitive set it can see). Ring flavour follows device polarity, as
            // the old code keyed off Pmos→n-well vs else→p-sub. Scalars are the
            // old representative defaults (tap 2 µm, width 0.5 µm, R 100 Ω).
            match kind {
                DeviceKind::Pmos => c.guard_rings.push(guard_ring(d, GuardRingType::Hcgr)),
                DeviceKind::Nmos => c.guard_rings.push(guard_ring(d, GuardRingType::Ecgr)),
                _ => {}
            }
        }

        // ── LDE bounds (flow.rs §13): every matched FET pair gets WPE/LOD budget ──
        // SA/SB from finger geometry: outer = poly pitch, inner = half pitch, where
        // poly_pitch ≈ W/nf + 0.2 µm (contacted-poly overhead). Old distances were
        // µm; skeleton fields are nm / µV / hundredths-of-percent.
        if matches!(kind, DeviceKind::Nmos | DeviceKind::Pmos) {
            for pair in b.devices.windows(2) {
                let (a, bb) = (pair[0], pair[1]);
                let w = param(netlist, a, "w", 0).max(0);
                let nf = param(netlist, a, "nf", 1).max(1);
                // poly pitch, nm: W/nf + 200 nm overhead.
                let poly_pitch = (w / nf + 200).clamp(0, i64::from(i32::MAX)) as i32;
                c.lde.push(LdeBound {
                    pair: (a, bb),
                    min_well_edge_distance_nm: 1_000, // 1 µm
                    max_wpe_dvth_uv: 2_000,           // 2 mV
                    max_lod_did_cpct: 200,            // 2.00 %
                    max_sa_sb_mismatch_nm: 100,       // 0.1 µm
                    outer_sa_nm: poly_pitch,
                    outer_sb_nm: poly_pitch,
                    inner_sa_nm: poly_pitch / 2,
                    inner_sb_nm: poly_pitch / 2,
                    sti_dvth_uv: 0,
                    same_width: true,
                    same_orientation: true,
                });
            }
        }
    }

    c
}

/// Guard ring with the old `flow.rs` §12 representative scalars. The ring ties to
/// the substrate/well rail; the annotator has no resolved net table for it, so it
/// uses the conventional tap net id 0 (`cells` binds the real rail downstream).
fn guard_ring(device: DeviceId, ring_type: GuardRingType) -> GuardRingRequirement {
    GuardRingRequirement {
        device,
        ring_type,
        // Matched FETs are *victims* / latch-up rings, not substrate injectors, so
        // their rings SHARE: `post_cell` merges same-class (net + ring_type)
        // adjacent requesters into one ring per well cluster, instead of a wide
        // private ring around each device that overlaps its neighbours (ADR 0006).
        // An injector would set this `false` to force a private ring.
        shareable: true,
        tap_pitch_nm: 2_000,           // 2 µm
        min_width_nm: 500,             // 0.5 µm
        max_ring_resistance_mohm: 100_000, // 100 Ω
        enclosure_complete: true,
        connection_net: NetId(0),
    }
}
