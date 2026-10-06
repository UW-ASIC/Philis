//! Placement-tier rules, scored against [`pnr_core::Layout`] (gp, dp).
//!
//! Each module owns one rule kind: a `Copy` [`crate::rule::Rule`] scored in a
//! `Vec` batch (pairwise spacing, symmetry), or a hand-written
//! [`crate::rule::RuleBatch`] when a rule reads a whole set at once (matched
//! sets, islands, environment, utilization, performance). Every cost is
//! dimensionless (PLC-18) so batches sum into one objective.

use pnr_core::ids::Target;

pub mod dti;
pub mod environment;
pub mod heat;
pub mod island;
pub mod isolation;
pub mod matched_set;
pub mod orientation;
pub mod perf;
pub mod proximity;
pub mod symmetry;
pub mod utilization;

pub use dti::DtiBand;
pub use environment::{EnvGeo, Environment, LiveEnvironment, Surroundings};
pub use heat::HeatSeparation;
pub use island::SymmetryIsland;
pub use isolation::{Isolation, SubstrateBalance};
pub use matched_set::MatchedSet;
pub use orientation::{OrientCheck, OrientationSet};
pub use perf::PlacePerf;
pub use proximity::Proximity;
pub use symmetry::{SymMode, Symmetry};

/// Appends the device id of every [`Target::Device`] in `targets`, in order;
/// groups contribute nothing (they name no single cell to re-score).
#[inline]
pub(crate) fn push_devices(out: &mut Vec<u32>, targets: &[Target]) {
    out.extend(targets.iter().filter_map(|t| match t {
        Target::Device(d) => Some(u32::from(d.0)),
        Target::Group(_) => None,
    }));
}
