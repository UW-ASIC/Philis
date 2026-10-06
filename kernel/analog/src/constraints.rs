//! Per-circuit structural intent (cold; read by `cells`, not scored).

use crate::cell::{GuardRingRequirement, Unitization};

/// The cell-tier directives of one circuit. `Clone` because `library::run`
/// retargets `guard_rings` into collapsed cell ids on its own copy.
#[derive(Default, Clone)]
pub struct Constraints {
    /// Matched sets to build from identical units, one entry per set.
    pub unitization: Vec<Unitization>,
    /// Guard rings to draw, one entry per protected device.
    pub guard_rings: Vec<GuardRingRequirement>,
}
