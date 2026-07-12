//! User-facing custom-cell API for analog place and route.
//!
//! `substrate3` is a frontend facade over the backend-owned cell contract. A
//! custom macro implements [`CellGenerator`] here, while its [`CellBuilder`]
//! receives the same validated [`Pdk`] values used by built-in generators.

#![forbid(unsafe_code)]

pub use pnr_cells::pdk::Pdk;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_macro_reads_pdk_roles_and_dimensions() {
        let json = r#"{
            "layers": {"MetalOne": {"layer": 101, "datatype": 0}},
            "drc": {"off_grid": {"grid": 5}},
            "cell": {
                "layers": {"met1": "MetalOne"},
                "mcon_size": 215
            }
        }"#;
        let deck = Deck::from_json(json).expect("deck");
        let pdk = Pdk::from_json(json).expect("cell PDK");
        let macro_cell = |builder: &mut CellBuilder| {
            let process = builder.pdk()?;
            builder.rect(
                &process.layers.met1,
                0,
                0,
                process.mcon_size,
                process.mcon_size,
            )?;
            Ok(())
        };
        let mut builder =
            CellBuilder::with_context(CellContext::new(&deck, &pdk), MatchingTier::None, 0);

        macro_cell.generate(&mut builder).expect("generate macro");
        let output = builder.finish();
        assert_eq!(output.bbox.width(), 215);
        assert_eq!(
            output.store.poly_layer[0],
            deck.layers.id("MetalOne").unwrap()
        );
    }
}
pub use pnr_cells::{
    run_drc, run_lvs, run_pex, snap_to_grid, verify, verify_lvs, Bbox, CellBuilder, CellContext,
    CellError, CellGenerator, CellMeta, CellOutput, Deck, DeviceFlavor, DeviceKind, DeviceType,
    Direction, DrcReport, GeometryStore, LayerId, LayerTable, LvsResult, MatchingTier,
    MatchingType, Orientation, Parasitic, PatternType, PexReport, PinAccess, PolyId, PortDef,
    RefDevice, RefNetlist, VerifyReport, Violation,
};
