//! # gdsverify
//!
//! An (optionally) GPU-accelerated GDS verification core: **DRC**, **LVS**, and **PEX** over a
//! data-oriented geometry store.
//!
//! ## Two ways in (granularity, per the API-design skill)
//!
//! 1. **From GDS** — read a `.gds` and a [`VerifySchema`], then run checks per cell.
//! 2. **From structs** — build a [`GeometryStore`] directly (immediate mode; the caller owns
//!    all data) and run the same checks. Nothing about the checkers requires going through GDS.
//!
//! ```ignore
//! use gdsverify::{Deck, GeometryStore, VerifySchema, run_drc};
//! let deck = Deck::from_schema(schema).unwrap();
//! let mut store = GeometryStore::new();
//! let met1 = deck.layers.id("met1").unwrap();
//! store.add_rect(met1, 0, 0, 500, 90); // a too-narrow wire
//! let report = run_drc(&store, &deck);
//! assert_eq!(report.by_kind("min_width").len(), 1);
//! ```
//!
//! ## The data-oriented core
//!
//! [`GeometryStore`] is struct-of-arrays: flat `verts_x`/`verts_y` plus per-polygon index
//! ranges and bboxes. Shapes are referenced by `u32` index, never by pointer, so the same
//! buffers feed the CPU scanline passes and (unchanged) a cross-platform GPU kernel via the
//! [`gpu`] backend.

pub mod geometry;
pub mod params;
pub mod schema;
pub mod drc;
pub mod lvs;
pub mod pex;
pub mod erc;
pub mod gds;
pub mod traits;
pub use traits as gpu;

pub use geometry::{Bbox, Edge, GeometryStore, LayerId, PolyId};
pub use params::{Deck, DrcRuleParam, LayerDef, LayerTable};
pub use schema::{DrcRuleSchema, LvsSchema, VerifySchema};
pub use drc::{run_drc, run_drc_backend, run_drc_backend_strict, DrcReport, Violation};
pub use pex::{run_pex, run_pex_by_net, NetParasitics, Parasitic, PexReport};
pub use lvs::{compare, extract_netlist, extract_netlist_opts, reduce_netlist,
              to_spice, CompareOpts, DeviceFlavor, DeviceKind, Device,
              ExtractOpts, ExtractedNetlist, LvsResult, PortMap, RefDevice, RefNetlist,
              RefTwoTerminal, SpiceOpts, TwoTerminalKind, TwoTerminalDevice};
pub use erc::{run_erc, ErcReport, ErcViolation, MultipleDriverCheck, TieHighLowCheck};
pub use gds::{read_gds, GdsLayout};
pub use traits::{Backend, VerifyCheck};

/// Convenience: read a GDS file into per-cell stores using the deck's layer table.
pub fn load_gds(path: &str, deck: &Deck) -> Result<GdsLayout, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    read_gds(&bytes, &deck.layers)
}

/// Run LVS on an extracted store against a reference netlist. Returns extraction errors
/// as a failing LvsResult (matched=false).
pub fn run_lvs(store: &GeometryStore, deck: &Deck, reference: &RefNetlist) -> LvsResult {
    let opts = ExtractOpts { cut_required: deck.lvs_cut_required, ..Default::default() };
    let ext = match extract_netlist_opts(store, deck, &opts, traits::Backend::Cpu) {
        Ok(e) => e,
        Err(e) => return LvsResult {
            matched: false, reason: format!("extraction failed: {}", e),
            extracted_devices: 0, nmos: 0, pmos: 0,
            ambiguous_classes: 0, label_conflicts: Vec::new(),
            mismatches: Vec::new(), floating_nets: Vec::new(),
        },
    };
    let cmp_opts = CompareOpts {
        strict: false,
        w_tolerance: deck.w_tolerance.clone(),
        l_tolerance: deck.l_tolerance.clone(),
    };
    compare(&ext, reference, &cmp_opts)
}
