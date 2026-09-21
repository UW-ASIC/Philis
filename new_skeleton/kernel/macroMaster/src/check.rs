//! The checks — deliberately **not** the gpurify geometric engine.
//!
//! Two tiers, two entry points (see `docs/adr/0004`):
//!
//! - [`check_device`] validates a [`DeviceGen`] against the [`GenericPdk`]: it
//!   draws the device (which enforces on-grid), verifies no declared port is
//!   floating, and runs the gpurify Device signoff — the one place gpurify
//!   attaches, kept behind the seam marked below.
//! - [`check`] / [`check_lvs`] validate a [`Composition`]: purely *structural*.
//!   Overlap is already rejected at [`crate::CompBuilder::place`] time (surfaced
//!   here as a `GenError`); what remains is the floating-port check and, with a
//!   [`Schematic`], **structural LVS** — the declared connect-graph vs the
//!   reference netlist, needing no geometry. Real geometric DRC/LVS of the
//!   *routed* result is downstream signoff (`backend/verify`), not this crate.
//!
//! Every `Report` here leaves `budget_violations` empty (`..Report::default()`).
//! Both tiers are strict-legality only — on-grid, non-overlapping, non-floating,
//! DRC — and none of them is a *budget* with a residual to price. Θ is the
//! orchestrator's tier, fed by the placement/routing stages.

use std::collections::HashSet;

use pnr_core::{Netlist, Report, Violation};

use crate::{
    Block, Composition, DeviceBuilder, DeviceGen, GenError, GenericPdk, Io, Process, Schematic,
};

// ═══════════════════════════════════════════════════════════════════════
//  Device tier — the only gpurify attachment point
// ═══════════════════════════════════════════════════════════════════════

/// Validate a [`DeviceGen`] against the [`GenericPdk`]. Draws it (enforcing
/// on-grid), checks no declared port is floating, then runs the gpurify Device
/// signoff seam. A clean report is the evidence the Device is portable.
#[must_use]
pub fn check_device<D: DeviceGen>(dev: &D, generic: &GenericPdk) -> Report {
    let mut builder = cells::Builder::new(generic.grid());
    let gen_err = {
        let mut cell = DeviceBuilder::new(&mut builder, generic);
        match dev.layout(&mut cell) {
            Ok(()) => floating(dev, &cell.nets_edges()),
            Err(e) => Some(e),
        }
    };
    if let Some(e) = gen_err {
        return Report { hard_violations: vec![gen_error_violation(e)], cost: f32::INFINITY, ..Report::default() };
    }
    let mac = builder.finish();

    // gpurify DEVICE signoff (real DRC vs the GenericPdk). The only gpurify
    // attachment in this crate; `gdsverify` is pulled in only under `gpurify`.
    let hard_violations = run_device_signoff(&mac, generic);
    Report { hard_violations, cost: 0.0, ..Report::default() }
}

/// gpurify Device-signoff: run real DRC (via `verify`/`gdsverify`) on the drawn
/// geometry against the hardcoded Generic PDK deck, mapping each finding to a
/// hard [`Violation`] with its nm shortfall as the margin.
#[cfg(feature = "gpurify")]
fn run_device_signoff(mac: &pnr_core::Macro, generic: &GenericPdk) -> Vec<Violation> {
    // Engine failures come back as `engine/…` findings and are kept: a Device
    // signoff that could not run must not read as clean (fail closed).
    verify::drc(&mac.shapes, &[], generic.pdk())
        .into_iter()
        .map(|f| Violation {
            rule: format!("drc/{}:{}", f.rule, f.layer),
            margin: f.margin_nm,
        })
        .collect()
}

/// Without the `gpurify` feature the Device check is structural only (on-grid +
/// no floating ports); the engine is not linked.
#[cfg(not(feature = "gpurify"))]
fn run_device_signoff(mac: &pnr_core::Macro, _generic: &GenericPdk) -> Vec<Violation> {
    let _ = mac;
    Vec::new()
}

// ═══════════════════════════════════════════════════════════════════════
//  Composition tier — structural only, no geometric engine
// ═══════════════════════════════════════════════════════════════════════

/// Validate a [`Composition`]: build it (surfacing any `place`-time overlap /
/// off-grid `GenError`) and check no declared port is floating. No LVS — see
/// [`check_lvs`].
#[must_use]
pub fn check<C: Composition>(comp: &C, pdk: &impl Process) -> Report {
    match build(comp, pdk) {
        Err(e) => Report { hard_violations: vec![gen_error_violation(e)], cost: f32::INFINITY, ..Report::default() },
        Ok(b) => Report { hard_violations: floating_from(&b), cost: 0.0, ..Report::default() },
    }
}

/// Validate a [`Composition`] that has a [`Schematic`], adding **structural LVS**
/// (declared connect-graph vs the reference netlist) to [`check`].
#[must_use]
pub fn check_lvs<C: Composition + Schematic>(comp: &C, pdk: &impl Process) -> Report {
    match build(comp, pdk) {
        Err(e) => Report { hard_violations: vec![gen_error_violation(e)], cost: f32::INFINITY, ..Report::default() },
        Ok(b) => {
            let mut hard = floating_from(&b);
            hard.extend(structural_lvs(&b, &comp.schematic()));
            Report { hard_violations: hard, cost: 0.0, ..Report::default() }
        }
    }
}

/// What a built [`Composition`] declared, distilled for the structural checks.
struct Built {
    ports: Vec<String>,
    edges: Vec<(String, String)>,
    devices: usize,
}

fn build<C: Composition, P: Process>(comp: &C, pdk: &P) -> Result<Built, GenError> {
    let b = crate::build_composition(comp, pdk)?;
    // Count **schematic devices**, not placed instances. They used to be the
    // same number only because every composition was flat `Mos`es; a
    // sub-composition is one instance and many transistors, and a
    // [`crate::variants::MatchedPair`] is one instance and two — so the old
    // `placed_count()` reported "built 2, schematic 4" for a hierarchy that was
    // in fact correct. An opaque `DeviceGen` leaves no netlist and no better
    // answer than the instance count.
    let devices = b.netlist.as_ref().map_or(b.instances.len(), |n| n.devices.len());
    Ok(Built { ports: b.ports, edges: b.edges, devices })
}

// --- structural checks ---------------------------------------------------

fn port_names<B: Block>(block: &B) -> Vec<String> {
    block.io().ports().into_iter().map(|p| p.name).collect()
}

/// Declared ports that no `connect` edge reaches ⇒ [`GenError::Disconnected`].
fn floating<D: DeviceGen>(dev: &D, edges: &[(String, String)]) -> Option<GenError> {
    let wired: HashSet<&str> =
        edges.iter().flat_map(|(a, b)| [a.as_str(), b.as_str()]).collect();
    port_names(dev)
        .iter()
        .any(|p| !wired.contains(p.as_str()))
        .then_some(GenError::Disconnected)
}

/// Floating declared ports on a composition, as [`Violation`]s (one per port).
fn floating_from(b: &Built) -> Vec<Violation> {
    let wired: HashSet<&str> =
        b.edges.iter().flat_map(|(a, x)| [a.as_str(), x.as_str()]).collect();
    b.ports
        .iter()
        .filter(|p| !wired.contains(p.as_str()))
        .map(|p| Violation { rule: format!("floating port {p}"), margin: 0 })
        .collect()
}

/// Structural LVS: the declared connectivity vs the reference schematic. Two
/// independent facts must agree — device **count** and the **port-named net
/// classes** the `connect` edges resolve to (union-find; a class is named by
/// the io port it contains) vs the schematic's nets that are ports. Purely
/// internal nets carry synthesized names on the built side, so they are not
/// name-comparable; graph-iso LVS of the routed result is downstream signoff.
fn structural_lvs(b: &Built, schem: &Netlist) -> Vec<Violation> {
    let mut v = Vec::new();
    if b.devices != schem.devices.len() {
        v.push(Violation {
            rule: format!("lvs device count: built {}, schematic {}", b.devices, schem.devices.len()),
            margin: (b.devices as i64 - schem.devices.len() as i64).abs(),
        });
    }
    let (names, _) = crate::resolve_nets(&b.edges, &b.ports);
    let portset: HashSet<&str> = b.ports.iter().map(String::as_str).collect();
    let built_nets: HashSet<&str> =
        names.iter().map(String::as_str).filter(|n| portset.contains(n)).collect();
    let schem_nets: HashSet<&str> = schem
        .nets
        .iter()
        .map(|n| n.name.as_str())
        .filter(|n| portset.contains(n))
        .collect();
    if built_nets != schem_nets {
        v.push(Violation { rule: "lvs net set mismatch vs schematic".into(), margin: 0 });
    }
    v
}

fn gen_error_violation(e: GenError) -> Violation {
    let rule = match e {
        GenError::OffGrid => "gen off-grid coordinate",
        GenError::IllegalLayer => "gen illegal layer",
        GenError::Disconnected => "gen disconnected/floating port",
        GenError::Overlap => "gen device overlap",
    };
    Violation { rule: rule.to_string(), margin: 0 }
}
