//! Per-circuit structural intent (cold; read by `cells`, not scored).

use crate::cell::{GuardRingRequirement, Unitization};

/// `Clone` because `library::run` retargets `guard_rings` into collapsed cell
/// ids on its own copy.
#[derive(Default, Clone)]
pub struct Constraints {
    pub unitization: Vec<Unitization>,
    pub guard_rings: Vec<GuardRingRequirement>,
}
