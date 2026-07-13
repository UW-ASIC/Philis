//! Device recognition stubs for Wave 4 (L4.3).
//!
//! MOS recognition lives in extract.rs (pick_type_and_flavor + the gate-split
//! loop). BJT and two-terminal extraction also live there. This module holds
//! the planned device families that don't exist yet, so L4.3 implementors have
//! a place to land code without touching the connectivity pipeline.

use crate::geometry::{GeometryStore, PolyId};
use crate::params::Deck;
use super::types::*;

// --- L4.3 planned device records ---

/// Extracted resistor with geometry-derived properties.
#[derive(Debug, Clone)]
pub struct ResistorDevice {
    pub terminal_a: u32,
    pub terminal_b: u32,
    pub body_polygon: PolyId,
    pub sheet_rho: f64,
    pub length: i32,
    pub width: i32,
    pub segments: u32,
}

/// Extracted capacitor with geometry-derived properties.
#[derive(Debug, Clone)]
pub struct CapacitorDevice {
    pub terminal_top: u32,
    pub terminal_bot: u32,
    pub area: i64,
    pub perimeter: i64,
}

/// Extracted diode.
#[derive(Debug, Clone)]
pub struct DiodeDevice {
    pub anode: u32,
    pub cathode: u32,
    pub area: i64,
    pub perimeter: i64,
}

// --- L4.3 stubs ---

// ponytail: stubs for Wave 4 L4.3 device families. Each returns empty until
// the foundry device deck (EXT-DEVICE gate) is available and recognition rules
// are implemented. The extract pipeline calls these after connectivity; they
// receive net_of_poly so terminals are net-resolved.

pub fn extract_resistors(
    _store: &GeometryStore,
    _deck: &Deck,
    _net_of_poly: &[u32],
) -> Vec<ResistorDevice> {
    // TODO(L4.3): R body/marker/terminal geometry, sheet resistance, segmentation
    Vec::new()
}

pub fn extract_capacitors(
    _store: &GeometryStore,
    _deck: &Deck,
    _net_of_poly: &[u32],
) -> Vec<CapacitorDevice> {
    // TODO(L4.3): C top/bottom overlap area, fringe perimeter
    Vec::new()
}

pub fn extract_diodes(
    _store: &GeometryStore,
    _deck: &Deck,
    _net_of_poly: &[u32],
) -> Vec<DiodeDevice> {
    // TODO(L4.3): diode anode/cathode/implant recognition
    Vec::new()
}

// TODO(L4.3): varactor/MOSCAP, inductor, custom device families
// TODO(L4.3): MOS AD/AS/PD/PS properties (source/drain area/perimeter)
// TODO(L4.3): model/class binding from deck device rules
