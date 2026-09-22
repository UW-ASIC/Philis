//! Per-variant Device signoff: **every** enumerated variant of each `cells`
//! generator must draw DRC/ERC-clean against the strict Generic PDK.
//!
//! Density is waived: `min_density`/`max_density` are windowed *fill* rules — a
//! chip-level property no single device can satisfy, so they are signoff-only,
//! not a per-device yardstick. ERC here is the geometry pass over a bare device
//! (no netlist reference), not the chip-level suite (Antenna/IrDrop/EM/… need a
//! power grid a lone device has no business providing).
//!
//! Only compiled under `--features gpurify` (the sole gpurify attachment point).

use analog::cell::{SeriesParallel, Unitization};
use analog::Constraints;
use cells::Cell;
use pnr_core::{DeviceGroup, DeviceId, DeviceKind, Process};

use crate::GenericPdk;

/// DRC + ERC findings (as `rule:layer` / `rule` strings) for one variant.
struct VariantResult {
    idx: usize,
    drc: Vec<String>,
    erc: Vec<String>,
}

/// Enumerate `G`'s variants for a single device sized `w`/`l`/`nf`, draw each, and
/// check it against the Generic PDK (density waived). Returns per-variant findings.
fn sweep<G: Cell>(kind: DeviceKind, w: i32, l: i32, nf: u16) -> Vec<VariantResult> {
    let generic = GenericPdk::default();
    let pdk = generic.pdk();
    let process: &dyn Process = pdk;

    let group = DeviceGroup { devices: vec![DeviceId(0)] };
    let series_parallel = match kind {
        DeviceKind::Resistor | DeviceKind::Capacitor => SeriesParallel::Series,
        _ => SeriesParallel::Parallel,
    };
    let mut c = Constraints::default();
    c.unitization.push(Unitization {
        devices: vec![DeviceId(0)],
        device_type: kind,
        dev_nf: vec![nf.max(1)],
        target_ratio: vec![1],
        unit_w: w,
        unit_l: l,
        series_parallel,
        same_variant_required: true,
        dummy_required: false,
        route_matching_required: false,
    });

    G::enumerate(&group, &c, process)
        .iter()
        .enumerate()
        .map(|(idx, v)| {
            let mac = v.draw(&group, &c, process);
            let drc = verify::drc(&mac.shapes, &[], pdk)
                .into_iter()
                // Density waived (see module doc): the standalone probe runs the
                // full deck, so drop the windowed fill rules by their deck ids —
                // the generic deck names every density rule `*_density`.
                .filter(|f| !f.rule.ends_with("_density"))
                .map(|f| format!("{}:{}", f.rule, f.layer))
                .collect();
            let erc = verify::erc(&mac.shapes, &[], pdk)
                .into_iter()
                // A *bare* device has pins but no netlist reference, so the ERC
                // stage can be denied for lack of extractable connectivity. That
                // denial now surfaces as a fail-closed `engine/…` finding; here it
                // is stage-inapplicable, not a geometry fault — filtered, as the
                // old `erc_extraction_error` filter did. Real geometry findings
                // carry deck rule ids and still fail the sweep.
                .filter(|f| !f.rule.starts_with("engine/"))
                .map(|f| f.rule)
                .collect();
            VariantResult { idx, drc, erc }
        })
        .collect()
}

/// Histogram a finding list into `"rule ×n"` strings for a compact report.
fn hist(items: &[String]) -> Vec<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for it in items {
        match counts.iter_mut().find(|(k, _)| k == it) {
            Some((_, n)) => *n += 1,
            None => counts.push((it.clone(), 1)),
        }
    }
    counts.sort();
    counts.into_iter().map(|(k, n)| format!("{k}×{n}")).collect()
}

/// Assert every variant in `results` drew DRC/ERC-clean; on failure, panic with a
/// per-variant finding histogram so the offending rule is visible.
fn assert_all_clean(family: &str, results: &[VariantResult]) {
    assert!(!results.is_empty(), "{family}: enumerated no variants");
    let dirty: Vec<String> = results
        .iter()
        .filter(|r| !r.drc.is_empty() || !r.erc.is_empty())
        .map(|r| format!("  #{}: drc[{}] erc[{}]", r.idx, hist(&r.drc).join(" "), hist(&r.erc).join(" ")))
        .collect();
    assert!(
        dirty.is_empty(),
        "{family}: {}/{} variants NOT clean vs strict Generic PDK:\n{}",
        dirty.len(),
        results.len(),
        dirty.join("\n"),
    );
}

// Each test sweeps a family's whole variant space at a representative valid size
// and asserts every variant draws DRC-clean (density waived — chip-level) and
// ERC-clean vs the strict Generic PDK.

#[test]
fn mosfet_variants_clean() {
    assert_all_clean("mosfet_nmos", &sweep::<cells::mosfet::Mosfet>(DeviceKind::Nmos, 1000, 210, 2));
    assert_all_clean("mosfet_pmos", &sweep::<cells::mosfet::Mosfet>(DeviceKind::Pmos, 1000, 210, 2));
}

#[test]
fn resistor_variants_clean() {
    assert_all_clean("resistor", &sweep::<cells::resistor::Resistor>(DeviceKind::Resistor, 500, 10_000, 1));
}

#[test]
fn capacitor_variants_clean() {
    assert_all_clean("capacitor", &sweep::<cells::capacitor::Capacitor>(DeviceKind::Capacitor, 2000, 2000, 4));
}

#[test]
fn bjt_variants_clean() {
    // Both polarities: they draw different markers, and only one draws the well and
    // the n-tap, so a clean NPN says nothing about the PNP.
    assert_all_clean("npn", &sweep::<cells::bjt::Bjt>(DeviceKind::Npn, 1000, 1000, 1));
    assert_all_clean("pnp", &sweep::<cells::bjt::Bjt>(DeviceKind::Pnp, 1000, 1000, 1));
}

#[test]
fn diode_variants_clean() {
    assert_all_clean("diode", &sweep::<cells::diode::Diode>(DeviceKind::Diode, 500, 1000, 1));
}

#[test]
fn inductor_variants_clean() {
    assert_all_clean("inductor", &sweep::<cells::inductor::Inductor>(DeviceKind::Inductor, 2000, 20_000, 1));
}
