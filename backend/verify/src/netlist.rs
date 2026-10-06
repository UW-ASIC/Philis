//! Post-layout netlist export: extract the drawn geometry and hand it to
//! gdsverify's SPICE writer. No schematic is consulted.

use gdsverify::engine::Checks;
use gdsverify::export::netlist::write_spice;
use gdsverify::export::Header;
use pnr_core::Shape;

use crate::checker::Checker;
use crate::geom::LabeledPin;
use crate::pdk::Pdk;

/// Whether to splice per-net parasitic R/C into the cards.
pub use gdsverify::export::netlist::Detail;

/// Extract `shapes` and write the recognised circuit as a SPICE `.subckt`.
///
/// `pins` name the nets that become subcircuit ports; an unlabelled net is
/// named by its `NetId`. [`Detail::WithParasitics`] runs PEX and splices its
/// per-net R/C in. The header carries no timestamp: same layout, same bytes.
///
/// # Errors
/// A deck the checker refuses, geometry that fails to load, an extraction
/// fault, or a circuit the writer cannot represent.
pub fn extract_spice(
    shapes: &[Shape],
    pins: &[LabeledPin],
    pdk: &Pdk,
    detail: Detail,
) -> Result<String, String> {
    let mut checker = Checker::new(pdk, false)?;
    let pex = detail == Detail::WithParasitics;
    checker.run(shapes, pins, Checks { drc: false, erc: false, lvs: false, pex })?;

    let header = Header {
        tool_version: concat!("philis ", env!("CARGO_PKG_VERSION")),
        deck_path: "<philis deck>".into(),
        layout_path: if pex { "<elaborated, post-layout>" } else { "<elaborated, schematic>" }
            .into(),
        timestamp: None,
    };
    let mut out = String::new();
    write_spice(
        checker.extracted().as_extraction(),
        checker.outputs().parasitics.as_ref(),
        detail,
        &checker.loaded.strings,
        &header,
        &mut out,
    )
    .map_err(|e| format!("netlist export: {e}"))?;
    Ok(out)
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use pnr_core::{Process, Rect};

    // Same layout, same bytes: the header carries no timestamp.
    #[test]
    fn extraction_is_deterministic_and_names_the_detail() {
        let pdk = Pdk::builtin("sky130").unwrap();
        let shapes = [Shape { layer: pdk.layer("met1").unwrap(), rect: Rect { x: 0, y: 0, w: 2000, h: 500 } }];
        let pins = [LabeledPin { name: "a".into(), layer: pdk.layer("met1").unwrap().0, x: 100, y: 250 }];
        for detail in [Detail::Schematic, Detail::WithParasitics] {
            let a = extract_spice(&shapes, &pins, &pdk, detail).unwrap();
            assert_eq!(a, extract_spice(&shapes, &pins, &pdk, detail).unwrap());
            assert!(a.to_lowercase().contains(".subckt"), "{a}");
        }
        let empty = extract_spice(&[], &[], &pdk, Detail::Schematic).unwrap();
        assert!(empty.to_lowercase().contains(".subckt"), "{empty}");
    }

    #[test]
    fn a_mislanded_pin_is_an_error() {
        let pdk = Pdk::builtin("sky130").unwrap();
        let pins = [LabeledPin { name: "a".into(), layer: pdk.layer("met1").unwrap().0, x: 0, y: 0 }];
        assert!(extract_spice(&[], &pins, &pdk, Detail::Schematic).is_err());
    }
}
