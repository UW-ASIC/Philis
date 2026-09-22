//! The COLD, structural layer: authored intent that *produces* [`crate::Rule`]s.
//!
//! Two-layer split (see crate docs): `Constraints` aggregate the structural
//! directives the annotator recovers — unit decomposition, dummies, guard rings,
//! LDE and stress bounds. They are read by `cells` to pick and draw device
//! variants, and by `apply_placement`/`apply_routing` to *derive* the per-move
//! scoring [`crate::Rule`]s. They are **not** evaluated per move; they are cold.
//!
//! There is deliberately **no `GroupConstraint`**: a group is just a
//! [`pnr_core::Target::Group`] a placement rule (matching, symmetry, …) points at
//! — the constraint is the same, generalized. The concrete structural types live
//! under [`crate::cell`], catalogued there with textbook provenance; this struct
//! just gathers them per circuit.

use crate::cell::{DummyRequirement, GuardRingRequirement, LdeBound, StressBound, Unitization};

/// All structural intent recovered for a circuit.
///
/// `Clone` exists for the group collapse: `library::run` rewrites
/// `guard_rings[i].device` from device ids into collapsed cell ids and needs its
/// own copy to do it, while the original stays the annotator's device-indexed
/// truth.
#[derive(Default, Clone)]
pub struct Constraints {
    pub unitization: Vec<Unitization>,
    pub dummies: Vec<DummyRequirement>,
    pub guard_rings: Vec<GuardRingRequirement>,
    pub lde: Vec<LdeBound>,
    pub stress: Vec<StressBound>,
}
