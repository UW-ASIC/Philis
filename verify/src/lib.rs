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

pub mod backend;
pub mod core;
pub mod rule;
pub mod io;
pub mod drc;
pub mod lvs;
pub mod pex;
pub mod erc;
pub mod signoff;

// Old top-level module paths, re-exported so call sites keep compiling.
pub use crate::core::{geometry, hierarchy_index};
// Old top-level module paths, re-exported so call sites keep compiling:
// gds + gds_lossless merged into io::read::gds; params + schema merged into
// io::schema; oasis moved under io::read.
pub use crate::io::read;
pub use crate::io::read::gds;
pub use crate::io::read::gds as gds_lossless;
pub use crate::io::read::oasis;
pub use crate::io::schema;
pub use crate::io::schema as params;
pub use crate::backend as gpu;

pub use geometry::{Bbox, Edge, GeometryStore, LayerId, PolyId};
pub use params::{Deck, DrcRuleParam, LayerDef, LayerTable};
pub use schema::{DrcRuleSchema, LvsSchema, VerifySchema};
pub use drc::{run_drc, run_drc_backend, run_drc_backend_strict, run_drc_no_density, same_shape_gap_fills, DrcReport, Violation};
pub use pex::{
    run_pex, run_pex_by_net, run_pex_by_net_checked, NetParasitics, Parasitic, PexReport,
};
pub use lvs::{compare, extract_netlist, extract_netlist_opts, reduce_netlist,
              to_spice, CompareOpts, DeviceFlavor, DeviceKind, Device,
              ExtractOpts, ExtractedNetlist, LvsResult, PortMap, RefDevice, RefNetlist,
              RefTwoTerminal, SpiceOpts, TwoTerminalKind, TwoTerminalDevice};
pub use lvs::netlist::{
    leaf_subcircuit_to_ref_netlist, parse_engineering_number, parse_netlist,
    parse_netlist_with_includes, BjtModelBinding, EngineeringNumber, EngineeringSuffix,
    IncludeDecl, InstanceKind, MosModelBinding, NetlistAst, NetlistError, NetlistErrorKind,
    ModelDecl, ModelPrimitive, NetlistInstance, ParameterDecl, ParameterExpr, RefConversionError,
    RefConversionErrorKind, RefConversionOptions, ResolvedInclude, SourceSpan, Subcircuit,
};
pub use lvs::production::{
    compare_production, BjtDeviceRecord, DetailedExtractedNetlist, DetailedNetlist,
    DetailedRefNetlist, DeviceIdentity, DeviceMapping, HierarchyPath, LegacyRefDeviceBuilder,
    MosDeviceRecord, NetIdentity, NetMapping, NumericTolerance, OpenCandidate, PortDirection,
    ProductionCompareOptions, ProductionLvsResult, ProductionLvsStatus, ProductionMismatch,
    PropertyDelta, PropertyUnit, SoftConnection, TerminalConnection, TopologyConflictKind,
    TopologyWitness, TwoTerminalRecord, TypedProperty, UnresolvedTerminal,
};
pub use lvs::binding::{
    bind_reference_hierarchy, evaluate_parameter_expression, BindingError, BindingErrorKind,
    BlackBoxSpec, BoundReferenceCell, BoundReferenceHierarchy, BoundReferenceInstance,
    ConfiguredModel, ParameterEnvironment, ReferenceBindingOptions,
};
pub use lvs::detailed_extract::{
    extract_detailed_netlist, DetailedExtractionError, DetailedExtractionErrorKind,
    DetailedExtractionOptions, NamedSoftConnection,
};
pub use lvs::hier_production::{
    compare_hierarchical_production, HierArray, HierCellComparison, HierFlattenPolicy,
    HierLayout, HierLayoutCell, HierLayoutInstance, HierLvsCache, HierProductionOptions,
    HierProductionResult, HierTransform,
};
pub use lvs::gds_adapter::{
    adapt_gds_hierarchy_to_lvs, export_w3_drc_hierarchy_context, GdsAdapterObjectKind,
    GdsBlackBoxAdapterSpec, GdsDrcHierarchyContext, GdsHierarchyAdapterError,
    GdsHierarchyAdapterErrorKind, GdsHierarchyAdapterOptions, GdsHierarchyAdapterResult,
    GdsHierarchyProvenance, GdsObjectProvenance, GdsPhysicalCorrelationStatus,
    GdsTextEvidenceRule, GDS_ADAPTER_MAX_STACK_SAFE_DEPTH,
};
pub use erc::{run_erc, ErcReport, ErcViolation, MultipleDriverCheck, TieHighLowCheck};
pub use signoff::*;
pub use gds::{read_gds, read_gds_checked, GdsLayout, GdsUnits, GdsUnmappedLayer};
pub use gds_lossless::{
    flatten_gds_library, read_gds_library, stroke_path, write_gds_library,
    GdsArrayReference, GdsBoundary, GdsBoxElement, GdsElement, GdsElementMeta,
    GdsEnvelope, GdsFlattenOptions, GdsGeometryPolicy, GdsLibrary, GdsNode, GdsPath, GdsProperty,
    GdsRawRecord, GdsReadMode, GdsReference, GdsStructure, GdsText,
    GdsTransform, GdsUnsupportedElement, LayoutError, LayoutErrorKind,
};
pub use hierarchy_index::{
    GdsLayerIdentity, HierarchyCandidate, HierarchyIndexOptions, HierarchySpatialIndex,
    IndexedShapeKind, InstancePathEntry, TileCandidates, TileGrid, TileId,
    VerificationTile,
};
pub use oasis::{
    read_oasis, write_oasis, OasisCapabilities, OasisError, OasisErrorKind,
    OASIS_CAPABILITIES,
};
pub use backend::{available_backends, gpu_ready, Backend};
pub use rule::{run_rules, DynRule, Rule, VerifyCheck};

// GDS REAL8 values are approximate, so compare units with a tight numerical
// tolerance rather than bit equality.  The tolerance is deliberately far below
// any meaningful process-grid difference: 1e-9 relative or 1e-12 nm absolute.
const GDS_DBU_REL_TOLERANCE: f64 = 1.0e-9;
const GDS_DBU_ABS_TOLERANCE_NM: f64 = 1.0e-12;

fn validate_gds_database_units(units: Option<GdsUnits>, deck_dbu_nm: f64) -> Result<(), String> {
    if !deck_dbu_nm.is_finite() || deck_dbu_nm <= 0.0 {
        return Err(format!(
            "deck database unit must be finite and positive, got {deck_dbu_nm} nm"
        ));
    }
    let units = units.ok_or_else(|| {
        "GDS has no UNITS record; deck-aware loading cannot establish coordinate units".to_string()
    })?;
    let gds_dbu_nm = units.database_unit_nm();
    if !gds_dbu_nm.is_finite() || gds_dbu_nm <= 0.0 {
        return Err(format!(
            "GDS database unit must be finite and positive, got {gds_dbu_nm} nm"
        ));
    }
    let tolerance = GDS_DBU_ABS_TOLERANCE_NM.max(
        GDS_DBU_REL_TOLERANCE * deck_dbu_nm.abs().max(gds_dbu_nm.abs()),
    );
    if (gds_dbu_nm - deck_dbu_nm).abs() > tolerance {
        return Err(format!(
            "GDS database unit {gds_dbu_nm} nm does not match deck database unit \
             {deck_dbu_nm} nm (tolerance {tolerance} nm); coordinates were not rescaled"
        ));
    }
    Ok(())
}

/// Convenience: read a GDS file into per-cell stores using the deck's layer table.
pub fn load_gds(path: &str, deck: &Deck) -> Result<GdsLayout, String> {
    load_gds_with_policy(path, deck, GdsLoadPolicy::LegacyDrcCompatibility)
}

/// Stream policy for deck-aware file loading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GdsLoadPolicy {
    /// Preserves closed invalid polygons so the always-on DRC polygon-validity
    /// check can report legacy/fuzz corpus defects.
    LegacyDrcCompatibility,
    /// Requires a complete GDS envelope and valid simple geometry. This is the
    /// required policy for signoff and deck qualification.
    StrictSignoff,
}

pub fn load_gds_strict(path: &str, deck: &Deck) -> Result<GdsLayout, String> {
    load_gds_with_policy(path, deck, GdsLoadPolicy::StrictSignoff)
}

pub fn load_gds_with_policy(
    path: &str,
    deck: &Deck,
    policy: GdsLoadPolicy,
) -> Result<GdsLayout, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let layout = match policy {
        GdsLoadPolicy::LegacyDrcCompatibility => read_gds(&bytes, &deck.layers)?,
        GdsLoadPolicy::StrictSignoff => read_gds_checked(
            &bytes,
            GdsReadMode::Strict,
            &deck.layers,
            &GdsFlattenOptions::default(),
        )?,
    };
    validate_gds_database_units(layout.units, deck.dbu_nm)?;
    Ok(layout)
}

/// Run LVS on an extracted store against a reference netlist. Returns extraction errors
/// as a failing LvsResult (matched=false).
pub fn run_lvs(store: &GeometryStore, deck: &Deck, reference: &RefNetlist) -> LvsResult {
    let opts = ExtractOpts { cut_required: deck.lvs_cut_required, ..Default::default() };
    let ext = match extract_netlist_opts(store, deck, &opts, backend::Backend::Cpu) {
        Ok(e) => e,
        Err(e) => return LvsResult {
            matched: false, reason: format!("extraction failed: {}", e),
            extracted_devices: 0, nmos: 0, pmos: 0,
            ambiguous_classes: 0, label_conflicts: Vec::new(),
            mismatches: Vec::new(), floating_nets: Vec::new(),
        },
    };
    let cmp_opts = CompareOpts {
        strict: deck.strict,
        w_tolerance: deck.w_tolerance.clone(),
        l_tolerance: deck.l_tolerance.clone(),
    };
    let mut result = compare(&ext, reference, &cmp_opts);
    if deck.fail_on_floating && !ext.floating_nets.is_empty() {
        result.matched = false;
        result.reason = format!("{} floating extracted net(s)", ext.floating_nets.len());
    }
    result
}

#[cfg(test)]
mod unit_contract_tests {
    use super::*;

    fn units(database_unit_nm: f64) -> GdsUnits {
        GdsUnits {
            user_units_per_database_unit: 1.0e-3,
            meters_per_database_unit: database_unit_nm * 1.0e-9,
        }
    }

    #[test]
    fn deck_aware_gds_units_fail_closed() {
        let missing = validate_gds_database_units(None, 1.0)
            .expect_err("missing GDS UNITS must not inherit the deck unit silently");
        assert!(missing.contains("no UNITS"), "{missing}");

        let mismatch = validate_gds_database_units(Some(units(2.0)), 1.0)
            .expect_err("mismatched coordinate units must not be silently rescaled");
        assert!(mismatch.contains("does not match"), "{mismatch}");

        validate_gds_database_units(Some(units(1.0 + 5.0e-10)), 1.0)
            .expect("REAL8 representation noise within the documented tolerance is accepted");
        assert!(validate_gds_database_units(Some(units(1.0 + 2.0e-9)), 1.0).is_err());
    }
}
