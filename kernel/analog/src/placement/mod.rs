//! Placement-tier rules, scored against [`pnr_core::Layout`] (gp, dp).

pub mod cc;
pub mod dti;
pub mod environment;
pub mod isolation;
pub mod matching_pair;
pub mod proximity;
pub mod symmetry;
pub mod thermal;
pub mod utilization;

pub use dti::DtiBand;
pub use environment::{Environment, Surroundings};
pub use isolation::Isolation;
pub use matching_pair::MatchingPair;
pub use proximity::Proximity;
pub use symmetry::Symmetry;
pub use thermal::ThermalGradient;
