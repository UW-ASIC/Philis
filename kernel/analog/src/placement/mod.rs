//! # Placement-tier rules — **Device↔Device**, scored against [`crate::Layout`].
//!
//! Consumed by `backend/gp` and `backend/dp`. Each rule is a small `Copy`
//! [`crate::Rule`] with `On = Layout`. Registered in [`crate::apply_placement`].
//! One rule per file; the file's rustdoc carries the analog theory and its
//! textbook provenance (`AOAL`/`FOLD`/`ALS`/`PNR_ANALOG`).

pub mod cc;
pub mod dti;
pub mod isolation;
pub mod matching_pair;
pub mod proximity;
pub mod symmetry;
pub mod thermal;

pub use cc::CommonCentroid;
pub use dti::DtiBand;
pub use isolation::Isolation;
pub use matching_pair::{Matching, MatchingPair};
pub use proximity::Proximity;
pub use symmetry::Symmetry;
pub use thermal::ThermalGradient;
