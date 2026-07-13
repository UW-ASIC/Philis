//! Shared geometry core.
//!
//! Geometry reused by every engine (DRC/LVS/PEX/ERC/signoff) lives here: the
//! data-oriented store, exact predicates, and hierarchy-preserving spatial
//! queries. Engines depend on `core`; `core` depends on no engine. Backend
//! selection and the rule abstraction are top-level (`crate::backend`,
//! `crate::rule`) — core only handles geometry.
pub mod connectivity;
pub mod exact;
pub mod geometry;
pub mod hierarchy_index;

pub use geometry::{Bbox, Edge, GeometryStore, LayerId, PolyId};
