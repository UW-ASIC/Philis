//! Cell generation for analog P&R.
//!
//! Backend-owned cell construction and built-in analog generators.
//!
//! The frontend `substrate3` crate re-exports this crate's construction
//! contract for user-defined macros. [`CellRegistry`] holds those custom
//! generators; built-ins live in [`generators`].

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod api;
pub mod device;
#[cfg(feature = "test-fixtures")]
#[doc(hidden)]
pub mod fixtures;
pub mod generators;
pub mod netlist;
pub mod pdk;
#[cfg(test)]
pub(crate) mod test_util;

pub use api::{
    run_drc, run_lvs, run_pex, snap_to_grid, verify, verify_lvs, Bbox, CellBuilder, CellContext,
    CellError, CellGenerator, CellMeta, CellOutput, DeviceFlavor, DeviceKind, DeviceType,
    Direction, DrcReport, GeometryStore, LayerId, LvsResult, MatchingTier, MatchingType,
    Orientation, Parasitic, PatternType, PexReport, PinAccess, PolyId, PortDef, RefDevice,
    RefNetlist, VerifyReport, Violation,
};
pub use api::{Deck, LayerTable};

// ---------------------------------------------------------------------------
//  CellRegistry
// ---------------------------------------------------------------------------

/// Custom cell entry: generator + metadata the pipeline needs.
pub struct CustomCellEntry {
    pub name: String,
    pub generator: Box<dyn CellGenerator>,
    pub instances: Vec<String>,
    pub device_type: DeviceType,
    pub matching_tier: MatchingTier,
    pub matching_group: Option<u32>,
}

/// Registry of custom cell generators.
pub struct CellRegistry {
    custom: Vec<CustomCellEntry>,
}

impl CellRegistry {
    pub fn new() -> Self {
        Self { custom: Vec::new() }
    }

    pub fn add_custom(&mut self, entry: CustomCellEntry) {
        self.custom.push(entry);
    }

    pub fn entries(&self) -> &[CustomCellEntry] {
        &self.custom
    }

    pub fn consumed_instances(&self) -> impl Iterator<Item = &str> {
        self.custom
            .iter()
            .flat_map(|e| e.instances.iter().map(String::as_str))
    }
}

impl Default for CellRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
//  Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_deck() -> Deck {
        test_util::deck_from_layers(&[("diff", 65, 20), ("poly", 66, 20), ("met1", 68, 20)])
    }

    #[test]
    fn registry_custom_cell() {
        let deck = test_deck();
        let gen = |b: &mut CellBuilder| -> Result<(), CellError> {
            b.rect("diff", 0, 0, 420, 150)?;
            b.rect("poly", 180, -20, 150, 190)?;
            b.pin("G", "met1", 180, -20, 170, 170)?;
            b.pin("S", "met1", 0, 50, 170, 170)?;
            b.pin("D", "met1", 420, 50, 170, 170)?;
            b.set_electrical(420, 150, 1, 420);
            Ok(())
        };

        let mut reg = CellRegistry::new();
        reg.add_custom(CustomCellEntry {
            name: "nmos_pair".into(),
            generator: Box::new(gen),
            instances: vec!["M1".into(), "M2".into()],
            device_type: DeviceType::Nmos,
            matching_tier: MatchingTier::Moderate,
            matching_group: Some(0),
        });

        let consumed: Vec<&str> = reg.consumed_instances().collect();
        assert_eq!(consumed, &["M1", "M2"]);

        // Generate and verify the custom cell produces geometry.
        let mut b = CellBuilder::new(&deck, MatchingTier::Moderate, 0);
        b.set_device_type(DeviceType::Nmos);
        b.set_matching_group(0);
        reg.entries()[0].generator.generate(&mut b).expect("gen ok");
        let out = b.finish();
        assert!(out.bbox.width() > 0);
        assert_eq!(out.pin_map.len(), 3, "G/S/D");
        assert!(out.store.poly_count() > 0);
    }
}
