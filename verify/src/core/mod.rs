//! Shared geometry core.
//!
//! Geometry reused by every engine (DRC/LVS/PEX/ERC/signoff) lives here: the
//! data-oriented store, exact predicates, and hierarchy-preserving spatial
//! queries. Engines depend on `core`; `core` depends on no engine. Backend
//! selection and the rule abstraction are top-level (`crate::backend`,
//! `crate::rule`) — core only handles geometry.
pub mod connectivity;
pub mod device_plane;
pub mod exact;
pub mod geometry;
pub mod hierarchy_index;
pub mod io;
pub mod sort_scan;

pub use geometry::{Bbox, Edge, GeometryStore, LayerId, PolyId};
pub use geometry::rects::{
    Rect, RectSet, decompose_all, decompose_rectilinear, rect_overlap_area_pos, rect_touch,
};
pub use device_plane::DevicePlane;
