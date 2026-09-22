//! # `cells` — automatic device geometry
//!
//! Turns one [`DeviceGroup`] into drawn [`Macro`] variants: a MOSFET stack, a
//! resistor, a capacitor array, a BJT, a diode, an inductor. One generator per
//! family, one file each. Every generator reads the group's sizing from its
//! covering [`analog::cell::Unitization`] and every layer/rule from the
//! [`Process`]; `draw` is pure and byte-deterministic.
//!
//! Pins are named `d{i}:{T}`: `i` is the member's index in `group.devices`,
//! `T` its schematic terminal (`G/D/S/B`, `P/N`, `C/B/E`). Pin nets are
//! synthetic placeholders that the caller rebinds by name.

pub mod bjt;
pub mod builder;
pub mod capacitor;
pub mod diode;
pub mod inductor;
pub mod mosfet;
pub mod post_cell;
pub mod resistor;

pub use builder::Builder;

use analog::Constraints;
use pnr_core::{DeviceGroup, Macro, Process};

/// How the fingers/segments of a matched group interleave.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pattern {
    /// Each device's fingers contiguous.
    Single,
    /// One-dimensional common centroid (ABBA).
    Cc1d,
    /// Simple interdigitation (ABAB).
    Interdig,
}

/// A device-family generator over its variant space.
pub trait Cell: Clone {
    /// Every feasible variant for `group`, in a deterministic order (the order
    /// `Layout::variant` indexes). Never ranked: the placer picks.
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self>;

    /// Draw this variant.
    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro;
}
