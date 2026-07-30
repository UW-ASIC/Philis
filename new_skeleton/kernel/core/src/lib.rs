//! # `pnr_core` — the shared foundation (no analog theory)
//!
//! Everything every crate needs but that carries no analog *reasoning*: geometry
//! primitives, indices (incl. the device/group [`ids::Target`]), the netlist,
//! placement/routing state, the macro, and the report. `analog` depends on this
//! crate and adds the theory on top; the backend/frontend crates use both.
//!
//! Not here: recognised-block types (`Block`/`BlockKind`) live in
//! `backend/annotator` — they are recognition *outputs*, not foundation — and
//! `Pattern` is internal to the `cells` generators.
//!
//! The directory is `kernel/core/` but the package is **`pnr_core`** — a crate
//! literally named `core` would shadow the standard library's `core`.

#![allow(dead_code)]

pub mod geom;
pub mod hypergraph;
pub mod ids;
pub mod layout;
pub mod r#macro;
pub mod netlist;
pub mod oracle;
pub mod process;
pub mod report;
pub mod routes;
pub mod thermal;
pub mod unionfind;

pub use geom::{Dir, LayerId, Orient, Pin, Port, Rect, Shape};
pub use hypergraph::BipartiteHypergraph;
pub use unionfind::UnionFind;
pub use ids::{AxisId, DeviceId, GroupId, NetId, Target};
pub use layout::Layout;
pub use r#macro::{place_macro, place_macros, Macro};
pub use netlist::{Device, DeviceGroup, DeviceKind, Net, Netlist};
pub use oracle::{DrcSummary, NullOracle, Oracle};
pub use process::Process;
pub use report::{Report, Violation};
pub use routes::Routes;
