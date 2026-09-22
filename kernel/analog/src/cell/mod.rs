//! # Cell-tier constraints — **structural**, consumed cold by `cells`.
//!
//! These are [`crate::Constraints`]-flavoured *directives*, not per-move
//! [`crate::Rule`]s: they shape how a device's geometry is drawn (dummies, guard
//! rings, unit fingers) or bound where it may be placed (LDE, stress). `cells`
//! reads them at generation time; nothing here is scored in the hot loop. One
//! constraint per file, each with its textbook provenance.

pub mod aging;
pub mod dummy;
pub mod environment;
pub mod guard_ring;
pub mod injector;
pub mod lde;
pub mod stress;
pub mod unitization;

pub use aging::{AgingConstraint, AgingMechanism, Severity};
pub use dummy::{DummyRequirement, DummyType};
pub use environment::{EnvironmentalConstraint, EnvironmentalKind, ThresholdUnit};
pub use guard_ring::{GuardRingRequirement, GuardRingType};
pub use injector::InjectorCandidate;
pub use lde::LdeBound;
pub use stress::StressBound;
pub use unitization::{SeriesParallel, Unitization};
