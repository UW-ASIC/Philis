//! # `cells` — automatic device geometry
//!
//! Turns a PDK primitive ([`analog::Device`]) into a drawn [`analog::Macro`]:
//! a MOSFET with `W/L/fingers`, a guard-ringed cap, a serpentine resistor.
//! **One generator per device family, one file each**, each carrying its own
//! layout theory (interdigitation, dummy devices, guard rings) in its rustdoc.
//!
//! Each generator **enumerates a variant space**, not one shape. A variant fixes
//! an [`analog::Pattern`], finger count, dummy count, etc. Both `enumerate` and
//! `draw` read the group's [`analog::Constraints`] — matching, symmetry,
//! unitization — to decide which variants are feasible and how to draw them; the
//! generator interprets constraints, it never privileges any one. It also never
//! *ranks* variants: the placer picks the winner during annealing (reshape moves).
//!
//! Everything here is **pure**: `(variant, group, constraints, pdk) -> Macro` is
//! deterministic to the byte, so a device's geometry is a table test with no
//! fixtures.

#![allow(dead_code)]

pub mod bjt;
pub mod builder;
pub mod capacitor;
pub mod diode;
pub mod inductor;
pub mod matching;
pub mod mosfet;
pub mod post_cell;
pub mod resistor;

pub use builder::{Builder, Instance};

use analog::Constraints;
use pnr_core::{DeviceGroup, Macro, Pin, Process};

/// Interdigitation / common-centroid style a generator lays a matched group out
/// in. **Internal to the generators** — it names how fingers/segments of a
/// matched group interleave, not a PDK fact. Not every family uses every
/// variant (a MOSFET has all four; a diode only `Single`/`Interdig`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pattern {
    /// One device, no interleaving.
    Single,
    /// Common-centroid, one dimension (ABBA finger order).
    Cc1d,
    /// Common-centroid, two dimensions (checkerboard).
    Cc2d,
    /// Simple interdigitation (ABAB).
    Interdig,
}

/// A device-family generator over its **variant space**.
///
/// Implementors are pure geometry-identity structs (see [`mosfet::Mosfet`]);
/// everything process-specific is read from `constraints`/`process` at build
/// time — via the PDK-agnostic [`Process`] seam, never a concrete PDK.
pub trait Cell: Clone {
    /// Every feasible variant for `group` under its `constraints` and the bound
    /// `process` — the set the placer chooses from. **Deterministic**, deduped by
    /// `(footprint, pattern)`, and capped (today: 16) so the reshape search stays
    /// bounded. Generators enumerate — they never rank.
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self>
    where
        Self: Sized;

    /// Footprint `(w, h)` in `nm` for **this** variant — feeds planning-area
    /// sizing before anything is drawn.
    fn estimate(&self, group: &DeviceGroup, process: &dyn Process) -> (i32, i32);

    /// Pins this variant exposes (routing entry points).
    fn ports(&self, group: &DeviceGroup) -> Vec<Pin>;

    /// Draw **this** variant. **Pure**: `(variant, group, constraints, process)
    /// -> Macro`, byte-deterministic.
    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro;
}
