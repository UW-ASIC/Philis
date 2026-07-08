//! Cell definition API for analog P&R.
//!
//! Users implement [`CellGenerator`] to define custom cell layouts.
//! Built-in generators (MOSFET, resistor, etc.) implement it too —
//! one trait for everything.
//!
//! Geometry lands in [`gdsverify::GeometryStore`] (SoA, DRC-ready).
//! The [`CellBuilder`] wraps that store with an ergonomic drawing API
//! and tracks pins/metadata the geometry store doesn't carry.

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

pub use gdsverify::{Bbox, GeometryStore, LayerId, PolyId};
pub use gdsverify::{Deck, LayerTable};

// ---------------------------------------------------------------------------
//  Device classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    Nmos,
    Pmos,
    Ncap,
    Pcap,
    Res,
    Cap,
    Diode,
    Bjt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum MatchingTier {
    #[default]
    None,
    Minimal,
    Moderate,
    Exceptional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PatternType {
    #[default]
    Single,
    Interdig,
    Cc1d,
    Cc2d,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatchingType {
    Mirror,
    Cross,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    In,
    Out,
    #[default]
    InOut,
}

// ---------------------------------------------------------------------------
//  Orientation (placement transforms)
// ---------------------------------------------------------------------------

/// Manhattan placement orientation for sub-cell instances.
///
/// Analog common-centroid layout uses `MX` (mirror-X) to flip alternate
/// devices, cancelling linear gradients across the die.
// ponytail: R90/R270 add when RF spiral inductors need it
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Orientation {
    /// No transform.
    #[default]
    R0,
    /// 180° rotation.
    R180,
    /// Mirror across vertical axis (flip left-right).
    MX,
    /// Mirror across horizontal axis (flip top-bottom).
    MY,
}

impl Orientation {
    /// Transform a point within a cell of size `(cw, ch)`.
    #[inline]
    pub fn transform(self, x: i32, y: i32, cw: i32, ch: i32) -> (i32, i32) {
        match self {
            Self::R0 => (x, y),
            Self::R180 => (cw - x, ch - y),
            Self::MX => (cw - x, y),
            Self::MY => (x, ch - y),
        }
    }

    /// Transform a rectangle `(x, y, w, h)` within a cell of size `(cw, ch)`.
    /// Returns `(new_x, new_y, new_w, new_h)`.
    pub fn transform_rect(self, x: i32, y: i32, w: i32, h: i32, cw: i32, ch: i32) -> (i32, i32, i32, i32) {
        let corners = [
            self.transform(x, y, cw, ch),
            self.transform(x + w, y, cw, ch),
            self.transform(x + w, y + h, cw, ch),
            self.transform(x, y + h, cw, ch),
        ];
        let min_x = corners.iter().map(|c| c.0).min().unwrap_or(0);
        let min_y = corners.iter().map(|c| c.1).min().unwrap_or(0);
        let max_x = corners.iter().map(|c| c.0).max().unwrap_or(0);
        let max_y = corners.iter().map(|c| c.1).max().unwrap_or(0);
        (min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

// ---------------------------------------------------------------------------
//  Pin / port types
// ---------------------------------------------------------------------------

/// Pin access rectangle: name + layer + bbox.
/// Tracked separately from `GeometryStore` because the store has no
/// concept of named electrical pins.
#[derive(Debug, Clone)]
pub struct PinAccess {
    pub name: String,
    pub layer: LayerId,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Port declaration: name + direction. Declared before generation so
/// the pipeline can verify net connectivity.
#[derive(Debug, Clone)]
pub struct PortDef {
    pub name: String,
    pub direction: Direction,
}

impl PortDef {
    pub fn inout(name: impl Into<String>) -> Self {
        Self { name: name.into(), direction: Direction::InOut }
    }
    pub fn input(name: impl Into<String>) -> Self {
        Self { name: name.into(), direction: Direction::In }
    }
    pub fn output(name: impl Into<String>) -> Self {
        Self { name: name.into(), direction: Direction::Out }
    }
}

// ---------------------------------------------------------------------------
//  Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum CellError {
    #[error("empty device list")]
    EmptyDevices,
    #[error("PDK missing analog parameters")]
    MissingAnalogParams,
    #[error("unknown layer `{name}`")]
    UnknownLayer { name: String },
    #[error("invalid dimensions: {reason}")]
    InvalidDimensions { reason: String },
    #[error("no layout of `{model}` meets tier {tier:?}")]
    NoFeasibleVariant { model: String, tier: MatchingTier },
    #[error("device `{model}` has invalid rules: {reason}")]
    InvalidRules { model: String, reason: String },
    #[error("unsupported: {reason}")]
    Unsupported { reason: &'static str },
}

// ---------------------------------------------------------------------------
//  CellGenerator trait
// ---------------------------------------------------------------------------

/// Anything that produces cell geometry.
///
/// Built-in generators (MOSFET, resistor, ...) and user-defined custom
/// cells both implement this. Construction captures config; `generate`
/// is a pure geometry pass.
pub trait CellGenerator: Send + Sync {
    /// Declare ports before generation. Default: none (derived from pins).
    fn ports(&self) -> Vec<PortDef> { Vec::new() }

    /// Draw geometry into the builder.
    fn generate(&self, b: &mut CellBuilder) -> Result<(), CellError>;
}

/// Closures are generators.
impl<F> CellGenerator for F
where
    F: Fn(&mut CellBuilder) -> Result<(), CellError> + Send + Sync,
{
    fn generate(&self, b: &mut CellBuilder) -> Result<(), CellError> {
        (self)(b)
    }
}

// ---------------------------------------------------------------------------
//  Cell metadata
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct CellMeta {
    pub device_type: Option<DeviceType>,
    pub pattern: PatternType,
    pub tier: MatchingTier,
    pub matching_group: Option<u32>,
    pub w_eff: i32,
    pub l_eff: i32,
    pub finger_count: u16,
    pub finger_width: i32,
}

// ---------------------------------------------------------------------------
//  Cell output
// ---------------------------------------------------------------------------

/// Raw output of cell generation. The `cells` crate splits this into
/// hot (placement) and cold (signoff) structs.
pub struct CellOutput {
    pub store: GeometryStore,
    pub pins: Vec<PinAccess>,
    pub ports: Vec<PortDef>,
    pub pin_map: HashMap<String, Vec<Bbox>>,
    pub bbox: Bbox,
    pub meta: CellMeta,
    pub group_id: u32,
}

impl CellOutput {
    /// Bitmask of PDK LayerIds present in this cell's geometry.
    pub fn layer_mask(&self) -> u64 {
        let mut mask = 0u64;
        for &lid in &self.store.poly_layer {
            if (lid as u32) < 64 { mask |= 1u64 << lid; }
        }
        mask
    }
}

impl std::fmt::Debug for CellOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellOutput")
            .field("polys", &self.store.poly_count())
            .field("pins", &self.pins.len())
            .field("bbox", &self.bbox)
            .field("meta", &self.meta)
            .finish()
    }
}

// ---------------------------------------------------------------------------
//  CellBuilder
// ---------------------------------------------------------------------------

/// Drawing surface for cell generators.
///
/// Wraps [`GeometryStore`] (SoA geometry) with named-layer resolution,
/// pin tracking, and PDK DRC lookups. After `generate()`, call
/// [`finish`](CellBuilder::finish) to extract [`CellOutput`].
pub struct CellBuilder<'a> {
    layers: &'a LayerTable,
    deck: &'a Deck,
    tier: MatchingTier,
    group_id: u32,
    store: GeometryStore,
    pins: Vec<PinAccess>,
    ports: Vec<PortDef>,
    meta: CellMeta,
}

impl<'a> CellBuilder<'a> {
    pub fn new(deck: &'a Deck, tier: MatchingTier, group_id: u32) -> Self {
        Self {
            layers: &deck.layers,
            deck,
            tier,
            group_id,
            store: GeometryStore::new(),
            pins: Vec::new(),
            ports: Vec::new(),
            meta: CellMeta { tier, ..Default::default() },
        }
    }

    // ---- Drawing ----

    /// Draw rectangle on named layer. Resolves name -> `LayerId` via the deck.
    pub fn rect(&mut self, layer: &str, x: i32, y: i32, w: i32, h: i32) -> Result<PolyId, CellError> {
        let lid = self.resolve(layer)?;
        Ok(self.store.add_rect(lid, x, y, w, h))
    }

    /// Draw rectangle on a resolved `LayerId` (skip name lookup).
    pub fn rect_id(&mut self, layer: LayerId, x: i32, y: i32, w: i32, h: i32) -> PolyId {
        self.store.add_rect(layer, x, y, w, h)
    }

    /// Draw polygon on named layer.
    pub fn polygon(&mut self, layer: &str, pts: &[(i32, i32)]) -> Result<PolyId, CellError> {
        let lid = self.resolve(layer)?;
        Ok(self.store.add_polygon(lid, pts))
    }

    /// Declare a pin access point.
    pub fn pin(&mut self, name: &str, layer: &str, x: i32, y: i32, w: i32, h: i32) -> Result<(), CellError> {
        let lid = self.resolve(layer)?;
        self.pins.push(PinAccess {
            name: name.into(),
            layer: lid,
            x, y, w, h,
        });
        Ok(())
    }

    /// Declare a pin on a resolved `LayerId`.
    pub fn pin_id(&mut self, name: &str, layer: LayerId, x: i32, y: i32, w: i32, h: i32) {
        self.pins.push(PinAccess {
            name: name.into(),
            layer, x, y, w, h,
        });
    }

    /// Declare a port (name + direction).
    pub fn port(&mut self, name: impl Into<String>, dir: Direction) {
        self.ports.push(PortDef { name: name.into(), direction: dir });
    }

    /// Bulk-add pins.
    pub fn pins_bulk(&mut self, iter: impl IntoIterator<Item = PinAccess>) {
        self.pins.extend(iter);
    }

    /// Place sub-cell at `(dx, dy)` with the given [`Orientation`],
    /// merging its geometry and pins into this cell.
    pub fn instance(
        &mut self,
        _name: &str,
        cell: &dyn CellGenerator,
        dx: i32,
        dy: i32,
        orient: Orientation,
    ) -> Result<(), CellError> {
        let mut sub = CellBuilder::new(self.deck, self.tier, self.group_id);
        cell.generate(&mut sub)?;
        let bb = sub.compute_bbox();
        let cw = bb.width();
        let ch = bb.height();
        let ox = bb.xmin;
        let oy = bb.ymin;
        // merge geometry: transform + offset every polygon
        for pid in 0..sub.store.poly_count() {
            let (s, e) = sub.store.poly_range(PolyId(pid as u32));
            let layer = sub.store.poly_layer[pid];
            let pts: Vec<(i32, i32)> = (s..e)
                .map(|i| {
                    let (tx, ty) = orient.transform(
                        sub.store.verts_x[i] - ox,
                        sub.store.verts_y[i] - oy,
                        cw, ch,
                    );
                    (tx + dx, ty + dy)
                })
                .collect();
            self.store.add_polygon(layer, &pts);
        }
        // merge pins: transform rect then offset
        for p in sub.pins {
            let (rx, ry, rw, rh) = orient.transform_rect(
                p.x - ox, p.y - oy, p.w, p.h, cw, ch,
            );
            self.pins.push(PinAccess {
                name: p.name,
                layer: p.layer,
                x: rx + dx,
                y: ry + dy,
                w: rw,
                h: rh,
            });
        }
        Ok(())
    }

    // ---- Metadata setters ----

    pub fn set_device_type(&mut self, dt: DeviceType) {
        self.meta.device_type = Some(dt);
    }
    pub fn set_pattern(&mut self, p: PatternType) {
        self.meta.pattern = p;
    }
    pub fn set_electrical(&mut self, w_eff: i32, l_eff: i32, nf: u16, fw: i32) {
        self.meta.w_eff = w_eff;
        self.meta.l_eff = l_eff;
        self.meta.finger_count = nf;
        self.meta.finger_width = fw;
    }
    pub fn set_matching_group(&mut self, g: u32) {
        self.meta.matching_group = Some(g);
    }

    // ---- PDK access ----

    pub fn deck(&self) -> &Deck { self.deck }
    pub fn layers(&self) -> &LayerTable { self.layers }
    pub fn tier(&self) -> MatchingTier { self.tier }

    /// Resolve layer name -> id. Cached in `LayerTable`.
    pub fn resolve(&self, name: &str) -> Result<LayerId, CellError> {
        self.layers.id(name).ok_or_else(|| CellError::UnknownLayer {
            name: name.into(),
        })
    }

    /// Look up a DRC rule value (nm) by rule id.
    pub fn drc(&self, rule_id: &str) -> Option<i32> {
        self.deck.drc_rules.iter().find_map(|r| {
            if r.id() != rule_id { return None; }
            match r {
                gdsverify::DrcRuleParam::MinWidth { min, .. }
                | gdsverify::DrcRuleParam::MinSpacing { min, .. }
                | gdsverify::DrcRuleParam::MinEnclosure { min, .. }
                | gdsverify::DrcRuleParam::MinExtension { min, .. }
                | gdsverify::DrcRuleParam::Notch { min, .. }
                | gdsverify::DrcRuleParam::MinEdgeLength { min, .. }
                | gdsverify::DrcRuleParam::CornerToCorner { min, .. }
                | gdsverify::DrcRuleParam::Overlap { min, .. } => Some(*min),
                gdsverify::DrcRuleParam::MinSpacingDiff { min, .. } => Some(*min),
                gdsverify::DrcRuleParam::MaxWidth { max, .. } => Some(*max),
                _ => None,
            }
        })
    }

    /// Snap value to manufacturing grid (from deck).
    pub fn snap(&self, v: i32) -> i32 {
        snap_to_grid(v, self.grid())
    }

    pub fn grid(&self) -> i32 {
        self.deck.drc_rules.iter().find_map(|r| match r {
            gdsverify::DrcRuleParam::OffGrid { grid, .. } => Some(*grid),
            _ => None,
        }).unwrap_or(1)
    }

    /// Direct access to underlying store (for built-in generators doing bulk work).
    pub fn store_mut(&mut self) -> &mut GeometryStore { &mut self.store }
    pub fn store(&self) -> &GeometryStore { &self.store }

    // ---- Finalize ----

    /// Consume builder, produce [`CellOutput`].
    pub fn finish(self) -> CellOutput {
        let bbox = self.compute_bbox();
        let pin_map = self.build_pin_map();
        CellOutput {
            store: self.store,
            pins: self.pins,
            ports: self.ports,
            pin_map,
            bbox,
            meta: self.meta,
            group_id: self.group_id,
        }
    }

    pub fn compute_bbox(&self) -> Bbox {
        if self.store.poly_count() == 0 {
            return Bbox::empty();
        }
        let mut bb = Bbox::empty();
        for b in &self.store.poly_bbox {
            bb.include(b.xmin, b.ymin);
            bb.include(b.xmax, b.ymax);
        }
        bb
    }

    fn build_pin_map(&self) -> HashMap<String, Vec<Bbox>> {
        let mut map: HashMap<String, Vec<Bbox>> = HashMap::new();
        for p in &self.pins {
            map.entry(p.name.clone()).or_default().push(Bbox {
                xmin: p.x,
                ymin: p.y,
                xmax: p.x + p.w,
                ymax: p.y + p.h,
            });
        }
        map
    }
}

// ---------------------------------------------------------------------------
//  Utility
// ---------------------------------------------------------------------------

pub fn snap_to_grid(value: i32, grid: i32) -> i32 {
    if grid <= 0 { return value; }
    let rem = value % grid;
    if rem == 0 { return value; }
    let abs_rem = rem.abs();
    let half = (grid + 1) / 2;
    if abs_rem >= half {
        value + (grid - abs_rem) * value.signum()
    } else {
        value - rem
    }
}

// ---------------------------------------------------------------------------
//  Verification: DRC / LVS / PEX on generated cells
// ---------------------------------------------------------------------------

pub use gdsverify::{DrcReport, Violation, run_drc};
pub use gdsverify::{PexReport, Parasitic, run_pex};
pub use gdsverify::{LvsResult, RefNetlist, RefDevice, DeviceFlavor, DeviceKind, run_lvs};

/// Verification results for a generated cell.
pub struct VerifyReport {
    pub drc: DrcReport,
    pub drc_clean: bool,
    pub pex: PexReport,
}

impl std::fmt::Debug for VerifyReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifyReport")
            .field("drc_clean", &self.drc_clean)
            .field("violations", &self.drc.violations.len())
            .field("parasitics", &self.pex.parasitics.len())
            .finish()
    }
}

/// Run DRC and PEX on a generated [`CellOutput`].
///
/// Returns a [`VerifyReport`] with violations and parasitics.
/// LVS requires a reference netlist — use [`verify_lvs`] separately.
pub fn verify(out: &CellOutput, deck: &Deck) -> VerifyReport {
    let drc = run_drc(&out.store, deck);
    let drc_clean = drc.violations.is_empty();
    let pex = run_pex(&out.store, deck);
    VerifyReport { drc, drc_clean, pex }
}

/// Run LVS: compare generated cell geometry against a reference netlist.
pub fn verify_lvs(out: &CellOutput, deck: &Deck, reference: &RefNetlist) -> LvsResult {
    run_lvs(&out.store, deck, reference)
}

// ---------------------------------------------------------------------------
//  Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_deck() -> Deck {
        let json = r#"{
            "layers": {
                "diff":  {"layer": 65, "datatype": 20},
                "poly":  {"layer": 66, "datatype": 20},
                "li":    {"layer": 67, "datatype": 20},
                "met1":  {"layer": 68, "datatype": 20},
                "nwell": {"layer": 64, "datatype": 20}
            },
            "drc": {
                "min_width": {"layer": "met1", "min": 140},
                "min_spacing": {"layer": "met1", "min": 140},
                "off_grid": {"grid": 5}
            }
        }"#;
        gdsverify::Deck::from_json(json).expect("test deck")
    }

    struct Inverter {
        w_n: i32,
        w_p: i32,
        l: i32,
    }

    impl CellGenerator for Inverter {
        fn ports(&self) -> Vec<PortDef> {
            vec![
                PortDef::inout("A"),
                PortDef::output("Y"),
                PortDef::inout("VDD"),
                PortDef::inout("VSS"),
            ]
        }

        fn generate(&self, b: &mut CellBuilder) -> Result<(), CellError> {
            b.set_device_type(DeviceType::Nmos);
            let gap = 500;
            b.rect("diff", 0, 0, self.w_n, self.l)?;
            b.rect("diff", 0, self.l + gap, self.w_p, self.l)?;
            let gx = self.w_n / 2 - self.l / 2;
            b.rect("poly", gx, -100, self.l, 2 * self.l + gap + 200)?;

            let ps = 170;
            b.pin("A", "met1", b.snap(gx), -100, ps, ps)?;
            b.pin("Y", "met1", b.snap(self.w_n), self.l / 2, ps, ps)?;
            b.pin("VSS", "met1", 0, 0, ps, ps)?;
            b.pin("VDD", "met1", 0, self.l + gap, ps, ps)?;
            Ok(())
        }
    }

    #[test]
    fn custom_cell_roundtrip() {
        let deck = test_deck();
        let inv = Inverter { w_n: 420, w_p: 840, l: 150 };
        assert_eq!(inv.ports().len(), 4);

        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        inv.generate(&mut b).expect("generate");
        let out = b.finish();

        assert!(out.bbox.width() > 0, "bbox width");
        assert!(out.bbox.height() > 0, "bbox height");
        assert_eq!(out.pins.len(), 4, "4 pins");
        assert_eq!(out.pin_map.len(), 4, "4 nets in pin_map");
        assert!(out.store.poly_count() > 0, "geometry in store");
    }

    #[test]
    fn closure_generator() {
        let deck = test_deck();
        let gen = |b: &mut CellBuilder| -> Result<(), CellError> {
            b.rect("met1", 0, 0, 100, 100)?;
            b.pin("A", "met1", 0, 0, 100, 100)?;
            Ok(())
        };
        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        gen.generate(&mut b).expect("generate");
        let out = b.finish();
        assert_eq!(out.store.poly_count(), 1);
        assert_eq!(out.pins.len(), 1);
    }

    #[test]
    fn hierarchical_instance() {
        let deck = test_deck();
        let sub_cell = |b: &mut CellBuilder| -> Result<(), CellError> {
            b.rect("met1", 0, 0, 100, 100)?;
            b.pin("A", "met1", 0, 0, 100, 100)?;
            Ok(())
        };
        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        b.instance("i0", &sub_cell, 0, 0, Orientation::R0).expect("i0");
        b.instance("i1", &sub_cell, 200, 0, Orientation::R0).expect("i1");
        let out = b.finish();
        assert_eq!(out.store.poly_count(), 2, "two sub-cell rects");
        assert_eq!(out.pins.len(), 2, "two pins");
        assert_eq!(out.pins[1].x, 200, "second instance offset");
    }

    #[test]
    fn instance_mirror_x() {
        let deck = test_deck();
        let sub = |b: &mut CellBuilder| -> Result<(), CellError> {
            b.rect("met1", 0, 0, 200, 100)?;
            b.pin("L", "met1", 0, 0, 50, 100)?;   // left pin
            b.pin("R", "met1", 150, 0, 50, 100)?;  // right pin
            Ok(())
        };
        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        b.instance("mx", &sub, 0, 0, Orientation::MX).expect("mx");
        let out = b.finish();
        // MX flips left-right: left pin (x=0) → right side (x=150)
        let l_pin = out.pins.iter().find(|p| p.name == "L").unwrap();
        let r_pin = out.pins.iter().find(|p| p.name == "R").unwrap();
        assert_eq!(l_pin.x, 150, "L pin flipped to right");
        assert_eq!(r_pin.x, 0, "R pin flipped to left");
    }

    #[test]
    fn unknown_layer_errors() {
        let deck = test_deck();
        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        let err = b.rect("nonexistent", 0, 0, 10, 10);
        assert!(err.is_err());
    }

    #[test]
    fn snap_to_grid_basics() {
        assert_eq!(snap_to_grid(100, 5), 100);
        assert_eq!(snap_to_grid(102, 5), 100);
        assert_eq!(snap_to_grid(103, 5), 105);
        assert_eq!(snap_to_grid(0, 5), 0);
        assert_eq!(snap_to_grid(42, 0), 42);
    }

    #[test]
    fn drc_lookup() {
        let deck = test_deck();
        let b = CellBuilder::new(&deck, MatchingTier::None, 0);
        assert_eq!(b.drc("min_width"), Some(140));
        assert_eq!(b.drc("min_spacing"), Some(140));
        assert_eq!(b.drc("nonexistent"), None);
        assert_eq!(b.grid(), 5);
    }
}
