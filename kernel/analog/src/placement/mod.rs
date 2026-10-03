//! Placement-tier rules, scored against [`pnr_core::Layout`] (gp, dp).

pub mod dti;
pub mod environment;
pub mod isolation;
pub mod matched_set;
pub mod matching_pair;
pub mod orientation;
pub mod proximity;
pub mod symmetry;
pub mod utilization;

pub use dti::DtiBand;
pub use environment::{Environment, Surroundings};
pub use isolation::Isolation;
pub use matched_set::MatchedSet;
pub use orientation::{OrientCheck, OrientationSet};
pub use proximity::Proximity;
pub use symmetry::Symmetry;
