//! `pnr_core` — shared foundation: geometry, ids, netlist, placement/routing
//! state, macros, reports. No analog theory (that is `analog`).
//!
//! Package is `pnr_core`, not `core`, so it does not shadow `std::core`.

pub mod geom;
pub mod hypergraph;
pub mod ids;
pub mod lanes;
pub mod layout;
pub mod r#macro;
pub mod netlist;
pub mod process;
pub mod report;
pub mod routes;
pub mod thermal;
pub mod unionfind;
pub mod units;

pub use geom::{Dir, LayerId, Orient, Pin, Rect, Shape};
pub use hypergraph::BipartiteHypergraph;
pub use ids::{AxisId, DeviceId, GroupId, NetId, Target};
pub use layout::Layout;
pub use netlist::{Device, DeviceGroup, DeviceKind, Net, Netlist};
pub use process::Process;
pub use r#macro::{pin_shares, place_macro, place_macros, Dummy, Macro};
pub use report::{Report, Violation};
pub use routes::Routes;
pub use unionfind::UnionFind;
pub use units::{PlacedUnit, Unit, UnitLib};
