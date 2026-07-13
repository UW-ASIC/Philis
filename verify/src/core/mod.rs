//! Shared verification core.
//!
//! Everything reused by more than one engine (DRC/LVS/PEX/ERC/signoff) lives
//! here: the data-oriented geometry store, exact predicates, and the typed
//! validation errors. Engines depend on `core`; `core` depends on no engine.
pub mod exact;
pub mod geometry;
pub mod hierarchy_index;
pub mod traits;

pub use geometry::{Bbox, Edge, GeometryStore, LayerId, PolyId};
