//! Shared test utilities for generator DRC/LVS verification.

use substrate3::{CellBuilder, CellGenerator, Deck, MatchingTier, verify};
use gdsverify::schema::{LvsSchema, VerifySchema};

/// Build a test deck from layer definitions (name, GDS layer, datatype).
pub fn deck_from_layers(layers: &[(&str, i32, i32)]) -> Deck {
    let schema = VerifySchema {
        layers: layers
            .iter()
            .map(|&(name, layer, dt)| (name.to_string(), (layer, dt)))
            .collect(),
        drc_rules: vec![],
        pex: Default::default(),
        lvs: LvsSchema::default(),
        connectivity: Default::default(),
        devices: Default::default(),
    };
    Deck::from_schema(schema).expect("test deck")
}

/// Load the real sky130 deck for DRC-aware tests.
pub fn sky130_deck() -> Deck {
    let path = format!("{}/../../pdks/sky130.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("load {path}: {e}"));
    Deck::from_json(&text).expect("parse sky130 deck")
}

/// Load the sky130 cell-construction params from the same deck file.
pub fn sky130_pdk() -> crate::pdk::Pdk {
    let path = format!("{}/../../pdks/sky130.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("load {path}: {e}"));
    crate::pdk::Pdk::from_json(&text).expect("parse sky130 cell params")
}

/// Generate a cell and run DRC, asserting clean.
pub fn generate_and_verify(
    gen: &dyn CellGenerator,
    deck: &Deck,
    tier: MatchingTier,
) -> substrate3::CellOutput {
    let mut b = CellBuilder::new(deck, tier, 0);
    gen.generate(&mut b).expect("generate");
    let out = b.finish();
    let report = verify(&out, deck);
    let real_violations: Vec<_> = report
        .drc
        .violations
        .iter()
        .filter(|v| v.kind != "min_density" && v.kind != "max_density")
        .collect();
    if !real_violations.is_empty() {
        let viols: Vec<_> = real_violations
            .iter()
            .map(|v| {
                format!(
                    "  {} [{}] on {}: measured={} limit={} @ ({},{})",
                    v.rule_id, v.kind, v.layer, v.measured, v.limit, v.x, v.y
                )
            })
            .collect();
        panic!(
            "DRC violations ({}):\n{}",
            real_violations.len(),
            viols.join("\n"),
        );
    }
    assert!(out.bbox.width() > 0, "bbox width > 0");
    assert!(!out.pins.is_empty(), "has pins");
    out
}

