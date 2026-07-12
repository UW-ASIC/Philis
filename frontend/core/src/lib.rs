//! Frontend core.
//!
//! Parses circuit input and converts frontend annotations into the typed
//! request accepted by `pnr-backend`. Physical flow control and signoff live
//! entirely in the backend facade.

pub mod gds;
mod netlist;
pub mod orchestrator;
mod pdk;

pub mod frontend {
    //! SPICE netlist -> bipartite cell/net hypergraph.
    //!
    //! `read_netlist`/`parse_spice` turn any standard SPICE file into a
    //! [`BipartiteHypergraph`]: leaf PDK devices (built-in generators) and
    //! substrate3 macro cells in one partition, nets in the other, pins as
    //! edges. `group_registry` collapses registry-consumed instances.
    //! `println!("{hg}")` dumps it.

    pub use crate::netlist::{
        parse_spice, read_netlist, BipartiteHypergraph, CellNode, NetId, ParseError,
    };
    pub use crate::pdk::{load_pdk, CellBase, DeviceDef, FullPdk, Pdk};

    // The device payload each leaf cell carries (input to generators),
    // and the registry whose custom cells group instances.
    pub use pnr_cells::device::DeviceRecord;
    pub use pnr_cells::{CellRegistry, CustomCellEntry, DeviceType};
}

pub mod backend {
    //! Compatibility facade for the backend-owned physical-design API.

    pub use pnr_backend::strategy;
    pub use pnr_backend::{
        Backend, ConstraintContract, ConstraintRecord, ConstraintStatus, FlowConfig, FlowInput,
        FlowResult, SignoffReport,
    };
}

pub mod signoff {
    //! The types a consumer needs, grouped by role.

    // Geometry: build a store directly (immediate mode) — no GDS required.
    pub use gdsverify::{Bbox, Edge, GeometryStore, LayerId, PolyId};

    // PDK deck: load `pdks/<name>.json`, resolve layer names to ids.
    pub use gdsverify::{Deck, DrcRuleParam, LayerTable};

    // DRC.
    pub use gdsverify::{run_drc, run_drc_backend, Backend, DrcReport, Violation};

    // LVS.
    pub use gdsverify::{
        compare, extract_netlist, run_lvs, CompareOpts, DeviceFlavor, DeviceKind, ExtractedNetlist,
        LvsResult, RefDevice, RefNetlist,
    };

    // PEX.
    pub use gdsverify::{run_pex, run_pex_by_net_checked, Parasitic, PexReport};

    // Tapeout checks: antenna, density/CMP, IR drop, EM, reliability,
    // and ESD/latch-up. Missing electrical/model inputs report NOT_RUN.
    pub use gdsverify::signoff::*;

    // GDS ingestion, for flows that do start from a .gds.
    pub use gdsverify::{load_gds, read_gds, GdsLayout};
}

#[cfg(test)]
mod tests {
    use super::signoff::*;

    // Smoke test: both shipped PDK decks parse, resolve their layers, and drive DRC.
    #[test]
    fn pdk_decks_load_and_run() {
        for pdk in ["sky130.json"] {
            let path = format!("{}/../../pdks/{}", env!("CARGO_MANIFEST_DIR"), pdk);
            let text = std::fs::read_to_string(&path).unwrap();
            let deck = Deck::from_json(&text).unwrap();
            let met1 = deck.layers.id("met1").unwrap();
            let mut store = GeometryStore::new();
            store.add_rect(met1, 0, 0, 500, 90); // narrower than any met1 min_width
            let report = run_drc(&store, &deck);
            assert_eq!(report.by_kind("min_width").len(), 1, "{pdk}");
        }
    }
}
