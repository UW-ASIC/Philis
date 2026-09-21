//! Post-layout netlist export — geometry in, SPICE out.
//!
//! The extractor already recovers everything a netlist needs: the deck's
//! `device_recognition` section names the model (`sky130_fd_pr__nfet_01v8`) and
//! the terminal roles, `connectivity` builds the nets, and `pex` measures the
//! parasitics. gdsverify's `export::netlist` writer turns that into a
//! `.subckt`. This module is the seam: run the checker, hand the extraction to
//! the writer.
//!
//! The netlist is a function of the drawn layout alone — no schematic is
//! consulted — so it is a *post-layout* netlist in the usual sense, and pairing
//! it with the same geometry's [`crate::drc`] is a complete signoff of a cell.

use gdsverify::engine::Checks;
use gdsverify::export::netlist::write_spice;
use gdsverify::export::parasitic::{write_dspf, write_spef};
use gdsverify::export::Header;
use pnr_core::Shape;

use crate::checker::Checker;
use crate::geom::LabeledPin;
use crate::pdk::Pdk;

/// Whether to splice per-net parasitic R/C into the cards.
pub use gdsverify::export::netlist::Detail;

/// What the writers stamp into the header comment. Not read from the clock:
/// the same layout must produce the same bytes.
fn header(detail: Detail) -> Header {
    Header {
        tool_version: concat!("philis ", env!("CARGO_PKG_VERSION")),
        deck_path: "<philis deck>".into(),
        layout_path: match detail {
            Detail::Schematic => "<elaborated, schematic>".into(),
            Detail::WithParasitics => "<elaborated, post-layout>".into(),
        },
        timestamp: None,
    }
}

/// Extract `shapes` and write the recognised circuit as a SPICE `.subckt`.
///
/// `pins` name the nets that become subcircuit ports; an unlabeled net gets its
/// `NetId` as a name. With [`Detail::WithParasitics`] the PEX stage runs and its
/// per-net R/C is spliced in, so the result simulates as a post-layout netlist.
///
/// # Errors
/// A deck the checker refuses, geometry that fails to load (a pin label landing
/// on no shape), an extraction fault, or a circuit the writer cannot represent.
pub fn extract_spice(
    shapes: &[Shape],
    pins: &[LabeledPin],
    pdk: &Pdk,
    detail: Detail,
) -> Result<String, String> {
    // `strip_density: false` — this is a signoff-grade read of finished
    // geometry, not an in-loop iteration where density windows are noise.
    let mut checker = Checker::new(pdk, false)?;
    let want_pex = detail == Detail::WithParasitics;
    checker.run(
        shapes,
        pins,
        Checks { drc: false, erc: false, lvs: false, pex: want_pex },
    )?;

    let mut out = String::new();
    write_spice(
        checker.extracted().as_extraction(),
        checker.outputs().parasitics.as_ref(),
        detail,
        &checker.loaded.strings,
        &header(detail),
        &mut out,
    )
    .map_err(|e| format!("netlist export: {e}"))?;
    Ok(out)
}

/// Extract `shapes` and write the parasitics alone, as SPEF or DSPF.
///
/// The netlist from [`extract_spice`] plus one of these is what a downstream
/// timing or signal-integrity tool expects; `Detail::WithParasitics` is the
/// single-file alternative.
///
/// # Errors
/// As [`extract_spice`], plus a network the chosen format cannot represent.
pub fn extract_parasitics(
    shapes: &[Shape],
    pins: &[LabeledPin],
    pdk: &Pdk,
    format: ParasiticFormat,
) -> Result<String, String> {
    let mut checker = Checker::new(pdk, false)?;
    checker.run(
        shapes,
        pins,
        Checks { drc: false, erc: false, lvs: false, pex: true },
    )?;

    // Fail closed: an empty string is a valid-looking file that claims the
    // layout has no parasitics, which is never true of real geometry.
    let Some(network) = checker.outputs().parasitics.as_ref() else {
        return Err("PEX produced no parasitic network".into());
    };

    let extraction = checker.extracted().as_extraction();
    let mut out = String::new();
    let head = header(Detail::WithParasitics);
    match format {
        ParasiticFormat::Spef => write_spef(
            network,
            extraction.ports,
            &checker.loaded.strings,
            &head,
            &mut out,
        ),
        ParasiticFormat::Dspf => write_dspf(
            network,
            extraction.ports,
            &checker.loaded.strings,
            &head,
            &mut out,
        ),
    }
    .map_err(|e| format!("parasitic export: {e}"))?;
    Ok(out)
}

/// Which standalone parasitic format [`extract_parasitics`] writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParasiticFormat {
    /// Standard Parasitic Exchange Format.
    Spef,
    /// Detailed Standard Parasitic Format.
    Dspf,
}
