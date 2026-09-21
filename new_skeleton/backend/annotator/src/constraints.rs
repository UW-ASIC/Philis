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
        //
        // `device_type`/`unit_w`/`unit_l` are **group** scalars — one kind and one
        // finger geometry for every member — so one per (kind, W, L) class of the
        // block, not one per block. A block whose members disagree (a CMOS
        // inverter: P at W=1u, N at W=0.5u) is not one unitization, and stating
        // `b.devices[0]`'s W for all of them draws every other member at the wrong
        // size: `cells::builder::sizing` resolves a lone device through whichever
        // unitization *covers* it, so the N above came out 1 µm wide and LVS read
        // `lvs.parameter_mismatch` (w: layout 1e-6 vs reference 5e-7).
        let class_of = |d: DeviceId| {
            (
                netlist.devices.get(d.0 as usize).map_or(DeviceKind::Nmos, |dev| dev.kind),
                param(netlist, d, "w", 0).clamp(0, i64::from(i32::MAX)) as i32,
                param(netlist, d, "l", 0).clamp(0, i64::from(i32::MAX)) as i32,
            )
        };
        let mut classes: Vec<(DeviceKind, i32, i32)> = Vec::new();
        for &d in &b.devices {
            let key = class_of(d);
            if !classes.contains(&key) {
                classes.push(key);
            }
        }
        // Kept for the genuinely block-scoped directives below (the LDE gate).
        // Anything *per-device* must read `class_of(d).0`, not this — see the
        // guard-ring flavour.
        let kind = classes[0].0;
        for (class_kind, unit_w, unit_l) in classes {
            let devices: Vec<DeviceId> = b
                .devices
                .iter()
                .copied()
                .filter(|&d| class_of(d) == (class_kind, unit_w, unit_l))
                .collect();
            let dev_nf: Vec<u16> = devices
                .iter()
                .map(|&d| param(netlist, d, "nf", 1).clamp(1, i64::from(u16::MAX)) as u16)
                .collect();
            c.unitization.push(Unitization {
                devices,
                device_type: class_kind,
                target_ratio: dev_nf.clone(),
                dev_nf,
                unit_w,
                unit_l,
                // Resistors/caps stack in series; FETs and their mirrors parallel fingers.
                series_parallel: match class_kind {
                    DeviceKind::Resistor | DeviceKind::Capacitor => SeriesParallel::Series,
                    _ => SeriesParallel::Parallel,
                },
                same_variant_required: true,
                dummy_required: true,
                route_matching_required: true,
            });
        }

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
            //
            // `class_of(d).0`, not the block's `kind`: ring flavour is the
            // *device's* polarity, and a mixed block is exactly the case that
            // matters — rc_filter's inverter (P first, then N) handed the NMOS
            // the PMOS's `Hcgr`, a p+ ring in an n-well wrapped round an
            // n-channel. Latent while `Hcgr` drew no well; the moment the well
            // moved to the ring type that actually has one it buried the NMOS
            // and magic stopped extracting the nfet. Same failure the
            // unitization comment above describes, one loop down.
            let bulk = bulk_net(netlist, d);
            match class_of(d).0 {
                DeviceKind::Pmos => c.guard_rings.push(guard_ring(d, GuardRingType::Hcgr, bulk)),
                DeviceKind::Nmos => c.guard_rings.push(guard_ring(d, GuardRingType::Ecgr, bulk)),
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

/// The device's bulk (`B`) terminal net — what a guard ring ties to.
///
/// Falls back to `NetId(0)` for a device whose card states no bulk: the
/// pre-existing behaviour, and no worse than it was.
fn bulk_net(nl: &Netlist, d: DeviceId) -> NetId {
    nl.devices
        .get(d.0 as usize)
        .and_then(|dev| dev.terminals.iter().find(|(t, _)| t == "B").map(|&(_, n)| n))
        .unwrap_or(NetId(0))
}

/// Guard ring with the old `flow.rs` §12 representative scalars.
///
/// `connection_net` is the guarded device's **bulk** net — the substrate/well
/// rail the ring taps. It used to be a hardcoded `NetId(0)`, the "conventional
/// tap net", on the promise that `cells` bound the real rail downstream. Nothing
/// did: `cellgen::bind_pins` only rewrites `d{N}:TERM` pins and a ring pin is
/// named `"ring"`, so every ring in the design stayed on net 0 — whatever net
/// the netlist happened to number first. `dr` folds ring pins in as real routing
/// terminals, so the router then wired every guard ring to that signal net. On
/// `chain4` net `a` grew 12 extra terminals spread across the die and 93 wires,
/// and the congestion left the 4-pin gate net open. It only looked harmless on
/// the other fixtures because their net 0 happens to be a real device net.
fn guard_ring(
    device: DeviceId,
    ring_type: GuardRingType,
    connection_net: NetId,
) -> GuardRingRequirement {
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
        connection_net,
    }
}
